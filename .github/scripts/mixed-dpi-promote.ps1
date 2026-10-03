[CmdletBinding()]
param(
    [ValidateSet('Validate','Apply','SelfTest')]
    [string]$Mode = 'Validate',
    [string]$Bundle,
    [string]$EvidenceFile,
    [string]$WorkspaceRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$scriptRepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$root = if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $scriptRepoRoot
} else {
    [IO.Path]::GetFullPath($WorkspaceRoot)
}

$releaseStatePath = Join-Path $root 'docs\RELEASE_STATE.json'
$checkerPath = Join-Path $root '.github\scripts\release-state-check.ps1'
$selfTestPath = Join-Path $root '.github\scripts\release-state-selftest.ps1'

function Assert-Promotion {
    param(
        [Parameter(Mandatory)] [bool]$Condition,
        [Parameter(Mandatory)] [string]$Message
    )
    if (-not $Condition) {
        throw "MIXED_DPI_PROMOTION_FAIL: $Message"
    }
}

function Read-Json {
    param([Parameter(Mandatory)] [string]$Path)
    Assert-Promotion (Test-Path -LiteralPath $Path -PathType Leaf) "missing JSON file: $Path"
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [string]$Text
    )
    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}

function Normalize-Text {
    param([Parameter(Mandatory)] [string]$Text)
    return ($Text -replace [Environment]::NewLine, [char]10)
}

function Replace-Exact {
    param(
        [Parameter(Mandatory)] [string]$Text,
        [Parameter(Mandatory)] [string]$Old,
        [Parameter(Mandatory)] [string]$New,
        [Parameter(Mandatory)] [string]$Name
    )
    $first = $Text.IndexOf($Old, [StringComparison]::Ordinal)
    Assert-Promotion ($first -ge 0) "$Name marker was not found"
    $second = $Text.IndexOf($Old, $first + $Old.Length, [StringComparison]::Ordinal)
    Assert-Promotion ($second -lt 0) "$Name marker was not unique"
    return $Text.Substring(0, $first) + $New + $Text.Substring($first + $Old.Length)
}

function Replace-RegexOnce {
    param(
        [Parameter(Mandatory)] [string]$Text,
        [Parameter(Mandatory)] [string]$Pattern,
        [Parameter(Mandatory)] [string]$Replacement,
        [Parameter(Mandatory)] [string]$Name
    )
    $regex = [regex]::new($Pattern, [Text.RegularExpressions.RegexOptions]::Singleline)
    $matches = $regex.Matches($Text)
    Assert-Promotion ($matches.Count -eq 1) "$Name expected exactly one match, found $($matches.Count)"
    return $regex.Replace($Text, $Replacement, 1)
}

