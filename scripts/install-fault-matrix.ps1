[CmdletBinding()]
param(
    [string]$SourceDir = '',
    [string]$Root = 'C:\SearchToolLab\install-fault-matrix',
    [string]$ServiceName = 'SearchToolIndexerFaultTest',
    [ValidatePattern('^[A-Za-z]:$')]
    [string]$Drive = 'C:',
    [string]$EvidencePath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$InstallScript = Join-Path $PSScriptRoot 'install.ps1'
if ([string]::IsNullOrWhiteSpace($SourceDir)) {
    $SourceDir = (Resolve-Path (Join-Path $RepoRoot 'target\release')).Path
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $RepoRoot 'docs\evidence\install-transaction-fault-matrix-20260929.json'
}
$InstallDir = Join-Path $Root 'install'
$DataDir = Join-Path $Root 'data'
$ConfigPath = Join-Path $DataDir 'service.conf'
$IndexMarker = Join-Path $DataDir 'index\fault-matrix-marker.txt'
$UpgradeMarker = "$InstallDir.upgrade.json"
$StageDir = "$InstallDir.new"
$BackupDir = "$InstallDir.old"
$ConfigBackup = "$ConfigPath.upgrade-backup"
$ShortcutRoot = Join-Path $Root 'shortcuts'
$StartupShortcutDir = Join-Path $ShortcutRoot 'startup'
$ProgramsShortcutDir = Join-Path $ShortcutRoot 'programs'
$StartupShortcut = Join-Path $StartupShortcutDir 'Search Tool.lnk'
$ProgramsShortcut = Join-Path $ProgramsShortcutDir 'Search Tool.lnk'
$Faults = [ordered]@{
    'after-stage-validated' = 'staged'
    'after-service-removed' = 'old-service-removed'
    'after-live-renamed' = 'live-renamed'
    'after-new-published' = 'new-published'
    'after-service-installed' = 'service-installed'
    'before-service-start' = 'service-installed'
    'after-service-started' = 'service-started'
    'before-user-artifacts' = 'service-started'
    'after-shortcuts' = 'service-started'
    'after-integration' = 'service-started'
}

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Install fault matrix requires an elevated PowerShell session.'
    }
}

function Get-ServiceSnapshot([string]$Name) {
    $escaped = $Name.Replace("'", "''")
    $service = Get-CimInstance Win32_Service -Filter "Name='$escaped'" -ErrorAction SilentlyContinue
    if (-not $service) { return $null }
    [ordered]@{
        state = [string]$service.State
        start_mode = [string]$service.StartMode
        path_name = [string]$service.PathName
        process_id = [uint32]$service.ProcessId
    }
}
function Remove-TestService {
    $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $service) { return }
    if ($service.Status -ne 'Stopped') {
        Stop-Service -Name $ServiceName -Force -ErrorAction Stop
        $service.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(20))
    }
    $service.Close()
    & sc.exe delete $ServiceName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to delete isolated service $ServiceName."
    }
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) { return }
        Start-Sleep -Milliseconds 100
    }
    throw "Isolated service $ServiceName is still pending deletion."
}

