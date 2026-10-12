# Read-only Windows Search (SystemIndex) availability probe.
# Runs under Windows PowerShell 5.1, which includes the .NET Framework OLE DB provider.
# No file names, paths, indexed values, or user queries are read into the report.
[CmdletBinding()]
param(
    [ValidateRange(1, 30)] [int]$TimeoutSeconds = 8,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$provider = "Provider=Search.CollatorDSO.1;Extended Properties='Application=Windows';"
$query = 'SELECT TOP 1 System.ItemName FROM SYSTEMINDEX'

function Assert-ReadOnlyQuery {
    param([Parameter(Mandatory)] [string]$Sql)
    # Never allow a future CLI flag or caller to inject arbitrary SQL.
    if ($Sql -cne 'SELECT TOP 1 System.ItemName FROM SYSTEMINDEX') {
        throw 'NATIVE_SEARCH_AUDIT_UNSAFE_SQL'
    }
}

function Test-TrustedSearchButton([string]$AutomationId, [string]$Role,
                                   [string]$ProcessName, [bool]$TaskbarAncestor) {
    return ($AutomationId -ceq 'SearchButton' -and
            $Role -ceq 'ControlType.Button' -and
            $ProcessName -ieq 'explorer' -and $TaskbarAncestor)
}

Assert-ReadOnlyQuery -Sql $query

if ($SelfTest) {
    if (Test-TrustedSearchButton 'SearchButton' 'ControlType.Button' 'notepad' $true) {
        throw 'NATIVE_SEARCH_AUDIT_UNTRUSTED_PROCESS'
    }
    if (Test-TrustedSearchButton 'SearchButton' 'ControlType.Edit' 'explorer' $true) {
        throw 'NATIVE_SEARCH_AUDIT_INVALID_BUTTON_ROLE'
    }
    if (Test-TrustedSearchButton 'SearchButton' 'ControlType.Button' 'explorer' $false) {
        throw 'NATIVE_SEARCH_AUDIT_MISSING_TASKBAR_ANCESTOR'
    }
    if (-not (Test-TrustedSearchButton 'SearchButton' 'ControlType.Button' 'explorer' $true)) {
        throw 'NATIVE_SEARCH_AUDIT_VALID_BUTTON_REJECTED'
    }
    $rejected = $false
    try {
        Assert-ReadOnlyQuery -Sql 'DELETE FROM SYSTEMINDEX'
    } catch {
        $rejected = ([string]$_.Exception.Message) -eq 'NATIVE_SEARCH_AUDIT_UNSAFE_SQL'
    }
    if (-not $rejected) { throw 'NATIVE_SEARCH_AUDIT_SELFTEST_FAIL' }
    [ordered]@{ schema = 1; result = 'PASS'; mode = 'SELFTEST'; unsafe_sql_rejected = $true; taskbar_identity_guards = 'PASS' } |
        ConvertTo-Json -Compress
    return
}

$report = [ordered]@{
    schema = 1
    mode = 'LIVE_READ_ONLY'
    result = 'BLOCKED'
    wsearch = 'UNAVAILABLE'
    provider = 'Search.CollatorDSO.1'
    system_index = 'UNAVAILABLE'
    query_rows = 0
    elapsed_ms = 0
    no_indexed_values_exported = $true
    taskbar_search_button = [ordered]@{
        found = $false
        automation_id = 'SearchButton'
        x = $null
        y = $null
        width = $null
        height = $null
    }
    reason = $null
}

if ($PSVersionTable.PSEdition -ne 'Desktop') {
    $report.reason = 'WINDOWS_POWERSHELL_5_REQUIRED'
    $report | ConvertTo-Json -Compress
    return
}

try {
    $svc = Get-Service -Name 'WSearch' -ErrorAction Stop
    $report.wsearch = [string]$svc.Status
} catch {
    $report.reason = 'WSEARCH_SERVICE_UNAVAILABLE'
    $report | ConvertTo-Json -Compress
    return
}

if ($report.wsearch -ne 'Running') {
    $report.reason = 'WSEARCH_NOT_RUNNING'
    $report | ConvertTo-Json -Compress
    return
}

$timer = [Diagnostics.Stopwatch]::StartNew()
$connection = $null
$reader = $null
try {
    $connection = New-Object System.Data.OleDb.OleDbConnection($provider)
    $connection.Open()
    $command = $connection.CreateCommand()
    $command.CommandText = $query
    $command.CommandTimeout = $TimeoutSeconds
    $reader = $command.ExecuteReader()
    # Do not inspect, copy or print the result value.
    if ($reader.Read()) { $report.query_rows = 1 }
    $report.system_index = 'PASS'
    $report.result = 'PASS'
} catch {
    $report.reason = 'SYSTEMINDEX_READONLY_QUERY_FAILED'
} finally {
    if ($null -ne $reader) { $reader.Dispose() }
    if ($null -ne $connection) { $connection.Dispose() }
    $timer.Stop()
    $report.elapsed_ms = $timer.ElapsedMilliseconds
}

# Inspect the native taskbar entry point without opening Search or
# interacting with the user's desktop. AutomationId is locale independent.
try {
    Add-Type -AssemblyName UIAutomationClient -ErrorAction Stop
    Add-Type -AssemblyName UIAutomationTypes -ErrorAction Stop
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'SearchButton'
    )
    # AutomationId alone is not unique or an ownership guarantee. A different
    # process may expose the same string; even Explorer can host other buttons.
    # Inspect all matches and accept only an actual Explorer taskbar button.
    $matches = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants, $condition
    )
    foreach ($button in $matches) {
        try {
            if ($button.Current.IsOffscreen) { continue }
            $ownerPid = [int]$button.Current.ProcessId
            $owner = Get-Process -Id $ownerPid -ErrorAction Stop
            $ancestor = $button
            $onTaskbar = $false
            for ($depth = 0; $depth -lt 8; $depth++) {
                $ancestor = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($ancestor)
                if ($null -eq $ancestor) { break }
                if ($ancestor.Current.AutomationId -ceq 'TaskbarFrame' -or
                    $ancestor.Current.ClassName -ceq 'TaskbarFrameRepeater') {
                    $onTaskbar = $true
                    break
                }
            }
            $trusted = Test-TrustedSearchButton `
                -AutomationId $button.Current.AutomationId `
                -Role $button.Current.ControlType.ProgrammaticName `
                -ProcessName $owner.ProcessName -TaskbarAncestor $onTaskbar
            if (-not $trusted) { continue }
            $rect = $button.Current.BoundingRectangle
            if ($rect.Width -le 0 -or $rect.Height -le 0) { continue }
            $report.taskbar_search_button.found = $true
            $report.taskbar_search_button.x = [int][Math]::Round($rect.X)
            $report.taskbar_search_button.y = [int][Math]::Round($rect.Y)
            $report.taskbar_search_button.width = [int][Math]::Round($rect.Width)
            $report.taskbar_search_button.height = [int][Math]::Round($rect.Height)
            break
        } catch {
            # A stale HWND/UIA peer must not abort the index connectivity probe.
            continue
        }
    }
} catch {
    # A service-only/session-0 context may not expose the interactive taskbar.
    # Lack of a taskbar button must not misrepresent catalog availability.
}

$report | ConvertTo-Json -Compress
