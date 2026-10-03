# Search Tool - Project Handoff

> Authoritative continuation note. Last updated: 2026-10-01.

This document exists so development can continue from the repository without needing the original ChatGPT conversation.

## Project goal

Search Tool is an ultra-light native Windows file search and safe maintenance utility. The final product goal is a familiar Windows Search-style experience backed by a much faster disk-first MFT/USN engine, without Electron/Chromium/JVM/Node runtime overhead.

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
- `apps/search-tool-gui`: Windows Search-style native Win32 resident UI, theme/backdrop support, query IPC and Shell protocol entrypoints.
- `apps/search-tool-service`: Windows SCM service, USN sync and idle maintenance.
- `apps/search-tool-worker`: isolated rich-document parser with IFilter/fallback parsers.
- `apps/search-tool-bench`: fresh-process scale/RSS/query benchmark.
- `scripts`: packaging, install/uninstall, soak, journal recovery and release gates.
- `.github`: Windows CI, soak and release-gate workflows.
- `models/tiny-intent-v1.stm`: ~1 MiB INT8 intent model.

## What is implemented

MFT initial index, USN incremental sync, checkpoint recovery, bounded delta overlay, external bounded-memory compaction, multi-volume search, exact/prefix/ranked/fuzzy/relationship search, filters, path reconstruction, metadata sidecars, content index, rich document worker, duplicate verification, quarantine/restore/purge, tiny-AI natural-language routing, privacy-sanitized optional web resolver, native GUI, Windows service, resource governor, install/package scripts, repair/maintain/doctor and Windows validation harnesses are implemented.

## Last verified release gate

On 2026-10-01 runtime/package source `d01b2717127adde68d0a21767aa494d6826ee537` passed the physical Windows x64 release gate after per-monitor DPI/topology hardening:

- release preflight, fmt, clippy `-D warnings`, 117 workspace tests and release build: PASS
- CLI smoke, NTFS/MFT/USN/service VHD integration and USN journal reset recovery: PASS
- portable package build + integrity and clean install/uninstall smoke: PASS
- exact-head GitHub CI run `36840720835`: PASS on Windows + Ubuntu
- the production `SearchToolIndexer` remained Running + Automatic with PID 2664 before and after the isolated release gate
- Defender interaction completed, but active protection is disabled/unavailable on this host, so active-AV evidence remains external
- the current physical display probe is BLOCKED for mixed DPI because only one 1600x900 96-DPI monitor is visible

Evidence: `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` and `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`.
Prior physical-gate package SHA-256: `568197814A9390F9486817E8828F16FD5CC43E5322F5000DFF7F0C9B27CE5C22` (1,837,866 bytes).

### Current packaged candidate after display-validation evidence hardening

The packaged-source head `6c4141d0bcf12ade21cf633fbaf42d361eb12977` hardens only `scripts/display-validation.ps1`; the runtime binaries remain the previously gated `d01b271` runtime generation. Exact-head CI `36847421304` passed on Windows + Ubuntu. A full hosted Windows release gate derived exactly from that source passed as run `36848221272`; its validation wrapper `79f061e09b8d0677ec67532ac0142a6e3d8449cc` differs from the packaged source only by the temporary release-gate workflow.

Current candidate ZIP SHA-256: `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65` (1,888,674 bytes). The uploaded artifact was independently re-hashed against its seal and matched exactly. Evidence: `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`.

