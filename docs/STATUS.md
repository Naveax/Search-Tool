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

2026-10-01 merged source `4700a6cc2e5e74fd8fa7094528ac4e7ad4451e28`: **PASS** on the physical Windows x64 final release gate.
The gate passed release preflight, fmt, clippy `-D warnings`, all 111 workspace tests, release build, CLI smoke, NTFS/USN/service integration, USN journal reset recovery, portable package build/integrity, clean install/uninstall smoke and the Defender interaction step. GitHub merged-main CI run `36787356719` also passed on Windows + Ubuntu. The production `SearchToolIndexer` remained Running + Automatic and the isolated release-gate service was removed after validation.

Final package SHA-256: `188B3D6C981020179AA6E2299C3CEF208926B0F68303290775684699EA5104F4` (1,840,325 bytes).
Evidence: `docs/evidence/windows-release-gate-4700a6c-final-20261001.json`.

Active Defender protection is unavailable on the physical validation host. A separate GitHub-hosted Windows Server 2025 enforced probe also returned `UNAVAILABLE`: AM/Antivirus/Antispyware were enabled, but `RealTimeProtectionEnabled=false` and `BehaviorMonitorEnabled=false`. Therefore neither environment is claimed as active-AV scan evidence. See `docs/evidence/defender-hosted-blocked-20261001.json`.
See `docs/TEST_MATRIX.md` for detailed evidence.

## Merged Shell integration checkpoint

PR #7 merged to `main` as `4700a6c` after exact-head CI `36786906440` passed. The registry rollback fix at code head `5ec0a74` was independently validated by CI run `36786244058`, including the disposable HKCU snapshot self-test and 10-boundary installer fault matrix. The final six-hour `fa92628` source-freeze soak is sealed PASS, and the merged source subsequently passed both GitHub main CI and the physical Windows release gate.

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
- pristine default-path install/uninstall: COMPLETE / PASS on GitHub-hosted Windows; production `SearchToolIndexer` Auto/Running, initial index/search/smart/doctor, GUI + scoped GUI, all supported Shell/protocol registrations present during install, then 14/14 zero-residue checks after purge uninstall. Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`, CI `36825801758`;

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.
