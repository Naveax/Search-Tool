# Search Tool Validation

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

This file is the executable validation runbook. For release ownership, use `docs/RELEASE_STATE.json`; for the detailed evidence matrix, use `docs/TEST_MATRIX.md`.

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

## Required local checks

Windows PowerShell:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

MSRV is Rust 1.89. Rust 1.98 is the currently exercised Windows CI/release toolchain.

Current workspace test count: **137**.

## Previous sealed production package (historical, not PR #62)

Current release authority is `docs/RELEASE_STATE.json`: **VALIDATED** for the new PR #62 sealed artifact listed above. The older Start Menu cleanup ZIP below remains a historical physical deployment, not the new package.

`docs/RELEASE_STATE.json` is authoritative.

Previously sealed packaged source:

`8bd3e8933d0482851a38bfed569458af3973b139`

Previously sealed ZIP SHA-256:

`07A02DB4F18FFD8D8DDD428DCB6C8B3C1AF679E5F263A6B5C5EC35AF79581A4D`

Size: **1,920,992 bytes**.

Package evidence:

`docs/evidence/windows-release-gate-pr59-start-menu-native-first-37357508957-20261005.json`

Evidence Git blob:

`06aa43ce158ab72cd5cab15f86ac3307fa54e152`

Previous sealed-source/full CI: `37357508957` SUCCESS on Windows + Ubuntu; package artifact `11365148367`; pristine validation artifact `11366210455`.

The pristine machine flow confirms the native-first default: resident startup is `--resident --no-shell-bridge`, Windows `search:` ownership remains untouched, and no separate Search Tool custom-search shortcut is exposed in the Start Menu.

Physical deployment of this sealed package on `DESKTOP-ONDD84S` is PASS: `docs/evidence/windows-physical-native-first-startmenu-final-11365148367-20261005.json` / blob `09baf9a8c1caeb17d55e7e3fcddec5b0266877a5`. The legacy Start Menu custom-search shortcut was present before upgrade and absent afterward; installed binaries match the sealed ZIP.

Post-package changes remain package-equivalent only while they stay inside the release-state allowlist (`.github/` and `docs/`). CI enforces this through `docs/RELEASE_STATE.json`, `.github/scripts/release-state-check.ps1` and `.github/scripts/release-state-selftest.ps1`.

## Physical/runtime release evidence

Physical runtime gate source:

`d01b2717127adde68d0a21767aa494d6826ee537`

Evidence:

`docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json`

The physical gate passed:

- release preflight;
- cargo fmt;
- cargo clippy with `-D warnings`;
- all 117 workspace tests;
- release build;
- CLI smoke;
- NTFS/MFT/USN/service integration;
- USN journal reset recovery;
- portable package build and integrity;
- clean install/uninstall smoke;
- single-monitor live GUI move/DPI validation.

The production `SearchToolIndexer` remained Running + Automatic during the physical gate.

## Six-hour source-freeze soak

Status: **PASS**.

Frozen runtime/service source:

`fa92628d515fe25681972fc983f427e1f5108fb3`

Evidence:

`docs/evidence/soak-6h-fa92628-final-20260930.json`

Result: 21,873.82 s, 223,632 filesystem operations, 9,318 validation checks, intentional service crash/restart exercised, exact source/service identity PASS, post-run doctor + verify-deep PASS, service Running + Automatic.

Do not rerun the six-hour soak unless search-core/platform/service runtime inputs change.

## Release-state integrity checks

Normal check:

```powershell
.\.github\scripts\release-state-check.ps1
```

Fail-closed self-test:

```powershell
.\.github\scripts\release-state-selftest.ps1
```

The self-test requires rejection of:

- a synthetic packaged-input mutation to `README.md`;
- a stale package SHA-256;
- a stale synchronized workspace-test count;
- BLOCKED Defender evidence falsely promoted to PASS;
- a missing required unresolved external-blocker entry;
- a missing required completed external-gate entry;
- a completed external gate whose result is not PASS;
- a missing required synchronized document;
- an unsafe expansion of the post-package allow-prefix set;
- a missing required transient validation path;
- swapped evidence paths between required unresolved blockers;
- a required blocker or completed-gate evidence file whose Git blob differs from the sealed value;
- a package/physical/6-hour-soak evidence file whose Git blob differs from the sealed value;
- a synchronized document missing any required release-state evidence marker.

