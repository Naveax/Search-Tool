# Search Tool Validation

Last updated: 2026-10-01.

This file is the executable validation runbook. For release ownership, use `docs/RELEASE_STATE.json`; for the detailed evidence matrix, use `docs/TEST_MATRIX.md`.

## Required local checks

Windows PowerShell:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

MSRV is Rust 1.89. Rust 1.98 is the currently exercised Windows CI/release toolchain.

Current workspace test count: **117**.

## Current validated release state

The validated package is intentionally tied to its packaged-source commit rather than the current docs-only `main` head.

Validated packaged source:

`6c4141d0bcf12ade21cf633fbaf42d361eb12977`

Current candidate ZIP SHA-256:

`0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`

Size: **1,888,674 bytes**.

Package evidence:

`docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`

Exact packaged-source CI: `36847421304` SUCCESS on Windows + Ubuntu.

Full hosted Windows package-refresh release gate: `36848221272` SUCCESS, summary PASS, artifact seal re-hash PASS.

Post-package changes remain package-equivalent only while they stay inside the release-state allowlist (`.github/` and `docs/`). CI enforces that invariant through `docs/RELEASE_STATE.json`, `.github/scripts/release-state-check.ps1` and `.github/scripts/release-state-selftest.ps1`.

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

For a `VALIDATED` package, the checker requires the exact current structural sets: unresolved blocker names `mixed_dpi`, `web_resolver`; completed external gates `smartscreen`, `defender`; synchronized documents `HANDOFF`, `STATUS`, `ROADMAP`, `TEST_MATRIX`, `VALIDATION`; post-package prefixes `.github/` and `docs/`; and transient path `.github/workflows/pr15-release-gate.yml`. Each unresolved blocker and completed gate is pinned to its exact evidence path and Git blob SHA at the checked `HeadRef`; changing identity, result or content without an explicit release-state update must fail.

Current unresolved-blocker evidence Git blob seals: `mixed_dpi=a1c0c329a1024ab02948361b9f8102e069f0db95`, `web_resolver=ed3d9b56fc75e7d56620e639917988882c732550`. Completed external-gate PASS seals: `smartscreen=355790cc0c0ec4e9aa5ca372f3ac5a58aa1e1952`, `defender=3349e503636f5c9c0a2613892b62c5bac15b0e02`; evidence `docs/evidence/smartscreen-physical-pass-f322126-20261002.json` and `docs/evidence/defender-hosted-active-pass-36972721866-20261002.json`.

Core release evidence seals: package `docs/evidence/windows-release-gate-pr15-display-validation-20261001.json` blob `9bf0fea273b90ac2ba3f164a2ed550cd8cf57294`; physical `docs/evidence/windows-release-gate-d01b271-dpi-topology-20261001.json` blob `dd104790f6c244050e175bb2f8a6d6cd8d1dfac6`; six-hour soak `docs/evidence/soak-6h-fa92628-final-20260930.json` blob `abcc1e0b9acf45d053cd32e8c183abefa6d172e6`.

While package status is `VALIDATED`, changes after packaged source `6c4141d0bcf12ade21cf633fbaf42d361eb12977` are allowed only under `.github/` and `docs/`. A change to packaged/runtime inputs must invalidate or replace the current package seal.

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