Machine-readable release ownership is sealed in `docs/RELEASE_STATE.json`. CI runs `.github/scripts/release-state-check.ps1` to cross-check the package seal, physical runtime gate, six-hour soak, unresolved external-blocker evidence, completed external-gate evidence and the five synchronized continuation/validation documents. Those documents must carry the sealed six-hour-soak evidence path, current workspace test count (117), every unresolved blocker evidence path/blob and every completed-gate evidence path/blob. The checker now requires the exact two unresolved blocker names (`mixed_dpi`, `web_resolver`), the exact completed external gates (`smartscreen`, `defender`), exact five synchronized documents, exact post-package allowlist (`.github/`, `docs/`) and the required transient validation path. `.github/scripts/release-state-selftest.ps1` proves fail-closed behavior for packaged-input drift, stale package/test-count claims, false blocker PASS, missing blocker/completed-gate/document entries, unsafe allow-prefix expansion, swapped blocker evidence identities, tampered blocker/completed-gate/core evidence blobs and a synchronized document missing a required evidence marker. While package status is `VALIDATED`, the final tree may differ from packaged source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` only under `.github/` and `docs/`; any packaged-input change fails CI until the release state is explicitly invalidated or replaced by a new release gate/package seal.

The final real mixed-DPI/topology exercise remains **BLOCKED BY ENVIRONMENT**; the harness hardening prevents same-DPI, wrong-process, or unrelated-monitor recovery from being accepted as final evidence.

Current unresolved external-blocker evidence set: `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json` and `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`. Completed external-gate evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` and `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`.

Current unresolved-blocker evidence Git blob seals: `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`.