For a `VALIDATED` package, the checker requires the exact current structural sets: unresolved blocker `mixed_dpi`; completed external gates `smartscreen`, `defender`, `web_resolver`; synchronized documents `HANDOFF`, `STATUS`, `ROADMAP`, `TEST_MATRIX`, `VALIDATION`; post-package prefixes `.github/` and `docs/`; transient path `.github/workflows/pr15-release-gate.yml`. Each gate is pinned to exact evidence path + Git blob at the checked `HeadRef`; changing identity, result or content without an explicit release-state update must fail.

Current unresolved-blocker evidence Git blob seal: `mixed_dpi=71da68834378b99dd8fdbdf7687378f664f722bf`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`, `web_resolver=cb239296533c381ce32f59f36ad2b1e9a016d4e0`.

Core release evidence seals: package `docs/evidence/windows-release-gate-pr53-readonly-index-37298666884-20261005.json` blob `1d6835e33cd552cdeb6da7c551bbb70619992f67`; physical runtime `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.

While package status is `VALIDATED`, changes after packaged source `67db5fd09515fa79a3652dd589ae00f464d4b1e3` are allowed only under `.github/` and `docs/`. A change to packaged/runtime inputs must invalidate or replace the current package seal.

## Full Windows release gate

```powershell
.\scripts\windows-release-gate.ps1 -SoakMinutes 5
```

The destructive NTFS/journal tests operate on temporary isolated VHDs. Do not reset the journal on the real system volume.

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

# Pristine default-path validation. Run only on a genuinely clean machine/VM.
.\scripts\pristine-validation.ps1 -Package .\dist\SearchTool-Windows-x64.zip -OutputJson .\docs\evidence\pristine-machine.json
```

### One-command external validation orchestrator

The repository-level wrapper aggregates the four remaining environment-dependent gates without changing packaged/runtime inputs:

```powershell
.\.github\scripts\external-validation-orchestrator.ps1
```

Defaults:
- Defender: active-protection check plus custom scan of `target\release`; for readiness-only probing, `-DefenderCustomScan:$false` checks the repository root and does not require built release binaries;
- SmartScreen: `dist\SearchTool-Windows-x64.zip`, enabled-policy + Internet-zone MOTW requirements, observed outcome initially `NotObserved`;
- display: mixed-DPI `Probe`;
- Web Resolver: `target\release\search-tool.exe` and the current process environment for Google key/CX.

The aggregate result is fail-closed:
- `PASS`: every non-skipped gate produced final PASS;
- `READY`: no failure/blocker exists, but an interactive or staged topology action is still required;
- `BLOCKED`: the host, display surface or credentials are insufficient;
- `PARTIAL`: one or more gates were explicitly skipped;
- `FAIL`: a validator failed, detected a problem, returned an unknown state or threw after claiming PASS.

Use `-EnforceAll` when the run is intended to be final evidence; any aggregate result other than PASS then throws.

SmartScreen example after an actual interactive warning:

```powershell
.\.github\scripts\external-validation-orchestrator.ps1 `
  -ObservedSmartScreenOutcome Warned `
  -EnforceAll
```

Mixed-DPI movement exercise:

```powershell
.\.github\scripts\external-validation-orchestrator.ps1 `
  -DisplayMode Exercise
```

Staged topology change, preserving the same state file between prepare and verify:

```powershell
.\.github\scripts\external-validation-orchestrator.ps1 `
  -DisplayMode PrepareTopology `
  -DisplayExpectedTopologyChange PrimaryChanged `
  -SkipDefender -SkipSmartScreen -SkipWebResolver

# Change the primary monitor, then:
.\.github\scripts\external-validation-orchestrator.ps1 `
  -DisplayMode VerifyTopology `
  -DisplayExpectedTopologyChange PrimaryChanged `
  -SkipDefender -SkipSmartScreen -SkipWebResolver
```

