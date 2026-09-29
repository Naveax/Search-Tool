[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidateSet('Prepare','Verify')] [string]$Mode,
    [Parameter(Mandatory)] [string]$IndexRoot,
    [ValidatePattern('^[A-Za-z]:$')] [string]$Drive = 'C:',
    [string]$StatePath = (Join-Path $env:TEMP 'search-tool-power-cycle-state.json'),
    [string]$MarkerDir = (Join-Path $env:TEMP 'SearchToolPowerCycle'),
    [string]$ReportPath = (Join-Path $env:TEMP 'search-tool-power-cycle-verify.json'),
    [ValidateSet('Any','Sleep','Reboot')] [string]$ExpectedCycle = 'Any',
    [ValidateRange(10, 600)] [int]$SearchCatchupTimeoutSeconds = 120
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cli = Join-Path $root 'target\release\search-tool.exe'
if (-not (Test-Path -LiteralPath $cli)) { throw "Missing release CLI: $cli" }

$letter = $Drive.Substring(0, 1).ToUpperInvariant()
$index = Join-Path $IndexRoot "$letter.stidx"

function Invoke-Capture {
    param(
        [Parameter(Mandatory)] [string[]]$CommandArgs,
        [int]$BusyRetrySeconds = 0
    )
    $deadline = (Get-Date).AddSeconds($BusyRetrySeconds)
    do {
        $out = @(& $cli @CommandArgs 2>&1 | ForEach-Object { [string]$_ })
        $code = $LASTEXITCODE
        if ($code -eq 0) { return $out }

        $busy = [bool]($out | Where-Object { $_ -match 'index mutation is already in progress' })
        if (-not $busy -or $BusyRetrySeconds -le 0 -or (Get-Date) -ge $deadline) {
            throw "search-tool $($CommandArgs -join ' ') failed: $($out -join ' | ')"
        }
        Start-Sleep -Milliseconds 250
    } while ($true)
}

function Wait-SearchHit([string]$Name, [int]$TimeoutSeconds = 45) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        $out = Invoke-Capture -CommandArgs @('search', $index, $Name)
        if ($out | Where-Object { $_ -like "*$Name*" }) { return $true }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)
    return $false
}

function Get-ServiceState {
    $svc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
    $proc = if ($svc -and $svc.Status -eq 'Running') {
        Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction SilentlyContinue
    } else { $null }
    return [ordered]@{
        exists = [bool]$svc
        status = if ($svc) { [string]$svc.Status } else { 'Missing' }
        start_type = if ($svc) { [string]$svc.StartType } else { $null }
        pid = if ($proc) { [int]$proc.ProcessId } else { 0 }
    }
}

function Get-BootTime {
    (Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('o')
}

New-Item -ItemType Directory -Force -Path $MarkerDir | Out-Null
$checkpoint = "$index.usn"

if ($Mode -eq 'Prepare') {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $markerName = "search-tool-powercycle-$stamp.txt"
    $markerPath = Join-Path $MarkerDir $markerName
    Set-Content -LiteralPath $markerPath -Value "Search Tool power-cycle marker $stamp" -Encoding UTF8
    $markerVisible = Wait-SearchHit $markerName -TimeoutSeconds $SearchCatchupTimeoutSeconds
    if (-not $markerVisible) {
        throw "Pre-cycle marker did not become visible through automatic USN sync: $markerName"
    }

    $serviceState = Get-ServiceState
    if (-not $serviceState.exists -or $serviceState.status -ne 'Running' -or $serviceState.start_type -ne 'Automatic') {
        throw "SearchToolIndexer must be Running + Automatic before power-cycle preparation."
    }

    $state = [ordered]@{
        prepared_utc = [DateTime]::UtcNow.ToString('o')
        boot_time = Get-BootTime
        marker_name = $markerName
        marker_path = $markerPath
        marker_search_hit = $markerVisible
        checkpoint_sha256 = if (Test-Path $checkpoint) { (Get-FileHash $checkpoint -Algorithm SHA256).Hash } else { $null }
        search_catchup_timeout_seconds = $SearchCatchupTimeoutSeconds
        service = $serviceState
        usn = Invoke-Capture -CommandArgs @('ntfs-status', $Drive)
        doctor = Invoke-Capture -CommandArgs @('doctor', $index) -BusyRetrySeconds 30
        verify_deep = Invoke-Capture -CommandArgs @('verify-deep', $index) -BusyRetrySeconds 30
    }
    $parent = Split-Path -Parent $StatePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $state | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $StatePath -Encoding UTF8
    $state | ConvertTo-Json -Depth 8
    exit 0
}

if (-not (Test-Path -LiteralPath $StatePath)) { throw "Prepare state missing: $StatePath" }
$before = Get-Content -Raw -LiteralPath $StatePath | ConvertFrom-Json
$preVisible = Wait-SearchHit ([string]$before.marker_name) -TimeoutSeconds $SearchCatchupTimeoutSeconds

$postName = "search-tool-postcycle-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
$postPath = Join-Path $MarkerDir $postName
Set-Content -LiteralPath $postPath -Value 'post-cycle marker' -Encoding UTF8
$postVisible = Wait-SearchHit $postName -TimeoutSeconds $SearchCatchupTimeoutSeconds
$boot = Get-BootTime
$bootChanged = ($boot -ne $before.boot_time)
$serviceState = Get-ServiceState
$cycleMatch = switch ($ExpectedCycle) {
    'Sleep' { -not $bootChanged }
    'Reboot' { $bootChanged }
    default { $true }
}
$serviceOk = $serviceState.exists -and
    $serviceState.status -eq 'Running' -and
    $serviceState.start_type -eq 'Automatic'
$usn = Invoke-Capture -CommandArgs @('ntfs-status', $Drive)
$doctor = Invoke-Capture -CommandArgs @('doctor', $index) -BusyRetrySeconds 30
$verifyDeep = Invoke-Capture -CommandArgs @('verify-deep', $index) -BusyRetrySeconds 30
$passed = $preVisible -and $postVisible -and $serviceOk -and $cycleMatch

$after = [ordered]@{
    verified_utc = [DateTime]::UtcNow.ToString('o')
    boot_time = $boot
    previous_boot_time = $before.boot_time
    boot_changed = $bootChanged
    expected_cycle = $ExpectedCycle
    cycle_match = $cycleMatch
    pre_marker_visible = $preVisible
    post_marker_visible = $postVisible
    checkpoint_sha256 = if (Test-Path $checkpoint) { (Get-FileHash $checkpoint -Algorithm SHA256).Hash } else { $null }
    previous_checkpoint_sha256 = $before.checkpoint_sha256
    checkpoint_changed = if (Test-Path $checkpoint) {
        ((Get-FileHash $checkpoint -Algorithm SHA256).Hash -ne $before.checkpoint_sha256)
    } else { $false }
    search_catchup_timeout_seconds = $SearchCatchupTimeoutSeconds
    service = $serviceState
    service_ok = $serviceOk
    usn = $usn
    doctor = $doctor
    verify_deep = $verifyDeep
    result = if ($passed) { 'PASS' } else { 'FAIL' }
}
$parent = Split-Path -Parent $ReportPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
$after | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
$after | ConvertTo-Json -Depth 8
if (-not $passed) {
    throw "Power-cycle verification failed: pre_marker=$preVisible post_marker=$postVisible service_ok=$serviceOk cycle_match=$cycleMatch"
}
