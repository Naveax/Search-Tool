[CmdletBinding()]
param(
    [ValidateSet('Probe','Exercise','PrepareTopology','VerifyTopology')] [string]$Mode = 'Probe',
    [ValidateSet('AnyChange','PrimaryChanged','MonitorRemoved')] [string]$ExpectedTopologyChange = 'AnyChange',
    [switch]$RequireMixedDpi,
    [switch]$Enforce,
    [string]$StateFile = (Join-Path $env:TEMP 'search-tool-display-state.json'),
    [string]$OutputJson,
    [string]$GuiPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

if (-not ('SearchToolDisplayProbe' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public sealed class SearchToolMonitorRecord {
    public IntPtr Handle;
    public string Device;
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
    public bool Primary;
    public uint DpiX;
    public uint DpiY;
}

public static class SearchToolDisplayProbe {
    public delegate bool MonitorEnumProc(IntPtr hMonitor, IntPtr hdcMonitor, IntPtr lprcMonitor, IntPtr dwData);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct MONITORINFOEX {
        public int cbSize;
        public RECT rcMonitor;
        public RECT rcWork;
        public uint dwFlags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
        public string szDevice;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorEnumProc callback, IntPtr data);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern bool GetMonitorInfo(IntPtr hMonitor, ref MONITORINFOEX info);

    [DllImport("Shcore.dll")]
    static extern int GetDpiForMonitor(IntPtr hMonitor, int dpiType, out uint dpiX, out uint dpiY);

    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);

    public static List<SearchToolMonitorRecord> Enumerate() {
        var result = new List<SearchToolMonitorRecord>();
        EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr h, IntPtr dc, IntPtr r, IntPtr d) {
            var info = new MONITORINFOEX();
            info.cbSize = Marshal.SizeOf(typeof(MONITORINFOEX));
            if (!GetMonitorInfo(h, ref info)) return true;
            uint dx = 96, dy = 96;
            try { GetDpiForMonitor(h, 0, out dx, out dy); } catch { dx = 96; dy = 96; }
            result.Add(new SearchToolMonitorRecord {
                Handle = h,
                Device = info.szDevice,
                Left = info.rcMonitor.Left,
                Top = info.rcMonitor.Top,
                Right = info.rcMonitor.Right,
                Bottom = info.rcMonitor.Bottom,
                Primary = (info.dwFlags & 1) != 0,
                DpiX = dx,
                DpiY = dy
            });
            return true;
        }, IntPtr.Zero);
        return result;
    }
}
'@
}

function Write-Report([object]$Report) {
    $json = $Report | ConvertTo-Json -Depth 8
    $json
    if ($OutputJson) {
        $parent = Split-Path -Parent $OutputJson
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
        Write-Host "Report=$OutputJson"
    }
}

function Get-MonitorSnapshot {
    return @([SearchToolDisplayProbe]::Enumerate() | ForEach-Object {
        [ordered]@{
            handle = $_.Handle.ToInt64()
            device = $_.Device
            primary = [bool]$_.Primary
            left = [int]$_.Left
            top = [int]$_.Top
            right = [int]$_.Right
            bottom = [int]$_.Bottom
            width = [int]($_.Right - $_.Left)
            height = [int]($_.Bottom - $_.Top)
            dpi_x = [int]$_.DpiX
            dpi_y = [int]$_.DpiY
            scale_percent = [Math]::Round(([double]$_.DpiX / 96.0) * 100.0, 1)
        }
    })
}

function Resolve-GuiBinary {
    if ($GuiPath) { return (Resolve-Path -LiteralPath $GuiPath).Path }
    $beside = Join-Path $PSScriptRoot 'search-tool-gui.exe'
    if (Test-Path -LiteralPath $beside) { return (Resolve-Path -LiteralPath $beside).Path }
    $built = Join-Path $root 'target\release\search-tool-gui.exe'
    if (Test-Path -LiteralPath $built) { return (Resolve-Path -LiteralPath $built).Path }
    throw 'search-tool-gui.exe was not found.'
}

function Wait-GuiWindow([int]$PreferredPid = 0, [switch]$LaunchIfMissing) {
    $launched = $false
    $proc = $null
    if ($PreferredPid -gt 0) {
        $proc = Get-Process -Id $PreferredPid -ErrorAction SilentlyContinue
    }
    if (-not $proc) {
        $proc = Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue | Select-Object -First 1
    }
    if (-not $proc -and $LaunchIfMissing) {
        $proc = Start-Process -FilePath (Resolve-GuiBinary) -PassThru
        $launched = $true
    }
    if (-not $proc) { return $null }

    $deadline = (Get-Date).AddSeconds(15)
    do {
        $proc.Refresh()
        if ($proc.HasExited) { throw "GUI exited before a window became available: pid=$($proc.Id)" }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
            return [pscustomobject]@{ process = $proc; launched = $launched; hwnd = $proc.MainWindowHandle }
        }
        Start-Sleep -Milliseconds 200
    } while ((Get-Date) -lt $deadline)
    throw "Timed out waiting for Search Tool GUI window: pid=$($proc.Id)"
}

