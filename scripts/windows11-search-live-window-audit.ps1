# Read-only Windows 11 SearchHost geometry inventory. No taskbar/SearchHost
# activation, desktop screenshot, keyboard input, process mutation or window titles.
[CmdletBinding()]
param(
    [switch] $SelfTest,
    [string] $OutputPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Select-SanitizedCandidates([object[]]$Windows, [int]$CurrentSession) {
    $result = @()
    foreach ($window in $Windows) {
        if ($window.session_id -ne $CurrentSession -or
            $window.process_name -notin @('SearchHost','SearchApp') -or
            -not $window.visible -or $window.cloaked -or
            -not $window.has_frame) { continue }
        $rect = $window.rect
        if ($rect.width -lt 200 -or $rect.height -lt 200 -or
            $rect.width -gt 2400 -or $rect.height -gt 1800) { continue }
        $result += [pscustomobject][ordered]@{
            process_name = $window.process_name
            class_name = $window.class_name
            rect_extended_frame_physical_px = $rect
            dpi_for_window = $window.dpi
        }
    }
    return $result
}
function Build-Audit([object[]]$Windows,[int]$Session,[string]$HostOs) {
    $candidates = @(Select-SanitizedCandidates $Windows $Session)
    $isWindows11 = $false
    try {
        $version = [Version]$HostOs
        $isWindows11 = ($version.Major -eq 10 -and $version.Minor -eq 0 -and $version.Build -ge 22000)
    } catch { $isWindows11 = $false }
    $status = if (-not $isWindows11) { 'BLOCKED' }
        elseif ($candidates.Count -eq 0) { 'BLOCKED' }
        else { 'PARTIAL' }
    $reason = if (-not $isWindows11) {
        'Windows 11 build 22000 or newer was not established'
    } elseif ($candidates.Count -eq 0) {
        'No visible uncloaked SearchHost/SearchApp candidate with measurable physical frame'
    } else {
        'Unverified candidate SearchHost windows; process identity alone does not establish Search flyout bounds'
    }
    return [pscustomobject][ordered]@{
        schema = 1
        result = $status
        acceptance = 'NOT_ACCEPTED'
        capture_kind = 'READ_ONLY_WINDOW_METADATA'
        session = $Session
        os_version = $HostOs
        reason = $reason
        candidates = @($candidates)
        limitations = @(
            'No Search window activated and no screenshots or text collected',
            'Visible SearchHost ownership alone does not prove native Search flyout identity',
            'Physical keyboard/IME, UIA, Narrator and SearchHost entrypoint parity remain unverified',
            'Comparison with Search Tool current-head candidate and real visual screenshots is still pending'
        )
    }
}
function Assert-Case([bool]$Condition,[string]$Name) {
    if (-not $Condition) { throw "SearchHost read-only geometry selftest failed: $Name" }
}
if ($SelfTest) {
    $rect = [pscustomobject]@{x=1332;y=655;width=776;height=725}
    $good = [pscustomobject]@{
        process_name='SearchHost'; session_id=2; visible=$true
        cloaked=$false; has_frame=$true; class_name='Windows.UI.Core.CoreWindow'
        rect=$rect;dpi=96
    }
    $empty = Build-Audit @() 2 '10.0.22631.0'
    Assert-Case ($empty.result -ceq 'BLOCKED' -and $empty.acceptance -ceq 'NOT_ACCEPTED') 'missing SearchHost remains blocked'
    $found = Build-Audit @($good) 2 '10.0.22631.0'
    Assert-Case ($found.result -ceq 'PARTIAL' -and $found.acceptance -ceq 'NOT_ACCEPTED' -and $found.candidates.Count -eq 1) 'visible candidate not accepted'
    Assert-Case ($found.candidates[0].rect_extended_frame_physical_px.width -eq 776) 'sanitized physical geometry retained'
    $other = [pscustomobject]@{process_name='Explorer';session_id=2;visible=$true;cloaked=$false;has_frame=$true;class_name='CabinetWClass';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($other) 2 '10.0.22631.0').result -ceq 'BLOCKED') 'Explorer cannot substitute SearchHost'
    $closed = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$false;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($closed) 2 '10.0.22631.0').result -ceq 'BLOCKED') 'hidden window cannot be measured as flyout'
    $cloaked = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$true;cloaked=$true;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($cloaked) 2 '10.0.22631.0').result -ceq 'BLOCKED') 'DWM-cloaked window not accepted'
    $wrongSession = [pscustomobject]@{process_name='SearchHost';session_id=1;visible=$true;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($wrongSession) 2 '10.0.22631.0').result -ceq 'BLOCKED') 'wrong session blocked'
    $small = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$true;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=([pscustomobject]@{x=0;y=0;width=5;height=10});dpi=96}
    Assert-Case ((Build-Audit @($small) 2 '10.0.22631.0').result -ceq 'BLOCKED') 'tiny placeholder blocked'
    Assert-Case (-not ($found.candidates[0] | ConvertTo-Json -Depth 8 | Select-String -Pattern 'window_title|query_text|file_path' -CaseSensitive)) 'no sensitive dynamic payload fields'
    Assert-Case ((Build-Audit @($good) 2 '10.0.19045.0').result -ceq 'BLOCKED') 'Windows 10 cannot be mistaken for Windows 11'
    Write-Output 'SearchHost read-only geometry selftest PASS (10 checks; physical acceptance unverified).'
    return
}
if (-not [Environment]::OSVersion.Platform.Equals([PlatformID]::Win32NT)) {
    throw 'Windows only'
}
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class SearchHostReadOnlyWindows {
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr p);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left, top, right, bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb,IntPtr data);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder buffer,int size);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h,int attribute,out RECT value,int size);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h,int attribute,out int value,int size);
    public static List<IntPtr> List() {
        var result = new List<IntPtr>();
        EnumWindows((h,p) => { result.Add(h); return true; },IntPtr.Zero);
        return result;
    }
    public static string Class(IntPtr h) {
        var name = new StringBuilder(128);
        GetClassName(h,name,name.Capacity);
        return name.ToString();
    }
}
'@
$os = [Environment]::OSVersion.Version.ToString()
$session = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
$observed = @()
# Pre-filter to the known shell process IDs. Enumerating the desktop HWND
# table must not repeatedly open handles to unrelated user applications.
$searchByPid = @{}
foreach ($process in @(Get-Process SearchHost,SearchApp -ErrorAction SilentlyContinue)) {
    if ($process.SessionId -eq $session) {
        $searchByPid[[int]$process.Id] = $process
    }
}
foreach ($h in [SearchHostReadOnlyWindows]::List()) {
    [uint32]$targetPid = 0
    [void][SearchHostReadOnlyWindows]::GetWindowThreadProcessId($h,[ref]$targetPid)
    if (-not $searchByPid.ContainsKey([int]$targetPid)) { continue }
    $proc = $searchByPid[[int]$targetPid]
    $rect = New-Object SearchHostReadOnlyWindows+RECT
    $hr = [SearchHostReadOnlyWindows]::DwmGetWindowAttribute($h,9,[ref]$rect,[Runtime.InteropServices.Marshal]::SizeOf($rect))
    [int]$cloak = 1
    [void][SearchHostReadOnlyWindows]::DwmGetWindowAttribute($h,14,[ref]$cloak,4)
    $geometry = if ($hr -eq 0) {
        [pscustomobject]@{x=$rect.left;y=$rect.top;width=($rect.right-$rect.left);height=($rect.bottom-$rect.top)}
    } else {
        [pscustomobject]@{x=0;y=0;width=0;height=0}
    }
    $observed += [pscustomobject]@{
        process_name=$proc.ProcessName
        session_id=$proc.SessionId
        visible=[SearchHostReadOnlyWindows]::IsWindowVisible($h)
        cloaked=($cloak -ne 0)
        has_frame=($hr -eq 0)
        class_name=[SearchHostReadOnlyWindows]::Class($h)
        rect=$geometry
        dpi=[SearchHostReadOnlyWindows]::GetDpiForWindow($h)
    }
}
$report = Build-Audit $observed $session $os
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $parent = Split-Path -Parent $OutputPath
    if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $report | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
}
$report | ConvertTo-Json -Depth 9
