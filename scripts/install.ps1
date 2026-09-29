[CmdletBinding()]
param(
    [ValidatePattern('^$|^[A-Za-z]:?$')]
    [string]$Drive = '',
    [string[]]$Drives = @(),
    [string]$SourceDir = '',
    [string]$InstallDir = "$env:ProgramFiles\Search Tool",
    [string]$DataDir = "$env:ProgramData\SearchTool",
    [switch]$SkipInitialIndex
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$StageDir = "$InstallDir.new"
$BackupDir = "$InstallDir.old"
$MarkerPath = "$InstallDir.upgrade.json"
$ConfigPath = Join-Path $DataDir 'service.conf'
$ConfigBackupPath = "$ConfigPath.upgrade-backup"
$BinNames = @(
    'search-tool.exe',
    'search-tool-gui.exe',
    'search-tool-service.exe',
    'search-tool-worker.exe'
)

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Search Tool installation requires an elevated PowerShell session.'
    }
}

function Normalize-Drive([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    $letter = $Value.Substring(0, 1).ToUpperInvariant()
    if ($letter -notmatch '^[A-Z]$') { throw "Invalid drive: $Value" }
    return ($letter + ':')
}

function Assert-ExistingDataDirCompatible {
    $pointer = Join-Path $InstallDir 'service.conf.path'
    if (-not (Test-Path -LiteralPath $pointer -PathType Leaf)) { return }

    $raw = (Get-Content -Raw -LiteralPath $pointer).Trim().TrimStart([char]0xFEFF)
    if ([string]::IsNullOrWhiteSpace($raw)) {
        throw "Existing service.conf.path is empty: $pointer"
    }
    $actual = [IO.Path]::GetFullPath($raw)
    $expected = [IO.Path]::GetFullPath($ConfigPath)
    if ($actual -ine $expected) {
        throw "Existing install uses config '$actual'; rerun with the matching -DataDir before upgrading."
    }
}

function Get-TargetDrives {
    $requested = @()
    if ($Drives.Count -gt 0) {
        $requested = $Drives
    } elseif (-not [string]::IsNullOrWhiteSpace($Drive)) {
        $requested = @($Drive)
    }
    if ($requested.Count -gt 0) {
        return @($requested | ForEach-Object { Normalize-Drive $_ } | Sort-Object -Unique)
    }

    $detected = @(Get-Volume -ErrorAction Stop |
        Where-Object {
            $_.DriveLetter -and
            $_.FileSystemType -eq 'NTFS' -and
            $_.DriveType -eq 'Fixed'
        } |
        ForEach-Object { $_.DriveLetter.ToString().ToUpperInvariant() + ':' } |
        Sort-Object -Unique)
    if ($detected.Count -eq 0) {
        throw 'No visible fixed NTFS volumes were detected.'
    }
    return $detected
}

function Wait-ServiceDeletion {
    $deadline = (Get-Date).AddSeconds(15)
    do {
        if (-not (Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue)) { return }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)
    throw 'SearchToolIndexer is still pending deletion.'
}

function Stop-InstalledGui {
    if (-not (Test-Path -LiteralPath $InstallDir)) { return }
    $guiTarget = [IO.Path]::GetFullPath((Join-Path $InstallDir 'search-tool-gui.exe'))
    foreach ($process in @(Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue)) {
        try {
            if ($process.Path -and [IO.Path]::GetFullPath($process.Path) -ieq $guiTarget) {
                Stop-Process -Id $process.Id -Force -ErrorAction Stop
                $process.WaitForExit(10000) | Out-Null
            }
        } catch {
            throw "Failed to stop installed Search Tool GUI process $($process.Id): $($_.Exception.Message)"
        }
    }
}

function Remove-ServiceRegistration {
    $svc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
    if (-not $svc) { return }
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
        Stop-Service -Name SearchToolIndexer -Force -ErrorAction Stop
        $svc.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Stopped,
            [TimeSpan]::FromSeconds(20)
        )
    }
    $svc.Close()
    & sc.exe delete SearchToolIndexer | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to delete SearchToolIndexer service: $LASTEXITCODE"
    }
    Wait-ServiceDeletion
}

function Register-ExistingService([bool]$StartAfterRegister) {
    $serviceExe = Join-Path $InstallDir 'search-tool-service.exe'
    if (-not (Test-Path -LiteralPath $serviceExe)) {
        throw "Rollback service binary is missing: $serviceExe"
    }
    $quotedPath = '"' + $serviceExe + '"'
    & sc.exe create SearchToolIndexer binPath= $quotedPath start= auto DisplayName= 'Search Tool Indexer' | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to restore SearchToolIndexer registration: $LASTEXITCODE"
    }
    if ($StartAfterRegister) {
        Start-Service -Name SearchToolIndexer
        $svc = Get-Service -Name SearchToolIndexer
        $svc.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds(20)
        )
    }
}

