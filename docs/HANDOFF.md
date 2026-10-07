# Search Tool - Project Handoff

> Authoritative continuation note. Last updated: 2026-10-06.

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

This document exists so development can continue from the repository without needing the original ChatGPT conversation.

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

## Project goal

Search Tool is an ultra-light native Windows file search and safe maintenance utility. The final product goal is to keep the real Windows Search and File Explorer surfaces native while Search Tool supplies a much faster disk-first MFT/USN backend, explicit helper entrypoints and Windows-native theme controls, without Electron/Chromium/JVM/Node runtime overhead.

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
- `apps/search-tool-gui`: explicit Search Tool Win32 helper/IPC surface; normal Win/Search/Explorer remain native Windows UI.
- `apps/search-tool-service`: Windows SCM service, USN sync and idle maintenance.
- `apps/search-tool-worker`: isolated rich-document parser with IFilter/fallback parsers.
- `apps/search-tool-bench`: fresh-process scale/RSS/query benchmark.
- `scripts`: packaging, install/uninstall, soak, journal recovery and release gates.
- `.github`: Windows CI, soak and release-gate workflows.
- `models/tiny-intent-v1.stm`: ~1 MiB INT8 intent model.

## What is implemented

MFT initial index, USN incremental sync, checkpoint recovery, bounded delta overlay, external bounded-memory compaction, multi-volume search, exact/prefix/ranked/fuzzy/relationship search, filters, path reconstruction, metadata sidecars, content index, rich document worker, duplicate verification, quarantine/restore/purge, tiny-AI natural-language routing, privacy-sanitized optional web resolver, native GUI, Windows service, resource governor, install/package scripts, repair/maintain/doctor and Windows validation harnesses are implemented.

## Previous validated package state before native-first reseal (historical)

- Latest runtime-equivalent main release checkpoint before this docs-only refresh: `4b0ab39bf76f0dc36098569573e6c9e4d0cc8027`; push CI `37308593787` PASS on Windows + Ubuntu.
- Package status: **VALIDATED**.
- Packaged source: `67db5fd09515fa79a3652dd589ae00f464d4b1e3`.
- Sealed ZIP SHA-256: `B98AE500D1F6E52DBE0C26228D58DD16A4FBA98C15647FA35AB0EAC8D7CBB169`; size **1,915,738 bytes**.
- Hosted release gate: `37298666884` PASS; package artifact `11341270143`; pristine-validation artifact `11340159789`.
- Package evidence: `docs/evidence/windows-release-gate-pr53-readonly-index-37298666884-20261005.json`; Git blob `1d6835e33cd552cdeb6da7c551bbb70619992f67`.
- Workspace validation: **134 tests PASS** (76 core + 7 platform + 13 CLI + 28 GUI + 4 service + 6 worker), fmt/clippy/release build PASS.
- Completed external gates: `smartscreen`, `defender`, `web_resolver`.
- Sole unresolved external blocker: `mixed_dpi`, with canonical evidence `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json` / blob `71da68834378b99dd8fdbdf7687378f664f722bf`.
- Final physical sealed deployment: `docs/evidence/windows-physical-readonly-index-final-11341270143-20261005.json` / blob `33185195c38e24328713922f13f4ca62364e1fdf`; normal-user `doctor` + `search` PASS without the temporary Modify ACL, resident GUI bridge PASS, production service Running + Automatic.

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
- Final code-freeze foreground-impact at head `215e6bc` passed: baseline p95 104.123 ms -> stressed p95 108.246 ms (+4.123 ms, 1.04x). The nested real-service soak passed 102.01 s / 480 ops / 40 checks. Two prior attempts exposed only harness timeout-budget defects and were fixed before the final PASS. Evidence: `docs/evidence/foreground-impact-final-20260929.json`.
- Current-main foreground-impact recheck at `6bbde9c` also passed; product/runtime inputs are unchanged from release-gate source `8e6498d`. Baseline p95 151.442 ms -> stressed p95 176.258 ms (+24.816 ms, 1.164x); nested real-service soak passed 230.86 s / 264 ops / 22 checks with ~5.574 MiB peak service working set. A bounded idle-window post-check then passed `doctor` + `verify-deep`; SearchToolIndexer remained Running + Automatic with `service_sync=Ok` and `last_error=0`. Evidence: `docs/evidence/foreground-impact-current-head-20260929.json`.
- Real 1M+ search latency matrix passed on a frozen 1,209,697-record C: index at source head `322fb4e`. p50/p95/p99 ms: exact 27.750/32.206/39.736, prefix 61.983/67.694/68.776, fuzzy 458.443/478.590/497.988, filtered 60.065/66.107/78.889, relationship 130.016/151.995/158.008, content 145.325/155.821/184.094. Evidence: `docs/evidence/search-latency-matrix-20260929.json`.

