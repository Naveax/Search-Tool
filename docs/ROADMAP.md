# Search Tool Roadmap

Last updated: 2026-10-01.

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
8. **Multi-monitor GUI implementation hardening — COMPLETE; physical evidence BLOCKED**
   - Native GUI now handles `WM_DPICHANGED`, uses Win32's suggested window rectangle, recreates fonts at the current DPI, and scales layout/hit-test/owner-draw metrics from 96-DPI logical units.
   - Window placement uses the nearest monitor `rcWork`; `WM_DISPLAYCHANGE` and work-area changes clamp/recover the window to an active monitor.
   - Deterministic regressions cover 96/144/192-DPI scaling, negative monitor origins, removed-monitor recovery and oversized-window clamping.
   - Exact-head CI `36840720835` and physical release gate on runtime source `d01b271` PASS.
   - Final physical mixed-DPI / primary-switch / monitor-removal evidence remains BLOCKED: current surface exposes one 1600x900 96-DPI monitor. Evidence: `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`.
9. Valid Web Resolver success/cache path with real Google Custom Search credentials; keep optional and privacy-sanitized.
   - Harness ready: `scripts/web-resolver-validation.ps1` requires real Google key + CX, proves the first request comes from the provider, removes credentials before the second request to prove a cache hit, and checks that a private parent-path marker is absent from output/cache. The physical host remains credential-blocked, and GitHub-hosted Windows probe run `36852274027` independently confirmed that repository secrets `SEARCH_TOOL_GOOGLE_KEY` and `SEARCH_TOOL_GOOGLE_CX` are both absent. Evidence: `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`.

10. **Windows Search-style final product UI + supported Shell integration — IMPLEMENTED, physical UX validation pending**
   - Native resident flyout with Tümü / Dosyalar / Klasörler / İçerik modes, owner-drawn result rows, path display, double-click/Enter open, single-instance query IPC and hidden startup resident mode.
   - Native Tema menu applies system/dark/light, Acrylic/Mica/none, 60/75/90/100% opacity and Windows color-picker accent changes immediately and persists them. `%APPDATA%\SearchTool\ui.conf` remains the advanced path for palette overrides and panel size; the same menu links directly to Windows Default Apps for `search:` selection.
   - Default install registers private `searchtool:` plus a Windows Default Apps contender for the documented `search:` protocol. Explorer-originated `crumb=location:` scope is honored, and classic unpackaged Explorer shell verbs are registered for folders, drives and folder backgrounds. It does not patch Start/Search internals or forcibly steal defaults; Windows 11 first-level modern context-menu placement would require a separate sparse-MSIX + `IExplorerCommand` packaging layer.
   - Per-monitor DPI/topology runtime handling is implemented and release-gate validated at `d01b271`; only final physical multi-monitor evidence remains.

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
13. **Run required 6-hour source-freeze soak — COMPLETE**
   - Final frozen source: `fa92628d515fe25681972fc983f427e1f5108fb3`.
   - Installed service SHA-256: `F26D4088CB03902C2BAB48637670085967E5658A2B594F32A6023285AA774899`.
   - Final run: **PASS**, 21,873.82 s / 223,632 filesystem operations / 9,318 validation checks / 10.22 ops/s.
   - Intentional service crash/restart was exercised; peak service working set 7.461 MiB and peak private 11.527 MiB.
   - Wrapper sealed source head/origin equality, dirty_count=0, exact service identity, post-run doctor exit 0, verify-deep exit 0, and Running/Automatic service state.
   - Run-scoped test root was removed after completion.
   - Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`.

14. **Defender external validation — BLOCKED BY ENVIRONMENT / SmartScreen — PASS**
   - Physical host: active Defender protection unavailable/disabled.
   - GitHub-hosted Windows Server 2025 enforced Defender probe: BLOCKED/UNAVAILABLE because real-time protection and behavior monitoring are disabled despite AM/Antivirus/Antispyware being enabled.
   - Hosted Defender evidence: `docs/evidence/defender-hosted-blocked-20261001.json`, workflow run `36835643698`.
   - Historical hosted SmartScreen probe reached MOTW/unsigned readiness but remained BLOCKED because no enabled configuration or interactive outcome existed. Final physical SmartScreen evidence is now PASS: temporary `Warn` policy + MOTW on `search-tool-gui.exe` produced SmartScreen Event 1000 with `Enforcement=warnByPolicy`, `Experience=Untrusted`, and the repository validator recorded `ObservedOutcome=Warned`.
   - Hosted SmartScreen evidence: `docs/evidence/smartscreen-hosted-blocked-20261001.json`, workflow run `36836915656`.
   - Defender final active-protection/custom-scan evidence still needs a genuinely protected Windows environment. SmartScreen is complete; evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`.