function Get-Sha256 {
    param([Parameter(Mandatory)] [string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToUpperInvariant()
}

function Get-GitBlobSha {
    param([Parameter(Mandatory)] [string]$Path)
    $sha = (& git -C $root hash-object $Path).Trim()
    Assert-Promotion ($LASTEXITCODE -eq 0 -and $sha -match '^[0-9a-f]{40}$') "unable to calculate Git blob SHA: $Path"
    return $sha
}

function Get-HeadSha {
    $sha = (& git -C $root rev-parse HEAD).Trim()
    Assert-Promotion ($LASTEXITCODE -eq 0 -and $sha -match '^[0-9a-f]{40}$') 'unable to resolve repository HEAD'
    return $sha
}

function Get-RequiredBundleChecks {
    return @(
        'exercise_pass',
        'exercise_mixed_dpi',
        'exercise_multiple_monitors',
        'exercise_distinct_dpi',
        'exercise_all_moves_pass',
        'primary_change_pass',
        'primary_change_observed',
        'primary_gui_survived',
        'primary_window_recovered',
        'primary_dpi_match',
        'monitor_removal_pass',
        'monitor_removal_observed',
        'removal_window_was_on_removed_monitor',
        'removal_gui_survived',
        'removal_window_recovered',
        'removal_dpi_match',
        'removal_target_recorded'
    )
}

function Validate-Bundle {
    param([Parameter(Mandatory)] [string]$BundlePath)

    $state = Read-Json $releaseStatePath
    Assert-Promotion ([string]$state.package.status -eq 'VALIDATED') 'release package is not VALIDATED'

    $blockers = @($state.external_blockers)
    Assert-Promotion ($blockers.Count -eq 1 -and [string]$blockers[0].name -eq 'mixed_dpi') 'mixed_dpi must be the only unresolved blocker before promotion'

    $completedNames = @($state.completed_external_gates | ForEach-Object { [string]$_.name })
    foreach ($required in @('smartscreen','defender','web_resolver')) {
        Assert-Promotion ($completedNames -contains $required) "required completed external gate is missing: $required"
    }
    Assert-Promotion ($completedNames -notcontains 'mixed_dpi') 'mixed_dpi is already completed'

    $bundleFull = [IO.Path]::GetFullPath($BundlePath)
    $bundle = Read-Json $bundleFull
    Assert-Promotion ([string]$bundle.result -eq 'PASS') 'finalizer bundle result is not PASS'
    Assert-Promotion ([string]$bundle.gate -eq 'mixed_dpi') 'finalizer bundle gate is not mixed_dpi'

    $head = Get-HeadSha
    Assert-Promotion ([string]$bundle.current_head_sha -eq $head) "bundle HEAD does not match current HEAD"
    Assert-Promotion ([string]$bundle.packaged_source_sha -eq [string]$state.package.packaged_source_sha) 'bundle packaged-source SHA does not match release state'
    Assert-Promotion (([string]$bundle.package_sha256).ToUpperInvariant() -eq ([string]$state.package.sha256).ToUpperInvariant()) 'bundle package SHA-256 does not match release state'
    Assert-Promotion ([int64]$bundle.package_bytes -eq [int64]$state.package.bytes) 'bundle package byte size does not match release state'

    foreach ($checkName in Get-RequiredBundleChecks) {
        $property = $bundle.checks.PSObject.Properties[$checkName]
        Assert-Promotion ($null -ne $property) "bundle is missing required check: $checkName"
        Assert-Promotion ([bool]$property.Value) "bundle check is not PASS: $checkName"
    }

    foreach ($key in @('exercise','primary_change','monitor_removal','monitor_removal_meta')) {
        $pathProperty = $bundle.evidence.PSObject.Properties[$key]
        $hashProperty = $bundle.evidence_sha256.PSObject.Properties[$key]
        Assert-Promotion ($null -ne $pathProperty) "bundle evidence path missing: $key"
        Assert-Promotion ($null -ne $hashProperty) "bundle evidence SHA-256 missing: $key"
        $sourcePath = [IO.Path]::GetFullPath([string]$pathProperty.Value)
        Assert-Promotion (Test-Path -LiteralPath $sourcePath -PathType Leaf) "bundle source evidence file missing: $sourcePath"
        $actual = Get-Sha256 $sourcePath
        Assert-Promotion ($actual -eq ([string]$hashProperty.Value).ToUpperInvariant()) "bundle source evidence SHA-256 mismatch: $key"
    }

    $exercise = Read-Json ([string]$bundle.evidence.exercise)
    $primary = Read-Json ([string]$bundle.evidence.primary_change)
    $removal = Read-Json ([string]$bundle.evidence.monitor_removal)
    $removalMeta = Read-Json ([string]$bundle.evidence.monitor_removal_meta)
    Assert-Promotion ([string]$exercise.result -eq 'PASS') 'exercise source evidence is not PASS'
    Assert-Promotion ([string]$primary.result -eq 'PASS') 'primary-change source evidence is not PASS'
    Assert-Promotion ([string]$removal.result -eq 'PASS') 'monitor-removal source evidence is not PASS'
    Assert-Promotion ([string]$removalMeta.result -eq 'PREPARED') 'monitor-removal metadata is not PREPARED'
    Assert-Promotion ([string]$removalMeta.package_sha256 -eq [string]$state.package.sha256) 'monitor-removal metadata package SHA-256 mismatch'
    Assert-Promotion ([int64]$removalMeta.package_bytes -eq [int64]$state.package.bytes) 'monitor-removal metadata package byte-size mismatch'

    return [pscustomobject]@{
        state = $state
        bundle = $bundle
        bundle_path = $bundleFull
        head = $head
    }
}

function New-CanonicalEvidence {
    param(
        [Parameter(Mandatory)] [object]$Validated,
        [Parameter(Mandatory)] [string]$Destination
    )

    $bundle = $Validated.bundle
    $payload = [ordered]@{
        schema = 1
        timestamp_utc = [string]$bundle.timestamp_utc
        result = 'PASS'
        gate = 'mixed_dpi'
        source_head_sha = [string]$bundle.current_head_sha
        packaged_source_sha = [string]$bundle.packaged_source_sha
        package_sha256 = ([string]$bundle.package_sha256).ToUpperInvariant()
        package_bytes = [int64]$bundle.package_bytes
        finalizer_bundle_sha256 = Get-Sha256 $Validated.bundle_path
        checks = $bundle.checks
        source_evidence = [ordered]@{
            exercise_sha256 = ([string]$bundle.evidence_sha256.exercise).ToUpperInvariant()
            primary_change_sha256 = ([string]$bundle.evidence_sha256.primary_change).ToUpperInvariant()
            monitor_removal_sha256 = ([string]$bundle.evidence_sha256.monitor_removal).ToUpperInvariant()
            monitor_removal_meta_sha256 = ([string]$bundle.evidence_sha256.monitor_removal_meta).ToUpperInvariant()
            exercise_file = [IO.Path]::GetFileName([string]$bundle.evidence.exercise)
            primary_change_file = [IO.Path]::GetFileName([string]$bundle.evidence.primary_change)
            monitor_removal_file = [IO.Path]::GetFileName([string]$bundle.evidence.monitor_removal)
            monitor_removal_meta_file = [IO.Path]::GetFileName([string]$bundle.evidence.monitor_removal_meta)
        }
    }

    Write-Utf8NoBom -Path $Destination -Text (($payload | ConvertTo-Json -Depth 12) + [Environment]::NewLine)
    return $payload
}

function Update-ReleaseState {
    param(
        [Parameter(Mandatory)] [object]$State,
        [Parameter(Mandatory)] [string]$EvidenceRelative,
        [Parameter(Mandatory)] [string]$EvidenceBlobSha
    )

    $State.updated_utc = [DateTime]::UtcNow.ToString('o')
    $State.external_blockers = @()
    $completed = @($State.completed_external_gates)
    $completed += [pscustomobject]@{
        name = 'mixed_dpi'
        expected_result = 'PASS'
        evidence = $EvidenceRelative
        evidence_blob_sha = $EvidenceBlobSha
    }
    $State.completed_external_gates = $completed
    Write-Utf8NoBom -Path $releaseStatePath -Text (($State | ConvertTo-Json -Depth 20) + [Environment]::NewLine)
}

function Update-Checker {
    param(
        [Parameter(Mandatory)] [string]$EvidenceRelative,
        [Parameter(Mandatory)] [string]$EvidenceBlobSha
    )

    $text = Normalize-Text ([IO.File]::ReadAllText($checkerPath))
    $text = Replace-Exact -Text $text -Old '$requiredExternalBlockers = @(''mixed_dpi'')' -New '$requiredExternalBlockers = @()' -Name 'required external blockers'
    $text = Replace-Exact -Text $text -Old '$requiredCompletedExternalGates = @(''smartscreen'', ''defender'', ''web_resolver'')' -New '$requiredCompletedExternalGates = @(''smartscreen'', ''defender'', ''web_resolver'', ''mixed_dpi'')' -Name 'required completed external gates'

    $oldBlockerMap = @'
$requiredExternalBlockerEvidence = @{
    mixed_dpi = @{
        path = 'docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json'
        blob_sha = 'a1c0c329a1024ab02948361b9f8102e069f0db95'
    }
}
'@
    $newBlockerMap = '$requiredExternalBlockerEvidence = @{}'
    $text = Replace-Exact -Text $text -Old (Normalize-Text $oldBlockerMap) -New $newBlockerMap -Name 'external blocker evidence map'

    $oldCompletedTail = @'
    web_resolver = @{
        path = 'docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json'
        blob_sha = 'cb239296533c381ce32f59f36ad2b1e9a016d4e0'
    }
}
'@
    $newCompletedTail = @"
    web_resolver = @{
        path = 'docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json'
        blob_sha = 'cb239296533c381ce32f59f36ad2b1e9a016d4e0'
    }
    mixed_dpi = @{
        path = '$EvidenceRelative'
        blob_sha = '$EvidenceBlobSha'
    }
}
"@
    $text = Replace-Exact -Text $text -Old (Normalize-Text $oldCompletedTail) -New (Normalize-Text $newCompletedTail) -Name 'completed gate evidence map'
    Write-Utf8NoBom -Path $checkerPath -Text $text
}

