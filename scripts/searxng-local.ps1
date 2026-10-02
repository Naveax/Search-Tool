[CmdletBinding()]
param(
    [ValidateSet('Start','Stop','Status')]
    [string]$Action = 'Start',
    [ValidateSet('Auto','Wsl','Docker')]
    [string]$Backend = 'Auto',
    [ValidateRange(1024, 65535)]
    [int]$Port = 8888,
    [string]$WslDistro = 'Ubuntu-24.04',
    [string]$ContainerName = 'search-tool-searxng',
    [switch]$Pull
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$stateDir = Join-Path $root '.searxng-local'
$configDir = Join-Path $stateDir 'config'
$dataDir = Join-Path $stateDir 'data'
$settingsPath = Join-Path $configDir 'settings.yml'
$image = 'docker.io/searxng/searxng:latest'
$wslPython = '/usr/local/searxng/searx-pyenv/bin/python'
$wslSource = '/usr/local/searxng/searxng-src'
$wslSettings = '/etc/searxng/settings.yml'

function New-SecretHex {
    $bytes = New-Object byte[] 32
    $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $rng.GetBytes($bytes)
    } finally {
        $rng.Dispose()
    }
    return (($bytes | ForEach-Object { $_.ToString('x2') }) -join '')
}

function Test-HttpReady {
    try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/config" -TimeoutSec 2
        return ($response.StatusCode -eq 200)
    } catch {
        return $false
    }
}

function Wait-Ready {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        if (Test-HttpReady) { return }
        Start-Sleep -Milliseconds 750
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "SearXNG did not become ready on localhost:$Port within 45 seconds."
}

function Write-ConnectionInfo([string]$SelectedBackend) {
    Write-Host "backend=$SelectedBackend"
    Write-Host "SearXNG=http://127.0.0.1:$Port"
    Write-Host "Set SEARCH_TOOL_SEARXNG_URL=http://127.0.0.1:$Port for Search Tool."
}
function Test-DockerReady {
    if (-not (Get-Command docker.exe -ErrorAction SilentlyContinue)) {
        return $false
    }
    & docker.exe info --format '{{.ServerVersion}}' *> $null
    return ($LASTEXITCODE -eq 0)
}

function Ensure-DockerConfig {
    New-Item -ItemType Directory -Force -Path $configDir, $dataDir | Out-Null
    if (Test-Path -LiteralPath $settingsPath -PathType Leaf) { return }

    $secret = New-SecretHex
    @"
use_default_settings: true

search:
  safe_search: 1
  formats:
    - html
    - json

server:
  secret_key: "$secret"
  limiter: false
  image_proxy: false
"@ | Set-Content -LiteralPath $settingsPath -Encoding UTF8
}

function Test-ContainerExists {
    & docker.exe container inspect $ContainerName *> $null
    return ($LASTEXITCODE -eq 0)
}

function Start-DockerBackend {
    if (-not (Test-DockerReady)) {
        throw 'Docker engine is not ready.'
    }
    Ensure-DockerConfig
    if ($Pull) {
        & docker.exe pull $image | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Failed to pull SearXNG image.' }
    }

    if (Test-ContainerExists) {
        & docker.exe start $ContainerName | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Failed to start SearXNG container.' }
    } else {
        $configMount = $configDir + ':/etc/searxng:rw'
        $dataMount = $dataDir + ':/var/cache/searxng:rw'
        $publish = "127.0.0.1:$($Port):8080"
        $args = @('run','--name',$ContainerName,'-d','-p',$publish,'-v',$configMount,'-v',$dataMount,$image)
        & docker.exe @args | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Failed to create SearXNG container.' }
    }
    Wait-Ready
}
function Stop-DockerBackend {
    if (-not (Test-DockerReady)) { return }
    if (Test-ContainerExists) {
        & docker.exe stop $ContainerName | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Failed to stop SearXNG container.' }
    }
}

function Test-WslInstalled {
    if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
        return $false
    }
    & wsl.exe -d $WslDistro -u searxng -- test -x $wslPython 2>$null
    if ($LASTEXITCODE -ne 0) { return $false }
    & wsl.exe -d $WslDistro -u searxng -- test -d $wslSource 2>$null
    if ($LASTEXITCODE -ne 0) { return $false }
    & wsl.exe -d $WslDistro -u searxng -- test -f $wslSettings 2>$null
    return ($LASTEXITCODE -eq 0)
}

function Start-WslBackend {
    if (-not (Test-WslInstalled)) {
        throw "SearXNG is not installed in WSL distro '$WslDistro'."
    }
    if (Test-HttpReady) { return }
    $command = @(
        "cd '$wslSource' &&",
        "setsid -f env SEARXNG_SETTINGS_PATH='$wslSettings'",
        "'$wslPython' -m searx.webapp",
        ">/tmp/search-tool-searxng.log 2>&1"
    ) -join ' '
    & wsl.exe -d $WslDistro -u searxng -- sh -lc $command | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Failed to launch WSL SearXNG.' }
    Wait-Ready
}

function Stop-WslBackend {
    if (-not (Test-WslInstalled)) { return }
    $command = "pkill -f '[p]ython -m searx.webapp' || true"
    & wsl.exe -d $WslDistro -u searxng -- sh -lc $command | Out-Null
}

function Resolve-Backend {
    if ($Backend -ne 'Auto') { return $Backend }
    if (Test-WslInstalled) { return 'Wsl' }
    if (Test-DockerReady) { return 'Docker' }
    throw 'No local SearXNG backend is ready. Configure WSL SearXNG or start Docker.'
}

$selected = Resolve-Backend
switch ($Action) {
    'Start' {
        if ($selected -eq 'Wsl') {
            Start-WslBackend
        } else {
            Start-DockerBackend
        }
        Write-ConnectionInfo $selected
    }
    'Stop' {
        if ($selected -eq 'Wsl') {
            Stop-WslBackend
        } else {
            Stop-DockerBackend
        }
        Write-Host "backend=$selected stopped"
    }
    'Status' {
        $ready = Test-HttpReady
        Write-Host "backend=$selected"
        Write-Host "ready=$($ready.ToString().ToLowerInvariant())"
        if ($ready) {
            Write-ConnectionInfo $selected
        } elseif ($selected -eq 'Wsl') {
            Write-Host "installed=$(Test-WslInstalled)"
        } else {
            Write-Host "docker_ready=$(Test-DockerReady)"
        }
    }
}
