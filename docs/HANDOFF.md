# Search Tool - Project Handoff

> Authoritative continuation note. Last updated: 2026-09-28.

This document exists so development can continue from the repository without needing the original ChatGPT conversation.

## Project goal

Search Tool is an ultra-light native Windows file search and safe maintenance utility designed for low-end systems. The reference target is a Celeron-class CPU, 4 GB RAM and a mechanical HDD.

Core rules:
- Rust-heavy native implementation; no Electron/Chromium/JVM/Node runtime.
- Initial NTFS indexing from MFT, incremental updates from USN Journal.
- Disk-first indexes and sparse checkpoints to keep idle RAM tiny.
- Search first, AI/web only as late optional fallbacks.
- Cleanup is separated from search and uses quarantine before permanent purge.
- System, user-data and unknown files are fail-closed for cleanup.

## Repository structure

- `crates/search-core`: index formats, live search, compaction, metadata/content, cleanup safety, duplicates, AI routing, web cache.
- `crates/search-platform-windows`: NTFS/USN, Windows process/background mode, WinHTTP.
- `apps/search-tool-cli`: indexing/search/maintenance/diagnostics commands.
- `apps/search-tool-gui`: native Win32 UI and resident launcher.
- `apps/search-tool-service`: Windows SCM service, USN sync and idle maintenance.
- `apps/search-tool-worker`: isolated rich-document parser with IFilter/fallback parsers.
- `apps/search-tool-bench`: fresh-process scale/RSS/query benchmark.
- `scripts`: packaging, install/uninstall, soak, journal recovery and release gates.
- `.github`: Windows CI, soak and release-gate workflows.
- `models/tiny-intent-v1.stm`: ~1 MiB INT8 intent model.

## What is implemented

MFT initial index, USN incremental sync, checkpoint recovery, bounded delta overlay, external bounded-memory compaction, multi-volume search, exact/prefix/ranked/fuzzy/relationship search, filters, path reconstruction, metadata sidecars, content index, rich document worker, duplicate verification, quarantine/restore/purge, tiny-AI natural-language routing, privacy-sanitized optional web resolver, native GUI, Windows service, resource governor, install/package scripts, repair/maintain/doctor and Windows validation harnesses are implemented.

## Last verified release gate

On 2026-09-28 commit `385a971` passed the full Windows release gate on a physical Windows x64 machine:

- release preflight: PASS
- cargo fmt: PASS
- cargo clippy with -D warnings: PASS
- cargo test: PASS
- release build: PASS
- CLI smoke: PASS
- NTFS/MFT/USN/service VHD integration: PASS
- USN journal reset recovery: PASS
- portable package build: PASS
- package integrity: PASS
- clean install/uninstall smoke: PASS
- Defender interaction step: PASS with Defender reported unavailable/disabled on that host

After the compaction crash-consistency changes, local verification is also green: 68 + 7 + 3 + 2 = 80 workspace tests, 0 failures; cargo fmt, workspace clippy with `-D warnings`, and workspace release build PASS. The full physical release gate must be rerun after the current source changes before its package SHA is treated as current.

Generated package SHA-256 after the soak-ownership hardening release gate:
`CBE38CDE38E1427AF11E6CEA1B5E8EAE1077715F83FF6CBD71072A555B19602D`

The ZIP itself is intentionally not tracked in Git; recreate it with `scripts/package.ps1`.

## Important physical Windows results

- Real C: index has exceeded 1.2 million records and opens/verifies successfully.
- Real D:/E: multi-volume test indexes around 100k + 70k records were exercised.
- 140k+ delta compaction was exercised successfully.
- Cross-volume duplicate detection was exercised.
- Quarantine -> restore -> purge was exercised.
- Journal reset recovery was exercised only on isolated temporary NTFS VHDs.
- Native GUI resident mode, single-instance behavior and Ctrl+Alt+Space fallback hotkey were exercised.
- Rich document extraction works for DOCX, XLSX, PPTX and PDF using IFilter and/or built-in fallback parsing.
- 15-minute soak passed twice with crash/restart; one representative run: ~914 s, 14,352 operations, 598 validation checks, peak service working set ~5.2 MiB.
- Foreground-impact test passed; representative p95 changed from 23.838 ms to 25.131 ms under mutation load.

## Current unfinished lab state

Do not mistake a dirty validation index for a product failure. At the latest checkpoint:
- D: was recovered through supported USN sync/compact/maintain semantics and returned to delta=0 with metadata/sizes/content fresh and verify-deep PASS.
- The original long-soak ownership failure is fixed. A later 60-minute attempt then ran 865.48 s / 9,984 operations / 416 checks before exposing a second harness race: periodic fast `verify` could collide with the service mutation lock. Its `result=FAIL` JSON was preserved.
- `windows-soak.ps1` now applies the same bounded retry only to the exact "index mutation is already in progress" condition for both `verify` and `verify-deep`; all other verification failures remain fail-fast.
- A high-load 1-minute regression after that fix passed: 65.84 s, 4,992 operations, 26 checks, crash/restart exercised. A fresh complete 60-minute PASS JSON on the committed current source is still required.
- E: remains clean/fresh from prior validation.
- C: opens and verify-deep passes at 1,427,984 records with delta=0; metadata/sizes are fresh and content is stale. An interrupted content build left staging files, but a new supported content build owns the build lock and cleans those stale staging files itself.
- Compaction publish/swap fault injection is complete. Eleven abrupt child-process exit boundaries are covered; the test exposed and fixed a mixed-generation recovery bug by making absence/presence of the main file the rollback/commit bit.
- SearchToolIndexer is currently absent after clean install/uninstall validation. Reinstall it only after the intended real C: index is fully refreshed.

## Immediate continuation order

1. Commit the current compaction + soak-harness fixes, then rerun isolated D: for a complete 60-minute crash/restart `result=PASS` JSON; clean/verify D: afterward.
2. Refresh the real 1.427M-record C: content sidecar through supported build semantics and finish doctor/verify-deep.
3. Reinstall/start SearchToolIndexer against that recovered real C: index and verify automatic USN sync.
4. Run controlled sleep -> resume validation using a pre/post marker and checkpoint comparison.
5. Run controlled reboot validation and verify SCM auto-start, USN catch-up and marker continuity.
6. Finish hostile parser-worker input matrix.
7. Run final performance matrix and final package/release gate once no source changes remain.

For the full backlog see `docs/ROADMAP.md`. For evidence and exact PASS/blocked states see `docs/TEST_MATRIX.md`.

## Safe resume rule

Before mutating a real index, always run:
`search-tool doctor <index-dir>`
and inspect whether an SCM SearchToolIndexer instance is running. `maintain` intentionally refuses unsafe concurrent mutation. Use isolated VHDs for destructive USN/journal recovery tests.