function Update-SelfTest {
    $text = Normalize-Text ([IO.File]::ReadAllText($selfTestPath))

    $finalBlock = @'
    # Final mixed-DPI completion may not be demoted or detached from its sealed PASS evidence.
    $falseMixedDpiStatePath = Join-Path $tempRoot 'false-mixed-dpi-completed-state.json'
    $falseMixedDpiState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $mixedDpiGate = @($falseMixedDpiState.completed_external_gates | Where-Object { $_.name -eq 'mixed_dpi' } | Select-Object -First 1)
    if ($mixedDpiGate.Count -ne 1) { throw 'mixed_dpi completed gate missing from release state' }
    $mixedDpiGate[0].expected_result = 'BLOCKED'
    $falseMixedDpiState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $falseMixedDpiStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "completed external gate 'mixed_dpi' expected_result must be PASS" -Command {
        & $checker -StateFile $falseMixedDpiStatePath -HeadRef HEAD
    }

    $swappedEvidenceStatePath = Join-Path $tempRoot 'swapped-mixed-dpi-completed-evidence-state.json'
    $swappedEvidenceState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $swappedMixedDpi = @($swappedEvidenceState.completed_external_gates | Where-Object { $_.name -eq 'mixed_dpi' } | Select-Object -First 1)
    $completedWebResolver = @($swappedEvidenceState.completed_external_gates | Where-Object { $_.name -eq 'web_resolver' } | Select-Object -First 1)
    if ($swappedMixedDpi.Count -ne 1 -or $completedWebResolver.Count -ne 1) { throw 'required mixed_dpi/web_resolver completed gates missing from release state' }
    $swappedMixedDpi[0].evidence = [string]$completedWebResolver[0].evidence
    $swappedEvidenceState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $swappedEvidenceStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "completed external gate 'mixed_dpi' evidence path mismatch" -Command {
        & $checker -StateFile $swappedEvidenceStatePath -HeadRef HEAD
    }

    & git read-tree HEAD
    if ($LASTEXITCODE -ne 0) { throw "git read-tree failed before mixed-DPI completed evidence probe with exit code $LASTEXITCODE" }

    $currentMixedDpiState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $currentMixedDpiGate = @($currentMixedDpiState.completed_external_gates | Where-Object { $_.name -eq 'mixed_dpi' } | Select-Object -First 1)
    if ($currentMixedDpiGate.Count -ne 1) { throw 'mixed_dpi completed gate missing before evidence tamper probe' }
    $mixedDpiEvidencePath = [string]$currentMixedDpiGate[0].evidence

    $evidencePayload = Join-Path $tempRoot 'mixed-dpi-completed-evidence-tamper.json'
    Set-Content -LiteralPath $evidencePayload -Value '{"schema":1,"result":"PASS","reason":"synthetic tamper"}' -Encoding UTF8
    $tamperedEvidenceBlob = (& git hash-object -w $evidencePayload).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceBlob -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI completed evidence blob'
    }

    & git update-index --add --cacheinfo "100644,$tamperedEvidenceBlob,$mixedDpiEvidencePath"
    if ($LASTEXITCODE -ne 0) { throw "git update-index failed for mixed-DPI completed evidence probe with exit code $LASTEXITCODE" }

    $tamperedEvidenceTree = (& git write-tree).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceTree -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI completed evidence tree'
    }

    $tamperedEvidenceCommit = (& git commit-tree $tamperedEvidenceTree -p $parent -m 'release-state mixed-DPI completed evidence tamper self-test').Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI completed evidence commit'
    }

    Assert-ExpectedFailure -ExpectedMessage "completed external gate 'mixed_dpi' evidence blob mismatch" -Command {
        & $checker -StateFile $statePath -HeadRef $tamperedEvidenceCommit
    }