## Historical validation lab state (superseded)

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
- The first direct physical 6-hour soak attempt at source `675aabb` ran 6381.94 s / 67,248 operations / 2,800 checks and exposed a base-family publication race (`failed to fill whole buffer`). A dedicated shared/exclusive publish snapshot lock was added and deterministic torn-family regressions pass. Evidence: `docs/evidence/soak-6h-publish-race-20260929.json`.
- The fixed-build rerun at `daad45d` reproduced the same raw I/O error after 136.24 s / 912 operations / 36 checks. With base-family publication already protected, this isolated the remaining race to the append-only `.delta` overlay. `read_delta_record` previously accepted `UnexpectedEof` as a clean tail only at the opcode byte; EOF in later fields escaped as `failed to fill whole buffer`. Current source treats `UnexpectedEof` anywhere in the final append record as an uncommitted/crash tail while fully-readable invalid operation/name-length/UTF-8 stays fail-closed. A restarted writer also scans to the last complete record and truncates only an incomplete final tail before appending, preventing a crash tail from absorbing bytes from a later record. Three deterministic tail regressions PASS. Post-failure `doctor` + `verify-deep` also PASS. Evidence: `docs/evidence/soak-6h-delta-tail-race-20260930.json`.
- At that checkpoint, source passed 117 workspace tests (74 core + 7 platform + 9 CLI + 17 GUI + 4 service + 6 worker), fmt, clippy `-D warnings` and release build. Publish-snapshot protection is owned by `SearchStore::open()` itself, so search, doctor/verify, metadata/content builders, benchmark paths and every other base-family reader open one coherent generation. `LiveSearchStore` keeps only delta-overlay responsibilities. Both direct-SearchStore and LiveSearch torn-family regressions PASS. The GUI is a Windows Search-style resident panel with file/folder/content modes, owner-drawn results, Mica/Acrylic, native live theme controls, single-instance query IPC, supported `search:` / `searchtool:` protocol paths, and per-monitor DPI/topology recovery. The advanced `%APPDATA%\SearchTool\ui.conf` remains available for palette/size overrides. The exact `fa92628` source-freeze soak is PASS: 21,873.82 s / 223,632 operations / 9,318 checks with intentional crash/restart, exact source/service identity, post-run doctor + verify-deep PASS, and SearchToolIndexer Running + Automatic. Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`.
- The first independent Task Scheduler short-soak on cfe3fef completed 2,496 filesystem operations / 104 checks with intentional service crash/restart and never reproduced the raw I/O race. Its only failure was final generic soak-g cleanup: an older SentinelX-killed invalid run had left C:\.search-tool-soak-3a53ba3d844d41d58e3b344e3cd188a8 physically on disk with the same repeated filenames. NTFS journal evidence showed separate old/new file IDs. Search Tool was correct to return that real directory. The harness now embeds a unique run-id in every workload filename and reports the run-id/test-root, preventing cross-run false positives.
- The isolated follow-up at source `6a22593` then passed end-to-end with the run-scoped harness: 273.19 s / 2,688 operations / 56 validation checks, BatchSize=64, intentional service crash/restart exercised, no recurrence of `failed to fill whole buffer`, and sealed post-check `doctor` + `verify-deep` PASS with SearchToolIndexer Running + Automatic. Evidence: `docs/evidence/short-soak-isolated-20260930.json`.
- External validation harnesses are explicit. `pristine-validation.ps1` has a clean hosted-Windows PASS: default Program Files/ProgramData install, production SCM Auto/Running, initial index/search/smart/doctor, GUI + scoped GUI, `search:` / `searchtool:` / App Paths / RegisteredApplications / OpenWith / Explorer verbs, then purge uninstall with 14/14 zero-residue checks. Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`. Earlier hosted Defender readiness remained BLOCKED/UNAVAILABLE because real-time/behavior protection was disabled; a later isolated hosted activation/custom-scan run `36972721866` produced final Defender PASS evidence. Historical hosted SmartScreen readiness proved MOTW `ZoneId=3` but could not expose an enabled policy or interactive protective outcome; final SmartScreen validation later passed physically. Per-monitor DPI/topology logic is hardened and release-gate validated at `d01b271`, while the physical display probe remains BLOCKED at one 1600x900 96-DPI monitor. `display-validation.ps1` and credential-backed `web-resolver-validation.ps1` still require their final external environments; Defender and SmartScreen final validation are complete. `.github/scripts/external-validation-orchestrator.ps1` now aggregates those four gates into one JSON summary with explicit PASS/READY/BLOCKED/PARTIAL/FAIL semantics; it never upgrades a blocked prerequisite into PASS, and CI smoke-tests both the all-skip aggregation path and credential-first Web Resolver preflight. Missing Google key/CX is reported before CLI existence is required, so source-only physical checkouts no longer need a placeholder file to identify the real blocker. Defender readiness behaves the same way when `-DefenderCustomScan:$false`: it probes active protection against the repository root and does not require `target\release`; final custom-scan evidence still requires the release binary directory. A GitHub-hosted Windows probe (run `36852274027`) also confirmed that repository secrets `SEARCH_TOOL_GOOGLE_KEY` and `SEARCH_TOOL_GOOGLE_CX` are both absent; evidence: `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`. A current-main physical preflight on `ed65fbc` then exercised the new orchestrator on `DESKTOP-ONDD84S`: aggregate `BLOCKED`, 0 FAIL, Defender `UNAVAILABLE`, display `BLOCKED`, Web Resolver `BLOCKED`, SmartScreen deliberately skipped because the candidate ZIP was not present. Follow-up diagnosis showed `WinDefend` and `WdNisSvc` Stopped/Disabled with Defender policy `DisableAntiSpyware=1` and `DisableAntiVirus=1`; one 1600x900 @ 96-DPI display was active; both Google credential-presence checks were false; no enabled SmartScreen machine/user/policy setting was exposed. Supplemental evidence: `docs/evidence/external-validation-physical-preflight-ed65fbc-20261001.json`. The exact sealed ZIP was then downloaded from release-gate run `36848221272`, rehashed to `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`, copied, and given a staged Internet-zone `Zone.Identifier` ADS. The ZIP byte hash and size remained unchanged, `motw_internet_zone=true`, and the artifact remained unsigned/untrusted; SmartScreen still returned BLOCKED solely because `effective_enabled=null` and no interactive Warned/Blocked outcome was observed. Supplemental readiness evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`. On the later current `main` `d376b244165c067be951ae45e0f9d5ea51129e3a`, the orchestrator was rerun with all four external gates enabled and the exact sealed/MOTW ZIP: aggregate `BLOCKED`, 0 FAIL, 0 skipped; Defender `UNAVAILABLE`, SmartScreen `BLOCKED`, display `BLOCKED`, Web Resolver `BLOCKED`. That run is historical pre-PASS SmartScreen context. A subsequent physical test on `f32212604c86079fdfd45ea60bffa353117568b6` temporarily enabled SmartScreen `Warn`, launched MOTW-marked `search-tool-gui.exe`, captured SmartScreen Event 1000 with `Enforcement=warnByPolicy` / `Experience=Untrusted`, and produced validator `ObservedOutcome=Warned` / PASS; rollback restored the prior host configuration. Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`. On then-current `main` `eae87f121f7b6333e0919696f0678649b0228e56`, release-state verification returned PASS and the orchestrator was rerun with completed SmartScreen intentionally skipped: aggregate `BLOCKED`, 3 unresolved gates, 0 FAIL, 1 skipped; Defender was still `UNAVAILABLE` at that historical checkpoint, display was `BLOCKED` at one 1600x900 @ 96-DPI monitor, and Web Resolver was `BLOCKED` on absent key/CX before CLI existence mattered. Defender was subsequently completed by hosted run `36972721866`. Evidence: `docs/evidence/external-validation-three-unresolved-eae87f1-20261002.json`.

