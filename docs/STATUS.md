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

`docs/RELEASE_STATE.json` is now the machine-readable release ownership record. The Windows CI consistency gate cross-validates it against the release evidence, the sealed six-hour soak, the physical runtime gate, every unresolved external-blocker evidence path, completed external-gate evidence and the synchronized continuation/validation documents, and rejects any final-tree change outside `.github/` or `docs/` while this package remains marked `VALIDATED`. It requires the exact unresolved blocker set (`mixed_dpi`, `web_resolver`), exact completed external gates (`smartscreen`, `defender`), synchronized-document set, `.github/` + `docs/` post-package allowlist and required transient validation path. The deterministic self-test covers packaged-input drift, stale package/test-count claims, false blocker PASS, invalid completed-gate result, missing structural entries, unsafe allow-prefix expansion, swapped blocker evidence, tampered blocker/completed-gate/core evidence blobs and missing evidence markers.

CI now detects proven documentation-only diffs. Those runs still execute the Windows PowerShell syntax gate, release-state consistency checker, fail-closed self-test and Windows PowerShell 5.1 compatibility check, while skipping Rust build/test and Windows integration/package/pristine stages. Any empty, unknown, `.github/`, or runtime-affecting diff falls back to the full CI path.

Current unresolved-blocker evidence Git blob seals: `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`. Completed external-gate PASS evidence: SmartScreen `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` / blob `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`; Defender `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json` / blob `3349e503636f5c9c0a2613892b62c5bac15b0e02`.

Core release evidence seals: package `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` blob `9bf0fea273b90ac2ba3f164a2ed550cd8cf57294`; physical `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.

The physical mixed-DPI proof remains blocked by the available display surface: one 1600x900 monitor at 96 DPI / 100%. Evidence: `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`.

The physical validation host still cannot provide Defender evidence because protection is disabled there, and earlier hosted readiness probes were also unavailable. Final Defender validation is nevertheless COMPLETE/PASS from hosted run `36972721866`, where real-time and behavior protection were active during a custom scan of the exact sealed candidate with zero new related detections. The hosted SmartScreen probe remains historical BLOCKED provenance only; final physical SmartScreen validation is PASS. Evidence: completed `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`, historical `docs/evidence/defender-hosted-blocked-20261001.json`, historical `docs/evidence/smartscreen-hosted-blocked-20261001.json`, and completed `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`.
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
- historical physical SmartScreen readiness on the exact sealed ZIP confirmed MOTW + unsigned/untrusted prerequisites but remained BLOCKED at that stage; this was superseded by final physical SmartScreen PASS. Evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`, completed evidence `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`.
- historical pre-PASS all-gates physical preflight on `d376b244165c067be951ae45e0f9d5ea51129e3a` returned `BLOCKED` with Defender/SmartScreen/display/Web Resolver unresolved at that time. SmartScreen is now complete. Current `main` `eae87f121f7b6333e0919696f0678649b0228e56` was rechecked with SmartScreen intentionally skipped as a completed gate: aggregate `BLOCKED`, 3 unresolved, 0 FAIL, 1 skipped; Defender `UNAVAILABLE`, one-monitor mixed-DPI `BLOCKED`, Web Resolver credential `BLOCKED`. Evidence: historical `docs/evidence/external-validation-current-main-d376b244-20261002.json`; current `docs/evidence/external-validation-three-unresolved-eae87f1-20261002.json`.

Final evidence:
- 1M+ real-index p50/p95/p99 search matrix: COMPLETE on 1,209,697-record C: index; see `docs/evidence/search-latency-matrix-20260929.json`;
- Windows Search-style product UI: IMPLEMENTED with resident modes, query+scope IPC, supported `search:`/`searchtool:` protocol paths, Explorer `crumb=location:` scoped search, folder/drive/background Explorer shell verbs, native live Tema controls, and per-monitor DPI/topology recovery; physical mixed-DPI UX validation remains;
- 6-hour soak: COMPLETE / PASS on exact frozen source `fa92628`; 21,873.82 s / 223,632 ops / 9,318 checks, intentional crash/restart exercised, exact source/service SHA identity PASS, post-run doctor + verify-deep PASS, service Running + Automatic. Evidence: `docs/evidence/soak-6h-fa92628-final-20260930.json`;
- Defender active-protection/custom-scan evidence: COMPLETE/PASS on hosted run `36972721866`; SmartScreen final physical validation is also PASS;
- pristine default-path install/uninstall: COMPLETE / PASS on GitHub-hosted Windows; production `SearchToolIndexer` Auto/Running, initial index/search/smart/doctor, GUI + scoped GUI, all supported Shell/protocol registrations present during install, then 14/14 zero-residue checks after purge uninstall. Evidence: `docs/evidence/pristine-default-path-hosted-20261001.json`, CI `36825801758`;

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.