Core release evidence seals: package `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` blob `9bf0fea273b90ac2ba3f164a2ed550cd8cf57294`; physical `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.

The ZIP itself is intentionally not tracked in Git; recreate it with `scripts/package.ps1`.

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

## Current unfinished lab state

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
- Current source passes 117 workspace tests (74 core + 7 platform + 9 CLI + 17 GUI + 4 service + 6 worker), fmt, clippy `-D warnings` and release build. Publish-snapshot protection is owned by `SearchStore::open()` itself, so search, doctor/verify, metadata/content builders, benchmark paths and every other base-family reader open one coherent generation. `LiveSearchStore` keeps only delta-overlay responsibilities. Both direct-SearchStore and LiveSearch torn-family regressions PASS. The GUI is a Windows Search-style resident panel with file/folder/content modes, owner-drawn results, Mica/Acrylic, native live theme controls, single-instance query IPC, supported `search:` / `searchtool:` protocol paths, and per-monitor DPI/topology recovery. The advanced `%APPDATA%\SearchTool\ui.conf` remains available for palette/size overrides. The exact `fa92628` source-freeze soak is PASS: 21,873.82 s / 223,632 operations / 9,318 checks with intentional crash/restart, exact source/service identity, post-run doctor + verify-deep PASS, and SearchToolIndexer Running + Automatic. Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`.
- The first independent Task Scheduler short-soak on cfe3fef completed 2,496 filesystem operations / 104 checks with intentional service crash/restart and never reproduced the raw I/O race. Its only failure was final generic soak-g cleanup: an older SentinelX-killed invalid run had left C:\.search-tool-soak-3a53ba3d844d41d58e3b344e3cd188a8 physically on disk with the same repeated filenames. NTFS journal evidence showed separate old/new file IDs. Search Tool was correct to return that real directory. The harness now embeds a unique run-id in every workload filename and reports the run-id/test-root, preventing cross-run false positives.
- The isolated follow-up at source `6a22593` then passed end-to-end with the run-scoped harness: 273.19 s / 2,688 operations / 56 validation checks, BatchSize=64, intentional service crash/restart exercised, no recurrence of `failed to fill whole buffer`, and sealed post-check `doctor` + `verify-deep` PASS with SearchToolIndexer Running + Automatic. Evidence: `docs/evidence/short-soak-isolated-20260930.json`.
- External validation harnesses are explicit. `pristine-validation.ps1` has a clean hosted-Windows PASS: default Program Files/ProgramData install, production SCM Auto/Running, initial index/search/smart/doctor, GUI + scoped GUI, `search:` / `searchtool:` / App Paths / RegisteredApplications / OpenWith / Explorer verbs, then purge uninstall with 14/14 zero-residue checks. Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`. Earlier hosted Defender readiness remained BLOCKED/UNAVAILABLE because real-time/behavior protection was disabled; a later isolated hosted activation/custom-scan run `36972721866` produced final Defender PASS evidence. Historical hosted SmartScreen readiness proved MOTW `ZoneId=3` but could not expose an enabled policy or interactive protective outcome; final SmartScreen validation later passed physically. Per-monitor DPI/topology logic is hardened and release-gate validated at `d01b271`, while the physical display probe remains BLOCKED at one 1600x900 96-DPI monitor. `display-validation.ps1` and credential-backed `web-resolver-validation.ps1` still require their final external environments; Defender and SmartScreen final validation are complete. `.github/scripts/external-validation-orchestrator.ps1` now aggregates those four gates into one JSON summary with explicit PASS/READY/BLOCKED/PARTIAL/FAIL semantics; it never upgrades a blocked prerequisite into PASS, and CI smoke-tests both the all-skip aggregation path and credential-first Web Resolver preflight. Missing Google key/CX is reported before CLI existence is required, so source-only physical checkouts no longer need a placeholder file to identify the real blocker. Defender readiness behaves the same way when `-DefenderCustomScan:$false`: it probes active protection against the repository root and does not require `target\release`; final custom-scan evidence still requires the release binary directory. A GitHub-hosted Windows probe (run `36852274027`) also confirmed that repository secrets `SEARCH_TOOL_GOOGLE_KEY` and `SEARCH_TOOL_GOOGLE_CX` are both absent; evidence: `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`. A current-main physical preflight on `ed65fbc` then exercised the new orchestrator on `DESKTOP-ONDD84S`: aggregate `BLOCKED`, 0 FAIL, Defender `UNAVAILABLE`, display `BLOCKED`, Web Resolver `BLOCKED`, SmartScreen deliberately skipped because the candidate ZIP was not present. Follow-up diagnosis showed `WinDefend` and `WdNisSvc` Stopped/Disabled with Defender policy `DisableAntiSpyware=1` and `DisableAntiVirus=1`; one 1600x900 @ 96-DPI display was active; both Google credential-presence checks were false; no enabled SmartScreen machine/user/policy setting was exposed. Supplemental evidence: `docs/evidence/external-validation-physical-preflight-ed65fbc-20261001.json`. The exact sealed ZIP was then downloaded from release-gate run `36848221272`, rehashed to `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`, copied, and given a staged Internet-zone `Zone.Identifier` ADS. The ZIP byte hash and size remained unchanged, `motw_internet_zone=true`, and the artifact remained unsigned/untrusted; SmartScreen still returned BLOCKED solely because `effective_enabled=null` and no interactive Warned/Blocked outcome was observed. Supplemental readiness evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`. On the later current `main` `d376b244165c067be951ae45e0f9d5ea51129e3a`, the orchestrator was rerun with all four external gates enabled and the exact sealed/MOTW ZIP: aggregate `BLOCKED`, 0 FAIL, 0 skipped; Defender `UNAVAILABLE`, SmartScreen `BLOCKED`, display `BLOCKED`, Web Resolver `BLOCKED`. That run is historical pre-PASS SmartScreen context. A subsequent physical test on `f32212604c86079fdfd45ea60bffa353117568b6` temporarily enabled SmartScreen `Warn`, launched MOTW-marked `search-tool-gui.exe`, captured SmartScreen Event 1000 with `Enforcement=warnByPolicy` / `Experience=Untrusted`, and produced validator `ObservedOutcome=Warned` / PASS; rollback restored the prior host configuration. Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`. On then-current `main` `eae87f121f7b6333e0919696f0678649b0228e56`, release-state verification returned PASS and the orchestrator was rerun with completed SmartScreen intentionally skipped: aggregate `BLOCKED`, 3 unresolved gates, 0 FAIL, 1 skipped; Defender was still `UNAVAILABLE` at that historical checkpoint, display was `BLOCKED` at one 1600x900 @ 96-DPI monitor, and Web Resolver was `BLOCKED` on absent key/CX before CLI existence mattered. Defender was subsequently completed by hosted run `36972721866`. Evidence: `docs/evidence/external-validation-three-unresolved-eae87f1-20261002.json`.

## Immediate continuation order

1. **Per-monitor DPI/topology runtime + evidence-harness hardening — VALIDATED.** Runtime source `d01b2717127adde68d0a21767aa494d6826ee537` retains the physical Windows gate. Packaged-source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` additionally hardens `display-validation.ps1`, passed exact-head CI `36847421304`, and passed full hosted release-gate run `36848221272`. Current candidate SHA-256: `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`. Evidence: `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`.
2. **The required long runtime evidence remains sealed.** The exact `fa92628` service/runtime build completed the 21,873.82-second / 223,632-operation / 9,318-check six-hour soak with intentional crash/restart and post-run doctor + verify-deep PASS. The DPI/topology hardening changes only the GUI packaged runtime, so the service/core soak does not need to be repeated.
3. **Remaining validation is environment-dependent, not a known product failure.** Hosted pristine install/uninstall is PASS. SmartScreen final physical validation is PASS with a real `warnByPolicy` event and validator `ObservedOutcome=Warned`; Defender final hosted active-protection/custom-scan validation is also PASS against the exact sealed candidate. Physical mixed-DPI/topology still needs two monitors with distinct DPI; Web Resolver still needs valid Google Custom Search key + CX.
4. **Package/evidence ownership is explicit.** The current package bytes are tied to packaged-source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` and SHA-256 `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`; later docs/evidence-only commits do not change those package bytes. The physical runtime gate remains tied to `d01b271`. Rerun the full release gate after any packaged source/input change, and rerun the six-hour soak only if runtime-core/service inputs change.

For the full backlog see `docs/ROADMAP.md`. For evidence and exact PASS/blocked states see `docs/TEST_MATRIX.md`.

## Safe resume rule

Before mutating a real index, always run:
`search-tool doctor <index-dir>`
and inspect whether an SCM SearchToolIndexer instance is running. `maintain` intentionally refuses unsafe concurrent mutation. Use isolated VHDs for destructive USN/journal recovery tests.

### 2026-10-02 credential-first Web Resolver physical recheck

On physical host DESKTOP-ONDD84S, checkout 218ac7eadb72412be0e68bccf11368d48b724faf passed release-state-check.ps1. A Web-Resolver-only orchestrator run then intentionally used the default release CLI path while that binary did not exist. The gate still returned BLOCKED for missing SEARCH_TOOL_GOOGLE_KEY / SEARCH_TOOL_GOOGLE_CX before CLI availability mattered, with failed=0, blocked=1, skipped=3, and no secret values recorded. Supplemental evidence: docs/evidence/web-resolver-credential-first-218ac7e-20261002.json. Canonical blocker evidence/blob seals and the validated package seal are unchanged.

## 2026-10-02 authoritative external-gate state after hosted Defender PASS

Defender is COMPLETE / PASS from hosted run 36972721866 against the exact sealed 1,888,674-byte candidate (0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65). Realtime and behavior protection were active at scan time, the custom scan returned PASS with zero new related detections, and restoration attempts succeeded. Evidence: docs/evidence/defender-hosted-active-pass-36972721866-20261002.json; Git blob 3349e503636f5c9c0a2613892b62c5bac15b0e02.

Completed external gates are now smartscreen and defender. SmartScreen remains sealed by docs/evidence/smartscreen-physical-pass-f322126-20261002.json, blob 355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952.

The only unresolved external blockers are mixed_dpi and web_resolver: docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json / blob a1c0c329a1024ab02948361b9f8102e069f0db95, and docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json / blob ed3d9b56fc75e7d56620e639917988882c732550. Historical Defender BLOCKED evidence remains provenance only and is no longer the authoritative release blocker. The validated package source/hash and core evidence seals remain unchanged.


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
