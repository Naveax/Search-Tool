[CmdletBinding()]
param(
    [string]$StateFile = (Join-Path $PSScriptRoot '..\..\docs\RELEASE_STATE.json'),
    [string]$HeadRef = 'HEAD'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Assert-ReleaseState {
    param(
        [Parameter(Mandatory)] [bool]$Condition,
        [Parameter(Mandatory)] [string]$Message
    )
    if (-not $Condition) {
        throw "RELEASE_STATE_FAIL: $Message"
    }
}

function Resolve-RepoPath {
    param([Parameter(Mandatory)] [string]$RelativePath)

    Assert-ReleaseState (-not [string]::IsNullOrWhiteSpace($RelativePath)) 'empty repository-relative path'
    $full = [IO.Path]::GetFullPath((Join-Path $root $RelativePath))
    $rootPrefix = $root.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    Assert-ReleaseState (
        $full.Equals($root, [StringComparison]::OrdinalIgnoreCase) -or
        $full.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)
    ) "path escapes repository root: $RelativePath"
    return $full
}

function Read-JsonFile {
    param([Parameter(Mandatory)] [string]$RelativePath)

    $full = Resolve-RepoPath $RelativePath
    Assert-ReleaseState (Test-Path -LiteralPath $full -PathType Leaf) "missing JSON evidence: $RelativePath"
    return (Get-Content -LiteralPath $full -Raw | ConvertFrom-Json)
}

function Assert-Sha {
    param(
        [Parameter(Mandatory)] [string]$Value,
        [Parameter(Mandatory)] [string]$Name
    )
    Assert-ReleaseState ($Value -match '^[0-9a-fA-F]{40}$') "$Name is not a 40-character Git SHA"
}

function Assert-ExactStringSet {
    param(
        [Parameter(Mandatory)] [AllowEmptyCollection()] [object[]]$Actual,
        [Parameter(Mandatory)] [AllowEmptyCollection()] [string[]]$Expected,
        [Parameter(Mandatory)] [string]$Name
    )

    $actualValues = @($Actual | ForEach-Object { [string]$_ })
    foreach ($expectedValue in $Expected) {
        Assert-ReleaseState ($actualValues -ccontains $expectedValue) "$Name missing required value: $expectedValue"
    }
    foreach ($actualValue in $actualValues) {
        Assert-ReleaseState ($Expected -ccontains $actualValue) "$Name contains unexpected value: $actualValue"
    }
    Assert-ReleaseState ($actualValues.Count -eq $Expected.Count) "$Name contains duplicate values"
}
function Assert-HeadBlobSha {
    param(
        [Parameter(Mandatory)] [string]$RelativePath,
        [Parameter(Mandatory)] [string]$ExpectedBlobSha,
        [Parameter(Mandatory)] [string]$Name
    )

    Assert-Sha $ExpectedBlobSha "$Name expected blob SHA"
    $objectSpec = "${HeadRef}:$RelativePath"
    $actualBlobSha = (& git -C $root rev-parse $objectSpec 2>$null).Trim()
    Assert-ReleaseState ($LASTEXITCODE -eq 0 -and $actualBlobSha -match '^[0-9a-fA-F]{40}$') "$Name blob is unavailable at head ref: $HeadRef"
    Assert-ReleaseState ($actualBlobSha -eq $ExpectedBlobSha) "$Name blob mismatch"
}

$statePath = [IO.Path]::GetFullPath($StateFile)
Assert-ReleaseState (Test-Path -LiteralPath $statePath -PathType Leaf) "missing release state file: $statePath"
$state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json

Assert-ReleaseState ([int]$state.schema -eq 1) 'unsupported release-state schema'
$status = ([string]$state.package.status).ToUpperInvariant()
Assert-ReleaseState ($status -in @('VALIDATED', 'INVALIDATED')) "unsupported package status: $status"

if ($status -eq 'INVALIDATED') {
    $reasonProperty = $state.package.PSObject.Properties['invalidated_reason']
    Assert-ReleaseState (
        $null -ne $reasonProperty -and
        -not [string]::IsNullOrWhiteSpace([string]$reasonProperty.Value)
    ) 'INVALIDATED package state requires package.invalidated_reason'

    [ordered]@{
        schema = 1
        result = 'PASS'
        package_status = 'INVALIDATED'
        reason = [string]$reasonProperty.Value
    } | ConvertTo-Json -Depth 4
    return
}