### 2026-10-02 Web Resolver credential-first physical verification

Physical main checkout 218ac7eadb72412be0e68bccf11368d48b724faf reverified the PR #28 behavior with no release CLI binary present: the orchestrator returned Web Resolver BLOCKED for missing Google key/CX, not for CLI availability; aggregate counts were 0 PASS / 0 READY / 1 BLOCKED / 0 FAIL / 3 SKIPPED. Evidence: docs/evidence/web-resolver-credential-first-218ac7e-20261002.json. This is supplemental only and does not change the canonical blocker seals.

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


## Mixed-DPI finalization automation

The only remaining external gate now has a dedicated finalizer: .github/scripts/mixed-dpi-finalizer.ps1. It verifies the sealed package before any live GUI stage and orchestrates Exercise -> primary-display change -> physical monitor removal -> final evidence bundle. Monitor-removal preparation deliberately moves the sealed GUI onto the exact non-primary display that must be physically disconnected. A deterministic SelfTest proves the final bundle passes valid synthetic evidence and rejects invalid removal provenance. The current one-monitor physical host remains correctly BLOCKED until a second active display with distinct effective DPI is available.

## Final promotion automation ready

The post-physical-test release promotion is now automated by `.github/scripts/mixed-dpi-promote.ps1`. It validates the final bundle and subordinate evidence, stages canonical PASS evidence, converts the release state from one blocker to zero blockers, updates checker/self-test policy, and synchronizes all release documents. CI self-tests the complete synthetic promotion in a disposable Git worktree. The only missing input remains real two-monitor mixed-DPI physical evidence.

## Physical evidence session hardening

Mixed-DPI finalization now distinguishes the real interactive desktop from service/session display surfaces. A SentinelX service probe on DESKTOP-ONDD84S was observed as non-interactive Session 0 and exposed a `WinDisc` 1024x768 surface; this is now explicitly rejected for physical release evidence. The remaining mixed-DPI run must execute in an interactive user session with real display devices.

The mixed-DPI provenance guard now has deterministic context/device SelfTest coverage in addition to the hosted-runner smoke: non-interactive, Session 0, `WinDisc`, and prefix-spoof display names are rejected while a normal interactive context and standard display names are accepted.

Mixed-DPI provenance now survives all the way into final evidence: live evidence files carry interactive desktop context and standard display-device topology, the final bundle requires them, and final promotion independently revalidates the subordinate evidence. A synthetic bad-context subordinate-evidence case is rejected in promotion SelfTest.

## Versioned final-gate launcher ready

The final physical gate no longer depends on an ad-hoc Desktop script. `.github/scripts/mixed-dpi-live-launcher.ps1` is the canonical interactive entrypoint, with package identity derived from release state and a CI SelfTest. The physical host can keep a tiny Desktop wrapper, but the release logic now lives in the repository under an allowed post-package path and does not invalidate the sealed package.


## 2026-10-04 refreshed interactive mixed-DPI blocker evidence

- Current authoritative unresolved blocker: `mixed_dpi`.
- Evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`.
- Evidence Git blob: `71da68834378b99dd8fdbdf7687378f664f722bf`.
- Collected through the versioned interactive live launcher from Windows user session 1 on `DESKTOP-ONDD84S`; `user_interactive=true`, standard `\\.\DISPLAY1` device naming verified.
- Physical topology at collection time: one active 1600x900 display, 96 DPI / 100%, one distinct effective DPI value.
- Result remains `BLOCKED` because at least two real active displays with distinct effective DPI values are required.
- Package seal remains unchanged: source `3dfe4ab4ae381c6e5fc8720e76254be0b3f8659d`, SHA-256 `5639177286DEEBBC6794CCAE9475E02C88CF05F001693484643EC8CE7D6ABA57`, 1,894,905 bytes.
