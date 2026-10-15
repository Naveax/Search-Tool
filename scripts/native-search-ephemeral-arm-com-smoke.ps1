# OS COM activation smoke in a single disposable GitHub-hosted Windows 11
# ARM64 guest. NEVER execute on a personal/workstation Windows installation.
# Only unique private fixture registry entries are temporarily added. There
# are no crawl roots/rules, Windows Search service changes or shell inputs.
[CmdletBinding()]
param([switch]$SelfTest, [switch]$RunHostedGuest)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Test-StrictHostedGuest {
    param($Facts)
    if ($Facts.github_actions -cne 'true' -or
        $Facts.repository -cne 'Naveax/Search-Tool' -or
        $Facts.runner_environment -cne 'github-hosted' -or
        $Facts.job -cne 'windows11-desktop-native-feasibility' -or
        $Facts.os -cne 'Windows' -or
        $Facts.arch -cne 'ARM64' -or
        $Facts.actual_architecture -cne 'Arm64' -or
        -not $Facts.admin -or
        $Facts.os_caption -notmatch 'Windows 11' -or
        $Facts.computer_manufacturer -notmatch 'Microsoft Corporation' -or
        $Facts.computer_model -notmatch 'Virtual Machine' -or
        $Facts.wsearch -cne 'Running') {
        return $false
    }
    return $true
}

if ($SelfTest) {
    $facts = [pscustomobject]@{
        github_actions='true'; repository='Naveax/Search-Tool'
        runner_environment='github-hosted'
        job='windows11-desktop-native-feasibility'
        os='Windows'; arch='ARM64'; actual_architecture='Arm64';admin=$true
        os_caption='Microsoft Windows 11 Enterprise'
        computer_manufacturer='Microsoft Corporation'
        computer_model='Virtual Machine';wsearch='Running'
    }
    if (-not (Test-StrictHostedGuest $facts)) { throw 'HOSTED_GUEST_POSITIVE_CONTROL_REJECTED' }
    foreach ($bad in @('github_actions','repository','runner_environment','job',
                       'arch','actual_architecture','os_caption','computer_model','wsearch')) {
        $negative = $facts.PSObject.Copy()
        $negative.$bad = 'UNAUTHORIZED'
        if (Test-StrictHostedGuest $negative) {
            throw ('HOSTED_GUEST_FAIL_OPEN_ON_' + $bad)
        }
    }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_COM_REGISTRATION_GATE_SELFTEST_PASS'
    return
}
if (-not $RunHostedGuest) {
    throw 'EXPLICIT_RUN_HOSTED_GUEST_FLAG_REQUIRED'
}

$os = Get-CimInstance Win32_OperatingSystem
$system = Get-CimInstance Win32_ComputerSystem
$running = 'Unknown'
try { $running = [string](Get-Service -Name WSearch -ErrorAction Stop).Status } catch {}
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$facts = [pscustomobject]@{
    github_actions=[string]$env:GITHUB_ACTIONS
    repository=[string]$env:GITHUB_REPOSITORY
    runner_environment=[string]$env:RUNNER_ENVIRONMENT
    job=[string]$env:GITHUB_JOB
    os=[string]$env:RUNNER_OS
    arch=[string]$env:RUNNER_ARCH
    actual_architecture=[Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    admin=$admin
    os_caption=[string]$os.Caption
    computer_manufacturer=[string]$system.Manufacturer
    computer_model=[string]$system.Model
    wsearch=$running
}
if (-not (Test-StrictHostedGuest $facts)) {
    throw 'PHYSICAL_OR_UNAUTHORIZED_OS_COM_REGISTRATION_FORBIDDEN'
}

$repo = [IO.Path]::GetFullPath([string]$env:GITHUB_WORKSPACE).TrimEnd('\') + '\'
$fixture = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\protocol_fixture.dll'))
$consumer = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\registered_protocol_smoke.exe'))
if (-not $fixture.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase) -or
    -not $consumer.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath $fixture -PathType Leaf) -or
    -not (Test-Path -LiteralPath $consumer -PathType Leaf)) {
    throw 'NATIVE_PRIVATE_FIXTURE_BINARIES_MUST_BE_INSIDE_CHECKED_OUT_REPOSITORY'
}

$baseline = & (Join-Path $PSScriptRoot 'native-search-fixture-registry-audit.ps1') | ConvertFrom-Json
if ($baseline.result -cne 'CLEAN_BASELINE' -or $baseline.checked_markers -ne 36) {
    throw 'REGISTRATION_REFUSED_UNCLEAN_PRIVATE_FIXTURE_BASELINE'
}

$clsid='{987AD765-D904-4F10-A650-915D8F4C0287}'
$scheme='stfixture-proto-test'
$prog='SearchTool.PrivateFixtureProtocol.1'
$clsidPath="HKLM:\SOFTWARE\Classes\CLSID\$clsid"
$progPath="HKLM:\SOFTWARE\Classes\$prog"
$protocolPath='HKLM:\SOFTWARE\Microsoft\Windows Search\ProtocolHandlers'
if (Test-Path -LiteralPath $clsidPath) { throw 'PREEXISTING_PRIVATE_CLSID_FORBIDS_INSTALL' }
if (Test-Path -LiteralPath $progPath) { throw 'PREEXISTING_PRIVATE_PROGID_FORBIDS_INSTALL' }
if (-not (Test-Path -LiteralPath $protocolPath)) { throw 'WINDOWS_SEARCH_PROTOCOL_REGISTRY_MISSING' }

$createdClsid=$false
$createdProg=$false
$createdScheme=$false
try {
    # This test registers a private known GUID for OS COM CoCreateInstance,
    # not a production provider. It does NOT call DllRegisterServer.
    New-Item -Path $clsidPath -ErrorAction Stop | Out-Null
    $createdClsid=$true
    New-Item -Path "$clsidPath\InprocServer32" -Value $fixture -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath "$clsidPath\InprocServer32" -Name ThreadingModel -Value Both -PropertyType String -ErrorAction Stop | Out-Null
    New-Item -Path $progPath -Value 'SearchTool private ephemeral test protocol' -ErrorAction Stop | Out-Null
    $createdProg=$true
    New-Item -Path "$progPath\CLSID" -Value $clsid -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath $protocolPath -Name $scheme -Value $prog -PropertyType String -ErrorAction Stop | Out-Null
    $createdScheme=$true
    & $consumer
    if ($LASTEXITCODE -ne 0) { throw 'OS_COM_ACTIVATION_SMOKE_FAILED' }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_OS_COM_ACTIVATION_PASS'
} finally {
    if ($createdScheme) {
        Remove-ItemProperty -LiteralPath $protocolPath -Name $scheme -ErrorAction Stop
    }
    if ($createdProg) {
        Remove-Item -LiteralPath $progPath -Recurse -Force -ErrorAction Stop
    }
    if ($createdClsid) {
        Remove-Item -LiteralPath $clsidPath -Recurse -Force -ErrorAction Stop
    }
    $after = & (Join-Path $PSScriptRoot 'native-search-fixture-registry-audit.ps1') | ConvertFrom-Json
    if ($after.result -cne 'CLEAN_BASELINE' -or $after.checked_markers -ne 36) {
        throw 'EPHEMERAL_PRIVATE_OS_COM_REGISTRATION_ROLLBACK_FAILED'
    }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_REGISTRY_CLEANUP_PASS'
}
