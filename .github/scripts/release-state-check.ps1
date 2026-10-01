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

$soakEvidence = Read-JsonFile ([string]$state.runtime.six_hour_soak_evidence)
$soakSource = [string]$state.runtime.six_hour_soak_source_sha
Assert-Sha $soakSource 'runtime.six_hour_soak_source_sha'
Assert-ReleaseState ([string]$soakEvidence.result -eq 'PASS') 'six-hour soak wrapper is not PASS'
Assert-ReleaseState ([string]$soakEvidence.source_sha -eq $soakSource) 'six-hour soak source mismatch'
Assert-ReleaseState ([string]$soakEvidence.soak.result -eq 'PASS') 'six-hour soak payload is not PASS'
Assert-ReleaseState ([bool]$soakEvidence.soak.crash_restart_exercised) 'six-hour soak did not exercise crash/restart'
Assert-ReleaseState ([bool]$soakEvidence.source_freeze.pass) 'six-hour soak source freeze is not sealed'
Assert-ReleaseState ([bool]$soakEvidence.service_identity.pass) 'six-hour soak service identity is not sealed'

foreach ($blocker in @($state.external_blockers)) {
    $blockerEvidence = Read-JsonFile ([string]$blocker.evidence)
    Assert-ReleaseState (
        [string]$blockerEvidence.result -eq [string]$blocker.expected_result
    ) "external blocker '$($blocker.name)' result mismatch"
}

$workspaceTestCount = [int]$state.validation.workspace_test_count
Assert-ReleaseState ($workspaceTestCount -gt 0) 'validation.workspace_test_count must be positive'

$syncNeedles = @(
    $packagedSource,
    $packageSha256,
    [string]$state.package.evidence,
    [string]$workspaceTestCount,
    [string]$state.runtime.six_hour_soak_evidence
)
$syncNeedles += @($state.external_blockers | ForEach-Object { [string]$_.evidence })
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
