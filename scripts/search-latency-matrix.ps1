[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Index,
    [ValidateRange(1, 100)] [int]$WarmupRounds = 5,
    [ValidateRange(10, 5000)] [int]$Rounds = 100,
    [ValidateRange(1, 100000000)] [int]$MinRecords = 1000000,
    [ValidateRange(0, 1800)] [int]$DoctorRetrySeconds = 300,
    [string]$OutputJson,
    [string]$ExactQuery = 'search-tool.exe',
    [string]$PrefixQuery = 'search-tool',
    [string]$FuzzyQuery = 'searh-tool',
    [string]$FilteredQuery = 'node ext:exe',
    [string]$RelationshipQuery = 'node js',
    [string]$ContentQuery = 'Search Tool',
    [switch]$AllowStaleContent,
    [switch]$AllowBackgroundMaintenance
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cli = Join-Path $root 'target\release\search-tool.exe'
if (-not (Test-Path -LiteralPath $cli)) {
    throw "Missing release CLI: $cli"
}

$resolvedIndex = (Resolve-Path -LiteralPath $Index).Path
if ((Get-Item -LiteralPath $resolvedIndex).PSIsContainer) {
    throw 'Latency matrix requires one concrete .stidx file, not an index directory.'
}

function Invoke-CliLines {
    param([Parameter(Mandatory)] [string[]]$CommandArgs)
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $lines = @(& $cli @CommandArgs | ForEach-Object { [string]$_ })
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    if ($code -ne 0) {
        throw "search-tool $($CommandArgs -join ' ') failed with exit code $code"
    }
    return $lines
}

function Invoke-CliQuiet {
    param([Parameter(Mandatory)] [string[]]$CommandArgs)
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $cli @CommandArgs *> $null
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    if ($code -ne 0) {
        throw "search-tool $($CommandArgs -join ' ') failed with exit code $code"
    }
}

function Invoke-DoctorLines {
    $deadline = (Get-Date).AddSeconds($DoctorRetrySeconds)
    do {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $lines = @(& $cli 'doctor' $resolvedIndex 2>&1 | ForEach-Object { [string]$_ })
            $code = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldPreference
        }

        if ($code -eq 0) {
            return $lines
        }

        $busy = [bool]($lines | Where-Object { $_ -match 'index mutation is already in progress' })
        if (-not $busy -or (Get-Date) -ge $deadline) {
            throw "search-tool doctor $resolvedIndex failed with exit code ${code}: $($lines -join ' | ')"
        }
        Start-Sleep -Milliseconds 500
    } while ($true)
}

function Get-Percentile {
    param(
        [Parameter(Mandatory)] [double[]]$Sorted,
        [Parameter(Mandatory)] [ValidateRange(0.0, 1.0)] [double]$P
    )
    if ($Sorted.Count -eq 0) { return $null }
    $index = [Math]::Ceiling($P * $Sorted.Count) - 1
    $index = [Math]::Max(0, [Math]::Min($Sorted.Count - 1, $index))
    return [double]$Sorted[$index]
}

function Measure-Case {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [string]$Command,
        [Parameter(Mandatory)] [string]$Query
    )

    $args = @($Command, $resolvedIndex, $Query)
    $probe = @(Invoke-CliLines -CommandArgs $args)

    for ($i = 0; $i -lt $WarmupRounds; $i++) {
        Invoke-CliQuiet -CommandArgs $args
    }

    $samples = [System.Collections.Generic.List[double]]::new()
    for ($i = 0; $i -lt $Rounds; $i++) {
        $sw = [Diagnostics.Stopwatch]::StartNew()
        Invoke-CliQuiet -CommandArgs $args
        $sw.Stop()
        $samples.Add($sw.Elapsed.TotalMilliseconds)
    }

    $sorted = [double[]]@($samples | Sort-Object)
    return [ordered]@{
        name = $Name
        command = $Command
        query = $Query
        probe_hits = $probe.Count
        warmup_rounds = $WarmupRounds
        rounds = $Rounds
        avg_ms = [Math]::Round((($samples | Measure-Object -Average).Average), 3)
        min_ms = [Math]::Round($sorted[0], 3)
        p50_ms = [Math]::Round((Get-Percentile -Sorted $sorted -P 0.50), 3)
        p95_ms = [Math]::Round((Get-Percentile -Sorted $sorted -P 0.95), 3)
        p99_ms = [Math]::Round((Get-Percentile -Sorted $sorted -P 0.99), 3)
        max_ms = [Math]::Round($sorted[-1], 3)
        samples_ms = @($samples | ForEach-Object { [Math]::Round($_, 3) })
    }
}

$doctor = @(Invoke-DoctorLines)
$doctorMap = @{}
foreach ($line in $doctor) {
    if ($line -match '^([^=]+)=(.*)$') {
        $doctorMap[$matches[1]] = $matches[2]
    }
}

if (-not $doctorMap.ContainsKey('base_records')) {
    throw 'doctor output did not include base_records'
}
$baseRecords = [uint64]$doctorMap['base_records']
if ($baseRecords -lt [uint64]$MinRecords) {
    throw "Index is too small for the requested matrix: $baseRecords < $MinRecords records"
}
if (-not $AllowStaleContent -and $doctorMap['content_fresh'] -ne 'true') {
    throw "Content sidecar is not fresh (content_fresh=$($doctorMap['content_fresh'])); wait for/rebuild content before measuring."
}
if (-not $AllowBackgroundMaintenance -and
    $doctorMap.ContainsKey('service_maintenance') -and
    $doctorMap['service_maintenance'] -ne 'None') {
    throw "Background maintenance is active ($($doctorMap['service_maintenance'])); wait for an idle service before measuring."
}

$sourceHead = $null
try {
    $sourceHead = (& git -C $root rev-parse HEAD 2>$null | Select-Object -First 1)
} catch {
    $sourceHead = $null
}

$cases = @(
    @{ Name = 'exact'; Command = 'search'; Query = $ExactQuery },
    @{ Name = 'prefix'; Command = 'search'; Query = $PrefixQuery },
    @{ Name = 'fuzzy'; Command = 'fuzzy'; Query = $FuzzyQuery },
    @{ Name = 'filtered'; Command = 'search'; Query = $FilteredQuery },
    @{ Name = 'relationship'; Command = 'related'; Query = $RelationshipQuery },
    @{ Name = 'content'; Command = 'content-search'; Query = $ContentQuery }
)

$results = [System.Collections.Generic.List[object]]::new()
foreach ($case in $cases) {
    Write-Host "==> $($case.Name): $($case.Command) '$($case.Query)'"
    $results.Add((Measure-Case -Name $case.Name -Command $case.Command -Query $case.Query))
}

$report = [ordered]@{
    schema = 1
    generated_utc = [DateTime]::UtcNow.ToString('o')
    source_head = $sourceHead
    index = $resolvedIndex
    base_records = $baseRecords
    warmup_rounds = $WarmupRounds
    rounds = $Rounds
    doctor = $doctor
    machine = [ordered]@{
        computer_name = $env:COMPUTERNAME
        logical_processors = [Environment]::ProcessorCount
        total_memory_mib = [Math]::Round(((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB), 1)
        os = (Get-CimInstance Win32_OperatingSystem).Caption
    }
    results = @($results)
}

$report.results |
    Select-Object name, query, probe_hits, avg_ms, p50_ms, p95_ms, p99_ms, max_ms |
    Format-Table -AutoSize

$json = $report | ConvertTo-Json -Depth 8
if ($OutputJson) {
    $parent = Split-Path -Parent $OutputJson
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
    Write-Host "Report=$OutputJson"
} else {
    $json
}
