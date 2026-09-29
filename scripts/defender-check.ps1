[CmdletBinding()]
param(
    [string]$Path = $PSScriptRoot,
    [switch]$CustomScan,
    [switch]$Enforce,
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
    if ($Enforce) { throw 'Required protection tooling is unavailable.' }
    return
}

$status = Get-MpComputerStatus
$activeProtection = [bool]$status.AMServiceEnabled -and
    [bool]$status.AntivirusEnabled -and
    [bool]$status.RealTimeProtectionEnabled -and
    [bool]$status.BehaviorMonitorEnabled -and
    [bool]$status.AntispywareEnabled
if (-not $activeProtection) {
    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        path = $resolved
        result = 'UNAVAILABLE'
        reason = 'Microsoft Defender active-protection requirements are not satisfied on this machine.'
        am_service_enabled = [bool]$status.AMServiceEnabled
        antivirus_enabled = [bool]$status.AntivirusEnabled
        realtime_protection_enabled = [bool]$status.RealTimeProtectionEnabled
        behavior_monitor_enabled = [bool]$status.BehaviorMonitorEnabled
        antispyware_enabled = [bool]$status.AntispywareEnabled
    }
    $json = $report | ConvertTo-Json -Depth 3
    $json
    if ($OutputJson) { $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8 }
    if ($Enforce) { throw 'Active protection is required for enforced validation.' }
    return
}
$preference = Get-MpPreference
$excluded = @($preference.ExclusionPath | Where-Object {
    $_ -and ($resolved.StartsWith($_, [StringComparison]::OrdinalIgnoreCase) -or $_.StartsWith($resolved, [StringComparison]::OrdinalIgnoreCase))
})
if ($excluded.Count -gt 0) {
    Write-Warning "Search Tool path overlaps a Defender exclusion. AV-impact results are not trustworthy: $($excluded -join ', ')"
    if ($Enforce) {
        $report = [ordered]@{
            timestamp_utc = [DateTime]::UtcNow.ToString('o')
            path = $resolved
            result = 'EXCLUDED'
            reason = 'The validation path overlaps a protection exclusion.'
            overlapping_exclusions = $excluded
            custom_scan_requested = [bool]$CustomScan
        }
        $json = $report | ConvertTo-Json -Depth 5
        $json
        if ($OutputJson) { $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8 }
        throw 'Protection exclusion overlaps the validation path; enforced evidence is invalid.'
    }
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
    am_service_enabled = [bool]$status.AMServiceEnabled
    antivirus_enabled = [bool]$status.AntivirusEnabled
    realtime_protection_enabled = [bool]$status.RealTimeProtectionEnabled
    behavior_monitor_enabled = [bool]$status.BehaviorMonitorEnabled
    antispyware_enabled = [bool]$status.AntispywareEnabled
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