The wrapper records only credential presence; it does not emit the key or CX value. Missing Google key/CX is now detected before CLI existence is required, so a source-only checkout can report the real credential blocker without a built `search-tool.exe`. If both credentials are present, the release CLI must then exist and the full provider/cache/privacy validator runs.

## Remaining external validation gates

Only three unresolved environment-dependent evidence groups remain. They are not known product failures. SmartScreen is complete and retained below as a finished reference.

### 1. Defender

Requirements:

- genuinely protected interactive Windows host;
- Microsoft Defender AM service, antivirus, realtime protection, behavior monitor and antispyware all active;
- no overlapping Defender exclusion for the candidate.

Defender:

```powershell
.\scripts\defender-check.ps1 `
  -Path .\target\release `
  -CustomScan `
  -Enforce `
  -OutputJson .\docs\evidence\defender-active-final.json
```

Physical-host Defender status remains BLOCKED for that specific machine, but final Defender release evidence is PASS from hosted run `36972721866`. The authorized physical host still has Defender AM/AV/realtime/behavior/antispyware disabled. A reversible enablement attempt changed policy values temporarily, but protected-service startup changes were denied, services remained Disabled/Stopped, and `Set-MpPreference` failed with `0x800106ba`; the original policy was restored.

#### Completed SmartScreen final validation

SmartScreen is now PASS. The exact sealed release ZIP from run `36848221272` was rehashed to `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`; `search-tool-gui.exe` was extracted, staged with `ZoneId=3` MOTW and validated as unsigned. Temporary SmartScreen policy `EnableSmartScreen=1` / `ShellSmartScreenLevel=Warn` produced readiness `READY_FOR_INTERACTIVE_CHECK`. Launching the binary created SmartScreen Debug Event 1000 for `search-tool-gui.exe` with `Enforcement=warnByPolicy`, `Experience=Untrusted`, and MOTW `ZoneId=3`. The validator then recorded `ObservedOutcome=Warned` and PASS. All temporary SmartScreen policy/log changes were rolled back and no SmartScreen/GUI process remained. Evidence: `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`.

### 2. Physical multi-monitor mixed-DPI / topology

Requirements:

- at least two active physical monitors;
- distinct effective DPI values;
- same prepared Search Tool GUI process must survive topology validation.

Initial probe:

```powershell
.\scripts\display-validation.ps1 `
  -Mode Probe `
  -RequireMixedDpi `
  -Enforce `
  -OutputJson .\docs\evidence\display-mixed-dpi-probe-final.json
```

Move the GUI across all monitors and validate per-monitor DPI:

```powershell
.\scripts\display-validation.ps1 `
  -Mode Exercise `
  -RequireMixedDpi `
  -Enforce `
  -OutputJson .\docs\evidence\display-mixed-dpi-exercise-final.json
```

Primary-monitor change:

```powershell
.\scripts\display-validation.ps1 `
  -Mode PrepareTopology `
  -RequireMixedDpi `
  -StateFile .\display-primary-state.json `
  -OutputJson .\docs\evidence\display-primary-prepare-final.json

# Change the primary monitor in Windows while the prepared GUI process remains running.

.\scripts\display-validation.ps1 `
  -Mode VerifyTopology `
  -RequireMixedDpi `
  -StateFile .\display-primary-state.json `
  -ExpectedTopologyChange PrimaryChanged `
  -Enforce `
  -OutputJson .\docs\evidence\display-primary-verify-final.json
```

Monitor-removal recovery:

```powershell
.\scripts\display-validation.ps1 `
  -Mode PrepareTopology `
  -RequireMixedDpi `
  -StateFile .\display-remove-state.json `
  -OutputJson .\docs\evidence\display-remove-prepare-final.json

# Move/leave the prepared GUI on the monitor that will be removed, then disconnect/disable that monitor.

.\scripts\display-validation.ps1 `
  -Mode VerifyTopology `
  -RequireMixedDpi `
  -StateFile .\display-remove-state.json `
  -ExpectedTopologyChange MonitorRemoved `
  -Enforce `
  -OutputJson .\docs\evidence\display-remove-verify-final.json
```

The verifier requires the window to have intersected an actually removed monitor, the exact prepared GUI PID to survive, recovery onto an active monitor and recovered window DPI to match an intersected active monitor.

