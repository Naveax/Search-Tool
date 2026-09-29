[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Package,
    [string]$ServiceName = 'SearchToolIndexerInstallSmoke'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($ServiceName) -or
    $ServiceName.Length -gt 256 -or
    $ServiceName -match '[\\/"]') {
    throw "Invalid Windows service name: '$ServiceName'"
}

$resolvedPackage = (Resolve-Path -LiteralPath $Package).Path
$baseTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$work = Join-Path $baseTemp ('search-tool-install-smoke-' + [Guid]::NewGuid().ToString('N'))
$portable = Join-Path $work 'portable'
$installDir = Join-Path $work 'Program Files\Search Tool Installed'
$dataDir = Join-Path $work 'ProgramData\SearchTool'
$vhd = Join-Path $work 'install-smoke.vhdx'
New-Item -ItemType Directory -Force -Path $portable | Out-Null

$lettersInUse = @(Get-PSDrive -PSProvider FileSystem | ForEach-Object { $_.Name.ToUpperInvariant() })
$letter = @('L','K','J','I','H') | Where-Object { $lettersInUse -notcontains $_ } | Select-Object -First 1
if (-not $letter) { throw 'No free drive letter available for installer smoke test.' }
$drive = "${letter}:"
$create = Join-Path $work 'create.txt'
$detach = Join-Path $work 'detach.txt'
@"
create vdisk file="$vhd" maximum=256 type=expandable
select vdisk file="$vhd"
attach vdisk
create partition primary
format fs=ntfs quick label=SearchToolInstallCI
assign letter=$letter
exit
"@ | Set-Content -LiteralPath $create -Encoding ASCII
@"
select vdisk file="$vhd"
detach vdisk
exit
"@ | Set-Content -LiteralPath $detach -Encoding ASCII

$mounted = $false
$installed = $false
try {
    Expand-Archive -LiteralPath $resolvedPackage -DestinationPath $portable -Force
    Write-Host '==> create isolated install-test NTFS VHD'
    & diskpart.exe /s $create
    if ($LASTEXITCODE -ne 0) { throw "diskpart failed: $LASTEXITCODE" }
    $mounted = $true
    & fsutil.exe usn createjournal m=10485760 a=1048576 $drive | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "USN journal creation failed: $LASTEXITCODE" }

    New-Item -ItemType Directory -Force -Path "$drive\payload" | Out-Null
    Set-Content -LiteralPath "$drive\payload\portable-install-marker.txt" -Value 'portable install marker' -Encoding UTF8

    Write-Host '==> install from extracted portable package'
    & (Join-Path $portable 'install.ps1') -Drive $drive -SourceDir $portable -InstallDir $installDir -DataDir $dataDir -ServiceName $ServiceName
    $installed = $true

    $service = Get-Service -Name $ServiceName -ErrorAction Stop
    $deadline = (Get-Date).AddSeconds(20)
    while ($service.Status -ne 'Running' -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 250
        $service.Refresh()
    }
    if ($service.Status -ne 'Running') { throw "$ServiceName did not reach Running state: $($service.Status)" }

    $cli = Join-Path $installDir 'search-tool.exe'
    $index = Join-Path $dataDir "index\$letter.stidx"
    if (-not (Test-Path -LiteralPath $index)) { throw "Installer did not create index: $index" }
    $result = @(& $cli search $index 'portable-install-marker' 32 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0 -or -not ($result | Where-Object { $_ -like '*portable-install-marker.txt*' })) {
        throw "Installed CLI could not find seeded file:`n$($result -join [Environment]::NewLine)"
    }

    $smart = @(& $cli smart $index 'node js ile alakalı her şeyi bul' 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0) {
        throw "Installed smart search/model resolution failed:`n$($smart -join [Environment]::NewLine)"
    }
    & $cli doctor (Join-Path $dataDir 'index') | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Installed doctor failed: $LASTEXITCODE" }

    Write-Host '==> installed native GUI smoke'
    & (Join-Path $installDir 'search-tool-gui.exe') --smoke (Join-Path $dataDir 'index')
    if ($LASTEXITCODE -ne 0) { throw "Installed GUI smoke failed: $LASTEXITCODE" }

    Write-Host '==> uninstall and purge test data'
    & (Join-Path $portable 'uninstall.ps1') -InstallDir $installDir -DataDir $dataDir -ServiceName $ServiceName -PurgeData
    $installed = $false
    $deadline = (Get-Date).AddSeconds(10)
    do {
        Start-Sleep -Milliseconds 250
        $remainingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        if (-not $remainingService) { break }
    } while ((Get-Date) -lt $deadline)
    if ($remainingService) { throw "$ServiceName still exists after uninstall: $($remainingService.Status)" }
    if (Test-Path -LiteralPath $installDir) { throw 'InstallDir still exists after uninstall.' }
    if (Test-Path -LiteralPath $dataDir) { throw 'DataDir still exists after -PurgeData uninstall.' }

    Write-Host 'INSTALL_SMOKE=PASS'
} finally {
    if ($installed -and (Test-Path -LiteralPath (Join-Path $portable 'uninstall.ps1'))) {
        & (Join-Path $portable 'uninstall.ps1') -InstallDir $installDir -DataDir $dataDir -ServiceName $ServiceName -PurgeData 2>$null | Out-Null
    }
    if ($mounted) { & diskpart.exe /s $detach | Out-Null }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