15. **Pristine default-path install/uninstall — COMPLETE**
   - Disposable GitHub-hosted Windows runner began with no Search Tool service/default install/default data/shortcuts/registrations.
   - Installed to default Program Files/ProgramData paths; production `SearchToolIndexer` reached Running + Automatic.
   - Initial NTFS index, marker search, smart search, doctor, GUI smoke and scoped GUI smoke all passed.
   - `SearchTool.Search`, `searchtool:`, Capabilities, RegisteredApplications, OpenWithProgids, App Paths and Directory/Background/Drive verbs were validated.
   - Purge uninstall left service/install/data/startup + programs shortcuts and all 9 registry integration surfaces absent: 14/14 cleanup checks PASS.
   - Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`; CI run `36825801758`.
16. **Multi-monitor mixed-DPI final GUI exercise — PHYSICAL EVIDENCE BLOCKED**
   - Code-level DPI/topology handling and deterministic regressions PASS at `d01b271`.
   - Packaged-source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` hardens the physical evidence harness: mixed-DPI intent survives prepare -> verify, `MonitorRemoved` requires the prepared window to have intersected an actually removed monitor, verify requires the exact prepared GUI PID to survive, and the recovered window DPI must match an intersected active monitor.
   - Exact-head CI `36847421304` and full hosted release-gate run `36848221272` PASS for the hardened harness/package.
   - Current physical display evidence still exposes one 1600x900 @ 96 DPI monitor, so real cross-monitor DPI transitions, primary switch and monitor removal remain externally blocked.
17. Real Web Resolver credential-backed provider/cache/privacy exercise.
18. **Current packaged-source Windows release gate + package — COMPLETE / PASS**
   - Current packaged source: `6c4141d0bcf12ade21cf633fbaf42d361eb12977`; exact-head CI `36847421304` SUCCESS on Windows + Ubuntu.
   - Full hosted Windows release-gate run `36848221272`: SUCCESS; summary PASS for preflight, fmt, clippy, 117 tests, release build, CLI smoke, NTFS/USN/service integration, journal-reset recovery, package build/integrity, clean install/uninstall and Defender interaction step.
   - Validation wrapper `79f061e09b8d0677ec67532ac0142a6e3d8449cc` differs from the packaged source only by the temporary workflow used to run and seal the gate, so packaged inputs are identical.
   - Current candidate ZIP SHA-256: `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`; size 1,888,674 bytes; artifact seal re-hash PASS.
   - Evidence: `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`.
   - The prior `d01b271` physical Windows runtime gate remains valid runtime evidence; this package refresh changes only `scripts/display-validation.ps1`.
   - The sealed six-hour service/runtime soak remains valid because search-core/platform/service/worker runtime inputs are unchanged.
   - Remaining environment-dependent items are active Defender, physical mixed-DPI/topology evidence, and credential-backed Web Resolver. SmartScreen is complete.
19. **Machine-readable release-state consistency gate — COMPLETE**
   - `docs/RELEASE_STATE.json` owns the validated package source/SHA/size, release-gate evidence, physical runtime gate, sealed six-hour soak and external blocker evidence.
   - `.github/scripts/release-state-check.ps1` cross-validates those artifacts and the five synchronized continuation/validation documents, including the sealed six-hour-soak evidence path, current 117-test workspace count, every unresolved blocker evidence path/blob, and completed external-gate evidence. It enforces exact structural sets for unresolved blockers, completed gates, synchronized documents, post-package allow-prefixes and transient validation paths.
   - While package status is `VALIDATED`, the current tree may differ from packaged source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` only under `.github/` and `docs/`; any packaged-input change fails CI until the state is explicitly invalidated or replaced with a fresh package seal.
   - `.github/scripts/release-state-selftest.ps1` proves fail-closed behavior against packaged-input mutation, stale package/test-count claims, a BLOCKED Defender gate falsely promoted to PASS, invalid/missing completed SmartScreen state, missing required blocker/document/transient entries, unsafe allow-prefix expansion, swapped blocker evidence, tampered blocker/completed-gate/core evidence blobs, and a synchronized document missing a required evidence marker.
   - Current unresolved-blocker evidence Git blob seals: `defender=332869529cf3b770c2d97f70ffbbfd416c6bd63f`, `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`. Completed SmartScreen PASS seal: `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952` (`docs/evidence/smartscreen-physical-pass-f322126-20261002.json`).
   - Core release evidence seals: package `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` blob `9bf0fea273b90ac2ba3f164a2ed550cd8cf57294`; physical `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
   - CI invokes both checks on Windows before the expensive integration/package stages.