Current environment status: BLOCKED. The authorized physical host recheck on 2026-10-01 still exposed exactly one active physical monitor.

A later current-main preflight on `ed65fbc3a9c1472fe99bb5731ecae32eeda46d67` exercised the aggregate orchestrator without changing system security settings: 3 BLOCKED/UNAVAILABLE gates, 0 FAIL, 1 intentionally skipped SmartScreen gate. Defender was policy-disabled (`WinDefend` and `WdNisSvc` Stopped/Disabled; `DisableAntiSpyware=1`, `DisableAntiVirus=1`), the display remained one 1600x900 @ 96 DPI monitor, and both Google credential-presence checks were false. SmartScreen machine/user/policy configuration remained unset, and no final interactive SmartScreen claim was made because the candidate ZIP was not present. Supplemental evidence: `docs/evidence/external-validation-physical-preflight-ed65fbc-20261001.json`.

A follow-up physical readiness check downloaded the exact sealed ZIP from release-gate run `36848221272` and verified `bytes=1888674` plus SHA-256 `0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`. A copy received a staged `Zone.Identifier` ADS with `ZoneId=3`; the byte hash and length remained unchanged. `smartscreen-validation.ps1 -RequireEnabled -RequireMotw` then reported `motw_internet_zone=true` and `unsigned_or_untrusted_artifact=true`, but `effective_enabled=null` and `protective_outcome_observed=false`, so the result correctly remained BLOCKED. This is readiness evidence only; do not treat staged MOTW as final interactive SmartScreen evidence. Evidence: `docs/evidence/smartscreen-physical-readiness-0aa5266-20261001.json`.

A historical source-only pre-PASS preflight on `d376b244165c067be951ae45e0f9d5ea51129e3a` ran all four external gates and returned `BLOCKED` with Defender, SmartScreen, display and Web Resolver unresolved at that time. Evidence: `docs/evidence/external-validation-current-main-d376b244-20261002.json`.

After SmartScreen final PASS was sealed, current `main` `eae87f121f7b6333e0919696f0678649b0228e56` was rechecked on the authorized Windows host. Release-state verification passed, SmartScreen was intentionally skipped as a completed gate, and the remaining three gates were run together. Aggregate result: `BLOCKED`, 3 unresolved, 0 FAIL, 1 skipped; Defender `UNAVAILABLE`, display `BLOCKED` at one 1600x900 @ 96-DPI monitor, Web Resolver `BLOCKED` because key/CX are absent before CLI existence matters. Evidence: `docs/evidence/external-validation-three-unresolved-eae87f1-20261002.json`.

### 3. Real Web Resolver provider/cache/privacy path

Requirements:

- valid Google Custom Search API key in `SEARCH_TOOL_GOOGLE_KEY`;
- valid Custom Search Engine ID in `SEARCH_TOOL_GOOGLE_CX`;
- release CLI available at `.\target\release\search-tool.exe`.

Set credentials only in the process/session used for validation. Do not commit them.

```powershell
$env:SEARCH_TOOL_GOOGLE_KEY = '<key>'
$env:SEARCH_TOOL_GOOGLE_CX = '<cx>'

.\scripts\web-resolver-validation.ps1 `
  -Enforce `
  -OutputJson .\docs\evidence\web-resolver-final.json

Remove-Item Env:SEARCH_TOOL_GOOGLE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:SEARCH_TOOL_GOOGLE_CX -ErrorAction SilentlyContinue
```

A PASS proves:

1. first request exits zero and reports `source=web`;
2. cache is created and non-empty;
3. credentials are removed before the second request;
4. second request exits zero and reports `source=cache`;
5. the private parent marker is absent from both outputs and the cache.

Current environment status: BLOCKED. The physical host and GitHub-hosted repository-secret probe both lack the required key/CX.

## Power-cycle validation

Sleep/resume or reboot can be validated with the same state file:

```powershell
.\scripts\power-cycle-validation.ps1 -Mode Prepare -Drive C: -IndexRoot C:\ProgramData\SearchTool\index
# Perform the controlled sleep/resume or reboot.
.\scripts\power-cycle-validation.ps1 -Mode Verify -Drive C: -IndexRoot C:\ProgramData\SearchTool\index
```

