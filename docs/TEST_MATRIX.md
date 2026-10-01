# Search Tool Test Matrix

Last updated: 2026-10-01.

Legend: PASS = exercised successfully. PARTIAL = path works but final evidence is incomplete. BLOCKED = environment dependency unavailable. TODO = not yet exercised to the desired release standard.

| Area | Status | Evidence / note |
|---|---|---|
| cargo fmt | PASS | Latest Windows release gate |
| cargo clippy -D warnings | PASS | Latest Windows release gate |
| workspace unit tests | PASS | 117 tests total: 74 core + 7 platform + 9 CLI + 17 GUI + 4 service + 6 worker |
| Windows release build/link | PASS | Physical Windows x64 |
| CLI smoke | PASS | Release gate |
| Release-state consistency | PASS | `docs/RELEASE_STATE.json` + `.github/scripts/release-state-check.ps1` cross-check the current package seal, physical runtime gate, six-hour soak, blocker evidence and synchronized docs; validated package ownership fails closed if the final tree changes outside `.github/` or `docs/` after packaged source `6c4141d0...`. The deterministic self-test also requires rejection of a synthetic `README.md` change, stale package SHA and false blocker PASS. |
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
| Web resolver real success request | BLOCKED | `web-resolver-validation.ps1` verifies provider success -> credential-free cache hit -> parent-path privacy. The physical host has no `SEARCH_TOOL_GOOGLE_KEY` / `SEARCH_TOOL_GOOGLE_CX`, and GitHub-hosted Windows probe `36852274027` also found both repository secrets absent. Evidence: `web-resolver-validation-blocked-20260929.json`, `web-resolver-hosted-secrets-blocked-20261001.json`. |
| Native Win32 GUI startup | PASS | Physical Windows |
| Per-monitor DPI/topology logic | PASS | Runtime source `d01b271`: handles `WM_DPICHANGED`, Win32 suggested RECT, DPI-scaled fonts/layout/rows, nearest-monitor work area and display/work-area recovery. Deterministic tests cover 96/144/192 DPI, negative monitor origins, removed-monitor recovery and oversized clamping; exact-head CI `36840720835` + physical release gate PASS. |
| Single instance / resident mode | PASS | Physical Windows |
| Native live theme menu | PASS | Windows-target unit/release validation: immediate system/dark/light, Acrylic/Mica/none, opacity presets, native color picker persistence and Default Apps deep link; advanced `ui.conf` remains available. |
| Windows Search + Explorer scope integration | PASS | GUI parser preserves documented `search:` query plus Explorer `crumb=location:` scope; private `searchtool:` accepts scope-only requests; scoped path matching rejects similar-prefix leakage; installer registers folder/drive/background shell verbs and pristine validation checks install + uninstall registry cleanup. |
| Ctrl+Alt+Space fallback hotkey | PASS | Real key injection hide/show |
| Alt+Space primary hotkey | EXPECTED FALLBACK | Windows reserves/conflicts on host |
| Multi-monitor mixed-DPI | BLOCKED | Runtime handling is PASS, but final physical evidence remains unavailable. Packaged-source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` hardens the evidence path: mixed-DPI intent is persisted from prepare to verify; monitor-removal recovery requires the prepared window to have intersected an actually removed monitor; the exact prepared GUI PID must survive; and recovered window DPI must match an intersected active monitor. Exact-head CI `36847421304` + full release-gate `36848221272` PASS. Current physical probe still exposes one 1600x900 96-DPI/100% monitor; at least two active monitors with distinct DPI are required. Evidence: `display-mixed-dpi-blocked-d01b271-20261001.json`, `windows-release-gate-pr15-display-validation-20261001.json`. |
| Windows SCM service | PASS | Real C: SearchToolIndexer Running + Automatic, service_sync=Ok, last_error=0 |
| Crash/restart soak | PASS | 15-minute soak |
| 15-minute soak | PASS | Two runs; one ~914 s / 14,352 ops / 598 checks |
| 60-minute soak | PASS | Commit `709cc275`: 3613.77 s / 56,496 ops / 2,354 checks / 15.63 ops/s; crash-restart exercised; peak service working set 5.199 MiB; final D: cleanup delta=0, metadata/sizes/content fresh, verify-deep PASS. |
| Real 1M+ search latency matrix | PASS | 1,209,697-record frozen C: index, 5 warmups + 100 rounds/class. p50/p95/p99 ms: exact 27.750/32.206/39.736; prefix 61.983/67.694/68.776; fuzzy 458.443/478.590/497.988; filtered 60.065/66.107/78.889; relationship 130.016/151.995/158.008; content 145.325/155.821/184.094. Evidence `search-latency-matrix-20260929.json`, source `322fb4e`. |
| 6-hour soak | PASS | Exact frozen source `fa92628`; 21,873.82 s / 223,632 ops / 9,318 checks / 10.22 ops/s, intentional crash/restart exercised, peak service WS 7.461 MiB, source/service identity PASS, post-run doctor + verify-deep exit 0, service Running/Automatic. Evidence: `soak-6h-fa92628-final-20260930.json`. |
| Foreground-impact | PASS | Release-freeze run `215e6bc` PASS. Current-main recheck `6bbde9c` also PASS: 85 baseline samples p95 151.442 ms -> 143 stressed samples p95 176.258 ms (+24.816 ms, 1.164x); nested real-service soak PASS with 264 ops / 22 checks / 230.86 s, then doctor + verify-deep PASS and service Running/Automatic. Evidence: `foreground-impact-current-head-20260929.json`. |
| Clean install/uninstall smoke | PASS | Release gate |
| Pristine default-path machine flow | PASS | GitHub-hosted Windows CI `36825801758`: clean default Program Files/ProgramData install; SearchToolIndexer Running/Automatic; initial index/search/smart/doctor; GUI + scoped GUI; `search:` / `searchtool:` / Capabilities / RegisteredApplications / OpenWithProgids / App Paths / Directory/Background/Drive verbs; purge uninstall; 14/14 post-uninstall residue checks true. Evidence: `pristine-default-path-hosted-20261001.json`. |
| Upgrade preserve/purge | PASS | Physical validation |
| Defender active scan | BLOCKED | Physical host lacks active protection. GitHub-hosted Windows Server 2025 enforced probe (`-CustomScan -Enforce`) also returned `UNAVAILABLE`: AMService/Antivirus/Antispyware=true, RealTimeProtection/BehaviorMonitor=false. Evidence: `defender-hosted-blocked-20261001.json`, CI run `36835643698`. |
| SmartScreen | BLOCKED | Hosted Windows probe attached real Internet-zone MOTW (`ZoneId=3`) and confirmed the ZIP is unsigned/untrusted, but `effective_enabled=null`, no machine/user/policy SmartScreen setting was exposed, and no interactive Warned/Blocked outcome was observed. Evidence: `smartscreen-hosted-blocked-20261001.json`, CI `36836915656`. A genuinely protected interactive Windows host remains required. |
| Sleep/resume | PASS | Real C: controlled sleep/resume; pre/post markers visible, boot session unchanged, checkpoint advanced, service Running/Automatic, doctor + verify-deep PASS; `power-cycle-sleep-20260929.json` |
| Reboot recovery | PASS | Real reboot: boot session changed, SearchToolIndexer auto-started Running/Automatic, pre/post markers visible, checkpoint advanced, service_sync=Ok, doctor + verify-deep PASS; 45 s harness false-negative reproduced then fixed with configurable 120 s catch-up window |
| Compaction publish kill-point | PASS | 11 deterministic abrupt-process-exit boundaries exercised; mixed-generation publish bug fixed; verify-deep + retry compaction + debris cleanup PASS |

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
- `docs/evidence/smartscreen-hosted-blocked-20261001.json`

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
