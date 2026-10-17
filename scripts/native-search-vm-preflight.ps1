# READ-ONLY P0 host/guest readiness check for a future disposable-VM Search proof.
# Does not register COM, change the shell, query private files or authorize installation.
[CmdletBinding()]
param(
    [switch]$SelfTest,
    [string]$ManifestPath
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Test-NativeVmGate {
    param(
        [Parameter(Mandatory)]$HostFacts,
        $Manifest
    )
    $blocks = New-Object 'System.Collections.Generic.List[string]'
    $guest = [bool]($HostFacts.manufacturer -match 'VMware|QEMU|Xen|innotek|Oracle|Parallels|Microsoft Corporation' -and
        $HostFacts.model -match 'Virtual|VMware|KVM|QEMU|HVM|Xen|Parallels')
    if (-not $guest) { $blocks.Add('DISPOSABLE_VM_NOT_PROVEN') }
    if ($HostFacts.os_family -notin @('Windows10', 'Windows11')) {
        $blocks.Add('WINDOWS_10_11_REQUIRED')
    }
    if ($HostFacts.wsearch -cne 'Running') { $blocks.Add('WSEARCH_NOT_RUNNING') }

    if ($null -eq $Manifest) {
        $blocks.Add('VM_EVIDENCE_MANIFEST_MISSING')
    } else {
        # ConvertFrom-Json may yield a sparse object. Treat missing fields as
        # invalid evidence instead of throwing or accidentally accepting them.
        $properties = @{}
        foreach ($name in @('schema', 'scope', 'os_family', 'operator_approved',
                            'snapshot_id', 'rollback_rehearsal_id')) {
            $entry = $Manifest.PSObject.Properties[$name]
            $properties[$name] = if ($null -eq $entry) { $null } else { $entry.Value }
        }
        if ($properties.schema -cne 'native-search-vm-evidence-v1' -or
            $properties.scope -cne 'stfixture-proto-test' -or
            $properties.os_family -cne $HostFacts.os_family) {
            $blocks.Add('VM_EVIDENCE_SCOPE_OR_OS_MISMATCH')
        }
        if ($properties.operator_approved -isnot [bool] -or
            $properties.operator_approved -ne $true) {
            $blocks.Add('EXPLICIT_TEST_AUTHORIZATION_MISSING')
        }
        if ([string]::IsNullOrWhiteSpace([string]$properties.snapshot_id) -or
            [string]::IsNullOrWhiteSpace([string]$properties.rollback_rehearsal_id) -or
            [string]$properties.snapshot_id -ceq [string]$properties.rollback_rehearsal_id) {
            $blocks.Add('DISTINCT_SNAPSHOT_AND_ROLLBACK_RECEIPTS_REQUIRED')
        }
    }
    [pscustomobject]@{
        decision = $(if ($blocks.Count) { 'BLOCKED' } else { 'PREFLIGHT_ONLY' })
        blockers = @($blocks.ToArray())
        guest_signature_detected = $guest
        os_family = [string]$HostFacts.os_family
        wsearch_state = [string]$HostFacts.wsearch
        evidence_receipts_externally_verified = $false
        registration_performed = $false
        search_service_modified = $false
        systemindex_accepted = $false
        explorer_accepted = $false
        start_accepted = $false
        win_s_accepted = $false
        release_eligible = $false
        permitted_actions = @('READ_ONLY_BASELINE')
    }
}

if ($SelfTest) {
    $physical = [pscustomobject]@{
        manufacturer = 'ASUSTeK COMPUTER INC.'; model = 'ROG STRIX'
        os_family = 'Windows11'; wsearch = 'Running'
    }
    $guest10 = [pscustomobject]@{
        manufacturer = 'Microsoft Corporation'; model = 'Virtual Machine'
        os_family = 'Windows10'; wsearch = 'Running'
    }
    $guest11 = [pscustomobject]@{
        manufacturer = 'VMware, Inc.'; model = 'VMware Virtual Platform'
        os_family = 'Windows11'; wsearch = 'Running'
    }
    $good = [pscustomobject]@{
        schema = 'native-search-vm-evidence-v1'; scope = 'stfixture-proto-test'
        os_family = 'Windows10'; operator_approved = $true
        snapshot_id = 'snapshot-fixture'; rollback_rehearsal_id = 'rollback-fixture'
    }
    $checks = @(
        [pscustomobject]@{ facts = $physical; evidence = $good; reason = 'DISPOSABLE_VM_NOT_PROVEN'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = $guest10; evidence = $null; reason = 'VM_EVIDENCE_MANIFEST_MISSING'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = $guest10; evidence = [pscustomobject]@{}; reason = 'VM_EVIDENCE_SCOPE_OR_OS_MISMATCH'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = $guest10; evidence = $good; reason = ''; decision = 'PREFLIGHT_ONLY' },
        [pscustomobject]@{ facts = $guest11; evidence = $good; reason = 'VM_EVIDENCE_SCOPE_OR_OS_MISMATCH'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = $guest10; evidence = [pscustomobject]@{
            schema='native-search-vm-evidence-v1';scope='stfixture-proto-test'
            os_family='Windows10';operator_approved=$false
            snapshot_id='one';rollback_rehearsal_id='two'
        }; reason = 'EXPLICIT_TEST_AUTHORIZATION_MISSING'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = $guest10; evidence = [pscustomobject]@{
            schema='native-search-vm-evidence-v1';scope='stfixture-proto-test'
            os_family='Windows10';operator_approved=$true
            snapshot_id='same';rollback_rehearsal_id='same'
        }; reason = 'DISTINCT_SNAPSHOT_AND_ROLLBACK_RECEIPTS_REQUIRED'; decision = 'BLOCKED' },
        [pscustomobject]@{ facts = [pscustomobject]@{
            manufacturer='Microsoft Corporation';model='Virtual Machine'
            os_family='Windows10';wsearch='Stopped'
        }; evidence=$good; reason='WSEARCH_NOT_RUNNING';decision='BLOCKED' }
    )
    foreach ($case in $checks) {
        $actual = Test-NativeVmGate -HostFacts $case.facts -Manifest $case.evidence
        if ($actual.decision -cne $case.decision -or
            ($case.reason -and $case.reason -cnotin $actual.blockers) -or
            $actual.release_eligible -or $actual.registration_performed -or
            $actual.search_service_modified -or $actual.start_accepted -or
            $actual.win_s_accepted -or $actual.permitted_actions.Count -ne 1) {
            throw "VM preflight fails closed incorrectly: $($case.reason)"
        }
    }
    Write-Output 'NATIVE_SEARCH_VM_PREFLIGHT_FAIL_CLOSED_SELFTEST_PASS'
    return
}

# These are read-only Win32/CIM and Service Controller queries. HypervisorPresent
# is deliberately NOT used: WSL2/Hyper-V can set it on a physical host.
$osFamily = 'Unsupported'
$manufacturer = ''
$model = ''
$wsearch = 'Unavailable'
try {
    $system = Get-CimInstance Win32_ComputerSystem
    $os = Get-CimInstance Win32_OperatingSystem
    $manufacturer = [string]$system.Manufacturer
    $model = [string]$system.Model
    $build = [int]$os.BuildNumber
    if ($build -ge 22000 -and $os.Caption -match 'Windows 11') {
        $osFamily = 'Windows11'
    } elseif ($build -ge 10240 -and $build -lt 22000 -and
              $os.Caption -match 'Windows 10') {
        $osFamily = 'Windows10'
    }
    $wsearch = [string](Get-Service -Name WSearch -ErrorAction Stop).Status
} catch {
    # Never assume a VM, Windows build or running service when inventory fails.
}
$manifest = $null
if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
    try {
        $manifest = Get-Content -LiteralPath $ManifestPath -Raw -ErrorAction Stop |
            ConvertFrom-Json -ErrorAction Stop
    } catch {
        # Malformed/unreadable evidence is equivalent to no evidence.
    }
}
$report = Test-NativeVmGate -HostFacts ([pscustomobject]@{
    manufacturer=$manufacturer; model=$model
    os_family=$osFamily; wsearch=$wsearch
}) -Manifest $manifest
$report | ConvertTo-Json -Depth 5
