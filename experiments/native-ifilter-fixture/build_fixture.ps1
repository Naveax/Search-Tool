# Compiler discovery for Windows GitHub runners and developer machines.
# No registry writes, COM registration, SearchHost interaction or shell hooks.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw 'NATIVE_IFILTER_BUILD_BLOCKED_VSWHERE_UNAVAILABLE'
}
$roots = @(& $vswhere -all -products '*' -requires 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64' -property installationPath 2>&1)
if ($LASTEXITCODE -ne 0) {
    throw 'NATIVE_IFILTER_BUILD_BLOCKED_VSWHERE_QUERY_FAILED'
}
# Some side-by-side BuildTools installations are under Program Files (x86).
# Prefer a native Program Files path for cmd.exe batch calling; vswhere's
# returned order is not a preference or toolchain compatibility guarantee.
$roots = @($roots | Sort-Object @{Expression = { if (($_ -is [string]) -and $_.Contains('(x86)')) { 1 } else { 0 } }}, @{Expression = { [string]$_ }})
$vcvars = $null
foreach ($root in $roots) {
    if (-not ($root -is [string]) -or [string]::IsNullOrWhiteSpace($root)) { continue }
    $candidate = Join-Path $root 'VC\Auxiliary\Build\vcvars64.bat'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        $vcvars = $candidate
        break
    }
}
if (-not $vcvars) {
    throw 'NATIVE_IFILTER_BUILD_BLOCKED_NO_VCVARS64_COMPONENT'
}
$env:SEARCHTOOL_VCVARS_PATH = $vcvars
$build = Join-Path $PSScriptRoot 'build_fixture.cmd'
Write-Output ('MSVC_VCVARS_DISCOVERED=' + $vcvars)
& cmd.exe /d /c ('"' + $build + '"')
if ($LASTEXITCODE -ne 0) {
    throw ('NATIVE_IFILTER_BUILD_OR_PRIVATE_COM_TEST_FAILED_EXIT_' + $LASTEXITCODE)
}
Write-Output 'NATIVE_IFILTER_VSWHERE_DISCOVERY_AND_COM_TEST_PASS'
