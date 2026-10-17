# Read-only inspection of the two Windows Search isolation hosts.
# Only fixed private DLL basenames are matched; never disclose module paths,
# PIDs, process owners, user filenames, argument lists or content.
[CmdletBinding()]
param([switch]$SelfTest)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Classify-PrivateModule {
    param([int]$Observed, [int]$Hosts, [int]$Inaccessible)
    if ($Observed -gt 0) { return 'OBSERVED' }
    if ($Hosts -eq 0) { return 'NO_SEARCH_HOST_PROCESS' }
    if ($Inaccessible -gt 0) { return 'INCONCLUSIVE_ACCESS_DENIED' }
    return 'NOT_OBSERVED'
}

if ($SelfTest) {
    $cases = @(
        [pscustomobject]@{observed=1;hosts=2;inaccessible=1;expected='OBSERVED'},
        [pscustomobject]@{observed=0;hosts=0;inaccessible=0;expected='NO_SEARCH_HOST_PROCESS'},
        [pscustomobject]@{observed=0;hosts=2;inaccessible=1;expected='INCONCLUSIVE_ACCESS_DENIED'},
        [pscustomobject]@{observed=0;hosts=2;inaccessible=0;expected='NOT_OBSERVED'}
    )
    foreach ($case in $cases) {
        $got = Classify-PrivateModule -Observed $case.observed -Hosts $case.hosts -Inaccessible $case.inaccessible
        if ($got -cne $case.expected) { throw 'NATIVE_SEARCH_HOST_PRIVATE_DLL_CLASSIFIER_FAILED' }
    }
    Write-Output 'NATIVE_SEARCH_HOST_PRIVATE_DLL_READONLY_SELFTEST_PASS'
    return
}

$fixedTargets = @(
    [pscustomobject]@{ process='SearchProtocolHost'; dll='protocol_fixture.dll' },
    [pscustomobject]@{ process='SearchFilterHost'; dll='fixture_filter.dll' }
)
$results = @()
foreach ($target in $fixedTargets) {
    $hosts = @(Get-Process -Name $target.process -ErrorAction SilentlyContinue)
    $matched = 0
    $denied = 0
    foreach ($hostProcess in $hosts) {
        try {
            # Read-only module enumeration; compare basename only, never
            # include the process ID, usernames or full DLL paths in output.
            $loaded = @($hostProcess.Modules)
            if (@($loaded | Where-Object {
                $_.ModuleName -ceq $target.dll
            }).Count -gt 0) { $matched++ }
        } catch {
            $denied++
        } finally {
            $hostProcess.Dispose()
        }
    }
    $results += [pscustomobject]@{
        host = $target.process
        private_module = $target.dll
        state = Classify-PrivateModule -Observed $matched -Hosts $hosts.Count -Inaccessible $denied
        processes_observed = $hosts.Count
        inaccessible_processes = $denied
        matching_processes = $matched
    }
}
[ordered]@{
    schema=1
    mode='LIVE_READ_ONLY'
    result='DIAGNOSTIC_ONLY'
    host_modules=$results
    registry_modified=$false
    service_modified=$false
    debugger_attached=$false
    private_dll_injection_performed=$false
    user_paths_exported=$false
    native_start_accepted=$false
    native_win_s_accepted=$false
    release_eligible=$false
} | ConvertTo-Json -Depth 6
