# Development Status

Last updated: 2026-10-01.

## Current state

Search Tool's core implementation is feature-complete for the current milestone. Remaining work is primarily release hardening, long-running validation and low-end hardware evidence rather than missing core functionality.

## Implemented

- NTFS MFT streaming initial index.
- USN Journal incremental sync, checkpointing, reset/truncation detection and full reconciliation.
- Append-only delta overlay with bounded-memory external compaction.
- Crash-safe rebuild/compaction publishing with main-last commit semantics, per-index OS mutation locking and 11-boundary abrupt-exit regression coverage.
- Disk-first main/name/id indexes with persistent sparse checkpoints.
- Fixed-NTFS multi-volume discovery and global search merge.
- Exact, prefix, ranked, fuzzy, relationship and filtered search.
- Path reconstruction and volume-scoped IDs.
- Size/date/attribute metadata sidecars with generation freshness.
- Plain content index with sparse checkpoints.
- Isolated rich-document worker with IFilter plus built-in PDF/OOXML fallback parsing.

- Resource Governor / low-end policies for CPU, RAM, user activity, battery and storage type.
- Windows background processing mode for service and expensive workers.
- Duplicate pipeline: size -> sample -> full fingerprint -> byte-for-byte final verification, including cross-volume groups.
- Knowledge/classification and cleanup safety policy.
- Quarantine, restore and explicit permanent purge.
- Tiny ~1 MiB INT8 natural-language intent router.
- Privacy-sanitized optional cached web resolver.
- Native Win32 GUI, single-instance resident launcher and fallback global hotkey.
- Per-monitor DPI/topology hardening: `WM_DPICHANGED`, DPI-scaled fonts/layout/owner-draw rows, nearest-monitor work-area placement, and display/work-area recovery.
- Native Windows SCM service for USN sync and idle maintenance.
- HDD/SSD seek-penalty detection.
- Verify, verify-deep, repair, maintain and doctor diagnostics.
- Multi-volume installer/uninstaller/package scripts.
- Isolated NTFS VHD integration suite and journal-reset recovery tests.
- Windows soak, foreground-impact, low-end, Defender and physical-validation scripts.
- Portable ZIP integrity/SHA verification.
- Single-command Windows release gate with JSON summary.

## Latest verified gate

2026-10-01 runtime/package source `d01b2717127adde68d0a21767aa494d6826ee537`: **PASS** on the physical Windows x64 release gate after per-monitor DPI/topology hardening.
The gate passed release preflight, fmt, clippy `-D warnings`, all 117 workspace tests, release build, CLI smoke, NTFS/USN/service integration, USN journal reset recovery, portable package build/integrity and clean install/uninstall smoke. Exact-head GitHub CI run `36840720835` also passed on Windows + Ubuntu. The production `SearchToolIndexer` remained Running + Automatic with PID 2664 before and after the isolated gate.

Prior physical-runtime-gate package SHA-256: `568197814A9390F9486817E8828F16FD5CC43E5322F5000DFF7F0C9B27CE5C22` (1,837,866 bytes).
Evidence: `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json`.

The current packaged candidate is source `6c4141d0bcf12ade21cf633fbaf42d361eb12977`, which changes only the packaged `scripts/display-validation.ps1` evidence harness relative to the physically gated runtime generation. Exact-head CI `36847421304` passed on Windows + Ubuntu. Full hosted Windows release-gate run `36848221272` passed, including fmt, clippy, 117 tests, release build, CLI smoke, NTFS/USN/service integration, journal-reset recovery, package build/integrity and clean install/uninstall. The validation wrapper `79f061e09b8d0677ec67532ac0142a6e3d8449cc` differs from the packaged source only by its temporary workflow file. Current ZIP SHA-256: `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65` (1,888,674 bytes), independently re-hashed against the artifact seal. Evidence: `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`.

`docs/RELEASE_STATE.json` is now the machine-readable release ownership record. The Windows CI consistency gate cross-validates it against the release evidence, the sealed six-hour soak, the physical runtime gate, every external-blocker evidence path and the synchronized continuation/validation documents, and rejects any final-tree change outside `.github/` or `docs/` while this package remains marked `VALIDATED`. It also requires the exact blocker-name set, synchronized-document set, `.github/` + `docs/` post-package allowlist and required transient validation path, and pins every blocker name to its exact evidence file. The deterministic self-test covers packaged-input drift, stale package/test-count claims, false blocker PASS, missing structural entries, unsafe allow-prefix expansion, swapped blocker-evidence identities, tampered blocker-evidence blobs, tampered core release evidence blobs and missing blocker-evidence markers.

