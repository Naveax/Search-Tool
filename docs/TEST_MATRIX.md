# Search Tool Test Matrix

Last updated: 2026-10-05.

## Native-first authoritative package — 2026-10-05

This section supersedes earlier statements that describe an older package as the current release. Older sections remain historical provenance.

- Package status: **VALIDATED**.
- Packaged source: `f4eb2195d672dc69a44232bc32cca056d8c0a974`.
- Exact-head/full CI: `37324593250` — Windows + Ubuntu SUCCESS.
- Windows package artifact: `11351777183` (`SearchTool-Windows-x64`).
- Pristine validation artifact: `11351677146` — PASS.
- Sealed ZIP SHA-256: `8B052E37AC1B3428A6688604A5343570944AC2C13A8C8818ACDF0775CD2D224F`; size **1,920,732 bytes**.
- Package evidence: `docs/evidence/windows-release-gate-pr56-native-first-37324593250-20261005.json`; Git blob `8529544fd6e5cbab6ff52d4db6cd8e3436248e3f`.
- Workspace validation: **137 tests PASS** (76 core + 9 platform + 14 CLI + 28 GUI + 4 service + 6 worker), fmt/clippy/release build PASS.
- Native shell policy: Win, taskbar Search and File Explorer search stay on Microsoft's own Windows UI. Resident startup uses `--no-shell-bridge`; the legacy keyboard bridge is opt-in only via `--shell-bridge`.
- Native search ownership: installer does not register `SearchTool.Search`, `search:` OpenWith, Capabilities or RegisteredApplications ownership. The private `searchtool:` protocol and explicit scoped Explorer command remain available.
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
| Release-state consistency | PASS | Current `VALIDATED` state pins package source/SHA/size/evidence, 137-test count, one unresolved blocker (`mixed_dpi`), completed gates (`smartscreen`, `defender`, `web_resolver`), synchronized docs, post-package allowlist and transient validation path. Fail-closed selftest rejects packaged-input drift, stale seals/counts, false gate promotion, evidence swaps/tampering and structural-set drift. |
Current unresolved-blocker evidence Git blob seal: `mixed_dpi=71da68834378b99dd8fdbdf7687378f664f722bf`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`, `web_resolver=cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
Core release evidence seals: package `docs/evidence/windows-release-gate-pr56-native-first-37324593250-20261005.json` blob `8529544fd6e5cbab6ff52d4db6cd8e3436248e3f`; physical deployment `docs/evidence/windows-physical-native-first-final-11351777183-20261005.json` blob `4eda606f3d935f57ee34f898d04fb9e1c0a3c426`; physical runtime `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
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
| Pristine default-path machine flow | PASS | Native-first package run `37324593250`, pristine artifact `11351677146`: default Program Files/ProgramData install, SearchToolIndexer Running/Automatic, index/search/doctor PASS, system `search:` ownership left untouched, private `searchtool:` retained, and purge uninstall returned the machine to zero Search Tool residue. Package evidence: `docs/evidence/windows-release-gate-pr56-native-first-37324593250-20261005.json`. |
| Upgrade preserve/purge | PASS | Physical validation |
| Defender active scan | PASS | Hosted run `36972721866` activated real-time + behavior protection for the isolated probe, verified the exact sealed candidate identity, and custom-scanned it with `new_related_detections=0` and no scan exception. All preference restoration attempts succeeded. Evidence: `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`; Git blob `3349e503636f5c9c0a2613892b62c5bac15b0e02`. Earlier physical/hosted BLOCKED evidence remains historical provenance only. |
| SmartScreen | PASS | Final physical validation used the exact sealed release ZIP (`0A48E178...E65`, 1,888,674 bytes), extracted `search-tool-gui.exe`, staged `ZoneId=3` MOTW, temporarily enabled SmartScreen `Warn` policy, and launched the unsigned binary. SmartScreen Debug Event 1000 reported `Enforcement=warnByPolicy` and `Experience=Untrusted`; `smartscreen-validation.ps1` then recorded `ObservedOutcome=Warned` and PASS. Temporary policy/log changes were rolled back and no SmartScreen/GUI process remained. Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`. Historical hosted/readiness BLOCKED evidence remains preserved for provenance. |
| Sleep/resume | PASS | Real C: controlled sleep/resume; pre/post markers visible, boot session unchanged, checkpoint advanced, service Running/Automatic, doctor + verify-deep PASS; `power-cycle-sleep-20260929.json` |
| Reboot recovery | PASS | Real reboot: boot session changed, SearchToolIndexer auto-started Running/Automatic, pre/post markers visible, checkpoint advanced, service_sync=Ok, doctor + verify-deep PASS; 45 s harness false-negative reproduced then fixed with configurable 120 s catch-up window |
| Compaction publish kill-point | PASS | 11 deterministic abrupt-process-exit boundaries exercised; mixed-generation publish bug fixed; verify-deep + retry compaction + debris cleanup PASS |

## Native-first physical deployment — 2026-10-05

- Physical host: `DESKTOP-ONDD84S`.
- Deployed sealed artifact: `11351777183`; package source `f4eb2195d672dc69a44232bc32cca056d8c0a974`.
- Package SHA-256: `8B052E37AC1B3428A6688604A5343570944AC2C13A8C8818ACDF0775CD2D224F`; size **1,920,732 bytes**.
- All four installed binaries hash-match the sealed ZIP.
- `SearchToolIndexer` is Running + Automatic; existing `C.stidx` was preserved.
- Normal-user `doctor` and `search` both exit 0 with zero explicit `umut` ACL entries on the index.
- Resident GUI is running in interactive Session 1 from the installed binary.
- Startup shortcut is `--resident --no-shell-bridge`; legacy `SearchTool.Search` / Capabilities / RegisteredApplications / `search:` OpenWith ownership is absent; private `searchtool:` remains.
- Native theme state is active and Windows-owned: apps light, system dark, transparency on, configured accent `#0078D7`.
- Evidence: `docs/evidence/windows-physical-native-first-final-11351777183-20261005.json`; Git blob `4eda606f3d935f57ee34f898d04fb9e1c0a3c426`.

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
