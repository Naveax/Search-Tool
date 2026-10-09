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
$argsString = '--ui-selftest "' + $index + '" --ui-selftest-report "' + $report + '" --ui-selftest-inspect-ms 30000'
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
    $queryName = [SearchToolMsaaRegression]::AccessibleName($controls['Edit'], 0)
    $resultsName = [SearchToolMsaaRegression]::AccessibleName($controls['ListBox'], 0)
    $expectedResults = 'Arama sonu' + [char]0x00E7 + 'lar' + [char]0x0131
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
    # Exercise one real WM_COMMAND handler on the isolated hidden parent.
    # Confirm the MSAA accessible caption responds to category changes.
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
    Write-Host 'MSAA hidden GUI PASS: Edit/ListBox, synthetic result, six button names and category state transitions.'
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $report -Force -ErrorAction SilentlyContinue
}
