# Read-only, privacy-minimal UI Automation diagnostics for Microsoft's
# ORIGINAL Windows 11 SearchHost/SearchApp flyout. NEVER opens Search,
# sends keys, clicks, reads UIA Name/Value/Text or captures screenshots.
[CmdletBinding()]
param([switch]$SelfTest)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

function Get-SafeNativeRoleSummary([object[]]$Entries,[bool]$TrustedFrame) {
    $allowed=@('Edit','Button','List','ListItem','Text','Pane','Group','Tab','TabItem')
    $counts=[ordered]@{
        edit=0;button=0;list=0;listitem=0;text=0;pane=0
        group=0;tab=0;tabitem=0
    }
    $focused=0
    $focusable=0
    $unknown=0
    foreach($entry in $Entries) {
        if ($null -eq $entry) { continue }
        if ($entry.role -notin $allowed) { $unknown++; continue }
        if ([bool]$entry.offscreen) { continue }
        $key=([string]$entry.role).ToLowerInvariant()
        $counts[$key]++
        if ([bool]$entry.focused) { $focused++ }
        if ([bool]$entry.focusable) { $focusable++ }
    }
    $status=if (-not $TrustedFrame) {'BLOCKED_UNTRUSTED_NATIVE_FRAME'}
        elseif (@($Entries).Count -eq 0) {'BLOCKED_NO_UIA_CONTROLS'}
        else {'PARTIAL_READ_ONLY_ROLES'}
    return [pscustomobject][ordered]@{
        status=$status
        native_edit_count=[int]$counts.edit
        native_button_count=[int]$counts.button
        native_list_count=[int]$counts.list
        native_listitem_count=[int]$counts.listitem
        native_text_count=[int]$counts.text
        native_pane_count=[int]$counts.pane
        native_group_count=[int]$counts.group
        native_tab_count=[int]$counts.tab
        native_tabitem_count=[int]$counts.tabitem
        native_focused_control_count=$focused
        native_focusable_control_count=$focusable
        other_role_count=$unknown
        visited_nodes=@($Entries).Count
        reads_user_text=$false
        sends_user_input=$false
        native_search_result_integration='UNVERIFIED'
        interaction_acceptance='NOT_ACCEPTED'
    }
}
function Assert-SafeReadOnlyMetadata($Data) {
    if ($Data.reads_user_text -cne $false -or
        $Data.sends_user_input -cne $false -or
        $Data.interaction_acceptance -cne 'NOT_ACCEPTED' -or
        $Data.native_search_result_integration -cne 'UNVERIFIED' -or
        $Data.visited_nodes -gt 160) {
        throw 'NATIVE_UIA_PRIVACY_OR_RELEASE_CONTRACT_FAILED'
    }
}
if ($SelfTest) {
    $controls=@(
        [pscustomobject]@{role='Edit';offscreen=$false;focused=$true;focusable=$true},
        [pscustomobject]@{role='Button';offscreen=$false;focused=$false;focusable=$true},
        [pscustomobject]@{role='Edit';offscreen=$true;focused=$true;focusable=$true},
        [pscustomobject]@{role='Unknown';offscreen=$false;focused=$false;focusable=$true}
    )
    $a=Get-SafeNativeRoleSummary -Entries $controls -TrustedFrame $true
    Assert-SafeReadOnlyMetadata $a
    if ($a.status -cne 'PARTIAL_READ_ONLY_ROLES' -or
        $a.native_edit_count -ne 1 -or $a.native_button_count -ne 1 -or
        $a.native_focused_control_count -ne 1 -or
        $a.other_role_count -ne 1) {throw 'NATIVE_UIA_CONTROL_ROLE_AGGREGATION_FAILED'}
    $bad=Get-SafeNativeRoleSummary -Entries $controls -TrustedFrame $false
    if ($bad.status -cne 'BLOCKED_UNTRUSTED_NATIVE_FRAME' -or
        $bad.interaction_acceptance -cne 'NOT_ACCEPTED') {
        throw 'NATIVE_UIA_UNTRUSTED_FRAME_GUARD_FAILED'
    }
    $empty=Get-SafeNativeRoleSummary -Entries @() -TrustedFrame $true
    if ($empty.status -cne 'BLOCKED_NO_UIA_CONTROLS') {
        throw 'NATIVE_UIA_MISSING_CONTROLS_GUARD_FAILED'
    }
    $invalid=$a.PSObject.Copy()
    $invalid.reads_user_text=$true
    $rejected=$false
    try {Assert-SafeReadOnlyMetadata $invalid} catch {
        $rejected=$_.Exception.Message -ceq 'NATIVE_UIA_PRIVACY_OR_RELEASE_CONTRACT_FAILED'
    }
    if (-not $rejected) {throw 'NATIVE_UIA_PRIVACY_CONTRACT_FAIL_OPEN'}
    'NATIVE_SEARCH_UIA_READONLY_METADATA_SELFTEST_PASS'
    return
}
$report=[ordered]@{
    schema=1;mode='READ_ONLY_NATIVE_SEARCH_UIA_ROLES'
    status='BLOCKED'
    reason='UNOBSERVED'
    native_window_frame_verified=$false
    controls=$null
    reads_user_text=$false
    sends_user_input=$false
    screenshot_taken=$false
    native_search_result_integration='UNVERIFIED'
    interaction_acceptance='NOT_ACCEPTED'
    release_eligible=$false
}
try {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        $report.reason='WINDOWS_ONLY'
        return
    }
    $baseline=& (Join-Path $PSScriptRoot 'windows11-search-live-window-audit.ps1') |
        ConvertFrom-Json
    if ($null -eq $baseline -or $baseline.result -cne 'PARTIAL' -or
        $baseline.acceptance -cne 'NOT_ACCEPTED' -or
        @($baseline.candidates).Count -ne 1) {
        $report.reason='NO_UNIQUE_VERIFIED_NATIVE_SEARCH_FRAME'
        return
    }
    Add-Type -AssemblyName UIAutomationClient -ErrorAction Stop
    Add-Type -AssemblyName UIAutomationTypes -ErrorAction Stop
    $candidate=$baseline.candidates[0]
    $rect=$candidate.rect_extended_frame_physical_px
    $processes=@(Get-Process -Name SearchHost,SearchApp -ErrorAction SilentlyContinue |
        Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId})
    if ($processes.Count -eq 0) {
        $report.reason='NO_SAME_SESSION_SEARCH_PROCESS'
        return
    }
    $knownPids=@($processes|ForEach-Object {[int]$_.Id})
    $desktop=[System.Windows.Automation.AutomationElement]::RootElement
    $children=$desktop.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition)
    $matches=[System.Collections.Generic.List[object]]::new()
    for($i=0;$i -lt $children.Count;$i++) {
        try {
            $elem=$children.Item($i)
            $c=$elem.Current
            if ([int]$c.ProcessId -notin $knownPids -or $c.IsOffscreen) {continue}
            $b=$c.BoundingRectangle
            if ([Math]::Abs($b.Left-[double]$rect.x) -le 24 -and
                [Math]::Abs($b.Top-[double]$rect.y) -le 24 -and
                [Math]::Abs($b.Width-[double]$rect.width) -le 24 -and
                [Math]::Abs($b.Height-[double]$rect.height) -le 24) {
                $matches.Add($elem)
            }
        } catch {continue}
    }
    if ($matches.Count -ne 1) {
        $report.reason='NO_UNIQUE_UIA_ROOT_MATCHING_NATIVE_FRAME'
        return
    }
    $report.native_window_frame_verified=$true
    $walker=[System.Windows.Automation.TreeWalker]::ControlViewWalker
    $queue=[System.Collections.Generic.Queue[object]]::new()
    $queue.Enqueue([pscustomobject]@{element=$matches[0];depth=0})
    $visited=[System.Collections.Generic.List[object]]::new()
    $budget=160
    while ($queue.Count -gt 0 -and $visited.Count -lt $budget) {
        $node=$queue.Dequeue()
        try {
            $item=$node.element.Current
            $role=[string]$item.ControlType.ProgrammaticName
            if ($role.StartsWith('ControlType.')) {$role=$role.Substring(12)}
            $visited.Add([pscustomobject]@{
                role=$role
                offscreen=[bool]$item.IsOffscreen
                focused=[bool]$item.HasKeyboardFocus
                focusable=[bool]$item.IsKeyboardFocusable
            })
            if ($node.depth -ge 5) {continue}
            $child=$walker.GetFirstChild($node.element)
            $childCount=0
            while ($null -ne $child -and
                   $childCount -lt 100 -and
                   ($visited.Count+$queue.Count) -lt $budget) {
                $queue.Enqueue([pscustomobject]@{
                    element=$child;depth=([int]$node.depth+1)
                })
                $childCount++
                $child=$walker.GetNextSibling($child)
            }
        } catch {continue}
    }
    $summary=Get-SafeNativeRoleSummary -Entries $visited.ToArray() -TrustedFrame $true
    Assert-SafeReadOnlyMetadata $summary
    $report.controls=$summary
    $report.status=$summary.status
    $report.reason=if($summary.status -ceq 'PARTIAL_READ_ONLY_ROLES') {
        'NATIVE_UIA_ROLE_STRUCTURE_OBSERVED_NO_INTERACTION_PROOF'
    } else {$summary.status}
} catch {
    # Never propagate UIA provider exceptions; they may include a user's
    # window title, search text or accessible name.
    $report.status='BLOCKED'
    $report.reason='UIA_OBSERVATION_UNAVAILABLE'
} finally {
    $report | ConvertTo-Json -Depth 6 -Compress
}
