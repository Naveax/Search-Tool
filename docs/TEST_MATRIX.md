# Search Tool Test Matrix

Last updated: 2026-09-29.

Legend: PASS = exercised successfully. PARTIAL = path works but final evidence is incomplete. BLOCKED = environment dependency unavailable. TODO = not yet exercised to the desired release standard.

| Area | Status | Evidence / note |
|---|---|---|
| cargo fmt | PASS | Latest Windows release gate |
| cargo clippy -D warnings | PASS | Latest Windows release gate |
| workspace unit tests | PASS | 96 tests total: 70 core + 7 platform + 9 CLI + 4 service + 6 worker |
| Windows release build/link | PASS | Physical Windows x64 |
| CLI smoke | PASS | Release gate |
| Initial NTFS MFT index | PASS | Isolated VHD + real C: |
| USN incremental sync | PASS | Isolated VHD + real C: service; steady-state create/rename/delete probe 4/4 PASS |
| Journal reset/truncation recovery | PASS | Isolated VHD only |
| Deep verify / repair | PASS | Real and isolated indexes |
| Multi-volume search | PASS | D:/E: validation indexes |
| External delta compaction | PASS | 140k+ delta stress exercised |
| Fresh-reader compaction publish snapshot | PASS | Dedicated shared/exclusive publish lock prevents a fresh `LiveSearchStore::open` from observing torn main/sidecar generations during final swap/recovery. Regression deliberately truncates the main file behind the exclusive publish lock and proves the reader waits, then opens the restored family successfully. |
| Mutation lock | PASS | Unit + runtime guard |
| Metadata filters | PASS | VHD + real indexes |
| Plain content index | PASS | VHD integration |
| DOCX extraction | PASS | Worker runtime |
| XLSX extraction | PASS | Worker runtime/fallback |
| PPTX extraction | PASS | Worker runtime/fallback |
| PDF extraction | PASS | Built-in fallback end-to-end |
| Hostile parser-worker matrix | PASS | Commit `e2be944`: hang/crash/partial stdout/invalid UTF-8/oversized response; corrupt + encrypted PDF; corrupt OOXML; 8,193-entry OOXML bomb; 256 MiB Job Object |
| Transactional installer abrupt-exit matrix | PASS | Commit `747702d`: 7 SCM boundaries, exit 197 + separate-process recovery, previous binary/config/index/service restored; live production service unchanged |