$requiredExternalBlockers = @('defender', 'smartscreen', 'mixed_dpi', 'web_resolver')
$requiredSynchronizedDocuments = @('docs/HANDOFF.md', 'docs/STATUS.md', 'docs/ROADMAP.md', 'docs/TEST_MATRIX.md', 'docs/VALIDATION.md')
$requiredAllowedPostPackagePaths = @('.github/', 'docs/')
$requiredTransientValidationPaths = @('.github/workflows/pr15-release-gate.yml')

Assert-ExactStringSet -Actual @($state.external_blockers | ForEach-Object { [string]$_.name }) -Expected $requiredExternalBlockers -Name 'external_blockers'
Assert-ExactStringSet -Actual @($state.synchronized_documents) -Expected $requiredSynchronizedDocuments -Name 'synchronized_documents'
Assert-ExactStringSet -Actual @($state.package.allowed_post_package_paths) -Expected $requiredAllowedPostPackagePaths -Name 'package.allowed_post_package_paths'
Assert-ExactStringSet -Actual @($state.package.transient_validation_paths) -Expected $requiredTransientValidationPaths -Name 'package.transient_validation_paths'
$packagedSource = [string]$state.package.packaged_source_sha
$packageSha256 = [string]$state.package.sha256
$packageBytes = [int64]$state.package.bytes
Assert-Sha $packagedSource 'package.packaged_source_sha'
Assert-ReleaseState ($packageSha256 -match '^[0-9a-fA-F]{64}$') 'package.sha256 is not a SHA-256'
Assert-ReleaseState ($packageBytes -gt 0) 'package.bytes must be positive'

$requiredPackageEvidencePath = 'docs/evidence/windows-release-gate-pr15-display-validation-20261001.json'
$requiredPackageEvidenceBlobSha = '9bf0fea273b90ac2ba3f164a2ed550cd8cf57294'
Assert-ReleaseState ([string]$state.package.evidence -eq $requiredPackageEvidencePath) 'package evidence path mismatch'
Assert-Sha ([string]$state.package.evidence_blob_sha) 'package.evidence_blob_sha'
Assert-ReleaseState ([string]$state.package.evidence_blob_sha -eq $requiredPackageEvidenceBlobSha) 'package evidence blob SHA mismatch'
Assert-HeadBlobSha -RelativePath $requiredPackageEvidencePath -ExpectedBlobSha $requiredPackageEvidenceBlobSha -Name 'package release-gate evidence'

$packageEvidence = Read-JsonFile ([string]$state.package.evidence)
Assert-ReleaseState ([string]$packageEvidence.result -eq 'PASS') 'package release-gate evidence is not PASS'
Assert-ReleaseState (
    [string]$packageEvidence.source.packaged_source_sha -eq $packagedSource
) 'package evidence packaged_source_sha mismatch'
Assert-ReleaseState ([bool]$packageEvidence.source.package_input_equivalence) 'package evidence does not assert package-input equivalence'
Assert-ReleaseState (
    [int64]$packageEvidence.source.exact_head_ci.run_id -eq [int64]$state.package.exact_head_ci_run_id
) 'exact-head CI run id mismatch'
Assert-ReleaseState (
    [string]$packageEvidence.source.exact_head_ci.result -eq 'SUCCESS'
) 'exact-head CI evidence is not SUCCESS'
Assert-ReleaseState (
    [int64]$packageEvidence.hosted_release_gate.run_id -eq [int64]$state.package.release_gate_run_id
) 'release-gate run id mismatch'
Assert-ReleaseState (
    [string]$packageEvidence.hosted_release_gate.result -eq 'SUCCESS' -and
    [string]$packageEvidence.hosted_release_gate.summary_result -eq 'PASS'
) 'release-gate evidence is not SUCCESS/PASS'
Assert-ReleaseState (
    [string]$packageEvidence.package.sha256 -ieq $packageSha256
) 'package SHA-256 mismatch'
Assert-ReleaseState (
    [int64]$packageEvidence.package.bytes -eq $packageBytes
) 'package byte-size mismatch'
Assert-ReleaseState ([bool]$packageEvidence.package.seal_rehash_pass) 'artifact seal re-hash is not PASS'

$requiredPhysicalEvidencePath = 'docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json'
$requiredPhysicalEvidenceBlobSha = 'dd104790f6c244050e175bb2f8a6d6cd8d1dfac6'
Assert-ReleaseState ([string]$state.runtime.physical_gate_evidence -eq $requiredPhysicalEvidencePath) 'physical runtime gate evidence path mismatch'
Assert-Sha ([string]$state.runtime.physical_gate_evidence_blob_sha) 'runtime.physical_gate_evidence_blob_sha'
Assert-ReleaseState ([string]$state.runtime.physical_gate_evidence_blob_sha -eq $requiredPhysicalEvidenceBlobSha) 'physical runtime gate evidence blob SHA mismatch'
Assert-HeadBlobSha -RelativePath $requiredPhysicalEvidencePath -ExpectedBlobSha $requiredPhysicalEvidenceBlobSha -Name 'physical runtime gate evidence'