function Invoke-InstallerProcess([string]$Fault = '', [switch]$RecoverOnly) {
    $oldFault = $env:SEARCH_TOOL_INSTALL_FAULT
    $oldMode = $env:SEARCH_TOOL_INSTALL_FAULT_MODE
    try {
        if ($Fault) {
            $env:SEARCH_TOOL_INSTALL_FAULT = $Fault
            $env:SEARCH_TOOL_INSTALL_FAULT_MODE = 'exit'
        } else {
            Remove-Item Env:SEARCH_TOOL_INSTALL_FAULT -ErrorAction SilentlyContinue
            Remove-Item Env:SEARCH_TOOL_INSTALL_FAULT_MODE -ErrorAction SilentlyContinue
        }
        $arguments = @(
            '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass',
            '-File', $InstallScript,
            '-Drive', $Drive,
            '-SourceDir', $SourceDir,
            '-InstallDir', $InstallDir,
            '-DataDir', $DataDir,
            '-ServiceName', $ServiceName,
            '-StartupShortcutDir', $StartupShortcutDir,
            '-ProgramsShortcutDir', $ProgramsShortcutDir,
            '-SkipInitialIndex'
        )
        if ($RecoverOnly) { $arguments += '-RecoverOnly' }
        & powershell.exe @arguments | Out-Host
        return [int]$LASTEXITCODE
    } finally {
        if ($null -eq $oldFault) {
            Remove-Item Env:SEARCH_TOOL_INSTALL_FAULT -ErrorAction SilentlyContinue
        } else {
            $env:SEARCH_TOOL_INSTALL_FAULT = $oldFault
        }
        if ($null -eq $oldMode) {
            Remove-Item Env:SEARCH_TOOL_INSTALL_FAULT_MODE -ErrorAction SilentlyContinue
        } else {
            $env:SEARCH_TOOL_INSTALL_FAULT_MODE = $oldMode
        }
    }
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
}
function Assert-BaselineRestored {
    if ((Get-Sha256 $BaselineCli) -ne $BaselineCliHash) {
        throw 'Rollback did not restore the previous CLI binary.'
    }
    if ((Get-Sha256 $ConfigPath) -ne $BaselineConfigHash) {
        throw 'Rollback did not restore the previous service config.'
    }
    if ((Get-Sha256 $IndexMarker) -ne $BaselineIndexHash) {
        throw 'Rollback changed the isolated index marker.'
    }
    if (-not (Test-Path -LiteralPath $StartupShortcut -PathType Leaf) -or
        (Get-Sha256 $StartupShortcut) -ne $BaselineStartupShortcutHash) {
        throw 'Rollback did not restore the previous Startup shortcut bytes.'
    }
    if (-not (Test-Path -LiteralPath $ProgramsShortcut -PathType Leaf) -or
        (Get-Sha256 $ProgramsShortcut) -ne $BaselineProgramsShortcutHash) {
        throw 'Rollback did not restore the previous Start Menu shortcut bytes.'
    }

    $isolated = Get-ServiceSnapshot $ServiceName
    if (-not $isolated) { throw 'Isolated service registration was not restored.' }
    if ($isolated.state -ne 'Running' -or $isolated.start_mode -ne 'Auto') {
        throw "Isolated service state was not restored: $($isolated.state)/$($isolated.start_mode)"
    }
    if ($isolated.path_name -notlike "*$InstallDir*") {
        throw "Isolated service points outside the test install: $($isolated.path_name)"
    }

    foreach ($debris in @($StageDir, $BackupDir, $UpgradeMarker, $ConfigBackup)) {
        if (Test-Path -LiteralPath $debris) {
            throw "Rollback debris remains: $debris"
        }
    }

    $liveNow = Get-ServiceSnapshot 'SearchToolIndexer'
    if (($null -eq $LiveBefore) -ne ($null -eq $liveNow)) {
        throw 'Default SearchToolIndexer registration changed during isolated test.'
    }
    if ($LiveBefore) {
        foreach ($field in @('state', 'start_mode', 'path_name', 'process_id')) {
            if ($liveNow.$field -ne $LiveBefore.$field) {
                throw "Default SearchToolIndexer changed field '$field'."
            }
        }
    }
}

Assert-Admin
foreach ($binary in @(
    'search-tool.exe',
    'search-tool-gui.exe',
    'search-tool-service.exe',
    'search-tool-worker.exe'
)) {
    $path = Join-Path $SourceDir $binary
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Release binary missing: $path"
    }
}

$LiveBefore = Get-ServiceSnapshot 'SearchToolIndexer'
$CaseResults = @()
$Failure = $null
$CleanupError = $null
$Result = 'FAIL'
$BaselineCli = Join-Path $InstallDir 'search-tool.exe'
$BaselineCliHash = $null
$BaselineConfigHash = $null
$BaselineIndexHash = $null
$BaselineStartupShortcutHash = $null
$BaselineProgramsShortcutHash = $null

