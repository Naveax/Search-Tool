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

Current merged `main`: `858476a05b33f129b72aa6b3c240a6cb71d4a537`.

Validated packaged source:

`6c4141d0bcf12ade21cf633fbaf42d361eb12977`

Current candidate ZIP SHA-256:

`0A48E17886874CD692206B2424A5F0459A683C75FE2FE0DE8A821030950E8E65`

Size: **1,888,674 bytes**.

Package evidence:

`docs/evidence/windows-release-gate-pr15-display-validation-20261001.json`

Exact packaged-source CI: `36847421304` SUCCESS on Windows + Ubuntu.

Full hosted Windows package-refresh release gate: `36848221272` SUCCESS, summary PASS, artifact seal re-hash PASS.

The later PR #16-#18 changes are package-equivalent because they touch only `.github/` and `docs/`. CI enforces that invariant through `docs/RELEASE_STATE.json`, `.github/scripts/release-state-check.ps1` and `.github/scripts/release-state-selftest.ps1`.

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
- BLOCKED Defender evidence falsely promoted to PASS.

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

## Remaining external validation gates

Only three environment-dependent evidence groups remain. They are not known product failures.

### 1. Defender + SmartScreen

Requirements:

- genuinely protected interactive Windows host;
- Microsoft Defender AM service, antivirus, realtime protection, behavior monitor and antispyware all active;
- no overlapping Defender exclusion for the candidate;
- SmartScreen enabled;
- unsigned/untrusted candidate with Internet-zone MOTW;
- observed interactive SmartScreen outcome.

Defender:

```powershell
.\scripts\defender-check.ps1 \
  -Path .\target\release \
  -CustomScan \
  -Enforce \
  -OutputJson .\docs\evidence\defender-active-final.json
```

SmartScreen readiness before launching the artifact:

```powershell
.\scripts\smartscreen-validation.ps1 \
  -Artifact .\target\release\search-tool-gui.exe \
  -RequireEnabled \
  -RequireMotw \
  -ObservedOutcome NotObserved \
  -OutputJson .\docs\evidence\smartscreen-ready-final.json
```

After launching the MOTW-marked artifact interactively and observing the actual result, record only what happened. For example, if SmartScreen warned:

```powershell
.\scripts\smartscreen-validation.ps1 \
  -Artifact .\target\release\search-tool-gui.exe \
  -RequireEnabled \
  -RequireMotw \
  -ObservedOutcome Warned \
  -Enforce \
  -OutputJson .\docs\evidence\smartscreen-final.json
```

Use `Blocked` instead of `Warned` only if that is the observed UI outcome. Do not convert `Allowed` or `NotObserved` into PASS.

Current environment status: BLOCKED. The authorized physical host recheck on 2026-10-01 still had Defender AM/AV/realtime/behavior/antispyware all disabled.

### 2. Physical multi-monitor mixed-DPI / topology

Requirements:

- at least two active physical monitors;
- distinct effective DPI values;
- same prepared Search Tool GUI process must survive topology validation.

Initial probe:

```powershell
.\scripts\display-validation.ps1 \
  -Mode Probe \
  -RequireMixedDpi \
  -Enforce \
  -OutputJson .\docs\evidence\display-mixed-dpi-probe-final.json
```

Move the GUI across all monitors and validate per-monitor DPI:

```powershell
.\scripts\display-validation.ps1 \
  -Mode Exercise \
  -RequireMixedDpi \
  -Enforce \
  -OutputJson .\docs\evidence\display-mixed-dpi-exercise-final.json
```

Primary-monitor change:

```powershell
.\scripts\display-validation.ps1 \
  -Mode PrepareTopology \
  -RequireMixedDpi \
  -StateFile .\display-primary-state.json \
  -OutputJson .\docs\evidence\display-primary-prepare-final.json

# Change the primary monitor in Windows while the prepared GUI process remains running.

.\scripts\display-validation.ps1 \
  -Mode VerifyTopology \
  -RequireMixedDpi \
  -StateFile .\display-primary-state.json \
  -ExpectedTopologyChange PrimaryChanged \
  -Enforce \
  -OutputJson .\docs\evidence\display-primary-verify-final.json
```

Monitor-removal recovery:

```powershell
.\scripts\display-validation.ps1 \
  -Mode PrepareTopology \
  -RequireMixedDpi \
  -StateFile .\display-remove-state.json \
  -OutputJson .\docs\evidence\display-remove-prepare-final.json

# Move/leave the prepared GUI on the monitor that will be removed, then disconnect/disable that monitor.

.\scripts\display-validation.ps1 \
  -Mode VerifyTopology \
  -RequireMixedDpi \
  -StateFile .\display-remove-state.json \
  -ExpectedTopologyChange MonitorRemoved \
  -Enforce \
  -OutputJson .\docs\evidence\display-remove-verify-final.json
```

The verifier requires the window to have intersected an actually removed monitor, the exact prepared GUI PID to survive, recovery onto an active monitor and recovered window DPI to match an intersected active monitor.

Current environment status: BLOCKED. The authorized physical host recheck on 2026-10-01 still exposed exactly one active physical monitor.

### 3. Real Web Resolver provider/cache/privacy path

Requirements:

- valid Google Custom Search API key in `SEARCH_TOOL_GOOGLE_KEY`;
- valid Custom Search Engine ID in `SEARCH_TOOL_GOOGLE_CX`;
- release CLI available at `.\target\release\search-tool.exe`.

Set credentials only in the process/session used for validation. Do not commit them.

```powershell
$env:SEARCH_TOOL_GOOGLE_KEY = '<key>'
$env:SEARCH_TOOL_GOOGLE_CX = '<cx>'

.\scripts\web-resolver-validation.ps1 \
  -Enforce \
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

- Defender + interactive SmartScreen: **BLOCKED BY ENVIRONMENT**.
- Physical multi-monitor mixed-DPI/topology: **BLOCKED BY ENVIRONMENT**.
- Web Resolver real provider/cache/privacy path: **BLOCKED BY CREDENTIALS**.

Everything else required by the current release matrix is already PASS.

For continuation, use:

- `docs/HANDOFF.md`
- `docs/STATUS.md`
- `docs/ROADMAP.md`
- `docs/TEST_MATRIX.md`
- `docs/RELEASE_STATE.json`
