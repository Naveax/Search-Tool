[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidateSet('Prepare','Verify')] [string]$Mode,
    [Parameter(Mandatory)] [string]$IndexRoot,
    [ValidatePattern('^[A-Za-z]:$')] [string]$Drive = 'C:',
    [string]$StatePath = (Join-Path $env:TEMP 'search-tool-power-cycle-state.json'),
    [string]$MarkerDir = (Join-Path $env:TEMP 'SearchToolPowerCycle'),
    [string]$ReportPath = (Join-Path $env:TEMP 'search-tool-power-cycle-verify.json')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cli = Join-Path $root 'target\release\search-tool.exe'
if (-not (Test-Path -LiteralPath $cli)) { throw "Missing release CLI: $cli" }

$letter = $Drive.Substring(0, 1).ToUpperInvariant()
$index = Join-Path $IndexRoot "$letter.stidx"

function Invoke-Capture([string[]]$Args) {
    $out = @(& $cli @Args 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0) {
        throw "search-tool $($Args -join ' ') failed: $($out -join ' | ')"
    }
    return $out
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
    Start-Sleep -Seconds 4

    $search = Invoke-Capture @('search', $index, $markerName)
    $state = [ordered]@{
        prepared_utc = [DateTime]::UtcNow.ToString('o')
        boot_time = Get-BootTime
        marker_name = $markerName
        marker_path = $markerPath
        marker_search_hit = [bool]($search | Where-Object { $_ -like "*$markerName*" })
        checkpoint_sha256 = if (Test-Path $checkpoint) { (Get-FileHash $checkpoint -Algorithm SHA256).Hash } else { $null }
        service = Get-ServiceState
        usn = Invoke-Capture @('ntfs-status', $Drive)
        doctor = Invoke-Capture @('doctor', $IndexRoot)
        verify_deep = Invoke-Capture @('verify-deep', $index)
    }
    $parent = Split-Path -Parent $StatePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $state | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $StatePath -Encoding UTF8
    $state | ConvertTo-Json -Depth 8
    exit 0
}

if (-not (Test-Path -LiteralPath $StatePath)) { throw "Prepare state missing: $StatePath" }
$before = Get-Content -Raw -LiteralPath $StatePath | ConvertFrom-Json
$deadline = (Get-Date).AddSeconds(30)
do {

    $search = @(& $cli search $index $before.marker_name 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -eq 0 -and ($search | Where-Object { $_ -like "*$($before.marker_name)*" })) { break }
    Start-Sleep -Seconds 2
} while ((Get-Date) -lt $deadline)

$postName = "search-tool-postcycle-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
$postPath = Join-Path $MarkerDir $postName
Set-Content -LiteralPath $postPath -Value 'post-cycle marker' -Encoding UTF8
Start-Sleep -Seconds 4
$postSearch = Invoke-Capture @('search', $index, $postName)
$boot = Get-BootTime
$after = [ordered]@{
    verified_utc = [DateTime]::UtcNow.ToString('o')
    boot_time = $boot
    previous_boot_time = $before.boot_time
    boot_changed = ($boot -ne $before.boot_time)
    pre_marker_visible = [bool]($search | Where-Object { $_ -like "*$($before.marker_name)*" })
    post_marker_visible = [bool]($postSearch | Where-Object { $_ -like "*$postName*" })
    checkpoint_sha256 = if (Test-Path $checkpoint) { (Get-FileHash $checkpoint -Algorithm SHA256).Hash } else { $null }

    previous_checkpoint_sha256 = $before.checkpoint_sha256
    service = Get-ServiceState
    usn = Invoke-Capture @('ntfs-status', $Drive)
    doctor = Invoke-Capture @('doctor', $IndexRoot)
    verify_deep = Invoke-Capture @('verify-deep', $index)
}
$parent = Split-Path -Parent $ReportPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
$after | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
$after | ConvertTo-Json -Depth 8