## Immediate continuation order

1. **PR #62 is merged and deployed.** Main `ea0ed06afa0df65eb4ca75661a0ca0235a865fa6` and CI `37442561672` are SUCCESS. Physical Windows upgrade and three-verb cleanup are PASS; evidence is recorded above.
2. **Do not rebuild/reseal unless packaged inputs change.** Artifact `11396301339` remains canonical (SHA-256 `4D7D0AF28CA1B8DC01BABB644F93CAB0133034205280F057ED037B8CB2F3734F`). Docs-only evidence updates remain inside the allowlist.
3. **Sole remaining external gate: mixed-DPI.** Test two real active monitors at distinct effective DPI; do not promote `mixed_dpi` based on simulation or virtual/session displays.

For the full backlog see `docs/ROADMAP.md`. For evidence and exact PASS/BLOCKED states see `docs/TEST_MATRIX.md`.
## Safe resume rule

Before mutating a real index, always run:
`search-tool doctor <index-dir>`
and inspect whether an SCM SearchToolIndexer instance is running. `maintain` intentionally refuses unsafe concurrent mutation. Use isolated VHDs for destructive USN/journal recovery tests.

### 2026-10-02 credential-first Web Resolver physical recheck

On physical host DESKTOP-ONDD84S, checkout 218ac7eadb72412be0e68bccf11368d48b724faf passed release-state-check.ps1. A Web-Resolver-only orchestrator run then intentionally used the default release CLI path while that binary did not exist. The gate still returned BLOCKED for missing SEARCH_TOOL_GOOGLE_KEY / SEARCH_TOOL_GOOGLE_CX before CLI availability mattered, with failed=0, blocked=1, skipped=3, and no secret values recorded. Supplemental evidence: docs/evidence/web-resolver-credential-first-218ac7e-20261002.json. Canonical blocker evidence/blob seals and the validated package seal are unchanged.

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


