# Deterministic hidden Win32 GUI regression. Never use the installed Search Tool
# executable, live index, SearchHost, global keyboard hooks or the visible desktop.
[CmdletBinding()]
param(
    [string] $ReportPath,
    [ValidateRange(1, 60)]
    [int] $TimeoutSeconds = 20
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$exe = (Resolve-Path -LiteralPath (Join-Path $repo 'target\release\search-tool-gui.exe')).Path
$fixture = (Resolve-Path -LiteralPath (Join-Path $repo 'tests\fixtures\gui-synthetic-index')).Path
foreach ($suffix in @('', '.ids', '.names', '.icp', '.ncp')) {
    $name = 'C.stidx' + $suffix
    if (-not (Test-Path -LiteralPath (Join-Path $fixture $name) -PathType Leaf)) {
        throw "Missing committed synthetic index fixture: $name"
    }
}
if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $tempDir = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [IO.Path]::GetTempPath()
    } else {
        $env:RUNNER_TEMP
    }
    $ReportPath = Join-Path $tempDir ('search-tool-hidden-ui-regression-' + $PID + '.txt')
}
$reportParent = Split-Path -Parent $ReportPath
if (-not (Test-Path -LiteralPath $reportParent -PathType Container)) {
    New-Item -ItemType Directory -Path $reportParent -Force | Out-Null
}
Remove-Item -LiteralPath $ReportPath -Force -ErrorAction SilentlyContinue

$commandArgs = '--ui-selftest "' + $fixture + '" --ui-selftest-report "' + $ReportPath + '"'
$proc = Start-Process -FilePath $exe -WorkingDirectory $repo -ArgumentList $commandArgs -PassThru
try {
    if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
        throw "Hidden GUI self-test exceeded $TimeoutSeconds seconds (PID $($proc.Id))"
    }
    $report = if (Test-Path -LiteralPath $ReportPath -PathType Leaf) {
        (Get-Content -LiteralPath $ReportPath -Raw).Trim()
    } else {
        '<missing report>'
    }
    if ($proc.ExitCode -ne 0 -or $report -ne 'PASS') {
        throw "Hidden GUI regression failed: exit=$($proc.ExitCode), report=$report"
    }
    Write-Host "Hidden Win32 GUI regression PASS; synthetic fixture; no production index/service modified."
} finally {
    if (-not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}
