# Search Tool Test Matrix

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

Legend: PASS = exercised successfully. PARTIAL = path works but final evidence is incomplete. BLOCKED = environment dependency unavailable. TODO = not yet exercised to the desired release standard.

| Area | Status | Evidence / note |
|---|---|---|
| cargo fmt | PASS | Latest Windows release gate |
| cargo clippy -D warnings | PASS | Latest Windows release gate |
| workspace unit tests | PASS | 137 tests total: 76 core + 9 platform + 14 CLI + 28 GUI + 4 service + 6 worker |
| Windows release build/link | PASS | Physical Windows x64 |
| CLI smoke | PASS | Release gate |
| Release-state consistency | PASS | Current `VALIDATED` state pins PR #62 package source, SHA-256, bytes, new exact-head CI evidence, 137-test count, immutable evidence blob hashes, required gates and synchronized documents. Checker/self-test PASS in Windows PowerShell 5.1 and PowerShell 7; historical releases remain historical. |
Current unresolved-blocker evidence Git blob seal: `mixed_dpi=71da68834378b99dd8fdbdf7687378f664f722bf`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`, `web_resolver=cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
Core release evidence seals: package `docs/evidence/windows-release-gate-pr59-start-menu-native-first-37357508957-20261005.json` blob `06aa43ce158ab72cd5cab15f86ac3307fa54e152`; physical deployment `docs/evidence/windows-physical-native-first-startmenu-final-11365148367-20261005.json` blob `09baf9a8c1caeb17d55e7e3fcddec5b0266877a5`; physical runtime `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
| Initial NTFS MFT index | PASS | Isolated VHD + real C: |
| USN incremental sync | PASS | Isolated VHD + real C: service; steady-state create/rename/delete probe 4/4 PASS |
| Journal reset/truncation recovery | PASS | Isolated VHD only |
| Deep verify / repair | PASS | Real and isolated indexes |
| Multi-volume search | PASS | D:/E: validation indexes |
| External delta compaction | PASS | 140k+ delta stress exercised |
| Base-family publish snapshot | PASS | Shared/exclusive publish locking is enforced inside `SearchStore::open()`, so every direct reader (live search, verify/doctor, metadata/content builders, benchmarks) opens main + sidecars from one generation. A direct SearchStore regression and the LiveSearch regression both deliberately expose a truncated main file behind the exclusive publish lock and prove the reader waits, then opens the restored family successfully. |
| Delta partial-tail concurrency + crash reopen | PASS | `read_delta_record` treats `UnexpectedEof` anywhere in the final variable-length append record as an uncommitted tail; fully-readable invalid op/name-length/UTF-8 stays fail-closed. `DeltaWriter::open` scans complete records and truncates only an incomplete final crash tail before new append. Partial fixed-header, filename-tail and reopen-then-append regressions PASS. |
| Soak run isolation | PASS | Initial scheduled run proved the old generic cleanup could collide with a real leftover `.search-tool-soak-*` directory. Harness now embeds a unique run-id in every workload filename, final absence query is run-scoped, and reports include run-id/test-root. Isolated follow-up PASS: 273.19 s / 2,688 ops / 56 checks, BatchSize=64, crash/restart exercised, doctor + verify-deep PASS. Evidence: `short-soak-cross-run-contamination-20260930.json`, `short-soak-isolated-20260930.json`. |
| Mutation lock | PASS | Unit + runtime guard |
| Metadata filters | PASS | VHD + real indexes |
| Plain content index | PASS | VHD integration |
| DOCX extraction | PASS | Worker runtime |
| XLSX extraction | PASS | Worker runtime/fallback |
| PPTX extraction | PASS | Worker runtime/fallback |
| PDF extraction | PASS | Built-in fallback end-to-end |
| Hostile parser-worker matrix | PASS | Commit `e2be944`: hang/crash/partial stdout/invalid UTF-8/oversized response; corrupt + encrypted PDF; corrupt OOXML; 8,193-entry OOXML bomb; 256 MiB Job Object |
| Transactional installer abrupt-exit matrix | PASS | Isolated upgrade matrix covers 10 abrupt-exit boundaries through shortcut/integration phases while preserving the live production service. |
| Registry rollback snapshot JSON round-trip | PASS | Exact code head `5ec0a74` passed CI run `36786244058`. The disposable HKCU self-test round-trips String/default, ExpandString, MultiString, DWord, QWord, Binary, zero-length Binary, nested keys, missing values and absent trees through JSON. A physical minimal repro also confirmed `New-Item -Force` erases values on an existing registry key; rollback now creates only missing keys. |

| Cross-volume duplicate detection | PASS | Physical validation |
| Quarantine -> restore -> purge | PASS | Physical validation |
| Cleanup protected-path deny | PASS | Unit/runtime |
| Tiny intent router | PASS | Unit/runtime |
| Web resolver sanitizer/cache | PASS | Unit tests |
| Web resolver real success request | PASS | API-keyless local SearXNG provider success -> credential-free cache hit -> parent-path privacy PASS against the packaged release. Evidence: `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json`; blob `cb239296533c381ce32f59f36ad2b1e9a016d4e0`. |
| Native Win32 GUI startup | PASS | Physical Windows |
| Per-monitor DPI/topology logic | PASS | Runtime source `d01b271`: handles `WM_DPICHANGED`, Win32 suggested RECT, DPI-scaled fonts/layout/rows, nearest-monitor work area and display/work-area recovery. Deterministic tests cover 96/144/192 DPI, negative monitor origins, removed-monitor recovery and oversized clamping; exact-head CI `36840720835` + physical release gate PASS. |
| Single instance / resident mode | PASS | Physical Windows |
| Native Windows theme controls | PASS | `search-tool theme` reads/writes Windows Personalization/DWM state for light/dark/mixed app/system mode, transparency, accent color and accent surfaces. Physical readback confirmed live application; Search/Explorer/Start remain Windows-drawn surfaces. |
| PR #62 physical Explorer migration | PASS | Merged main `ea0ed06` and main CI `37442561672` SUCCESS; 3/3 old Explorer verbs removed; normal-user doctor/search, Session 1 GUI, service/index, native theme, and 4/4 binary hashes PASS. Evidence: `docs/evidence/windows-physical-pr62-explorer-native-first-11396301339-20261006.json`. |
| Native-first Windows Search + Explorer integration | PASS | Win, taskbar Search and Explorer keep Windows ownership. Resident startup defaults to `--no-shell-bridge`; installer does not register `SearchTool.Search`, `search:` OpenWith, Capabilities or RegisteredApplications ownership. Private `searchtool:` and explicit scoped Explorer commands remain available; legacy keyboard bridge is opt-in only. |
| Ctrl+Alt+Space fallback hotkey | PASS | Real key injection hide/show |
| Alt+Space primary hotkey | EXPECTED FALLBACK | Windows reserves/conflicts on host |
| Multi-monitor mixed-DPI | BLOCKED | Runtime handling and deterministic topology/DPI tests PASS. Latest real interactive probe exposes one active `\\.\DISPLAY1` at 1600x900 / 96 DPI / 100%; final gate requires at least two real active displays with distinct effective DPI and rejects virtual/session displays. Canonical blocker evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`. |
| Windows SCM service | PASS | Real C: SearchToolIndexer Running + Automatic, service_sync=Ok, last_error=0 |
| Crash/restart soak | PASS | 15-minute soak |
| 15-minute soak | PASS | Two runs; one ~914 s / 14,352 ops / 598 checks |
| 60-minute soak | PASS | Commit `709cc275`: 3613.77 s / 56,496 ops / 2,354 checks / 15.63 ops/s; crash-restart exercised; peak service working set 5.199 MiB; final D: cleanup delta=0, metadata/sizes/content fresh, verify-deep PASS. |
| Real 1M+ search latency matrix | PASS | 1,209,697-record frozen C: index, 5 warmups + 100 rounds/class. p50/p95/p99 ms: exact 27.750/32.206/39.736; prefix 61.983/67.694/68.776; fuzzy 458.443/478.590/497.988; filtered 60.065/66.107/78.889; relationship 130.016/151.995/158.008; content 145.325/155.821/184.094. Evidence `search-latency-matrix-20260929.json`, source `322fb4e`. |
| 6-hour soak | PASS | Exact frozen source `fa92628`; 21,873.82 s / 223,632 ops / 9,318 checks / 10.22 ops/s, intentional crash/restart exercised, peak service WS 7.461 MiB, source/service identity PASS, post-run doctor + verify-deep exit 0, service Running/Automatic. Evidence: `soak-6h-fa92628-final-20260930.json`. |
| Foreground-impact | PASS | Release-freeze run `215e6bc` PASS. Current-main recheck `6bbde9c` also PASS: 85 baseline samples p95 151.442 ms -> 143 stressed samples p95 176.258 ms (+24.816 ms, 1.164x); nested real-service soak PASS with 264 ops / 22 checks / 230.86 s, then doctor + verify-deep PASS and service Running/Automatic. Evidence: `foreground-impact-current-head-20260929.json`. |
| Clean install/uninstall smoke | PASS | Release gate |
| Pristine default-path machine flow | PASS | Native-first Start Menu cleanup package run `37357508957`, pristine artifact `11366210455`: default Program Files/ProgramData install, SearchToolIndexer Running/Automatic, index/search/doctor PASS, system `search:` ownership left untouched, private `searchtool:` retained, and purge uninstall returned the machine to zero Search Tool residue. Package evidence: `docs/evidence/windows-release-gate-pr59-start-menu-native-first-37357508957-20261005.json`. The pristine install also verifies that no separate Start Menu custom-search shortcut is exposed. |
| Upgrade preserve/purge | PASS | Physical validation |
| Defender active scan | PASS | Hosted run `36972721866` activated real-time + behavior protection for the isolated probe, verified the exact sealed candidate identity, and custom-scanned it with `new_related_detections=0` and no scan exception. All preference restoration attempts succeeded. Evidence: `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`; Git blob `3349e503636f5c9c0a2613892b62c5bac15b0e02`. Earlier physical/hosted BLOCKED evidence remains historical provenance only. |
| SmartScreen | PASS | Final physical validation used the exact sealed release ZIP (`0A48E178...E65`, 1,888,674 bytes), extracted `search-tool-gui.exe`, staged `ZoneId=3` MOTW, temporarily enabled SmartScreen `Warn` policy, and launched the unsigned binary. SmartScreen Debug Event 1000 reported `Enforcement=warnByPolicy` and `Experience=Untrusted`; `smartscreen-validation.ps1` then recorded `ObservedOutcome=Warned` and PASS. Temporary policy/log changes were rolled back and no SmartScreen/GUI process remained. Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`. Historical hosted/readiness BLOCKED evidence remains preserved for provenance. |
| Sleep/resume | PASS | Real C: controlled sleep/resume; pre/post markers visible, boot session unchanged, checkpoint advanced, service Running/Automatic, doctor + verify-deep PASS; `power-cycle-sleep-20260929.json` |
| Reboot recovery | PASS | Real reboot: boot session changed, SearchToolIndexer auto-started Running/Automatic, pre/post markers visible, checkpoint advanced, service_sync=Ok, doctor + verify-deep PASS; 45 s harness false-negative reproduced then fixed with configurable 120 s catch-up window |
| Compaction publish kill-point | PASS | 11 deterministic abrupt-process-exit boundaries exercised; mixed-generation publish bug fixed; verify-deep + retry compaction + debris cleanup PASS |

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

## Latest full Windows release gate

Date: 2026-10-01

Result: **PASS**

Steps:
- release preflight PASS
- cargo fmt PASS
- cargo clippy PASS
- cargo test PASS (117 workspace tests)
- release build PASS
- CLI smoke PASS
- NTFS/USN/service integration PASS
- USN journal reset recovery PASS
- portable package build PASS
- portable package integrity PASS
- clean install/uninstall smoke PASS
- Defender interaction step PASS, but Defender itself reported unavailable/disabled

Current packaged source: `6c4141d0bcf12ade21cf633fbaf42d361eb12977`

Exact-head CI: `36847421304` SUCCESS on Windows + Ubuntu

Full hosted release-gate run: `36848221272` SUCCESS; validation wrapper `79f061e09b8d0677ec67532ac0142a6e3d8449cc` differs only by the temporary workflow file. The prior physical runtime gate remains `d01b271` / `36840720835`.

Current package SHA-256:
`0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65` (1,888,674 bytes; artifact seal re-hash PASS)

Final release-gate evidence:
- `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` (current packaged candidate)
- `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` (prior physical runtime gate)
- `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`
- `docs/evidence/windows-release-gate-4700a6c-final-20261001.json` (prior release baseline)
- `docs/evidence/pristine-default-path-hosted-20261001.json`
- `docs/evidence/defender-hosted-blocked-20261001.json`
- `docs/evidence/smartscreen-hosted-blocked-20261001.json` (historical BLOCKED provenance)
- `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` (current completed SmartScreen PASS)

Soak hardening evidence:
- `docs/evidence/soak-failure-diagnosis-20260928.json`
- `docs/evidence/soak-ownership-regression-20260928.json`
- `docs/evidence/soak-fast-verify-failure-20260928.json`
- `docs/evidence/soak-fast-verify-regression-20260928.json`
- `docs/evidence/soak-60m-709cc275-20260928.json`
- `docs/evidence/soak-6h-fa92628-final-20260930.json`
- `docs/evidence/windows-release-gate-soak-hardening-20260928.json`

Real C: recovery/service evidence:
- `docs/evidence/real-c-recovery-service-20260929.json`

Installer transaction evidence:
- `docs/evidence/install-transaction-safe-smoke-20260929.json`

Compaction crash-consistency evidence:
- `docs/evidence/compaction-fault-injection-20260928.json`

Current candidate package SHA corresponds to packaged-source `6c4141d0bcf12ade21cf633fbaf42d361eb12977`. Later docs/evidence-only commits do not change the package bytes. The physical runtime gate remains tied to `d01b271`. Rebuild and rerun the full release gate after any packaged source/input change.

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


## 2026-10-07 physical recovery and content-builder fault regression

| Test / gate | Current result | Evidence |
| --- | --- | --- |
| Builder dropped before finish | PASS | Tracks and removes temporary spill chunks without waiting for another reindex |
| Builder fails while creating checkpoint staging | PASS | Removes partial staging/chunks and preserves published content-search results |
| Windows workspace + Clippy + Release build | LOCAL PASS (141/141) | `docs/evidence/windows-physical-enospc-content-staging-20261007.json` |
| Newly packaged runtime release | INVALIDATED pending reseal | Previously installed binary and published index remain unchanged |
| Mixed effective DPI | BLOCKED | A second real active monitor with distinct DPI is still required |


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


## 2026-10-07 Windows 11 physical portable coverage
| Test / gate | Result | Evidence |
| --- | --- | --- |
| Exact canonical package hash on Windows 11 | PASS | `3D293843BE71D322CFF9729C5678401CAB1FFB0510BBDD897328459531CE40A2` |
| Windows 11 exact-release GUI render/focus/input | PASS | `docs/evidence/windows11-physical-portable-gui-20261007.json` |
| Synthetic index verify/search | PASS | 100,000 records; `status=ok`; `node` query PASS |
| Portable cleanup/no persistent integration | PASS | No service, protocol/ProgID or Startup shortcut remained |
| Windows 11 service/install path | NOT RUN | Admin elevation intentionally not bypassed; hosted/Windows 10 install gates already cover installer behavior |
| Mixed-DPI physical exercise | BLOCKED | Both available real Windows hosts currently expose only one active monitor |


## 2026-10-07 themed Görünüm release coverage
| Test / gate | Result |
| --- | --- |
| Workspace tests | PASS (141/141) |
| Clippy / Release build | PASS |
| PR #71 exact-head CI | PASS (`37613524060`) |
| Merged-main CI | PASS (`37614290344`) |
| Windows release gate | PASS (`37616034729`) |
| Exact canonical package verify | PASS (`E29D50B4...6252F`) |
| Windows 11 exact-artifact GUI/render/search | PASS (39 results / 54.7 ms) |
| Production transactional upgrade | PASS |
| Production installed binary hashes | PASS / match artifact |
| Production normal-user verify/search | PASS |
| Production physical-console screenshot after deploy | BLOCKED by unapproved local console grant; not claimed PASS |
| Mixed-DPI physical exercise | BLOCKED; two real distinct-DPI displays still required |


### Release-state synchronized values (2026-10-07 themed Görünüm seal)
- packaged_source_sha: `6d703dc3ae20a5cb0f95d45323338832ac1a2554`
- package_sha256: `E29D50B4BFD0B5AFD523C98BAE3691B167D4C099E53F12736F90CC182516252F`
- package_evidence: `docs/evidence/windows-release-gate-theme-button-37616034729-20261007.json`
- package_evidence_blob_sha: `14456cd92fe777144181d3f19dce308aa81ab80e`
- physical_gate_evidence: `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json`
- physical_gate_evidence_blob_sha: `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`
- workspace_test_count: `141`
- six_hour_soak_evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`
- six_hour_soak_evidence_blob_sha: `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`
- mixed_dpi_evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`
- smartscreen_evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`
- defender_evidence: `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`
- web_resolver_evidence: `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json`
