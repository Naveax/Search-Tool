[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Index,
    [int]$IdleSeconds = 30,
    [int]$QueryRounds = 40,
    [string]$OutputJson,
    [switch]$Enforce
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
function Resolve-ToolBinary([string]$Name) {
    $beside = Join-Path $PSScriptRoot $Name
    if (Test-Path -LiteralPath $beside) { return (Resolve-Path -LiteralPath $beside).Path }
    $repoBuild = Join-Path $root "target\release\$Name"
    if (Test-Path -LiteralPath $repoBuild) { return (Resolve-Path -LiteralPath $repoBuild).Path }
    throw "Missing release executable: $Name"
}
$cli = Resolve-ToolBinary 'search-tool.exe'
$bench = Resolve-ToolBinary 'search-tool-bench.exe'

$resolvedIndex = (Resolve-Path -LiteralPath $Index).Path
$item = Get-Item -LiteralPath $resolvedIndex
if ($item.PSIsContainer) {
    $measureIndex = Get-ChildItem -LiteralPath $resolvedIndex -File -Filter '*.stidx' | Sort-Object Name | Select-Object -First 1
    if (-not $measureIndex) { throw "No .stidx files found in $resolvedIndex" }
    $measureIndex = $measureIndex.FullName
} else {
    $measureIndex = $resolvedIndex
}
$logicalCpu = [Math]::Max(1, [Environment]::ProcessorCount)

function Get-ServiceProcess {
    try {
        $svc = Get-CimInstance Win32_Service -Filter "Name='SearchToolIndexer'" -ErrorAction Stop
        if ($svc -and [int]$svc.ProcessId -gt 0) {
            return Get-Process -Id ([int]$svc.ProcessId) -ErrorAction Stop
        }
    } catch {
        # Fall back to a console-service process when validating without SCM installation.
    }
    Get-Process -Name 'search-tool-service' -ErrorAction SilentlyContinue | Select-Object -First 1
}

function Get-TransferCounters([int]$ProcessId) {
    try {
        $p = Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId" -ErrorAction Stop
        return [pscustomobject]@{
            ReadBytes = [uint64]$p.ReadTransferCount
            WriteBytes = [uint64]$p.WriteTransferCount
        }
    } catch {
        return $null
    }
}

Write-Host '==> core read-side benchmark'
$coreLines = @(& $bench '--measure' $measureIndex 2>&1 | ForEach-Object { [string]$_ })
if ($LASTEXITCODE -ne 0) {
    $coreLines | ForEach-Object { Write-Host $_ }
    throw "Core benchmark failed: $LASTEXITCODE"
}
$core = @{}
foreach ($line in $coreLines) {
    if ($line -match '^([^=]+)=(.+)$') { $core[$matches[1]] = $matches[2] }
}

$service = Get-ServiceProcess
$idle = $null
if ($service) {
    Write-Host "==> service idle sampling ($IdleSeconds s)"
    $startProc = Get-Process -Id $service.Id
    $startCpu = $startProc.TotalProcessorTime.TotalSeconds
    $startIo = Get-TransferCounters $service.Id
    $workingSets = [System.Collections.Generic.List[long]]::new()
    $privateBytes = [System.Collections.Generic.List[long]]::new()
    $samples = [Math]::Max(1, $IdleSeconds)
    for ($i = 0; $i -lt $samples; $i++) {
        Start-Sleep -Seconds 1
        $p = Get-Process -Id $service.Id -ErrorAction Stop
        $workingSets.Add($p.WorkingSet64)
        $privateBytes.Add($p.PrivateMemorySize64)
    }
    $endProc = Get-Process -Id $service.Id
    $endCpu = $endProc.TotalProcessorTime.TotalSeconds
    $endIo = Get-TransferCounters $service.Id
    $cpuPercent = (($endCpu - $startCpu) / [Math]::Max(1, $IdleSeconds) / $logicalCpu) * 100.0
    $readBps = $null
    $writeBps = $null
    if ($startIo -and $endIo) {
        $readBps = ([double]($endIo.ReadBytes - $startIo.ReadBytes)) / [Math]::Max(1, $IdleSeconds)
        $writeBps = ([double]($endIo.WriteBytes - $startIo.WriteBytes)) / [Math]::Max(1, $IdleSeconds)
    }
    $idle = [ordered]@{
        pid = $service.Id
        seconds = $IdleSeconds
        cpu_percent = [Math]::Round($cpuPercent, 4)
        working_set_avg_mib = [Math]::Round((($workingSets | Measure-Object -Average).Average / 1MB), 3)
        working_set_peak_mib = [Math]::Round((($workingSets | Measure-Object -Maximum).Maximum / 1MB), 3)
        private_peak_mib = [Math]::Round((($privateBytes | Measure-Object -Maximum).Maximum / 1MB), 3)
        read_bytes_per_sec = if ($null -eq $readBps) { $null } else { [Math]::Round($readBps, 1) }
        write_bytes_per_sec = if ($null -eq $writeBps) { $null } else { [Math]::Round($writeBps, 1) }
    }
} else {
    Write-Warning 'search-tool-service is not running; idle service metrics will be omitted.'
}

Write-Host "==> end-to-end CLI query latency ($QueryRounds rounds)"
$queries = @('node','package','report_','project_')
$times = [System.Collections.Generic.List[double]]::new()
for ($i = 0; $i -lt [Math]::Max(1, $QueryRounds); $i++) {
    $q = $queries[$i % $queries.Count]
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & $cli 'search' $resolvedIndex $q 32 | Out-Null
    $exit = $LASTEXITCODE
    $sw.Stop()
    if ($exit -ne 0) { throw "Query failed ($exit): $q" }
    $times.Add($sw.Elapsed.TotalMilliseconds)
}
$sorted = @($times | Sort-Object)
$p95Index = [Math]::Min($sorted.Count - 1, [Math]::Floor(($sorted.Count - 1) * 0.95))
$queryStats = [ordered]@{
    rounds = $times.Count
    avg_ms = [Math]::Round((($times | Measure-Object -Average).Average), 3)
    p95_ms = [Math]::Round($sorted[$p95Index], 3)
    max_ms = [Math]::Round((($times | Measure-Object -Maximum).Maximum), 3)
}

$doctor = @(& $cli 'doctor' $resolvedIndex 2>&1 | ForEach-Object { [string]$_ })
$report = [ordered]@{
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    machine = [ordered]@{
        computer_name = $env:COMPUTERNAME
        logical_processors = $logicalCpu
        total_memory_mib = [Math]::Round(((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB), 1)
        os = (Get-CimInstance Win32_OperatingSystem).Caption
    }
    core = $core
    service_idle = $idle
    cli_query = $queryStats
    doctor = $doctor
}

$json = $report | ConvertTo-Json -Depth 6
$json
if ($OutputJson) {
    $parent = Split-Path -Parent $OutputJson
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
    Write-Host "Report=$OutputJson"
}

if ($Enforce) {
    if ($idle) {
        if ($idle.cpu_percent -gt 0.5) { throw "Idle CPU target exceeded: $($idle.cpu_percent)% > 0.5%" }
        if ($idle.working_set_peak_mib -gt 64) { throw "Service RAM target exceeded: $($idle.working_set_peak_mib) MiB > 64 MiB" }
    }
    if ($core['rss_after_queries_kib'] -match '^\d+$' -and [double]$core['rss_after_queries_kib'] -gt (64 * 1024)) {
        throw "Read-side RSS target exceeded: $($core['rss_after_queries_kib']) KiB > 65536 KiB"
    }
}
