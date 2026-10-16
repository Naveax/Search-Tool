# Read-only, CI-only, privacy-minimal Windows Search gatherer diagnostic.
# NEVER exports raw Application event XML, URLs, users, SIDs, event messages
# or other applications' indexed paths.
[CmdletBinding()]
param(
    [switch]$SelfTest,
    [string]$SinceUtc
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Convert-PrivateSearchEvent {
    param([Parameter(Mandatory)][xml]$Xml)
    $provider = $Xml.SelectSingleNode('/*[local-name()="Event"]/*[local-name()="System"]/*[local-name()="Provider"]')
    if ($null -eq $provider -or $provider.GetAttribute('Name') -cne 'Microsoft-Windows-Search') { return $null }
    $data = @($Xml.SelectNodes('/*[local-name()="Event"]/*[local-name()="EventData"]/*[local-name()="Data"]'))
    $urls = @($data | Where-Object { $_.GetAttribute('Name') -ceq 'URL' })
    if ($urls.Count -ne 1) { return $null }
    $url = [string]$urls[0].InnerText
    if (-not $url.StartsWith('stfixture-proto-test://', [StringComparison]::OrdinalIgnoreCase)) {
        return $null
    }
    $details = @($data | Where-Object { $_.GetAttribute('Name') -ceq 'ExtraInfo' })
    $code = 'NOT_RECORDED'
    if ($details.Count -eq 1) {
        $m = [regex]::Match([string]$details[0].InnerText, '(?i)0x[0-9a-f]{8}')
        if ($m.Success) { $code = $m.Value.ToUpperInvariant() }
    }
    $eventIdNode = $Xml.SelectSingleNode('/*[local-name()="Event"]/*[local-name()="System"]/*[local-name()="EventID"]')
    if ($null -eq $eventIdNode) { return $null }
    $id = [int]$eventIdNode.InnerText
    if ($id -lt 1 -or $id -gt 65535) { return $null }
    # No raw URL is returned: only the stable numeric error shape.
    return [pscustomobject]@{ event_id = $id; hresult = $code }
}

function Measure-SyntheticEvents {
    param([object[]]$Records)
    $found = New-Object 'System.Collections.Generic.List[object]'
    foreach ($record in @($Records)) {
        $parsed = Convert-PrivateSearchEvent -Xml $record
        if ($null -ne $parsed) { $found.Add($parsed) }
    }
    $groups = @($found | Group-Object {
        'EVENT_{0}_HRESULT_{1}' -f $_.event_id, $_.hresult
    } | Sort-Object Name)
    $counts = [ordered]@{}
    foreach ($group in $groups) {
        if ($group.Name -cnotmatch '^EVENT_[0-9]{1,5}_HRESULT_(?:0X[0-9A-F]{8}|NOT_RECORDED)$') {
            throw 'PRIVATE_GATHERER_UNEXPECTED_GROUP_KEY'
        }
        $counts[$group.Name] = [int]$group.Count
    }
    return [pscustomobject]@{
        matched_private_events = [int]$found.Count
        groups = $counts
    }
}

if ($SelfTest) {
    $synthetic = [xml]@'
<Event>
 <System><Provider Name="Microsoft-Windows-Search"/><EventID>3036</EventID></System>
 <EventData>
  <Data Name="URL">stfixture-proto-test://root/document/</Data>
  <Data Name="ExtraInfo">Context: SystemIndex HRESULT : 0x80040d16</Data>
 </EventData>
</Event>
'@
    $other = [xml]@'
<Event>
 <System><Provider Name="Microsoft-Windows-Search"/><EventID>3036</EventID></System>
 <EventData>
  <Data Name="URL">file:///C:/Users/Private/secret.txt</Data>
  <Data Name="ExtraInfo">HRESULT : 0x80070005</Data>
 </EventData>
</Event>
'@
    $result = Measure-SyntheticEvents -Records @($synthetic, $synthetic, $other)
    if ($result.matched_private_events -ne 2 -or
        $result.groups['EVENT_3036_HRESULT_0X80040D16'] -ne 2 -or
        $result.groups.Count -ne 1) {
        throw 'PRIVATE_GATHERER_SOURCE_FILTER_OR_COUNTER_TEST_FAILED'
    }
    Write-Output 'NATIVE_SEARCH_PRIVATE_GATHERER_ZERO_PAYLOAD_SELFTEST_PASS'
    return
}

# Require the same single disposable, Microsoft-hosted ARM64 Windows 11
# VM identity as the separate mutating guest gate; this audit itself is
# read-only and refuses all other machines.
$os = Get-CimInstance Win32_OperatingSystem
$system = Get-CimInstance Win32_ComputerSystem
if ([string]$env:GITHUB_ACTIONS -cne 'true' -or
    [string]$env:GITHUB_REPOSITORY -cne 'Naveax/Search-Tool' -or
    [string]$env:GITHUB_JOB -cne 'windows11-desktop-native-feasibility' -or
    [string]$env:RUNNER_ENVIRONMENT -cne 'github-hosted' -or
    [string]$env:RUNNER_ARCH -cne 'ARM64' -or
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -cne 'Arm64' -or
    [string]$os.Caption -notmatch 'Windows 11' -or
    [string]$system.Manufacturer -notmatch 'Microsoft Corporation' -or
    [string]$system.Model -notmatch 'Virtual Machine') {
    throw 'PRIVATE_GATHERER_AUDIT_HOSTED_GUEST_REQUIRED'
}
if ([string]::IsNullOrWhiteSpace($SinceUtc)) {
    throw 'PRIVATE_GATHERER_AUDIT_REQUIRES_EXPLICIT_START_TIME'
}
$begin = [datetime]::MinValue
if (-not [datetime]::TryParse($SinceUtc,
    [Globalization.CultureInfo]::InvariantCulture,
    [Globalization.DateTimeStyles]::AssumeUniversal -bor
        [Globalization.DateTimeStyles]::AdjustToUniversal, [ref]$begin)) {
    throw 'PRIVATE_GATHERER_INVALID_START_TIMESTAMP'
}
$age = [datetime]::UtcNow - $begin.ToUniversalTime()
if ($age.TotalSeconds -lt 0 -or $age.TotalMinutes -gt 20) {
    throw 'PRIVATE_GATHERER_START_TIMESTAMP_OUTSIDE_TEST_WINDOW'
}
$events = @()
try {
    $events = @(Get-WinEvent -FilterHashtable @{
        LogName = 'Application'
        ProviderName = 'Microsoft-Windows-Search'
        StartTime = $begin
    } -MaxEvents 1200 -ErrorAction Stop)
} catch {
    if ($_.FullyQualifiedErrorId -notlike 'NoMatchingEventsFound*') {
        throw 'PRIVATE_GATHERER_EVENT_QUERY_FAILED'
    }
}
if ($events.Count -ge 1200) {
    throw 'PRIVATE_GATHERER_EVENT_LIMIT_REACHED_INCOMPLETE'
}
$xml = New-Object 'System.Collections.Generic.List[object]'
foreach ($event in $events) {
    try { $xml.Add([xml]$event.ToXml()) }
    catch { throw 'PRIVATE_GATHERER_XML_PARSE_INCOMPLETE' }
}
$summary = Measure-SyntheticEvents -Records $xml.ToArray()
[pscustomobject]([ordered]@{
    schema = 1
    source = 'APPLICATION_WINDOWS_SEARCH_GATHERER'
    mode = 'READ_ONLY_EXACT_PRIVATE_SCHEME_ONLY'
    coverage = 'BOUNDED_EVENT_LOG_WINDOW'
    matched_private_events = $summary.matched_private_events
    outcome = if ($summary.matched_private_events -gt 0) { 'PRIVATE_ERRORS_OBSERVED' }
              else { 'NO_MATCHING_PRIVATE_EVENTS_OBSERVED' }
    groups = $summary.groups
    raw_urls_exported = $false
    raw_event_xml_exported = $false
    service_or_index_modified = $false
    native_search_accepted = $false
    release_eligible = $false
}) | ConvertTo-Json -Depth 6 -Compress
