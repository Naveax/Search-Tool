[CmdletBinding()]
param(
    [ValidateRange(1, 120)] [int]$SoakMinutes = 1,
    [string]$ServiceName = 'SearchToolIndexerIntegration'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($ServiceName) -or
    $ServiceName.Length -gt 256 -or
    $ServiceName -match '[\\/"]') {
    throw "Invalid Windows service name: '$ServiceName'"
}
$serviceNameFilter = $ServiceName.Replace("'", "''")

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$conflicts = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $command = [string]$_.CommandLine
    $filePos = $command.IndexOf('-File', [StringComparison]::OrdinalIgnoreCase)
    $commandPos = $command.IndexOf('-Command', [StringComparison]::OrdinalIgnoreCase)
    $_.ProcessId -ne $PID -and $_.Name -match '^(powershell|pwsh)(\.exe)?$' -and
    $filePos -ge 0 -and ($commandPos -lt 0 -or $filePos -lt $commandPos) -and
    $command -match '(?i)(windows-soak|windows-integration|windows-release-gate)\.ps1'
})
if ($conflicts) {
    $details = ($conflicts | ForEach-Object { "pid=$($_.ProcessId) command=$($_.CommandLine)" }) -join '; '
    throw "Refusing Windows integration while another Search Tool validation is active: $details"
}
$cli = Join-Path $repo 'target\release\search-tool.exe'
$worker = Join-Path $repo 'target\release\search-tool-worker.exe'
$service = Join-Path $repo 'target\release\search-tool-service.exe'
$gui = Join-Path $repo 'target\release\search-tool-gui.exe'

foreach ($exe in @($cli, $worker, $service, $gui)) {
    if (-not (Test-Path -LiteralPath $exe)) {
        throw "Missing release executable: $exe"
    }
}

function Trace-IndexerService([string]$Phase) {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($svc) {
        $info = Get-CimInstance Win32_Service -Filter "Name='$serviceNameFilter'" -ErrorAction SilentlyContinue
        Write-Host ("TRACE_SERVICE phase={0} status={1} pid={2} path={3}" -f $Phase, $svc.Status, $info.ProcessId, $info.PathName)
    } else {
        Write-Host ("TRACE_SERVICE phase={0} status=ABSENT" -f $Phase)
    }
}

function Remove-OwnedIndexerService {
    $existing = Get-CimInstance Win32_Service -Filter "Name='$serviceNameFilter'" -ErrorAction SilentlyContinue
    if (-not $existing) { return }

    $expected = [IO.Path]::GetFullPath($service)
    $actual = [string]$existing.PathName
    if ($actual -notlike "*$expected*") {
        throw "Refusing to remove $ServiceName owned by another installation: $actual"
    }

    $state = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($state -and $state.Status -ne 'Stopped') {
        & $service '--service-name' $ServiceName '--stop' | Out-Host
        if ($LASTEXITCODE -ne 0) { throw "Failed to stop stale integration service: $LASTEXITCODE" }
    }

    & $service '--service-name' $ServiceName '--uninstall' | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Failed to uninstall stale integration service: $LASTEXITCODE" }

    $deadline = (Get-Date).AddSeconds(20)
    do {
        $remaining = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        if (-not $remaining) { return }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    throw "$ServiceName still exists after uninstall"
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory)] [string]$FilePath,
        [Parameter(ValueFromRemainingArguments)] [string[]]$ArgumentList
    )
    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed ($LASTEXITCODE): $FilePath $($ArgumentList -join ' ')"
    }
}

function Invoke-SearchToolCapture {
    param([Parameter(ValueFromRemainingArguments)] [string[]]$ArgumentList)
    $output = & $cli @ArgumentList 2>&1
    if ($LASTEXITCODE -ne 0) {
        $output | ForEach-Object { Write-Host $_ }
        throw "search-tool failed ($LASTEXITCODE): $($ArgumentList -join ' ')"
    }
    return @($output | ForEach-Object { [string]$_ })
}

function Assert-Contains {
    param(
        [Parameter(Mandatory)] [AllowNull()] [AllowEmptyCollection()] [string[]]$Lines,
        [Parameter(Mandatory)] [string]$Needle
    )
    if (-not ($Lines | Where-Object { $_ -like "*$Needle*" })) {
        throw "Expected output containing '$Needle', got:`n$($Lines -join [Environment]::NewLine)"
    }
}

