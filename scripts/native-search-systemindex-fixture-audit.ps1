# Read-only, fixed-URL Windows SystemIndex evidence for the synthetic protocol
# fixture. No crawling, registration, shell opening, file reading or UI input.
# Live OLE DB execution uses Windows PowerShell 5.1 (Desktop/.NET Framework).
[CmdletBinding()]
param(
    [ValidateRange(1, 15)][int]$TimeoutSeconds = 8,
    [switch]$SelfTest
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$syntheticUrl = 'stfixture-proto-test://root/document'
$query = "SELECT TOP 2 System.ItemUrl FROM SYSTEMINDEX WHERE System.ItemUrl = '$syntheticUrl'"
$provider = "Provider=Search.CollatorDSO.1;Extended Properties='Application=Windows';"

function Assert-ExactSyntheticQuery {
    param([Parameter(Mandatory)][string]$Sql)
    if ($Sql -cne "SELECT TOP 2 System.ItemUrl FROM SYSTEMINDEX WHERE System.ItemUrl = 'stfixture-proto-test://root/document'") {
        throw 'SYSTEMINDEX_FIXTURE_AUDIT_REJECTS_ARBITRARY_SQL'
    }
}

function Measure-SyntheticRows {
    param([string[]]$Rows, [bool]$QuerySucceeded)
    if (-not $QuerySucceeded) { return 'QUERY_FAILED' }
    if ($null -eq $Rows -or $Rows.Count -eq 0) { return 'NOT_FOUND' }
    if ($Rows.Count -ne 1) { return 'DUPLICATE_OR_UNBOUNDED_ROWS' }
    if ($Rows[0] -cne 'stfixture-proto-test://root/document') {
        return 'WRONG_URL'
    }
    return 'FOUND_SINGLE_FIXTURE_ROW'
}

Assert-ExactSyntheticQuery -Sql $query

if ($SelfTest) {
    $cases = @(
        [pscustomobject]@{ rows=@(); ok=$true; expected='NOT_FOUND' },
        [pscustomobject]@{ rows=@($syntheticUrl); ok=$true; expected='FOUND_SINGLE_FIXTURE_ROW' },
        [pscustomobject]@{ rows=@($syntheticUrl,$syntheticUrl); ok=$true; expected='DUPLICATE_OR_UNBOUNDED_ROWS' },
        [pscustomobject]@{ rows=@('file:///C:/Users/user/secret'); ok=$true; expected='WRONG_URL' },
        [pscustomobject]@{ rows=@($syntheticUrl); ok=$false; expected='QUERY_FAILED' }
    )
    foreach ($case in $cases) {
        $observed = Measure-SyntheticRows -Rows $case.rows -QuerySucceeded $case.ok
        if ($observed -cne $case.expected) {
            throw ('SYSTEMINDEX_FIXTURE_CLASSIFIER_FAILED: ' + $case.expected)
        }
    }
    $arbitraryQueryRejected = $false
    try {
        Assert-ExactSyntheticQuery -Sql 'SELECT TOP 2 System.ItemUrl FROM SYSTEMINDEX'
    } catch {
        $arbitraryQueryRejected = ($_.Exception.Message -ceq 'SYSTEMINDEX_FIXTURE_AUDIT_REJECTS_ARBITRARY_SQL')
    }
    if (-not $arbitraryQueryRejected) { throw 'SYSTEMINDEX_FIXTURE_QUERY_GUARD_FAILED' }
    Write-Output 'NATIVE_SYSTEMINDEX_EXACT_FIXTURE_READONLY_SELFTEST_PASS'
    return
}

$report = [ordered]@{
    schema = 1
    mode = 'LIVE_READ_ONLY'
    candidate_url = $syntheticUrl
    provider = 'Search.CollatorDSO.1'
    query_scope = 'EXACT_SYNTHETIC_URL_ONLY'
    wsearch = 'UNKNOWN'
    result = 'BLOCKED'
    reason = 'NOT_CHECKED'
    systemindex_observation = 'QUERY_FAILED'
    query_rows = 0
    elapsed_ms = 0
    registration_performed = $false
    native_windows_search_modified = $false
    user_file_content_exported = $false
    explorer_accepted = $false
    native_start_accepted = $false
    native_win_s_accepted = $false
    release_eligible = $false
    acceptance = 'NOT_ACCEPTED'
}

try {
    $report.wsearch = [string](Get-Service WSearch -ErrorAction Stop).Status
} catch {
    $report.reason = 'WSEARCH_STATE_UNAVAILABLE'
}
if ($report.wsearch -cne 'Running') {
    if ($report.reason -ceq 'NOT_CHECKED') { $report.reason = 'WSEARCH_NOT_RUNNING' }
    $report | ConvertTo-Json -Depth 5
    return
}

# .NET Core PowerShell does not expose the Windows .NET Framework's OLE DB
# query provider. Never fall back to an arbitrary external binary or shell.
if ($PSVersionTable.PSEdition -cne 'Desktop') {
    $report.reason = 'LIVE_QUERY_REQUIRES_WINDOWS_POWERSHELL_5_1'
    $report | ConvertTo-Json -Depth 5
    return
}

$watch = [System.Diagnostics.Stopwatch]::StartNew()
$connection = $null
$reader = $null
$rows = New-Object 'System.Collections.Generic.List[string]'
try {
    $connection = New-Object System.Data.OleDb.OleDbConnection($provider)
    $connection.Open()
    $command = $connection.CreateCommand()
    $command.CommandText = $query
    $command.CommandTimeout = $TimeoutSeconds
    $reader = $command.ExecuteReader()
    while ($rows.Count -lt 2 -and $reader.Read()) {
        if ($reader.IsDBNull(0)) {
            $rows.Add('')
        } else {
            $rows.Add([string]$reader.GetValue(0))
        }
    }
    $report.query_rows = $rows.Count
    $report.systemindex_observation = Measure-SyntheticRows -Rows $rows.ToArray() -QuerySucceeded $true
    $report.reason = if ($report.systemindex_observation -ceq 'FOUND_SINGLE_FIXTURE_ROW') {
        'FIXTURE_INDEXED_UI_NOT_PROVEN'
    } elseif ($report.systemindex_observation -ceq 'NOT_FOUND') {
        'SYNTHETIC_FIXTURE_NOT_INDEXED'
    } else {
        'SYSTEMINDEX_FIXTURE_RESULT_INVALID'
    }
} catch {
    $report.reason = 'SYSTEMINDEX_EXACT_URL_QUERY_FAILED'
    $report.systemindex_observation = 'QUERY_FAILED'
} finally {
    if ($null -ne $reader) { $reader.Dispose() }
    if ($null -ne $connection) { $connection.Dispose() }
    $watch.Stop()
    $report.elapsed_ms = $watch.ElapsedMilliseconds
}
# An indexed fixture is only a SystemIndex-level observation. There is no
# inference to Explorer, taskbar Search, Win/Start or Win+S presentation.
$report | ConvertTo-Json -Depth 5
