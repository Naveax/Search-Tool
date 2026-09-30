# Search Tool Roadmap

Last updated: 2026-09-30.

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
   - Harness ready: `scripts/web-resolver-validation.ps1` requires real Google key + CX, proves the first request comes from the provider, removes credentials before the second request to prove a cache hit, and checks that a private parent-path marker is absent from output/cache. Current host remains BLOCKED because credentials are absent.

10. **Windows Search-style final product UI + supported Shell integration — IMPLEMENTED, physical UX validation pending**
   - Native resident flyout with Tümü / Dosyalar / Klasörler / İçerik modes, owner-drawn result rows, path display, double-click/Enter open, single-instance query IPC and hidden startup resident mode.
   - Native Tema menu applies system/dark/light, Acrylic/Mica/none, 60/75/90/100% opacity and Windows color-picker accent changes immediately and persists them. `%APPDATA%\SearchTool\ui.conf` remains the advanced path for palette overrides and panel size; the same menu links directly to Windows Default Apps for `search:` selection.
   - Default install registers private `searchtool:` plus a Windows Default Apps contender for the documented `search:` protocol. It does not patch Start/Search internals or forcibly steal defaults.

## P2 - Performance and release evidence

11. **Real 1M+ C: latency table — COMPLETE**
   - Frozen real C: index measured at 1,209,697 base records with delta=0 and metadata/sizes/content all fresh.
   - 100 measured rounds after 5 warmups per class: exact 27.750/32.206/39.736 ms, prefix 61.983/67.694/68.776 ms, fuzzy 458.443/478.590/497.988 ms, filtered 60.065/66.107/78.889 ms, relationship 130.016/151.995/158.008 ms, content 145.325/155.821/184.094 ms (p50/p95/p99).
   - Evidence: `docs/evidence/search-latency-matrix-20260929.json`, source head `322fb4e`.
12. **Rerun foreground-impact after the final source freeze — COMPLETE**
   - Code-freeze head `215e6bc`: baseline p95 104.123 ms -> stressed p95 108.246 ms (+4.123 ms, 1.04x), PASS.
   - Nested real-service mutation soak also PASS: 102.01 s, 480 operations, 40 validation checks, 120 s marker timeout, 330 s outer wait budget.
   - Two preceding attempts exposed harness-only timeout defects (hardcoded 30 s marker catch-up, then a shorter 90 s outer wait); both were fixed before the final PASS.
   - Evidence: `docs/evidence/foreground-impact-final-20260929.json`.
   - Current-main recheck at `6bbde9c` also PASS: baseline/stressed p95 151.442/176.258 ms (1.164x), nested service soak 230.86 s / 264 ops / 22 checks, followed by doctor + verify-deep PASS. Product/runtime inputs remain unchanged from `8e6498d`. Evidence: `docs/evidence/foreground-impact-current-head-20260929.json`.
13. **Run required 6-hour source-freeze soak — REQUIRED**
   - `675aabb` exposed base-family publish race after 6381.94 s / 67,248 ops / 2,800 checks. Evidence: `docs/evidence/soak-6h-publish-race-20260929.json`.
   - `daad45d` rerun exposed the independent `.delta` partial-tail race after 136.24 s / 912 ops / 36 checks.
   - Current source treats `UnexpectedEof` anywhere in the final delta record as an uncommitted/crash tail while fully-readable corruption stays fail-closed. On writer reopen, any incomplete final tail is truncated to the last complete record boundary before new append, preventing a crash tail from absorbing future bytes. Three deterministic regressions PASS; current workspace is 104/104 with the native-theme GUI regression added, clippy/release build PASS.
   - Before COMPLETE: independent Task Scheduler short high-churn reproduction soak on exact final binaries, then full 6-hour physical soak with intentional service crash/restart. The first scheduled short soak ran 2,496 ops / 104 checks without reproducing the I/O race, but its generic soak-g cleanup query collided with a real test directory left by the earlier SentinelX-killed invalid run. The harness now prefixes every workload filename with a unique run-id and validates only that run; rerun required on the committed final candidate. Evidence: docs/evidence/short-soak-cross-run-contamination-20260930.json.
14. Run Defender + SmartScreen on a clean Windows installation with Defender enabled.
15. Run pristine-machine install -> initial index -> search -> service -> GUI -> uninstall, including theme creation, shortcuts and `searchtool:` / `search:` registration cleanup.
16. Multi-monitor mixed-DPI final GUI exercise.
17. Real Web Resolver credential-backed provider/cache/privacy exercise.
18. **Final current-source Windows release gate + package — REQUIRED**
   - Current source changed delta parsing, GUI and installer/Shell integration; rerun the full gate and record the new package SHA-256.

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
