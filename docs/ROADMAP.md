# Search Tool Roadmap

Last updated: 2026-10-06.

## PR #64 - normal-user live GUI refresh fix (VALIDATED, 2026-10-06)

- Canonical packaged source: a34b462cfe3dc523f042ea74e2325564cdc16d56. Exact package-input equivalent Windows and Ubuntu CI run: 37474144716 SUCCESS.
- Hosted release gate 37474050916 SUCCESS, 12/12 gates and 139 workspace tests PASS.
- Canonical installed ZIP: artifact 11418726329 (SearchTool-Windows-release-gate), SHA-256 0AC3DDAA35FE19AD9C5FDECEEDA6DEA1107027DA60EBEDA806488F469CE58B62, 1920874 bytes.
- Package evidence: docs/evidence/windows-release-gate-pr64-readonly-gui-37474050916-20261006.json; Git blob 4cacd14aec22df9fcb7d4968273198ca5a7054fc.
- Real Windows 10 production install: SearchToolIndexer Running/Auto, original C: index preserved, four installed binaries match canonical gate payload, Startup shortcut untouched. Normal-user GUI screenshots: readme 80 results / 24.4 ms; notepad 15 results / 3.8 ms.
- Root cause fixed: live index refresh previously requested write access on the SYSTEM-owned mutation lock. Read-only shared reader locking now excludes writers without requiring write permissions.
- The separate exact-head CI Windows ZIP (SHA-256 71111BA1C8ABB8E41E1296E97879C9FE4E471FB0E5161674DA0B36B81313C5D7) is NOT byte-identical to this canonical release-gate ZIP. CI pristine PASS is evidence only for that other ZIP. The canonical ZIP has its own release-gate install/uninstall PASS and physical upgrade PASS.
- Start, taskbar Search and Explorer search UI remain Windows-owned. The fixed GUI is Search Tool's separately invoked Win32 application; no system SearchHost backend replacement is claimed.
- External mixed_dpi gate remains BLOCKED pending two genuinely active monitors at distinct DPI.

## Historical PR #62 package (VALIDATED at that time, 2026-10-06)

This section records the historical PR #62 sealed package previously deployed on DESKTOP-ONDD84S. PR #64 supersedes it as the current production package.