function Trace-IsolatedIndexParentRecords([string]$IndexPath) {
    # This integration test creates its own disposable NTFS VHD. Print only
    # synthetic MFT record IDs and flags, never filesystem names or contents.
    # It helps distinguish missing roots from wrong parent type mappings.
    $bytes = [IO.File]::ReadAllBytes($IndexPath)
    if ($bytes.Length -lt 64 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 5) -ne 'STIDX') {
        throw 'Unexpected integration index header'
    }
    $count = [BitConverter]::ToUInt64($bytes, 16)
    if ($count -gt 4096 -or $bytes.Length -lt (64 + [int64]$count * 40)) {
        throw 'Unexpected integration index record count or size'
    }
    $ids = [Collections.Generic.HashSet[uint64]]::new()
    for ($i = 0; $i -lt $count; $i++) {
        [void]$ids.Add([BitConverter]::ToUInt64($bytes, (64 + 40 * $i)))
    }
    for ($i = 0; $i -lt $count; $i++) {
        $offset = 64 + 40 * $i
        $id = [BitConverter]::ToUInt64($bytes, $offset)
        $parentId = [BitConverter]::ToUInt64($bytes, ($offset + 8))
        $flags = [BitConverter]::ToUInt16($bytes, ($offset + 36))
        Write-Host ("TRACE_ISOLATED_INDEX record={0} id={1} parent={2} flags={3} parent_present={4}" -f $i, $id, $parentId, $flags, ($parentId -eq 0 -or $ids.Contains($parentId)))
    }
}

$baseTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$work = Join-Path $baseTemp ("search-tool-windows-integration-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
$vhd = Join-Path $work 'search-tool-ci.vhdx'
$indexDir = Join-Path $work 'indexes'
New-Item -ItemType Directory -Force -Path $indexDir | Out-Null
$diskpartCreate = Join-Path $work 'create-vhd.txt'
$diskpartDetach = Join-Path $work 'detach-vhd.txt'

$lettersInUse = @(
    Get-PSDrive -PSProvider FileSystem | ForEach-Object { $_.Name.ToUpperInvariant() }
    Get-Volume -ErrorAction SilentlyContinue | Where-Object DriveLetter | ForEach-Object { ([string]$_.DriveLetter).ToUpperInvariant() }
    Get-Partition -ErrorAction SilentlyContinue | Where-Object DriveLetter | ForEach-Object { ([string]$_.DriveLetter).ToUpperInvariant() }
) | Sort-Object -Unique
$driveLetter = @('T','S','R','Q','P','O','N','M','L','K','J') | Where-Object { $lettersInUse -notcontains $_ } | Select-Object -First 1
if (-not $driveLetter) {
    throw 'No free integration-test drive letter found.'
}
$drive = "${driveLetter}:"
$root = "${drive}\"
$index = Join-Path $indexDir ("$driveLetter.stidx")
$serviceConfig = Join-Path $work 'service.conf'
$servicePointer = Join-Path (Split-Path -Parent $service) ("service.{0}.conf.path" -f $ServiceName)

@"
create vdisk file="$vhd" maximum=256 type=expandable
select vdisk file="$vhd"
attach vdisk
create partition primary
format fs=ntfs quick label=SearchToolCI
assign letter=$driveLetter
exit
"@ | Set-Content -LiteralPath $diskpartCreate -Encoding ASCII

@"
select vdisk file="$vhd"
detach vdisk
exit
"@ | Set-Content -LiteralPath $diskpartDetach -Encoding ASCII

$mounted = $false
$serviceInstalled = $false
$previousMaintenanceServiceName = [Environment]::GetEnvironmentVariable(
    'SEARCH_TOOL_SERVICE_NAME',
    [EnvironmentVariableTarget]::Process
)
Remove-OwnedIndexerService
Set-Content -LiteralPath $servicePointer -Value $serviceConfig -Encoding UTF8
$env:SEARCH_TOOL_SERVICE_NAME = $ServiceName
Trace-IndexerService 'pre-try'
try {
    Write-Host '==> create isolated NTFS VHD'
    & diskpart.exe /s $diskpartCreate
    if ($LASTEXITCODE -ne 0) { throw "diskpart create failed: $LASTEXITCODE" }
    $mounted = $true

    if (-not (Test-Path -LiteralPath $root)) {
        throw "VHD drive $drive did not mount"
    }

    Write-Host '==> enable USN journal'
    & fsutil.exe usn createjournal m=10485760 a=1048576 $drive
    if ($LASTEXITCODE -ne 0) { throw "fsutil USN create failed: $LASTEXITCODE" }

    Write-Host '==> seed deterministic files'
    New-Item -ItemType Directory -Force -Path "$drive\projects\node-sample" | Out-Null
    New-Item -ItemType Directory -Force -Path "$drive\notes" | Out-Null
    New-Item -ItemType Directory -Force -Path "$drive\cache" | Out-Null
    Set-Content -LiteralPath "$drive\projects\node-sample\node.exe" -Value 'fake-node-binary-marker' -Encoding UTF8
    Set-Content -LiteralPath "$drive\projects\node-sample\package.json" -Value '{"name":"node-sample","scripts":{"start":"node index.js"}}' -Encoding UTF8
    Set-Content -LiteralPath "$drive\projects\node-sample\index.js" -Value 'console.log("search-tool-content-marker")' -Encoding UTF8
    Set-Content -LiteralPath "$drive\notes\websocket.txt" -Value 'websocket content integration marker' -Encoding UTF8
    Set-Content -LiteralPath "$drive\cache\old.tmp" -Value 'temporary cache marker' -Encoding UTF8

    Write-Host '==> raw NTFS / USN smoke'
    Invoke-Checked $cli 'ntfs-status' $drive
    Invoke-Checked $cli 'mft-count' $drive

    Write-Host '==> initial MFT index'
    Invoke-Checked $cli 'index' $drive $index
    Invoke-Checked $cli 'verify' $index
    Trace-IsolatedIndexParentRecords $index
    Trace-IndexerService 'after-initial-verify'

    $node = Invoke-SearchToolCapture 'search' $index 'node'
    Assert-Contains $node 'node.exe'

    Write-Host '==> native GUI initialization smoke'
    Invoke-Checked $gui '--smoke' $indexDir
    Trace-IndexerService 'after-gui-smoke'

    Write-Host '==> metadata + filtered search'
    Invoke-Checked $cli 'metadata-build' $drive $index
    $filtered = Invoke-SearchToolCapture 'search' $index 'node ext:exe size:>1b'
    Assert-Contains $filtered 'node.exe'
    Trace-IndexerService 'after-metadata-filter'

    Write-Host '==> plain content sidecar'
    Invoke-Checked $cli 'content-build' $drive $index
    $content = Invoke-SearchToolCapture 'content-search' $index 'websocket content'
    Assert-Contains $content 'websocket.txt'
    Trace-IndexerService 'after-content'

    Write-Host '==> isolated parser worker plain-text smoke'
    $preview = & $worker 'preview' "$drive\notes\websocket.txt" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "worker preview failed: $LASTEXITCODE" }
    Assert-Contains @($preview | ForEach-Object { [string]$_ }) 'integration marker'

    Write-Host '==> mutate filesystem after checkpoint'
    Set-Content -LiteralPath "$drive\projects\node-sample\fresh-usn-marker.txt" -Value 'fresh usn marker' -Encoding UTF8
    Rename-Item -LiteralPath "$drive\projects\node-sample\node.exe" -NewName 'node-renamed.exe'
    Remove-Item -LiteralPath "$drive\cache\old.tmp" -Force

    # Ensure handles are closed and NTFS has emitted close-associated records before reading.
    Start-Sleep -Milliseconds 300

    Write-Host '==> incremental USN sync'
    Invoke-Checked $cli 'sync' $drive $index
    Trace-IndexerService 'after-usn-sync'

    $fresh = Invoke-SearchToolCapture 'search' $index 'fresh-usn-marker'
    Assert-Contains $fresh 'fresh-usn-marker.txt'
    $renamed = Invoke-SearchToolCapture 'search' $index 'node-renamed'
    Assert-Contains $renamed 'node-renamed.exe'
    $oldName = Invoke-SearchToolCapture 'search' $index 'node.exe'
    if ($oldName | Where-Object { $_ -like '*\node.exe' }) {
        throw "Renamed old path is still visible after USN sync:`n$($oldName -join [Environment]::NewLine)"
    }

    Write-Host '==> sidecar freshness after USN change'
    $savedErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $staleFiltered = & $cli 'search' $index 'node-renamed ext:exe size:>1b' 2>&1
    $staleFilteredExit = $LASTEXITCODE
    $ErrorActionPreference = $savedErrorAction
    if ($staleFilteredExit -ne 3) {
        $staleFiltered | ForEach-Object { Write-Host $_ }
        throw "Expected stale metadata exit code 3, got $staleFilteredExit"
    }
    Assert-Contains @($staleFiltered | ForEach-Object { [string]$_ }) 'metadata is stale'

    # Stale content remains usable for unchanged files, but cannot promise newly changed content.
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $cli
    $psi.Arguments = 'content-search "' + $index + '" "websocket content"'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $proc = [Diagnostics.Process]::new()
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $staleStdout = $proc.StandardOutput.ReadToEnd()
    $staleStderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    if ($proc.ExitCode -ne 0) { throw "stale content search failed: $($proc.ExitCode)" }
    $staleContent = @($staleStdout -split '\r?\n' | Where-Object { $_ })
    $staleContentWarning = @($staleStderr -split '\r?\n' | Where-Object { $_ })
    Assert-Contains $staleContent 'websocket.txt'
    Assert-Contains $staleContentWarning 'content index is stale'

    Write-Host '==> compact, refresh sidecars and deep verify'
    Invoke-Checked $cli 'compact' $index
    Invoke-Checked $cli 'metadata-build' $drive $index
    Invoke-Checked $cli 'content-build' $drive $index
    $freshFiltered = Invoke-SearchToolCapture 'search' $index 'node-renamed ext:exe size:>1b'
    Assert-Contains $freshFiltered 'node-renamed.exe'
    $freshContent = Invoke-SearchToolCapture 'content-search' $index 'fresh usn marker'
    Assert-Contains $freshContent 'fresh-usn-marker.txt'
    Trace-IndexerService 'before-repair'
    Invoke-Checked $cli 'verify' $index
    Invoke-Checked $cli 'verify-deep' $index

    Write-Host '==> directory-level sidecar repair'
    $nameCheckpoint = "$index.ncp"
    if (-not (Test-Path -LiteralPath $nameCheckpoint)) {
        throw "Expected name checkpoint before repair test: $nameCheckpoint"
    }
    Remove-Item -LiteralPath $nameCheckpoint -Force
    Invoke-Checked $cli 'repair' $indexDir
    if (-not (Test-Path -LiteralPath $nameCheckpoint)) {
        throw 'Directory repair did not recreate the name checkpoint.'
    }
    Invoke-Checked $cli 'verify' $index

    Write-Host '==> installed service + manual-maintenance guard'
    Invoke-Checked $service '--service-name' $ServiceName '--install' $drive $index
    $serviceInstalled = $true
    Invoke-Checked $service '--service-name' $ServiceName '--start'
    $deadline = (Get-Date).AddSeconds(20)
    do {
        $serviceState = (Get-Service -Name $ServiceName -ErrorAction Stop).Status
        if ($serviceState -eq 'Running') { break }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    if ($serviceState -ne 'Running') {
        throw "$ServiceName did not reach Running state: $serviceState"
    }

    Write-Host '==> background service USN sync'
    Set-Content -LiteralPath "$drive\notes\service-auto-marker.txt" -Value 'service automatic sync marker' -Encoding UTF8
    $deadline = (Get-Date).AddSeconds(20)
    $serviceSearch = @()
    do {
        Start-Sleep -Milliseconds 500
        $serviceSearch = @(& $cli 'search' $index 'service-auto-marker' 2>&1 | ForEach-Object { [string]$_ })
        if ($LASTEXITCODE -eq 0 -and ($serviceSearch | Where-Object { $_ -like '*service-auto-marker.txt*' })) { break }
    } while ((Get-Date) -lt $deadline)
    Assert-Contains $serviceSearch 'service-auto-marker.txt'

    Rename-Item -LiteralPath "$drive\notes\service-auto-marker.txt" -NewName 'service-auto-renamed.txt'
    $deadline = (Get-Date).AddSeconds(20)
    $serviceRenamed = @()
    do {
        Start-Sleep -Milliseconds 500
        $serviceRenamed = @(& $cli 'search' $index 'service-auto-renamed' 2>&1 | ForEach-Object { [string]$_ })
        if ($LASTEXITCODE -eq 0 -and ($serviceRenamed | Where-Object { $_ -like '*service-auto-renamed.txt*' })) { break }
    } while ((Get-Date) -lt $deadline)
    Assert-Contains $serviceRenamed 'service-auto-renamed.txt'
    $serviceOld = Invoke-SearchToolCapture 'search' $index 'service-auto-marker'
    if ($serviceOld | Where-Object { $_ -like '*service-auto-marker.txt*' }) {
        throw "Service rename left old path visible:`n$($serviceOld -join [Environment]::NewLine)"
    }

    Remove-Item -LiteralPath "$drive\notes\service-auto-renamed.txt" -Force
    # Service sync intentionally yields to foreground activity. Give delete
    # propagation more headroom than create/rename so a busy CI host does not
    # turn a healthy USN path into a timing-only release-gate failure.
    $deadline = (Get-Date).AddSeconds(45)
    do {
        Start-Sleep -Milliseconds 500
        $serviceDeleted = Invoke-SearchToolCapture 'search' $index 'service-auto-renamed'
        if (-not ($serviceDeleted | Where-Object { $_ -like '*service-auto-renamed.txt*' })) { break }
    } while ((Get-Date) -lt $deadline)
    if ($serviceDeleted | Where-Object { $_ -like '*service-auto-renamed.txt*' }) {
        Write-Host 'DELETE_DIAG doctor-before-stop'
        & $cli 'doctor' $indexDir | ForEach-Object { Write-Host $_ }
        & $service '--service-name' $ServiceName '--stop' | Out-Host
        Write-Host 'DELETE_DIAG manual-sync'
        & $cli 'sync' $drive $index | ForEach-Object { Write-Host $_ }
        $manualSyncExit = $LASTEXITCODE
        $afterManualDelete = Invoke-SearchToolCapture 'search' $index 'service-auto-renamed'
        $manualCleared = -not ($afterManualDelete | Where-Object { $_ -like '*service-auto-renamed.txt*' })
        Write-Host ("DELETE_DIAG manual_sync_exit={0} cleared={1}" -f $manualSyncExit, $manualCleared)
        if ($manualCleared) {
            throw 'Service loop did not ingest delete in time; manual sync cleared it.'
        }
        throw 'Delete remained visible even after manual sync.'
    }

    Write-Host '==> short installed-service soak'
    $soakScript = Join-Path $repo 'scripts\windows-soak.ps1'
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $soakScript -Drive $drive -Index $index -ServiceName $ServiceName -DurationMinutes $SoakMinutes -BatchSize 8 -PollTimeoutSeconds 20 -NestedValidation
    if ($LASTEXITCODE -ne 0) { throw "windows soak failed: $LASTEXITCODE" }
    $postSoakService = Get-Service -Name $ServiceName -ErrorAction Stop
    $postSoakService.Refresh()
    $postSoakInfo = Get-CimInstance Win32_Service -Filter "Name='$serviceNameFilter'" -ErrorAction Stop
    Write-Host ("POST_SOAK_SERVICE status={0} pid={1} exit={2}" -f $postSoakService.Status, $postSoakInfo.ProcessId, $postSoakInfo.ExitCode)
    if ($postSoakService.Status -ne 'Running') { throw "Installed service stopped during soak: $($postSoakService.Status) exit=$($postSoakInfo.ExitCode)" }

    $savedErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $blockedMaintenance = & $cli 'maintain' $indexDir 2>&1
    $blockedMaintenanceExit = $LASTEXITCODE
    $ErrorActionPreference = $savedErrorAction
    if ($blockedMaintenanceExit -eq 0) {
        $blockedMaintenance | ForEach-Object { Write-Host $_ }
        throw 'Manual maintenance unexpectedly succeeded while the service was running.'
    }
    Assert-Contains @($blockedMaintenance | ForEach-Object { [string]$_ }) 'service_guard=BLOCK'

    Invoke-Checked $service '--service-name' $ServiceName '--stop'
    $deadline = (Get-Date).AddSeconds(20)
    do {
        $serviceState = (Get-Service -Name $ServiceName -ErrorAction Stop).Status
        if ($serviceState -eq 'Stopped') { break }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    if ($serviceState -ne 'Stopped') {
        throw "$ServiceName did not stop cleanly: $serviceState"
    }

    Write-Host '==> directory-level maintain after service stop'
    Set-Content -LiteralPath "$drive\notes\maintain-marker.txt" -Value 'maintain delta marker' -Encoding UTF8
    Start-Sleep -Milliseconds 300
    Invoke-Checked $cli 'sync' $drive $index
    $maintained = Invoke-SearchToolCapture 'maintain' $indexDir
    Assert-Contains $maintained 'final_verify=PASS'
    Invoke-Checked $cli 'doctor' $indexDir

    Write-Host 'WINDOWS_NTFS_INTEGRATION=PASS'
}
finally {
    Write-Host '==> uninstall integration service'
    Remove-OwnedIndexerService
    Remove-Item -LiteralPath $servicePointer -Force -ErrorAction SilentlyContinue
    if ($null -eq $previousMaintenanceServiceName) {
        Remove-Item Env:SEARCH_TOOL_SERVICE_NAME -ErrorAction SilentlyContinue
    } else {
        $env:SEARCH_TOOL_SERVICE_NAME = $previousMaintenanceServiceName
    }
    if ($mounted -or (Test-Path -LiteralPath $vhd)) {
        Write-Host '==> detach integration VHD'
        & diskpart.exe /s $diskpartDetach | Out-Host
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