The script records marker visibility, boot time, USN checkpoint hash, SCM service state, `doctor` and `verify-deep` results.

Sleep/resume and real reboot continuity are both already PASS. Evidence is recorded in the test matrix and handoff.

## Current external blocker summary

- Defender active-protection/custom-scan evidence: **PASS**.
- SmartScreen interactive protective outcome: **PASS** (`Warned`, `warnByPolicy`).
- Physical multi-monitor mixed-DPI/topology: **BLOCKED BY ENVIRONMENT**.
- Web Resolver real provider/cache/privacy path: **BLOCKED BY CREDENTIALS**.

Current unresolved blocker evidence is sealed by:

- `docs/evidence/display-mixed-dpi-blocked-d01b271-20261001.json`
- `docs/evidence/web-resolver-hosted-secrets-blocked-20261001.json`

Completed external-gate evidence is sealed by:

- `docs/evidence/smartscreen-physical-pass-f322126-20261002.json`
- `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`

Everything else required by the current release matrix is already PASS.

For continuation, use:

- `docs/HANDOFF.md`
- `docs/STATUS.md`
- `docs/ROADMAP.md`
- `docs/TEST_MATRIX.md`
- `docs/RELEASE_STATE.json`

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


## Final mixed-DPI evidence runbook

Use .github/scripts/mixed-dpi-finalizer.ps1 for the last external gate. It is deliberately outside packaged inputs, so adding or hardening this orchestration does not invalidate the sealed package.

Command sequence:

    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode Probe
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode Exercise
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode PreparePrimaryChanged
    # Change the Windows primary display, keep the Search Tool GUI open.
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode VerifyPrimaryChanged
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode PrepareMonitorRemoved
    # Physically disconnect the TARGET_DEVICE printed by the command.
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode VerifyMonitorRemoved
    .\.github\scripts\mixed-dpi-finalizer.ps1 -Mode Bundle

The finalizer refuses unsealed package bytes, refuses any release state where mixed_dpi is not the sole unresolved blocker, refuses pre-existing ambiguous GUI processes, stages monitor-removal with the exact sealed GUI on the monitor that will be removed, binds prepare/verify to one GUI PID, and fail-closes the final bundle if removal provenance or DPI/window recovery is false. -Mode SelfTest exercises the bundle positive path and proves invalid removal provenance is rejected.

## Promote the final mixed-DPI PASS

Once `.github/scripts/mixed-dpi-finalizer.ps1 -Mode Bundle` produces a PASS bundle, run:

    .\.github\scripts\mixed-dpi-promote.ps1 -Mode Validate -Bundle <bundle-path>
    .\.github\scripts\mixed-dpi-promote.ps1 -Mode Apply -Bundle <bundle-path>

Apply mode is fail-closed. It requires the bundle HEAD to equal the current checkout HEAD, validates the sealed package source/SHA-256/byte-size identity, requires every finalizer check to be true, rehashes all four subordinate evidence files, and verifies the monitor-removal metadata still refers to the sealed package. It then stages canonical mixed-DPI PASS evidence, zero unresolved blockers, `mixed_dpi` as a completed external gate, matching checker/self-test policy, and synchronized documentation. Commit those staged changes and require exact-head CI before merge. CI runs `mixed-dpi-promote.ps1 -Mode SelfTest` on Windows PowerShell to prove the future zero-blocker promotion state is accepted and a bad removal-provenance bundle is rejected.

## Interactive desktop provenance guard

Do not run the live mixed-DPI physical stages from SentinelX service context, Windows Session 0, a service account, or any non-interactive shell. The finalizer records `desktop_context` in Probe output and fail-closes non-interactive/Session 0 contexts before display enumeration can be accepted. It also requires standard Windows interactive display device naming and blocks session/virtual display devices. The physical stages must be launched inside the logged-in user desktop that actually owns the displays being tested.

