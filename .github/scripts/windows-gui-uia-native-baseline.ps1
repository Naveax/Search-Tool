# Cross-process UIA baseline for untouched native Win32 EDIT and LISTBOX.
# Creates only a short-lived offscreen synthetic Form; no keyboard input.
param([int]$TimeoutSeconds = 12)
$ErrorActionPreference = 'Stop'
$guid = [guid]::NewGuid().ToString('N')
$driver = Join-Path $env:TEMP ("search-tool-uia-native-$guid.ps1")
$marker = Join-Path $env:TEMP ("search-tool-uia-native-$guid.txt")
$childLines = @(
 'param([Parameter(Mandatory=$true)][string]$Marker)'
 '$ErrorActionPreference="Stop"'
 'Add-Type -AssemblyName System.Windows.Forms'
 'Add-Type -TypeDefinition ''using System; using System.Runtime.InteropServices; public static class NativeBaselineWin32 { [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern IntPtr CreateWindowEx(int ex,string cls,string text,uint style,int x,int y,int w,int h,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param); }'''
 '$form=New-Object System.Windows.Forms.Form'
 '$form.Text="Search Tool UIA native baseline"'
 '$form.ShowInTaskbar=$false'
 '$form.StartPosition="Manual"'
 '$form.Location=New-Object System.Drawing.Point(-30000,-30000)'
 '$form.Size=New-Object System.Drawing.Size(400,300)'
 '$form.Show()'
 '$form.Location=New-Object System.Drawing.Point(-30000,-30000)'
 '$edit=[NativeBaselineWin32]::CreateWindowEx(0,"EDIT","NativeQuery",[uint32]0x50010080,20,20,220,30,$form.Handle,[IntPtr]301,[IntPtr]::Zero,[IntPtr]::Zero)'
 '$list=[NativeBaselineWin32]::CreateWindowEx(0,"LISTBOX","",[uint32]0x50010041,20,60,220,180,$form.Handle,[IntPtr]302,[IntPtr]::Zero,[IntPtr]::Zero)'
 'if($edit -eq [IntPtr]::Zero -or $list -eq [IntPtr]::Zero){throw "CreateWindowEx failed"}'
 '[IO.File]::WriteAllText($Marker,("$([long]$form.Handle)|$([long]$edit)|$([long]$list)"))'
 '$until=[DateTime]::UtcNow.AddSeconds(20)'
 'while([DateTime]::UtcNow -lt $until){[System.Windows.Forms.Application]::DoEvents();[Threading.Thread]::Sleep(30)}'
 '$form.Close()'
)
$p = $null
try {
    $childLines -join [Environment]::NewLine | Set-Content -LiteralPath $driver -Encoding UTF8
    $argsString = '-STA -NoProfile -ExecutionPolicy Bypass -File "' + $driver + '" -Marker "' + $marker + '"'
    $p = Start-Process -FilePath powershell.exe -ArgumentList $argsString -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while(-not (Test-Path -LiteralPath $marker) -and
          [DateTime]::UtcNow -lt $deadline -and -not $p.HasExited){
        Start-Sleep -Milliseconds 100
    }
    if(-not (Test-Path -LiteralPath $marker)){
        throw 'Native Win32 UIA baseline did not create its offscreen controls'
    }
    Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
    Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class NativeBaselineRect { [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; } [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out Rect rect); }'
    $parts=(Get-Content -LiteralPath $marker -Raw).Trim().Split('|')
    if($parts.Length -ne 3){throw 'Malformed native UIA baseline handles'}
    $handles=@($parts | ForEach-Object {[IntPtr][long]$_})
    $rect=New-Object NativeBaselineRect+Rect
    if(-not [NativeBaselineRect]::GetWindowRect($handles[0],[ref]$rect) -or
       $rect.Right -gt -20000 -or $rect.Bottom -gt -20000){
        throw 'Native UIA baseline moved onscreen'
    }
    $roles=@{}
    foreach($spec in @(@('Edit',1),@('ListBox',2))){
        $cls=$spec[0]; $index=$spec[1]
        $element=[System.Windows.Automation.AutomationElement]::FromHandle($handles[$index])
        if($null -eq $element -or $element.Current.ClassName -cne $cls -or
           $element.Current.NativeWindowHandle -ne $handles[$index].ToInt64() -or
           $element.Current.ProcessId -ne $p.Id){
            throw "UIA native baseline identity mismatch: $cls"
        }
        $roles[$cls]=$element.Current.ControlType.ProgrammaticName
    }
    Write-Output ('BASELINE_EDIT_ROLE=' + $roles['Edit'])
    Write-Output ('BASELINE_LIST_ROLE=' + $roles['ListBox'])
    if($roles['Edit'] -ceq 'ControlType.Edit' -and
       $roles['ListBox'] -ceq 'ControlType.List'){
        Write-Output 'BASELINE_NATIVE_UIA_ROLES=AVAILABLE'
    }else{
        Write-Output 'BASELINE_NATIVE_UIA_ROLES=LIMITED'
    }
}finally{
    if($null -ne $p -and -not $p.HasExited){
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $driver,$marker -Force -ErrorAction SilentlyContinue
}
