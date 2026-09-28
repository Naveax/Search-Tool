# Search Tool Test Matrix

Last updated: 2026-09-28.

Legend: PASS = exercised successfully. PARTIAL = path works but final evidence is incomplete. BLOCKED = environment dependency unavailable. TODO = not yet exercised to the desired release standard.

| Area | Status | Evidence / note |
|---|---|---|
| cargo fmt | PASS | Latest Windows release gate |
| cargo clippy -D warnings | PASS | Latest Windows release gate |
| workspace unit tests | PASS | 77 tests total: 65 core + 7 platform + 3 CLI + 2 worker |
| Windows release build/link | PASS | Physical Windows x64 |
| CLI smoke | PASS | Release gate |
| Initial NTFS MFT index | PASS | Isolated VHD + real C: |
| USN incremental sync | PASS | Isolated VHD + service |
| Journal reset/truncation recovery | PASS | Isolated VHD only |
| Deep verify / repair | PASS | Real and isolated indexes |
| Multi-volume search | PASS | D:/E: validation indexes |
| External delta compaction | PASS | 140k+ delta stress exercised |
| Mutation lock | PASS | Unit + runtime guard |
| Metadata filters | PASS | VHD + real indexes |
| Plain content index | PASS | VHD integration |
| DOCX extraction | PASS | Worker runtime |
| XLSX extraction | PASS | Worker runtime/fallback |
| PPTX extraction | PASS | Worker runtime/fallback |
| PDF extraction | PASS | Built-in fallback end-to-end |

| Cross-volume duplicate detection | PASS | Physical validation |
| Quarantine -> restore -> purge | PASS | Physical validation |
| Cleanup protected-path deny | PASS | Unit/runtime |
| Tiny intent router | PASS | Unit/runtime |
| Web resolver sanitizer/cache | PASS | Unit tests |
| Web resolver real success request | BLOCKED | Requires valid Google key + CX |
| Native Win32 GUI startup | PASS | Physical Windows |
| Single instance / resident mode | PASS | Physical Windows |
| Ctrl+Alt+Space fallback hotkey | PASS | Real key injection hide/show |
| Alt+Space primary hotkey | EXPECTED FALLBACK | Windows reserves/conflicts on host |
| Multi-monitor mixed-DPI | TODO | Needs physical multi-monitor pass |
| Windows SCM service | PASS | Integration and physical machine |
| Crash/restart soak | PASS | 15-minute soak |
| 15-minute soak | PASS | Two runs; one ~914 s / 14,352 ops / 598 checks |
| 60-minute soak | PARTIAL | Generated workload but process ended without final JSON |
| 6-hour soak | TODO | Confidence test |
| 24-hour soak | TODO | Confidence test |
| Foreground-impact | PASS | p95 23.838 ms -> 25.131 ms representative run |
| Clean install/uninstall smoke | PASS | Release gate |
| Upgrade preserve/purge | PASS | Physical validation |
| Interrupted-upgrade rollback | TODO | Fault injection |
| Defender active scan | BLOCKED | Defender disabled on current host |
| SmartScreen | TODO | Clean Windows / unsigned binary behavior |
| Sleep/resume | TODO | Harness prepared |
| Reboot recovery | TODO | Harness prepared |
| Compaction publish kill-point | TODO | Final crash-consistency fault injection |
| Celeron + 4 GB + HDD | TODO | Required to validate low-end UX target |

## Latest full Windows release gate

Date: 2026-09-28

Result: **PASS**

Steps:
- release preflight PASS
- cargo fmt PASS
- cargo clippy PASS
- cargo test PASS
- release build PASS
- CLI smoke PASS
- NTFS/USN/service integration PASS
- USN journal reset recovery PASS
- portable package build PASS
- portable package integrity PASS
- clean install/uninstall smoke PASS
- Defender interaction step PASS, but Defender itself reported unavailable/disabled

Package SHA-256 produced by that source tree:
`1A3D0E629D445598C4250A843F9D9F7EE1BA1EDD2A2167BAFBBF4C36FE96EA6B`

This SHA is evidence only. Rebuild package after any source change.