function Get-WindowSnapshot([IntPtr]$Hwnd) {
    $rect = New-Object SearchToolDisplayProbe+RECT
    if (-not [SearchToolDisplayProbe]::GetWindowRect($Hwnd, [ref]$rect)) { throw 'GetWindowRect failed.' }
    return [ordered]@{
        visible = [bool][SearchToolDisplayProbe]::IsWindowVisible($Hwnd)
        left = [int]$rect.Left
        top = [int]$rect.Top
        right = [int]$rect.Right
        bottom = [int]$rect.Bottom
        width = [int]($rect.Right - $rect.Left)
        height = [int]($rect.Bottom - $rect.Top)
        dpi = [int][SearchToolDisplayProbe]::GetDpiForWindow($Hwnd)
    }
}

function Test-WindowIntersectsMonitor([object]$Window, [object[]]$Monitors) {
    foreach ($m in $Monitors) {
        if ($Window.right -gt $m.left -and $Window.left -lt $m.right -and
            $Window.bottom -gt $m.top -and $Window.top -lt $m.bottom) { return $true }
    }
    return $false
}

function Topology-Key([object[]]$Monitors) {
    return (($Monitors | Sort-Object device | ForEach-Object {
        "$($_.device)|$($_.primary)|$($_.left),$($_.top),$($_.right),$($_.bottom)|$($_.dpi_x),$($_.dpi_y)"
    }) -join ';')
}

$monitors = @(Get-MonitorSnapshot)
$distinctDpi = @($monitors | ForEach-Object { "$($_.dpi_x)x$($_.dpi_y)" } | Sort-Object -Unique)
$base = [ordered]@{
    schema = 1
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    mode = $Mode
    computer_name = [Environment]::MachineName
    monitor_count = $monitors.Count
    distinct_dpi_count = $distinctDpi.Count
    mixed_dpi = ($distinctDpi.Count -gt 1)
    require_mixed_dpi = [bool]$RequireMixedDpi
    monitors = $monitors
}

if ($Mode -eq 'Probe') {
    $blocked = $RequireMixedDpi -and ($monitors.Count -lt 2 -or $distinctDpi.Count -lt 2)
    $base['result'] = if ($blocked) { 'BLOCKED' } else { 'READY' }
    if ($blocked) { $base['reason'] = 'At least two active monitors with distinct effective DPI values are required.' }
    Write-Report $base
    if ($Enforce -and $blocked) { throw $base.reason }
    return
}

if ($Mode -eq 'PrepareTopology') {
    $blocked = $RequireMixedDpi -and ($monitors.Count -lt 2 -or $distinctDpi.Count -lt 2)
    if ($blocked) {
        $base['result'] = 'BLOCKED'
        $base['reason'] = 'PrepareTopology requires at least two active monitors with distinct effective DPI values when -RequireMixedDpi is set.'
        Write-Report $base
        if ($Enforce) { throw $base.reason }
        return
    }

    $gui = Wait-GuiWindow -LaunchIfMissing
    if (-not $gui) { throw 'Search Tool GUI could not be started.' }
    $base['gui_pid'] = $gui.process.Id
    $base['window'] = Get-WindowSnapshot $gui.hwnd
    $base['topology_key'] = Topology-Key $monitors
    $base['result'] = 'PREPARED'
    $parent = Split-Path -Parent $StateFile
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $base | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $StateFile -Encoding UTF8
    Write-Report $base
    Write-Host "State=$StateFile"
    return
}

