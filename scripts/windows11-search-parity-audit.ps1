# Offline, read-only comparison of sanitized Windows 11 Search flyout evidence.
# Never opens SearchHost, presses keys, uses production index or captures the desktop.
# No input may upgrade approximate/synthetic evidence to visual acceptance PASS.
[CmdletBinding()]
param(
    [string] $ReferencePath,
    [string] $CandidatePath,
    [string] $ExpectedHead,
    [string] $OutputPath,
    [switch] $SelfTest
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-JsonEvidence([string] $Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing visual evidence: $Path"
    }
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}
function Has-Field($Object, [string] $Field) {
    return ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Field)
}
function Numeric-Rect($Rect, [string] $Label) {
    if ($null -eq $Rect) { throw "$Label rect missing" }
    $numbers = [ordered]@{}
    foreach ($key in @('x', 'y', 'width', 'height')) {
        if (-not (Has-Field $Rect $key)) { throw "$Label missing $key" }
        $value = $Rect.$key
        if ($null -eq $value -or [string]$value -notmatch '^-?[0-9]+$') {
            throw "$Label non-integer $key"
        }
        $numbers[$key] = [long]$value
    }
    if ($numbers.width -le 0 -or $numbers.height -le 0 -or
        $numbers.width -gt 16384 -or $numbers.height -gt 16384) {
        throw "$Label has invalid bounds"
    }
    return [pscustomobject]$numbers
}
function Compare-Evidence($Ref, $Candidate, [string] $Head) {
    if (-not (Has-Field $Ref 'search_flyout') -or
        -not (Has-Field $Ref.search_flyout 'rect_approx')) {
        throw 'Reference must contain a native Search flyout approximate rect'
    }
    $native = Numeric-Rect $Ref.search_flyout.rect_approx 'native_reference'
    $legacy = Has-Field $Candidate 'flyout_physical_px'
    $candidateRect = $null
    if ($legacy) {
        $candidateRect = Numeric-Rect $Candidate.flyout_physical_px 'candidate_legacy'
    } elseif ((Has-Field $Candidate 'search_tool_flyout') -and
              (Has-Field $Candidate.search_tool_flyout 'rect')) {
        $candidateRect = Numeric-Rect $Candidate.search_tool_flyout.rect 'candidate_physical'
    } else {
        throw 'Candidate must have flyout_physical_px or search_tool_flyout.rect'
    }
    $delta = [ordered]@{}
    foreach ($key in @('x', 'y', 'width', 'height')) {
        $delta[$key] = $candidateRect.$key - $native.$key
    }
    $blockers = [System.Collections.Generic.List[string]]::new()
    $sourceSha = if (Has-Field $Candidate 'source_sha') {
        [string]$Candidate.source_sha
    } elseif (Has-Field $Candidate 'source_commit_before_change') {
        [string]$Candidate.source_commit_before_change
    } else { '' }
    if ($Head -notmatch '^[a-fA-F0-9]{40}$' -or $sourceSha -cne $Head) {
        $blockers.Add('Candidate evidence is not from the exact checked-out HEAD commit')
    }
    if ($legacy -or -not (Has-Field $Candidate 'capture_kind') -or
        [string]$Candidate.capture_kind -cne 'PHYSICAL_WINDOWS11_DESKTOP') {
        $blockers.Add('Candidate evidence is synthetic/legacy or not a physical Windows 11 capture')
    }
    if (-not (Has-Field $Candidate 'desktop_physical_pixels') -or
        -not (Has-Field $Ref 'desktop_physical_pixels') -or
        [string]$Candidate.desktop_physical_pixels.width -cne [string]$Ref.desktop_physical_pixels.width -or
        [string]$Candidate.desktop_physical_pixels.height -cne [string]$Ref.desktop_physical_pixels.height) {
        $blockers.Add('Candidate and native reference desktop pixel sizes are not verified equal')
    }
    if (-not (Has-Field $Candidate 'physical_dpi') -or [int]$Candidate.physical_dpi -ne 96) {
        $blockers.Add('Candidate DPI must be measured and match reference 96 DPI')
    }
    if (-not (Has-Field $Candidate 'native_search_untouched') -or
        $Candidate.native_search_untouched -cne $true) {
        $blockers.Add('Native SearchHost/taskbar ownership preservation is not evidenced')
    }
    $result = if ($blockers.Count -eq 0) { 'PARTIAL' } else { 'BLOCKED' }
    return [pscustomobject][ordered]@{
        schema = 1
        result = $result
        acceptance = 'NOT_ACCEPTED'
        exact_head = $Head
        candidate_source_sha = $sourceSha
        reference_precision = [string]$Ref.search_flyout.precision
        comparison_kind = 'SANITIZED_APPROXIMATE_GEOMETRY_ONLY'
        native_rect_approx_px = $native
        candidate_rect_px = $candidateRect
        delta_candidate_minus_native_px = [pscustomobject]$delta
        blockers = @($blockers.ToArray())
        remaining = @(
            'Reference rect is approximated from a screenshot, not exact pixel bounds',
            'Independent Search Tool window does not replace Windows-owned SearchHost',
            'Native Search results hierarchy/ranking and supported entry-point behavior not compared',
            'Real physical keyboard/IME, Narrator/UIA, high contrast and screenshot review pending',
            'Release package remains INVALIDATED until separate acceptance gates complete'
        )
    }
}
function Assert-Check([bool] $Condition, [string] $Name) {
    if (-not $Condition) { throw "Visual parity selftest failed: $Name" }
}
if ($SelfTest) {
    $ref = [pscustomobject]@{
        desktop_physical_pixels = [pscustomobject]@{ width = 3440; height = 1440 }
        search_flyout = [pscustomobject]@{
            rect_approx = [pscustomobject]@{ x = 1332; y = 655; width = 776; height = 725 }
            precision = 'MANUALLY_APPROXIMATED_FROM_PHYSICAL_SCREENSHOT'
        }
    }
    $head = 'a' * 40
    $legacy = [pscustomobject]@{
        source_commit_before_change = 'b' * 40
        flyout_physical_px = [pscustomobject]@{ x = 1330; y = 660; width = 780; height = 720 }
    }
    $old = Compare-Evidence $ref $legacy $head
    Assert-Check ($old.result -ceq 'BLOCKED' -and $old.acceptance -ceq 'NOT_ACCEPTED') 'legacy evidence must fail closed'
    Assert-Check ($old.delta_candidate_minus_native_px.x -eq -2 -and
        $old.delta_candidate_minus_native_px.y -eq 5 -and
        $old.delta_candidate_minus_native_px.width -eq 4 -and
        $old.delta_candidate_minus_native_px.height -eq -5) 'signed position and size deltas'
    $candidate = [pscustomobject]@{
        source_sha = $head
        capture_kind = 'PHYSICAL_WINDOWS11_DESKTOP'
        physical_dpi = 96
        native_search_untouched = $true
        desktop_physical_pixels = $ref.desktop_physical_pixels
        search_tool_flyout = [pscustomobject]@{ rect = $legacy.flyout_physical_px }
    }
    $partial = Compare-Evidence $ref $candidate $head
    Assert-Check ($partial.result -ceq 'PARTIAL' -and $partial.acceptance -ceq 'NOT_ACCEPTED') 'physical same-head evidence must not imply parity'
    $candidate.source_sha = 'c' * 40
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'stale head blocked'
    $candidate.source_sha = $head
    $candidate.physical_dpi = 120
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'mismatched DPI blocked'
    $candidate.physical_dpi = 96
    $candidate.native_search_untouched = $false
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'native Search ownership not proven'
    Write-Output 'Windows 11 Search geometry/parity fail-closed selftest PASS (6 assertions).'
    return
}
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if ([string]::IsNullOrWhiteSpace($ReferencePath)) {
    $ReferencePath = Join-Path $repo 'docs/evidence/windows11-search-reference-20261008.json'
}
if ([string]::IsNullOrWhiteSpace($CandidatePath)) {
    $CandidatePath = Join-Path $repo 'docs/evidence/windows11-search-two-column-20261008.json'
}
if ([string]::IsNullOrWhiteSpace($ExpectedHead)) {
    $ExpectedHead = (& git -C $repo rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve current source commit SHA' }
}
$report = Compare-Evidence (Read-JsonEvidence $ReferencePath) (Read-JsonEvidence $CandidatePath) $ExpectedHead
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $directory = Split-Path -Parent $OutputPath
    if ($directory -and -not (Test-Path -LiteralPath $directory -PathType Container)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
}
$report | ConvertTo-Json -Depth 8
# This is an audit/report, not a PASS assertion: BLOCKED is an expected,
# truthful result for the existing stale/synthetic reference pair.
