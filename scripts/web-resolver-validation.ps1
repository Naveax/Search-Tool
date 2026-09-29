[CmdletBinding()]
param(
    [string]$Cli = (Join-Path $PSScriptRoot '..\target\release\search-tool.exe'),
    [string]$LookupTarget = 'C:\Users\SearchToolPrivacyProbe\Desktop\powershell.exe',
    [switch]$Enforce,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Save-Report([hashtable]$Report) {
    $json = $Report | ConvertTo-Json -Depth 7
    $json
    if ($OutputJson) {
        $parent = Split-Path -Parent $OutputJson
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
        Write-Host "Report=$OutputJson"
    }
}

$resolvedCli = (Resolve-Path -LiteralPath $Cli).Path
$keyPresent = -not [string]::IsNullOrWhiteSpace($env:SEARCH_TOOL_GOOGLE_KEY)
$cxPresent = -not [string]::IsNullOrWhiteSpace($env:SEARCH_TOOL_GOOGLE_CX)
if (-not $keyPresent -or -not $cxPresent) {
    $report = [ordered]@{
        schema = 1
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        computer_name = [Environment]::MachineName
        cli = $resolvedCli
        lookup_target_filename = [IO.Path]::GetFileName($LookupTarget)
        credentials = [ordered]@{
            google_key_present = [bool]$keyPresent
            google_cx_present = [bool]$cxPresent
        }
        result = 'BLOCKED'
        reason = 'SEARCH_TOOL_GOOGLE_KEY and SEARCH_TOOL_GOOGLE_CX are required for a real provider validation.'
    }
    Save-Report $report
    if ($Enforce) { throw $report.reason }
    return
}

$work = Join-Path ([IO.Path]::GetTempPath()) ('search-tool-web-validation-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
$index = Join-Path $work 'probe.stidx'
$cache = "$index.webcache"
$privacyMarker = 'SearchToolPrivacyProbe'
$savedKey = $env:SEARCH_TOOL_GOOGLE_KEY
$savedCx = $env:SEARCH_TOOL_GOOGLE_CX
try {
    Remove-Item -LiteralPath $cache -Force -ErrorAction SilentlyContinue

    $firstOutput = @(& $resolvedCli web-lookup $index $LookupTarget 2>&1 | ForEach-Object { [string]$_ })
    $firstExit = $LASTEXITCODE
    $firstSourceWeb = [bool]($firstOutput | Where-Object { $_ -eq 'source=web' })
    $firstLeakedParent = (($firstOutput -join [Environment]::NewLine).Contains($privacyMarker))

    $cacheExists = Test-Path -LiteralPath $cache
    $cacheBytes = if ($cacheExists) { (Get-Item -LiteralPath $cache).Length } else { 0 }
    $cacheSha256 = if ($cacheExists) { (Get-FileHash -LiteralPath $cache -Algorithm SHA256).Hash } else { $null }
    $cacheLeakedParent = $false
    if ($cacheExists) {
        $cacheText = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($cache))
        $cacheLeakedParent = $cacheText.Contains($privacyMarker)
    }

    $env:SEARCH_TOOL_GOOGLE_KEY = $null
    $env:SEARCH_TOOL_GOOGLE_CX = $null
    $secondOutput = @(& $resolvedCli web-lookup $index $LookupTarget 2>&1 | ForEach-Object { [string]$_ })
    $secondExit = $LASTEXITCODE
    $secondSourceCache = [bool]($secondOutput | Where-Object { $_ -eq 'source=cache' })
    $secondLeakedParent = (($secondOutput -join [Environment]::NewLine).Contains($privacyMarker))

    $checks = [ordered]@{
        first_request_exit_zero = ($firstExit -eq 0)
        first_request_source_web = $firstSourceWeb
        cache_created = [bool]$cacheExists
        cache_nonempty = ($cacheBytes -gt 32)
        second_request_exit_zero_without_credentials = ($secondExit -eq 0)
        second_request_source_cache = $secondSourceCache
        parent_path_not_in_first_output = (-not $firstLeakedParent)
        parent_path_not_in_second_output = (-not $secondLeakedParent)
        parent_path_not_in_cache = (-not $cacheLeakedParent)
    }
    $passed = -not ($checks.Values -contains $false)
    $report = [ordered]@{
        schema = 1
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        computer_name = [Environment]::MachineName
        cli = $resolvedCli
        lookup_target_filename = [IO.Path]::GetFileName($LookupTarget)
        first_exit_code = $firstExit
        second_exit_code = $secondExit
        cache_bytes = $cacheBytes
        cache_sha256 = $cacheSha256
        checks = $checks
        result = if ($passed) { 'PASS' } else { 'FAIL' }
    }
    Save-Report $report
    if ($Enforce -and -not $passed) { throw 'Web Resolver validation did not produce PASS.' }
} finally {
    $env:SEARCH_TOOL_GOOGLE_KEY = $savedKey
    $env:SEARCH_TOOL_GOOGLE_CX = $savedCx
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
