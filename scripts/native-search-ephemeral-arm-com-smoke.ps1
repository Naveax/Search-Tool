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
$filterBinary = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\fixture_filter.dll'))
if (-not $filterBinary.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath $filterBinary -PathType Leaf)) {
    throw 'PRIVATE_IFILTER_BINARY_MISSING'
}
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

# Use a dedicated ephemeral Program Files folder to give SYSTEM/low-rights
# SearchFilterHost a readable DLL location. No existing folder is altered.
$privateBinDir = Join-Path $env:ProgramFiles 'SearchTool-PrivateFixture-CI-Only'
if (Test-Path -LiteralPath $privateBinDir) {
    throw 'PREEXISTING_EPHEMERAL_FIXTURE_DIR_FORBIDS_INSTALL'
}
$registeredFixture = Join-Path $privateBinDir 'protocol_fixture.dll'
$registeredFilter = Join-Path $privateBinDir 'fixture_filter.dll'
$filterDefinitions = @(
    [pscustomobject]@{
        extension='.stfixture'
        clsid='{44A4B197-7237-4958-A1D0-7CB26E804165}'
        persistent='{2E43BE1A-75EC-4FD6-9BA3-969516DB143A}'
    },
    [pscustomobject]@{
        extension='.stfixture-root'
        clsid='{5965EB3B-44AA-405F-9C60-1364074DF297}'
        persistent='{A78D9B02-39CD-4D0D-8D6E-74CB19392892}'
    }
)
foreach ($f in $filterDefinitions) {
    foreach ($candidate in @("HKLM:\SOFTWARE\Classes\CLSID\$($f.clsid)",
                            "HKLM:\SOFTWARE\Classes\CLSID\$($f.persistent)",
                            "HKLM:\SOFTWARE\Classes\$($f.extension)")) {
        if (Test-Path -LiteralPath $candidate) {
            throw 'PRIVATE_IFILTER_PREEXISTING_CLSID_OR_EXTENSION'
        }
    }
}
$ifilterRegistrationInterface = '{89BCB740-6119-101A-BCB7-00DD010655AF}'
$createdFilterPaths=New-Object 'System.Collections.Generic.List[string]'
$createdPersistentPaths=New-Object 'System.Collections.Generic.List[string]'
$createdExtensionPaths=New-Object 'System.Collections.Generic.List[string]'
$createdBinDir=$false
$createdClsid=$false
$createdProg=$false
$createdScheme=$false
try {
    New-Item -Path $privateBinDir -ItemType Directory -ErrorAction Stop | Out-Null
    $createdBinDir=$true
    Copy-Item -LiteralPath $fixture -Destination $registeredFixture -ErrorAction Stop
    Copy-Item -LiteralPath $filterBinary -Destination $registeredFilter -ErrorAction Stop

    # Create only custom filter classes and private file extensions, following
    # Microsoft Windows Search's PersistentHandler -> IFilter mapping.
    # Do not touch any standard Windows file types, existing COM keys or shell.
    foreach ($f in $filterDefinitions) {
        $filterPath = "HKLM:\SOFTWARE\Classes\CLSID\$($f.clsid)"
        $persistPath = "HKLM:\SOFTWARE\Classes\CLSID\$($f.persistent)"
        $extPath = "HKLM:\SOFTWARE\Classes\$($f.extension)"
        New-Item -Path $filterPath -ErrorAction Stop | Out-Null
        $createdFilterPaths.Add($filterPath)
        New-Item -Path "$filterPath\InprocServer32" -Value $registeredFilter -ErrorAction Stop | Out-Null
        New-ItemProperty -LiteralPath "$filterPath\InprocServer32" -Name ThreadingModel -Value Both -PropertyType String -ErrorAction Stop | Out-Null

        New-Item -Path $persistPath -ErrorAction Stop | Out-Null
        $createdPersistentPaths.Add($persistPath)
        New-Item -Path "$persistPath\PersistentAddinsRegistered" -ErrorAction Stop | Out-Null
        New-Item -Path "$persistPath\PersistentAddinsRegistered\$ifilterRegistrationInterface" -Value $f.clsid -ErrorAction Stop | Out-Null

        New-Item -Path $extPath -ErrorAction Stop | Out-Null
        $createdExtensionPaths.Add($extPath)
        New-Item -Path "$extPath\PersistentHandler" -Value $f.persistent -ErrorAction Stop | Out-Null
    }
    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_IFILTER_REGISTRATION_ONLY_PASS'
    # This test registers a private known GUID for OS COM CoCreateInstance,
    # not a production provider. It does NOT call DllRegisterServer.
    New-Item -Path $clsidPath -ErrorAction Stop | Out-Null
    $createdClsid=$true
    New-Item -Path "$clsidPath\InprocServer32" -Value $registeredFixture -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath "$clsidPath\InprocServer32" -Name ThreadingModel -Value Both -PropertyType String -ErrorAction Stop | Out-Null
    New-Item -Path $progPath -Value 'SearchTool private ephemeral test protocol' -ErrorAction Stop | Out-Null
    $createdProg=$true
    New-Item -Path "$progPath\CLSID" -Value $clsid -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath $protocolPath -Name $scheme -Value $prog -PropertyType String -ErrorAction Stop | Out-Null
    $createdScheme=$true
    & $consumer
    if ($LASTEXITCODE -ne 0) { throw 'OS_COM_ACTIVATION_SMOKE_FAILED' }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_OS_COM_ACTIVATION_PASS'

    # Observe SystemIndex Crawl Scope Manager, not merely the COM registry.
    # CSM changes are scoped to the same isolated private URL. No changes to
    # file: rules, existing roots, Start, Explorer or taskbar integration.
    $scopeReader = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\crawl_scope_readonly.exe'))
    $scopeTrial = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\crawl_scope_guest_trial.exe'))
    if (-not (Test-Path -LiteralPath $scopeReader -PathType Leaf) -or
        -not (Test-Path -LiteralPath $scopeTrial -PathType Leaf) -or
        -not $scopeTrial.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'PRIVATE_CSM_GUEST_BINARIES_MISSING_OR_OUTSIDE_REPOSITORY'
    }
    function Read-PrivateCrawlCounts {
        $lines = @(& $scopeReader)
        if ($LASTEXITCODE -ne 0) { throw 'WINDOWS_SEARCH_CRAWL_SCOPE_READONLY_FAILED' }
        $found = @{}
        foreach ($line in $lines) {
            if ($line -match '^PRIVATE_(ROOT_COUNT|RULE_COUNT)=(\d+)$') {
                $found[$Matches[1]] = [int]$Matches[2]
            }
        }
        if (-not $found.ContainsKey('ROOT_COUNT') -or
            -not $found.ContainsKey('RULE_COUNT')) {
            throw 'PRIVATE_CSM_READONLY_OUTPUT_MISSING_COUNTS'
        }
        return [pscustomobject]@{ roots=$found.ROOT_COUNT; rules=$found.RULE_COUNT }
    }
    $scopeBefore = Read-PrivateCrawlCounts
    if ($scopeBefore.roots -ne 0 -or $scopeBefore.rules -ne 0) {
        throw 'PRIVATE_CSM_ROOT_OR_RULE_ALREADY_PRESENT'
    }
    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ZERO_BASELINE_PASS'
    $scopeAttempted = $false
    try {
        $scopeAttempted = $true
        & $scopeTrial add
        if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_CSM_ADD_ROOT_OR_RULE_FAILED' }
        $scopeDuring = Read-PrivateCrawlCounts
        if ($scopeDuring.roots -ne 1 -or $scopeDuring.rules -ne 1) {
            throw 'PRIVATE_CSM_ADD_NOT_VISIBLE_THROUGH_REAL_WINDOWS_SEARCH'
        }
        Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ROOT_AND_RULE_OBSERVED_PASS'
        # Bounded evidence window. If an item is not indexed, report precisely
        # that, never infer successful SearchProtocolHost execution or UI.
        $seen = $false
        foreach ($attempt in 1..7) {
            $indexResult = & (Join-Path $PSScriptRoot 'native-search-systemindex-fixture-audit.ps1') | ConvertFrom-Json
            if ($indexResult.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $seen = $true
                break
            }
            Start-Sleep -Seconds 4
        }
        if ($seen) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_REAL_SYSTEMINDEX_ROW_OBSERVED'
        } else {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_SYSTEMINDEX_ROW_NOT_PROVEN'
        }
    } finally {
        if ($scopeAttempted) {
            & $scopeTrial remove
            if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_CSM_RULE_ROOT_ROLLBACK_FAILED' }
            $scopeAfter = Read-PrivateCrawlCounts
            if ($scopeAfter.roots -ne 0 -or $scopeAfter.rules -ne 0) {
                throw 'PRIVATE_CSM_NOT_RESTORED_AFTER_HOSTED_TEST'
            }
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ROLLBACK_PASS'
        }
    }

    # Different identity path: Windows Search's documented per-user URL.
    # Direct document root bypasses container child enumeration, isolating
    # service ACL vs user SID behavior. The SID is never logged.
    $userScopeBefore = Read-PrivateCrawlCounts
    if ($userScopeBefore.roots -ne 0 -or $userScopeBefore.rules -ne 0) {
        throw 'PRIVATE_USER_SCOPE_REQUIRES_FULLY_REMOVED_PREVIOUS_SCOPE'
    }
    $userScopeAttempted = $false
    try {
        $userScopeAttempted = $true
        & $scopeTrial add-user
        if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_USER_SCOPED_CSM_ADD_FAILED' }
        $userScopeDuring = Read-PrivateCrawlCounts
        if ($userScopeDuring.roots -ne 1 -or $userScopeDuring.rules -ne 1) {
            throw 'PRIVATE_USER_SCOPED_CSM_NOT_VISIBLE'
        }
        Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_USER_SID_CSM_OBSERVED_PASS'
        $userIndexed = $false
        foreach ($attempt in 1..7) {
            $userIndex = & (Join-Path $PSScriptRoot 'native-search-systemindex-fixture-audit.ps1') -CurrentUserScoped | ConvertFrom-Json
            if ($userIndex.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $userIndexed = $true
                break
            }
            Start-Sleep -Seconds 4
        }
        if ($userIndexed) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_USER_SID_SYSTEMINDEX_ROW_OBSERVED'
        } else {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_USER_SID_SYSTEMINDEX_ROW_NOT_PROVEN'
        }
    } finally {
        if ($userScopeAttempted) {
            & $scopeTrial remove-user
            if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_USER_SCOPED_CSM_ROLLBACK_FAILED' }
            $userScopeAfter = Read-PrivateCrawlCounts
            if ($userScopeAfter.roots -ne 0 -or $userScopeAfter.rules -ne 0) {
                throw 'PRIVATE_USER_SCOPED_CSM_ROLLBACK_NOT_CLEAN'
            }
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_USER_SID_CSM_ROLLBACK_PASS'
        }
    }
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
    foreach ($path in $createdExtensionPaths) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
    }
    foreach ($path in $createdPersistentPaths) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
    }
    foreach ($path in $createdFilterPaths) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
    }
    if ($createdBinDir) {
        Remove-Item -LiteralPath $privateBinDir -Recurse -Force -ErrorAction Stop
    }
    $after = & (Join-Path $PSScriptRoot 'native-search-fixture-registry-audit.ps1') | ConvertFrom-Json
    if ($after.result -cne 'CLEAN_BASELINE' -or $after.checked_markers -ne 36) {
        throw 'EPHEMERAL_PRIVATE_OS_COM_REGISTRATION_ROLLBACK_FAILED'
    }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_REGISTRY_CLEANUP_PASS'
}
