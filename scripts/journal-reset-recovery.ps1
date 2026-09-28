[CmdletBinding()]
param(
    [int]$VhdSizeMiB = 256
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cli = Join-Path $PSScriptRoot 'search-tool.exe'
if (-not (Test-Path -LiteralPath $cli)) { $cli = Join-Path $root 'target\release\search-tool.exe' }
if (-not (Test-Path -LiteralPath $cli)) { throw "Missing release executable: search-tool.exe" }
$cli = (Resolve-Path -LiteralPath $cli).Path

$baseTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$work = Join-Path $baseTemp ('search-tool-journal-reset-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
$vhd = Join-Path $work 'journal-reset.vhdx'
$lettersInUse = @(Get-PSDrive -PSProvider FileSystem | ForEach-Object { $_.Name.ToUpperInvariant() })
$letter = @('U','V','W','X','Y') | Where-Object { $lettersInUse -notcontains $_ } | Select-Object -First 1
if (-not $letter) { throw 'No free drive letter available.' }
$drive = "${letter}:"
$index = Join-Path $work "$letter.stidx"
$create = Join-Path $work 'create.txt'
$detach = Join-Path $work 'detach.txt'
@"
create vdisk file="$vhd" maximum=$VhdSizeMiB type=expandable
select vdisk file="$vhd"
attach vdisk
create partition primary
format fs=ntfs quick label=SearchToolResetCI
assign letter=$letter
exit
"@ | Set-Content -LiteralPath $create -Encoding ASCII
@"
select vdisk file="$vhd"
detach vdisk
exit
"@ | Set-Content -LiteralPath $detach -Encoding ASCII

function Invoke-Checked {
    param(
        [Parameter(Mandatory)] [string]$FilePath,
        [Parameter(ValueFromRemainingArguments)] [string[]]$ArgumentList
    )
    & $FilePath @ArgumentList | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $FilePath $($ArgumentList -join ' ')" }
}

$mounted = $false
try {
    Write-Host '==> create isolated NTFS VHD'
    & diskpart.exe /s $create
    if ($LASTEXITCODE -ne 0) { throw "diskpart failed: $LASTEXITCODE" }
    $mounted = $true
    Invoke-Checked -FilePath fsutil.exe -ArgumentList @('usn','createjournal',$drive,'m=10485760','a=1048576')
    New-Item -ItemType Directory -Force -Path "$drive\seed" | Out-Null
    Set-Content -LiteralPath "$drive\seed\before-reset.txt" -Value 'before reset' -Encoding UTF8
    Invoke-Checked -FilePath $cli -ArgumentList @('index',$drive,$index)
    Invoke-Checked -FilePath $cli -ArgumentList @('verify-deep',$index)

    Write-Host '==> delete/recreate USN journal on isolated test volume'
    Invoke-Checked -FilePath fsutil.exe -ArgumentList @('usn','deletejournal','/d',$drive)
    Invoke-Checked -FilePath fsutil.exe -ArgumentList @('usn','createjournal',$drive,'m=10485760','a=1048576')
    Set-Content -LiteralPath "$drive\seed\after-reset.txt" -Value 'after reset' -Encoding UTF8

    Write-Host '==> stale checkpoint must be rejected'
    $savedErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $syncOutput = @(& $cli 'sync' $drive $index 2>&1 | ForEach-Object { [string]$_ })
    $syncExitCode = $LASTEXITCODE
    $ErrorActionPreference = $savedErrorAction
    if ($syncExitCode -eq 0) {
        throw "Expected sync to reject reset journal, but it succeeded:`n$($syncOutput -join [Environment]::NewLine)"
    }
    if (-not ($syncOutput | Where-Object { $_ -match 'reset|truncation|rebuild' })) {
        throw "Sync failed, but reset/rebuild reason was not reported:`n$($syncOutput -join [Environment]::NewLine)"
    }

    Write-Host '==> full rebuild recovers cleanly'
    Invoke-Checked -FilePath $cli -ArgumentList @('index',$drive,$index)
    $found = @(& $cli 'search' $index 'after-reset' 32 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0 -or -not ($found | Where-Object { $_ -like '*after-reset.txt*' })) {
        throw "Rebuilt index did not contain after-reset sentinel:`n$($found -join [Environment]::NewLine)"
    }
    Invoke-Checked -FilePath $cli -ArgumentList @('verify-deep',$index)
    Write-Host 'Journal reset recovery: PASS'
} finally {
    if ($mounted) { & diskpart.exe /s $detach | Out-Null }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
