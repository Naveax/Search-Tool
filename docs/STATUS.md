# Development Status

Last updated: 2026-09-28.

## Current state

Search Tool's core implementation is feature-complete for the current milestone. Remaining work is primarily release hardening, long-running validation and low-end hardware evidence rather than missing core functionality.

## Implemented

- NTFS MFT streaming initial index.
- USN Journal incremental sync, checkpointing, reset/truncation detection and full reconciliation.
- Append-only delta overlay with bounded-memory external compaction.
- Generation-safe rebuild/compaction publishing and per-index OS mutation locking.
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

2026-09-28 physical Windows x64 release gate: **PASS**.
Workspace tests at the current source state: **78 passed, 0 failed**.
See `docs/TEST_MATRIX.md` for detailed evidence.

## Remaining work

Release blockers:
- D: lab recovery is complete and the soak failure is diagnosed/hardened; complete a fresh 60-minute crash/restart soak with final PASS JSON;
- recover/refresh the real C: validation index and reinstall the SCM service against the verified real index;
- controlled sleep/resume and reboot continuity tests;
- compaction publish/swap kill-point fault injection.

Hardening:
- hostile parser-worker input matrix;
- interrupted-upgrade rollback;
- real multi-monitor mixed-DPI GUI validation;
- valid Web Resolver success/cache request when credentials are available.

Final evidence:
- 1M+ real-index p50/p95/p99 search matrix;
- 6-hour and preferably 24-hour soak;
- Defender/SmartScreen on a clean Defender-enabled Windows install;
- pristine-machine install flow;
- Celeron-class CPU + 4 GB RAM + mechanical HDD physical benchmark.

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.
