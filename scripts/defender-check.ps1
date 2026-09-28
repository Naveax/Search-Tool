[CmdletBinding()]
param(
    [string]$Path = $PSScriptRoot,
    [switch]$CustomScan,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$resolved = (Resolve-Path -LiteralPath $Path).Path

if (-not (Get-Command Get-MpComputerStatus -ErrorAction SilentlyContinue)) {
    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        path = $resolved
        result = 'UNAVAILABLE'
        reason = 'Microsoft Defender cmdlets are not available on this machine.'
    }
    $json = $report | ConvertTo-Json -Depth 3
    $json
    if ($OutputJson) { $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8 }
    return
}

$status = Get-MpComputerStatus
if (-not $status.AMServiceEnabled -or -not $status.AntivirusEnabled) {
    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        path = $resolved
        result = 'UNAVAILABLE'
        reason = 'Microsoft Defender antivirus service is disabled on this machine.'
        antivirus_enabled = [bool]$status.AntivirusEnabled
        realtime_protection_enabled = [bool]$status.RealTimeProtectionEnabled
    }
    $json = $report | ConvertTo-Json -Depth 3
    $json
    if ($OutputJson) { $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8 }
    return
}
$preference = Get-MpPreference
$excluded = @($preference.ExclusionPath | Where-Object {
    $_ -and ($resolved.StartsWith($_, [StringComparison]::OrdinalIgnoreCase) -or $_.StartsWith($resolved, [StringComparison]::OrdinalIgnoreCase))
})
if ($excluded.Count -gt 0) {
    Write-Warning "Search Tool path overlaps a Defender exclusion. AV-impact results are not trustworthy: $($excluded -join ', ')"
}

$before = @(Get-MpThreatDetection -ErrorAction SilentlyContinue)
$scanMs = $null
if ($CustomScan) {
    Write-Host "==> Microsoft Defender custom scan: $resolved"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-MpScan -ScanType CustomScan -ScanPath $resolved
    $sw.Stop()
    $scanMs = [Math]::Round($sw.Elapsed.TotalMilliseconds, 1)
}
$after = @(Get-MpThreatDetection -ErrorAction SilentlyContinue)
$new = @($after | Where-Object {
    $candidate = $_
    -not ($before | Where-Object { $_.ThreatID -eq $candidate.ThreatID -and $_.InitialDetectionTime -eq $candidate.InitialDetectionTime })
})
$related = @($new | Where-Object {
    ($_.Resources | Out-String) -like "*$resolved*" -or ($_.ProcessName -like '*search-tool*')
})

$report = [ordered]@{
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    path = $resolved
    antivirus_enabled = [bool]$status.AntivirusEnabled
    realtime_protection_enabled = [bool]$status.RealTimeProtectionEnabled
    behavior_monitor_enabled = [bool]$status.BehaviorMonitorEnabled
    signature_version = [string]$status.AntivirusSignatureVersion
    signature_last_updated = if ($status.AntivirusSignatureLastUpdated) { $status.AntivirusSignatureLastUpdated.ToUniversalTime().ToString('o') } else { $null }
    overlapping_exclusions = $excluded
    custom_scan_requested = [bool]$CustomScan
    custom_scan_ms = $scanMs
    new_related_detections = $related.Count
    result = if ($related.Count -eq 0) { 'PASS' } else { 'DETECTION' }
}
$json = $report | ConvertTo-Json -Depth 5
$json
if ($OutputJson) {
    $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
    Write-Host "Report=$OutputJson"
}
if ($related.Count -gt 0) { throw 'Microsoft Defender reported a Search Tool-related detection.' }
