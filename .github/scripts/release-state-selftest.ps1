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

# Positive control: current sealed state must pass first.
& $checker -StateFile $statePath -HeadRef HEAD | Out-Null

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

    # A blocker may not be promoted to PASS unless its evidence says PASS.
    $falsePassStatePath = Join-Path $tempRoot 'false-blocker-pass-state.json'
    $falsePassState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $defender = @($falsePassState.external_blockers | Where-Object { $_.name -eq 'defender' } | Select-Object -First 1)
    if ($defender.Count -ne 1) { throw 'defender blocker missing from release state' }
    $defender[0].expected_result = 'PASS'
    $falsePassState | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $falsePassStatePath -Encoding UTF8

    Assert-ExpectedFailure -ExpectedMessage "external blocker 'defender' result mismatch" -Command {
        & $checker -StateFile $falsePassStatePath -HeadRef HEAD
    }

    [ordered]@{
        schema = 1
        result = 'PASS'
        positive_control = 'PASS'
        synthetic_packaged_input_change_rejected = $true
        stale_package_sha_rejected = $true
        false_blocker_pass_rejected = $true
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
