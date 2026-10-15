# Read-only inventory for the *private, unregistered* Windows Search fixture.
# A CLEAN_BASELINE only means listed registry markers were absent; it is
# not proof of a clean crawl scope, restored snapshot or native UI support.
[CmdletBinding()]
param([switch]$SelfTest)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$scheme = 'stfixture-proto-test'
$ids = @(
    '{987AD765-D904-4F10-A650-915D8F4C0287}',
    '{44A4B197-7237-4958-A1D0-7CB26E804165}',
    '{5965EB3B-44AA-405F-9C60-1364074DF297}'
)

function Get-FixedRegistryTargets {
    param([string]$Scheme, [string[]]$Clsids)
    $targets = New-Object 'System.Collections.Generic.List[object]'
    foreach ($view in @('Registry64', 'Registry32')) {
        foreach ($hive in @('LocalMachine', 'CurrentUser')) {
            $targets.Add([pscustomobject]@{
                hive=$hive; view=$view
                subkey='SOFTWARE\Microsoft\Windows Search\ProtocolHandlers'
                type='value'; name=$Scheme
            })
            $targets.Add([pscustomobject]@{
                hive=$hive; view=$view
                subkey=('SOFTWARE\Microsoft\Windows Search\ProtocolHandlers\' + $Scheme)
                type='key'; name=''
            })
            foreach ($clsid in $Clsids) {
                $targets.Add([pscustomobject]@{
                    hive=$hive; view=$view
                    subkey=('SOFTWARE\Classes\CLSID\' + $clsid)
                    type='key'; name=''
                })
            }
            foreach ($extension in @('.stfixture', '.stfixture-root')) {
                $targets.Add([pscustomobject]@{
                    hive=$hive; view=$view
                    subkey=('SOFTWARE\Classes\' + $extension)
                    type='key'; name=''
                })
            }
            # This does not claim a registered Shell namespace exists.
            # Only look for an accidental fixture-specific key or value.
            $targets.Add([pscustomobject]@{
                hive=$hive; view=$view
                subkey=('SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Desktop\NameSpace\' + $Clsids[0])
                type='key'; name=''
            })
            $targets.Add([pscustomobject]@{
                hive=$hive; view=$view
                subkey='SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Shell Extensions\Approved'
                type='value'; name=$Clsids[0]
            })
        }
    }
    return $targets.ToArray()
}

function Get-FootprintDecision {
    param([string[]]$States)
    if ($States.Count -eq 0 -or @($States | Where-Object { $_ -eq 'ERROR' }).Count -gt 0) {
        return 'BLOCKED_INCOMPLETE'
    }
    if (@($States | Where-Object { $_ -eq 'PRESENT' }).Count -gt 0) {
        return 'BLOCKED_PRIVATE_REGISTRATION_DETECTED'
    }
    if (@($States | Where-Object { $_ -ne 'ABSENT' }).Count -gt 0) {
        return 'BLOCKED_INCOMPLETE'
    }
    return 'CLEAN_BASELINE'
}

$targets = @(Get-FixedRegistryTargets -Scheme $scheme -Clsids $ids)
if ($SelfTest) {
    if ($targets.Count -ne 36) { throw 'UNEXPECTED_REGISTRY_TARGET_COUNT' }
    if ((@($targets | Where-Object { $_.name -eq 'stfixture-proto-test' -and
                   $_.type -eq 'value' })).Count -ne 4) {
        throw 'PROTOCOL_HANDLER_VALUE_NOT_CHECKED_IN_ALL_VIEWS'
    }
    foreach ($case in @(
        [pscustomobject]@{states=@('ABSENT');expected='CLEAN_BASELINE'},
        [pscustomobject]@{states=@('ABSENT','PRESENT');expected='BLOCKED_PRIVATE_REGISTRATION_DETECTED'},
        [pscustomobject]@{states=@('ERROR');expected='BLOCKED_INCOMPLETE'},
        [pscustomobject]@{states=@('ABSENT','UNEXPECTED');expected='BLOCKED_INCOMPLETE'}
    )) {
        if ((Get-FootprintDecision -States $case.states) -cne $case.expected) {
            throw ('REGISTRY_FOOTPRINT_FAIL_CLOSED_SELFTEST: ' + $case.expected)
        }
    }
    Write-Output 'NATIVE_SEARCH_PRIVATE_REGISTRY_FOOTPRINT_SELFTEST_PASS'
    return
}

$observed = New-Object 'System.Collections.Generic.List[object]'
foreach ($target in $targets) {
    $state = 'ABSENT'
    $registry = $null
    $key = $null
    try {
        $hive = [Enum]::Parse([Microsoft.Win32.RegistryHive], [string]$target.hive)
        $view = [Enum]::Parse([Microsoft.Win32.RegistryView], [string]$target.view)
        $registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, $view)
        $key = $registry.OpenSubKey([string]$target.subkey, $false)
        if ($null -ne $key) {
            if ($target.type -eq 'key') {
                $state = 'PRESENT'
            } elseif ($key.GetValueNames() -ccontains [string]$target.name) {
                $state = 'PRESENT'
            }
        }
    } catch {
        # Unauthorized or inaccessible hives cannot be called clean.
        $state = 'ERROR'
    } finally {
        if ($null -ne $key) { $key.Dispose() }
        if ($null -ne $registry) { $registry.Dispose() }
    }
    # Only the fixed marker names and presence are returned. No value data,
    # COM paths, account SIDs, file paths or unrelated registry keys.
    $observed.Add([pscustomobject]@{
        hive=$target.hive; view=$target.view; type=$target.type
        path=$target.subkey; name=$target.name; state=$state
    })
}
$states = @($observed | ForEach-Object { $_.state })
[ordered]@{
    schema=1
    mode='LIVE_READ_ONLY'
    fixture_scheme=$scheme
    result=(Get-FootprintDecision -States $states)
    checked_markers=$targets.Count
    present_markers=@($observed | Where-Object { $_.state -eq 'PRESENT' })
    errors=@($observed | Where-Object { $_.state -eq 'ERROR' })
    registry_write_performed=$false
    com_registration_performed=$false
    search_service_modified=$false
    crawl_scope_verified=$false
    other_user_hives_verified=$false
    systemindex_verified=$false
    native_search_accepted=$false
    release_eligible=$false
} | ConvertTo-Json -Depth 6