Current external-blocker evidence Git blob seals: `defender=332869529cf3b770c2d97f70ffbbfd416c6bd63f`, `smartscreen=74b3e16bc372070cca2ce3a83e4e617d9681b627`, `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`.

Core release evidence seals: package `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` blob `9bf0fea273b90ac2ba3f164a2ed550cd8cf57294`; physical `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.

The physical mixed-DPI proof remains blocked by the available display surface: one 1600x900 monitor at 96 DPI / 100%. Evidence: `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`.

Active Defender protection is unavailable on the physical validation host. A separate GitHub-hosted Windows Server 2025 enforced probe also returned `UNAVAILABLE`: AM/Antivirus/Antispyware were enabled, but `RealTimeProtectionEnabled=false` and `BehaviorMonitorEnabled=false`. A hosted SmartScreen probe successfully attached Internet-zone MOTW (`ZoneId=3`) to the unsigned ZIP, but no enabled SmartScreen configuration was exposed and no interactive Warned/Blocked outcome could be observed, so it correctly remained BLOCKED. Evidence: `docs/evidence/defender-hosted-blocked-20261001.json` and `docs/evidence/smartscreen-hosted-blocked-20261001.json`.
See `docs/TEST_MATRIX.md` for detailed evidence.

## Merged Shell integration checkpoint

PR #7 merged to `main` as `4700a6c` after exact-head CI `36786906440` passed. The registry rollback fix at code head `5ec0a74` was independently validated by CI run `36786244058`, including the disposable HKCU snapshot self-test and 10-boundary installer fault matrix. The final six-hour `fa92628` source-freeze soak is sealed PASS, and the merged source subsequently passed both GitHub main CI and the physical Windows release gate.

## Remaining work

Release blockers:
- none currently open in the power-cycle/core validation path; sleep/resume and real reboot continuity are both PASS.

Hardening:
- real multi-monitor mixed-DPI GUI validation; implementation hardening and deterministic topology/DPI tests are PASS, but the physical two-monitor evidence is still blocked;
- valid Web Resolver success/cache request when credentials are available; GitHub-hosted probe run `36852274027` also found both `SEARCH_TOOL_GOOGLE_KEY` and `SEARCH_TOOL_GOOGLE_CX` absent. Evidence: `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`.
- current-main physical external preflight on `ed65fbc` reconfirmed the blockers with 0 validator failures: Defender policy-disabled (`WinDefend`/`WdNisSvc` Disabled; `DisableAntiSpyware=1`, `DisableAntiVirus=1`), one 1600x900 @ 96-DPI monitor, no Google key/CX, and no exposed enabled SmartScreen configuration. Supplemental evidence: `docs/evidence/external-validation-physical-preflight-ed65fbc-20261001.json`.
- physical SmartScreen readiness on the exact sealed ZIP confirms the artifact itself is ready: 1,888,674 bytes, sealed SHA-256 unchanged before/after staged `ZoneId=3` ADS, MOTW Internet-zone requirement true and unsigned/untrusted requirement true. The remaining SmartScreen blocker is host configuration (`effective_enabled=null`) plus missing interactive Warned/Blocked observation. Evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`.

Final evidence:
- 1M+ real-index p50/p95/p99 search matrix: COMPLETE on 1,209,697-record C: index; see `docs/evidence/search-latency-matrix-20260929.json`;
- Windows Search-style product UI: IMPLEMENTED with resident modes, query+scope IPC, supported `search:`/`searchtool:` protocol paths, Explorer `crumb=location:` scoped search, folder/drive/background Explorer shell verbs, native live Tema controls, and per-monitor DPI/topology recovery; physical mixed-DPI UX validation remains;
- 6-hour soak: COMPLETE / PASS on exact frozen source `fa92628`; 21,873.82 s / 223,632 ops / 9,318 checks, intentional crash/restart exercised, exact source/service SHA identity PASS, post-run doctor + verify-deep PASS, service Running + Automatic. Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`;
- Defender/SmartScreen on a clean Defender-enabled Windows install;
- pristine default-path install/uninstall: COMPLETE / PASS on GitHub-hosted Windows; production `SearchToolIndexer` Auto/Running, initial index/search/smart/doctor, GUI + scoped GUI, all supported Shell/protocol registrations present during install, then 14/14 zero-residue checks after purge uninstall. Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`, CI `36825801758`;

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.
