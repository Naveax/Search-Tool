[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidatePattern('^[A-Za-z]:$')] [string]$Drive,
    [Parameter(Mandatory)] [string]$Index,
    [ValidateRange(1, 1440)] [int]$SoakMinutes = 30,
    [ValidateRange(10, 600)] [int]$IdleSeconds = 60,
    [switch]$DefenderScan,
    [switch]$EnforceTargets,
    [string]$OutputDir = (Join-Path $PSScriptRoot 'validation-results')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$benchReport = Join-Path $OutputDir "low-end-$stamp.json"
$soakReport = Join-Path $OutputDir "soak-$stamp.json"
$defenderReport = Join-Path $OutputDir "defender-$stamp.json"
$impactReport = Join-Path $OutputDir "foreground-impact-$stamp.json"
$summaryReport = Join-Path $OutputDir "physical-summary-$stamp.json"

function Resolve-ToolBinary([string]$Name) {
    $beside = Join-Path $PSScriptRoot $Name
    if (Test-Path -LiteralPath $beside) { return (Resolve-Path -LiteralPath $beside).Path }
    $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
    $repoBuild = Join-Path $repoRoot "target\release\$Name"
    if (Test-Path -LiteralPath $repoBuild) { return (Resolve-Path -LiteralPath $repoBuild).Path }
    throw "Missing release executable: $Name"
}