'@

    $text = Replace-RegexOnce -Text $text -Pattern '    # A blocker may not be promoted to PASS unless its evidence says PASS\..*?(?=    # A completed external gate may not be demoted or detached from its sealed PASS evidence\.)' -Replacement (Normalize-Text $finalBlock) -Name 'mixed-DPI blocker self-test block'

    $unexpectedBlock = @'
    $unexpectedBlockerStatePath = Join-Path $tempRoot 'unexpected-blocker-state.json'
    $unexpectedBlockerState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $unexpectedBlockerState.external_blockers = @(
        [pscustomobject]@{
            name = 'mixed_dpi'
            expected_result = 'BLOCKED'
            evidence = 'docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json'
            evidence_blob_sha = 'a1c0c329a1024ab02948361b9f8102e069f0db95'
        }
    )
    $unexpectedBlockerState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $unexpectedBlockerStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'external_blockers contains unexpected value: mixed_dpi' -Command {
        & $checker -StateFile $unexpectedBlockerStatePath -HeadRef HEAD
    }

'@

    $text = Replace-RegexOnce -Text $text -Pattern '    \$missingBlockerStatePath = Join-Path \$tempRoot ''missing-required-blocker-state\.json''.*?(?=    \$missingCompletedStatePath =)' -Replacement (Normalize-Text $unexpectedBlock) -Name 'required blocker structural self-test block'

    $text = Replace-Exact -Text $text -Old '        false_blocker_pass_rejected = $true' -New '        mixed_dpi_completed_gate_demote_rejected = $true' -Name 'self-test demote output'
    $text = Replace-Exact -Text $text -Old '        swapped_blocker_evidence_rejected = $true' -New '        mixed_dpi_completed_gate_evidence_swap_rejected = $true' -Name 'self-test swap output'
    $text = Replace-Exact -Text $text -Old '        tampered_blocker_evidence_blob_rejected = $true' -New '        mixed_dpi_completed_gate_evidence_tamper_rejected = $true' -Name 'self-test tamper output'
    $text = Replace-Exact -Text $text -Old '        missing_required_blocker_rejected = $true' -New '        unexpected_blocker_rejected = $true' -Name 'self-test blocker output'
    $text = Replace-Exact -Text $text -Old '        missing_blocker_evidence_marker_rejected = $true' -New '        missing_completed_gate_evidence_marker_rejected = $true' -Name 'self-test marker output'
    $text = Replace-Exact -Text $text -Old '    # Every synchronized document must carry every blocker evidence marker.' -New '    # Every synchronized document must carry every completed-gate evidence marker.' -Name 'self-test marker comment'

    Write-Utf8NoBom -Path $selfTestPath -Text $text
}

