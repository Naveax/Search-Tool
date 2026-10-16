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
    # Read-only corroboration of visibility with the sole ephemeral owner.
    # Booleans, no SID or actual row/URL is emitted.
    query_user_matches_fixture_owner=$false
    fixture_owner_source='NOT_CHECKED'
    catalog_query_visible_count=-1
    catalog_total_count_observation='NOT_CHECKED'
    # Non-identifying provider sanity check, never selecting item URLs.
    provider_numeric_row_observation='NOT_CHECKED'
    provider_numeric_row_error_hresult='NOT_RECORDED'
    raw_urls_exported=$false
    raw_sid_exported=$false
    release_eligible=$false
}
# The guest runner recorded the fixed synthetic owner in this one
# private CLSID. Compare identity ONLY; never print either SID.
try {
    $owner = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Classes\CLSID\{987AD765-D904-4F10-A650-915D8F4C0287}' -Name FixtureOwnerSid -ErrorAction Stop).FixtureOwnerSid
    $queryUser = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    if ($owner -match '^S-1-[0-9]+(?:-[0-9]+)+$' -and $owner.Length -le 180 -and
        $queryUser -match '^S-1-[0-9]+(?:-[0-9]+)+$') {
        $report.query_user_matches_fixture_owner = [bool]([string]::Equals(
            $owner, $queryUser, [StringComparison]::OrdinalIgnoreCase))
        $report.fixture_owner_source = 'VALID_PRIVATE_TEST_OWNER'
    } else {
        $report.fixture_owner_source = 'INVALID_PRIVATE_TEST_OWNER'
    }
} catch {
    $report.fixture_owner_source = 'PRIVATE_TEST_OWNER_NOT_READABLE'
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

    # ADO/OLE DB providers may refuse a second command while any earlier
    # DataReader remains active. Close the narrow private query first,
    # otherwise a provider-busy failure masquerades as a COUNT limitation.
    $reader.Dispose()
    $reader = $null
    # Independent provider health control: query only an aggregate scalar,
    # never enumerate or read ANY non-fixture URL or metadata. This measures
    # rows visible to the same user token running our private-scheme SQL.
    $control = $connection.CreateCommand()
    $control.CommandText = 'SELECT COUNT(*) FROM SYSTEMINDEX'
    $control.CommandTimeout = 12
    try {
        $value = $control.ExecuteScalar()
        if ($null -ne $value -and $value -is [ValueType] -and
            [long]$value -ge 0 -and [long]$value -le 1000000000) {
            $report.catalog_query_visible_count = [long]$value
            $report.catalog_total_count_observation = 'SCALAR_COUNT_OK'
        } else {
            $report.catalog_total_count_observation = 'INVALID_SCALAR_COUNT'
        }
    } catch {
        $report.catalog_total_count_observation = 'SCALAR_COUNT_UNSUPPORTED_OR_FAILED'
    }

    # The Windows Search SQL dialect may reject COUNT(*). That alone
    # does not prove the provider cannot return any visible indexed row.
    # A separate TOP 1 numeric rank projection reads NO URL/filename,
    # and emits only whether a scalar value was returned.
    $rankProbe = $connection.CreateCommand()
    $rankProbe.CommandText = 'SELECT TOP 1 System.Search.Rank FROM SYSTEMINDEX'
    $rankProbe.CommandTimeout = 12
    try {
        $numericReader = $rankProbe.ExecuteReader()
        try {
            if ($numericReader.Read()) {
                $report.provider_numeric_row_observation = if ($numericReader.IsDBNull(0)) {
                    'ROW_VISIBLE_NUMERIC_NULL'
                } elseif ($numericReader.GetValue(0) -is [ValueType]) {
                    'ROW_VISIBLE_NUMERIC_VALUE'
                } else {
                    'ROW_VISIBLE_UNEXPECTED_VALUE_TYPE'
                }
            } else {
                $report.provider_numeric_row_observation = 'NO_ROWS_VISIBLE_TO_QUERY_TOKEN'
            }
        } finally {
            $numericReader.Dispose()
        }
    } catch {
        $report.provider_numeric_row_observation = 'NUMERIC_QUERY_UNSUPPORTED_OR_FAILED'
        # Numeric HRESULT only, never a provider exception message that
        # could contain user paths/queries.
        $code = [Runtime.InteropServices.Marshal]::GetHRForException($_.Exception)
        $report.provider_numeric_row_error_hresult = '0x{0:X8}' -f ([uint32]($code -band 0xffffffffL))
    }
} catch {
    # Never emit error messages: an OLE DB failure may contain the query.
    $report.observation = 'QUERY_FAILED_OR_RESULT_UNTRUSTED'
} finally {
    if ($null -ne $reader) { $reader.Dispose() }
    if ($null -ne $connection) { $connection.Dispose() }
}
[pscustomobject]$report | ConvertTo-Json -Compress -Depth 4
