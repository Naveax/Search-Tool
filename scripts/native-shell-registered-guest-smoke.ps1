# Disposable GitHub-hosted ARM64 Windows 11 guest ONLY.
# Registers only the fixture HKCU CLSID and cleans it in finally.
[CmdletBinding()]
param([switch]$SelfTest,[switch]$RunHostedGuest)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
function Test-StrictGuest($f) {
    return ($f.actions -ceq 'true' -and
      $f.repo -ceq 'Naveax/Search-Tool' -and
      $f.environment -ceq 'github-hosted' -and
      $f.job -ceq 'windows11-desktop-native-feasibility' -and
      $f.os -ceq 'Windows' -and $f.arch -ceq 'ARM64' -and
      $f.actual -ceq 'Arm64' -and $f.caption -match 'Windows 11' -and
      $f.manufacturer -ceq 'Microsoft Corporation' -and
      $f.model -match 'Virtual Machine' -and $f.wsearch -ceq 'Running')
}
if($SelfTest) {
    $good=[pscustomobject]@{
        actions='true';repo='Naveax/Search-Tool';environment='github-hosted'
        job='windows11-desktop-native-feasibility';os='Windows'
        arch='ARM64';actual='Arm64';caption='Windows 11 Enterprise'
        manufacturer='Microsoft Corporation';model='Virtual Machine'
        wsearch='Running'
    }
    if(-not (Test-StrictGuest $good)){throw 'GUEST_POSITIVE_TEST_FAILED'}
    foreach($key in @('actions','repo','environment','job','os','arch',
       'actual','caption','manufacturer','model','wsearch')) {
        $bad=$good.PSObject.Copy();$bad.$key='UNAUTHORIZED'
        if(Test-StrictGuest $bad){throw ('GUEST_FAIL_OPEN_'+$key)}
    }
    'PRIVATE_SHELL_REGISTERED_GUEST_GUARD_SELFTEST_PASS'
    return
}
if(-not $RunHostedGuest){throw 'EXPLICIT_HOSTED_GUEST_FLAG_REQUIRED'}
$os=Get-CimInstance Win32_OperatingSystem
$system=Get-CimInstance Win32_ComputerSystem
$facts=[pscustomobject]@{
    actions=[string]$env:GITHUB_ACTIONS
    repo=[string]$env:GITHUB_REPOSITORY
    environment=[string]$env:RUNNER_ENVIRONMENT
    job=[string]$env:GITHUB_JOB
    os=[string]$env:RUNNER_OS
    arch=[string]$env:RUNNER_ARCH
    actual=[Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    caption=[string]$os.Caption
    manufacturer=[string]$system.Manufacturer
    model=[string]$system.Model
    wsearch=[string](Get-Service WSearch -ErrorAction Stop).Status
}
if(-not (Test-StrictGuest $facts)){
    throw 'PHYSICAL_OR_UNAUTHORIZED_SHELL_REGISTRATION_FORBIDDEN'
}
$root=[IO.Path]::GetFullPath([string]$env:GITHUB_WORKSPACE).TrimEnd('\')+'\'
$buildDir=Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out'
$dll=[IO.Path]::GetFullPath((Join-Path $buildDir 'private_shell_server_fixture.dll'))
$smoke=[IO.Path]::GetFullPath((Join-Path $buildDir 'private_shell_registered_guest.exe'))
foreach($path in @($dll,$smoke)){
    if(-not $path.StartsWith($root,[StringComparison]::OrdinalIgnoreCase) -or
       -not (Test-Path -LiteralPath $path -PathType Leaf)){
       throw 'PRIVATE_GUEST_BINARY_MISSING_OR_OUTSIDE_CHECKOUT'
    }
}
$clsid='{35DC8A17-4CD6-4A1B-A00B-6F0D17E44552}'
$regBase='HKCU:\Software\Classes\CLSID\'+$clsid
$hcr='Registry::HKEY_CLASSES_ROOT\CLSID\'+$clsid
if((Test-Path -LiteralPath $regBase) -or (Test-Path -LiteralPath $hcr)){
    throw 'EXISTING_PRIVATE_COM_CLASS_REFUSES_GUEST_INSTALL'
}
$created=$false
try {
    $created=$true
    New-Item -Path $regBase -Force -ErrorAction Stop | Out-Null
    $serverKey=Join-Path $regBase 'InprocServer32'
    New-Item -Path $serverKey -Force -ErrorAction Stop | Out-Null
    Set-Item -LiteralPath $serverKey -Value $dll -ErrorAction Stop
    New-ItemProperty -LiteralPath $serverKey -Name ThreadingModel -Value Apartment -PropertyType String -Force -ErrorAction Stop | Out-Null
    & $smoke --private-hosted-guest
    if($LASTEXITCODE -ne 0){throw 'GUEST_OS_COM_ACTIVATION_FAILED'}
    'NATIVE_SEARCH_HOSTED_SHELL_REGISTERED_COM_ACTIVATION_PASS'
} finally {
    if($created){
        Remove-Item -LiteralPath $regBase -Force -Recurse -ErrorAction Stop
        if((Test-Path -LiteralPath $regBase) -or (Test-Path -LiteralPath $hcr)){
            throw 'GUEST_SHELL_CLSID_CLEANUP_FAILED'
        }
        'NATIVE_SEARCH_HOSTED_SHELL_PRIVATE_CLSID_CLEANUP_PASS'
    }
}