function Update-SynchronizedDocs {
    param(
        [Parameter(Mandatory)] [object]$State,
        [Parameter(Mandatory)] [string]$EvidenceRelative,
        [Parameter(Mandatory)] [string]$EvidenceBlobSha
    )

    $lines = @(
        '',
        '',
        '## Final mixed-DPI release promotion',
        '',
        '- External blockers: **none**.',
        '- Completed external gates: smartscreen, defender, web_resolver, mixed_dpi.',
        "- Mixed-DPI physical evidence: $EvidenceRelative",
        "- Mixed-DPI evidence Git blob: $EvidenceBlobSha",
        "- Validated package source remains $($State.package.packaged_source_sha)",
        "- Validated package SHA-256 remains $($State.package.sha256)",
        "- Validated package size remains $($State.package.bytes) bytes.",
        '- The final physical evidence covers cross-monitor DPI movement, primary-display change, real monitor removal, GUI survival/recovery, active-monitor DPI matching and proof that the window was on the removed monitor.',
        ''
    )
    $block = $lines -join [Environment]::NewLine

    foreach ($relative in @($State.synchronized_documents)) {
        $path = Join-Path $root ([string]$relative)
        Assert-Promotion (Test-Path -LiteralPath $path -PathType Leaf) "synchronized document missing: $relative"
        $existing = [IO.File]::ReadAllText($path)
        Assert-Promotion (-not $existing.Contains('## Final mixed-DPI release promotion')) "synchronized document already contains final promotion block: $relative"
        Write-Utf8NoBom -Path $path -Text ($existing.TrimEnd() + $block)
    }
}

