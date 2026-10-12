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
function Test-SufficientDesktopOverlap($WindowRect, $ReferenceDesktop, $CandidateDesktop) {
    # Matching display resolution does not establish that the candidate
    # HWND actually occupies the physical desktop. Accept explicit virtual
    # origin coordinates for legitimate negative-X multimonitor layouts.
    try {
        if ($null -eq $ReferenceDesktop -or $null -eq $CandidateDesktop) { return $false }
        [decimal]$originX = 0
        [decimal]$originY = 0
        [decimal]$candidateX = 0
        [decimal]$candidateY = 0
        if (Has-Field $ReferenceDesktop 'x') { $originX = [decimal]$ReferenceDesktop.x }
        if (Has-Field $ReferenceDesktop 'y') { $originY = [decimal]$ReferenceDesktop.y }
        if (Has-Field $CandidateDesktop 'x') { $candidateX = [decimal]$CandidateDesktop.x }
        if (Has-Field $CandidateDesktop 'y') { $candidateY = [decimal]$CandidateDesktop.y }
        if ($candidateX -ne $originX -or $candidateY -ne $originY) { return $false }
        [decimal]$displayWidth = $ReferenceDesktop.width
        [decimal]$displayHeight = $ReferenceDesktop.height
        if ($displayWidth -le 0 -or $displayHeight -le 0 -or
            $displayWidth -gt 65536 -or $displayHeight -gt 65536) { return $false }
        [decimal]$x = $WindowRect.x
        [decimal]$y = $WindowRect.y
        [decimal]$width = $WindowRect.width
        [decimal]$height = $WindowRect.height
        [decimal]$overlapW = [Math]::Max([decimal]0,
            [Math]::Min(($x + $width), ($originX + $displayWidth)) - [Math]::Max($x,$originX))
        [decimal]$overlapH = [Math]::Max([decimal]0,
            [Math]::Min(($y + $height), ($originY + $displayHeight)) - [Math]::Max($y,$originY))
        return ($overlapW * $overlapH * 2 -ge $width * $height)
    } catch { return $false }
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
    $refDesktop = if (Has-Field $Ref 'desktop_physical_pixels') { $Ref.desktop_physical_pixels } else { $null }
    $candidateDesktop = if (Has-Field $Candidate 'desktop_physical_pixels') { $Candidate.desktop_physical_pixels } else { $null }
    if (-not (Test-SufficientDesktopOverlap $candidateRect $refDesktop $candidateDesktop)) {
        $blockers.Add('Candidate frame is mostly outside the recorded physical desktop or virtual-desktop origin differs')
    }
    if (-not (Has-Field $Candidate 'physical_dpi') -or [int]$Candidate.physical_dpi -ne 96) {
        $blockers.Add('Candidate DPI must be measured and match reference 96 DPI')
    }
    # A similar screen size and DPI do not make Windows 11 builds
    # interchangeable: native SearchHost visual geometry can vary by build.
    # Block missing/unparseable versions rather than silently claiming that
    # a different Windows release matches the archived 22631 reference.
    $referenceBuild = ''
    if (Has-Field $Ref 'platform') {
        $buildMatch = [regex]::Match([string]$Ref.platform,'\bWindows 11\b.*\bbuild\s+(\d{5,6})\b', [Text.RegularExpressions.RegexOptions]::IgnoreCase)
        if ($buildMatch.Success) { $referenceBuild = $buildMatch.Groups[1].Value }
    }
    $candidateBuild = if (Has-Field $Candidate 'windows_build') { [string]$Candidate.windows_build } else { '' }
    if ($referenceBuild -eq '' -or $candidateBuild -notmatch '^\d{5,6}$' -or $candidateBuild -cne $referenceBuild) {
        $blockers.Add('Candidate Windows 11 build is missing or differs from native reference build')
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
        reference_windows_build = $referenceBuild
        candidate_windows_build = $candidateBuild
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
        platform = 'Windows 11 Pro build 22631'
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
        windows_build = 22631
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
    $candidate.windows_build = 26100
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'different Windows 11 build cannot use earlier native geometry'
    $candidate.windows_build = $null
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'missing candidate Windows build must be rejected'
    $candidate.windows_build = 22631
    $ref.platform = 'Windows 11 Pro'
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'unidentified reference build must be rejected'
    $ref.platform = 'Windows 11 Pro build 22631'
    $candidate.native_search_untouched = $false
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'native Search ownership not proven'
    $candidate.native_search_untouched = $true
    $originalRect = $candidate.search_tool_flyout.rect
    $candidate.search_tool_flyout.rect = [pscustomobject]@{x=-30000;y=-30000;width=780;height=720}
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'fully offscreen HWND must not be physical parity evidence'
    $candidate.search_tool_flyout.rect = [pscustomobject]@{x=3439;y=500;width=800;height=600}
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'one-pixel desktop overlap cannot pass provenance'
    $candidate.search_tool_flyout.rect = [pscustomobject]@{x=3040;y=500;width=800;height=600}
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'PARTIAL') 'exactly half the frame visible may remain diagnostic'
    $candidate.search_tool_flyout.rect = $originalRect
    $ref.desktop_physical_pixels = [pscustomobject]@{x=-1920;y=0;width=5360;height=1440}
    $candidate.desktop_physical_pixels = [pscustomobject]@{x=-1920;y=0;width=5360;height=1440}
    $candidate.search_tool_flyout.rect = [pscustomobject]@{x=-1700;y=500;width=780;height=720}
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'PARTIAL') 'legitimate negative monitor origin remains diagnostic'
    $candidate.desktop_physical_pixels.x = 0
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'mismatched virtual desktop origin cannot pass'
    $candidate.desktop_physical_pixels.x = 'invalid'
    Assert-Check ((Compare-Evidence $ref $candidate $head).result -ceq 'BLOCKED') 'malformed virtual desktop origin cannot pass'
    Write-Output 'Windows 11 Search geometry/parity fail-closed selftest PASS (15 assertions).'
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
