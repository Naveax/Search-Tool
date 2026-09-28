[CmdletBinding()]
param(
    [ValidateRange(1, 120)] [int]$SoakMinutes = 1,
    [switch]$DefenderScan,
    [string]$OutputDir = (Join-Path $PSScriptRoot '..\dist\validation')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$OutputDir = if ([IO.Path]::IsPathRooted($OutputDir)) {
    [IO.Path]::GetFullPath($OutputDir)
} else {
    [IO.Path]::GetFullPath((Join-Path $root $OutputDir))
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$summaryPath = Join-Path $OutputDir "windows-release-gate-$stamp.json"
$packageDir = Join-Path $root 'dist'
$package = Join-Path $packageDir 'SearchTool-Windows-x64.zip'
$steps = [System.Collections.Generic.List[object]]::new()

function Clear-ReleaseRuntimeLocks {
    $releaseDir = [IO.Path]::GetFullPath((Join-Path $root 'target\release'))
    foreach ($process in @(Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue)) {
        try {
            if ($process.Path -and [IO.Path]::GetFullPath($process.Path).StartsWith($releaseDir, [StringComparison]::OrdinalIgnoreCase)) {
                Stop-Process -Id $process.Id -Force -ErrorAction Stop
                $process.WaitForExit(10000) | Out-Null
            }
        } catch {
            throw "Failed to stop release GUI process $($process.Id): $($_.Exception.Message)"
        }
    }

    $svc = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction SilentlyContinue
    if ($svc) {
        $expected = [IO.Path]::GetFullPath((Join-Path $releaseDir 'search-tool-service.exe'))
        $actual = [string]$svc.PathName
        if ($actual -notlike "*$expected*") {
            throw "Refusing to remove SearchToolIndexer owned by another installation: $actual"
        }
        & $expected --stop
        & $expected --uninstall
        if ($LASTEXITCODE -ne 0) { throw "Failed to remove stale release-gate service" }
    }
}

function Invoke-GateStep {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [scriptblock]$Action
    )
    Write-Host "==> $Name"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $result = 'PASS'
    $errorText = $null
    try {
        $global:LASTEXITCODE = 0
        & $Action
        if ($LASTEXITCODE -ne 0) { throw "$Name exited with code $LASTEXITCODE" }
    } catch {
        $result = 'FAIL'
        $errorText = $_.Exception.Message
        throw
    } finally {
        $sw.Stop()
        $steps.Add([pscustomobject]@{
            name = $Name
            result = $result
            duration_ms = [Math]::Round($sw.Elapsed.TotalMilliseconds, 1)
            error = $errorText
        })
    }
}

Push-Location $root
$overall = 'FAIL'
try {
    Invoke-GateStep 'release preflight' { Clear-ReleaseRuntimeLocks }
    Invoke-GateStep 'cargo fmt' { cargo fmt --all -- --check }
    Invoke-GateStep 'cargo clippy' { cargo clippy --workspace --all-targets -- -D warnings }
    Invoke-GateStep 'cargo test' { cargo test --workspace }
    Invoke-GateStep 'release build' { cargo build --workspace --release }
    Invoke-GateStep 'CLI smoke' {
        .\target\release\search-tool.exe version | Out-Host
        if ($LASTEXITCODE -ne 0) { return }
        .\target\release\search-tool.exe status | Out-Host
        if ($LASTEXITCODE -ne 0) { return }
        .\target\release\search-tool-bench.exe 100000 | Out-Host
    }
    Invoke-GateStep 'NTFS/USN/service integration' {
        & .\.github\scripts\windows-integration.ps1 -SoakMinutes $SoakMinutes
    }
    Invoke-GateStep 'USN journal reset recovery' {
        & .\scripts\journal-reset-recovery.ps1
    }
    Invoke-GateStep 'portable package build' {
        & .\scripts\package.ps1 -OutputDir $packageDir
    }
    Invoke-GateStep 'portable package integrity' {
        & .\scripts\verify-package.ps1 -Package $package
    }
    Invoke-GateStep 'clean install/uninstall smoke' {
        & .\scripts\install-smoke.ps1 -Package $package
    }
    Invoke-GateStep 'Microsoft Defender interaction' {
        $defenderArgs = @{
            Path = (Join-Path $root 'target\release')
        }
        if ($DefenderScan) { $defenderArgs.CustomScan = $true }
        & .\scripts\defender-check.ps1 @defenderArgs
    }
    $overall = 'PASS'
} finally {
    Pop-Location
    $summary = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        result = $overall
        computer_name = $env:COMPUTERNAME
        soak_minutes = $SoakMinutes
        defender_scan = [bool]$DefenderScan
        package = if (Test-Path -LiteralPath $package) { $package } else { $null }
        steps = $steps
    }
    $summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $summaryPath -Encoding UTF8
    Write-Host "Summary=$summaryPath"
}

if ($overall -ne 'PASS') { throw 'WINDOWS_RELEASE_GATE=FAIL' }
Write-Host 'WINDOWS_RELEASE_GATE=PASS'
