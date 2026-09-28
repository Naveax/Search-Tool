[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidatePattern('^[A-Za-z]:$')] [string]$Drive,
    [Parameter(Mandatory)] [string]$Index,
    [int]$DurationMinutes = 15,
    [int]$BatchSize = 32,
    [int]$PollTimeoutSeconds = 30,
    [switch]$ManualSync,
    [switch]$CrashRestartService,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
function Resolve-ToolBinary([string]$Name) {
    $beside = Join-Path $PSScriptRoot $Name
    if (Test-Path -LiteralPath $beside) { return (Resolve-Path -LiteralPath $beside).Path }
    $repoBuild = Join-Path $root "target\release\$Name"
    if (Test-Path -LiteralPath $repoBuild) { return (Resolve-Path -LiteralPath $repoBuild).Path }
    throw "Missing release executable: $Name"
}
$cli = Resolve-ToolBinary 'search-tool.exe'
$serviceExe = Resolve-ToolBinary 'search-tool-service.exe'
$indexPath = (Resolve-Path -LiteralPath $Index).Path
$driveRoot = "$($Drive.Substring(0,1).ToUpperInvariant()):\"
$testRoot = Join-Path $driveRoot ('.search-tool-soak-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null

function Invoke-Checked {
    param(
        [Parameter(Mandatory)] [string]$FilePath,
        [Parameter(Mandatory)] [string[]]$ArgumentList
    )
    & $FilePath @ArgumentList | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $FilePath $($ArgumentList -join ' ')" }
}

function Search-Lines([string]$Query) {
    $out = @(& $cli 'search' $indexPath $Query 64 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0) { throw "search failed ($LASTEXITCODE): $Query`n$($out -join [Environment]::NewLine)" }
    return $out
}

function Invoke-VerifyDeepEventually {
    $deadline = (Get-Date).AddSeconds([Math]::Max(5, $PollTimeoutSeconds))
    do {
        $savedErrorAction = $ErrorActionPreference
        try {
            # PowerShell 5 turns native stderr into ErrorRecord objects when the
            # preference is Stop. Capture stderr as data so the busy-lock case can
            # be classified by exit code/message instead of becoming an exception.
            $ErrorActionPreference = 'Continue'
            $out = @(& $cli 'verify-deep' $indexPath 2>&1 | ForEach-Object { [string]$_ })
            $code = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $savedErrorAction
        }
        if ($code -eq 0) {
            $out | ForEach-Object { Write-Host $_ }
            return
        }
        $busy = $out | Where-Object { $_ -match 'mutation is already in progress' }
        if (-not $busy) {
            throw "verify-deep failed ($code): $($out -join [Environment]::NewLine)"
        }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    throw "Timed out waiting for verify-deep mutation lock"
}

function Wait-Visible([string]$Needle) {
    $deadline = (Get-Date).AddSeconds($PollTimeoutSeconds)
    do {
        if ($ManualSync) { Invoke-Checked -FilePath $cli -ArgumentList @('sync', $Drive, $indexPath) }
        $lines = Search-Lines $Needle
        if ($lines | Where-Object { $_ -like "*$Needle*" }) { return }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    Write-Host "SOAK_DIAG_VISIBLE needle=$Needle"
    & $cli 'doctor' $indexPath 2>&1 | ForEach-Object { Write-Host $_ }
    $svc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
    Write-Host ("SOAK_DIAG_SERVICE_QUERY exists={0} status={1}" -f [bool]$svc, $(if ($svc) { $svc.Status } else { 'none' }))
    if ($svc) {
        $info = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction SilentlyContinue
        Write-Host ("SOAK_DIAG_SERVICE status={0} pid={1} exit={2}" -f $svc.Status, $info.ProcessId, $info.ExitCode)
    }
    Get-Process -Name 'search-tool-service' -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("SOAK_DIAG_PROCESS pid={0} cpu={1} ws={2} private={3}" -f $_.Id, $_.CPU, $_.WorkingSet64, $_.PrivateMemorySize64)
    }
    throw "Timed out waiting for '$Needle' to become searchable"
}

function Wait-Absent([string]$Needle) {
    $deadline = (Get-Date).AddSeconds($PollTimeoutSeconds)
    do {
        if ($ManualSync) { Invoke-Checked -FilePath $cli -ArgumentList @('sync', $Drive, $indexPath) }
        $lines = Search-Lines $Needle
        if (-not ($lines | Where-Object { $_ -like "*$Needle*" })) { return }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    Write-Host "SOAK_DIAG_ABSENT needle=$Needle"
    & $cli 'doctor' $indexPath 2>&1 | ForEach-Object { Write-Host $_ }
    $svc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
    Write-Host ("SOAK_DIAG_SERVICE_QUERY exists={0} status={1}" -f [bool]$svc, $(if ($svc) { $svc.Status } else { 'none' }))
    if ($svc) {
        $info = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction SilentlyContinue
        Write-Host ("SOAK_DIAG_SERVICE status={0} pid={1} exit={2}" -f $svc.Status, $info.ProcessId, $info.ExitCode)
    }
    throw "Timed out waiting for '$Needle' to disappear"
}

$consoleService = $null
$started = Get-Date
$ops = 0
$checks = 0
$crashDone = $false
$servicePid = $null
$serviceCpuStart = $null
$serviceCpuAccumulated = 0.0
$servicePeakWorkingSet = 0L
$servicePeakPrivate = 0L
$lastVisible = $null
$cleaned = $false
try {
    if (-not $ManualSync) {
        $installed = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
        if (-not $installed -or $installed.Status -ne 'Running') {
            Write-Host '==> starting console service for soak'
            $consoleService = Start-Process -FilePath $serviceExe -ArgumentList @('--console', $Drive, $indexPath) -PassThru -WindowStyle Hidden
            Start-Sleep -Seconds 2
            if ($consoleService.HasExited) { throw "console service exited early: $($consoleService.ExitCode)" }
            $servicePid = $consoleService.Id
        } else {
            $serviceInfo = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction Stop
            $servicePid = [int]$serviceInfo.ProcessId
        }
        if ($servicePid) {
            $p = Get-Process -Id $servicePid -ErrorAction Stop
            $serviceCpuStart = $p.TotalProcessorTime.TotalSeconds
        }
        $startSvc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
        Write-Host ("SOAK_START service_exists={0} status={1} pid={2}" -f [bool]$startSvc, $(if ($startSvc) { $startSvc.Status } else { 'none' }), $servicePid)
    }

    $deadline = $started.AddMinutes([Math]::Max(1, $DurationMinutes))
    $generation = 0
    while ((Get-Date) -lt $deadline) {
        $generation++
        $names = [System.Collections.Generic.List[string]]::new()
        for ($i = 0; $i -lt [Math]::Max(4, $BatchSize); $i++) {
            $name = "soak-g{0:D6}-f{1:D4}.txt" -f $generation, $i
            Set-Content -LiteralPath (Join-Path $testRoot $name) -Value "search tool soak generation=$generation file=$i" -Encoding UTF8
            $names.Add($name)
            $ops++
        }
        for ($i = 0; $i -lt $names.Count; $i += 4) {
            $old = $names[$i]
            $new = $old.Replace('.txt', '-renamed.txt')
            Rename-Item -LiteralPath (Join-Path $testRoot $old) -NewName $new
            $names[$i] = $new
            $ops++
        }
        for ($i = 1; $i -lt $names.Count; $i += 4) {
            Remove-Item -LiteralPath (Join-Path $testRoot $names[$i]) -Force
            $ops++
        }

        $visible = $names[0]
        $lastVisible = $visible
        $deleted = "soak-g{0:D6}-f{1:D4}.txt" -f $generation, 1
        $liveProc = if ($servicePid) { Get-Process -Id $servicePid -ErrorAction SilentlyContinue } else { $null }
        if (-not $liveProc) {
            $liveSvc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
            Write-Host ("SOAK_SERVICE_LOST generation={0} service_exists={1} status={2} pid={3}" -f $generation, [bool]$liveSvc, $(if ($liveSvc) { $liveSvc.Status } else { 'none' }), $servicePid)
        }
        Wait-Visible $visible
        Wait-Absent $deleted
        $checks += 2

        if ($servicePid) {
            $p = Get-Process -Id $servicePid -ErrorAction SilentlyContinue
            if ($p) {
                $servicePeakWorkingSet = [Math]::Max($servicePeakWorkingSet, [long]$p.WorkingSet64)
                $servicePeakPrivate = [Math]::Max($servicePeakPrivate, [long]$p.PrivateMemorySize64)
            }
        }

        if ($CrashRestartService -and -not $crashDone -and (Get-Date) -ge $started.AddMinutes([Math]::Max(1, $DurationMinutes) / 2)) {
            if ($servicePid) {
                Write-Host '==> intentional service crash/restart'
                $oldProcess = Get-Process -Id $servicePid -ErrorAction SilentlyContinue
                if ($oldProcess -and $serviceCpuStart -ne $null) {
                    $serviceCpuAccumulated += [Math]::Max(0.0, $oldProcess.TotalProcessorTime.TotalSeconds - $serviceCpuStart)
                }
                Stop-Process -Id $servicePid -Force
                if ($consoleService) {
                    $consoleService.WaitForExit()
                    $consoleService = Start-Process -FilePath $serviceExe -ArgumentList @('--console', $Drive, $indexPath) -PassThru -WindowStyle Hidden
                    Start-Sleep -Seconds 2
                    if ($consoleService.HasExited) { throw "console service failed after restart: $($consoleService.ExitCode)" }
                    $servicePid = $consoleService.Id
                } else {
                    $deadlineRestart = (Get-Date).AddSeconds(20)
                    do {
                        Start-Sleep -Milliseconds 250
                        $svc = Get-Service -Name SearchToolIndexer -ErrorAction Stop
                        $svc.Refresh()
                        if ($svc.Status -eq 'Stopped') { break }
                    } while ((Get-Date) -lt $deadlineRestart)
                    Start-Service -Name SearchToolIndexer
                    do {
                        Start-Sleep -Milliseconds 250
                        $svc.Refresh()
                        if ($svc.Status -eq 'Running') { break }
                    } while ((Get-Date) -lt $deadlineRestart)
                    if ($svc.Status -ne 'Running') { throw "installed service failed to restart: $($svc.Status)" }
                    $serviceInfo = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction Stop
                    $servicePid = [int]$serviceInfo.ProcessId
                }
                $serviceCpuStart = (Get-Process -Id $servicePid -ErrorAction Stop).TotalProcessorTime.TotalSeconds
                $crashDone = $true
            }
        }

        if ($generation % 8 -eq 0) {
            Invoke-Checked -FilePath $cli -ArgumentList @('verify', $indexPath)
        }
    }

    if ($ManualSync) { Invoke-Checked -FilePath $cli -ArgumentList @('sync', $Drive, $indexPath) }
    Invoke-VerifyDeepEventually

    # Remove the workload while the sync mechanism is still alive so the test does
    # not leave stale Search Tool entries behind on a real validation machine.
    Remove-Item -LiteralPath $testRoot -Recurse -Force
    $cleaned = $true
    if ($lastVisible) { Wait-Absent $lastVisible }
    # Catch any stale entry from any generation, not just the final sentinel.
    Wait-Absent 'soak-g'
    Invoke-VerifyDeepEventually

    $elapsed = ((Get-Date) - $started).TotalSeconds
    $serviceCpuSeconds = $serviceCpuAccumulated
    if ($servicePid -and $serviceCpuStart -ne $null) {
        $p = Get-Process -Id $servicePid -ErrorAction SilentlyContinue
        if ($p) { $serviceCpuSeconds += [Math]::Max(0.0, $p.TotalProcessorTime.TotalSeconds - $serviceCpuStart) }
    }
    $serviceCpuPercent = if ($servicePid) {
        ($serviceCpuSeconds / [Math]::Max(0.001, $elapsed) / [Math]::Max(1, [Environment]::ProcessorCount)) * 100.0
    } else { $null }
    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        drive = $Drive
        index = $indexPath
        duration_seconds = [Math]::Round($elapsed, 2)
        operations = $ops
        validation_checks = $checks
        operations_per_second = [Math]::Round(($ops / [Math]::Max(0.001, $elapsed)), 2)
        crash_restart_exercised = $crashDone
        service_cpu_percent = if ($null -eq $serviceCpuPercent) { $null } else { [Math]::Round($serviceCpuPercent, 4) }
        service_peak_working_set_mib = if ($servicePeakWorkingSet -eq 0) { $null } else { [Math]::Round($servicePeakWorkingSet / 1MB, 3) }
        service_peak_private_mib = if ($servicePeakPrivate -eq 0) { $null } else { [Math]::Round($servicePeakPrivate / 1MB, 3) }
        mode = if ($ManualSync) { 'manual-sync' } else { 'service' }
        result = 'PASS'
    }
    $json = $report | ConvertTo-Json -Depth 4
    $json
    if ($OutputJson) {
        $parent = Split-Path -Parent $OutputJson
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
        Write-Host "Report=$OutputJson"
    }
} finally {
    if ($consoleService -and -not $consoleService.HasExited) {
        Stop-Process -Id $consoleService.Id -Force -ErrorAction SilentlyContinue
    }
    if (-not $cleaned) { Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue }
}
