[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Package
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$resolved = (Resolve-Path -LiteralPath $Package).Path
$temp = $null
$root = $resolved
try {
    if ([IO.Path]::GetExtension($resolved) -ieq '.zip') {
        $temp = Join-Path ([IO.Path]::GetTempPath()) ('search-tool-package-verify-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $temp | Out-Null
        Expand-Archive -LiteralPath $resolved -DestinationPath $temp -Force
        $root = $temp
    }

    $manifest = Join-Path $root 'MANIFEST.sha256'
    if (-not (Test-Path -LiteralPath $manifest)) { throw 'MANIFEST.sha256 is missing.' }
    $required = @(
        'search-tool.exe',
        'search-tool-gui.exe',
        'search-tool-service.exe',
        'search-tool-worker.exe',
        'search-tool-bench.exe',
        'install.ps1',
        'uninstall.ps1',
        'low-end-benchmark.ps1',
        'windows-soak.ps1',
        'journal-reset-recovery.ps1',
        'defender-check.ps1',
        'smartscreen-validation.ps1',
        'display-validation.ps1',
        'pristine-validation.ps1',
        'physical-validation.ps1',
        'foreground-impact.ps1',
        'install-smoke.ps1',
        'models\tiny-intent-v1.stm',
        'README.md'
    )
    foreach ($name in $required) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $name))) { throw "Package file missing: $name" }
    }

    $listed = @{}
    foreach ($line in Get-Content -LiteralPath $manifest) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -notmatch '^([0-9a-fA-F]{64})\s\s(.+)$') { throw "Malformed manifest line: $line" }
        $expected = $matches[1].ToLowerInvariant()
        $relative = $matches[2]
        $path = Join-Path $root $relative
        if (-not (Test-Path -LiteralPath $path)) { throw "Manifest file missing: $relative" }
        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $expected) { throw "SHA-256 mismatch: $relative" }
        $listed[$relative.ToLowerInvariant()] = $true
    }

    $actualFiles = @(Get-ChildItem -LiteralPath $root -File -Recurse |
        Where-Object { $_.FullName -ne $manifest } |
        ForEach-Object { $_.FullName.Substring($root.Length).TrimStart('\') })
    foreach ($relative in $actualFiles) {
        if (-not $listed.ContainsKey($relative.ToLowerInvariant())) { throw "Unlisted package file: $relative" }
    }

    & (Join-Path $root 'search-tool.exe') version | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Packaged CLI smoke failed: $LASTEXITCODE" }
    & (Join-Path $root 'search-tool.exe') status | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Packaged CLI status failed: $LASTEXITCODE" }
    & (Join-Path $root 'search-tool-bench.exe') 10000 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Packaged benchmark smoke failed: $LASTEXITCODE" }

    Write-Host 'PACKAGE_VERIFY=PASS'
} finally {
    if ($temp) { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
}
