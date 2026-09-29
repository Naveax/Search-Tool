[CmdletBinding()]
param(
    [string]$OutputDir = (Join-Path $PSScriptRoot '..\dist')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$OutputDir = if ([IO.Path]::IsPathRooted($OutputDir)) {
    [IO.Path]::GetFullPath($OutputDir)
} else {
    [IO.Path]::GetFullPath((Join-Path $root $OutputDir))
}
Push-Location $root
try {
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build --workspace --release

    $stage = Join-Path $OutputDir 'SearchTool'
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    foreach ($name in @('search-tool.exe','search-tool-gui.exe','search-tool-service.exe','search-tool-worker.exe','search-tool-bench.exe')) {
        Copy-Item -LiteralPath (Join-Path $root "target\release\$name") -Destination $stage
    }
    $modelSource = Join-Path $root 'models\tiny-intent-v1.stm'
    if (-not (Test-Path -LiteralPath $modelSource)) { throw "Missing required tiny intent model: $modelSource" }
    New-Item -ItemType Directory -Force -Path (Join-Path $stage 'models') | Out-Null
    Copy-Item -LiteralPath $modelSource -Destination (Join-Path $stage 'models')
    foreach ($script in @('install.ps1','uninstall.ps1','low-end-benchmark.ps1','windows-soak.ps1','journal-reset-recovery.ps1','defender-check.ps1','web-resolver-validation.ps1','smartscreen-validation.ps1','display-validation.ps1','pristine-validation.ps1','verify-package.ps1','physical-validation.ps1','foreground-impact.ps1','install-smoke.ps1')) {
        Copy-Item -LiteralPath (Join-Path $root "scripts\$script") -Destination $stage
    }
    Copy-Item -LiteralPath (Join-Path $root 'README.md') -Destination $stage

    $manifest = Join-Path $stage 'MANIFEST.sha256'
    Get-ChildItem -LiteralPath $stage -File -Recurse |
        Where-Object { $_.FullName -ne $manifest } |
        Sort-Object FullName |
        ForEach-Object {
            $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $relative = $_.FullName.Substring($stage.Length).TrimStart('\')
            "$hash  $relative"
        } | Set-Content -LiteralPath $manifest -Encoding ASCII

    $zip = Join-Path $OutputDir 'SearchTool-Windows-x64.zip'
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -CompressionLevel Optimal
    Write-Host "Package=$zip"
} finally {
    Pop-Location
}
