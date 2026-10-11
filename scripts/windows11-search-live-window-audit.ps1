# Read-only Windows 11 SearchHost geometry inventory. No taskbar/SearchHost
# activation, desktop screenshot, keyboard input, process mutation or window titles.
[CmdletBinding()]
param(
    [switch] $SelfTest,
    [string] $OutputPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Test-SufficientDesktopOverlap($Rect,$Desktop) {
    if ($null -eq $Rect -or $null -eq $Desktop) { return $false }
    try {
        $x=[long]$Rect.x; $y=[long]$Rect.y
        $width=[long]$Rect.width; $height=[long]$Rect.height
        $dx=[long]$Desktop.x; $dy=[long]$Desktop.y
        $dw=[long]$Desktop.width; $dh=[long]$Desktop.height
    } catch { return $false }
    if ($width -le 0 -or $height -le 0 -or $dw -le 0 -or $dh -le 0) { return $false }
    $overlapW = [Math]::Max(0,[Math]::Min($x+$width,$dx+$dw)-[Math]::Max($x,$dx))
    $overlapH = [Math]::Max(0,[Math]::Min($y+$height,$dy+$dh)-[Math]::Max($y,$dy))
    # WS_VISIBLE does not mean onscreen: require >=50% virtual desktop overlap.
    return ($overlapW * $overlapH * 2 -ge $width * $height)
}
function Select-SanitizedCandidates([object[]]$Windows, [int]$CurrentSession, $VirtualDesktop) {
    $result = @()
    foreach ($window in $Windows) {
        if ($window.session_id -ne $CurrentSession -or
            $window.process_name -notin @('SearchHost','SearchApp') -or
            -not $window.visible -or $window.cloaked -or
            -not $window.has_frame) { continue }
        $rect = $window.rect
        if (-not (Test-SufficientDesktopOverlap $rect $VirtualDesktop)) { continue }
        # GetDpiForWindow returns zero for a stale HWND. Without DPI we
        # cannot interpret a physical SearchHost frame reliably.
        try { $windowDpi = [uint32]$window.dpi } catch { continue }
        if ($windowDpi -eq 0) { continue }
        if ($rect.width -lt 200 -or $rect.height -lt 200 -or
            $rect.width -gt 2400 -or $rect.height -gt 1800) { continue }
        $result += [pscustomobject][ordered]@{
            process_name = $window.process_name
            class_name = $window.class_name
            rect_extended_frame_physical_px = $rect
            dpi_for_window = $windowDpi
        }
    }
    return $result
}
function Build-Audit([object[]]$Windows,[int]$Session,[string]$HostOs,$VirtualDesktop) {
    $candidates = @(Select-SanitizedCandidates $Windows $Session $VirtualDesktop)
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
    # Negative coordinates are valid for an actual left-hand display.
    $virtual = [pscustomobject]@{x=-1920;y=0;width=5360;height=1440}
    $rect = [pscustomobject]@{x=1332;y=655;width=776;height=725}
    $good = [pscustomobject]@{
        process_name='SearchHost'; session_id=2; visible=$true
        cloaked=$false; has_frame=$true; class_name='Windows.UI.Core.CoreWindow'
        rect=$rect;dpi=96
    }
    $empty = Build-Audit @() 2 '10.0.22631.0' $virtual
    Assert-Case ($empty.result -ceq 'BLOCKED' -and $empty.acceptance -ceq 'NOT_ACCEPTED') 'missing SearchHost remains blocked'
    $found = Build-Audit @($good) 2 '10.0.22631.0' $virtual
    Assert-Case ($found.result -ceq 'PARTIAL' -and $found.acceptance -ceq 'NOT_ACCEPTED' -and $found.candidates.Count -eq 1) 'visible candidate not accepted'
    Assert-Case ($found.candidates[0].rect_extended_frame_physical_px.width -eq 776) 'sanitized physical geometry retained'
    $other = [pscustomobject]@{process_name='Explorer';session_id=2;visible=$true;cloaked=$false;has_frame=$true;class_name='CabinetWClass';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($other) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'Explorer cannot substitute SearchHost'
    $closed = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$false;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($closed) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'hidden window cannot be measured as flyout'
    $cloaked = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$true;cloaked=$true;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($cloaked) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'DWM-cloaked window not accepted'
    $wrongSession = [pscustomobject]@{process_name='SearchHost';session_id=1;visible=$true;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=96}
    Assert-Case ((Build-Audit @($wrongSession) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'wrong session blocked'
    $small = [pscustomobject]@{process_name='SearchHost';session_id=2;visible=$true;cloaked=$false;has_frame=$true;class_name='CoreWindow';rect=([pscustomobject]@{x=0;y=0;width=5;height=10});dpi=96}
    Assert-Case ((Build-Audit @($small) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'tiny placeholder blocked'
    Assert-Case (-not ($found.candidates[0] | ConvertTo-Json -Depth 8 | Select-String -Pattern 'window_title|query_text|file_path' -CaseSensitive)) 'no sensitive dynamic payload fields'
    Assert-Case ((Build-Audit @($good) 2 '10.0.19045.0' $virtual).result -ceq 'BLOCKED') 'Windows 10 cannot be mistaken for Windows 11'
    # WS_VISIBLE does not mean a HWND actually intersects a real monitor.
    # Offscreen placeholder windows can retain an uncloaked, valid-size frame.
    $offscreen = [pscustomobject]@{
        process_name='SearchHost';session_id=2;visible=$true;cloaked=$false
        has_frame=$true;class_name='CoreWindow'
        rect=([pscustomobject]@{x=-30000;y=-30000;width=780;height=720});dpi=96
    }
    Assert-Case ((Build-Audit @($offscreen) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'offscreen visible HWND cannot count as a Search flyout'
    $onePixel = [pscustomobject]@{
        process_name='SearchHost';session_id=2;visible=$true;cloaked=$false
        has_frame=$true;class_name='CoreWindow'
        rect=([pscustomobject]@{x=3439;y=620;width=780;height=720});dpi=96
    }
    Assert-Case ((Build-Audit @($onePixel) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'one-pixel overlap is not adequate visible geometry'
    $negativeMonitor = [pscustomobject]@{
        process_name='SearchHost';session_id=2;visible=$true;cloaked=$false
        has_frame=$true;class_name='CoreWindow'
        rect=([pscustomobject]@{x=-1700;y=500;width=780;height=720});dpi=96
    }
    Assert-Case ((Build-Audit @($negativeMonitor) 2 '10.0.22631.0' $virtual).result -ceq 'PARTIAL') 'negative X within second monitor is allowed, not accepted'
    $unknownDpi = [pscustomobject]@{
        process_name='SearchHost';session_id=2;visible=$true;cloaked=$false
        has_frame=$true;class_name='CoreWindow';rect=$rect;dpi=0
    }
    Assert-Case ((Build-Audit @($unknownDpi) 2 '10.0.22631.0' $virtual).result -ceq 'BLOCKED') 'missing window DPI cannot become a candidate'
    Write-Output 'SearchHost read-only geometry selftest PASS (14 checks; physical acceptance unverified).'
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
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
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
$virtualDesktop = [pscustomobject]@{
    x=[SearchHostReadOnlyWindows]::GetSystemMetrics(76)
    y=[SearchHostReadOnlyWindows]::GetSystemMetrics(77)
    width=[SearchHostReadOnlyWindows]::GetSystemMetrics(78)
    height=[SearchHostReadOnlyWindows]::GetSystemMetrics(79)
}
$report = Build-Audit $observed $session $os $virtualDesktop
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $parent = Split-Path -Parent $OutputPath
    if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $report | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
}
$report | ConvertTo-Json -Depth 9
