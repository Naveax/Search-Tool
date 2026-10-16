# EXACT private-scheme SystemIndex probe in a disposable Win11 ARM64 VM.
# Unlike the per-URL checks, this detects alternate canonical forms.
# It never prints or persists returned URLs, SIDs, file paths or content.
[CmdletBinding()]
param([switch]$SelfTest)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$prefix = 'stfixture-proto-test://'
function Classify-PrivateUrl {
    param([AllowEmptyString()][string]$Value)
    if ($null -eq $Value -or
        -not $Value.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        return 'OUTSIDE_PRIVATE_SCHEME'
    }
    if ($Value -ceq 'stfixture-proto-test://root/document') { return 'EXACT_DOCUMENT' }
    if ($Value -ceq 'stfixture-proto-test://root/document/') { return 'CANONICAL_DOCUMENT' }
    if ($Value -ceq 'stfixture-proto-test://root' -or
        $Value -ceq 'stfixture-proto-test://root/') { return 'PRIVATE_ROOT' }
    if ($Value -cmatch '^stfixture-proto-test://\{S-1-[0-9]+(?:-[0-9]+)+\}/root/document/?$') {
        return 'SID_SCOPED_DOCUMENT'
    }
    return 'OTHER_PRIVATE_URL_SHAPE'
}
function Summarize-PrivateRows {
    param([string[]]$Urls)
    $counts = [ordered]@{
        EXACT_DOCUMENT = 0
        CANONICAL_DOCUMENT = 0
        PRIVATE_ROOT = 0
        SID_SCOPED_DOCUMENT = 0
        OTHER_PRIVATE_URL_SHAPE = 0
    }
    foreach ($url in $Urls) {
        $category = Classify-PrivateUrl -Value $url
        if ($category -ceq 'OUTSIDE_PRIVATE_SCHEME') {
            throw 'PRIVATE_SCHEME_QUERY_RETURNED_UNRELATED_URL'
        }
        $counts[$category]++
    }
    return [pscustomobject]$counts
}
if ($SelfTest) {
    $examples = @(
        'stfixture-proto-test://root/document'
        'stfixture-proto-test://root/document/'
        'stfixture-proto-test://root'
        'stfixture-proto-test://{S-1-5-21-12-34-56-1001}/root/document'
        'stfixture-proto-test://root/surprise'
    )
    $summary = Summarize-PrivateRows -Urls $examples
    if ($summary.EXACT_DOCUMENT -ne 1 -or
        $summary.CANONICAL_DOCUMENT -ne 1 -or
        $summary.PRIVATE_ROOT -ne 1 -or
        $summary.SID_SCOPED_DOCUMENT -ne 1 -or
        $summary.OTHER_PRIVATE_URL_SHAPE -ne 1) {
        throw 'PRIVATE_INDEX_SCHEMA_COUNTER_SELFTEST_FAILED'
    }
    $rejected = $false
    try {
        $null = Summarize-PrivateRows -Urls @('file:///C:/Users/Other/private.txt')
    } catch {
        $rejected = $_.Exception.Message -ceq 'PRIVATE_SCHEME_QUERY_RETURNED_UNRELATED_URL'
    }
    if (-not $rejected) { throw 'PRIVATE_INDEX_OTHER_USER_URL_REJECTION_FAILED' }
    Write-Output 'NATIVE_SEARCH_PRIVATE_SCHEME_INDEX_SHAPE_SELFTEST_PASS'
    return
}

# Fail closed on developer PCs; no mutation or local index queries.
if ($PSVersionTable.PSEdition -cne 'Desktop') {
    throw 'PRIVATE_SCHEME_QUERY_REQUIRES_WINDOWS_POWERSHELL_5_1'
}
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
    throw 'PRIVATE_SCHEME_QUERY_REQUIRES_DISPOSABLE_WIN11_VM'
}
$report = [ordered]@{
    mode='DISPOSABLE_GUEST_PRIVATE_SCHEME_READ_ONLY'
    observation='QUERY_FAILED'
    matched_count=0
    exact_document=0
    canonical_document=0
    private_root=0
    sid_scoped_document=0
    other_private_url_shape=0
    complete_window=$false
    any_private_row_observed=$false
    raw_urls_exported=$false
    raw_sid_exported=$false
    release_eligible=$false
}
$connection = $null
$reader = $null
$rows = New-Object 'System.Collections.Generic.List[string]'
try {
    $connection = New-Object System.Data.OleDb.OleDbConnection -ArgumentList "Provider=Search.CollatorDSO.1;Extended Properties='Application=Windows';"
    $connection.Open()
    $command = $connection.CreateCommand()
    # Hard-coded private test scheme only. The broader index cannot be
    # queried with caller-supplied parameters or user-visible wildcards.
    $command.CommandText = "SELECT TOP 20 System.ItemUrl FROM SYSTEMINDEX WHERE System.ItemUrl LIKE 'stfixture-proto-test:%'"
    $command.CommandTimeout = 12
    $reader = $command.ExecuteReader()
    while ($reader.Read()) {
        if ($reader.IsDBNull(0)) { throw 'PRIVATE_INDEX_QUERY_NULL_URL' }
        $value = [string]$reader.GetValue(0)
        if ((Classify-PrivateUrl -Value $value) -ceq 'OUTSIDE_PRIVATE_SCHEME') {
            throw 'PRIVATE_INDEX_QUERY_RETURNED_UNRELATED_URL'
        }
        $rows.Add($value)
    }
    $summary = Summarize-PrivateRows -Urls $rows.ToArray()
    $report.matched_count = $rows.Count
    $report.exact_document = $summary.EXACT_DOCUMENT
    $report.canonical_document = $summary.CANONICAL_DOCUMENT
    $report.private_root = $summary.PRIVATE_ROOT
    $report.sid_scoped_document = $summary.SID_SCOPED_DOCUMENT
    $report.other_private_url_shape = $summary.OTHER_PRIVATE_URL_SHAPE
    $report.complete_window = $rows.Count -lt 20
    $report.any_private_row_observed = $rows.Count -gt 0
    $report.observation = if ($rows.Count -ge 20) { 'PRIVATE_ROW_CAP_REACHED_INCOMPLETE' }
        elseif ($rows.Count -gt 0) { 'PRIVATE_SCHEME_ROWS_FOUND' }
        else { 'NO_PRIVATE_SCHEME_ROWS_FOUND' }
} catch {
    # Never emit error messages: an OLE DB failure may contain the query.
    $report.observation = 'QUERY_FAILED_OR_RESULT_UNTRUSTED'
} finally {
    if ($null -ne $reader) { $reader.Dispose() }
    if ($null -ne $connection) { $connection.Dispose() }
}
[pscustomobject]$report | ConvertTo-Json -Compress -Depth 4
