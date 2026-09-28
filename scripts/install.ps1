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
    return "$letter`:"
}

function Stop-ExistingInstallation {
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

    $svc = Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue
    if ($svc) {
        if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
            Stop-Service -Name SearchToolIndexer -Force -ErrorAction Stop
            $svc.WaitForStatus([System.ServiceProcess.ServiceControllerStatus]::Stopped, [TimeSpan]::FromSeconds(20))
        }
        $svc.Close()
        & sc.exe delete SearchToolIndexer | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Failed to delete previous SearchToolIndexer service: $LASTEXITCODE" }
        $deadline = (Get-Date).AddSeconds(10)
        do {
            if (-not (Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue)) { break }
            Start-Sleep -Milliseconds 100
        } while ((Get-Date) -lt $deadline)
        if (Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue) {
            throw 'Previous SearchToolIndexer service is still pending deletion.'
        }
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
        ForEach-Object { "$($_.DriveLetter.ToString().ToUpperInvariant())`:" } |
        Sort-Object -Unique)

    if ($detected.Count -eq 0) {
        throw 'No visible fixed NTFS volumes were detected.'
    }
    return $detected
}

Assert-Admin
if ([string]::IsNullOrWhiteSpace($SourceDir)) {
    $portableCli = Join-Path $PSScriptRoot 'search-tool.exe'
    if (Test-Path -LiteralPath $portableCli) {
        $SourceDir = $PSScriptRoot
    } else {
        $SourceDir = (Resolve-Path (Join-Path $PSScriptRoot '..\target\release')).Path
    }
}

Stop-ExistingInstallation

$TargetDrives = @(Get-TargetDrives)
$IndexDir = Join-Path $DataDir 'index'
$BinNames = @('search-tool.exe', 'search-tool-gui.exe', 'search-tool-service.exe', 'search-tool-worker.exe')

New-Item -ItemType Directory -Force -Path $InstallDir, $IndexDir | Out-Null
foreach ($name in $BinNames) {
    $source = Join-Path $SourceDir $name
    if (-not (Test-Path -LiteralPath $source)) { throw "Missing release binary: $source" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $InstallDir $name) -Force
}

$modelSource = Join-Path $PSScriptRoot 'models\tiny-intent-v1.stm'
if (-not (Test-Path -LiteralPath $modelSource)) {
    $modelSource = Join-Path $PSScriptRoot '..\models\tiny-intent-v1.stm'
}
if (-not (Test-Path -LiteralPath $modelSource)) {
    throw "Missing required tiny intent model: $modelSource"
}
$modelDir = Join-Path $InstallDir 'models'
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
Copy-Item -LiteralPath $modelSource -Destination (Join-Path $modelDir 'tiny-intent-v1.stm') -Force

$service = Join-Path $InstallDir 'search-tool-service.exe'
$cli = Join-Path $InstallDir 'search-tool.exe'
$configPath = Join-Path $DataDir 'service.conf'
$configPointer = Join-Path $InstallDir 'service.conf.path'
[IO.File]::WriteAllText($configPointer, $configPath, [Text.UTF8Encoding]::new($false))
foreach ($targetDrive in $TargetDrives) {
    $letter = $targetDrive.Substring(0, 1)
    $indexPath = Join-Path $IndexDir ($letter + '.stidx')
    if (-not $SkipInitialIndex) {
        Write-Host "Building initial $targetDrive index..."
        & $cli index $targetDrive $indexPath
        if ($LASTEXITCODE -ne 0) {
            throw "Initial index build for $targetDrive failed with exit code $LASTEXITCODE"
        }
    } elseif (-not (Test-Path -LiteralPath $indexPath)) {
        Write-Warning "Initial index skipped and no index exists at $indexPath. That volume will be unavailable until an index is built."
    }
}

$driveList = (($TargetDrives | ForEach-Object { $_.Substring(0, 1) }) -join ',')
& $service --install-multi $IndexDir $driveList
if ($LASTEXITCODE -ne 0) { throw "Service installation failed with exit code $LASTEXITCODE" }
& $service --start
if ($LASTEXITCODE -ne 0) { throw "Service start failed with exit code $LASTEXITCODE" }

# Keep the native GUI launcher resident with negligible idle CPU so the global
# hotkey remains available. Passing the index directory enables all volumes.
$startup = [Environment]::GetFolderPath('Startup')
if ($startup) {
    $shortcutPath = Join-Path $startup 'Search Tool.lnk'
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $InstallDir 'search-tool-gui.exe'
    $shortcut.Arguments = ('"{0}" --resident' -f $IndexDir)
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.Save()
}

Write-Host 'Search Tool installed.'
Write-Host "InstallDir=$InstallDir"
Write-Host "IndexDir=$IndexDir"
Write-Host "Volumes=$($TargetDrives -join ',')"
