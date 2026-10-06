# Physical GUI acceptance test: run as the interactive user, never as SYSTEM.
# Requires one installed Search Tool resident GUI and its running indexer service.
[CmdletBinding()]
param(
    [string] $GuiExe = (Join-Path $env:ProgramFiles 'Search Tool\search-tool-gui.exe'),
    [string] $IndexDirectory = (Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'SearchTool\index'),
    [string[]] $Queries = @('readme', 'notepad'),
    [int] $TimeoutSeconds = 10,
    [switch] $CaptureScreenshots,
    [string] $EvidenceDirectory = (Join-Path $env:TEMP 'SearchTool-Gui-Physical-Smoke')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not [Environment]::UserInteractive) { throw 'An interactive Windows session is required' }
if (-not (Test-Path -LiteralPath $GuiExe -PathType Leaf)) { throw "GUI not found: $GuiExe" }
if (-not (Test-Path -LiteralPath $IndexDirectory -PathType Container)) { throw 'Index directory missing' }
if ($TimeoutSeconds -lt 1 -or $TimeoutSeconds -gt 60) { throw 'TimeoutSeconds must be 1..60' }
if ($Queries.Count -eq 0) { throw 'Queries must not be empty' }
foreach ($query in $Queries) {
    if ([string]::IsNullOrWhiteSpace($query) -or $query.Contains('"')) { throw 'Query must be nonempty and must not contain a double quote' }
}
$service = Get-Service SearchToolIndexer -ErrorAction Stop
if ($service.Status -ne 'Running') { throw 'SearchToolIndexer is not running' }
$active = @(Get-CimInstance Win32_Process -Filter "Name='search-tool-gui.exe'")
if ($active.Count -ne 1) { throw "Expected exactly one GUI, found $($active.Count)" }
$expectedExe = [IO.Path]::GetFullPath($GuiExe)
if (-not [string]::Equals($active[0].ExecutablePath, $expectedExe, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Running GUI is not the installed binary: $($active[0].ExecutablePath)"
}

Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class GuiPhysicalNative {
 public delegate bool Proc(IntPtr h, IntPtr data);
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
 [DllImport("user32.dll")] public static extern bool EnumWindows(Proc cb,IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,Proc cb,IntPtr data);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint id);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder s,int len);
 // SMTO_ABORTIFHUNG | SMTO_BLOCK. Every UI read must be bounded, including
 // the WM_GETTEXT and LB_GETTEXT messages to an unresponsive GUI process.
 [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW",CharSet=CharSet.Unicode,SetLastError=true)]
 private static extern IntPtr SendTextTimeout(IntPtr h,uint m,IntPtr w,StringBuilder l,uint flags,uint milliseconds,out IntPtr response);
 [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW",CharSet=CharSet.Unicode,SetLastError=true)]
 private static extern IntPtr SendNumberTimeout(IntPtr h,uint m,IntPtr w,IntPtr l,uint flags,uint milliseconds,out IntPtr response);
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int n);
 private const uint TimeoutFlags=0x0003;
 private const uint MessageTimeoutMs=1500;
 private static int BoundedTextMessage(IntPtr h,uint m,IntPtr w,StringBuilder l){
  IntPtr value;
  if(SendTextTimeout(h,m,w,l,TimeoutFlags,MessageTimeoutMs,out value)==IntPtr.Zero)
   throw new InvalidOperationException("GUI message timed out or failed: "+m);
  return value.ToInt32();
 }
 private static int BoundedNumberMessage(IntPtr h,uint m,IntPtr w,IntPtr l){
  IntPtr value;
  if(SendNumberTimeout(h,m,w,l,TimeoutFlags,MessageTimeoutMs,out value)==IntPtr.Zero)
   throw new InvalidOperationException("GUI message timed out or failed: "+m);
  return value.ToInt32();
 }
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int n);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect rect);
 public static string Class(IntPtr h){
  var b=new StringBuilder(128);GetClassName(h,b,b.Capacity);return b.ToString();
 }
 public static IntPtr Window(int target){
  IntPtr result=IntPtr.Zero;
  EnumWindows((h,d)=>{
   uint id;GetWindowThreadProcessId(h,out id);
   if(id==(uint)target && Class(h)=="SearchToolWindow"){result=h;return false;}
   return true;
  },IntPtr.Zero);
  return result;
 }
 public static List<IntPtr> Children(IntPtr parent){
  var result=new List<IntPtr>();
  EnumChildWindows(parent,(h,d)=>{result.Add(h);return true;},IntPtr.Zero);
  return result;
 }
 public static string Text(IntPtr h){
  var b=new StringBuilder(4096);
  BoundedTextMessage(h,0x000D,new IntPtr(b.Capacity),b);
  return b.ToString();
 }
 public static void RestoreText(IntPtr h,string text){
  var b=new StringBuilder(text);
  if(BoundedTextMessage(h,0x000C,IntPtr.Zero,b)==0)
   throw new InvalidOperationException("Unable to restore previous GUI query");
 }
 public static int Count(IntPtr h){return BoundedNumberMessage(h,0x018B,IntPtr.Zero,IntPtr.Zero);}
 public static string First(IntPtr h){
  var b=new StringBuilder(4096);
  if(BoundedTextMessage(h,0x0189,IntPtr.Zero,b)<0)return "";
  return b.ToString();
 }
}
'@
$window = [GuiPhysicalNative]::Window([int]$active[0].ProcessId)
if ($window -eq [IntPtr]::Zero) { throw 'SearchToolWindow not found' }
$children = [GuiPhysicalNative]::Children($window)
$edits = @($children | Where-Object { [GuiPhysicalNative]::Class($_) -eq 'Edit' })
$lists = @($children | Where-Object { [GuiPhysicalNative]::Class($_) -eq 'ListBox' })
$statics = @($children | Where-Object { [GuiPhysicalNative]::Class($_) -eq 'Static' })
if ($edits.Count -ne 1 -or $lists.Count -ne 1) { throw 'Required native Edit/ListBox controls missing' }
$edit = [IntPtr]$edits[0]
$list = [IntPtr]$lists[0]
$oldQuery = [GuiPhysicalNative]::Text($edit)
$oldVisible = [GuiPhysicalNative]::IsWindowVisible($window)
$oldForeground = [GuiPhysicalNative]::GetForegroundWindow()
New-Item -ItemType Directory -Path $EvidenceDirectory -Force | Out-Null
$results = @()
$failure = $null
try {
    foreach ($query in $Queries) {
        $arguments = '"{0}" --query "{1}"' -f $IndexDirectory, $query
        $invocation = Start-Process -FilePath $GuiExe -ArgumentList $arguments -PassThru
        try {
            if (-not $invocation.WaitForExit($TimeoutSeconds * 1000)) {
                $invocation.Kill()
                [void]$invocation.WaitForExit(3000)
                throw "GUI IPC invocation timed out for '$query'"
            }
            if ($invocation.ExitCode -ne 0) { throw "IPC invocation failed for '$query' (exit $($invocation.ExitCode))" }
        } finally {
            $invocation.Dispose()
        }
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $hit = $null
        do {
            Start-Sleep -Milliseconds 150
            $text = [GuiPhysicalNative]::Text($edit)
            $count = [GuiPhysicalNative]::Count($list)
            $first = if ($count -gt 0) { [GuiPhysicalNative]::First($list) } else { '' }
            $status = @($statics | ForEach-Object { [GuiPhysicalNative]::Text($_) } |
                Where-Object { $_ -match '^\s*\d+\s+sonu' } | Select-Object -First 1)
            $statusText = if ($status.Count -gt 0) { [string]$status[0] } else { '' }
            $statusCount = if ($statusText -match '^\s*(\d+)\s+sonu') { [int]$Matches[1] } else { -1 }
            if ([string]::Equals($text,$query,[StringComparison]::OrdinalIgnoreCase) -and
                $count -gt 0 -and $statusCount -eq $count -and
                $first.IndexOf($query,[StringComparison]::OrdinalIgnoreCase) -ge 0) {
                $hit = [ordered]@{
                    query = $query
                    first_result = $first
                    result_count = $count
                    status = $statusText
                    result = 'PASS'
                }
                break
            }
        } while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds)
        if ($null -eq $hit) {
            throw "Query '$query' failed: edit='$text', count=$count, status='$statusText', first='$first'"
        }
        if ($CaptureScreenshots) {
            Add-Type -AssemblyName System.Drawing
            [void][GuiPhysicalNative]::ShowWindowAsync($window,9)
            [void][GuiPhysicalNative]::SetForegroundWindow($window)
            Start-Sleep -Milliseconds 250
            if ([GuiPhysicalNative]::GetForegroundWindow() -ne $window) {
                throw 'Screenshot capture blocked: GUI is not the foreground window'
            }
            $rect = New-Object GuiPhysicalNative+Rect
            if (-not [GuiPhysicalNative]::GetWindowRect($window,[ref]$rect)) { throw 'GetWindowRect failed' }
            $w = $rect.Right - $rect.Left
            $h = $rect.Bottom - $rect.Top
            if ($w -lt 100 -or $h -lt 100) { throw 'Invalid screenshot bounds' }
            $bitmap = New-Object Drawing.Bitmap($w,$h)
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try { $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size) }
            finally { $graphics.Dispose() }
            $safeQuery = [regex]::Replace($query,'[^a-zA-Z0-9_-]','_')
            $screen = Join-Path $EvidenceDirectory "gui-query-$safeQuery.png"
            try { $bitmap.Save($screen,[Drawing.Imaging.ImageFormat]::Png) }
            finally { $bitmap.Dispose() }
            $hit['screenshot'] = $screen
        }
        $results += [pscustomobject]$hit
    }
} catch {
    $failure = $_.Exception.Message
} finally {
    # Preserve whether the resident GUI was hidden, and restore the prior query.
    try {
        [GuiPhysicalNative]::RestoreText($edit,$oldQuery)
        if ([GuiPhysicalNative]::Text($edit) -cne $oldQuery) {
            throw 'Restored GUI query did not match the original'
        }
        if (-not $oldVisible) {
            [void][GuiPhysicalNative]::ShowWindowAsync($window,0)
            Start-Sleep -Milliseconds 150
            if ([GuiPhysicalNative]::IsWindowVisible($window)) {
                throw 'GUI was not returned to its originally hidden state'
            }
        }
        if ($oldForeground -ne [IntPtr]::Zero -and $oldForeground -ne $window) {
            [void][GuiPhysicalNative]::SetForegroundWindow($oldForeground)
        }
    } catch {
        $restorationError = $_.Exception.Message
        if ($null -eq $failure) { $failure = "GUI restoration failed: $restorationError" }
        else { $failure += "; GUI restoration failed: $restorationError" }
    }
}
$evidence = [ordered]@{
    schema = 1
    timestamp_utc = (Get-Date).ToUniversalTime().ToString('o')
    result = if ($null -eq $failure) { 'PASS' } else { 'FAIL' }
    reason = $failure
    machine = [Environment]::MachineName
    source = 'Real Windows desktop, Win32 Edit/ListBox/Static UI controls'
    gui_executable = $expectedExe
    gui_sha256 = (Get-FileHash -LiteralPath $GuiExe -Algorithm SHA256).Hash
    gui_process_id = [int]$active[0].ProcessId
    service_state = $service.Status.ToString()
    checks = $results
}
$path = Join-Path $EvidenceDirectory 'gui-physical-smoke.json'
$evidence | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $path -Encoding UTF8
$evidence | ConvertTo-Json -Depth 7
Write-Output "Evidence: $path"
if ($null -ne $failure) { throw $failure }