| Cross-volume duplicate detection | PASS | Physical validation |
| Quarantine -> restore -> purge | PASS | Physical validation |
| Cleanup protected-path deny | PASS | Unit/runtime |
| Tiny intent router | PASS | Unit/runtime |
| Web resolver sanitizer/cache | PASS | Unit tests |
| Web resolver real success request | BLOCKED | `web-resolver-validation.ps1` now verifies provider success -> credential-free cache hit -> parent-path privacy; current host has no SEARCH_TOOL_GOOGLE_KEY / SEARCH_TOOL_GOOGLE_CX. Evidence: `web-resolver-validation-blocked-20260929.json`. |
| Native Win32 GUI startup | PASS | Physical Windows |
| Single instance / resident mode | PASS | Physical Windows |
| Ctrl+Alt+Space fallback hotkey | PASS | Real key injection hide/show |
| Alt+Space primary hotkey | EXPECTED FALLBACK | Windows reserves/conflicts on host |
| Multi-monitor mixed-DPI | BLOCKED | `display-validation.ps1` now probes monitors/DPI, exercises GUI moves and supports prepare/verify topology recovery; current remote surface exposes one 1024x768 100% DPI display, so physical mixed-DPI evidence is still required |
| Windows SCM service | PASS | Real C: SearchToolIndexer Running + Automatic, service_sync=Ok, last_error=0 |
| Crash/restart soak | PASS | 15-minute soak |
| 15-minute soak | PASS | Two runs; one ~914 s / 14,352 ops / 598 checks |
| 60-minute soak | PASS | Commit `709cc275`: 3613.77 s / 56,496 ops / 2,354 checks / 15.63 ops/s; crash-restart exercised; peak service working set 5.199 MiB; final D: cleanup delta=0, metadata/sizes/content fresh, verify-deep PASS. |
| Real 1M+ search latency matrix | PASS | 1,209,697-record frozen C: index, 5 warmups + 100 rounds/class. p50/p95/p99 ms: exact 27.750/32.206/39.736; prefix 61.983/67.694/68.776; fuzzy 458.443/478.590/497.988; filtered 60.065/66.107/78.889; relationship 130.016/151.995/158.008; content 145.325/155.821/184.094. Evidence `search-latency-matrix-20260929.json`, source `322fb4e`. |
| 6-hour soak | TODO | Direct physical run at `675aabb` ran 6381.94 s / 67,248 ops / 2,800 checks and exposed a real fresh-reader/compaction publication race (`failed to fill whole buffer`). Post-failure service remained Running and doctor + verify-deep PASS. Publish-snapshot fix + 2 regressions now pass; full fixed-build 6-hour rerun remains required. Evidence: `soak-6h-publish-race-20260929.json`. |
| 24-hour soak | TODO | Confidence test |
| Foreground-impact | PASS | Release-freeze run `215e6bc` PASS. Current-main recheck `6bbde9c` also PASS: 85 baseline samples p95 151.442 ms -> 143 stressed samples p95 176.258 ms (+24.816 ms, 1.164x); nested real-service soak PASS with 264 ops / 22 checks / 230.86 s, then doctor + verify-deep PASS and service Running/Automatic. Evidence: `foreground-impact-current-head-20260929.json`. |
| Clean install/uninstall smoke | PASS | Release gate |
| Pristine default-path machine flow | BLOCKED | `pristine-validation.ps1` now requires no existing service/default install/default data/shortcut and covers package verify -> default install -> SCM auto-start -> initial index/search/smart/doctor/GUI smoke -> purge uninstall -> zero residue. Current validation host correctly blocks on existing SearchToolIndexer/ProgramData state. |
| Upgrade preserve/purge | PASS | Physical validation |
| Defender active scan | BLOCKED | 2026-09-29 host reports Antivirus/RealTime/Antispyware/BehaviorMonitor disabled |
| SmartScreen | BLOCKED | `smartscreen-validation.ps1` records policy, MOTW, signature and observed Warned/Blocked outcome; current host has no usable enabled policy/MOTW, so clean-Windows evidence is still required |
| Sleep/resume | PASS | Real C: controlled sleep/resume; pre/post markers visible, boot session unchanged, checkpoint advanced, service Running/Automatic, doctor + verify-deep PASS; `power-cycle-sleep-20260929.json` |
| Reboot recovery | PASS | Real reboot: boot session changed, SearchToolIndexer auto-started Running/Automatic, pre/post markers visible, checkpoint advanced, service_sync=Ok, doctor + verify-deep PASS; 45 s harness false-negative reproduced then fixed with configurable 120 s catch-up window |
| Compaction publish kill-point | PASS | 11 deterministic abrupt-process-exit boundaries exercised; mixed-generation publish bug fixed; verify-deep + retry compaction + debris cleanup PASS |
| Celeron + 4 GB + HDD | BLOCKED | Current host is Ryzen 5 2600X / ~16 GiB / SATA SSD; physical reference hardware still required |

## Latest full Windows release gate

Date: 2026-09-29

Result: **PASS**

Steps:
- release preflight PASS
- cargo fmt PASS
- cargo clippy PASS
- cargo test PASS (94 workspace tests)
- release build PASS
- CLI smoke PASS
- NTFS/USN/service integration PASS
- USN journal reset recovery PASS
- portable package build PASS
- portable package integrity PASS
- clean install/uninstall smoke PASS
- Defender interaction step PASS, but Defender itself reported unavailable/disabled

Commit: `8e6498d`

Final package SHA-256:
`282A2882EB66186E58935ECD3C5C1169B351ABCD3B47FF83A82A6E87F5ACA585`

Final release-gate evidence:
- `docs/evidence/windows-release-gate-final-20260929.json`

Soak hardening evidence:
- `docs/evidence/soak-failure-diagnosis-20260928.json`
- `docs/evidence/soak-ownership-regression-20260928.json`
- `docs/evidence/soak-fast-verify-failure-20260928.json`
- `docs/evidence/soak-fast-verify-regression-20260928.json`
- `docs/evidence/soak-60m-709cc275-20260928.json`
- `docs/evidence/windows-release-gate-soak-hardening-20260928.json`

Real C: recovery/service evidence:
- `docs/evidence/real-c-recovery-service-20260929.json`

Installer transaction evidence:
- `docs/evidence/install-transaction-safe-smoke-20260929.json`

Compaction crash-consistency evidence:
- `docs/evidence/compaction-fault-injection-20260928.json`

Current final package SHA corresponds to commit `8e6498d`; rebuild the package after any source change.