$physicalEvidence = Read-JsonFile ([string]$state.runtime.physical_gate_evidence)
$physicalSource = [string]$state.runtime.physical_gate_source_sha
Assert-Sha $physicalSource 'runtime.physical_gate_source_sha'
Assert-ReleaseState ([string]$physicalEvidence.result -eq 'PASS') 'physical runtime gate evidence is not PASS'
Assert-ReleaseState (
    [string]$physicalEvidence.source.head_sha -eq $physicalSource
) 'physical runtime gate source mismatch'
Assert-ReleaseState (
    [string]$physicalEvidence.source.exact_head_ci.result -eq 'SUCCESS'
) 'physical runtime gate exact-head CI is not SUCCESS'

$requiredSoakEvidencePath = 'docs/evidence/soak-6h-fa92628-final-20260930.json'
$requiredSoakEvidenceBlobSha = 'abcc1e0b9acf45d053cd32e8c183abefa6d172e6'
Assert-ReleaseState ([string]$state.runtime.six_hour_soak_evidence -eq $requiredSoakEvidencePath) 'six-hour soak evidence path mismatch'
Assert-Sha ([string]$state.runtime.six_hour_soak_evidence_blob_sha) 'runtime.six_hour_soak_evidence_blob_sha'
Assert-ReleaseState ([string]$state.runtime.six_hour_soak_evidence_blob_sha -eq $requiredSoakEvidenceBlobSha) 'six-hour soak evidence blob SHA mismatch'
Assert-HeadBlobSha -RelativePath $requiredSoakEvidencePath -ExpectedBlobSha $requiredSoakEvidenceBlobSha -Name 'six-hour soak evidence'

$soakEvidence = Read-JsonFile ([string]$state.runtime.six_hour_soak_evidence)
$soakSource = [string]$state.runtime.six_hour_soak_source_sha
Assert-Sha $soakSource 'runtime.six_hour_soak_source_sha'
Assert-ReleaseState ([string]$soakEvidence.result -eq 'PASS') 'six-hour soak wrapper is not PASS'
Assert-ReleaseState ([string]$soakEvidence.source_sha -eq $soakSource) 'six-hour soak source mismatch'
Assert-ReleaseState ([string]$soakEvidence.soak.result -eq 'PASS') 'six-hour soak payload is not PASS'
Assert-ReleaseState ([bool]$soakEvidence.soak.crash_restart_exercised) 'six-hour soak did not exercise crash/restart'
Assert-ReleaseState ([bool]$soakEvidence.source_freeze.pass) 'six-hour soak source freeze is not sealed'
Assert-ReleaseState ([bool]$soakEvidence.service_identity.pass) 'six-hour soak service identity is not sealed'

$requiredExternalBlockerEvidence = @{
    defender = @{
        path = 'docs/evidence/defender-hosted-blocked-20261001.json'
        blob_sha = '332869529cf3b770c2d97f70ffbbfd416c6bd63f'
    }
    smartscreen = @{
        path = 'docs/evidence/smartscreen-hosted-blocked-20261001.json'
        blob_sha = '74b3e16bc372070cca2ce3a83e4e617d9681b627'
    }
    mixed_dpi = @{
        path = 'docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json'
        blob_sha = 'a1c0c329a1024ab02948361b9f8102e069f0db95'
    }
    web_resolver = @{
        path = 'docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json'
        blob_sha = 'ed3d9b56fc75e7d56620e639917988882c732550'
    }
}

foreach ($blocker in @($state.external_blockers)) {
    $blockerName = [string]$blocker.name
    $requiredEvidence = $requiredExternalBlockerEvidence[$blockerName]
    Assert-ReleaseState ($null -ne $requiredEvidence) "external blocker '$blockerName' has no required evidence mapping"

    $expectedEvidencePath = [string]$requiredEvidence.path
    $expectedEvidenceBlobSha = [string]$requiredEvidence.blob_sha
    Assert-ReleaseState ([string]$blocker.evidence -eq $expectedEvidencePath) "external blocker '$blockerName' evidence path mismatch"
    Assert-ReleaseState ([string]$blocker.expected_result -eq 'BLOCKED') "external blocker '$blockerName' expected_result must be BLOCKED"
    Assert-Sha ([string]$blocker.evidence_blob_sha) "external blocker '$blockerName' evidence_blob_sha"
    Assert-ReleaseState ([string]$blocker.evidence_blob_sha -eq $expectedEvidenceBlobSha) "external blocker '$blockerName' evidence blob SHA mismatch"

    Assert-HeadBlobSha -RelativePath $expectedEvidencePath -ExpectedBlobSha $expectedEvidenceBlobSha -Name "external blocker '$blockerName' evidence"

    $blockerEvidence = Read-JsonFile ([string]$blocker.evidence)
    Assert-ReleaseState ([string]$blockerEvidence.result -eq 'BLOCKED') "external blocker '$blockerName' evidence result is not BLOCKED"
}

