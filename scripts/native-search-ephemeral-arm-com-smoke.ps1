# OS COM activation smoke in a single disposable GitHub-hosted Windows 11
# ARM64 guest. NEVER execute on a personal/workstation Windows installation.
# Only unique private fixture registry entries are temporarily added. There
# are no crawl roots/rules, Windows Search service changes or shell inputs.
[CmdletBinding()]
param([switch]$SelfTest, [switch]$RunHostedGuest, [switch]$ReloadGuestWSearch, [switch]$TraceGuestCallbacks)
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

function Assert-PrivateSyntheticIndexAcceptance {
    param(
        [Parameter(Mandatory)][bool]$ExactUrlFound,
        [Parameter(Mandatory)][object]$Shapes
    )
    # This is an isolated fixture acceptance gate, NOT Start/Explorer UX.
    if (-not $ExactUrlFound -or
        $Shapes.observation -cne 'PRIVATE_SCHEME_ROWS_FOUND' -or
        [int]$Shapes.matched_count -ne 1 -or
        [int]$Shapes.exact_document -ne 1 -or
        -not [bool]$Shapes.query_user_matches_fixture_owner -or
        $Shapes.fixture_content_token_observation -cne
            'SYNTHETIC_MARKER_MATCH_VISIBLE') {
        throw 'PRIVATE_SYNTHETIC_SYSTEMINDEX_DOCUMENT_AND_CONTENT_NOT_ACCEPTED'
    }
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
    # ETW EVENT_HEADER.Task/Id carry the upper/lower HRESULT halves,
    # respectively. Verify decoder without touching the host.
    $fake = [pscustomobject]@{ Task=32772; Id=5900 }
    if (('0x{0:X4}{1:X4}' -f [int]$fake.Task, [int]$fake.Id) -cne '0x8004170C') {
        throw 'PRIVATE_ETW_HRESULT_HEADER_DECODER_FAILED'
    }
    $ok = [pscustomobject]@{
        observation='PRIVATE_SCHEME_ROWS_FOUND'
        matched_count=1
        exact_document=1
        query_user_matches_fixture_owner=$true
        fixture_content_token_observation='SYNTHETIC_MARKER_MATCH_VISIBLE'
    }
    Assert-PrivateSyntheticIndexAcceptance -ExactUrlFound $true -Shapes $ok
    foreach ($bad in @('matched_count','exact_document',
                       'query_user_matches_fixture_owner',
                       'fixture_content_token_observation')) {
        $negative = $ok.PSObject.Copy()
        $negative.$bad = if ($bad -ceq 'fixture_content_token_observation') {
            'SYNTHETIC_MARKER_NOT_VISIBLE'
        } else { 0 }
        $rejected=$false
        try {
            Assert-PrivateSyntheticIndexAcceptance -ExactUrlFound $true -Shapes $negative
        } catch {
            $rejected = $_.Exception.Message -ceq
                'PRIVATE_SYNTHETIC_SYSTEMINDEX_DOCUMENT_AND_CONTENT_NOT_ACCEPTED'
        }
        if (-not $rejected) {
            throw ('PRIVATE_SYNTHETIC_ACCEPTANCE_FAIL_OPEN_' + $bad)
        }
    }
    $missingUrlRejected=$false
    try {
        Assert-PrivateSyntheticIndexAcceptance -ExactUrlFound $false -Shapes $ok
    } catch {
        $missingUrlRejected = $_.Exception.Message -ceq
            'PRIVATE_SYNTHETIC_SYSTEMINDEX_DOCUMENT_AND_CONTENT_NOT_ACCEPTED'
    }
    if (-not $missingUrlRejected) { throw 'PRIVATE_SYNTHETIC_ACCEPTANCE_URL_GATE_FAILED' }
    Write-Output 'NATIVE_SEARCH_PRIVATE_SYSTEMINDEX_ACCEPTANCE_GATE_SELFTEST_PASS'
    Write-Output 'NATIVE_SEARCH_PRIVATE_ETW_HRESULT_DECODER_SELFTEST_PASS'
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
$shellProbe = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\private_shell_resolution_guest.exe'))
$filterBinary = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\experiments\native-ifilter-fixture\out\fixture_filter.dll'))
if (-not $shellProbe.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath $shellProbe -PathType Leaf)) {
    throw 'PRIVATE_SHELL_RESOLUTION_GUEST_PROBE_MISSING'
}
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
$independentProg='SearchTool.PrivateFixtureProtocol'
$independentPath="HKLM:\SOFTWARE\Classes\$independentProg"
$clsidPath="HKLM:\SOFTWARE\Classes\CLSID\$clsid"
$progPath="HKLM:\SOFTWARE\Classes\$prog"
$protocolPath='HKLM:\SOFTWARE\Microsoft\Windows Search\ProtocolHandlers'
if (Test-Path -LiteralPath $clsidPath) { throw 'PREEXISTING_PRIVATE_CLSID_FORBIDS_INSTALL' }
if (Test-Path -LiteralPath $progPath) { throw 'PREEXISTING_PRIVATE_PROGID_FORBIDS_INSTALL' }
if (Test-Path -LiteralPath $independentPath) { throw 'PREEXISTING_PRIVATE_INDEPENDENT_PROGID_FORBIDS_INSTALL' }
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
        documentClass='{A14BB0EF-4769-42EC-9B6A-0B06E0B14201}'
    },
    [pscustomobject]@{
        extension='.stfixture-root'
        clsid='{5965EB3B-44AA-405F-9C60-1364074DF297}'
        persistent='{A78D9B02-39CD-4D0D-8D6E-74CB19392892}'
        documentClass='{4A45139F-BB04-4C20-A69B-8F2049C919F0}'
    }
)
foreach ($f in $filterDefinitions) {
    foreach ($candidate in @("HKLM:\SOFTWARE\Classes\CLSID\$($f.clsid)",
                            "HKLM:\SOFTWARE\Classes\CLSID\$($f.persistent)",
                            "HKLM:\SOFTWARE\Classes\CLSID\$($f.documentClass)",
                            "HKLM:\SOFTWARE\Classes\$($f.extension)")) {
        if (Test-Path -LiteralPath $candidate) {
            throw 'PRIVATE_IFILTER_PREEXISTING_CLSID_OR_EXTENSION'
        }
    }
}
$ifilterRegistrationInterface = '{89BCB740-6119-101A-BCB7-00DD010655AF}'
$createdFilterPaths=New-Object 'System.Collections.Generic.List[string]'
$createdPersistentPaths=New-Object 'System.Collections.Generic.List[string]'
$createdDocumentClasses=New-Object 'System.Collections.Generic.List[string]'
$createdExtensionPaths=New-Object 'System.Collections.Generic.List[string]'
$createdBinDir=$false
$createdClsid=$false
$createdProg=$false
$createdIndependent=$false
$createdScheme=$false
$guestWsearchReloadAttempted=$false
$privateEtwSession='SearchToolPrivateFixtureCI'
$privateEtwProviderGuid='CBBAE291-F9D2-4E9D-A779-9DECE2129912'
$privateEtwPath=Join-Path $env:RUNNER_TEMP 'SearchToolPrivateFixtureCI.etl'
$privateEtwStarted=$false
$privateEtwStopped=$false
$privateEtwIssue=$null
$gathererBeginUtc=[DateTime]::UtcNow.ToString('o')
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
        # Fallback for Windows' document-CLSID selector: this class only
        # identifies the private synthetic data format, not a COM server.
        $docClassPath = "HKLM:\SOFTWARE\Classes\CLSID\$($f.documentClass)"
        New-Item -Path $docClassPath -ErrorAction Stop | Out-Null
        $createdDocumentClasses.Add($docClassPath)
        New-Item -Path "$docClassPath\PersistentHandler" -Value $f.persistent -ErrorAction Stop | Out-Null
    }
    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_IFILTER_DOCUMENT_CLASSES_REGISTERED_PASS'
    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_IFILTER_REGISTRATION_ONLY_PASS'
    # This test registers a private known GUID for OS COM CoCreateInstance,
    # not a production provider. It does NOT call DllRegisterServer.
    New-Item -Path $clsidPath -ErrorAction Stop | Out-Null
    $createdClsid=$true
    # Synthetic fixture source metadata: a CI runner user SID, NOT the
    # privileged Windows Search host service identity. Never print SID.
    $syntheticOwnerSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    if ($syntheticOwnerSid -cnotmatch '^S-1-[0-9]+(?:-[0-9]+)+$' -or
        $syntheticOwnerSid.Length -gt 180) {
        throw 'EPHEMERAL_PRIVATE_FIXTURE_OWNER_MUST_BE_VALID_CURRENT_USER'
    }
    New-ItemProperty -LiteralPath $clsidPath -Name FixtureOwnerSid -Value $syntheticOwnerSid -PropertyType String -ErrorAction Stop | Out-Null
    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_OWNER_ACL_SOURCE_INSTALLED'
    New-Item -Path "$clsidPath\InprocServer32" -Value $registeredFixture -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath "$clsidPath\InprocServer32" -Name ThreadingModel -Value Both -PropertyType String -ErrorAction Stop | Out-Null
    # Microsoft's documented protocol registration also declares the
    # dependent and independent ProgID, plus reverse links from CLSID.
    # These are PRIVATE synthetic classes, not standard Shell handlers.
    New-Item -Path "$clsidPath\ProgID" -Value $prog -ErrorAction Stop | Out-Null
    New-Item -Path "$clsidPath\VersionIndependentProgID" -Value $independentProg -ErrorAction Stop | Out-Null
    New-Item -Path $independentPath -Value 'SearchTool private CI-only protocol' -ErrorAction Stop | Out-Null
    $createdIndependent=$true
    New-Item -Path "$independentPath\CLSID" -Value $clsid -ErrorAction Stop | Out-Null
    New-Item -Path "$independentPath\CurVer" -Value $prog -ErrorAction Stop | Out-Null
    New-Item -Path $progPath -Value 'SearchTool private ephemeral test protocol' -ErrorAction Stop | Out-Null
    $createdProg=$true
    New-Item -Path "$progPath\CLSID" -Value $clsid -ErrorAction Stop | Out-Null
    New-ItemProperty -LiteralPath $protocolPath -Name $scheme -Value $prog -PropertyType String -ErrorAction Stop | Out-Null
    $createdScheme=$true
    & $consumer
    if ($LASTEXITCODE -ne 0) { throw 'OS_COM_ACTIVATION_SMOKE_FAILED' }
    Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_OS_COM_ACTIVATION_PASS'

    if ($TraceGuestCallbacks) {
        # ETW provider from our own private fixture DLLs only: fixed event
        # IDs, no strings, SIDs, URLs, payload, user process enumeration or
        # system-wide kernel events. No manifest registration is required.
        if (Test-Path -LiteralPath $privateEtwPath) {
            throw 'PREEXISTING_EPHEMERAL_PRIVATE_ETW_TRACE_FORBIDS_START'
        }
        & logman.exe start $privateEtwSession -p "{$privateEtwProviderGuid}" 0xffffffff 5 -o $privateEtwPath -ets | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'EPHEMERAL_PRIVATE_ETW_START_FAILED' }
        $privateEtwStarted=$true
        Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_SESSION_STARTED'
    }

    if ($ReloadGuestWSearch) {
        # This is an EXPLICIT extra experiment after all strict CI-only
        # checks. WSearch was proven Running on this one ephemeral Win11
        # ARM64 GitHub runner. Never run on a physical Windows computer.
        $guestWsearchReloadAttempted=$true
        Restart-Service -Name WSearch -Force -ErrorAction Stop
        (Get-Service WSearch -ErrorAction Stop).WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds(65))
        Write-Output 'NATIVE_SEARCH_HOSTED_GUEST_WSEARCH_RELOADED_AFTER_PRIVATE_REGISTRATION'
    }

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
    function Measure-PrivateSearchHostModules {
        # A pure read-only observer of Windows-owned SearchProtocolHost and
        # SearchFilterHost process modules. It reports only our two private DLL
        # basenames; never other modules, process IDs, user paths or tokens.
        $observation = & (Join-Path $PSScriptRoot 'native-search-host-module-probe.ps1') | ConvertFrom-Json
        if ($LASTEXITCODE -ne 0 -or $observation.mode -cne 'LIVE_READ_ONLY' -or
            $observation.result -cne 'DIAGNOSTIC_ONLY' -or
            $observation.registry_modified -or $observation.debugger_attached) {
            throw 'PRIVATE_SEARCH_HOST_MODULE_INSPECTION_NOT_READONLY'
        }
        # Catalog status and queue counters are aggregate, documented
        # Windows Search properties. Never query URLBeingIndexed or expose
        # any file/owner name from the guest.
        $catalogLines = @(& $scopeReader)
        if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_CATALOG_READONLY_QUERY_FAILED' }
        foreach ($line in $catalogLines) {
            if ($line -match '^CATALOG_(STATUS|PAUSED_REASON|PENDING_INCREMENTAL|PENDING_NOTIFICATIONS|PENDING_HIGH_PRIORITY|TOTAL_ITEMS)=(-?[0-9]+)$') {
                Write-Output ('NATIVE_SEARCH_GUEST_' + $line)
            }
        }
        foreach ($m in @($observation.host_modules)) {
            if ($m.host -cne 'SearchProtocolHost' -and $m.host -cne 'SearchFilterHost') {
                throw 'PRIVATE_SEARCH_HOST_UNEXPECTED_PROCESS_IN_REPORT'
            }
            Write-Output ('NATIVE_SEARCH_PRIVATE_MODULE_' +
                $m.host.ToUpperInvariant() + '_' + $m.state)
        }
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
        Measure-PrivateSearchHostModules
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
        Measure-PrivateSearchHostModules
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
        Measure-PrivateSearchHostModules
        $userIndexed = $false
        foreach ($attempt in 1..7) {
            $userIndex = & (Join-Path $PSScriptRoot 'native-search-systemindex-fixture-audit.ps1') -CurrentUserScoped | ConvertFrom-Json
            if ($userIndex.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $userIndexed = $true
                break
            }
            Start-Sleep -Seconds 4
        }
        Measure-PrivateSearchHostModules
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

    # Independent positive indexability test: a single SID-less PRIVATE
    # document root. This bypasses container enumeration but retains
    # source-owned restricted ACL; only fixed synthetic text is eligible
    # for the Windows-owned built-in text IFilter. Never touch file: roots.
    $directBefore = Read-PrivateCrawlCounts
    if ($directBefore.roots -ne 0 -or $directBefore.rules -ne 0) {
        throw 'PRIVATE_DIRECT_DOCUMENT_REQUIRES_ZERO_PREVIOUS_SCOPE'
    }
    $directAttempted = $false
    try {
        $directAttempted = $true
        & $scopeTrial add-document
        if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_DIRECT_DOCUMENT_ADD_FAILED' }
        $directDuring = Read-PrivateCrawlCounts
        if ($directDuring.roots -ne 1 -or $directDuring.rules -ne 1) {
            throw 'PRIVATE_DIRECT_DOCUMENT_SCOPE_NOT_OBSERVED'
        }
        Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_DOCUMENT_SCOPE_PASS'
        # Request queue acknowledgement for ONE predefined private item.
        # Existing ReindexSearchRoot only confirms an accepted root
        # operation; this returns a separate per-URL HRESULT. Its success
        # must never be treated as proof of a committed SystemIndex row.
        & $scopeTrial notify-document
        if ($LASTEXITCODE -ne 0) {
            throw 'PRIVATE_EXACT_DOCUMENT_NOTIFICATION_PROBE_FAILED'
        }
        # Independent provider-managed notification for the SAME fixed
        # private document. Only its indexer-assigned DocID is observed
        # in memory, allowing Gatherer/Queryable/Persisted callback counts
        # and HRESULTs without exposing other URL/DocID information.
        & $scopeTrial monitor-document
        if ($LASTEXITCODE -ne 0) {
            throw 'PRIVATE_INLINE_COMPLETION_PROBE_FAILED'
        }
        Measure-PrivateSearchHostModules
        $directIndexed = $false
        $directCanonicalIndexed = $false
        # Gatherer/indexer persistence may follow delivery asynchronously.
        # Bound this direct document crawl to ~92 seconds in the disposable VM.
        foreach ($attempt in 1..23) {
            $audit = Join-Path $PSScriptRoot 'native-search-systemindex-fixture-audit.ps1'
            $directObservation = & $audit | ConvertFrom-Json
            if ($directObservation.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $directIndexed = $true
            }
            $canonicalObservation = & $audit -CanonicalDocumentRoot | ConvertFrom-Json
            if ($canonicalObservation.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $directCanonicalIndexed = $true
            }
            if ($directIndexed -or $directCanonicalIndexed) { break }
            Start-Sleep -Seconds 4
        }
        Measure-PrivateSearchHostModules
        # Read-only, bounded private SCHEME audit while the root still
        # exists. A URL may be stored under an unexpected canonical form.
        # Emit only category counts; NEVER returned URLs or SIDs.
        $shapes = & (Join-Path $PSScriptRoot 'native-search-private-index-shapes.ps1') | ConvertFrom-Json
        if ($shapes.mode -cne 'DISPOSABLE_GUEST_PRIVATE_SCHEME_READ_ONLY' -or
            $shapes.raw_urls_exported -cne $false -or
            $shapes.raw_sid_exported -cne $false -or
            $shapes.release_eligible -cne $false -or
            $shapes.fixture_owner_source -notin @(
                'VALID_PRIVATE_TEST_OWNER','INVALID_PRIVATE_TEST_OWNER',
                'PRIVATE_TEST_OWNER_NOT_READABLE') -or
            $shapes.catalog_total_count_observation -notin @(
                'SCALAR_COUNT_OK','INVALID_SCALAR_COUNT',
                'SCALAR_COUNT_UNSUPPORTED_OR_FAILED','NOT_CHECKED') -or
            $shapes.provider_numeric_row_observation -notin @(
                'NOT_CHECKED','ROW_VISIBLE_NUMERIC_NULL',
                'ROW_VISIBLE_NUMERIC_VALUE','ROW_VISIBLE_UNEXPECTED_VALUE_TYPE',
                'NO_ROWS_VISIBLE_TO_QUERY_TOKEN','NUMERIC_QUERY_UNSUPPORTED_OR_FAILED') -or
            $shapes.provider_numeric_row_error_hresult -cnotmatch '^(?:NOT_RECORDED|0x[0-9A-F]{8})$' -or
            $shapes.fixture_content_token_observation -notin @(
                'NOT_CHECKED','SYNTHETIC_MARKER_MATCH_VISIBLE',
                'SYNTHETIC_MARKER_NOT_VISIBLE','CONTENT_QUERY_UNSUPPORTED_OR_FAILED') -or
            $shapes.fixture_content_token_error_hresult -cnotmatch '^(?:NOT_RECORDED|0x[0-9A-F]{8})$' -or
            $shapes.matched_count -lt 0 -or $shapes.matched_count -gt 20) {
            throw 'PRIVATE_SCHEME_INDEX_AUDIT_CONTRACT_VIOLATION'
        }
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_SCHEME_INDEX_OBSERVATION=' + $shapes.observation)
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_SCHEME_INDEX_COUNT=' + $shapes.matched_count)
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_QUERY_TOKEN_MATCHES_FIXTURE_OWNER=' +
            [int][bool]$shapes.query_user_matches_fixture_owner)
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_FIXTURE_OWNER_STATUS=' +
            $shapes.fixture_owner_source)
        Write-Output ('NATIVE_SEARCH_HOSTED_QUERY_PROVIDER_GLOBAL_SCALAR_STATUS=' +
            $shapes.catalog_total_count_observation)
        Write-Output ('NATIVE_SEARCH_HOSTED_QUERY_PROVIDER_NUMERIC_ROW_STATUS=' +
            $shapes.provider_numeric_row_observation)
        Write-Output ('NATIVE_SEARCH_HOSTED_QUERY_PROVIDER_NUMERIC_ROW_HRESULT=' +
            $shapes.provider_numeric_row_error_hresult)
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_CONTENT_TOKEN_OBSERVATION=' +
            $shapes.fixture_content_token_observation)
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_CONTENT_TOKEN_HRESULT=' +
            $shapes.fixture_content_token_error_hresult)
        if ($shapes.catalog_total_count_observation -ceq 'SCALAR_COUNT_OK') {
            $total = [long]$shapes.catalog_query_visible_count
            if ($total -lt 0 -or $total -gt 1000000000) {
                throw 'PRIVATE_SCHEME_PROVIDER_GLOBAL_COUNT_OUT_OF_BOUNDS'
            }
            Write-Output ('NATIVE_SEARCH_HOSTED_QUERY_PROVIDER_VISIBLE_GLOBAL_COUNT=' + $total)
        }
        foreach ($name in @('exact_document','canonical_document','private_root',
                            'sid_scoped_document','other_private_url_shape')) {
            $number = [int]$shapes.$name
            if ($number -lt 0 -or $number -gt 20) {
                throw 'PRIVATE_SCHEME_INDEX_UNBOUNDED_CATEGORY'
            }
            Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_SCHEME_INDEX_' +
                $name.ToUpperInvariant() + '_COUNT=' + $number)
        }
        if ($directIndexed) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_INDEXED_EXACT'
        }
        if ($directCanonicalIndexed) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_INDEXED_TRAILING_SLASH'
        }
        if (-not $directIndexed -and -not $directCanonicalIndexed) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_NOT_PROVEN_EITHER_EXACT_URL'
        }
        # Convert the now-live-proven isolated index fixture into a strict
        # regression gate. Missing exact URL/content must fail CI, while
        # the surrounding finally still restores private CSM and WSearch.
        Assert-PrivateSyntheticIndexAcceptance -ExactUrlFound $directIndexed -Shapes $shapes
        Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_SYSTEMINDEX_DOCUMENT_AND_CONTENT_ACCEPTANCE_PASS'
        # Next independent acceptance boundary: Does the actual OS Shell
        # parser materialize this indexed private URI as an IShellItem?
        # This cannot establish Start/Win+S UI integration by itself.
        # Run ONLY inside this strictly admitted disposable Win11 ARM guest,
        # while the single private test document is still in its crawl scope.
        # Windows PowerShell 5.1 Start-Process -PassThru returned a blank
        # ExitCode on the actual hosted ARM64 run despite a successful
        # exact-URI observation. Own the native .NET Process object, with
        # an explicit typed exit code and asynchronous bounded streams.
        $probeStart = New-Object System.Diagnostics.ProcessStartInfo
        $probeStart.FileName = $shellProbe
        $probeStart.Arguments = '--guest-private-shell-observation'
        $probeStart.UseShellExecute = $false
        $probeStart.CreateNoWindow = $true
        $probeStart.RedirectStandardOutput = $true
        $probeStart.RedirectStandardError = $true
        $probeProcess = New-Object System.Diagnostics.Process
        $probeStarted = $false
        try {
            $probeProcess.StartInfo = $probeStart
            if (-not $probeProcess.Start()) {
                throw 'PRIVATE_SHELL_URI_PARSER_PROCESS_START_FAILED'
            }
            $probeStarted = $true
            $stdoutTask = $probeProcess.StandardOutput.ReadToEndAsync()
            $stderrTask = $probeProcess.StandardError.ReadToEndAsync()
            if (-not $probeProcess.WaitForExit(15000)) {
                throw 'PRIVATE_SHELL_URI_PARSER_BOUNDED_TIMEOUT'
            }
            $probeProcess.WaitForExit()
            $exitCode = [int]$probeProcess.ExitCode
            # Never log returned raw Shell identity or stderr. The result
            # format is fixed and the output cannot exceed 1024 characters.
            $stdout = [string]$stdoutTask.GetAwaiter().GetResult()
            $null = $stderrTask.GetAwaiter().GetResult()
            if ($stdout.Length -gt 1024) {
                throw 'PRIVATE_SHELL_URI_PARSER_UNBOUNDED_OUTPUT'
            }
            $probeLines = @($stdout.Trim() -split '[\r\n]+' | Where-Object { $_.Length -gt 0 })
            $allowedCategory = '^NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=(?:UNRESOLVED|NONMATCHING_PARSING_NAME|GENERIC_URI_PARSER_ACCEPTS_UNINDEXED_CONTROL|PRIVATE_URI_RESOLVED_CONTROL_REJECTED)$'
            $allowedHr = '^NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_HRESULT=0x[0-9A-F]{8}$'
            if ($probeLines.Count -gt 0 -and $probeLines[0] -cmatch $allowedCategory) {
                Write-Output $probeLines[0]
            }
            Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_PROBE_EXIT_CODE=' + $exitCode)
            if ($exitCode -ne 0) {
                throw 'PRIVATE_SHELL_URI_PARSER_PROCESS_REJECTED'
            }
            if ($probeLines.Count -ne 2 -or
                $probeLines[0] -cnotmatch $allowedCategory -or
                $probeLines[1] -cnotmatch $allowedHr) {
                throw 'PRIVATE_SHELL_URI_PARSER_UNSAFE_OR_MALFORMED_OUTPUT'
            }
            Write-Output $probeLines[1]
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_OBSERVATION_ONLY_NOT_UX_ACCEPTANCE'
        } finally {
            if ($probeStarted -and -not $probeProcess.HasExited) {
                $probeProcess.Kill()
            }
            if ($probeProcess) { $probeProcess.Dispose() }
        }
    } finally {
        if ($directAttempted) {
            & $scopeTrial remove-document
            if ($LASTEXITCODE -ne 0) {
                throw 'PRIVATE_DIRECT_DOCUMENT_ROLLBACK_FAILED'
            }
            $directAfter = Read-PrivateCrawlCounts
            if ($directAfter.roots -ne 0 -or $directAfter.rules -ne 0) {
                throw 'PRIVATE_DIRECT_DOCUMENT_SCOPE_NOT_REMOVED'
            }
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_DOCUMENT_ROLLBACK_PASS'
        }
    }
    # Read only Search events in this short guest-only trial; the separate
    # audit rejects nonprivate URLs and emits ONLY event IDs/HRESULT counts.
    $gathererSummary = & (Join-Path $PSScriptRoot 'native-search-private-gatherer-audit.ps1') -SinceUtc $gathererBeginUtc | ConvertFrom-Json
    if ($gathererSummary.mode -cne 'READ_ONLY_EXACT_PRIVATE_SCHEME_ONLY' -or
        $gathererSummary.raw_urls_exported -cne $false -or
        $gathererSummary.raw_event_xml_exported -cne $false -or
        $gathererSummary.release_eligible -cne $false) {
        throw 'EPHEMERAL_PRIVATE_GATHERER_AUDIT_SAFETY_VIOLATION'
    }
    Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_GATHERER_MATCHED_COUNT=' + $gathererSummary.matched_private_events)
    Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_GATHERER_OUTCOME=' + $gathererSummary.outcome)
    foreach ($property in @($gathererSummary.groups.PSObject.Properties)) {
        if ($property.Name -cnotmatch '^EVENT_[0-9]{1,5}_HRESULT_(?:0X[0-9A-F]{8}|NOT_RECORDED)$') {
            throw 'EPHEMERAL_PRIVATE_GATHERER_UNEXPECTED_COUNTER'
        }
        Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_GATHERER_' + $property.Name + '_COUNT=' + $property.Value)
    }
} finally {
    if ($privateEtwStarted) {
        # Deliberately validate our provider with a known in-process canary
        # AFTER the crawl window. Timestamp separates these selftest events
        # from any genuine Windows Search host callbacks.
        $canaryUtc = [DateTime]::UtcNow
        try {
            & $consumer | Out-Null
            if ($LASTEXITCODE -ne 0) {
                # This is a trace-transport canary, not a second full COM
                # acceptance test. The factory event below is the proof.
                Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_CANARY_COM_SMOKE_NONZERO'
            }
        } catch {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_CANARY_LAUNCH_BLOCKED'
        }
        try {
            & logman.exe stop $privateEtwSession -ets | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'PRIVATE_ETW_STOP_FAILED' }
            $privateEtwStopped=$true
            # Read only events with our fixed custom provider GUID; never
            # export the raw ETL file or records, even as Actions artifacts.
            $traceGuid = [Guid]$privateEtwProviderGuid
            $allEvents = @(Get-WinEvent -Path $privateEtwPath -Oldest -ErrorAction Stop)
            $ownEvents = @($allEvents | Where-Object { $_.ProviderId -eq $traceGuid })
            $canaryEvents = @($ownEvents | Where-Object { $_.TimeCreated.ToUniversalTime() -ge $canaryUtc })
            $canaryFactoryCount = @($canaryEvents | Where-Object { $_.Id -eq 100 }).Count
            if ($canaryFactoryCount -lt 1) {
                $privateEtwIssue = 'PRIVATE_ETW_CANARY_NOT_CAPTURED'
                Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_TRACE_INCONCLUSIVE_CANARY_MISSING'
            } else {
                Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_CANARY_CAPTURED_PASS'
            }
            $hostEvents = @($ownEvents | Where-Object { $_.TimeCreated.ToUniversalTime() -lt $canaryUtc })
            $labels = @{
                100='PROTOCOL_FACTORY'; 101='PROTOCOL_INIT'
                102='CREATE_ACCESSOR'; 103='GET_SECURITY_DESCRIPTOR'
                104='BIND_TO_FILTER'; 105='BIND_TO_STREAM'
                107='SECURITY_DESCRIPTOR_REJECTED'; 108='SECURITY_DESCRIPTOR_RETURNED'
                109='SECURITY_DESCRIPTOR_SIZE_PROBE'; 110='USER_SID_ACL_NOTIMPL'
                111='URL_REJECTED'; 112='URL_ACCEPTED'
                113='SYNTHETIC_DOCUMENT_OWNER_ACL_USED'
                120='GET_FILTER_FAILED'; 121='GET_FILTER_SUCCEEDED'
                124='FILTER_CLASS_NOT_REGISTERED'; 125='FILTER_ACCESS_DENIED'
                126='FILTER_INVALID_ARGUMENT'; 127='FILTER_NOT_IMPLEMENTED'
                128='FILTER_FILE_NOT_FOUND'; 129='FILTER_NOT_FOUND'
                130='FILTER_GENERAL_FAILURE'; 131='FILTER_MODULE_NOT_FOUND'
                132='FILTER_OTHER_FAILURE'
                133='PRIVATE_DOCUMENT_CLASS_FALLBACK_STARTED'
                134='PRIVATE_DOCUMENT_CLASS_FALLBACK_FAILED'
                135='PRIVATE_DOCUMENT_CLASS_FALLBACK_SUCCEEDED'
                136='FILTER_NOT_FOUND'
                137='BUILTIN_TEXT_CONTROL_ATTEMPTED'
                138='BUILTIN_TEXT_CONTROL_RESOLVED'
                139='BUILTIN_TEXT_CONTROL_NOT_RESOLVED'
                140='SYNTHETIC_TEXT_FALLBACK_SELECTED'
                141='DOCUMENT_ROOT_TRAILING_SLASH_RECOGNIZED'
                142='REJECTED_PRIVATE_ROOT_CHILD_SHAPE'
                143='REJECTED_PRIVATE_SCHEME_OTHER_SHAPE'
                144='REJECTED_OUTSIDE_PRIVATE_SCHEME'
                145='PERSIST_STREAM_QUERY_FAILED'
                146='BIND_TO_STREAM_FAILED'
                147='PERSIST_STREAM_LOAD_FAILED'
                148='PERSIST_STREAM_LOAD_SUCCEEDED'
                149='BIND_TO_FILTER_FINAL_FAILED'
                150='BIND_TO_FILTER_FINAL_SUCCEEDED'
                151='CONTAINER_FILTER_RETURNED'
                152='GET_DOC_FORMAT'; 153='GET_DOC_FORMAT_SUCCEEDED'
                154='GET_DOC_FORMAT_BUFFER_FAILED'
                155='GET_FILE_NAME'; 156='GET_FILE_NAME_SUCCEEDED'
                157='GET_FILE_NAME_BUFFER_FAILED'
                158='GET_LAST_MODIFIED'
                159='IS_DIRECTORY_YES'; 160='IS_DIRECTORY_NO'
                161='GET_SIZE'
                162='CLOSE_ACCESSOR_HOST_OWNS_RELEASE'
                163='GET_DOCUMENT_CLSID_UNAVAILABLE'
                200='FILTER_FACTORY'; 201='FILTER_INIT'; 202='FILTER_GET_CHUNK'
            }
            foreach ($id in @(100,101,102,103,104,105,107,108,109,110,111,112,113,120,121,124,125,126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159,160,161,162,163,200,201,202)) {
                $count = @($hostEvents | Where-Object { $_.Id -eq $id }).Count
                if ($count -gt 0) {
                    Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_ETW_' + $labels[$id] + '_COUNT=' + $count)
                }
            }
            # Private ETW numeric-header channel: task=upper WORD and
            # event ID=lower WORD of the exact GetFilter HRESULT. Never
            # export raw ETL/PIDs or filter URLs; only error code/count.
            $hrEvents = @($hostEvents | Where-Object {
                $_.Task -ge 32768 -and $_.Task -le 65535 -and
                $_.Id -ge 0 -and $_.Id -le 65535
            })
            foreach ($group in @($hrEvents | Group-Object {
                '0x{0:X4}{1:X4}' -f [int]$_.Task, [int]$_.Id
            })) {
                if ($group.Name -notmatch '^0x[0-9A-F]{8}$') {
                    throw 'PRIVATE_ETW_HRESULT_HEADER_UNEXPECTED_VALUE'
                }
                Write-Output ('NATIVE_SEARCH_HOSTED_PRIVATE_GETFILTER_HRESULT_' +
                    $group.Name + '_COUNT=' + $group.Count)
            }
            if ($hostEvents.Count -eq 0) {
                Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_NO_CALLBACKS_OBSERVED'
            }
        } catch {
            $privateEtwIssue='PRIVATE_ETW_CAPTURE_OR_PARSE_BLOCKED'
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_TRACE_INCONCLUSIVE_READ_ERROR'
        } finally {
            # Remove any session left by a failed stop and delete raw ETL.
            if (-not $privateEtwStopped) {
                & logman.exe stop $privateEtwSession -ets 2>$null | Out-Null
                if ($LASTEXITCODE -ne 0) {
                    Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_EMERGENCY_STOP_INCONCLUSIVE'
                }
            }
            if (Test-Path -LiteralPath $privateEtwPath) {
                Remove-Item -LiteralPath $privateEtwPath -Force -ErrorAction Stop
            }
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_ETW_RAW_TRACE_CLEANED'
        }
    }
    if ($guestWsearchReloadAttempted) {
        # Unload private COM DLLs before removing their CI-only directory.
        # A failure to stop is reported but must not skip registry cleanup.
        try {
            Stop-Service -Name WSearch -Force -ErrorAction Stop
            (Get-Service WSearch -ErrorAction Stop).WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Stopped,
                [TimeSpan]::FromSeconds(65))
            Write-Output 'NATIVE_SEARCH_HOSTED_GUEST_WSEARCH_STOPPED_FOR_PRIVATE_CLEANUP'
        } catch {
            Write-Output 'NATIVE_SEARCH_HOSTED_GUEST_WSEARCH_STOP_FOR_CLEANUP_BLOCKED'
        }
    }
    try {
        if ($createdScheme) {
            Remove-ItemProperty -LiteralPath $protocolPath -Name $scheme -ErrorAction Stop
        }
        if ($createdProg) {
            Remove-Item -LiteralPath $progPath -Recurse -Force -ErrorAction Stop
        }
        if ($createdIndependent) {
            Remove-Item -LiteralPath $independentPath -Recurse -Force -ErrorAction Stop
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
        foreach ($path in $createdDocumentClasses) {
            Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
        }
        foreach ($path in $createdFilterPaths) {
            Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
        }
        if ($createdBinDir) {
            Remove-Item -LiteralPath $privateBinDir -Recurse -Force -ErrorAction Stop
        }
        if (Test-Path -LiteralPath $independentPath) {
            throw 'EPHEMERAL_PRIVATE_INDEPENDENT_PROGID_CLEANUP_FAILED'
        }
        foreach ($path in $createdDocumentClasses) {
            if (Test-Path -LiteralPath $path) {
                throw 'EPHEMERAL_PRIVATE_DOCUMENT_CLASS_CLEANUP_FAILED'
            }
        }
        $after = & (Join-Path $PSScriptRoot 'native-search-fixture-registry-audit.ps1') | ConvertFrom-Json
        if ($after.result -cne 'CLEAN_BASELINE' -or $after.checked_markers -ne 36) {
            throw 'EPHEMERAL_PRIVATE_OS_COM_REGISTRATION_ROLLBACK_FAILED'
        }
        Write-Output 'NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_REGISTRY_CLEANUP_PASS'
    } finally {
        if ($guestWsearchReloadAttempted) {
            # Restore the original Running service state after all CI-only
            # COM/Crawl Scope registrations have been removed.
            Start-Service -Name WSearch -ErrorAction Stop
            (Get-Service WSearch -ErrorAction Stop).WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Running,
                [TimeSpan]::FromSeconds(65))
            Write-Output 'NATIVE_SEARCH_HOSTED_GUEST_WSEARCH_RESTORED_RUNNING'
        }
    }
}
if ($privateEtwIssue) { throw $privateEtwIssue }
