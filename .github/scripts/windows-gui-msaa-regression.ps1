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
    public static extern IntPtr GetNextDlgTabItem(IntPtr dialog, IntPtr current, bool previous);
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
    private static WinEventCallback retainedSelectionCallback;
    private static readonly List<string> selectionEvents = new List<string>();
    private static void OnSelectionEvent(IntPtr hook, uint eventType, IntPtr hwnd,
        int objectId, int childId, uint eventThread, uint eventTime) {
        if (hwnd == watchedList && objectId == -4) {
            selectionEvents.Add(eventType.ToString("X4") + ":" + childId);
        }
    }
    public static IntPtr BeginListSelectionWatch(IntPtr list, uint processId) {
        watchedList = list;
        selectionEvents.Clear();
        retainedSelectionCallback = OnSelectionEvent;
        // Observe only EVENT_OBJECT_SELECTION. Do not treat other
        // selection-related events as proof of this exact notification.
        return SetWinEventHook(0x8006, 0x8006, IntPtr.Zero,
            retainedSelectionCallback, processId, 0, 0);
    }
    public static string PumpUntilSelectionEvents(int expectedCount, int timeoutMs) {
        DateTime end = DateTime.UtcNow.AddMilliseconds(timeoutMs);
        do {
            WinMessage message;
            while (PeekMessage(out message, IntPtr.Zero, 0, 0, 1)) {
                TranslateMessage(ref message);
                DispatchMessage(ref message);
            }
            if (selectionEvents.Count >= expectedCount) break;
            System.Threading.Thread.Sleep(10);
        } while (DateTime.UtcNow < end);
        return String.Join(",", selectionEvents.ToArray());
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

    public static void AssertNativeListSelection(IntPtr hwnd, int count, int selectedChild) {
        var iid = new Guid("618736e0-3c3d-11cf-810c-00aa00389b71");
        object raw;
        int hr = AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC, ref iid, out raw);
        if (hr != 0) throw new InvalidOperationException(
            "ListBox MSAA HRESULT=0x" + hr.ToString("X8"));
        try {
            var acc = (Accessibility.IAccessible)raw;
            if (acc.accChildCount != count ||
                Convert.ToInt32(acc.get_accRole(0)) != 33 ||
                Convert.ToInt32(acc.accSelection) != selectedChild) {
                throw new InvalidOperationException(
                    "MSAA list count, role or selected child mismatch: count=" +
                    acc.accChildCount + " role=" + acc.get_accRole(0) +
                    " selection=" + acc.accSelection);
            }
            for (int child = 1; child <= count; child++) {
                if (Convert.ToInt32(acc.get_accRole(child)) != 34) {
                    throw new InvalidOperationException("MSAA list item role mismatch: " + child);
                }
                var state = Convert.ToInt64(acc.get_accState(child));
                bool selected = (state & 0x2L) != 0; // STATE_SYSTEM_SELECTED
                if (selected != (child == selectedChild)) {
                    throw new InvalidOperationException(
                        "MSAA selected state mismatch: child=" + child + " state=" + state);
                }
                if (String.IsNullOrWhiteSpace(acc.get_accName(child))) {
                    throw new InvalidOperationException("MSAA item name empty: " + child);
                }
            }
        } finally {
            if (Marshal.IsComObject(raw)) Marshal.ReleaseComObject(raw);
        }
    }
    public static void AssertNativeFocusable(IntPtr hwnd, int expectedRole) {
        var iid = new Guid("618736e0-3c3d-11cf-810c-00aa00389b71");
        object raw;
        int hr = AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC, ref iid, out raw);
        if (hr != 0) throw new InvalidOperationException(
            "MSAA focusability HRESULT=0x" + hr.ToString("X8"));
        try {
            var acc = (Accessibility.IAccessible)raw;
            var state = Convert.ToInt64(acc.get_accState(0));
            if (Convert.ToInt32(acc.get_accRole(0)) != expectedRole ||
                (state & 0x100000L) == 0) {
                throw new InvalidOperationException(
                    "MSAA native focusable role/state mismatch: role=" +
                    acc.get_accRole(0) + " state=" + state);
            }
        } finally {
            if (Marshal.IsComObject(raw)) Marshal.ReleaseComObject(raw);
        }
    }
    public static string AccessibleStateSnapshot(IntPtr hwnd) {
        var iid = new Guid("618736e0-3c3d-11cf-810c-00aa00389b71");
        object raw;
        int hr = AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC, ref iid, out raw);
        if (hr != 0) throw new InvalidOperationException(
            "Accessible focus state HRESULT=0x" + hr.ToString("X8"));
        try {
            var acc = (Accessibility.IAccessible)raw;
            var state = Convert.ToInt64(acc.get_accState(0));
            return "role=" + acc.get_accRole(0) + " state=" + state +
                " focus=" + (acc.accFocus == null ? "null" : acc.accFocus.ToString());
        } finally {
            if (Marshal.IsComObject(raw)) Marshal.ReleaseComObject(raw);
        }
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
    $rectOk = [SearchToolMsaaRegression]::GetWindowRect($parent, [ref]$windowRect)
    $becameForeground = [SearchToolMsaaRegression]::GetForegroundWindow() -eq $parent
    Write-Host ("Offscreen WinEvent fixture: rect=({0},{1},{2},{3}) foreground={4}" -f
        $windowRect.Left, $windowRect.Top, $windowRect.Right,
        $windowRect.Bottom, $becameForeground)
    if (-not $rectOk -or $windowRect.Right -gt -20000 -or
        $windowRect.Bottom -gt -20000) {
        throw 'WinEvent probe unexpectedly entered the visible desktop'
    }
    # CI runners can assign the only top-level GUI window as foreground
    # despite SW_SHOWNOACTIVATE. Never inject keyboard or pointer input.
    # Its actual offscreen geometry is the non-negotiable safety boundary.
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
    [SearchToolMsaaRegression]::AssertNativeListSelection($controls['ListBox'], 3, 1)
    $selectionHook = [SearchToolMsaaRegression]::BeginListSelectionWatch(
        $controls['ListBox'], [uint32]$process.Id)
    if ($selectionHook -eq [IntPtr]::Zero) {
        throw 'External selection WinEvent observer could not subscribe'
    }
    try {
    # Simulate selection through the native list control without keyboard
    # injection; inspect the actual cross-process MSAA states afterward.
    $secondSet = [SearchToolMsaaRegression]::SendMessage(
        $controls['ListBox'], [uint32]0x0186, [IntPtr]1, [IntPtr]::Zero)
    if ($secondSet.ToInt64() -ne 1) {
        throw 'Synthetic ListBox could not select second result'
    }
    [SearchToolMsaaRegression]::AssertNativeListSelection($controls['ListBox'], 3, 2)
    $firstEvent = [SearchToolMsaaRegression]::PumpUntilSelectionEvents(1, 5000)
    if ($firstEvent -cne '8006:2') {
        throw "External selected-item WinEvent missing or unexpected for second item: $firstEvent"
    }
    $firstSet = [SearchToolMsaaRegression]::SendMessage(
        $controls['ListBox'], [uint32]0x0186, [IntPtr]0, [IntPtr]::Zero)
    if ($firstSet.ToInt64() -ne 0) {
        throw 'Synthetic ListBox could not restore first result'
    }
    [SearchToolMsaaRegression]::AssertNativeListSelection($controls['ListBox'], 3, 1)
    $bothEvents = [SearchToolMsaaRegression]::PumpUntilSelectionEvents(2, 5000)
    if ($bothEvents -cne '8006:2,8006:1') {
        throw "External selected-item WinEvent order or item identity mismatch: $bothEvents"
    }
    Write-Host ("MSAA external selection events PASS: " + $bothEvents)
    } finally {
        [void][SearchToolMsaaRegression]::UnhookWinEvent($selectionHook)
    }
    $firstResult = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 1)
    if ($firstResult -notmatch 'SearchTool' -or $firstResult -notmatch 'Dosya' -or
        $firstResult -notmatch 'C:\\Users\\Demo') {
        throw "First synthetic MSAA result missing title, kind or path: $firstResult"
    }

    # External UI Automation client runs in this PowerShell process,
    # inspecting only native controls of the synthetic offscreen GUI.
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
    $uiaEdit = [System.Windows.Automation.AutomationElement]::FromHandle($controls['Edit'])
    $uiaList = [System.Windows.Automation.AutomationElement]::FromHandle($controls['ListBox'])
    if ($null -eq $uiaEdit -or $null -eq $uiaList) {
        throw 'UIA FromHandle returned null for synthetic native controls'
    }
    $uiaRoot = [System.Windows.Automation.AutomationElement]::FromHandle($parent)
    if ($null -eq $uiaRoot -or $uiaRoot.Current.ClassName -cne 'SearchToolWindow' -or
        $uiaRoot.Current.NativeWindowHandle -ne $parent.ToInt64() -or
        $uiaRoot.Current.ProcessId -ne $process.Id) {
        throw 'UIA parent identity mismatch for isolated Search Tool GUI'
    }
    # This checks actual cross-process UIA tree membership and HWND identity;
    # it does NOT mistake generic Pane proxies for accessible Edit/List roles.
    $uiaChildren = $uiaRoot.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    if ($uiaChildren.Count -lt 8) {
        throw "UIA subtree missing native controls: count=$($uiaChildren.Count)"
    }
    $expectedUiClasses = @(
        @{Element=$uiaEdit; Handle=$controls['Edit']; Class='Edit'},
        @{Element=$uiaList; Handle=$controls['ListBox']; Class='ListBox'}
    )
    foreach ($item in $expectedUiClasses) {
        if ($item.Element.Current.NativeWindowHandle -ne $item.Handle.ToInt64() -or
            $item.Element.Current.ClassName -cne $item.Class) {
            throw "UIA native control identity mismatch: $($item.Class)"
        }
    }
    Write-Host ("FOCUSABILITY Edit: MSAA={0}; UIA keyboard-focusable={1}; UIA has-focus={2}" -f
        [SearchToolMsaaRegression]::AccessibleStateSnapshot($controls['Edit']),
        $uiaEdit.Current.IsKeyboardFocusable, $uiaEdit.Current.HasKeyboardFocus)
    Write-Host ("FOCUSABILITY List: MSAA={0}; UIA keyboard-focusable={1}; UIA has-focus={2}" -f
        [SearchToolMsaaRegression]::AccessibleStateSnapshot($controls['ListBox']),
        $uiaList.Current.IsKeyboardFocusable, $uiaList.Current.HasKeyboardFocus)
    $nativeUiaRoles = ($uiaEdit.Current.ControlType.ProgrammaticName -ceq 'ControlType.Edit' -and
        $uiaList.Current.ControlType.ProgrammaticName -ceq 'ControlType.List' -and
        $uiaEdit.Current.Name -ceq $queryName -and
        $uiaList.Current.Name -ceq $expectedResults)
    Write-Host ("UIA provider coverage: Edit={0} Name='{1}'; List={2} Name='{3}'; correct roles and names={4}" -f
        $uiaEdit.Current.ControlType.ProgrammaticName, $uiaEdit.Current.Name,
        $uiaList.Current.ControlType.ProgrammaticName, $uiaList.Current.Name,
        $nativeUiaRoles)
    [SearchToolMsaaRegression]::AssertNativeFocusable($controls['Edit'], 42)
    [SearchToolMsaaRegression]::AssertNativeFocusable($controls['ListBox'], 33)
    # True Windows child HWND traversal. No SendInput or SetFocus: inspect
    # the dialog manager's Tab/Shift+Tab targets without changing focus.
    $tabStops = @(
        $controls['Edit'],
        [SearchToolMsaaRegression]::GetDlgItem($parent, 10),
        [SearchToolMsaaRegression]::GetDlgItem($parent, 11),
        [SearchToolMsaaRegression]::GetDlgItem($parent, 12),
        [SearchToolMsaaRegression]::GetDlgItem($parent, 13),
        $controls['ListBox'],
        [SearchToolMsaaRegression]::GetDlgItem($parent, 14),
        [SearchToolMsaaRegression]::GetDlgItem($parent, 19)
    )
    foreach ($handle in $tabStops) {
        if ($handle -eq [IntPtr]::Zero) { throw 'Native Tab target HWND missing' }
    }
    for ($i = 0; $i -lt ($tabStops.Count - 1); $i++) {
        $forward = [SearchToolMsaaRegression]::GetNextDlgTabItem(
            $parent, $tabStops[$i], $false)
        $reverse = [SearchToolMsaaRegression]::GetNextDlgTabItem(
            $parent, $tabStops[$i + 1], $true)
        if ($forward -ne $tabStops[$i + 1] -or $reverse -ne $tabStops[$i]) {
            throw "External native Tab order mismatch at step $i"
        }
    }
    Write-Host 'MSAA focusability and native Tab/Shift+Tab traversal PASS (no focus change)'
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
        # Independently confirm UI Automation sees the same native HWND
        # and readable button name. A generic Pane role is still logged as
        # a separate unresolved UIA provider issue, not passed as a Button.
        $uiaButton = [System.Windows.Automation.AutomationElement]::FromHandle($control)
        if ($null -eq $uiaButton -or
            $uiaButton.Current.NativeWindowHandle -ne $control.ToInt64() -or
            $uiaButton.Current.ClassName -cne 'Button' -or
            $uiaButton.Current.Name -cne $button.Name) {
            throw "UIA native button name/identity mismatch: $($button.Id)"
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
    if ([System.Windows.Automation.AutomationElement]::FromHandle($filesButton).Current.Name -cne
        $filesSelected) {
        throw 'UIA category button did not report selected Files name'
    }
    [void][SearchToolMsaaRegression]::SendMessage($parent, [uint32]0x0111, [IntPtr]10, $allButton)
    if ([SearchToolMsaaRegression]::AccessibleName($allButton, 0) -cne $expectedButtons[0].Name -or
        [SearchToolMsaaRegression]::AccessibleName($filesButton, 0) -cne 'Dosyalar') {
        throw 'MSAA category accessible names did not restore after All command'
    }
    if ([System.Windows.Automation.AutomationElement]::FromHandle($allButton).Current.Name -cne
        $expectedButtons[0].Name) {
        throw 'UIA category button did not restore selected All name'
    }

    # Run the separate native Win32 baseline only AFTER all Search Tool
    # WinEvent hooks and category transitions finish. Starting another
    # GUI process before hooking introduced cross-session timing noise in CI.
    # Distinguish an actual Search Tool UIA provider regression from a
    # Windows test-session limitation. A separate process creates untouched
    # genuine user32 EDIT/LISTBOX child windows and another UIA client reads
    # their roles. If the baseline supports native control roles, our
    # controls must expose them too. Otherwise leave UIA acceptance OPEN.
    $baselineScript = Join-Path $PSScriptRoot 'windows-gui-uia-native-baseline.ps1'
    $baselineOutput = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $baselineScript 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw ("Native Windows UIA provider baseline failed: " + ($baselineOutput -join '; '))
    }
    $baselineEdit = @($baselineOutput | Where-Object { $_ -match '^BASELINE_EDIT_ROLE=' })
    $baselineList = @($baselineOutput | Where-Object { $_ -match '^BASELINE_LIST_ROLE=' })
    if ($baselineEdit.Count -ne 1 -or $baselineList.Count -ne 1) {
        throw ("Native Windows UIA baseline missing expected roles: " + ($baselineOutput -join '; '))
    }
    $baselineEditRole = ($baselineEdit[0] -replace '^BASELINE_EDIT_ROLE=', '')
    $baselineListRole = ($baselineList[0] -replace '^BASELINE_LIST_ROLE=', '')
    Write-Host ("Native Windows UIA baseline: Edit={0}; List={1}" -f
        $baselineEditRole, $baselineListRole)
    if ($baselineEditRole -ceq 'ControlType.Edit' -and
        $uiaEdit.Current.ControlType.ProgrammaticName -cne 'ControlType.Edit') {
        throw 'Search Tool Edit UIA role regressed despite working standard EDIT baseline'
    }
    if ($baselineListRole -ceq 'ControlType.List' -and
        $uiaList.Current.ControlType.ProgrammaticName -cne 'ControlType.List') {
        throw 'Search Tool ListBox UIA role regressed despite working standard LISTBOX baseline'
    }
    if (($baselineEditRole -ceq 'ControlType.Edit' -or
         $baselineListRole -ceq 'ControlType.List') -and
        -not $nativeUiaRoles) {
        Write-Warning 'UIA control names and selection still require manual provider acceptance'
    }
    # A limited standard Win32 UIA baseline is NOT evidence that the app
    # is fully accessible. Narrator, roles/names and UIA patterns remain a
    # separate product release gate even when this regression passes.

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