## Mixed-DPI final physical gate finalizer

The remaining external blocker can now be driven by .github/scripts/mixed-dpi-finalizer.ps1 without changing packaged inputs. The finalizer first verifies docs/RELEASE_STATE.json is VALIDATED, requires mixed_dpi to be the only unresolved blocker, verifies SmartScreen/Defender/Web Resolver are completed, and for live stages verifies the exact sealed ZIP SHA-256 + byte size before extracting search-tool-gui.exe.

Sequence once two real active displays with distinct effective DPI are available:

1. Run: .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode Probe
2. Place the exact sealed SearchTool-Windows-x64.zip at dist\SearchTool-Windows-x64.zip (or pass -PackageZip), then run -Mode Exercise.
3. Run -Mode PreparePrimaryChanged, change which active display is Windows primary without closing the GUI, then run -Mode VerifyPrimaryChanged.
4. Run -Mode PrepareMonitorRemoved. The helper selects a non-primary target, launches the sealed GUI, moves it onto that display, records the exact GUI PID and target monitor, and prints the display to physically disconnect.
5. Physically disconnect that target display while the GUI remains alive, then run -Mode VerifyMonitorRemoved.
6. Run -Mode Bundle. PASS requires cross-monitor move/DPI checks, observed primary change, observed real monitor removal, exact GUI survival/recovery, active-monitor DPI match, and proof that the window was actually on the removed monitor.

-Mode SelfTest provides a deterministic synthetic positive bundle plus a negative provenance case and must stay PASS in CI.

## Final mixed-DPI promotion automation

After the physical finalizer produces a PASS bundle, use `.github/scripts/mixed-dpi-promote.ps1` to validate and stage the final release-state promotion. The promotion helper verifies the bundle HEAD/package identity, every required PASS check, every subordinate evidence SHA-256 and monitor-removal package identity before it writes anything. Apply mode creates canonical PASS evidence under `docs/evidence/`, moves `mixed_dpi` from the sole external blocker to a completed PASS gate, updates the release-state checker/self-test for the zero-blocker final state, and appends the final evidence markers to every synchronized release document. It never commits or merges automatically; exact-head CI is still required after the staged changes are committed.

Promotion sequence:

    .\.github\scripts\mixed-dpi-promote.ps1 -Mode Validate -Bundle <mixed-dpi-final-bundle.json>
    .\.github\scripts\mixed-dpi-promote.ps1 -Mode Apply -Bundle <mixed-dpi-final-bundle.json>
    git add .github docs
    git commit -m "Promote final mixed-DPI physical PASS"
    .\.github\scripts\release-state-check.ps1
    .\.github\scripts\release-state-selftest.ps1

`-Mode SelfTest` creates an isolated temporary worktree, rejects an intentionally invalid bundle, performs a synthetic full promotion, commits it locally inside the disposable worktree, and requires the promoted release-state checker and promoted fail-closed self-test to PASS.

## Interactive desktop provenance requirement

Final mixed-DPI evidence must be collected from a real interactive Windows user desktop. The finalizer now records desktop context, blocks non-interactive execution and Windows Session 0/service context, and rejects non-standard display devices instead of treating service/session surfaces as physical topology. This specifically prevents SentinelX/Windows-service execution from turning a `WinDisc`-style 1024x768 session surface into release evidence. Live Exercise/Prepare/Verify stages must be run in the logged-in interactive desktop session; `Bundle` and `SelfTest` may run non-interactively because they only validate already-produced evidence.

Final mixed-DPI evidence provenance is now end-to-end: Exercise/PrimaryVerify/RemovalVerify reports embed interactive desktop context and active monitor device identities, removal metadata embeds its prepare context/target, Bundle requires those fields to be valid, and the promotion helper independently reopens and validates the subordinate evidence before staging the zero-blocker release state.