function Apply-Promotion {
    param(
        [Parameter(Mandatory)] [string]$BundlePath,
        [string]$RequestedEvidenceFile
    )

    $trackedChanges = @(& git -C $root status --porcelain --untracked-files=no)
    Assert-Promotion ($LASTEXITCODE -eq 0) 'git status failed'
    Assert-Promotion ($trackedChanges.Count -eq 0) 'tracked worktree changes exist before promotion'

    $validated = Validate-Bundle $BundlePath
    $state = $validated.state

    $evidenceRelative = if ([string]::IsNullOrWhiteSpace($RequestedEvidenceFile)) {
        $date = [DateTime]::UtcNow.ToString('yyyyMMdd')
        "docs/evidence/mixed-dpi-physical-pass-$($validated.head.Substring(0,7))-$date.json"
    } else {
        $candidate = $RequestedEvidenceFile.Replace('\','/')
        Assert-Promotion ($candidate.StartsWith('docs/evidence/', [StringComparison]::Ordinal)) 'EvidenceFile must be under docs/evidence/'
        $candidate
    }

    $evidenceFull = Join-Path $root $evidenceRelative
    Assert-Promotion (-not (Test-Path -LiteralPath $evidenceFull)) "canonical mixed-DPI evidence already exists: $evidenceRelative"
    New-CanonicalEvidence -Validated $validated -Destination $evidenceFull | Out-Null
    $blobSha = Get-GitBlobSha $evidenceFull

    Update-ReleaseState -State $state -EvidenceRelative $evidenceRelative -EvidenceBlobSha $blobSha
    Update-Checker -EvidenceRelative $evidenceRelative -EvidenceBlobSha $blobSha
    Update-SelfTest
    Update-SynchronizedDocs -State $state -EvidenceRelative $evidenceRelative -EvidenceBlobSha $blobSha

    & git -C $root diff --check
    Assert-Promotion ($LASTEXITCODE -eq 0) 'git diff --check failed after promotion'

    [ordered]@{
        schema = 1
        result = 'PREPARED'
        head_sha = $validated.head
        evidence = $evidenceRelative
        evidence_blob_sha = $blobSha
        next_steps = @(
            'git add .github docs',
            'git commit -m "Promote final mixed-DPI physical PASS"',
            'run .github/scripts/release-state-check.ps1',
            'run .github/scripts/release-state-selftest.ps1',
            'open PR and require exact-head CI before merge'
        )
    } | ConvertTo-Json -Depth 6
}

function Invoke-SelfTest {
    $tempBase = Join-Path $env:TEMP ("SearchToolMixedDpiPromote-" + [Guid]::NewGuid().ToString('N'))
    $worktree = Join-Path $tempBase 'repo'
    New-Item -ItemType Directory -Force -Path $tempBase | Out-Null

    try {
        & git -C $scriptRepoRoot worktree add --detach $worktree HEAD | Out-Host
        Assert-Promotion ($LASTEXITCODE -eq 0) 'failed to create promotion self-test worktree'

        $state = Read-Json (Join-Path $worktree 'docs\RELEASE_STATE.json')
        $head = (& git -C $worktree rev-parse HEAD).Trim()
        $evidenceDir = Join-Path $worktree 'dist\mixed-dpi-promotion-selftest'
        New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

        $exercisePath = Join-Path $evidenceDir 'exercise.json'
        $primaryPath = Join-Path $evidenceDir 'primary-change-verify.json'
        $removalPath = Join-Path $evidenceDir 'monitor-removal-verify.json'
        $metaPath = Join-Path $evidenceDir 'monitor-removal-meta.json'
        $bundlePath = Join-Path $evidenceDir 'bundle.json'

        Write-Utf8NoBom -Path $exercisePath -Text (([ordered]@{schema=1;result='PASS'} | ConvertTo-Json -Compress) + [Environment]::NewLine)
        Write-Utf8NoBom -Path $primaryPath -Text (([ordered]@{schema=1;result='PASS'} | ConvertTo-Json -Compress) + [Environment]::NewLine)
        Write-Utf8NoBom -Path $removalPath -Text (([ordered]@{schema=1;result='PASS'} | ConvertTo-Json -Compress) + [Environment]::NewLine)

        $metaPayload = [ordered]@{
            schema = 1
            result = 'PREPARED'
            package_sha256 = [string]$state.package.sha256
            package_bytes = [int64]$state.package.bytes
        }
        Write-Utf8NoBom -Path $metaPath -Text (($metaPayload | ConvertTo-Json -Depth 4) + [Environment]::NewLine)

        $checks = [ordered]@{}
        foreach ($name in Get-RequiredBundleChecks) {
            $checks[$name] = $true
        }

        $bundlePayload = [ordered]@{
            schema = 1
            timestamp_utc = [DateTime]::UtcNow.ToString('o')
            result = 'PASS'
            gate = 'mixed_dpi'
            current_head_sha = $head
            packaged_source_sha = [string]$state.package.packaged_source_sha
            package_sha256 = [string]$state.package.sha256
            package_bytes = [int64]$state.package.bytes
            checks = $checks
            evidence = [ordered]@{
                exercise = $exercisePath
                primary_change = $primaryPath
                monitor_removal = $removalPath
                monitor_removal_meta = $metaPath
            }
            evidence_sha256 = [ordered]@{
                exercise = Get-Sha256 $exercisePath
                primary_change = Get-Sha256 $primaryPath
                monitor_removal = Get-Sha256 $removalPath
                monitor_removal_meta = Get-Sha256 $metaPath
            }
        }
        Write-Utf8NoBom -Path $bundlePath -Text (($bundlePayload | ConvertTo-Json -Depth 12) + [Environment]::NewLine)

        $badBundlePath = Join-Path $evidenceDir 'bad-bundle.json'
        $badBundlePayload = Get-Content -LiteralPath $bundlePath -Raw | ConvertFrom-Json
        $badBundlePayload.checks.removal_window_was_on_removed_monitor = $false
        Write-Utf8NoBom -Path $badBundlePath -Text (($badBundlePayload | ConvertTo-Json -Depth 12) + [Environment]::NewLine)

        $negativeRejected = $false
        try {
            & $PSCommandPath -Mode Validate -WorkspaceRoot $worktree -Bundle $badBundlePath | Out-Null
        } catch {
            $negativeRejected = $true
        }
        Assert-Promotion $negativeRejected 'self-test failed to reject an invalid mixed-DPI bundle'

        & $PSCommandPath -Mode Apply -WorkspaceRoot $worktree -Bundle $bundlePath -EvidenceFile 'docs/evidence/mixed-dpi-selftest-pass.json' | Out-Host
        Assert-Promotion ($LASTEXITCODE -eq 0) 'promotion Apply failed in self-test worktree'

        & git -C $worktree add .github docs
        Assert-Promotion ($LASTEXITCODE -eq 0) 'git add failed in promotion self-test worktree'

        $oldAuthorName = $env:GIT_AUTHOR_NAME
        $oldAuthorEmail = $env:GIT_AUTHOR_EMAIL
        $oldCommitterName = $env:GIT_COMMITTER_NAME
        $oldCommitterEmail = $env:GIT_COMMITTER_EMAIL
        try {
            $env:GIT_AUTHOR_NAME = 'Search Tool CI'
            $env:GIT_AUTHOR_EMAIL = 'search-tool-ci@example.invalid'
            $env:GIT_COMMITTER_NAME = 'Search Tool CI'
            $env:GIT_COMMITTER_EMAIL = 'search-tool-ci@example.invalid'
            & git -C $worktree commit -m 'mixed-DPI promotion self-test' | Out-Host
            Assert-Promotion ($LASTEXITCODE -eq 0) 'self-test promotion commit failed'
        } finally {
            $env:GIT_AUTHOR_NAME = $oldAuthorName
            $env:GIT_AUTHOR_EMAIL = $oldAuthorEmail
            $env:GIT_COMMITTER_NAME = $oldCommitterName
            $env:GIT_COMMITTER_EMAIL = $oldCommitterEmail
        }

        & (Join-Path $worktree '.github\scripts\release-state-check.ps1') | Out-Host
        Assert-Promotion ($LASTEXITCODE -eq 0) 'promoted release-state checker failed in self-test'
        & (Join-Path $worktree '.github\scripts\release-state-selftest.ps1') | Out-Host
        Assert-Promotion ($LASTEXITCODE -eq 0) 'promoted release-state self-test failed'

        $finalState = Read-Json (Join-Path $worktree 'docs\RELEASE_STATE.json')
        Assert-Promotion (@($finalState.external_blockers).Count -eq 0) 'self-test final state still has an external blocker'
        $mixedGate = @($finalState.completed_external_gates | Where-Object { $_.name -eq 'mixed_dpi' })
        Assert-Promotion ($mixedGate.Count -eq 1 -and [string]$mixedGate[0].expected_result -eq 'PASS') 'self-test final state did not promote mixed_dpi'

        [ordered]@{
            schema = 1
            result = 'PASS'
            invalid_bundle_rejected = $true
            apply_completed = $true
            promoted_checker_pass = $true
            promoted_self_test_pass = $true
            external_blocker_count = @($finalState.external_blockers).Count
            mixed_dpi_completed = $true
        } | ConvertTo-Json -Depth 6
    } finally {
        if (Test-Path -LiteralPath $worktree) {
            & git -C $scriptRepoRoot worktree remove --force $worktree 2>$null | Out-Null
        }
        Remove-Item -LiteralPath $tempBase -Recurse -Force -ErrorAction SilentlyContinue
    }
}

switch ($Mode) {
    'Validate' {
        Assert-Promotion (-not [string]::IsNullOrWhiteSpace($Bundle)) 'Bundle is required for Validate mode'
        $validated = Validate-Bundle $Bundle
        [ordered]@{
            schema = 1
            result = 'PASS'
            head_sha = $validated.head
            package_sha256 = [string]$validated.state.package.sha256
            package_bytes = [int64]$validated.state.package.bytes
        } | ConvertTo-Json -Depth 4
    }
    'Apply' {
        Assert-Promotion (-not [string]::IsNullOrWhiteSpace($Bundle)) 'Bundle is required for Apply mode'
        Apply-Promotion -BundlePath $Bundle -RequestedEvidenceFile $EvidenceFile
    }
    'SelfTest' {
        Invoke-SelfTest
    }
}
