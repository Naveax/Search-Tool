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
            if ($line -match '^CATALOG_(STATUS|PAUSED_REASON|PENDING_INCREMENTAL|PENDING_NOTIFICATIONS|PENDING_HIGH_PRIORITY)=(-?[0-9]+)$') {
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
        Measure-PrivateSearchHostModules
        $directIndexed = $false
        foreach ($attempt in 1..7) {
            $directObservation = & (Join-Path $PSScriptRoot 'native-search-systemindex-fixture-audit.ps1') | ConvertFrom-Json
            if ($directObservation.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
                $directIndexed = $true
                break
            }
            Start-Sleep -Seconds 4
        }
        Measure-PrivateSearchHostModules
        if ($directIndexed) {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_INDEXED'
        } else {
            Write-Output 'NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_NOT_PROVEN'
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
                200='FILTER_FACTORY'; 201='FILTER_INIT'; 202='FILTER_GET_CHUNK'
            }
            foreach ($id in @(100,101,102,103,104,105,107,108,109,110,111,112,113,120,121,124,125,126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,200,201,202)) {
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