## Versioned interactive mixed-DPI launcher

The live physical workflow now has a version-controlled launcher under `.github/scripts/mixed-dpi-live-launcher.ps1` with `.cmd` wrapper support. The launcher derives the expected package SHA-256 and byte size from `docs/RELEASE_STATE.json`, requires the current clean `main`, verifies the sealed ZIP before live stages, rejects non-interactive/Session 0 execution, and exposes the full Probe -> Exercise -> primary-change -> monitor-removal -> Bundle -> Promotion flow. `-Mode SelfTest` runs in CI without requiring a real monitor. Any Desktop shortcut/copy should delegate to this repository launcher rather than duplicating release logic.


## 2026-10-04 refreshed interactive mixed-DPI blocker evidence

- Current authoritative unresolved blocker: `mixed_dpi`.
- Evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`.
- Evidence Git blob: `71da68834378b99dd8fdbdf7687378f664f722bf`.
- Collected through the versioned interactive live launcher from Windows user session 1 on `DESKTOP-ONDD84S`; `user_interactive=true`, standard `\\.\DISPLAY1` device naming verified.
- Physical topology at collection time: one active 1600x900 display, 96 DPI / 100%, one distinct effective DPI value.
- Result remains `BLOCKED` because at least two real active displays with distinct effective DPI values are required.
- Package seal remains unchanged: source `3dfe4ab4ae381c6e5fc8720e76254be0b3f8659d`, SHA-256 `5639177286DEEBBC6794CCAE9475E02C88CF05F001693484643EC8CE7D6ABA57`, 1,894,905 bytes.

## Remote interactive-session bridge for mixed-DPI validation

SentinelX runs as a non-interactive Windows service and must never be treated as physical display evidence directly. `.github/scripts/mixed-dpi-interactive-task.ps1` bridges that service context into the currently logged-in Windows user's real interactive session by registering a temporary Task Scheduler job with `InteractiveToken`, running one finalizer mode, waiting for completion, and deleting the task/runner artifacts. The helper supports Probe, Exercise, Prepare/VerifyPrimaryChanged, Prepare/VerifyMonitorRemoved and Bundle. `-Mode SelfTest` is CI-safe and does not create an interactive task. On DESKTOP-ONDD84S the helper was physically verified to run as user `umut`, `UserInteractive=true`, `SessionId=1`, and to observe the real `\\.\DISPLAY1` topology instead of the Session 0 WinDisc surface.

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

## Distinctive UI v1 physical release gate

- Physical release-gate evidence: `docs/evidence/windows-release-gate-physical-distinctive-ui-5cf797b-20261004.json`.
- Evidence Git blob: `b7b145bb9bbd5441bdee6889a2af5830b22b2dd5`.
- Physical host: `DESKTOP-ONDD84S`.
- Source head: `5cf797bf21b89d03760c803526afef84b7096c31`.
- Result: **PASS** for fmt, clippy, 126 tests, release build, CLI smoke, NTFS/USN/service, USN reset recovery, package build/integrity, clean install/uninstall and Defender interaction.
- The local physical ZIP is supplemental runtime evidence only; the authoritative release seal remains hosted artifact `11312471590` with SHA-256 `7D84E45B4D7018929200F802226C1A4C23EA7235CB0ADBF75BB89D9C13743958` and 1,903,718 bytes.

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


## 2026-10-07: content sidecar I/O failure cleanup, reseal pending

Evidence: `docs/evidence/windows-physical-enospc-content-staging-20261007.json`. A physical Windows 10 host reached 0 free bytes on C:. Recovery removed only generated Rust debug outputs and 312 stale content sidecar temporary files (2,077,846,240 logical bytes), while preserving the published content index, its checkpoints, installed binaries and NTFS permissions.

`ContentIndexBuilder` now tracks chunk spill files before writes and cleans its chunks, content staging and checkpoint staging when dropped after an incomplete or failed build. Two new regression tests verify cleanup and preservation of the previously published index. On the physical Windows host, 141/141 workspace tests, Clippy and Release compilation PASS.

`docs/RELEASE_STATE.json` is intentionally `INVALIDATED` because packaged runtime source changed. The installed Search-Tool binaries still contain the previous sealed version. Do not claim the new fix is deployed, or update the package to VALIDATED, before exact-head CI, a new hosted Windows package/release gate, new evidence and physical deployment validation. Mixed-DPI remains physically BLOCKED; native Windows Search and Explorer backend ownership remains Microsoft's.


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