$workspaceTestCount = [int]$state.validation.workspace_test_count
Assert-ReleaseState ($workspaceTestCount -gt 0) 'validation.workspace_test_count must be positive'

$syncNeedles = @(
    $packagedSource,
    $packageSha256,
    [string]$state.package.evidence,
    [string]$state.package.evidence_blob_sha,
    [string]$state.runtime.physical_gate_evidence,
    [string]$state.runtime.physical_gate_evidence_blob_sha,
    [string]$workspaceTestCount,
    [string]$state.runtime.six_hour_soak_evidence,
    [string]$state.runtime.six_hour_soak_evidence_blob_sha
)
$syncNeedles += @($state.external_blockers | ForEach-Object { [string]$_.evidence })
$syncNeedles += @($state.external_blockers | ForEach-Object { [string]$_.evidence_blob_sha })
$syncNeedles = @($syncNeedles | Select-Object -Unique)

foreach ($doc in @($state.synchronized_documents)) {
    $docPath = Resolve-RepoPath ([string]$doc)
    Assert-ReleaseState (Test-Path -LiteralPath $docPath -PathType Leaf) "missing synchronized document: $doc"
    $docText = Get-Content -LiteralPath $docPath -Raw
    foreach ($needle in $syncNeedles) {
        Assert-ReleaseState ($docText.Contains($needle)) "$doc is missing release-state value: $needle"
    }
}

foreach ($transient in @($state.package.transient_validation_paths)) {
    Assert-ReleaseState (
        -not (Test-Path -LiteralPath (Resolve-RepoPath ([string]$transient)))
    ) "transient validation path leaked into final tree: $transient"
}

Push-Location $root
try {
    & git cat-file -e "$packagedSource^{commit}" 2>$null
    Assert-ReleaseState ($LASTEXITCODE -eq 0) "packaged source commit is unavailable locally: $packagedSource"

    & git cat-file -e "$HeadRef^{commit}" 2>$null
    Assert-ReleaseState ($LASTEXITCODE -eq 0) "head ref is unavailable locally: $HeadRef"

    & git merge-base --is-ancestor $packagedSource $HeadRef
    Assert-ReleaseState ($LASTEXITCODE -eq 0) "packaged source is not an ancestor of head ref: $HeadRef"

    $changedFiles = @(& git diff --name-only $packagedSource $HeadRef)
    Assert-ReleaseState ($LASTEXITCODE -eq 0) "git diff against packaged source failed for head ref: $HeadRef"

    $allowedPrefixes = @($state.package.allowed_post_package_paths | ForEach-Object {
        ([string]$_).Replace('\', '/')
    })
    $invalidChanges = @()
    foreach ($path in $changedFiles) {
        $normalized = ([string]$path).Replace('\', '/')
        $allowed = $false
        foreach ($prefix in $allowedPrefixes) {
            if ($normalized.StartsWith($prefix, [StringComparison]::Ordinal)) {
                $allowed = $true
                break
            }
        }
        if (-not $allowed) {
            $invalidChanges += $normalized
        }
    }

    Assert-ReleaseState (
        $invalidChanges.Count -eq 0
    ) ("packaged inputs changed after validated source: " + ($invalidChanges -join ', '))

    $head = (& git rev-parse $HeadRef).Trim()
    Assert-ReleaseState ($LASTEXITCODE -eq 0) "failed to resolve head ref: $HeadRef"

    [ordered]@{
        schema = 1
        result = 'PASS'
        package_status = $status
        packaged_source_sha = $packagedSource
        current_head_sha = $head
        package_sha256 = $packageSha256.ToUpperInvariant()
        package_bytes = $packageBytes
        post_package_changed_files = $changedFiles.Count
        allowed_post_package_prefixes = $allowedPrefixes
        external_blockers = @($state.external_blockers | ForEach-Object {
            [ordered]@{
                name = [string]$_.name
                result = [string]$_.expected_result
                evidence = [string]$_.evidence
            }
        })
    } | ConvertTo-Json -Depth 6
} finally {
    Pop-Location
}