The finalizer `SelfTest` also uses synthetic desktop contexts and device names to prove non-interactive execution is rejected, Session 0 is rejected, a normal interactive session is accepted, standard `\\.\DISPLAY<n>` names are accepted, `WinDisc` is rejected, and prefix-spoof names such as `\\.\DISPLAY1VIRTUAL` are rejected. The standard-device matcher is end-anchored, not prefix-only.

Final evidence provenance is also sealed into the evidence set itself. Exercise, primary-change verify and monitor-removal verify reports carry `desktop_context` plus their current monitor list; monitor-removal metadata carries its prepare context and exact target display. `Bundle` refuses non-interactive context or non-standard display names, and `mixed-dpi-promote.ps1` independently reopens the four subordinate evidence files and revalidates those provenance fields instead of trusting bundle booleans alone.

## Versioned live launcher

For the final physical run, prefer `.github/scripts/mixed-dpi-live-launcher.ps1` (or the adjacent `.cmd` wrapper). It is version-controlled and reads package identity from `docs/RELEASE_STATE.json`, so no package hash is duplicated in a Desktop-only script. The launcher verifies interactive Windows context, clean/current `main`, release-state consistency and the exact sealed ZIP before starting live finalizer stages. CI runs `mixed-dpi-live-launcher.ps1 -Mode SelfTest` on Windows to keep this operational entrypoint from drifting.


## 2026-10-04 refreshed interactive mixed-DPI blocker evidence

- Current authoritative unresolved blocker: `mixed_dpi`.
- Evidence: `docs/evidence/display-mixed-dpi-blocked-interactive-10b9f9d-20261004.json`.
- Evidence Git blob: `71da68834378b99dd8fdbdf7687378f664f722bf`.
- Collected through the versioned interactive live launcher from Windows user session 1 on `DESKTOP-ONDD84S`; `user_interactive=true`, standard `\\.\DISPLAY1` device naming verified.
- Physical topology at collection time: one active 1600x900 display, 96 DPI / 100%, one distinct effective DPI value.
- Result remains `BLOCKED` because at least two real active displays with distinct effective DPI values are required.
- Package seal remains unchanged: source `3dfe4ab4ae381c6e5fc8720e76254be0b3f8659d`, SHA-256 `5639177286DEEBBC6794CCAE9475E02C88CF05F001693484643EC8CE7D6ABA57`, 1,894,905 bytes.

## Running physical finalizer stages remotely in the logged-in desktop

When control originates from SentinelX/service context, do not run the finalizer directly. Use `.github/scripts/mixed-dpi-interactive-task.ps1`. It resolves the logged-in console account, registers a temporary Task Scheduler definition using the account's `InteractiveToken`, runs exactly one finalizer mode inside that user's interactive desktop, waits for completion, then removes the temporary task and runner files. Example: `.\.github\scripts\mixed-dpi-interactive-task.ps1 -Mode Probe`. The helper itself verifies that the child reported `UserInteractive=true` and `SessionId>0`. A physical-host verification on DESKTOP-ONDD84S produced `SessionId=1`, standard device `\\.\DISPLAY1`, one 1600x900 monitor at 96 DPI, and the expected mixed-DPI BLOCKED result.

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

## Interactive GUI result validation (2026-10-06)

After PR #64 repaired unprivileged live-index refresh, the native Win32 Search Tool GUI has a repeatable physical acceptance test. Run as the interactive normal user:

    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .github/scripts/gui-physical-smoke.ps1 -CaptureScreenshots

- Requires the installed resident Search Tool GUI, running SearchToolIndexer, an existing index, and the interactive user session. It does not run as SYSTEM or mutate Windows Search/Explorer ownership, NTFS ACLs, index contents, or installed binaries.
- Uses existing single-instance query IPC; reads the real GUI's Win32 Edit, ListBox and Static controls; verifies the displayed query, nonempty matching filename, and agreement between list count and status. Saves cropped GUI screenshots and JSON to a temp folder, failing closed if the GUI cannot be brought to foreground for a screenshot.
- Preserves the original GUI query and visible/hidden state. On other machines with different indexed files use the -Queries parameter to supply known local file names.
- Physical Windows 10 Pro 22H2 with exact sealed PR #64 GUI: Windows PowerShell 5.1 and PowerShell 7 PASS for readme (80 results) and notepad (15 results), with visually reviewed screenshots. An intentionally impossible query returns FAIL and exit code 1 (expected fail-closed control).
- Redacted screenshot hashes and test metadata: docs/evidence/windows-physical-gui-search-controls-20261006.json; screenshots stay on the local machine and are not committed.
- This verifies the installed Search Tool's own GUI, not replacement of the native Windows Search/Explorer backend. The external mixed-DPI physical gate remains BLOCKED pending two active monitors with different effective DPI.