if ($Mode -eq 'VerifyTopology') {
    if (-not (Test-Path -LiteralPath $StateFile)) { throw "Missing topology state: $StateFile" }
    $before = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
    $beforeMonitors = @($before.monitors)
    $beforeDistinctDpi = @($beforeMonitors | ForEach-Object { "$($_.dpi_x)x$($_.dpi_y)" } | Sort-Object -Unique)
    $preparedMixedDpi = ($beforeMonitors.Count -ge 2 -and $beforeDistinctDpi.Count -ge 2)
    if ($RequireMixedDpi -and -not $preparedMixedDpi) {
        $base['prepared_monitor_count'] = $beforeMonitors.Count
        $base['prepared_distinct_dpi_count'] = $beforeDistinctDpi.Count
        $base['prepared_mixed_dpi'] = $false
        $base['result'] = 'BLOCKED'
        $base['reason'] = 'VerifyTopology requires a prepared state with at least two active monitors and distinct effective DPI values when -RequireMixedDpi is set.'
        Write-Report $base
        if ($Enforce) { throw $base.reason }
        return
    }

    $gui = Wait-GuiWindow -PreferredPid ([int]$before.gui_pid)
    $window = if ($gui) { Get-WindowSnapshot $gui.hwnd } else { $null }
    $oldPrimary = @($beforeMonitors | Where-Object { $_.primary } | Select-Object -First 1).device
    $newPrimary = @($monitors | Where-Object { $_.primary } | Select-Object -First 1).device
    $changed = (Topology-Key $monitors) -ne [string]$before.topology_key
    $currentDevices = @($monitors | ForEach-Object { [string]$_.device })
    $removedMonitors = @($beforeMonitors | Where-Object { $currentDevices -notcontains ([string]$_.device) })
    $windowWasOnRemovedMonitor = $false
    if ($before.window -and $removedMonitors.Count -gt 0) {
        $windowWasOnRemovedMonitor = Test-WindowIntersectsMonitor $before.window $removedMonitors
    }
    $expected = switch ($ExpectedTopologyChange) {
        'PrimaryChanged' { $oldPrimary -and $newPrimary -and $oldPrimary -ne $newPrimary }
        'MonitorRemoved' {
            ($monitors.Count -lt [int]$before.monitor_count) -and ($removedMonitors.Count -gt 0) -and $windowWasOnRemovedMonitor
        }
        default { $changed }
    }
    $recovered = $null -ne $window -and $window.visible -and (Test-WindowIntersectsMonitor $window $monitors)
    $base['expected_topology_change'] = $ExpectedTopologyChange
    $base['topology_changed'] = $changed
    $base['expected_change_observed'] = [bool]$expected
    $base['prepared_monitor_count'] = $beforeMonitors.Count
    $base['prepared_distinct_dpi_count'] = $beforeDistinctDpi.Count
    $base['prepared_mixed_dpi'] = [bool]$preparedMixedDpi
    $base['removed_devices'] = @($removedMonitors | ForEach-Object { [string]$_.device })
    $base['window_was_on_removed_monitor'] = [bool]$windowWasOnRemovedMonitor
    $base['gui_pid'] = [int]$before.gui_pid
    $base['gui_process_survived'] = ($null -ne $gui)
    $base['window'] = $window
    $base['window_recovered_to_active_monitor'] = [bool]$recovered
    $base['result'] = if ($expected -and $recovered) { 'PASS' } else { 'FAIL' }
    Write-Report $base
    if ($Enforce -and $base.result -ne 'PASS') { throw 'Topology recovery validation failed.' }
    return
}

# Exercise: reject an invalid physical setup before launching or moving the GUI.
$blocked = $RequireMixedDpi -and ($monitors.Count -lt 2 -or $distinctDpi.Count -lt 2)
if ($blocked) {
    $base['result'] = 'BLOCKED'
    $base['reason'] = 'At least two active monitors with distinct effective DPI values are required.'
    Write-Report $base
    if ($Enforce) { throw $base.reason }
    return
}

# Move the live GUI through every active monitor and verify per-monitor DPI/window placement.
$gui = Wait-GuiWindow -LaunchIfMissing
if (-not $gui) { throw 'Search Tool GUI could not be started.' }
try {
    $moves = [System.Collections.Generic.List[object]]::new()
    foreach ($m in $monitors) {
        $w = [Math]::Min(900, [Math]::Max(320, $m.width - 80))
        $h = [Math]::Min(700, [Math]::Max(240, $m.height - 80))
        $x = $m.left + [Math]::Max(20, [int](($m.width - $w) / 2))
        $y = $m.top + [Math]::Max(20, [int](($m.height - $h) / 2))
        $ok = [SearchToolDisplayProbe]::SetWindowPos($gui.hwnd, [IntPtr]::Zero, $x, $y, $w, $h, 0x0014)
        Start-Sleep -Milliseconds 600
        $gui.process.Refresh()
        if ($gui.process.HasExited) { throw "GUI exited while moving to $($m.device)." }
        $window = Get-WindowSnapshot $gui.hwnd
        $intersects = Test-WindowIntersectsMonitor $window @($m)
        $dpiMatch = [Math]::Abs([int]$window.dpi - [int]$m.dpi_x) -le 1
        $moves.Add([ordered]@{
            device = $m.device
            move_api_ok = [bool]$ok
            monitor_dpi = [int]$m.dpi_x
            window_dpi = [int]$window.dpi
            dpi_match = [bool]$dpiMatch
            intersects_target = [bool]$intersects
            window = $window
        })
    }
    $failedMove = @($moves | Where-Object { -not $_.move_api_ok -or -not $_.intersects_target -or -not $_.dpi_match }).Count -gt 0
    $base['gui_pid'] = $gui.process.Id
    $base['moves'] = $moves
    $base['result'] = if ($failedMove) { 'FAIL' } else { 'PASS' }
    Write-Report $base
    if ($Enforce -and $base.result -ne 'PASS') { throw 'Mixed-DPI GUI move validation failed.' }
} finally {
    if ($gui.launched -and -not $gui.process.HasExited) {
        Stop-Process -Id $gui.process.Id -Force -ErrorAction SilentlyContinue
    }
}
