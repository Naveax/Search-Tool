# Search Tool Roadmap

Last updated: 2026-09-29.

This is the ordered continuation backlog. Items marked blocker should be completed before calling the current source tree a final release candidate.

## P0 - Release blockers

1. **Complete the hardened D: 60-minute soak — COMPLETE**
   - Commit `709cc275` completed a full crash/restart soak: 3613.77 s, 56,496 operations, 2,354 validation checks, `result=PASS`.
   - Evidence: `docs/evidence/soak-60m-709cc275-20260928.json`.
   - Final D: cleanup refreshed the content sidecar and ended with delta=0, pending_delta=false, metadata/sizes/content fresh and verify-deep PASS.

2. **Recover the real C: validation index — COMPLETE**
   - Supported content recovery cleaned the interrupted staging and rebuilt 391,281 files / 85,257,251 postings.
   - Before service startup: 1,427,984 records, delta=0, metadata/sizes/content fresh and verify-deep PASS.
   - SearchToolIndexer is Running + Automatic against the real C: index with `service_sync=Ok` and `last_error=0`.
   - Initial real-volume catch-up/automatic compaction converged to 1,304,339 base records with delta=0; steady-state create/rename/delete automatic-USN probe passed 4/4.
   - Evidence: `docs/evidence/real-c-recovery-service-20260929.json`.

3. **Sleep/resume validation — COMPLETE**
   - Controlled sleep/resume passed on the real C: service/index path.
   - Pre-sleep and post-resume markers were visible, boot time remained unchanged as expected, the checkpoint hash advanced, and SearchToolIndexer remained Running + Automatic.
   - `doctor` reported service_sync=Ok / last_error=0 and `verify-deep` returned status=ok after resume.
   - Evidence: `docs/evidence/power-cycle-sleep-20260929.json`.

4. **Reboot validation — COMPLETE**
   - Real Windows reboot changed the boot session and SearchToolIndexer auto-started Running + Automatic with a new PID.
   - The pre-reboot marker remained searchable, the post-reboot marker became searchable through automatic USN catch-up, and the checkpoint hash advanced.
   - `doctor` reported service_sync=Ok / last_error=0 and `verify-deep` returned status=ok.
   - The first verification exposed a harness-only false negative: the 45 s marker window expired while cold-start idle metadata maintenance was still active. The marker appeared shortly afterward. The harness now uses a configurable 120 s default catch-up window and the same reboot state then passed.
   - Evidence: `docs/evidence/power-cycle-reboot-catchup-failure-20260929.json` and `docs/evidence/power-cycle-reboot-20260929.json`.

5. **Compaction commit/swap fault injection — COMPLETE**
   - Added deterministic abrupt child-process termination at 11 publish boundaries from durable marker through final main-file rename.
   - Found a real mixed-generation bug: the old main file remained present during sidecar swaps, so existence-only recovery could misclassify a partial publish as committed.
   - Fixed the protocol by removing the old main immediately after the durable marker, publishing sidecars, then renaming the staged main last. Main-file presence is now the commit bit.
   - Recovery + retry requires verify-deep PASS, correct logical search results, consumed delta and no compact/delta-sort debris. Regression PASS.

## P1 - Hardening

6. **Hostile parser-worker matrix — COMPLETE**
   - Commit `e2be944` adds a 256 MiB per-worker Windows Job Object plus deterministic real-child fixtures for hang, abrupt exit, partial stdout, invalid UTF-8 and oversized response frames.
   - Worker fixtures reject corrupt PDF, password-encrypted PDF, corrupt OOXML and an 8,193-entry OOXML bomb without panics or parent-process failure.
   - Evidence: `docs/evidence/parser-hostile-matrix-20260929.json`.
7. **Upgrade rollback fault injection — COMPLETE**
   - Commit `747702d` adds an isolated service-name path, durable marker ownership, abrupt-exit fault mode, recovery-only execution and an end-to-end SCM fault-matrix harness.
   - Seven boundaries PASS: staged, old-service-removed, live-renamed, new-published, service-installed, before-service-start and service-started.
   - Each boundary exits the installer process with code 197, recovers in a separate process, restores the previous binary/config/index marker and Running/Automatic service state, and removes transaction debris.
   - The production `SearchToolIndexer` remained Running/Automatic with identical PID and binary path throughout. Evidence: `docs/evidence/install-transaction-fault-matrix-20260929.json`.
8. Multi-monitor GUI validation including mixed DPI, primary-display switch and monitor removal recovery.
   - Harness ready: `scripts/display-validation.ps1` can exercise per-monitor GUI DPI moves and record prepare/verify topology recovery across primary-display changes or monitor removal. Current host remains physically blocked at one 100% DPI monitor.
9. Valid Web Resolver success/cache path with real Google Custom Search credentials; keep optional and privacy-sanitized.

## P2 - Performance and release evidence

10. **Real 1M+ C: latency table — COMPLETE**
   - Frozen real C: index measured at 1,209,697 base records with delta=0 and metadata/sizes/content all fresh.
   - 100 measured rounds after 5 warmups per class: exact 27.750/32.206/39.736 ms, prefix 61.983/67.694/68.776 ms, fuzzy 458.443/478.590/497.988 ms, filtered 60.065/66.107/78.889 ms, relationship 130.016/151.995/158.008 ms, content 145.325/155.821/184.094 ms (p50/p95/p99).
   - Evidence: `docs/evidence/search-latency-matrix-20260929.json`, source head `322fb4e`.
11. Rerun foreground-impact after the final source freeze.
12. Run 6-hour soak; ideally also 24-hour soak for leak/delta-growth confidence.
13. Run Defender + SmartScreen on a clean Windows installation with Defender enabled.
   - Strict Defender gate now requires active AV/realtime/behavior/antispyware, no overlapping exclusion, custom scan and zero related detections. `scripts/smartscreen-validation.ps1` records enabled policy, MOTW, signature and an observed Warned/Blocked outcome. Physical clean-machine evidence remains required.
14. Run pristine-machine install -> initial index -> search -> service -> GUI -> uninstall.
   - Harness ready: `scripts/pristine-validation.ps1` fail-closes unless the default service/install/data/shortcut state is absent, then validates package integrity, default Program Files/ProgramData install, SCM Automatic+Running, initial VHD index/search/smart/doctor/GUI smoke, purge uninstall and zero residue. Current host correctly reports BLOCKED because it is not pristine.
15. Run the reference physical target: Celeron-class CPU, 4 GB RAM, mechanical HDD.
16. Tune governor/batch/compaction/content settings only from reference-machine evidence.

## Release freeze checklist

When no source changes remain:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace --release`
- `scripts/windows-release-gate.ps1`
- package + SHA-256 manifest
- record final benchmark/soak reports

Do not commit `target/`, VHDs, live indexes, temp sidecars or machine-specific validation data.