function Read-JsonReport([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { throw "Expected report was not created: $Path" }
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Get-DriveProfile([string]$DriveValue) {
    $letter = $DriveValue.Substring(0, 1).ToUpperInvariant()
    $disk = $null
    $physical = $null
    try {
        $partition = Get-Partition -DriveLetter $letter -ErrorAction Stop
        $disk = $partition | Get-Disk -ErrorAction Stop
        try {
            $physical = Get-PhysicalDisk -ErrorAction Stop |
                Where-Object { [string]$_.DeviceId -eq [string]$disk.Number } |
                Select-Object -First 1
        } catch {
            $physical = $null
        }
    } catch {
        return [ordered]@{
            drive = "$letter`:"
            detected = $false
            error = $_.Exception.Message
            is_hdd = $null
        }
    }

    $mediaType = if ($physical) { [string]$physical.MediaType } else { 'Unknown' }
    $spindle = if ($physical -and $null -ne $physical.SpindleSpeed) { [int64]$physical.SpindleSpeed } else { $null }
    $isHdd = $null
    if ($mediaType -match 'HDD') {
        $isHdd = $true
    } elseif ($mediaType -match 'SSD|SCM') {
        $isHdd = $false
    } elseif ($null -ne $spindle -and $spindle -gt 0) {
        $isHdd = $true
    }

    return [ordered]@{
        drive = "$letter`:"
        detected = $true
        disk_number = [int]$disk.Number
        friendly_name = [string]$disk.FriendlyName
        bus_type = [string]$disk.BusType
        partition_style = [string]$disk.PartitionStyle
        size_gib = [Math]::Round(([double]$disk.Size / 1GB), 2)
        media_type = $mediaType
        spindle_speed_rpm = $spindle
        is_hdd = $isHdd
    }
}

$cli = Resolve-ToolBinary 'search-tool.exe'
$resolvedIndex = (Resolve-Path -LiteralPath $Index).Path
$indexItem = Get-Item -LiteralPath $resolvedIndex
if ($indexItem.PSIsContainer) {
    $letter = $Drive.Substring(0, 1).ToUpperInvariant()
    $volumeIndex = Join-Path $resolvedIndex "$letter.stidx"
    if (-not (Test-Path -LiteralPath $volumeIndex)) { throw "Missing volume index: $volumeIndex" }
} else {
    $volumeIndex = $resolvedIndex
}
$binaryDir = Split-Path -Parent $cli

$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$computer = Get-CimInstance Win32_ComputerSystem
$os = Get-CimInstance Win32_OperatingSystem
$driveProfile = Get-DriveProfile $Drive
$totalMemoryMiB = [Math]::Round(([double]$computer.TotalPhysicalMemory / 1MB), 1)
$hardwareProfile = [ordered]@{
    cpu = [string]$cpu.Name
    logical_processors = [int]$computer.NumberOfLogicalProcessors
    total_memory_mib = $totalMemoryMiB
    drive = $driveProfile
    os = [string]$os.Caption
    os_build = [string]$os.BuildNumber
}

Write-Host '==> Search Tool doctor (before)'
$doctorBefore = @(& $cli doctor $resolvedIndex 2>&1 | ForEach-Object { [string]$_ })
if ($LASTEXITCODE -ne 0) {
    $doctorBefore | ForEach-Object { Write-Host $_ }
    throw 'doctor failed before validation'
}

Write-Host '==> low-end idle/query benchmark'
$benchmarkArgs = @{
    Index = $resolvedIndex
    IdleSeconds = $IdleSeconds
    QueryRounds = 100
    OutputJson = $benchReport
}
if ($EnforceTargets) { $benchmarkArgs.Enforce = $true }
& (Join-Path $PSScriptRoot 'low-end-benchmark.ps1') @benchmarkArgs
if ($LASTEXITCODE -ne 0) { throw "low-end benchmark failed: $LASTEXITCODE" }

Write-Host '==> foreground-impact benchmark'
$impactArgs = @{
    Drive = $Drive
    Index = $resolvedIndex
    BaselineSeconds = 10
    StressSeconds = 20
    OutputJson = $impactReport
}
if ($EnforceTargets) { $impactArgs.Enforce = $true }
& (Join-Path $PSScriptRoot 'foreground-impact.ps1') @impactArgs
if ($LASTEXITCODE -ne 0) { throw "foreground-impact benchmark failed: $LASTEXITCODE" }

Write-Host "==> service soak ($SoakMinutes minutes)"
& (Join-Path $PSScriptRoot 'windows-soak.ps1') -Drive $Drive -Index $volumeIndex -DurationMinutes $SoakMinutes -BatchSize 32 -CrashRestartService -OutputJson $soakReport
if ($LASTEXITCODE -ne 0) { throw "soak failed: $LASTEXITCODE" }

Write-Host '==> Search Tool doctor (after soak/crash recovery)'
$doctorAfter = @(& $cli doctor $resolvedIndex 2>&1 | ForEach-Object { [string]$_ })
if ($LASTEXITCODE -ne 0) {
    $doctorAfter | ForEach-Object { Write-Host $_ }
    throw 'doctor failed after soak/crash recovery'
}

Write-Host '==> Defender interaction check'
$defenderArgs = @{
    Path = $binaryDir
    OutputJson = $defenderReport
}
if ($DefenderScan) {
    $defenderArgs.CustomScan = $true
    $defenderArgs.Enforce = $true
}
& (Join-Path $PSScriptRoot 'defender-check.ps1') @defenderArgs
if ($LASTEXITCODE -ne 0) { throw "Defender check failed: $LASTEXITCODE" }

$bench = Read-JsonReport $benchReport
$soak = Read-JsonReport $soakReport
$impact = Read-JsonReport $impactReport
$defender = Read-JsonReport $defenderReport
$defenderAccepted = if ($DefenderScan) {
    $defender.result -eq 'PASS'
} else {
    $defender.result -in @('PASS', 'UNAVAILABLE')
}
$overall = if ($soak.result -eq 'PASS' -and $defenderAccepted) { 'PASS' } else { 'FAIL' }

$summary = [ordered]@{
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    result = $overall
    hardware = [ordered]@{
        computer_name = $env:COMPUTERNAME
        cpu_name = [string]$cpu.Name
        physical_cores = [int]$cpu.NumberOfCores
        logical_processors = [int]$cpu.NumberOfLogicalProcessors
        total_memory_mib = $totalMemoryMiB
        os = [string]$os.Caption
        os_version = [string]$os.Version
        target_drive = $driveProfile
        hardware_profile = $hardwareProfile
    }
    low_end = $bench
    soak = $soak
    foreground_impact = $impact
    defender = $defender
    doctor_before = $doctorBefore
    doctor_after = $doctorAfter
    source_reports = [ordered]@{
        low_end = $benchReport
        soak = $soakReport
        foreground_impact = $impactReport
        defender = $defenderReport
    }
}
$summary | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $summaryReport -Encoding UTF8

if ($overall -ne 'PASS') { throw "Physical validation failed. Summary=$summaryReport" }
Write-Host 'PHYSICAL_VALIDATION=PASS'
Write-Host "Summary=$summaryReport"
Write-Host "Results=$OutputDir"
