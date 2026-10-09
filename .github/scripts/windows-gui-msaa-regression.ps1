# Real Win32 MSAA labels and owner-drawn item names, no desktop input.
# CI uses a synthetic index and a hidden GUI. Windows PowerShell 5.1 is
# required because it exposes the standard .NET Accessibility COM interop.
[CmdletBinding()]
param(
    [ValidateRange(1, 60)]
    [int] $TimeoutSeconds = 20
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$gui = (Resolve-Path -LiteralPath (Join-Path $root 'target\release\search-tool-gui.exe')).Path
$index = (Resolve-Path -LiteralPath (Join-Path $root 'tests\fixtures\gui-synthetic-index')).Path
$report = Join-Path $env:TEMP ('search-tool-msaa-' + $PID + '.txt')
Remove-Item -LiteralPath $report -Force -ErrorAction SilentlyContinue

Add-Type -AssemblyName Accessibility
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class SearchToolMsaaRegression {
    public delegate bool EnumCallback(IntPtr hwnd, IntPtr context);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumCallback callback, IntPtr context);
    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr hwnd, EnumCallback callback, IntPtr context);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hwnd, StringBuilder value, int length);
    [DllImport("user32.dll")]
    public static extern IntPtr GetDlgItem(IntPtr hwnd, int id);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);
    [StructLayout(LayoutKind.Sequential)]
    public struct WinRect { public int Left; public int Top; public int Right; public int Bottom; }
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out WinRect rect);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    public delegate void WinEventCallback(IntPtr hook, uint eventType, IntPtr hwnd,
        int objectId, int childId, uint eventThread, uint eventTime);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SetWinEventHook(uint eventMin, uint eventMax,
        IntPtr module, WinEventCallback callback, uint processId,
        uint threadId, uint flags);
    [DllImport("user32.dll")]
    public static extern bool UnhookWinEvent(IntPtr hook);
    [StructLayout(LayoutKind.Sequential)]
    public struct WinPoint { public int X; public int Y; }
    [StructLayout(LayoutKind.Sequential)]
    public struct WinMessage {
        public IntPtr Hwnd; public uint Message; public IntPtr WParam;
        public IntPtr LParam; public uint Time; public WinPoint Pt;
        public uint Private;
    }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern bool PeekMessage(out WinMessage msg, IntPtr hwnd,
        uint min, uint max, uint remove);
    [DllImport("user32.dll")]
    public static extern bool TranslateMessage(ref WinMessage msg);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr DispatchMessage(ref WinMessage msg);
    private static WinEventCallback retainedEventCallback;
    private static IntPtr watchedList;
    public static int ReceivedListNameChanges;
    private static void OnWinEvent(IntPtr hook, uint eventType, IntPtr hwnd,
        int objectId, int childId, uint eventThread, uint eventTime) {
        if (eventType == 0x800c && hwnd == watchedList &&
            objectId == -4 && childId == 0) {
            System.Threading.Interlocked.Increment(ref ReceivedListNameChanges);
        }
    }
    public static IntPtr BeginListNameWatch(IntPtr list, uint processId) {
        watchedList = list;
        ReceivedListNameChanges = 0;
        retainedEventCallback = OnWinEvent;
        // Out-of-context callback, explicitly scoped to the isolated process.
        return SetWinEventHook(0x800c, 0x800c, IntPtr.Zero,
            retainedEventCallback, processId, 0, 0);
    }
    public static bool PumpUntilEvents(int count, int timeoutMs) {
        DateTime end = DateTime.UtcNow.AddMilliseconds(timeoutMs);
        while (DateTime.UtcNow < end) {
            WinMessage message;
            while (PeekMessage(out message, IntPtr.Zero, 0, 0, 1)) {
                TranslateMessage(ref message);
                DispatchMessage(ref message);
            }
            if (ReceivedListNameChanges >= count) return true;
            System.Threading.Thread.Sleep(10);
        }
        return ReceivedListNameChanges >= count;
    }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr SendMessage(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("oleacc.dll")]
    public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint objectId,
        ref Guid interfaceId, [MarshalAs(UnmanagedType.Interface)] out object accessible);

    private static string WindowClass(IntPtr hwnd) {
        var value = new StringBuilder(128);
        GetClassName(hwnd, value, value.Capacity);
        return value.ToString();
    }

    public static IntPtr FindHiddenParent(int pid) {
        IntPtr match = IntPtr.Zero;
        EnumWindows((hwnd, ignored) => {
            uint found;
            GetWindowThreadProcessId(hwnd, out found);
            if (found == (uint)pid && WindowClass(hwnd) == "SearchToolWindow") {
                match = hwnd;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return match;
    }

    public static Dictionary<string, IntPtr> Controls(IntPtr parent) {
        var controls = new Dictionary<string, IntPtr>();
        EnumChildWindows(parent, (hwnd, ignored) => {
            string name = WindowClass(hwnd);
            if (name == "Edit" || name == "ListBox") controls[name] = hwnd;
            return true;
        }, IntPtr.Zero);
        return controls;
    }

    public static string AccessibleName(IntPtr hwnd, int childId) {
        var iid = new Guid("618736e0-3c3d-11cf-810c-00aa00389b71");
        object raw;
        int hr = AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC, ref iid, out raw);
        if (hr != 0) throw new InvalidOperationException(
            "AccessibleObjectFromWindow HRESULT=0x" + hr.ToString("X8"));
        try {
            var accessible = (Accessibility.IAccessible)raw;
            return accessible.get_accName(childId) ?? "";
        } finally {
            if (Marshal.IsComObject(raw)) Marshal.ReleaseComObject(raw);
        }
    }
}
'@ -ReferencedAssemblies 'Accessibility'

# Quoted args support paths containing spaces; the test never touches a real index.
$argsString = '--ui-selftest "' + $index + '" --ui-selftest-report "' + $report + '" --ui-selftest-inspect-ms 30000 --ui-selftest-winevent-offscreen'
$process = Start-Process -FilePath $gui -WorkingDirectory $root -ArgumentList $argsString -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $parent = [IntPtr]::Zero
    while ($parent -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        $parent = [SearchToolMsaaRegression]::FindHiddenParent($process.Id)
        if ($parent -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 100 }
    }
    if ($parent -eq [IntPtr]::Zero) { throw 'Hidden GUI parent was not created' }
    # EnumWindows may see the parent before WM_CREATE has created children.
    $controls = [SearchToolMsaaRegression]::Controls($parent)
    while ((-not $controls.ContainsKey('Edit') -or -not $controls.ContainsKey('ListBox') -or
        -not (Test-Path -LiteralPath $report -PathType Leaf)) -and
        [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        Start-Sleep -Milliseconds 100
        $controls = [SearchToolMsaaRegression]::Controls($parent)
    }
    if (-not $controls.ContainsKey('Edit') -or -not $controls.ContainsKey('ListBox')) {
        throw 'Expected native Edit and ListBox HWNDs are missing'
    }
    # Fixture was hidden during native selftest and then changed to an
    # offscreen, non-activating WS_VISIBLE popup for real WinEvent delivery.
    while (-not [SearchToolMsaaRegression]::IsWindowVisible($parent) -and
        [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        Start-Sleep -Milliseconds 50
    }
    if (-not [SearchToolMsaaRegression]::IsWindowVisible($parent)) {
        throw 'Synthetic offscreen WinEvent fixture did not become WS_VISIBLE'
    }
    $windowRect = New-Object SearchToolMsaaRegression+WinRect
    if (-not [SearchToolMsaaRegression]::GetWindowRect($parent, [ref]$windowRect) -or
        $windowRect.Right -gt -20000 -or $windowRect.Bottom -gt -20000 -or
        [SearchToolMsaaRegression]::GetForegroundWindow() -eq $parent) {
        throw 'WinEvent probe unexpectedly entered the visible desktop or took focus'
    }
    $queryName = [SearchToolMsaaRegression]::AccessibleName($controls['Edit'], 0)
    $resultsName = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 0)
    # The first synthetic query has three results; MSAA should expose the
    # count via the preceding (hidden) STATIC label, not just the generic role.
    $expectedResults = 'Arama sonu' + [char]0x00E7 + 'lar' + [char]0x0131 +
        ' (3 sonu' + [char]0xE7 + ')'
    if ($queryName -cne 'Arama sorgusu') {
        throw "Incorrect accessible search label: $queryName"
    }
    if ($resultsName -cne $expectedResults) {
        throw "Incorrect accessible ListBox label: $resultsName"
    }
    $firstResult = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 1)
    if ($firstResult -notmatch 'SearchTool' -or $firstResult -notmatch 'Dosya' -or
        $firstResult -notmatch 'C:\\Users\\Demo') {
        throw "First synthetic MSAA result missing title, kind or path: $firstResult"
    }

    # The four owner-drawn category chips render independently of their HWND
    # captions. Verify their actual MSAA names, including the selected state.
    # Compose Turkish characters in ASCII-only Windows PowerShell 5.1 source.
    $expectedButtons = @(
        @{ Id = 10; Name = ('T' + [char]0xFC + 'm' + [char]0xFC + ' (se' + [char]0xE7 + 'ili)') }
        @{ Id = 11; Name = 'Dosyalar' }
        @{ Id = 12; Name = ('Klas' + [char]0xF6 + 'rler') }
        @{ Id = 13; Name = ([char]0x130 + [char]0xE7 + 'erik') }
        @{ Id = 14; Name = ('G' + [char]0xF6 + 'r' + [char]0xFC + 'n' + [char]0xFC + 'm') }
        @{ Id = 19; Name = ('A' + [char]0xE7) }
    )
    foreach ($button in $expectedButtons) {
        $control = [SearchToolMsaaRegression]::GetDlgItem($parent, [int]$button.Id)
        if ($control -eq [IntPtr]::Zero) {
            throw "Button HWND missing: $($button.Id)"
        }
        $actual = [SearchToolMsaaRegression]::AccessibleName($control, 0)
        if ($actual -cne $button.Name) {
            throw "Button $($button.Id) accessible name mismatch: expected=$($button.Name), got=$actual"
        }
    }
    # External SetWinEventHook observer: this PowerShell process has its own
    # message loop and monitors the other, isolated GUI process only.
    # Category changes go through the actual WM_COMMAND handler and must
    # produce two name-change notifications, 3 -> N -> 3.
    $folderButton = [SearchToolMsaaRegression]::GetDlgItem($parent, 12)
    $eventHook = [SearchToolMsaaRegression]::BeginListNameWatch(
        $controls['ListBox'], [uint32]$process.Id)
    if ($eventHook -eq [IntPtr]::Zero) {
        throw 'SetWinEventHook could not subscribe to the synthetic GUI process'
    }
    try {
        [void][SearchToolMsaaRegression]::SendMessage(
            $parent, [uint32]0x0111, [IntPtr]12, $folderButton)
        $filteredCount = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 0)
        if ($filteredCount -ceq $expectedResults) {
            throw 'Folder category did not change synthetic search result count'
        }
        if (-not [SearchToolMsaaRegression]::PumpUntilEvents(1, 5000)) {
            throw 'External WinEvent callback missed filtered native ListBox name change'
        }
        [void][SearchToolMsaaRegression]::SendMessage(
            $parent, [uint32]0x0111, [IntPtr]10,
            [SearchToolMsaaRegression]::GetDlgItem($parent, 10))
        $restoredCount = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 0)
        if ($restoredCount -cne $expectedResults) {
            throw "Restored native ListBox name mismatch: $restoredCount"
        }
        if (-not [SearchToolMsaaRegression]::PumpUntilEvents(2, 5000)) {
            throw 'External WinEvent callback missed restored ListBox name change'
        }
        # Selecting the same category again must NOT emit duplicate
        # name-change notifications for an unchanged result count.
        [void][SearchToolMsaaRegression]::SendMessage(
            $parent, [uint32]0x0111, [IntPtr]10,
            [SearchToolMsaaRegression]::GetDlgItem($parent, 10))
        [void][SearchToolMsaaRegression]::PumpUntilEvents(3, 200)
        if ([SearchToolMsaaRegression]::ReceivedListNameChanges -ne 2) {
            throw 'Duplicate or unexpected external ListBox name-change event'
        }
    } finally {
        [void][SearchToolMsaaRegression]::UnhookWinEvent($eventHook)
    }

    # Exercise the actual category WM_COMMAND handler on the offscreen
    # synthetic parent. No physical input or user index is touched.
    $allButton = [SearchToolMsaaRegression]::GetDlgItem($parent, 10)
    $filesButton = [SearchToolMsaaRegression]::GetDlgItem($parent, 11)
    [void][SearchToolMsaaRegression]::SendMessage($parent, [uint32]0x0111, [IntPtr]11, $filesButton)
    $filesSelected = 'Dosyalar (se' + [char]0xE7 + 'ili)'
    if ([SearchToolMsaaRegression]::AccessibleName($filesButton, 0) -cne $filesSelected -or
        [SearchToolMsaaRegression]::AccessibleName($allButton, 0) -cne ('T' + [char]0xFC + 'm' + [char]0xFC)) {
        throw 'MSAA category accessible names did not update after category command'
    }
    [void][SearchToolMsaaRegression]::SendMessage($parent, [uint32]0x0111, [IntPtr]10, $allButton)
    if ([SearchToolMsaaRegression]::AccessibleName($allButton, 0) -cne $expectedButtons[0].Name -or
        [SearchToolMsaaRegression]::AccessibleName($filesButton, 0) -cne 'Dosyalar') {
        throw 'MSAA category accessible names did not restore after All command'
    }

    if (-not (Test-Path -LiteralPath $report -PathType Leaf) -or
        (Get-Content -LiteralPath $report -Raw).Trim() -ne 'PASS') {
        throw 'Hidden Win32 GUI regression did not report PASS'
    }
    Write-Host 'MSAA offscreen GUI PASS: Edit/ListBox, six buttons, categories and external cross-process WinEvent name changes.'
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $report -Force -ErrorAction SilentlyContinue
}
