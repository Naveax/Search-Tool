# Search Tool - Project Handoff

> Authoritative continuation note. Last updated: 2026-09-29.

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

Current-source local verification after power-cycle harness and Windows CI timing hardening is green: 68 + 7 + 9 + 3 + 6 = 93 workspace tests, 0 failures; cargo fmt, workspace clippy with `-D warnings`, and workspace release build PASS. The oversized-response worker regression also passed 5 consecutive targeted runs. The full physical release gate must be rerun after the current source changes before its package SHA is treated as current.

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
- Hardened 60-minute soak on commit `709cc275` passed: 3613.77 s, 56,496 operations, 2,354 validation checks, crash/restart exercised, peak service working set ~5.2 MiB. Final D: cleanup restored delta=0 and metadata/sizes/content freshness with verify-deep PASS.
- Foreground-impact test passed; representative p95 changed from 23.838 ms to 25.131 ms under mutation load.
- Real 1M+ search latency matrix passed on a frozen 1,209,697-record C: index at source head `322fb4e`. p50/p95/p99 ms: exact 27.750/32.206/39.736, prefix 61.983/67.694/68.776, fuzzy 458.443/478.590/497.988, filtered 60.065/66.107/78.889, relationship 130.016/151.995/158.008, content 145.325/155.821/184.094. Evidence: `docs/evidence/search-latency-matrix-20260929.json`.

## Current unfinished lab state

Do not mistake a dirty validation index for a product failure. At the latest checkpoint:
- D: hardened 60-minute crash/restart soak is complete on commit `709cc275`: 3613.77 s / 56,496 operations / 2,354 checks / `result=PASS`. Evidence: `docs/evidence/soak-60m-709cc275-20260928.json`.
- After the soak, D: was refreshed through supported `content-build` semantics and is clean: delta=0, pending_delta=false, metadata/sizes/content fresh and verify-deep PASS.
- E: remains clean/fresh from prior validation.
- C: stale content staging was recovered through supported `content-build` semantics. The rebuild indexed 391,281 files / 85,257,251 postings; all freshness markers were true and verify-deep passed at 1,427,984 records before service startup.
- SearchToolIndexer is installed against the real C: index, Running with Automatic start, `service_sync=Ok` and `last_error=0`. Initial catch-up/automatic compaction converged to 1,304,339 base records with delta=0; a steady-state create/rename/delete marker probe passed 4/4. Evidence: `docs/evidence/real-c-recovery-service-20260929.json`.
- For the final real-index latency capture, C: was frozen through supported compact/metadata-build/content-build steps. The measured generation has 1,209,697 base records, delta=0, pending_delta=false, metadata/sizes/content fresh, verify-deep PASS; SearchToolIndexer was then restarted Running + Automatic.
- Parser-worker hostile-input hardening is complete on commit `e2be944`: the 32 KiB path limit is aligned, the watchdog is armed before writes, protocol corruption triggers restart, oversized unread frames fail closed, the worker is constrained by a 256 MiB Windows Job Object, and deterministic hang/crash/partial-stdout/invalid-UTF8/oversized-response/corrupt-PDF/encrypted-PDF/corrupt-OOXML/8,193-entry OOXML-bomb fixtures PASS. Evidence: `docs/evidence/parser-hostile-matrix-20260929.json`.
- Transactional installer hardening is complete on commit `747702d`: production keeps the default `SearchToolIndexer`, tests can use an isolated SCM name, durable markers record service ownership, and `-RecoverOnly` can recover an interrupted upgrade without beginning a new install. Seven SCM-disruptive boundaries were exercised with abrupt process exit 197 and separate-process recovery; previous binary/config/index state returned every time, while the live `SearchToolIndexer` stayed Running/Auto with the same PID/path. Evidence: `docs/evidence/install-transaction-fault-matrix-20260929.json`.
- Compaction publish/swap fault injection is complete. Eleven abrupt child-process exit boundaries are covered; the test exposed and fixed a mixed-generation recovery bug by making absence/presence of the main file the rollback/commit bit.
- Controlled sleep/resume continuity is complete. The real C: validation kept the same boot session, preserved the pre-sleep marker, observed a post-resume marker, advanced the checkpoint, kept SearchToolIndexer Running + Automatic and passed doctor + verify-deep. Evidence: `docs/evidence/power-cycle-sleep-20260929.json`.
- Controlled reboot continuity is complete. Windows boot time changed, SearchToolIndexer auto-started Running + Automatic with a new PID, the old marker survived, a new marker arrived through automatic USN catch-up, the checkpoint advanced and doctor + verify-deep passed. The first verify exposed only a 45 s harness catch-up-window false negative during cold-start metadata maintenance; the harness now defaults to 120 s and the same reboot state passed. Evidence: `docs/evidence/power-cycle-reboot-catchup-failure-20260929.json` and `docs/evidence/power-cycle-reboot-20260929.json`.
- External validation harnesses are now explicit: `display-validation.ps1` exercises mixed-DPI GUI moves and topology-recovery prepare/verify; `smartscreen-validation.ps1` records policy/MOTW/signature plus observed Warned/Blocked outcome. Current host still cannot supply the physical evidence.

## Immediate continuation order

1. Integrate the final validation-gate hardening and rerun foreground-impact after source freeze.
2. Run the long-soak / clean-machine / low-end hardware evidence still listed in `docs/ROADMAP.md`.
3. Run the final package/release gate once no source changes remain.

For the full backlog see `docs/ROADMAP.md`. For evidence and exact PASS/blocked states see `docs/TEST_MATRIX.md`.

## Safe resume rule

Before mutating a real index, always run:
`search-tool doctor <index-dir>`
and inspect whether an SCM SearchToolIndexer instance is running. `maintain` intentionally refuses unsafe concurrent mutation. Use isolated VHDs for destructive USN/journal recovery tests.
