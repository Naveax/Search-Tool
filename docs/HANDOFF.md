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

On 2026-09-28 the latest source tree passed the full Windows release gate on a physical Windows x64 machine:

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

Workspace test groups at that point: 65 + 7 + 3 + 2 = 77 passing tests, 0 failures.

Generated package SHA-256 at that point:
`1A3D0E629D445598C4250A843F9D9F7EE1BA1EDD2A2167BAFBBF4C36FE96EA6B`

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

Do not mistake a dirty validation index for a product failure. At the last checkpoint:
- D: test index had accumulated ~26k soak delta entries because a 60-minute soak process ended without writing its final JSON report.
- E: was clean/fresh.
- C: real index remained verify-PASS but had accumulated delta and stale metadata/content after an interrupted background content-maintenance run.
- The final release gate itself still passed on isolated VHDs, so source/release integrity is currently green.

## Immediate continuation order

1. Clean D: lab index: sync/compact/maintain, rebuild metadata/content, verify-deep, doctor.
2. Find why the 60-minute soak exited without writing `soak-60m-20260928.json`; rerun until a final PASS JSON exists.
3. Clean the real C: lab index and remove/recover stale content/compaction staging safely.
4. Reinstall/start SearchToolIndexer on C: and verify automatic USN sync.
5. Run controlled sleep -> resume validation using a pre/post marker and checkpoint comparison.
6. Run controlled reboot validation and verify SCM auto-start, USN catch-up and marker continuity.
7. Fault-inject kill during compaction commit/swap and verify recovery.
8. Finish hostile parser-worker input matrix.
9. Run final performance matrix and final package/release gate once no source changes remain.

For the full backlog see `docs/ROADMAP.md`. For evidence and exact PASS/blocked states see `docs/TEST_MATRIX.md`.

## Safe resume rule

Before mutating a real index, always run:
`search-tool doctor <index-dir>`
and inspect whether an SCM SearchToolIndexer instance is running. `maintain` intentionally refuses unsafe concurrent mutation. Use isolated VHDs for destructive USN/journal recovery tests.
