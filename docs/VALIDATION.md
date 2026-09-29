# Validation

Last updated: 2026-09-29.

## Required local checks

Windows PowerShell:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

MSRV is Rust 1.89. Rust 1.98 has been used successfully for current Windows validation.

## Latest physical Windows result

The latest complete release gate on 2026-09-28 passed on physical Windows x64 at parent commit `385a971`. Current source after power-cycle harness and Windows CI timing hardening passes 93 workspace tests, workspace clippy with `-D warnings`, and a workspace release build; the oversized-response worker regression passed 5 consecutive targeted Windows runs. Rerun the full physical gate before final packaging.

```text
release preflight                  PASS
cargo fmt                          PASS
cargo clippy -D warnings           PASS
cargo test                         PASS (78 total tests)
release build                      PASS
CLI smoke                          PASS
NTFS/USN/service integration       PASS
USN journal reset recovery         PASS
portable package build             PASS
portable package integrity         PASS
clean install/uninstall smoke      PASS
Defender interaction step          PASS*
```

`*` Defender was disabled/unavailable on that host, so this is not evidence of an active antivirus scan.

## Runtime evidence collected

- Real C: index >1.2M records has opened and verify-PASSed.
- D:/E: multi-volume indexes around 100k and 70k records have been exercised.
- External compaction has been stressed with 140k+ delta entries.
- Compaction publish recovery now passes deterministic abrupt-process exits at 11 marker/remove/rename boundaries; verify-deep, retry compaction, logical results and debris cleanup are checked.
- Journal deletion/reset recovery has been exercised on isolated NTFS VHD only.
- Cross-volume duplicate detection and quarantine/restore/purge have been exercised.
- Rich extraction works for DOCX, XLSX, PPTX and PDF.
- Native GUI resident/single-instance and Ctrl+Alt+Space fallback hotkey have been exercised.
- 15-minute service soak passed twice including crash/restart.
- Representative 15-minute run: ~914 seconds, 14,352 operations, 598 validation checks, ~5.2 MiB peak service working set.
- Representative foreground-impact p95: 23.838 ms baseline -> 25.131 ms stressed, PASS.

## Real C: latency matrix

On 2026-09-29 a frozen real C: index with 1,209,697 base records, zero delta and fresh metadata/sizes/content sidecars was measured with 5 warmups and 100 timed rounds per search class at source head `322fb4e`.

| Class | p50 ms | p95 ms | p99 ms |
|---|---:|---:|---:|
| exact | 27.750 | 32.206 | 39.736 |
| prefix | 61.983 | 67.694 | 68.776 |
| fuzzy | 458.443 | 478.590 | 497.988 |
| filtered | 60.065 | 66.107 | 78.889 |
| relationship | 130.016 | 151.995 | 158.008 |
| content | 145.325 | 155.821 | 184.094 |

Evidence: `docs/evidence/search-latency-matrix-20260929.json`.
## Release gate

```powershell
.\scripts\windows-release-gate.ps1 -SoakMinutes 5
```

This runs isolated destructive tests only against temporary VHDs. It does not reset the journal on the real system volume.

## Additional validation commands

```powershell
# Isolated NTFS MFT/USN/service integration
.\.github\scripts\windows-integration.ps1 -SoakMinutes 1

# Isolated journal reset/rebuild
.\scripts\journal-reset-recovery.ps1

# Service mutation soak
.\scripts\windows-soak.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index\C.stidx -DurationMinutes 60 -CrashRestartService

# Foreground impact
.\scripts\foreground-impact.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index -Enforce

# Physical validation aggregate
.\scripts\physical-validation.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index -SoakMinutes 30 -EnforceTargets

# Require low-end reference class (default: CPU name matches Celeron, RAM <= 4096 MiB, confirmed HDD)
.\scripts\physical-validation.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index -SoakMinutes 30 -EnforceTargets -RequireReferenceClass

# Active Defender evidence: active protection + no overlapping exclusion + custom scan + no related detection
.\scripts\defender-check.ps1 -Path .\target\release -CustomScan -Enforce

# SmartScreen clean-machine evidence (record actual UI result with -ObservedOutcome Warned/Blocked)
.\scripts\smartscreen-validation.ps1 -Artifact .\target\release\search-tool-gui.exe -RequireEnabled -RequireMotw -ObservedOutcome Warned -Enforce

# Mixed-DPI GUI movement validation
.\scripts\display-validation.ps1 -Mode Exercise -RequireMixedDpi -Enforce

# Prepare before changing primary monitor / disconnecting a monitor
.\scripts\display-validation.ps1 -Mode PrepareTopology -StateFile .\display-state.json
# After topology change, verify the same GUI process recovered onto an active monitor
.\scripts\display-validation.ps1 -Mode VerifyTopology -StateFile .\display-state.json -ExpectedTopologyChange PrimaryChanged -Enforce
```

## Power-cycle validation

Sleep/resume veya reboot Ã¶ncesi ve sonrasÄ± aynÄ± state dosyasÄ±yla doÄŸrulama yapÄ±labilir:

```powershell
.\scripts\power-cycle-validation.ps1 -Mode Prepare -Drive C: -IndexRoot C:\ProgramData\SearchTool\index
# burada kontrollÃ¼ sleep/resume veya reboot yapÄ±lÄ±r
.\scripts\power-cycle-validation.ps1 -Mode Verify -Drive C: -IndexRoot C:\ProgramData\SearchTool\index
```

Script marker gÃ¶rÃ¼nÃ¼rlÃ¼ÄŸÃ¼, boot time, USN checkpoint hash, SCM service durumu, `doctor` ve `verify-deep` Ã§Ä±ktÄ±sÄ±nÄ± JSON olarak kaydeder.

Controlled sleep/resume passed on 2026-09-29 with marker continuity, checkpoint advancement, Running/Automatic SCM state, `doctor` PASS semantics and `verify-deep` status=ok. Evidence: `docs/evidence/power-cycle-sleep-20260929.json`.

Controlled reboot also passed on 2026-09-29: boot time changed, the SCM service auto-started Running + Automatic, pre/post markers were searchable, the USN checkpoint advanced, and doctor + verify-deep passed. A first verification recorded a harness-only false negative because the old 45 s catch-up window expired during cold-start metadata maintenance; the script now defaults to a configurable 120 s marker window. Evidence: `docs/evidence/power-cycle-reboot-catchup-failure-20260929.json` and `docs/evidence/power-cycle-reboot-20260929.json`.

Current validation-host capability probe confirms the remaining external dependencies are genuinely unavailable here: Defender protection is disabled, Google Custom Search credentials are absent, the remote display surface exposes only one 1024x768 100% DPI monitor, and the host is Ryzen 5 2600X / ~16 GiB / SATA SSD rather than the Celeron + 4 GB + HDD reference target. Evidence: docs/evidence/external-validation-environment-20260929.json.

## Still missing final evidence
- active Defender + SmartScreen clean-machine result;
- multi-monitor mixed-DPI result;
- 6/24-hour long soak;
- Celeron + 4 GB + mechanical HDD benchmark.

For exact current status, use `docs/TEST_MATRIX.md` rather than old chat history.
