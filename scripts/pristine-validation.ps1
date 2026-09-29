[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Package,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Pristine validation requires an elevated PowerShell session.'
    }
}

function Write-Result([string]$Result, [string]$Reason, [hashtable]$Extra = @{}) {
    $report = [ordered]@{
        schema = 1
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        computer_name = [Environment]::MachineName
        package = $resolvedPackage
        install_dir = $installDir
        data_dir = $dataDir
        result = $Result
        reason = $Reason
    }
    foreach ($key in $Extra.Keys) { $report[$key] = $Extra[$key] }
    $json = $report | ConvertTo-Json -Depth 7
    $json
    if ($OutputJson) {
        $parent = Split-Path -Parent $OutputJson
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
        Write-Host "Report=$OutputJson"
    }
}

Assert-Admin
$resolvedPackage = (Resolve-Path -LiteralPath $Package).Path
$installDir = [IO.Path]::Combine(
    [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles),
    'Search Tool'
)
$dataDir = [IO.Path]::Combine(
    [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonApplicationData),
    'SearchTool'
)
$startup = [Environment]::GetFolderPath([Environment+SpecialFolder]::Startup)
$shortcut = if ($startup) { Join-Path $startup 'Search Tool.lnk' } else { $null }

$preexisting = [ordered]@{
    service = [bool](Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue)
    install_dir = [bool](Test-Path -LiteralPath $installDir)
    data_dir = [bool](Test-Path -LiteralPath $dataDir)
    startup_shortcut = [bool]($shortcut -and (Test-Path -LiteralPath $shortcut))
}
if ($preexisting.service -or $preexisting.install_dir -or $preexisting.data_dir -or $preexisting.startup_shortcut) {
    Write-Result -Result 'BLOCKED' -Reason 'Host is not pristine: Search Tool state already exists.' -Extra @{ preexisting = $preexisting }
    throw 'Pristine validation refused: pre-existing Search Tool state detected.'
}

$baseTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$work = Join-Path $baseTemp ('search-tool-pristine-' + [Guid]::NewGuid().ToString('N'))
$portable = Join-Path $work 'portable'
$vhd = Join-Path $work 'pristine.vhdx'
New-Item -ItemType Directory -Force -Path $portable | Out-Null

$lettersInUse = @(Get-PSDrive -PSProvider FileSystem | ForEach-Object { $_.Name.ToUpperInvariant() })
$letter = @('L','K','J','I','H','G') | Where-Object { $lettersInUse -notcontains $_ } | Select-Object -First 1
if (-not $letter) { throw 'No free drive letter available for pristine validation.' }
$drive = $letter + ':'
$create = Join-Path $work 'create.txt'
$detach = Join-Path $work 'detach.txt'
@"
create vdisk file="$vhd" maximum=512 type=expandable
select vdisk file="$vhd"
attach vdisk
create partition primary
format fs=ntfs quick label=SearchToolPristine
assign letter=$letter
exit
"@ | Set-Content -LiteralPath $create -Encoding ASCII
@"
select vdisk file="$vhd"
detach vdisk
exit
"@ | Set-Content -LiteralPath $detach -Encoding ASCII

$mounted = $false
$installationStarted = $false
$started = Get-Date
try {
    Expand-Archive -LiteralPath $resolvedPackage -DestinationPath $portable -Force
    & (Join-Path $portable 'verify-package.ps1') -Package $portable
    if ($LASTEXITCODE -ne 0) { throw "Package verification failed: $LASTEXITCODE" }

    & diskpart.exe /s $create | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "diskpart failed: $LASTEXITCODE" }
    $mounted = $true
    & fsutil.exe usn createjournal m=10485760 a=1048576 $drive | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "USN journal creation failed: $LASTEXITCODE" }

    New-Item -ItemType Directory -Force -Path "$drive\payload" | Out-Null
    $marker = 'pristine-install-marker.txt'
    Set-Content -LiteralPath "$drive\payload\$marker" -Value 'Search Tool pristine validation marker' -Encoding UTF8

    $installationStarted = $true
    & (Join-Path $portable 'install.ps1') -Drive $drive -SourceDir $portable
    if ($LASTEXITCODE -ne 0) { throw "Install failed: $LASTEXITCODE" }

    $svc = Get-Service -Name SearchToolIndexer -ErrorAction Stop
    $deadline = (Get-Date).AddSeconds(30)
    do {
        $svc.Refresh()
        if ($svc.Status -eq 'Running') { break }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    if ($svc.Status -ne 'Running') { throw "SearchToolIndexer did not reach Running: $($svc.Status)" }

    $svcInfo = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction Stop
    if ($svcInfo.StartMode -ne 'Auto') { throw "SearchToolIndexer start mode is not Auto: $($svcInfo.StartMode)" }

    $cli = Join-Path $installDir 'search-tool.exe'
    $indexRoot = Join-Path $dataDir 'index'
    $index = Join-Path $indexRoot "$letter.stidx"
    if (-not (Test-Path -LiteralPath $index)) { throw "Initial index missing: $index" }

    $found = @(& $cli search $index 'pristine-install-marker' 32 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0 -or -not ($found | Where-Object { $_ -like "*$marker*" })) {
        throw "Installed CLI did not find pristine marker: $($found -join ' | ')"
    }

    & $cli smart $index 'node js ile alakalı her şeyi bul' | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Installed smart search failed: $LASTEXITCODE" }
    & $cli doctor $indexRoot | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Installed doctor failed: $LASTEXITCODE" }
    & (Join-Path $installDir 'search-tool-gui.exe') --smoke $indexRoot
    if ($LASTEXITCODE -ne 0) { throw "Installed GUI smoke failed: $LASTEXITCODE" }

    & (Join-Path $portable 'uninstall.ps1') -PurgeData
    if ($LASTEXITCODE -ne 0) { throw "Uninstall failed: $LASTEXITCODE" }
    $installationStarted = $false

    $deadline = (Get-Date).AddSeconds(15)
    do {
        if (-not (Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue)) { break }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)

    $post = [ordered]@{
        service_absent = -not [bool](Get-Service -Name SearchToolIndexer -ErrorAction SilentlyContinue)
        install_dir_absent = -not [bool](Test-Path -LiteralPath $installDir)
        data_dir_absent = -not [bool](Test-Path -LiteralPath $dataDir)
        startup_shortcut_absent = -not [bool]($shortcut -and (Test-Path -LiteralPath $shortcut))
    }
    if (-not $post.service_absent -or -not $post.install_dir_absent -or -not $post.data_dir_absent -or -not $post.startup_shortcut_absent) {
        throw "Post-uninstall pristine cleanup failed: $($post | ConvertTo-Json -Compress)"
    }

    $elapsed = [Math]::Round(((Get-Date) - $started).TotalSeconds, 2)
    Write-Result -Result 'PASS' -Reason $null -Extra @{
        drive = $drive
        marker = $marker
        service_start_mode = [string]$svcInfo.StartMode
        elapsed_seconds = $elapsed
        post_uninstall = $post
    }
    Write-Host 'PRISTINE_VALIDATION=PASS'
} catch {
    if ($_.Exception.Message -notlike 'Pristine validation refused:*') {
        Write-Result -Result 'FAIL' -Reason $_.Exception.Message -Extra @{ drive = $drive }
    }
    throw
} finally {
    if ($installationStarted -and (Test-Path -LiteralPath (Join-Path $portable 'uninstall.ps1'))) {
        try { & (Join-Path $portable 'uninstall.ps1') -PurgeData 2>$null | Out-Null } catch {}
    }
    if ($mounted) { & diskpart.exe /s $detach | Out-Null }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
