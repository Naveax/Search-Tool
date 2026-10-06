# Development Status

Last updated: 2026-10-06.

## Native-first PR #62 candidate — package INVALIDATED (2026-10-06)

This section describes the unsealed PR #62 source. Previously sealed and deployed package details below are historical evidence, not approval for the PR #62 candidate.

- Package status: **INVALIDATED**. Full exact-head Windows+Ubuntu CI, new canonical Windows package/pristine artifacts and reseal are required.
- Latest packaged-input change: `aeed22401cfe972f466fdb7b39a1e8949528ef46` (PR #62 Explorer cleanup and temp-name hardening); later docs-only commits do not change package inputs.
- Previous sealed-source CI (historical): `37357508957` — Windows + Ubuntu SUCCESS.
- First PR #62 run `37365145724`: Ubuntu SUCCESS, Windows job CANCELLED without runner assignment on 2026-10-05; GitHub reports a stale QUEUED workflow/check-suite and no Windows artifact. Not a complete exact-head CI PASS.
- Previous Windows package artifact (historical): `11365148367` (`SearchTool-Windows-x64`).
- Previous pristine validation artifact (historical): `11366210455` — PASS.
- Previous sealed ZIP (not PR #62): SHA-256 `07A02DB4F18FFD8D8DDD428DCB6C8B3C1AF679E5F263A6B5C5EC35AF79581A4D`; size **1,920,992 bytes**.
- Previous package evidence (historical): `docs/evidence/windows-release-gate-pr59-start-menu-native-first-37357508957-20261005.json`; Git blob `06aa43ce158ab72cd5cab15f86ac3307fa54e152`.
- Local PR #62 validation: **137 tests PASS** (76 core + 9 platform + 14 CLI + 28 GUI + 4 service + 6 worker), fmt/clippy/release build/ZIP verification PASS; hosted Windows artifact is still unavailable.
- Native shell policy: Win, taskbar Search and File Explorer search stay on Microsoft's own Windows UI. Resident startup uses `--no-shell-bridge`; the legacy keyboard bridge is opt-in only via `--shell-bridge`.
- Native search ownership: installer does not register `SearchTool.Search`, `search:` OpenWith, Capabilities or RegisteredApplications ownership. Private `searchtool:` and explicitly invoked scoped search remain available; PR #62 removes all three legacy Explorer right-click shell verbs during install/upgrade.
- Native theme layer: system/app light-dark mode, Windows transparency, accent color and accent surfaces are changed through Windows Personalization/DWM settings; Search/Explorer/Start remain Windows-drawn controls.
- Physical runtime evidence remains `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` / blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`.
- Six-hour source-freeze soak remains `docs/evidence/soak-6h-fa92628-final-20260930.json` / blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.
- Completed external gates remain `smartscreen`, `defender`, `web_resolver`: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` / `355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`; `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json` / `3349e503636f5c9c0a2613892b62c5bac15b0e02`; `docs/evidence/web-resolver-searxng-packaged-pass-37029906278-20261003.json` / `cb239296533c381ce32f59f36ad2b1e9a016d4e0`.
- Sole unresolved external blocker remains `mixed_dpi`: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json` / blob `71da68834378b99dd8fdb7687378f664f722bf`.

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

## Current state

Search Tool's native-first Windows shell milestone and Start Menu cleanup are implemented, package-validated and physically deployed. Win/Search/Explorer remain Windows-owned surfaces with no separate default Start Menu search-panel entry. The only external release dependency remains the real-hardware mixed-DPI gate.

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

## Previous read-only-index release state (historical)

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

## Merged Shell integration checkpoint

PR #7 merged to `main` as `4700a6c` after exact-head CI `36786906440` passed. The registry rollback fix at code head `5ec0a74` was independently validated by CI run `36786244058`, including the disposable HKCU snapshot self-test and 10-boundary installer fault matrix. The final six-hour `fa92628` source-freeze soak is sealed PASS, and the merged source subsequently passed both GitHub main CI and the physical Windows release gate.

## Remaining work

Release/runtime code blockers:
- **none known**. Current package, service, installer, native Search/Explorer integration, read-only index access, external web resolver, Defender and SmartScreen validation are PASS.

External release blocker:
- **mixed_dpi only**. The latest interactive probe on `DESKTOP-ONDD84S` sees one real active `\\.\DISPLAY1` surface at 1600x900 / 96 DPI / 100%, so the final two-monitor exercise cannot run yet.
- Parsec/session/virtual display surfaces cannot satisfy this gate: the finalizer intentionally rejects non-standard/virtual devices and requires a real monitor-removal step.

Final evidence already complete:
- 137 workspace tests, hosted NTFS/USN/journal/rollback/package/pristine validation, six-hour soak, native-first physical sealed deployment, SmartScreen, Defender and API-keyless SearXNG Web Resolver are PASS.
- Production `SearchToolIndexer` is Running + Automatic and the resident GUI is installed under `C:\Program Files\Search Tool`.

The ordered continuation plan is in `docs/ROADMAP.md`; the self-contained project handoff is `docs/HANDOFF.md`.
### 2026-10-02 Web Resolver credential-first physical verification

Physical main checkout 218ac7eadb72412be0e68bccf11368d48b724faf reverified the PR #28 behavior with no release CLI binary present: the orchestrator returned Web Resolver BLOCKED for missing Google key/CX, not for CLI availability; aggregate counts were 0 PASS / 0 READY / 1 BLOCKED / 0 FAIL / 3 SKIPPED. Evidence: docs/evidence/web-resolver-credential-first-218ac7e-20261002.json. This is supplemental only and does not change the canonical blocker seals.

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

## Remote interactive execution path ready

The final physical gate can now be driven remotely from SentinelX without accepting Session 0 display state. `.github/scripts/mixed-dpi-interactive-task.ps1` temporarily enters the logged-in user's real desktop through Task Scheduler `InteractiveToken`, runs one finalizer stage, verifies interactive-session provenance, and cleans up the task. Physical-host Probe verification succeeded in `umut` Session 1 and still correctly reports the sole blocker: only one real 96-DPI monitor is active.

## Distinctive Search UI v1 in development

The active UI branch `ui/distinctive-search-v1` moves the product away from a Windows Search lookalike. It adds a branded header/subtitle, card-style result rendering with accent rails and FILE/FOLDER badges, Signature/Midnight/Graphite/Frost/Native presets, GUI-accessible palette controls, result density, panel sizing, window opacity, and GDI+ background-image rendering with fit/fill/stretch plus image opacity. Appearance changes hot-reload and persist to `%APPDATA%\SearchTool\ui.conf`.

Windows compatibility is explicit rather than accidental: Windows 10 build families use legacy-safe DPI/font/frame fallbacks; Windows 11 build 22000+ enables modern frame attributes, and system backdrop/Mica is gated to build 22621+. Unit tests cover representative Windows 10 builds 10240 through 19045 and Windows 11 builds 22000/22621/22631/26100. This branch changes packaged GUI runtime input, so the previous sealed release package remains historical evidence only once this UI branch is promoted; a new package candidate and validation cycle will be required.

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