try {
    Remove-TestService
    if (Test-Path -LiteralPath $Root) {
        Remove-Item -LiteralPath $Root -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Root | Out-Null

    $baselineExit = Invoke-InstallerProcess
    if ($baselineExit -ne 0) {
        throw "Baseline isolated install failed with exit code $baselineExit."
    }

    $isolated = Get-ServiceSnapshot $ServiceName
    if (-not $isolated -or $isolated.state -ne 'Running') {
        throw 'Baseline isolated service did not reach Running state.'
    }

    $token = [Guid]::NewGuid().ToString('N')
    foreach ($entry in @(
        @{ Path = $StartupShortcut; Arguments = "legacy-startup-$token" },
        @{ Path = $ProgramsShortcut; Arguments = "legacy-programs-$token" }
    )) {
        if (-not (Test-Path -LiteralPath $entry.Path -PathType Leaf)) {
            throw "Baseline shortcut missing: $($entry.Path)"
        }
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($entry.Path)
        $shortcut.Arguments = [string]$entry.Arguments
        $shortcut.Description = "fault-matrix-legacy-$token"
        $shortcut.Save()
    }
    [IO.File]::AppendAllText(
        $BaselineCli,
        [Environment]::NewLine + "FAULT_MATRIX_PREVIOUS_BINARY=$token",
        [Text.UTF8Encoding]::new($false)
    )
    Add-Content -LiteralPath $ConfigPath -Value "# fault-matrix-previous-config=$token"
    $markerParent = Split-Path -Parent $IndexMarker
    New-Item -ItemType Directory -Force -Path $markerParent | Out-Null
    [IO.File]::WriteAllText(
        $IndexMarker,
        "fault-matrix-index=$token",
        [Text.UTF8Encoding]::new($false)
    )
    $BaselineCliHash = Get-Sha256 $BaselineCli
    $BaselineConfigHash = Get-Sha256 $ConfigPath
    $BaselineIndexHash = Get-Sha256 $IndexMarker
    $BaselineStartupShortcutHash = Get-Sha256 $StartupShortcut
    $BaselineProgramsShortcutHash = Get-Sha256 $ProgramsShortcut
    Assert-BaselineRestored

    foreach ($entry in $Faults.GetEnumerator()) {
        $fault = [string]$entry.Key
        $expectedPhase = [string]$entry.Value
        $started = Get-Date
        $faultExit = Invoke-InstallerProcess -Fault $fault
        if ($faultExit -eq 0) {
            throw "Fault '$fault' unexpectedly returned success."
        }
        if (-not (Test-Path -LiteralPath $UpgradeMarker -PathType Leaf)) {
            throw "Fault '$fault' did not leave an upgrade recovery marker."
        }

        $interrupted = Get-Content -Raw -LiteralPath $UpgradeMarker | ConvertFrom-Json
        $actualPhase = [string]$interrupted.phase
        if ($actualPhase -ne $expectedPhase) {
            throw "Fault '$fault' stopped at '$actualPhase', expected '$expectedPhase'."
        }
        $interruptedService = Get-ServiceSnapshot $ServiceName

        if ($fault -in @('after-shortcuts', 'after-integration')) {
            if ((Get-Sha256 $StartupShortcut) -eq $BaselineStartupShortcutHash) {
                throw "Fault '$fault' did not replace the Startup shortcut before recovery."
            }
            if ((Get-Sha256 $ProgramsShortcut) -eq $BaselineProgramsShortcutHash) {
                throw "Fault '$fault' did not replace the Start Menu shortcut before recovery."
            }
        }

        $recoveryExit = Invoke-InstallerProcess -RecoverOnly
        if ($recoveryExit -ne 0) {
            throw "Recovery after '$fault' failed with exit code $recoveryExit."
        }
        Assert-BaselineRestored

        $CaseResults += [ordered]@{
            fault = $fault
            interrupted_phase = $actualPhase
            injected_exit_code = $faultExit
            interrupted_service_state = if ($interruptedService) { $interruptedService.state } else { 'Absent' }
            recovery_exit_code = $recoveryExit
            elapsed_ms = [int]((Get-Date) - $started).TotalMilliseconds
            result = 'PASS'
        }
        Write-Host "PASS fault=$fault phase=$actualPhase"
    }

    $Result = 'PASS'
} catch {
    $Failure = $_.Exception.Message
    $Result = 'FAIL'
} finally {
    $LiveAfter = Get-ServiceSnapshot 'SearchToolIndexer'
    try {
        Remove-TestService
        if (Test-Path -LiteralPath $Root) {
            Remove-Item -LiteralPath $Root -Recurse -Force
        }
    } catch {
        $CleanupError = $_.Exception.Message
        $Result = 'FAIL'
        if (-not $Failure) { $Failure = "Cleanup failed: $CleanupError" }
    }

    $head = (& git -C $RepoRoot rev-parse HEAD 2>$null)
    $evidence = [ordered]@{
        schema = 1
        generated_utc = [DateTime]::UtcNow.ToString('o')
        source_head = [string]$head
        result = $Result
        service_name = $ServiceName
        source_dir = $SourceDir
        drive = $Drive
        baseline_cli_sha256 = $BaselineCliHash
        baseline_config_sha256 = $BaselineConfigHash
        baseline_index_marker_sha256 = $BaselineIndexHash
        baseline_startup_shortcut_sha256 = $BaselineStartupShortcutHash
        baseline_programs_shortcut_sha256 = $BaselineProgramsShortcutHash
        live_service_before = $LiveBefore
        live_service_after = $LiveAfter
        cases = @($CaseResults)
        failure = $Failure
        cleanup_error = $CleanupError
    }
    $evidenceParent = Split-Path -Parent $EvidencePath
    if ($evidenceParent) {
        New-Item -ItemType Directory -Force -Path $evidenceParent | Out-Null
    }
    $json = $evidence | ConvertTo-Json -Depth 10
    [IO.File]::WriteAllText($EvidencePath, $json, [Text.UTF8Encoding]::new($false))
}

if ($Result -ne 'PASS') {
    throw "Install fault matrix failed: $Failure"
}

Write-Host "install_fault_matrix=PASS"
Write-Host "cases=$($CaseResults.Count)"
Write-Host "evidence=$EvidencePath"
