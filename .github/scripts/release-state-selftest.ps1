[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$checker = Join-Path $PSScriptRoot 'release-state-check.ps1'
$statePath = Join-Path $root 'docs\RELEASE_STATE.json'

function Assert-ExpectedFailure {
    param(
        [Parameter(Mandatory)] [scriptblock]$Command,
        [Parameter(Mandatory)] [string]$ExpectedMessage
    )

    $failed = $false
    try {
        & $Command | Out-Null
    } catch {
        $failed = $true
        $message = [string]$_.Exception.Message
        if (-not $message.Contains($ExpectedMessage)) {
            throw "Expected failure containing '$ExpectedMessage', got: $message"
        }
    }

    if (-not $failed) {
        throw "Expected failure was not observed: $ExpectedMessage"
    }
}

# Positive control: current release state must pass first.
& $checker -StateFile $statePath -HeadRef HEAD | Out-Null

$currentState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
$currentStatus = ([string]$currentState.package.status).ToUpperInvariant()
if ($currentStatus -eq 'INVALIDATED') {
    $invalidProbePath = [IO.Path]::GetTempFileName()
    try {
        $invalidProbe = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
        $invalidProbe.package.invalidated_reason = ''
        $invalidProbe | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $invalidProbePath -Encoding UTF8
        Assert-ExpectedFailure -ExpectedMessage 'INVALIDATED package state requires package.invalidated_reason' -Command {
            & $checker -StateFile $invalidProbePath -HeadRef HEAD
        }

        # Prove fail-closed behavior if the candidate is prematurely marked VALIDATED.
        $prematureValidatedProbePath = [IO.Path]::GetTempFileName()
        try {
            $prematureValidatedProbe = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
            $prematureValidatedProbe.package.status = 'VALIDATED'
            $prematureValidatedProbe | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $prematureValidatedProbePath -Encoding UTF8
            Assert-ExpectedFailure -ExpectedMessage 'production deployment evidence is not PASS' -Command {
                & $checker -StateFile $prematureValidatedProbePath -HeadRef HEAD
            }
        } finally {
            Remove-Item -LiteralPath $prematureValidatedProbePath -Force -ErrorAction SilentlyContinue
        }

        [ordered]@{
            schema = 1
            result = 'PASS'
            positive_control = 'PASS'
            package_status = 'INVALIDATED'
            missing_invalidation_reason_rejected = $true
            premature_validated_without_production_deploy_rejected = $true
        } | ConvertTo-Json -Depth 4
    } finally {
        Remove-Item -LiteralPath $invalidProbePath -Force -ErrorAction SilentlyContinue
    }
    return
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("SearchToolReleaseStateSelfTest-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

$oldIndexFile = $env:GIT_INDEX_FILE
$oldAuthorName = $env:GIT_AUTHOR_NAME
$oldAuthorEmail = $env:GIT_AUTHOR_EMAIL
$oldCommitterName = $env:GIT_COMMITTER_NAME
$oldCommitterEmail = $env:GIT_COMMITTER_EMAIL

Push-Location $root
try {
    # Build an unreachable synthetic commit whose only extra non-doc/.github change is README.md.
    # This exercises the real git ancestry/diff path without mutating the checkout or a branch.
    $indexFile = Join-Path $tempRoot 'probe.index'
    $payload = Join-Path $tempRoot 'README.md'
    Set-Content -LiteralPath $payload -Value 'release-state-negative-probe' -Encoding UTF8

    $env:GIT_INDEX_FILE = $indexFile
    & git read-tree HEAD
    if ($LASTEXITCODE -ne 0) { throw "git read-tree failed with exit code $LASTEXITCODE" }

    $probeBlob = (& git hash-object -w $payload).Trim()
    if ($LASTEXITCODE -ne 0 -or $probeBlob -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic README blob'
    }

    & git update-index --add --cacheinfo "100644,$probeBlob,README.md"
    if ($LASTEXITCODE -ne 0) { throw "git update-index failed with exit code $LASTEXITCODE" }

    $probeTree = (& git write-tree).Trim()
    if ($LASTEXITCODE -ne 0 -or $probeTree -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic tree'
    }

    $parent = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $parent -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to resolve parent commit'
    }

    $env:GIT_AUTHOR_NAME = 'Search Tool Release Self-Test'
    $env:GIT_AUTHOR_EMAIL = 'release-selftest.invalid@example.invalid'
    $env:GIT_COMMITTER_NAME = $env:GIT_AUTHOR_NAME
    $env:GIT_COMMITTER_EMAIL = $env:GIT_AUTHOR_EMAIL

    $probeCommit = (& git commit-tree $probeTree -p $parent -m 'release-state negative self-test').Trim()
    if ($LASTEXITCODE -ne 0 -or $probeCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic commit'
    }

    Assert-ExpectedFailure -ExpectedMessage 'packaged inputs changed after validated source: README.md' -Command {
        & $checker -StateFile $statePath -HeadRef $probeCommit
    }

    # A stale package SHA in the machine-readable state must also fail.
    $staleStatePath = Join-Path $tempRoot 'stale-package-state.json'
    $staleState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $sha = [string]$staleState.package.sha256
    $replacement = if ($sha.StartsWith('0')) { '1' } else { '0' }
    $staleState.package.sha256 = $replacement + $sha.Substring(1)
    $staleState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $staleStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage 'package SHA-256 mismatch' -Command {
        & $checker -StateFile $staleStatePath -HeadRef HEAD
    }

    # A stale workspace test-count claim must not remain synchronized with the release docs.
    $staleDocsStatePath = Join-Path $tempRoot 'stale-doc-sync-state.json'
    $staleDocsState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $staleDocsState.validation.workspace_test_count = 99999991
    $staleDocsState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $staleDocsStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage 'is missing release-state value: 99999991' -Command {
        & $checker -StateFile $staleDocsStatePath -HeadRef HEAD
    }

    # A blocker may not be promoted to PASS unless its evidence says PASS.
    $falsePassStatePath = Join-Path $tempRoot 'false-blocker-pass-state.json'
    $falsePassState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $mixedDpi = @($falsePassState.external_blockers | Where-Object { $_.name -eq 'mixed_dpi' } | Select-Object -First 1)
    if ($mixedDpi.Count -ne 1) { throw 'mixed_dpi blocker missing from release state' }
    $mixedDpi[0].expected_result = 'PASS'
    $falsePassState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $falsePassStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "external blocker 'mixed_dpi' expected_result must be BLOCKED" -Command {
        & $checker -StateFile $falsePassStatePath -HeadRef HEAD
    }

    # Blocker identity must stay bound to its exact evidence path.
    $swappedEvidenceStatePath = Join-Path $tempRoot 'swapped-blocker-evidence-state.json'
    $swappedEvidenceState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $swappedMixedDpi = @($swappedEvidenceState.external_blockers | Where-Object { $_.name -eq 'mixed_dpi' } | Select-Object -First 1)
    $completedWebResolver = @($swappedEvidenceState.completed_external_gates | Where-Object { $_.name -eq 'web_resolver' } | Select-Object -First 1)
    if ($swappedMixedDpi.Count -ne 1 -or $completedWebResolver.Count -ne 1) { throw 'required mixed_dpi/web_resolver entries missing from release state' }
    $swappedMixedDpi[0].evidence = [string]$completedWebResolver[0].evidence
    $swappedEvidenceState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $swappedEvidenceStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "external blocker 'mixed_dpi' evidence path mismatch" -Command {
        & $checker -StateFile $swappedEvidenceStatePath -HeadRef HEAD
    }

    # The evidence path may remain unchanged, but the exact Git blob must stay sealed.
    & git read-tree HEAD
    if ($LASTEXITCODE -ne 0) { throw "git read-tree failed before blocker evidence probe with exit code $LASTEXITCODE" }

    $evidencePayload = Join-Path $tempRoot 'mixed-dpi-evidence-tamper.json'
    Set-Content -LiteralPath $evidencePayload -Value '{"schema":1,"result":"BLOCKED","reason":"synthetic tamper"}' -Encoding UTF8
    $tamperedEvidenceBlob = (& git hash-object -w $evidencePayload).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceBlob -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI evidence blob'
    }

    & git update-index --add --cacheinfo "100644,$tamperedEvidenceBlob,docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json"
    if ($LASTEXITCODE -ne 0) { throw "git update-index failed for mixed-DPI evidence probe with exit code $LASTEXITCODE" }

    $tamperedEvidenceTree = (& git write-tree).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceTree -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI evidence tree'
    }

    $tamperedEvidenceCommit = (& git commit-tree $tamperedEvidenceTree -p $parent -m 'release-state blocker evidence tamper self-test').Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedEvidenceCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic mixed-DPI evidence commit'
    }

    Assert-ExpectedFailure -ExpectedMessage "external blocker 'mixed_dpi' evidence blob mismatch" -Command {
        & $checker -StateFile $statePath -HeadRef $tamperedEvidenceCommit
    }
    # A completed external gate may not be demoted or detached from its sealed PASS evidence.
    $falseCompletedStatePath = Join-Path $tempRoot 'false-completed-gate-state.json'
    $falseCompletedState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $smartScreenGate = @($falseCompletedState.completed_external_gates | Where-Object { $_.name -eq 'smartscreen' } | Select-Object -First 1)
    if ($smartScreenGate.Count -ne 1) { throw 'SmartScreen completed gate missing from release state' }
    $smartScreenGate[0].expected_result = 'BLOCKED'
    $falseCompletedState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $falseCompletedStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "completed external gate 'smartscreen' expected_result must be PASS" -Command {
        & $checker -StateFile $falseCompletedStatePath -HeadRef HEAD
    }

    & git read-tree HEAD
    if ($LASTEXITCODE -ne 0) { throw "git read-tree failed before completed-gate evidence probe with exit code $LASTEXITCODE" }

    $completedEvidencePayload = Join-Path $tempRoot 'smartscreen-pass-evidence-tamper.json'
    Set-Content -LiteralPath $completedEvidencePayload -Value '{"schema":1,"result":"PASS","reason":"synthetic tamper"}' -Encoding UTF8
    $tamperedCompletedEvidenceBlob = (& git hash-object -w $completedEvidencePayload).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedCompletedEvidenceBlob -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic SmartScreen PASS evidence blob'
    }

    & git update-index --add --cacheinfo "100644,$tamperedCompletedEvidenceBlob,docs/evidence/smartscreen-physical-pass-f322126-20261002.json"
    if ($LASTEXITCODE -ne 0) { throw "git update-index failed for SmartScreen completed-gate evidence probe with exit code $LASTEXITCODE" }

    $tamperedCompletedEvidenceTree = (& git write-tree).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedCompletedEvidenceTree -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic SmartScreen completed-gate evidence tree'
    }

    $tamperedCompletedEvidenceCommit = (& git commit-tree $tamperedCompletedEvidenceTree -p $parent -m 'release-state completed gate evidence tamper self-test').Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedCompletedEvidenceCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic SmartScreen completed-gate evidence commit'
    }

    Assert-ExpectedFailure -ExpectedMessage "completed external gate 'smartscreen' evidence blob mismatch" -Command {
        & $checker -StateFile $statePath -HeadRef $tamperedCompletedEvidenceCommit
    }

    # Core release evidence must also stay byte-for-byte bound to the checked Git tree.
    & git read-tree HEAD
    if ($LASTEXITCODE -ne 0) { throw "git read-tree failed before package evidence probe with exit code $LASTEXITCODE" }

    $packageEvidencePayload = Join-Path $tempRoot 'package-evidence-tamper.json'
    Set-Content -LiteralPath $packageEvidencePayload -Value '{"schema":1,"result":"PASS","reason":"synthetic tamper"}' -Encoding UTF8
    $tamperedPackageEvidenceBlob = (& git hash-object -w $packageEvidencePayload).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedPackageEvidenceBlob -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic package evidence blob'
    }

    $packageEvidenceRelativePath = [string]((Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json).package.evidence)
    if ([string]::IsNullOrWhiteSpace($packageEvidenceRelativePath)) {
        throw 'package evidence path missing from release state'
    }
    & git update-index --add --cacheinfo "100644,$tamperedPackageEvidenceBlob,$packageEvidenceRelativePath"
    if ($LASTEXITCODE -ne 0) { throw "git update-index failed for package evidence probe with exit code $LASTEXITCODE" }

    $tamperedPackageEvidenceTree = (& git write-tree).Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedPackageEvidenceTree -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic package evidence tree'
    }

    $tamperedPackageEvidenceCommit = (& git commit-tree $tamperedPackageEvidenceTree -p $parent -m 'release-state package evidence tamper self-test').Trim()
    if ($LASTEXITCODE -ne 0 -or $tamperedPackageEvidenceCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'failed to create synthetic package evidence commit'
    }

    Assert-ExpectedFailure -ExpectedMessage 'package release-gate evidence blob mismatch' -Command {
        & $checker -StateFile $statePath -HeadRef $tamperedPackageEvidenceCommit
    }
    # Required structural sets may not silently shrink or broaden.
    $missingBlockerStatePath = Join-Path $tempRoot 'missing-required-blocker-state.json'
    $missingBlockerState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $missingBlockerState.external_blockers = @($missingBlockerState.external_blockers | Where-Object { $_.name -ne 'mixed_dpi' })
    $missingBlockerState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $missingBlockerStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'external_blockers missing required value: mixed_dpi' -Command {
        & $checker -StateFile $missingBlockerStatePath -HeadRef HEAD
    }

    $missingCompletedStatePath = Join-Path $tempRoot 'missing-required-completed-gate-state.json'
    $missingCompletedState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $missingCompletedState.completed_external_gates = @($missingCompletedState.completed_external_gates | Where-Object { $_.name -ne 'defender' })
    $missingCompletedState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $missingCompletedStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'completed_external_gates missing required value: defender' -Command {
        & $checker -StateFile $missingCompletedStatePath -HeadRef HEAD
    }

    $missingSyncDocStatePath = Join-Path $tempRoot 'missing-required-sync-doc-state.json'
    $missingSyncDocState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $missingSyncDocState.synchronized_documents = @($missingSyncDocState.synchronized_documents | Where-Object { $_ -ne 'docs/VALIDATION.md' })
    $missingSyncDocState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $missingSyncDocStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'synchronized_documents missing required value: docs/VALIDATION.md' -Command {
        & $checker -StateFile $missingSyncDocStatePath -HeadRef HEAD
    }

    $unsafeAllowStatePath = Join-Path $tempRoot 'unsafe-allow-prefix-state.json'
    $unsafeAllowState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $unsafeAllowState.package.allowed_post_package_paths = @('.github/', 'docs/', '')
    $unsafeAllowState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $unsafeAllowStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'package.allowed_post_package_paths contains unexpected value:' -Command {
        & $checker -StateFile $unsafeAllowStatePath -HeadRef HEAD
    }

    $missingTransientStatePath = Join-Path $tempRoot 'missing-transient-path-state.json'
    $missingTransientState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $missingTransientState.package.transient_validation_paths = @()
    $missingTransientState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $missingTransientStatePath -Encoding UTF8
    Assert-ExpectedFailure -ExpectedMessage 'package.transient_validation_paths missing required value: .github/workflows/pr15-release-gate.yml' -Command {
        & $checker -StateFile $missingTransientStatePath -HeadRef HEAD
    }

    # Every synchronized document must carry every blocker evidence marker.
    $currentState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $webResolverEvidence = [string](@($currentState.completed_external_gates | Where-Object { $_.name -eq 'web_resolver' } | Select-Object -First 1).evidence)
    if ([string]::IsNullOrWhiteSpace($webResolverEvidence)) { throw 'web_resolver evidence marker missing from release state' }

    $validationDocPath = Join-Path $root 'docs\VALIDATION.md'
    $validationDocBytes = [IO.File]::ReadAllBytes($validationDocPath)
    $validationDocText = [Text.Encoding]::UTF8.GetString($validationDocBytes)
    if (-not $validationDocText.Contains($webResolverEvidence)) {
        throw 'VALIDATION.md does not contain the Web Resolver evidence marker before self-test'
    }

    try {
        $mutatedValidationDoc = $validationDocText.Replace($webResolverEvidence, '')
        [IO.File]::WriteAllText($validationDocPath, $mutatedValidationDoc, [Text.UTF8Encoding]::new($false))
        Assert-ExpectedFailure -ExpectedMessage "docs/VALIDATION.md is missing release-state value: $webResolverEvidence" -Command {
            & $checker -StateFile $statePath -HeadRef HEAD
        }
    } finally {
        [IO.File]::WriteAllBytes($validationDocPath, $validationDocBytes)
    }

    [ordered]@{
        schema = 1
        result = 'PASS'
        positive_control = 'PASS'
        synthetic_packaged_input_change_rejected = $true
        stale_package_sha_rejected = $true
        stale_workspace_test_count_rejected = $true
        false_blocker_pass_rejected = $true
        swapped_blocker_evidence_rejected = $true
        tampered_blocker_evidence_blob_rejected = $true
        false_completed_gate_result_rejected = $true
        tampered_completed_gate_evidence_blob_rejected = $true
        tampered_package_evidence_blob_rejected = $true
        missing_required_blocker_rejected = $true
        missing_required_completed_gate_rejected = $true
        missing_required_sync_doc_rejected = $true
        unsafe_allowed_prefix_rejected = $true
        missing_required_transient_path_rejected = $true
        missing_blocker_evidence_marker_rejected = $true
        synthetic_commit = $probeCommit
    } | ConvertTo-Json -Depth 4
} finally {
    Pop-Location
    $env:GIT_INDEX_FILE = $oldIndexFile
    $env:GIT_AUTHOR_NAME = $oldAuthorName
    $env:GIT_AUTHOR_EMAIL = $oldAuthorEmail
    $env:GIT_COMMITTER_NAME = $oldCommitterName
    $env:GIT_COMMITTER_EMAIL = $oldCommitterEmail
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
