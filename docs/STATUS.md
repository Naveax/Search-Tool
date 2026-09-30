# Development Status

Last updated: 2026-10-01.

## Current state

Search Tool's core implementation is feature-complete for the current milestone. Remaining work is primarily release hardening, long-running validation and low-end hardware evidence rather than missing core functionality.

## Implemented

- NTFS MFT streaming initial index.
- USN Journal incremental sync, checkpointing, reset/truncation detection and full reconciliation.
- Append-only delta overlay with bounded-memory external compaction.
- Crash-safe rebuild/compaction publishing with main-last commit semantics, per-index OS mutation locking and 11-boundary abrupt-exit regression coverage.
- Disk-first main/name/id indexes with persistent sparse checkpoints.
- Fixed-NTFS multi-volume discovery and global search merge.
- Exact, prefix, ranked, fuzzy, relationship and filtered search.
- Path reconstruction and volume-scoped IDs.
- Size/date/attribute metadata sidecars with generation freshness.
- Plain content index with sparse checkpoints.
- Isolated rich-document worker with IFilter plus built-in PDF/OOXML fallback parsing.

- Resource Governor / low-end policies for CPU, RAM, user activity, battery and storage type.
- Windows background processing mode for service and expensive workers.
- Duplicate pipeline: size -> sample -> full fingerprint -> byte-for-byte final verification, including cross-volume groups.
- Knowledge/classification and cleanup safety policy.
- Quarantine, restore and explicit permanent purge.
- Tiny ~1 MiB INT8 natural-language intent router.
- Privacy-sanitized optional cached web resolver.
- Native Win32 GUI, single-instance resident launcher and fallback global hotkey.
- Native Windows SCM service for USN sync and idle maintenance.
- HDD/SSD seek-penalty detection.
- Verify, verify-deep, repair, maintain and doctor diagnostics.
- Multi-volume installer/uninstaller/package scripts.
- Isolated NTFS VHD integration suite and journal-reset recovery tests.
- Windows soak, foreground-impact, low-end, Defender and physical-validation scripts.
- Portable ZIP integrity/SHA verification.
- Single-command Windows release gate with JSON summary.

## Latest verified gate

2026-09-29 physical Windows x64 full release gate on commit `8e6498d`: **PASS**.
Current source passes **111 workspace tests, 0 failed** (74 core + 7 platform + 9 CLI + 11 GUI + 4 service + 6 worker), workspace clippy `-D warnings` and release build. Base-family publish consistency is now enforced inside `SearchStore::open()`, covering live search, doctor/verify, metadata/content builders and other direct readers with the same coherent-generation snapshot. The last full physical release gate remains commit `8e6498d` with its 94-test snapshot, NTFS/USN integration, journal-reset recovery, portable package integrity and clean install/uninstall smoke. Final package SHA-256: `282A2882EB66186E58935ECD3C5C1169B351ABCD3B47FF83A82A6E87F5ACA585`. Defender interaction completed, but active Defender protection is unavailable on this host and is not evidence of an active antivirus scan. Evidence: `docs/evidence/windows-release-gate-final-20260929.json`.
Post-gate current-main foreground-impact recheck at `6bbde9c` is also **PASS**: p95 151.442 ms -> 176.258 ms (1.164x), nested real-service soak PASS, followed by doctor + verify-deep PASS while SearchToolIndexer remained Running + Automatic. Evidence: `docs/evidence/foreground-impact-current-head-20260929.json`.
See `docs/TEST_MATRIX.md` for detailed evidence.

## Current integration-branch checkpoint

PR #7 remains draft, but the required `fa92628` 6-hour physical soak is now sealed PASS and committed as `docs/evidence/soak-6h-fa92628-final-20260930.json`. Exact code head `5ec0a74` also passed CI run `36786244058`; only the evidence/docs-only head CI must remain green before merge. The installer rollback path preserves exact pre-existing Search Tool registry trees/values. A dedicated `-RegistrySnapshotSelfTest` round-trips the snapshot format through JSON and is invoked by the Windows installer fault matrix, covering value kinds, nested keys and absent-before cleanup without touching production HKLM registrations.

## Remaining work

Release blockers:
- none currently open in the power-cycle/core validation path; sleep/resume and real reboot continuity are both PASS.

Hardening:
- real multi-monitor mixed-DPI GUI validation;
- valid Web Resolver success/cache request when credentials are available.

Final evidence:
- 1M+ real-index p50/p95/p99 search matrix: COMPLETE on 1,209,697-record C: index; see `docs/evidence/search-latency-matrix-20260929.json`;
- Windows Search-style product UI: IMPLEMENTED with resident modes, query+scope IPC, supported `search:`/`searchtool:` protocol paths, Explorer `crumb=location:` scoped search, folder/drive/background Explorer shell verbs, and a native live Tema menu for theme/backdrop/opacity/accent plus a direct Windows Default Apps link; physical mixed-DPI UX validation remains;
- 6-hour soak: COMPLETE / PASS on exact frozen source `fa92628`; 21,873.82 s / 223,632 ops / 9,318 checks, intentional crash/restart exercised, exact source/service SHA identity PASS, post-run doctor + verify-deep PASS, service Running + Automatic. Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`;
- Defender/SmartScreen on a clean Defender-enabled Windows install;
- pristine-machine install flow (strict `pristine-validation.ps1` harness ready; clean-host evidence still pending);

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.