### GUI physical acceptance timeout hardening (2026-10-06)

The same physical Search Tool GUI acceptance test now uses bounded Win32 SendMessageTimeout calls (1,500 ms per query/response), bounds the single-instance IPC helper process wait using -TimeoutSeconds, and uses asynchronous window state changes for screenshot capture and restoration. Query restoration is checked; failure becomes FAIL rather than a warning followed by PASS. The underlying production GUI binary, index contents, Windows Search/Explorer ownership and NTFS permissions remain unchanged.

Real interactive Windows 10 desktop: Windows PowerShell 5.1 and PowerShell 7 each PASS for readme (80 results) and notepad (15 results); an intentional zero-result query produces FAIL and exit code 1. Physical screenshot SHA-256 hashes and the test script SHA-256 are recorded in docs/evidence/windows-physical-gui-smoke-timeout-20261006.json. Screenshots remain on the test machine. This does not claim an artificially simulated GUI-hang test; the bounds are implemented directly in the native message and process-wait calls.


## 2026-10-07 content-sidecar failure cleanup (pre-release)

Evidence: `docs/evidence/windows-physical-enospc-content-staging-20261007.json`. A physical Windows 10 host reached 0 free bytes on C:. Recovery removed only generated Rust debug outputs and 312 stale content sidecar temporary files (2,077,846,240 logical bytes), while preserving the published content index, its checkpoints, installed binaries and NTFS permissions.

Two deterministic Rust regression tests were added: `dropping_unfinished_builder_removes_spill_chunks` and `failed_finish_removes_staging_without_destroying_published_content`. They model ordinary abandoned builds and a checkpoint-staging failure; neither test claims to reproduce a literal full-disk fault. The `Drop` path runs while the exclusive content build lock is owned, and newly opened chunk files are tracked before their postings are written.

Local physical Windows validation: 141/141 workspace tests PASS, Clippy PASS, Release build PASS. Previous published content and checkpoints remain present, and normal-user `search-tool search` still returns results. Package status stays INVALIDATED until a newly built archive is sealed and physical acceptance is complete. Mixed-DPI hardware remains unavailable.


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


## 2026-10-07 Windows 11 physical portable GUI
On `NAVEAX` (Windows 11 Pro 23H2, build 22631, user session 1), the exact canonical release ZIP re-hashed to `3D293843BE71D322CFF9729C5678401CAB1FFB0510BBDD897328459531CE40A2`.
Normal-user `ntfs-status C:` correctly failed with Access Denied. A temporary highest-privilege Scheduled Task probe also failed with Access Denied, so no elevation bypass or permanent installation was attempted.
A 100,000-record valid synthetic index was captured from the exact packaged benchmark after its build completed. Exact release `search-tool.exe verify` returned `status=ok`, and CLI `node` search passed. The exact release GUI then rendered at 900x640 with the `WINDOWS 11` banner and, under explicitly approved physical-console control, returned 39 `node` results in 83.2 ms.
After GUI close, all C:/D: temporary test paths were deleted and there was no SearchToolIndexer service, `searchtool:` registration, native Search ProgID or Startup shortcut. Evidence: `docs/evidence/windows11-physical-portable-gui-20261007.json`.
This closes the real Windows 11 GUI compatibility gap. It does not claim Windows 11 service-install validation, because elevation was intentionally not bypassed. Mixed-DPI remains the sole physical blocker.


