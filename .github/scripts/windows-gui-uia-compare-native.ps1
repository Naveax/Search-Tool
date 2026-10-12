# Compare native Win32 UIA roles with the Search Tool synthetic offscreen GUI.
# No desktop input; equal limited roles do NOT constitute Narrator/UIA acceptance.
[CmdletBinding()]
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-UiaRole([string]$Text, [string]$Pattern, [string]$Label) {
    $found = [regex]::Matches($Text,$Pattern,[System.Text.RegularExpressions.RegexOptions]::Multiline)
    if ($found.Count -ne 1) { throw "Expected exactly one $Label UIA role; found $($found.Count)" }
    return $found[0].Groups[1].Value
}
function Compare-NativeUia([string]$Native, [string]$Tool) {
    $nativeEdit = Read-UiaRole $Native '^BASELINE_EDIT_ROLE=(ControlType\.[A-Za-z]+)\s*$' 'native Edit'
    $nativeList = Read-UiaRole $Native '^BASELINE_LIST_ROLE=(ControlType\.[A-Za-z]+)\s*$' 'native List'
    $toolEdit = Read-UiaRole $Tool 'UIA provider coverage: Edit=(ControlType\.[A-Za-z]+) ' 'Search Tool Edit'
    $toolList = Read-UiaRole $Tool 'UIA provider coverage: Edit=ControlType\.[A-Za-z]+ Name=.*?; List=(ControlType\.[A-Za-z]+) ' 'Search Tool List'
    if ($nativeEdit -cnotin @('ControlType.Edit','ControlType.Pane') -or
        $nativeList -cnotin @('ControlType.List','ControlType.Pane') -or
        $toolEdit -cnotin @('ControlType.Edit','ControlType.Pane') -or
        $toolList -cnotin @('ControlType.List','ControlType.Pane')) {
        throw 'Unexpected UIA control role in native baseline or Search Tool observation'
    }
    $nativeExpected = $nativeEdit -ceq 'ControlType.Edit' -and $nativeList -ceq 'ControlType.List'
    $toolExpected = $toolEdit -ceq 'ControlType.Edit' -and $toolList -ceq 'ControlType.List'
    $relativeRegression = (($nativeEdit -ceq 'ControlType.Edit' -and $toolEdit -cne 'ControlType.Edit') -or
        ($nativeList -ceq 'ControlType.List' -and $toolList -cne 'ControlType.List'))
    $classification = if ($relativeRegression) { 'REGRESSION_AGAINST_NATIVE' }
        elseif ($nativeExpected -and $toolExpected) { 'STANDARD_ROLES_PRESENT_BUT_PHYSICAL_FOCUS_UNTESTED' }
        elseif ($toolExpected) { 'STANDARD_TOOL_ROLES_WITH_LIMITED_NATIVE_BASELINE' }
        elseif ($nativeEdit -ceq $toolEdit -and $nativeList -ceq $toolList) { 'MATCHED_LIMITED_NATIVE_BASELINE' }
        else { 'DIFFERENT_LIMITED_NATIVE_BASELINE' }
    return [pscustomobject][ordered]@{
        schema = 1
        result = if ($relativeRegression) { 'FAIL' } else { 'DIAGNOSTIC_ONLY' }
        acceptance = 'NOT_ACCEPTED'
        classification = $classification
        native_edit_role = $nativeEdit
        native_list_role = $nativeList
        search_tool_edit_role = $toolEdit
        search_tool_list_role = $toolList
        remaining = 'Actual focus, Narrator and interactive Windows 11 UI Automation remain unverified'
    }
}
function Assert-Check([bool]$Condition,[string]$Message) {
    if (-not $Condition) { throw "Relative UIA selftest failure: $Message" }
}
if ($SelfTest) {
    $limitedNative = 'BASELINE_EDIT_ROLE=ControlType.Pane','BASELINE_LIST_ROLE=ControlType.Pane' -join [Environment]::NewLine
    $standardNative = 'BASELINE_EDIT_ROLE=ControlType.Edit','BASELINE_LIST_ROLE=ControlType.List' -join [Environment]::NewLine
    $limitedTool = "UIA provider coverage: Edit=ControlType.Pane Name='SearchTool'; List=ControlType.Pane Name=''; correct roles and names=False"
    $standardTool = "UIA provider coverage: Edit=ControlType.Edit Name='SearchTool'; List=ControlType.List Name='Arama'; correct roles and names=True"
    $limited = Compare-NativeUia $limitedNative $limitedTool
    Assert-Check ($limited.result -ceq 'DIAGNOSTIC_ONLY' -and $limited.classification -ceq 'MATCHED_LIMITED_NATIVE_BASELINE' -and $limited.acceptance -ceq 'NOT_ACCEPTED') 'equal limited providers not accepted'
    $standard = Compare-NativeUia $standardNative $standardTool
    Assert-Check ($standard.result -ceq 'DIAGNOSTIC_ONLY' -and $standard.classification -ceq 'STANDARD_ROLES_PRESENT_BUT_PHYSICAL_FOCUS_UNTESTED') 'standard offscreen roles not physical acceptance'
    $better = Compare-NativeUia $limitedNative $standardTool
    Assert-Check ($better.classification -ceq 'STANDARD_TOOL_ROLES_WITH_LIMITED_NATIVE_BASELINE') 'tool may exceed native baseline'
    $regressed = Compare-NativeUia $standardNative $limitedTool
    Assert-Check ($regressed.result -ceq 'FAIL' -and $regressed.classification -ceq 'REGRESSION_AGAINST_NATIVE') 'native-capable role regression fails'
    $threw = $false
    try { [void](Compare-NativeUia $limitedNative 'not a UIA provider report') }
    catch { $threw = $true }
    Assert-Check $threw 'missing observation fails closed'
    $duplicateNative = $limitedNative + [Environment]::NewLine + 'BASELINE_EDIT_ROLE=ControlType.Edit'
    $threw = $false
    try { [void](Compare-NativeUia $duplicateNative $limitedTool) }
    catch { $threw = $true }
    Assert-Check $threw 'conflicting native observations must fail closed'
    $duplicateTool = $limitedTool + [Environment]::NewLine + $standardTool
    $threw = $false
    try { [void](Compare-NativeUia $limitedNative $duplicateTool) }
    catch { $threw = $true }
    Assert-Check $threw 'conflicting Search Tool observations must fail closed'
    $wrongNative = 'BASELINE_EDIT_ROLE=ControlType.Button','BASELINE_LIST_ROLE=ControlType.Pane' -join [Environment]::NewLine
    $threw = $false
    try { [void](Compare-NativeUia $wrongNative $limitedTool) }
    catch { $threw = $true }
    Assert-Check $threw 'unexpected native control role must fail closed'
    $wrongTool = "UIA provider coverage: Edit=ControlType.Button Name='SearchTool'; List=ControlType.Pane Name=''; correct roles and names=False"
    $threw = $false
    try { [void](Compare-NativeUia $limitedNative $wrongTool) }
    catch { $threw = $true }
    Assert-Check $threw 'unexpected Search Tool control role must fail closed'
    Write-Output 'Native UIA relative-provider selftest PASS (9 cases; physical UIA not accepted).'
    return
}
$toolPath = Join-Path $PSScriptRoot 'windows-gui-msaa-regression.ps1'
if (-not (Test-Path -LiteralPath $toolPath -PathType Leaf)) {
    throw "Missing UIA regression dependency: $toolPath"
}
# The underlying MSAA/UIA regression already launches an independent
# untouched Win32 baseline *after* its own hooks complete. Reuse that exact
# result instead of starting a second concurrent baseline process.
$toolLines = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $toolPath)
if ($LASTEXITCODE -ne 0) { throw "Search Tool MSAA/UIA regression failed: $LASTEXITCODE" }
$toolOutput = $toolLines -join [Environment]::NewLine
$nativeSummaries = [regex]::Matches($toolOutput,'Native Windows UIA baseline: Edit=(ControlType\.[A-Za-z]+); List=(ControlType\.[A-Za-z]+)')
if ($nativeSummaries.Count -ne 1) { throw "Expected exactly one native baseline summary; found $($nativeSummaries.Count)" }
$nativeSummary = $nativeSummaries[0]
$nativeLines = @(
    ('BASELINE_EDIT_ROLE=' + $nativeSummary.Groups[1].Value),
    ('BASELINE_LIST_ROLE=' + $nativeSummary.Groups[2].Value)
)
$comparison = Compare-NativeUia ($nativeLines -join [Environment]::NewLine) $toolOutput
$comparison | ConvertTo-Json -Depth 5
if ($comparison.result -ceq 'FAIL') {
    throw 'Search Tool UIA roles are worse than untouched Win32 controls on the same host'
}