- Package status: **VALIDATED**. Canonical exact-head Windows+Ubuntu CI, portable package verification and pristine validation all PASS.
- Latest packaged-input change: `aeed22401cfe972f466fdb7b39a1e8949528ef46` (PR #62 Explorer cleanup and temp-name hardening); later docs-only commits do not change package inputs.
- Exact-head/full CI: `37428078637` — Windows + Ubuntu SUCCESS (tested docs-only head `041733a61e4670efb522f71ccb74fc77df6087de`, packaged source `aeed22401cfe972f466fdb7b39a1e8949528ef46`).
- Windows package artifact: `11396301339` (`SearchTool-Windows-x64`).
- Pristine validation artifact: `11396331342` — PASS.
- Sealed ZIP SHA-256: `4D7D0AF28CA1B8DC01BABB644F93CAB0133034205280F057ED037B8CB2F3734F`; size **1,920,840 bytes**.
- Package evidence: `docs/evidence/windows-release-gate-pr62-explorer-native-first-37428078637-20261006.json`; Git blob `b283bca59990ea0852470a665e9424afcf362130`.
- Previous sealed-source CI (historical): `37357508957` — Windows + Ubuntu SUCCESS.
- First PR #62 run `37365145724`: Ubuntu SUCCESS, Windows job CANCELLED without runner assignment on 2026-10-05; GitHub reports a stale QUEUED workflow/check-suite and no Windows artifact. Not a complete exact-head CI PASS.
- Previous Windows package artifact (historical): `11365148367` (`SearchTool-Windows-x64`).
- Previous pristine validation artifact (historical): `11366210455` — PASS.
- Previous sealed ZIP (not PR #62): SHA-256 `07A02DB4F18FFD8D8DDD428DCB6C8B3C1AF679E5F263A6B5C5EC35AF79581A4D`; size **1,920,992 bytes**.
- Previous package evidence (historical): `docs/evidence/windows-release-gate-pr59-start-menu-native-first-37357508957-20261005.json`; Git blob `06aa43ce158ab72cd5cab15f86ac3307fa54e152`.
- PR #62 validation: **137 tests PASS** (76 core + 9 platform + 14 CLI + 28 GUI + 4 service + 6 worker), fmt/clippy/release build, Windows installer/NTFS/USN/pristine and packaged ZIP verification PASS.
- Native shell policy: Win, taskbar Search and File Explorer search stay on Microsoft's own Windows UI. Resident startup uses `--no-shell-bridge`; the legacy keyboard bridge is opt-in only via `--shell-bridge`.
- Native search ownership: installer does not register `SearchTool.Search`, `search:` OpenWith, Capabilities or RegisteredApplications ownership. Private `searchtool:` and explicitly invoked scoped search remain available; PR #62 removes all three legacy Explorer right-click shell verbs during install/upgrade.
- Native theme layer: system/app light-dark mode, Windows transparency, accent color and accent surfaces are changed through Windows Personalization/DWM settings; Search/Explorer/Start remain Windows-drawn controls.
- Physical runtime evidence remains `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` / blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`.
- Six-hour source-freeze soak remains `docs/evidence/soak-6h-fa92628-final-20260930.json` / blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
- Completed external gates remain `smartscreen`, `defender`, `web_resolver`: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` / `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`; `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json` / `3349e503636f5c9c0a2613892b62c5bac15b0e02`; `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json` / `cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
- Sole unresolved external blocker remains `mixed_dpi`: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json` / blob `71da68834378b99dd8fdb7687378f664f722bf`.

This is the ordered continuation backlog. Items marked blocker should be completed before calling the current source tree a final release candidate.

## Native-first physical deployment — PR #62 Explorer cleanup final

- Physical host: `DESKTOP-ONDD84S`.
- Merged main: `ea0ed06afa0df65eb4ca75661a0ca0235a865fa6`; main CI `37442561672` SUCCESS on Windows + Ubuntu.
- Deployed sealed Windows artifact: `11396301339`; pristine validation artifact: `11396331342`; package source `aeed22401cfe972f466fdb7b39a1e8949528ef46`.
- Package SHA-256: `4D7D0AF28CA1B8DC01BABB644F93CAB0133034205280F057ED037B8CB2F3734F`; size **1,920,840 bytes**.
- Legacy Explorer context-menu verbs were present **3/3** before upgrade and are absent **3/3** afterward for Directory, Directory Background and Drive.
- Four installed binaries hash-match the sealed ZIP; transactional upgrade preserved the C: index, and `SearchToolIndexer` is Running + Automatic.
- Normal-user `doctor` and `search` both exit 0, with no explicit user ACL entry added; resident GUI runs in interactive Session 1.
- Existing user Startup shortcut was preserved byte-for-byte and still starts `--resident --no-shell-bridge`; no standalone custom-search Start Menu shortcut exists.
- Windows `search:` ownership remains native, private `searchtool:` remains registered, and native Windows theme readback is PASS.
- Physical evidence: `docs/evidence/windows-physical-pr62-explorer-native-first-11396301339-20261006.json`. The external `mixed_dpi` gate remains BLOCKED until two real active monitors have distinct effective DPI.

## Native-first physical deployment — Start Menu cleanup final

- Physical host: `DESKTOP-ONDD84S`.
- Deployed sealed artifact: `11365148367`; package source `8bd3e8933d0482851a38bfed569458af3973b139`.
- Merged main: `afe073a08c7bfa2dc3b22618bc2677c4484838df`; main CI `37359619398` SUCCESS.
- Package SHA-256: `07A02DB4F18FFD8D8DDD428DCB6C8B3C1AF679E5F263A6B5C5EC35AF79581A4D`; size **1,920,992 bytes**.
- Upgrade migration verified: the legacy per-user Start Menu `Search Tool.lnk` existed before upgrade (SHA-256 `1995DB45208410435F838842F4690AF5E5FE396A45533745CD5546B11CB7C85E`) and is absent after upgrade.
- All four installed binaries hash-match the sealed ZIP.
- `SearchToolIndexer` is Running + Automatic; existing `C.stidx` was preserved.
- Normal-user `doctor` and `search` both exit 0 with zero explicit `umut` ACL entries on the index.
- Resident GUI is running in interactive Session 1; Startup remains `--resident --no-shell-bridge`.
- The separate Start Menu custom-search panel shortcut is absent. Win, taskbar Search and Explorer remain the visible Windows-native search surfaces.
- Legacy `SearchTool.Search` / Capabilities / RegisteredApplications / `search:` OpenWith ownership is absent; private `searchtool:` remains.
- Native theme readback is PASS: apps light, system dark, transparency on, configured accent `#0078D7`.
- Evidence: `docs/evidence/windows-physical-native-first-startmenu-final-11365148367-20261005.json`; Git blob `09baf9a8c1caeb17d55e7e3fcddec5b0266877a5`.

## Previous read-only-index release checkpoint (historical)

This checkpoint was superseded by the native-first package and physical deployment recorded above.

- Read-only-index package `B98AE500D1F6E52DBE0C26228D58DD16A4FBA98C15647FA35AB0EAC8D7CBB169` (artifact `11341270143`) was VALIDATED with 134 workspace tests.
- SmartScreen, Defender and Web Resolver external gates were already complete/PASS.
- The remaining blocker at that checkpoint was the same real physical mixed-DPI evidence gate.

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
9. **Web Resolver provider/cache/privacy exercise — COMPLETE / PASS**
   - The final provider path uses API-keyless local SearXNG against the packaged release.
   - Provider success, credential-free cache hit and private parent-path suppression all PASS.
   - Evidence: `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json`; Git blob `cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
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

14. **Defender external validation — PASS / SmartScreen — PASS**
   - The physical host remains unsuitable for Defender evidence because protection is disabled there; that host-specific limitation is historical provenance, not the current release state.
   - Earlier GitHub-hosted Windows Server 2025 readiness runs were BLOCKED/UNAVAILABLE, but hosted run `36972721866` successfully enabled real-time and behavior protection for the isolated probe, verified all required protection booleans, and custom-scanned the exact sealed candidate with zero new related detections.
   - Final Defender evidence: `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`; Git blob `3349e503636f5c9c0a2613892b62c5bac15b0e02`. Historical blocked evidence remains at `docs/evidence/defender-hosted-blocked-20261001.json`.
   - Historical hosted SmartScreen probe reached MOTW/unsigned readiness but remained BLOCKED because no enabled configuration or interactive outcome existed. Final physical SmartScreen evidence is PASS: temporary `Warn` policy + MOTW on `search-tool-gui.exe` produced SmartScreen Event 1000 with `Enforcement=warnByPolicy`, `Experience=Untrusted`, and the repository validator recorded `ObservedOutcome=Warned`.
   - Final SmartScreen evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`; historical hosted evidence remains at `docs/evidence/smartscreen-hosted-blocked-20261001.json`.
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
17. **Web Resolver external exercise — COMPLETE / PASS via API-keyless local SearXNG.** Credential-backed Google CSE is no longer a release requirement.
18. **Current packaged-source Windows release gate + package — COMPLETE / PASS**
   - Current packaged source: `6c4141d0bcf12ade21cf633fbaf42d361eb12977`; exact-head CI `36847421304` SUCCESS on Windows + Ubuntu.
   - Full hosted Windows release-gate run `36848221272`: SUCCESS; summary PASS for preflight, fmt, clippy, 117 tests, release build, CLI smoke, NTFS/USN/service integration, journal-reset recovery, package build/integrity, clean install/uninstall and Defender interaction step.
   - Validation wrapper `79f061e09b8d0677ec67532ac0142a6e3d8449cc` differs from the packaged source only by the temporary workflow used to run and seal the gate, so packaged inputs are identical.
   - Current candidate ZIP SHA-256: `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`; size 1,888,674 bytes; artifact seal re-hash PASS.
   - Evidence: `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`.
   - The prior `d01b271` physical Windows runtime gate remains valid runtime evidence; this package refresh changes only `scripts/display-validation.ps1`.
   - The sealed six-hour service/runtime soak remains valid because search-core/platform/service/worker runtime inputs are unchanged.
   - Remaining environment-dependent items are physical mixed-DPI/topology evidence and credential-backed Web Resolver. Defender and SmartScreen are complete.
19. **Machine-readable release-state consistency gate — COMPLETE**
   - `docs/RELEASE_STATE.json` owns the validated package source/SHA/size, release-gate evidence, physical runtime gate, sealed six-hour soak and external blocker evidence.
   - `.github/scripts/release-state-check.ps1` cross-validates those artifacts and the five synchronized continuation/validation documents, including the sealed six-hour-soak evidence path, current 117-test workspace count, every unresolved blocker evidence path/blob, and completed external-gate evidence. It enforces exact structural sets for unresolved blockers, completed gates, synchronized documents, post-package allow-prefixes and transient validation paths.
   - While package status is `VALIDATED`, the current tree may differ from packaged source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` only under `.github/` and `docs/`; any packaged-input change fails CI until the state is explicitly invalidated or replaced with a fresh package seal.
   - `.github/scripts/release-state-selftest.ps1` proves fail-closed behavior against packaged-input mutation, stale package/test-count claims, a BLOCKED unresolved gate falsely promoted to PASS, invalid/missing completed-gate state, missing required blocker/document/transient entries, unsafe allow-prefix expansion, swapped blocker evidence, tampered blocker/completed-gate/core evidence blobs, and a synchronized document missing a required evidence marker.
   - Current unresolved-blocker evidence Git blob seals: `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`.
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
   - At that readiness stage SmartScreen still reported BLOCKED because no enabled machine/user/policy configuration was exposed (`effective_enabled=null`) and no interactive Warned/Blocked outcome was observed; milestone 24 supersedes this with final PASS evidence.
   - Evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`. Canonical sealed blocker evidence remains unchanged.
23. **Latest current-main all-gates physical preflight — BLOCKED BY ENVIRONMENT**
   - Source `d376b244165c067be951ae45e0f9d5ea51129e3a` was clean on the authorized Windows checkout.
   - The exact sealed ZIP (1,888,674 bytes, SHA-256 `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`, `ZoneId=3`) was supplied to the orchestrator.
   - Aggregate result: `BLOCKED`, 0 FAIL, 0 skipped. Defender `UNAVAILABLE`; SmartScreen `BLOCKED` with MOTW/unsigned readiness true but `effective_enabled=null`; display `BLOCKED` at one 1600x900 @ 96-DPI monitor; Web Resolver `BLOCKED` because key/CX are absent before CLI availability matters.
   - Evidence: `docs/evidence/external-validation-current-main-d376b244-20261002.json`.
24. **SmartScreen physical final validation — PASS**
   - Exact sealed ZIP from release-gate run `36848221272` rehashed to `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`; `search-tool-gui.exe` was extracted, remained unsigned, and received `ZoneId=3` MOTW.
   - Temporary SmartScreen policy `EnableSmartScreen=1` / `ShellSmartScreenLevel=Warn` produced `READY_FOR_INTERACTIVE_CHECK`. Launching the binary emitted SmartScreen Event 1000 with `Enforcement=warnByPolicy` and `Experience=Untrusted`.
   - Final validator result: `PASS`, `ObservedOutcome=Warned`. Temporary SmartScreen policy and Debug log changes were rolled back; no SmartScreen/GUI process remained.
   - Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`; sealed Git blob `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`.
25. **Post-SmartScreen current-main unresolved-gate preflight — BLOCKED BY ENVIRONMENT**
   - Current `main` `eae87f121f7b6333e0919696f0678649b0228e56` passed release-state verification on the authorized Windows checkout.
   - SmartScreen was intentionally skipped because it is already a completed sealed PASS gate. Defender, mixed-DPI and Web Resolver were exercised together.
   - Aggregate result: `BLOCKED`, 3 unresolved gates, 0 FAIL, 1 skipped. Defender `UNAVAILABLE`; display `BLOCKED` with one 1600x900 @ 96-DPI monitor; Web Resolver `BLOCKED` because key/CX are absent before CLI existence matters.
   - Evidence: `docs/evidence/external-validation-three-unresolved-eae87f1-20261002.json`.
   - Canonical blocker seals remain unchanged.

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

## 2026-10-02 authoritative external-gate state after hosted Defender PASS

Defender is COMPLETE / PASS from hosted run 36972721866 against the exact sealed 1,888,674-byte candidate (0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65). Realtime and behavior protection were active at scan time, the custom scan returned PASS with zero new related detections, and restoration attempts succeeded. Evidence: docs/evidence/defender-hosted-active-pass-36972721866-20261002.json; Git blob 3349e503636f5c9c0a2613892b62c5bac15b0e02.

Completed external gates are now smartscreen and defender. SmartScreen remains sealed by docs/evidence/smartscreen-physical-pass-f322126-20261002.json, blob 355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952.

At the 2026-10-02 historical checkpoint, the unresolved external blockers were mixed_dpi and web_resolver: docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json / blob a1c0c329a1024ab02948361b9f8102e069f0db95, and docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json / blob ed3d9b56fc75e7d56620e639917988882c732550. This is retained as provenance only; the current authoritative state is the 2026-10-05 release-state summary above.


## 2026-10-03 authoritative package and external-gate state

This section supersedes older "current" Web Resolver/package statements above; older BLOCKED/credential-only entries remain historical provenance.

- Package status: **VALIDATED**.
- Packaged source: `3dfe4ab4ae381c6e5fc8720e76254be0b3f8659d`.
- Windows package: SHA-256 `5639177286DEEBBC6794CCAE9475E02C88CF05F001693484643EC8CE7D6ABA57`, 1,894,905 bytes.
- Package evidence: `docs/evidence/windows-release-gate-pr41-searxng-37029906278-20261003.json`; Git blob `a12db2af2f6a52c2c097119b3c6e938c70c62893`.
- Exact-head/full release CI: run `37029906278`, Windows + Ubuntu PASS; package verify, portable installer smoke and pristine default-path validation PASS.
- Current workspace test count: **121**.
- Runtime physical gate evidence remains `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json`; blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`.
- Six-hour soak remains applicable because runtime/service inputs did not change: `docs/evidence/soak-6h-fa92628-final-20260930.json`; blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
- The only unresolved external blocker is **mixed_dpi**: `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`; blob `a1c0c329a1024ab02948361b9f8102e069f0db95`.
- SmartScreen is completed/PASS: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`; blob `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`.
- Defender is completed/PASS: `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`; blob `3349e503636f5c9c0a2613892b62c5bac15b0e02`.
- Web Resolver is completed/PASS using API-keyless local SearXNG against the packaged release binary, with Google key/CX absent, real provider success, credential-free cache hit and parent-path privacy PASS: `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json`; blob `cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
- Completed external gates are exactly: `smartscreen`, `defender`, `web_resolver`.
- Remaining release work is environment-only: a real second active display with distinct effective DPI for the final mixed-DPI/topology exercise.


## 2026-10-04 refreshed interactive mixed-DPI blocker evidence

- Current authoritative unresolved blocker: `mixed_dpi`.
- Evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`.
- Evidence Git blob: `71da68834378b99dd8fdbdf7687378f664f722bf`.
- Collected through the versioned interactive live launcher from Windows user session 1 on `DESKTOP-ONDD84S`; `user_interactive=true`, standard `\\.\DISPLAY1` device naming verified.
- Physical topology at collection time: one active 1600x900 display, 96 DPI / 100%, one distinct effective DPI value.
- Result remains `BLOCKED` because at least two real active displays with distinct effective DPI values are required.
- Package seal remains unchanged: source `3dfe4ab4ae381c6e5fc8720e76254be0b3f8659d`, SHA-256 `5639177286DEEBBC6794CCAE9475E02C88CF05F001693484643EC8CE7D6ABA57`, 1,894,905 bytes.

## Distinctive UI promotion

Active work: `ui/distinctive-search-v1`. Promotion criteria: Windows GUI fmt/tests/clippy/release build/smoke PASS; hosted Windows + Ubuntu CI PASS; physical Windows interactive smoke confirms branded header, card results, preset switching, color persistence and background-image hot reload; then rebuild/reseal a new package candidate because GUI runtime inputs changed. The existing mixed-DPI physical gate remains required for the new candidate rather than being waived or copied from the older sealed package.

## Distinctive UI v1 package reseal

- Package status: **VALIDATED**.
- Packaged source: `9cdef4d0e33446d39254893cbfe41c8ebb1e92ce`.
- Merged main with identical runtime tree: `5cf797bf21b89d03760c803526afef84b7096c31`.
- Exact-head CI: `37226531224` — SUCCESS on Windows + Ubuntu.
- Merged-main hosted release gate: `37227446525` — SUCCESS.
- Package evidence: `docs/evidence/windows-release-gate-pr49-distinctive-ui-37227446525-20261004.json`.
- Package evidence Git blob: `c984afed1b56e814d64514c30e215631739cf78e`.
- Hosted package artifact: `11312471590` (`SearchTool-Windows-x64`).
- Pristine validation artifact: `11312441766` — PASS.
- Sealed ZIP SHA-256: `7D84E45B4D7018929200F802226C1A4C23EA7235CB0ADBF75BB89D9C13743958`.
- Sealed ZIP size: **1,903,718 bytes**.
- Workspace tests: **126 passed**.
- Remaining external blocker remains only `mixed_dpi`.

## Native Search v1 package reseal (historical)

- Package status: **VALIDATED**.
- Packaged source: `ec1f30be861dc5ad06f6701f874674caef2a773e`.
- Merged main with identical runtime tree: `1e0477042a530196a7309eca3da4b79b97e13ac0`.
- Runtime tree SHA: `fdb7bce19a46c4236f768ae5e0049e8e3a0e833b` on both packaged source and merged main.
- Exact-head CI: `37285730879` — SUCCESS on Windows + Ubuntu.
- Merged-main hosted release gate: `37286499794` — SUCCESS.
- Package evidence: `docs/evidence/windows-release-gate-pr51-native-search-37286499794-20261005.json`.
- Package evidence Git blob: `545e32cf1991179708bb30f63662ec90fd47550f`.
- Hosted package artifact: `11335205559` (`SearchTool-Windows-x64`).
- Pristine validation artifact: `11335165734` — PASS.
- Sealed ZIP SHA-256: `9793FBA354B3A48089E49657962843708BFB88B908B25FF3436841E727C91C28`.
- Sealed ZIP size: **1,914,618 bytes**.
- Workspace tests: **132 passed**.
- Previous Windows package was invalidated by the native Windows Search/Explorer bridge input changes and is superseded by this hosted seal.
- Physical local full release gate could not be elevated in the Remote Desktop Commander session; authoritative NTFS/USN, journal recovery, package integrity, installer smoke and pristine validation are supplied by hosted run `37286499794`.
- Remaining external blocker remains only `mixed_dpi`.

## Read-only index access package reseal (historical)

- Package status: **VALIDATED**.
- Packaged source: `67db5fd09515fa79a3652dd589ae00f464d4b1e3`.
- Merged main with identical runtime tree: `eeac61e8973a1658efb46a0b0cbd4c5ef080bad0`.
- Runtime tree SHA: `8d37cb0adb287ab11dcb73cf77df165905dc02f1` on both packaged source and merged main.
- Exact-head CI: `37297909015` - SUCCESS on Windows + Ubuntu.
- Merged-main hosted release gate: `37298666884` - SUCCESS.
- Package evidence: `docs/evidence/windows-release-gate-pr53-readonly-index-37298666884-20261005.json`.
- Package evidence Git blob: `1d6835e33cd552cdeb6da7c551bbb70619992f67`.
- Hosted package artifact: `11341270143` (`SearchTool-Windows-x64`).
- Pristine validation artifact: `11340159789` - PASS.
- Sealed ZIP SHA-256: `B98AE500D1F6E52DBE0C26228D58DD16A4FBA98C15647FA35AB0EAC8D7CBB169`.
- Sealed ZIP size: **1,915,738 bytes**.
- Workspace tests: **134 passed**.
- Physical Windows A/B proof showed the previous sealed build failed normal-user `doctor` and `search` with Access Denied under read-only index permissions, while the fixed build passed both on the same index and ACL.
- Hosted Windows validation passed NTFS/USN integration, USN reset recovery, installer rollback, package verification, portable installer smoke and pristine default-path validation.
- Final physical sealed deployment evidence: `docs/evidence/windows-physical-readonly-index-final-11341270143-20261005.json` (Git blob `33185195c38e24328713922f13f4ca62364e1fdf`).
- Hosted artifact `11341270143` was installed on `DESKTOP-ONDD84S`; the temporary per-user Modify ACL was removed, sealed `doctor` and `search` both exited 0, and resident GUI bridge smoke passed `ABC123 -> ABC12 -> Esc`.
- The obsolete lab index family was removed after production verification: 19 `C.stidx*` files / 1,508,875,888 bytes (~1.41 GiB), with production search and service still PASS afterward.
- Remaining external blocker remains only `mixed_dpi`.


## Immediate release steps after 2026-10-07 content-index cleanup

- [x] Recover the full system disk without deleting published index data; document evidence and add fail-safe cleanup to the builder.

- [x] Validate 141 workspace tests, Clippy and Release build on real Windows.

- [ ] Complete exact-head Windows/Ubuntu CI and merge the fix.

- [ ] Build a new canonical Windows package, run the hosted release gate, update package seal and release-state evidence, then physically validate the installed new binaries before declaring VALIDATED.

- [ ] Complete native Explorer/Start UI interaction testing after the user activates Nexowire's local physical-console grant; perform Windows 11 and mixed-DPI tests only on real suitable hardware.


## 2026-10-07: PR #67 content-build cleanup release validated

- Packaged source: `adde2939ac31ad0ccd29b958ab5ed34390fbfd61` (tree `88820c9e8ff536a7d4319bfb21504c495f2405bb`).
- Exact-head PR CI: `37546142917` SUCCESS (Windows + Ubuntu). Merged main: `17c311190b1969ee0296b393e81637003259e9a2`; main CI `37546796447` SUCCESS.
- Hosted Windows release gate: `37546647959` SUCCESS, 5-minute soak, package verify/install smoke/NTFS-USN/journal-reset checks PASS.
- Canonical package: SHA-256 `3D293843BE71D322CFF9729C5678401CAB1FFB0510BBDD897328459531CE40A2`, 1,921,283 bytes, artifact `11451351070` (`SearchTool-Windows-release-gate`).
- Canonical release evidence: `docs/evidence/windows-release-gate-pr67-content-cleanup-37546647959-20261007.json`; Git blob `d5d05f8a3790d2632ce09f81c59f8ef1789c1389`.
- Physical production upgrade PASS on `DESKTOP-ONDD84S`: service Running/Auto; installed binary hashes match the canonical package; published content index/checkpoints and user Startup shortcut were preserved; native `search:` ownership remains absent; legacy Explorer Search Tool verbs remain absent; private `searchtool:` protocol remains present.
- Normal-user smoke PASS: verify `status=ok` (1,162,631 records), search exit 0, resident GUI relaunched in Session 1 from Program Files. The interactive keyboard/mouse UI smoke is still unverified because the local Nexowire physical-console grant was not approved.
- Workspace validation count: 141 tests. Package state is `VALIDATED`. External `mixed_dpi` remains legitimately `BLOCKED` until two real active monitors expose distinct effective DPI.


## 2026-10-07: final physical Windows UI smoke

- Evidence: `docs/evidence/windows-physical-final-ui-smoke-20261007.json`.
- Nexowire physical-console control was explicitly approved for the test session.
- Windows Explorer native search PASS: the real `SearchEditBox` was clicked, `readme` was typed, Enter submitted, and two visible `README.md` results were rendered.
- Windows Search PASS: `Win+S` opened the real SearchApp panel, `notepad` was typed, and `Not Defteri` appeared as the best match.
- Installed Search Tool GUI PASS: the resident Program Files build was shown, clicked, `readme` was typed, and it displayed `80 sonuç • 30.1 ms`; Escape hid the window while preserving the resident Session 1 process.
- Post-test runtime remained healthy: SearchToolIndexer Running/Auto, index verify `status=ok`, and zero content staging temp files.
- This closes the previous interactive-console UI-smoke gap. `mixed_dpi` remains the only physical external blocker.
