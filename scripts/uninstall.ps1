[CmdletBinding()]
param(
    [string]$InstallDir = [IO.Path]::Combine(
        [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles),
        'Search Tool'
    ),
    [string]$DataDir = [IO.Path]::Combine(
        [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonApplicationData),
        'SearchTool'
    ),
    [string]$ServiceName = 'SearchToolIndexer',
    [switch]$PurgeData
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($ServiceName) -or
    $ServiceName.Length -gt 256 -or
    $ServiceName -match '[\\/"]') {
    throw "Invalid Windows service name: '$ServiceName'"
}

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Search Tool uninstall requires an elevated PowerShell session.'
    }
}

function Stop-InstalledGui {
    $target = [IO.Path]::GetFullPath((Join-Path $InstallDir 'search-tool-gui.exe'))
    foreach ($process in @(Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue)) {
        $processId = $process.Id
        try { $processPath = $process.Path } catch { continue }
        if (-not $processPath -or [IO.Path]::GetFullPath($processPath) -ine $target) { continue }

        # The GUI may exit by itself between enumeration and Stop-Process. Treat
        # that race as success; uninstall only fails if the matching process is
        # still alive after the bounded shutdown window.
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
        $deadline = (Get-Date).AddSeconds(10)
        do {
            $remaining = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if (-not $remaining) { break }
            Start-Sleep -Milliseconds 50
        } while ((Get-Date) -lt $deadline)
        if (Get-Process -Id $processId -ErrorAction SilentlyContinue) {
            throw "Failed to stop installed Search Tool GUI process $processId within 10 seconds."
        }
    }
}

function Remove-InstalledService {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) { return }
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
        Stop-Service -Name $ServiceName -Force -ErrorAction Stop
        $svc.WaitForStatus([System.ServiceProcess.ServiceControllerStatus]::Stopped, [TimeSpan]::FromSeconds(20))
    }
    $svc.Close()
    & sc.exe delete $ServiceName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to delete $ServiceName service: $LASTEXITCODE" }
    $deadline = (Get-Date).AddSeconds(10)
    do {
        if (-not (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) { return }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)
    throw "$ServiceName service is still pending deletion."
}

Assert-Admin
Stop-InstalledGui
Remove-InstalledService

$startup = [Environment]::GetFolderPath('Startup')
if ($startup) {
    Remove-Item -LiteralPath (Join-Path $startup 'Search Tool.lnk') -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $InstallDir) {
    Remove-Item -LiteralPath $InstallDir -Recurse -Force
}

if ($PurgeData -and (Test-Path -LiteralPath $DataDir)) {
    Remove-Item -LiteralPath $DataDir -Recurse -Force
} elseif (Test-Path -LiteralPath $DataDir) {
    Write-Host "Preserved data/index at $DataDir (use -PurgeData to remove it)."
}

Write-Host 'Search Tool uninstalled.'
