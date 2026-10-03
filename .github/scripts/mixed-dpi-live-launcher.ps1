[CmdletBinding()]
param(
    [ValidateSet('Menu','Preflight','SelfTest')]
    [string]$Mode = 'Menu',
    [string]$PackageZip,
    [string]$OutputDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$statePath = Join-Path $root 'docs\RELEASE_STATE.json'
$finalizer = Join-Path $root '.github\scripts\mixed-dpi-finalizer.ps1'
$promoter = Join-Path $root '.github\scripts\mixed-dpi-promote.ps1'
$checker = Join-Path $root '.github\scripts\release-state-check.ps1'

if ([string]::IsNullOrWhiteSpace($PackageZip)) {
    $PackageZip = Join-Path $root 'dist\SearchTool-Windows-x64.zip'
} elseif (-not [IO.Path]::IsPathRooted($PackageZip)) {
    $PackageZip = Join-Path $root $PackageZip
}
$PackageZip = [IO.Path]::GetFullPath($PackageZip)

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root 'dist\mixed-dpi-final-live'
} elseif (-not [IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $root $OutputDir
}
$OutputDir = [IO.Path]::GetFullPath($OutputDir)
$bundlePath = Join-Path $OutputDir 'mixed-dpi-final-bundle.json'

function Assert-LiveLauncher {
    param([bool]$Condition,[string]$Message)
    if (-not $Condition) {
        throw "MIXED_DPI_LAUNCHER_FAIL: $Message"
    }
}

function Read-JsonFile {
    param([Parameter(Mandatory)][string]$Path)
    Assert-LiveLauncher (Test-Path -LiteralPath $Path -PathType Leaf) "missing file: $Path"
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Get-ReleaseState {
    $state = Read-JsonFile $statePath
    Assert-LiveLauncher ([string]$state.package.status -eq 'VALIDATED') 'release package is not VALIDATED'
    $blockers = @($state.external_blockers)
    Assert-LiveLauncher ($blockers.Count -eq 1 -and [string]$blockers[0].name -eq 'mixed_dpi') 'mixed_dpi is not the sole unresolved external blocker'
    $completed = @($state.completed_external_gates | ForEach-Object { [string]$_.name })
    foreach ($required in @('smartscreen','defender','web_resolver')) {
        Assert-LiveLauncher ($completed -contains $required) "required completed gate missing: $required"
    }
    return $state
}

function Assert-InteractiveDesktop {
    $proc = [Diagnostics.Process]::GetCurrentProcess()
    Assert-LiveLauncher ([Environment]::UserInteractive) 'live physical stages require an interactive Windows user desktop'
    Assert-LiveLauncher ($proc.SessionId -gt 0) "live physical stages must not run in Windows Session 0; SessionId=$($proc.SessionId)"
}

function Assert-CleanCurrentMain {
    & git -C $root fetch origin main | Out-Host
    Assert-LiveLauncher ($LASTEXITCODE -eq 0) 'git fetch origin main failed'

    $branch = (& git -C $root rev-parse --abbrev-ref HEAD).Trim()
    Assert-LiveLauncher ($LASTEXITCODE -eq 0 -and $branch -eq 'main') "checkout must be on main; current=$branch"

    $head = (& git -C $root rev-parse HEAD).Trim()
    $origin = (& git -C $root rev-parse origin/main).Trim()
    Assert-LiveLauncher ($head -eq $origin) "local main is not current; HEAD=$head origin/main=$origin"

    $tracked = @(& git -C $root status --porcelain --untracked-files=no)
    Assert-LiveLauncher ($LASTEXITCODE -eq 0 -and $tracked.Count -eq 0) 'tracked worktree changes exist'
    return $head
}

function Assert-SealedPackage {
    $state = Get-ReleaseState
    Assert-LiveLauncher (Test-Path -LiteralPath $PackageZip -PathType Leaf) "sealed package is missing: $PackageZip"
    $file = Get-Item -LiteralPath $PackageZip
    $sha = (Get-FileHash -LiteralPath $PackageZip -Algorithm SHA256).Hash.ToUpperInvariant()
    $expectedSha = ([string]$state.package.sha256).ToUpperInvariant()
    $expectedBytes = [int64]$state.package.bytes
    Assert-LiveLauncher ($sha -eq $expectedSha) "package SHA-256 mismatch; expected=$expectedSha actual=$sha"
    Assert-LiveLauncher ([int64]$file.Length -eq $expectedBytes) "package byte-size mismatch; expected=$expectedBytes actual=$($file.Length)"
    return $state
}

function Invoke-ReleaseStateCheck {
    & $checker | Out-Host
    Assert-LiveLauncher ($LASTEXITCODE -eq 0) "release-state checker failed with exit code $LASTEXITCODE"
}

function Invoke-FinalizerStep {
    param([Parameter(Mandatory)][string]$Step)
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    & $finalizer -Mode $Step -PackageZip $PackageZip -OutputDir $OutputDir | Out-Host
    Assert-LiveLauncher ($LASTEXITCODE -eq 0) "finalizer mode $Step failed with exit code $LASTEXITCODE"
}

function Invoke-Preflight {
    Assert-InteractiveDesktop
    $head = Assert-CleanCurrentMain
    $state = Assert-SealedPackage
    Invoke-ReleaseStateCheck
    Write-Host "PREFLIGHT_PASS HEAD=$head" -ForegroundColor Green
    Write-Host "PACKAGE_SHA256=$($state.package.sha256)" -ForegroundColor Green
    Invoke-FinalizerStep -Step 'Probe'
}

function Invoke-SelfTest {
    foreach ($path in @($statePath,$finalizer,$promoter,$checker)) {
        Assert-LiveLauncher (Test-Path -LiteralPath $path -PathType Leaf) "required launcher dependency missing: $path"
    }

    $state = Get-ReleaseState
    Assert-LiveLauncher ([string]$state.package.sha256 -match '^[0-9A-Fa-f]{64}$') 'release-state package SHA-256 is invalid'
    Assert-LiveLauncher ([int64]$state.package.bytes -gt 0) 'release-state package byte-size is invalid'
    Assert-LiveLauncher ($PackageZip -eq (Join-Path $root 'dist\SearchTool-Windows-x64.zip')) 'default package path changed unexpectedly'
    Assert-LiveLauncher ($OutputDir -eq (Join-Path $root 'dist\mixed-dpi-final-live')) 'default live output path changed unexpectedly'

    $ignored = & git -C $root check-ignore $PackageZip 2>$null
    Assert-LiveLauncher ($LASTEXITCODE -eq 0) 'default sealed package path is not ignored by Git'

    [ordered]@{
        schema = 1
        result = 'PASS'
        package_status = [string]$state.package.status
        package_sha256 = ([string]$state.package.sha256).ToUpperInvariant()
        package_bytes = [int64]$state.package.bytes
        sole_external_blocker = 'mixed_dpi'
        required_dependencies_present = $true
        default_package_path_ignored = $true
    } | ConvertTo-Json -Depth 4
}

function Show-Menu {
    while ($true) {
        Clear-Host
        Write-Host 'Search Tool - Final Mixed-DPI Physical Gate' -ForegroundColor Cyan
        Write-Host ''
        Write-Host '1  Preflight + Probe'
        Write-Host '2  Exercise'
        Write-Host '3  PreparePrimaryChanged'
        Write-Host '4  VerifyPrimaryChanged'
        Write-Host '5  PrepareMonitorRemoved'
        Write-Host '6  VerifyMonitorRemoved'
        Write-Host '7  Bundle'
        Write-Host '8  Promotion Validate'
        Write-Host '9  Promotion Apply (commit/merge yapmaz)'
        Write-Host '0  Exit'
        Write-Host ''

        $choice = Read-Host 'Secim'
        try {
            switch ($choice) {
                '1' { Invoke-Preflight }
                '2' { Assert-InteractiveDesktop; Assert-CleanCurrentMain | Out-Null; Assert-SealedPackage | Out-Null; Invoke-FinalizerStep 'Exercise' }
                '3' { Assert-InteractiveDesktop; Assert-CleanCurrentMain | Out-Null; Assert-SealedPackage | Out-Null; Invoke-FinalizerStep 'PreparePrimaryChanged' }
                '4' { Assert-InteractiveDesktop; Assert-CleanCurrentMain | Out-Null; Invoke-FinalizerStep 'VerifyPrimaryChanged' }
                '5' { Assert-InteractiveDesktop; Assert-CleanCurrentMain | Out-Null; Assert-SealedPackage | Out-Null; Invoke-FinalizerStep 'PrepareMonitorRemoved' }
                '6' { Assert-InteractiveDesktop; Assert-CleanCurrentMain | Out-Null; Invoke-FinalizerStep 'VerifyMonitorRemoved' }
                '7' { Assert-CleanCurrentMain | Out-Null; Invoke-FinalizerStep 'Bundle' }
                '8' {
                    Assert-CleanCurrentMain | Out-Null
                    Assert-LiveLauncher (Test-Path -LiteralPath $bundlePath -PathType Leaf) "final bundle missing: $bundlePath"
                    & $promoter -Mode Validate -Bundle $bundlePath | Out-Host
                    Assert-LiveLauncher ($LASTEXITCODE -eq 0) 'promotion validation failed'
                }
                '9' {
                    Assert-CleanCurrentMain | Out-Null
                    Assert-LiveLauncher (Test-Path -LiteralPath $bundlePath -PathType Leaf) "final bundle missing: $bundlePath"
                    & $promoter -Mode Apply -Bundle $bundlePath | Out-Host
                    Assert-LiveLauncher ($LASTEXITCODE -eq 0) 'promotion apply failed'
                    Write-Host 'PROMOTION_PREPARED: stop here; commit/PR/exact-head CI/merge remains required.' -ForegroundColor Yellow
                }
                '0' { return }
                default { Write-Host 'Gecersiz secim.' -ForegroundColor Yellow }
            }
        } catch {
            Write-Host ''
            Write-Host ("HATA: " + $_.Exception.Message) -ForegroundColor Red
        }

        Write-Host ''
        Read-Host 'Devam etmek icin Enter'
    }
}

switch ($Mode) {
    'SelfTest' { Invoke-SelfTest }
    'Preflight' { Invoke-Preflight }
    'Menu' { Show-Menu }
}
