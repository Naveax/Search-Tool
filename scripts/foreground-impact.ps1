[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidatePattern('^[A-Za-z]:$')] [string]$Drive,
    [Parameter(Mandatory)] [string]$Index,
    [ValidateRange(5, 120)] [int]$BaselineSeconds = 10,
    [ValidateRange(5, 120)] [int]$StressSeconds = 20,
    [switch]$Enforce,
    [string]$OutputJson
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

function Measure-SearchWindow([string]$Cli, [string]$IndexPath, [int]$Seconds) {
    $queries = @('node','package','config','project','report','cache')
    $samples = [System.Collections.Generic.List[double]]::new()
    $deadline = (Get-Date).AddSeconds($Seconds)
    $i = 0
    while ((Get-Date) -lt $deadline) {
        $query = $queries[$i % $queries.Count]
        $sw = [Diagnostics.Stopwatch]::StartNew()
        & $Cli search $IndexPath $query 32 | Out-Null
        $exit = $LASTEXITCODE
        $sw.Stop()
        if ($exit -ne 0) { throw "Search failed ($exit): $query" }
        $samples.Add($sw.Elapsed.TotalMilliseconds)
        $i++
    }
    if ($samples.Count -eq 0) { throw 'No query samples were collected.' }
    $sorted = @($samples | Sort-Object)
    $p95Index = [Math]::Min($sorted.Count - 1, [Math]::Floor(($sorted.Count - 1) * 0.95))
    return [ordered]@{
        samples = $samples.Count
        avg_ms = [Math]::Round((($samples | Measure-Object -Average).Average), 3)
        p95_ms = [Math]::Round($sorted[$p95Index], 3)
        max_ms = [Math]::Round((($samples | Measure-Object -Maximum).Maximum), 3)
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

Write-Host "==> baseline foreground search ($BaselineSeconds s)"
$baseline = Measure-SearchWindow $cli $resolvedIndex $BaselineSeconds

$tempDir = Join-Path ([IO.Path]::GetTempPath()) ('search-tool-impact-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
$soakReport = Join-Path $tempDir 'soak.json'
$soakScript = Join-Path $PSScriptRoot 'windows-soak.ps1'
$job = $null
try {
    Write-Host '==> starting concurrent create/rename/delete soak'
    $job = Start-Job -ScriptBlock {
        param($Script, $DriveArg, $IndexArg, $Report)
        & $Script -Drive $DriveArg -Index $IndexArg -DurationMinutes 1 -BatchSize 16 -PollTimeoutSeconds 30 -OutputJson $Report
        if ($LASTEXITCODE -ne 0) { throw "soak failed: $LASTEXITCODE" }
    } -ArgumentList $soakScript, $Drive, $volumeIndex, $soakReport

    Start-Sleep -Seconds 3
    if ($job.State -eq 'Failed') {
        Receive-Job $job | Out-Host
        throw 'Concurrent soak job failed before foreground measurement.'
    }

    Write-Host "==> foreground search under background mutation ($StressSeconds s)"
    $stressed = Measure-SearchWindow $cli $resolvedIndex $StressSeconds

    Write-Host '==> waiting for background soak cleanup'
    Wait-Job $job -Timeout 90 | Out-Null
    Receive-Job $job | Out-Host
    if ($job.State -ne 'Completed') { throw "Concurrent soak did not complete cleanly: $($job.State)" }

    $ratio = if ($baseline.p95_ms -gt 0) { [double]$stressed.p95_ms / [double]$baseline.p95_ms } else { $null }
    $delta = [double]$stressed.p95_ms - [double]$baseline.p95_ms
    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        drive = $Drive
        index = $resolvedIndex
        baseline = $baseline
        stressed = $stressed
        p95_increase_ms = [Math]::Round($delta, 3)
        p95_ratio = if ($null -eq $ratio) { $null } else { [Math]::Round($ratio, 3) }
        soak = if (Test-Path -LiteralPath $soakReport) { Get-Content -LiteralPath $soakReport -Raw | ConvertFrom-Json } else { $null }
        result = 'PASS'
    }

    $json = $report | ConvertTo-Json -Depth 7
    $json
    if ($OutputJson) {
        $parent = Split-Path -Parent $OutputJson
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
        Write-Host "Report=$OutputJson"
    }

    if ($Enforce) {
        $allowedP95 = [Math]::Max(([double]$baseline.p95_ms * 3.0), ([double]$baseline.p95_ms + 50.0))
        if ([double]$stressed.p95_ms -gt $allowedP95) {
            throw "Foreground p95 degradation exceeded target: stressed=$($stressed.p95_ms) ms allowed=$([Math]::Round($allowedP95,3)) ms baseline=$($baseline.p95_ms) ms"
        }
    }
    Write-Host 'FOREGROUND_IMPACT=PASS'
} finally {
    if ($job) {
        if ($job.State -eq 'Running') { Stop-Job $job -ErrorAction SilentlyContinue }
        Remove-Job $job -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