function Write-UpgradePhase([System.Collections.IDictionary]$State, [string]$Phase) {
    $State['phase'] = $Phase
    $tmp = "$MarkerPath.tmp"
    $json = $State | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText($tmp, $json, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $tmp -Destination $MarkerPath -Force
}

function Invoke-FaultPoint([string]$Name) {
    if ($env:SEARCH_TOOL_INSTALL_FAULT -eq $Name) {
        throw "Injected installer fault at $Name"
    }
}

function Remove-PathIfPresent([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Restore-PreviousInstallation($State) {
    if ([string]$State.phase -eq 'staged') {
        Remove-PathIfPresent $StageDir
        if (Test-Path -LiteralPath $ConfigBackupPath) {
            Remove-Item -LiteralPath $ConfigBackupPath -Force
        }
        if (Test-Path -LiteralPath $MarkerPath) {
            Remove-Item -LiteralPath $MarkerPath -Force
        }
        return
    }

    Stop-InstalledGui
    Remove-ServiceRegistration

    if (Test-Path -LiteralPath $BackupDir) {
        Remove-PathIfPresent $InstallDir
        Move-Item -LiteralPath $BackupDir -Destination $InstallDir
    } elseif (-not [bool]$State.had_live_install) {
        Remove-PathIfPresent $InstallDir
    } elseif (-not (Test-Path -LiteralPath $InstallDir)) {
        throw 'Rollback cannot find the previous installation directory.'
    }

    if ([bool]$State.config_existed) {
        if (-not (Test-Path -LiteralPath $ConfigBackupPath)) {
            throw 'Rollback config backup is missing.'
        }
        New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
        Copy-Item -LiteralPath $ConfigBackupPath -Destination $ConfigPath -Force
    } elseif (Test-Path -LiteralPath $ConfigPath) {
        Remove-Item -LiteralPath $ConfigPath -Force
    }

    if ([bool]$State.previous_service_exists) {
        Register-ExistingService ([bool]$State.previous_service_running)
    }

    Remove-PathIfPresent $StageDir
    if (Test-Path -LiteralPath $ConfigBackupPath) {
        Remove-Item -LiteralPath $ConfigBackupPath -Force
    }
    if (Test-Path -LiteralPath $MarkerPath) {
        Remove-Item -LiteralPath $MarkerPath -Force
    }
}

function Recover-InterruptedUpgrade {
    if (-not (Test-Path -LiteralPath $MarkerPath)) {
        if (Test-Path -LiteralPath $BackupDir) {
            throw "Orphaned upgrade backup exists without marker: $BackupDir"
        }
        Remove-PathIfPresent $StageDir
        return
    }

    $state = Get-Content -Raw -LiteralPath $MarkerPath | ConvertFrom-Json
    if ($state.phase -eq 'committed') {
        Remove-PathIfPresent $BackupDir
        Remove-PathIfPresent $StageDir
        if (Test-Path -LiteralPath $ConfigBackupPath) {
            Remove-Item -LiteralPath $ConfigBackupPath -Force
        }
        Remove-Item -LiteralPath $MarkerPath -Force
        return
    }
    Write-Warning "Recovering interrupted Search Tool upgrade at phase '$($state.phase)'."
    Restore-PreviousInstallation $state
}

function Validate-StagedPayload([string]$Stage) {
    foreach ($name in $BinNames) {
        $path = Join-Path $Stage $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Staged binary missing: $path"
        }
        if ((Get-Item -LiteralPath $path).Length -le 0) {
            throw "Staged binary is empty: $path"
        }
    }
    $model = Join-Path $Stage 'models\tiny-intent-v1.stm'
    if (-not (Test-Path -LiteralPath $model -PathType Leaf)) {
        throw "Staged model missing: $model"
    }
    $version = @(& (Join-Path $Stage 'search-tool.exe') version 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0 -or -not ($version | Where-Object { $_ -match '^search-tool\s+' })) {
        throw "Staged CLI validation failed: $($version -join ' | ')"
    }
}

function New-SearchToolShortcut([string]$IndexDir) {
    $startup = [Environment]::GetFolderPath('Startup')
    if (-not $startup) { return }
    $shortcutPath = Join-Path $startup 'Search Tool.lnk'
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $InstallDir 'search-tool-gui.exe'
    $shortcut.Arguments = ('"{0}" --resident' -f $IndexDir)
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.Save()
}

Assert-Admin
Assert-ExistingDataDirCompatible
Recover-InterruptedUpgrade

if ([string]::IsNullOrWhiteSpace($SourceDir)) {
    $portableCli = Join-Path $PSScriptRoot 'search-tool.exe'
    if (Test-Path -LiteralPath $portableCli) {
        $SourceDir = $PSScriptRoot
    } else {
        $SourceDir = (Resolve-Path (Join-Path $PSScriptRoot '..\target\release')).Path
    }
}

$TargetDrives = @(Get-TargetDrives)
$IndexDir = Join-Path $DataDir 'index'
$parent = Split-Path -Parent $InstallDir
if ([string]::IsNullOrWhiteSpace($parent)) {
    throw "InstallDir must have a parent directory: $InstallDir"
}
New-Item -ItemType Directory -Force -Path $parent, $DataDir, $IndexDir | Out-Null

Remove-PathIfPresent $StageDir
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
foreach ($name in $BinNames) {
    $source = Join-Path $SourceDir $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing release binary: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $StageDir $name)
}

$modelSource = Join-Path $PSScriptRoot 'models\tiny-intent-v1.stm'
if (-not (Test-Path -LiteralPath $modelSource)) {
    $modelSource = Join-Path $PSScriptRoot '..\models\tiny-intent-v1.stm'
}
if (-not (Test-Path -LiteralPath $modelSource -PathType Leaf)) {
    throw "Missing required tiny intent model: $modelSource"
}
$modelDir = Join-Path $StageDir 'models'
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
Copy-Item -LiteralPath $modelSource -Destination (Join-Path $modelDir 'tiny-intent-v1.stm')

$configPointer = Join-Path $StageDir 'service.conf.path'
[IO.File]::WriteAllText($configPointer, $ConfigPath, [Text.UTF8Encoding]::new($false))
Validate-StagedPayload $StageDir

$previousService = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
$state = [ordered]@{
    version = 1
    phase = 'staged'
    created_utc = [DateTime]::UtcNow.ToString('o')
    had_live_install = [bool](Test-Path -LiteralPath $InstallDir)
    previous_service_exists = [bool]$previousService
    previous_service_running = [bool]($previousService -and $previousService.Status -eq 'Running')
    config_existed = [bool](Test-Path -LiteralPath $ConfigPath)
    install_dir = $InstallDir
    stage_dir = $StageDir
    backup_dir = $BackupDir
    data_dir = $DataDir
    index_dir = $IndexDir
    target_drives = @($TargetDrives)
}
if ($state.config_existed) {
    Copy-Item -LiteralPath $ConfigPath -Destination $ConfigBackupPath -Force
} elseif (Test-Path -LiteralPath $ConfigBackupPath) {
    Remove-Item -LiteralPath $ConfigBackupPath -Force
}
Write-UpgradePhase $state 'staged'

try {
    Invoke-FaultPoint 'after-stage-validated'

    $stagedCli = Join-Path $StageDir 'search-tool.exe'
    foreach ($targetDrive in $TargetDrives) {
        $letter = $targetDrive.Substring(0, 1)
        $indexPath = Join-Path $IndexDir ($letter + '.stidx')
        if (Test-Path -LiteralPath $indexPath) {
            Write-Host "Preserving existing $targetDrive index: $indexPath"
            continue
        }
        if ($SkipInitialIndex) {
            Write-Warning "Initial index skipped and no index exists at $indexPath."
            continue
        }
        Write-Host "Building initial $targetDrive index..."
        & $stagedCli index $targetDrive $indexPath
        if ($LASTEXITCODE -ne 0) {
            throw "Initial index build for $targetDrive failed with exit code $LASTEXITCODE"
        }
    }

    Stop-InstalledGui
    Remove-ServiceRegistration
    Write-UpgradePhase $state 'old-service-removed'
    Invoke-FaultPoint 'after-service-removed'

    Remove-PathIfPresent $BackupDir
    if (Test-Path -LiteralPath $InstallDir) {
        Move-Item -LiteralPath $InstallDir -Destination $BackupDir
    }
    Write-UpgradePhase $state 'live-renamed'
    Invoke-FaultPoint 'after-live-renamed'

    Move-Item -LiteralPath $StageDir -Destination $InstallDir
    Write-UpgradePhase $state 'new-published'
    Invoke-FaultPoint 'after-new-published'

    $service = Join-Path $InstallDir 'search-tool-service.exe'
    $driveList = (($TargetDrives | ForEach-Object { $_.Substring(0, 1) }) -join ',')
    & $service --install-multi $IndexDir $driveList
    if ($LASTEXITCODE -ne 0) {
        throw "Service installation failed with exit code $LASTEXITCODE"
    }
    Write-UpgradePhase $state 'service-installed'
    Invoke-FaultPoint 'after-service-installed'
    Invoke-FaultPoint 'before-service-start'

    & $service --start
    if ($LASTEXITCODE -ne 0) {
        throw "Service start failed with exit code $LASTEXITCODE"
    }
    Write-UpgradePhase $state 'service-started'
    Invoke-FaultPoint 'after-service-started'

    New-SearchToolShortcut $IndexDir
    Write-UpgradePhase $state 'committed'
} catch {
    $installError = $_
    try {
        Restore-PreviousInstallation $state
    } catch {
        throw "Upgrade failed: $($installError.Exception.Message); rollback also failed: $($_.Exception.Message). Recovery marker preserved at $MarkerPath"
    }
    throw $installError
}

try {
    Remove-PathIfPresent $BackupDir
    if (Test-Path -LiteralPath $ConfigBackupPath) {
        Remove-Item -LiteralPath $ConfigBackupPath -Force
    }
    Remove-Item -LiteralPath $MarkerPath -Force
} catch {
    Write-Warning "Install committed but cleanup is deferred to the next installer run: $($_.Exception.Message)"
}

Write-Host 'Search Tool installed.'
Write-Host "InstallDir=$InstallDir"
Write-Host "IndexDir=$IndexDir"
Write-Host "Volumes=$($TargetDrives -join ',')"