20. **One-command external validation orchestration — IMPLEMENTED**
   - `.github/scripts/external-validation-orchestrator.ps1` aggregates Defender, SmartScreen, mixed-DPI/topology and credential-backed Web Resolver validation without modifying packaged inputs.
   - Aggregate states are explicit and fail-closed: PASS, READY, BLOCKED, PARTIAL or FAIL. `-EnforceAll` requires final PASS.
   - SmartScreen observed outcome and display Probe/Exercise/PrepareTopology/VerifyTopology modes remain explicit; the wrapper does not invent interactive or physical evidence.
   - CI runs an all-skip Windows smoke and requires PARTIAL with four skipped gates and zero failures, proving argument/default/report aggregation without external dependencies.
   - Web Resolver credential presence is checked before CLI existence. Missing key/CX now yields the real BLOCKED reason and a JSON subreport even on source-only checkouts; CI covers this with a deliberately missing CLI and cleared credential environment.
   - Defender readiness with `-DefenderCustomScan:$false` no longer depends on `target\release`; it probes the repository root, while final custom-scan evidence still requires release binaries. CI covers the source-only/missing-release-dir path.
21. **Current-main physical external preflight — BLOCKED BY ENVIRONMENT**
   - Source `ed65fbc3a9c1472fe99bb5731ecae32eeda46d67` was fast-forwarded onto the authorized Windows checkout and the orchestrator was run non-destructively.
   - Aggregate result: BLOCKED, 0 FAIL, with Defender UNAVAILABLE, one-monitor mixed-DPI BLOCKED, Web Resolver credential BLOCKED and SmartScreen intentionally skipped because no candidate ZIP was present.
   - Defender diagnosis is explicit: `WinDefend` and `WdNisSvc` are Stopped/Disabled and policy values `DisableAntiSpyware=1` / `DisableAntiVirus=1`; the host therefore cannot provide final active-Defender evidence without an environment/policy change.
   - Evidence: `docs/evidence/external-validation-physical-preflight-ed65fbc-20261001.json`. Existing sealed blocker evidence remains canonical and unchanged.
22. **Sealed ZIP physical SmartScreen readiness — BLOCKED BY HOST CONFIGURATION**
   - The exact sealed release ZIP from run `36848221272` rehashed to `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65` at 1,888,674 bytes on the physical host.
   - A staged Internet-zone `Zone.Identifier` ADS on a copy did not change package bytes; MOTW + unsigned/untrusted prerequisites became true.
   - SmartScreen still reported BLOCKED because no enabled machine/user/policy configuration was exposed (`effective_enabled=null`) and no interactive Warned/Blocked outcome was observed.
   - Evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`. Canonical sealed blocker evidence remains unchanged.
23. **Latest current-main all-gates physical preflight — BLOCKED BY ENVIRONMENT**
   - Source `d376b244165c067be951ae45e0f9d5ea51129e3a` was clean on the authorized Windows checkout.
   - The exact sealed ZIP (1,888,674 bytes, SHA-256 `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`, `ZoneId=3`) was supplied to the orchestrator.
   - Aggregate result: `BLOCKED`, 0 FAIL, 0 skipped. Defender `UNAVAILABLE`; SmartScreen `BLOCKED` with MOTW/unsigned readiness true but `effective_enabled=null`; display `BLOCKED` at one 1600x900 @ 96-DPI monitor; Web Resolver `BLOCKED` because key/CX are absent before CLI availability matters.
   - Evidence: `docs/evidence/external-validation-current-main-d376b244-20261002.json`. Canonical blocker seals remain unchanged.

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
