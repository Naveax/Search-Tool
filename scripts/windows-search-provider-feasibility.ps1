# READ-ONLY exploration of the supported Windows Search protocol-handler
# route. No COM registration, shell hook, process launch or user input.
[CmdletBinding()]
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-ProtocolFeasibility {
    param(
        [string]$OsCaption,
        [string]$OsVersion,
        [string]$WSearchState,
        [bool]$NativeStartResultProof,
        [bool]$ExplorerResultProof,
        [bool]$ShellNamespaceProof,
        [bool]$IsolatedInstallRollbackProof
    )
    $supportedTarget = ($OsCaption -match 'Windows (10|11)' -and
        $OsCaption -notmatch 'Server')
    $availableIndex = $supportedTarget -and $WSearchState -eq 'Running'
    # Historical Microsoft Windows 7 connector guidance does NOT prove
    # Windows 10/11 Start/SearchHost renders custom protocol results.
    # Even Explorer/SystemIndex participation is insufficient by itself.
    $accepted = $availableIndex -and $NativeStartResultProof -and
        $ExplorerResultProof -and $ShellNamespaceProof -and
        $IsolatedInstallRollbackProof
    $reason = if (-not $supportedTarget) {
        'WINDOWS_10_11_DESKTOP_TARGET_UNVERIFIED'
    } elseif (-not $availableIndex) {
        'WINDOWS_SEARCH_INDEX_NOT_RUNNING'
    } elseif (-not $ShellNamespaceProof) {
        'SHELL_NAMESPACE_FIXTURE_UNVERIFIED'
    } elseif (-not $ExplorerResultProof) {
        'EXPLORER_PROVIDER_RESULT_UNVERIFIED'
    } elseif (-not $NativeStartResultProof) {
        'WINDOWS_10_11_START_SEARCH_RESULT_UNVERIFIED'
    } elseif (-not $IsolatedInstallRollbackProof) {
        'ISOLATED_INSTALL_ROLLBACK_UNVERIFIED'
    } else {
        'CANDIDATE_FIXTURE_PROVEN'
    }
    return [ordered]@{
        result = if ($accepted) { 'CANDIDATE_PROVEN' } else { 'BLOCKED' }
        reason = $reason
        os_caption = $OsCaption
        os_version = $OsVersion
        wsearch = $WSearchState
        index_service_available = $availableIndex
        shell_namespace_proven = $ShellNamespaceProof
        explorer_results_proven = $ExplorerResultProof
        win10_win11_start_results_proven = $NativeStartResultProof
        isolated_rollback_proven = $IsolatedInstallRollbackProof
        # CANDIDATE_PROVEN is only provider feasibility, NOT release approval.
        release_eligible = $false
    }
}

if ($SelfTest) {
    $blocked = Get-ProtocolFeasibility 'Windows 11 Pro' '10.0.22631' 'Running' $false $true $true $true
    if ($blocked.result -cne 'BLOCKED' -or
        $blocked.reason -cne 'WINDOWS_10_11_START_SEARCH_RESULT_UNVERIFIED') {
        throw 'PROTO_HANDLER_FALSE_NATIVE_START_ACCEPTANCE'
    }
    $blocked = Get-ProtocolFeasibility 'Windows 10 Pro' '10.0.19045' 'Stopped' $true $true $true $true
    if ($blocked.result -cne 'BLOCKED' -or
        $blocked.reason -cne 'WINDOWS_SEARCH_INDEX_NOT_RUNNING') {
        throw 'PROTO_HANDLER_ACCEPTED_STOPPED_INDEX'
    }
    $blocked = Get-ProtocolFeasibility 'Windows Server 2025' '10.0.26100' 'Running' $true $true $true $true
    if ($blocked.result -cne 'BLOCKED') {
        throw 'PROTO_HANDLER_ACCEPTED_SERVER_AS_DESKTOP'
    }
    $proved = Get-ProtocolFeasibility 'Windows 10 Pro' '10.0.19045' 'Running' $true $true $true $true
    if ($proved.result -cne 'CANDIDATE_PROVEN' -or $proved.release_eligible -ne $false) {
        throw 'PROTO_HANDLER_REJECTED_GOOD_TEST_FIXTURE_OR_ENABLED_RELEASE'
    }
    'WINDOWS_SEARCH_PROVIDER_FEASIBILITY_SELFTEST_PASS'
    return
}

$caption = 'UNKNOWN'
$version = 'UNKNOWN'
try {
    $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
    $caption = [string]$os.Caption
    $version = [string]$os.Version
} catch {
    # Keep this a read-only BLOCKED result on unavailable host metadata.
}
$serviceState = 'UNAVAILABLE'
try {
    $serviceState = [string](Get-Service -Name WSearch -ErrorAction Stop).Status
} catch {
    # A service-less test runner cannot establish a native provider.
}
$report = Get-ProtocolFeasibility $caption $version $serviceState $false $false $false $false
[ordered]@{
    schema = 1
    mode = 'LIVE_READ_ONLY'
    candidate = 'ISearchProtocol+IUrlAccessor+IFilter+ShellNamespace'
    registration_performed = $false
    native_windows_search_modified = $false
    microsoft_windows7_connector_docs_do_not_prove_win10_win11 = $true
    supported_extensibility = 'CUSTOM_DATA_STORE_ONLY'
    general_existing_file_ranking_replacement = 'NOT_DOCUMENTED'
    acceptance = 'NOT_ACCEPTED'
    details = $report
    next_required = @(
        'ISOLATED_WIN10_AND_WIN11_NATIVE_SEARCH_FIXTURES',
        'PROPRIETARY_NON_DUPLICATE_DATA_SOURCE',
        'SHELL_NAMESPACE_AND_EXPLORER_RESULT_EVIDENCE',
        'REAL_START_WIN_S_NATIVE_RESULT_VISIBILITY',
        'NO_COMPANION_GUI_OR_SHELL_TAKEOVER',
        'ISOLATED_INSTALL_AND_UNINSTALL_ROLLBACK'
    )
} | ConvertTo-Json -Compress -Depth 6