## 2026-10-07 themed Görünüm canonical acceptance
The exact canonical package from release gate `37616034729` has SHA-256 `E29D50B4BFD0B5AFD523C98BAE3691B167D4C099E53F12736F90CC182516252F` and size 1,921,593 bytes. Its four packaged executable hashes were recorded and later matched byte-for-byte against the production installation.
On real Windows 11 hardware, the exact artifact package was re-hashed, opened against a valid 100,000-record synthetic index, and physically exercised. The new dark-surface/cyan-accent `Görünüm` button rendered correctly; `node` returned 39 visible results in 54.7 ms. The portable test left no service, protocol/ProgID, Startup shortcut or temporary test path.
On DESKTOP-ONDD84S, the exact artifact was installed transactionally as SYSTEM with the existing index preserved. SearchToolIndexer returned Running/Auto; content/checkpoint and user Startup shortcut hashes were preserved across the upgrade; native Search ownership stayed absent; legacy Explorer verbs stayed absent. Normal-user verify returned `status=ok`, 1,173,808 records, and `readme` search exited 0.
A post-deployment physical-console screenshot on DESKTOP-ONDD84S was not completed because the local Nexowire console grant was denied. A stale HWND capture was explicitly rejected as evidence. This limitation does not change package identity: the installed GUI SHA-256 equals the exact artifact GUI binary already physically rendered on Windows 11. Evidence: `docs/evidence/windows-release-gate-theme-button-37616034729-20261007.json`.
`mixed_dpi` remains the sole external physical blocker.


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


## 2026-10-08: native-polish artifact against live production index (pre-deploy)

- Evidence: `docs/evidence/windows-native-polish-real-index-compat-20261008.json`; exact release-gate ZIP and manifest PASS.
- On DESKTOP-ONDD84S the sealed package CLI, run as a normal user without installation, verified the active 1220987-record index and scanned all 1220987 names/IDs using `verify-deep` (`status=ok`). The report includes the nonfatal missing-parent-links count and timings.
- Real-index `readme` and `notepad` searches produced byte-identical results between installed and packaged CLIs, with 50 and 19 results respectively. Production SearchToolIndexer remained Running/Auto and no compaction temps were present at the evidence snapshot.
- The new GUI is NOT installed in production yet. Package release status remains intentionally INVALIDATED until authorized deployment, installed hash parity, service/index continuity and resident GUI verification all PASS. No UAC or elevation workaround was attempted.

## 2026-10-08: native-polish deployment receipt fail-closed verifier (pre-deploy)

- Added `.github/scripts/native-polish-deploy-receipt-check.ps1` to compare the elevated installer receipt and normal-user post-deployment receipt against the sealed canonical ZIP SHA-256 and exact four executable SHA-256 values.
- The read-only evidence checker rejects missing/invalid receipt files, false PASS flags, different host names, mismatched package or installed binaries, service state/startup failures, index path/hash/verify inconsistencies, invalid GUI evidence and backward timestamps. A receipt PASS is explicitly scoped to receipt consistency, not a proof of currently installed files or a substitute for live verification.
- Added `.github/scripts/native-polish-deploy-receipt-selftest.ps1`: ten synthetic fixtures (one positive and nine fail-closed controls) PASS locally under PowerShell 7 and Windows PowerShell 5.1. Both test variants are in Windows CI.
- The existing host-specific `finalize-release-state-guarded.ps1` still independently checks live installed binaries, service/index and interactive GUI before calling the release finalizer. Production deployment of the new package remains PENDING_LOCAL_ADMIN_GRANT; the release state remains INVALIDATED, and mixed-DPI remains BLOCKED.

## 2026-10-08: extended native-polish receipt fingerprints

- Strengthened `.github/scripts/native-polish-deploy-receipt-check.ps1`: after a receipt-only PASS, the data must also contain installer exit 0, schema 1, canonical production index paths, nonempty 64-hex index SHA-256/positive byte lengths, consistent index-preservation flags, exact installed resident GUI executable path, and one `status=ok` line in each index verifier output.
- Added nine fail-closed synthetic cases for those invariants. The synthetic receipt suite now has **19 tests (one positive and eighteen negative)**, PASS under local PowerShell 7 and Windows PowerShell 5.1. None of these tests requires elevation or changes the installed service/index.
- Distinguish receipt consistency from live installation proof. The exact production package remains INVALIDATED until the authorized local deployment and live host finalizer pass; mixed-DPI stays BLOCKED.
