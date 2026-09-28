$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Write-Host '==> rustc / cargo'
rustc --version
cargo --version

Write-Host '==> format'
cargo fmt --all -- --check

Write-Host '==> clippy'
cargo clippy --workspace --all-targets -- -D warnings

Write-Host '==> tests'
cargo test --workspace

Write-Host '==> release build'
cargo build --workspace --release

Write-Host '==> CLI smoke'
& .\target\release\search-tool.exe status
if ($LASTEXITCODE -ne 0) { throw "CLI smoke failed: $LASTEXITCODE" }
& .\target\release\search-tool.exe version
if ($LASTEXITCODE -ne 0) { throw "Version smoke failed: $LASTEXITCODE" }

Write-Host '==> synthetic 100k read-side benchmark'
& .\target\release\search-tool-bench.exe 100000
if ($LASTEXITCODE -ne 0) { throw "Benchmark smoke failed: $LASTEXITCODE" }
