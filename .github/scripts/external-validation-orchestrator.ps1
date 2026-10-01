[CmdletBinding()]
param(
    [ValidateSet('Probe','Exercise','PrepareTopology','VerifyTopology')]
    [string]$DisplayMode = 'Probe',

    [ValidateSet('AnyChange','PrimaryChanged','MonitorRemoved')]
    [string]$DisplayExpectedTopologyChange = 'AnyChange',

    [ValidateSet('NotObserved','Warned','Blocked','Allowed')]
    [string]$ObservedSmartScreenOutcome = 'NotObserved',

    [string]$Artifact,
    [string]$Cli,
    [string]$GuiPath,
    [string]$DisplayStateFile,

    [bool]$DefenderCustomScan = $true,
    [bool]$RequireSmartScreenEnabled = $true,
    [bool]$RequireSmartScreenMotw = $true,
    [bool]$RequireMixedDpi = $true,

    [switch]$SkipDefender,
    [switch]$SkipSmartScreen,
    [switch]$SkipDisplay,
    [switch]$SkipWebResolver,

    [switch]$EnforceAll,
    [string]$OutputDir,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'

function Resolve-FromRoot {
    param(
        [AllowNull()] [AllowEmptyString()] [string]$Value,
        [Parameter(Mandatory)] [string]$DefaultRelative
    )

    $candidate = if ([string]::IsNullOrWhiteSpace($Value)) {
        Join-Path $root $DefaultRelative
    } elseif ([IO.Path]::IsPathRooted($Value)) {
        $Value
    } else {
        Join-Path $root $Value
    }
    return [IO.Path]::GetFullPath($candidate)
}

$Artifact = Resolve-FromRoot -Value $Artifact -DefaultRelative 'dist\SearchTool-Windows-x64.zip'
$Cli = Resolve-FromRoot -Value $Cli -DefaultRelative 'target\release\search-tool.exe'
$GuiPath = Resolve-FromRoot -Value $GuiPath -DefaultRelative 'target\release\search-tool-gui.exe'
$OutputDir = Resolve-FromRoot -Value $OutputDir -DefaultRelative 'dist\external-validation'
$DisplayStateFile = if ([string]::IsNullOrWhiteSpace($DisplayStateFile)) {
    [IO.Path]::GetFullPath((Join-Path $OutputDir 'display-topology-state.json'))
} else {
    Resolve-FromRoot -Value $DisplayStateFile -DefaultRelative 'dist\external-validation\display-topology-state.json'
}
$OutputJson = if ([string]::IsNullOrWhiteSpace($OutputJson)) {
    [IO.Path]::GetFullPath((Join-Path $OutputDir "external-validation-summary-$stamp.json"))
} else {
    Resolve-FromRoot -Value $OutputJson -DefaultRelative ("dist\external-validation\external-validation-summary-$stamp.json")
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$outputParent = Split-Path -Parent $OutputJson
if ($outputParent) {
    New-Item -ItemType Directory -Force -Path $outputParent | Out-Null
}

function New-GateEntry {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [string]$Result,
        [string]$ReportPath,
        [string]$Reason,
        [object]$Report,
        [string]$ValidatorError
    )

    return [ordered]@{
        name = $Name
        result = $Result
        report_path = $ReportPath
        reason = $Reason
        validator_error = $ValidatorError
        report = $Report
    }
}

function Read-Report {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [string]$GateName
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$GateName did not create its expected report: $Path"
    }
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Invoke-Validator {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [string]$ReportPath,
        [Parameter(Mandatory)] [scriptblock]$Action
    )

    Remove-Item -LiteralPath $ReportPath -Force -ErrorAction SilentlyContinue
    $validatorError = $null
    try {
        & $Action | Out-Host
    } catch {
        $validatorError = [string]$_.Exception.Message
    }

    try {
        $report = Read-Report -Path $ReportPath -GateName $Name
        $result = [string]$report.result
        if ([string]::IsNullOrWhiteSpace($result)) {
            return (New-GateEntry -Name $Name -Result 'FAIL' -ReportPath $ReportPath -Reason 'Validator report did not contain a result.' -Report $report -ValidatorError $validatorError)
        }

        $normalizedResult = $result.ToUpperInvariant()
        if ($validatorError -and $normalizedResult -eq 'PASS') {
            return (New-GateEntry -Name $Name -Result 'FAIL' -ReportPath $ReportPath -Reason 'Validator threw after producing a PASS report.' -Report $report -ValidatorError $validatorError)
        }
        return (New-GateEntry -Name $Name -Result $normalizedResult -ReportPath $ReportPath -Report $report -ValidatorError $validatorError)
    } catch {
        $readError = [string]$_.Exception.Message
        $reason = if ($validatorError) { "$validatorError; $readError" } else { $readError }
        return (New-GateEntry -Name $Name -Result 'FAIL' -ReportPath $ReportPath -Reason $reason -ValidatorError $validatorError)
    }
}

function New-SkippedEntry {
    param([Parameter(Mandatory)] [string]$Name)
    return (New-GateEntry -Name $Name -Result 'SKIPPED' -Reason 'Skipped by caller.')
}

function New-BlockedEntry {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [string]$Reason
    )
    return (New-GateEntry -Name $Name -Result 'BLOCKED' -Reason $Reason)
}

function Get-NextAction {
    param([Parameter(Mandatory)] [object]$Gate)

    $name = [string]$Gate.name
    $result = [string]$Gate.result
    switch ($name) {
        'defender' {
            if ($result -in @('BLOCKED','UNAVAILABLE','EXCLUDED')) {
                return 'Run on a Windows host with active Microsoft Defender real-time, behavior, antivirus and antispyware protection and no overlapping exclusion.'
            }
        }
        'smartscreen' {
            if ($result -eq 'READY_FOR_INTERACTIVE_CHECK') {
                return 'Launch the MOTW-marked unsigned artifact interactively, observe Warned or Blocked, then rerun with -ObservedSmartScreenOutcome Warned or Blocked.'
            }
            if ($result -eq 'BLOCKED') {
                return 'Use an interactive Windows host with SmartScreen enabled and Internet-zone MOTW on the artifact.'
            }
        }
        'display' {
            if ($result -eq 'READY') {
                return 'The display surface is ready. Rerun with -DisplayMode Exercise, or use PrepareTopology/VerifyTopology around the required primary-change or monitor-removal action.'
            }
            if ($result -eq 'PREPARED') {
                return 'Perform the requested physical topology change, then rerun with -DisplayMode VerifyTopology using the same state file and expected topology change.'
            }
            if ($result -eq 'BLOCKED') {
                return 'Attach at least two active monitors with distinct effective DPI values.'
            }
        }
        'web_resolver' {
            if ($result -eq 'BLOCKED') {
                return 'Set SEARCH_TOOL_GOOGLE_KEY and SEARCH_TOOL_GOOGLE_CX in the process environment, then rerun.'
            }
        }
    }
    return $null
}

$runningOnWindows = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([Runtime.InteropServices.OSPlatform]::Windows)
$gates = [System.Collections.Generic.List[object]]::new()

if (-not $runningOnWindows) {
    foreach ($name in @('defender','smartscreen','display','web_resolver')) {
        $gates.Add((New-BlockedEntry -Name $name -Reason 'External validation orchestrator requires Windows.'))
    }
} else {
    $defenderReport = Join-Path $OutputDir "defender-$stamp.json"
    if ($SkipDefender) {
        $gates.Add((New-SkippedEntry -Name 'defender'))
    } elseif (-not (Test-Path -LiteralPath (Split-Path -Parent $Cli) -PathType Container)) {
        $gates.Add((New-BlockedEntry -Name 'defender' -Reason "Release binary directory is missing: $(Split-Path -Parent $Cli)"))
    } else {
        $defenderArgs = @{
            Path = (Split-Path -Parent $Cli)
            OutputJson = $defenderReport
        }
        if ($DefenderCustomScan) { $defenderArgs.CustomScan = $true }
        $gates.Add((Invoke-Validator -Name 'defender' -ReportPath $defenderReport -Action {
            & (Join-Path $root 'scripts\defender-check.ps1') @defenderArgs
        }))
    }

    $smartscreenReport = Join-Path $OutputDir "smartscreen-$stamp.json"
    if ($SkipSmartScreen) {
        $gates.Add((New-SkippedEntry -Name 'smartscreen'))
    } elseif (-not (Test-Path -LiteralPath $Artifact -PathType Leaf)) {
        $gates.Add((New-BlockedEntry -Name 'smartscreen' -Reason "Portable artifact is missing: $Artifact"))
    } else {
        $smartArgs = @{
            Artifact = $Artifact
            ObservedOutcome = $ObservedSmartScreenOutcome
            OutputJson = $smartscreenReport
        }
        if ($RequireSmartScreenEnabled) { $smartArgs.RequireEnabled = $true }
        if ($RequireSmartScreenMotw) { $smartArgs.RequireMotw = $true }
        $gates.Add((Invoke-Validator -Name 'smartscreen' -ReportPath $smartscreenReport -Action {
            & (Join-Path $root 'scripts\smartscreen-validation.ps1') @smartArgs
        }))
    }

    $displayReport = Join-Path $OutputDir "display-$stamp.json"
    if ($SkipDisplay) {
        $gates.Add((New-SkippedEntry -Name 'display'))
    } elseif ($DisplayMode -in @('Exercise','PrepareTopology') -and -not (Test-Path -LiteralPath $GuiPath -PathType Leaf)) {
        $gates.Add((New-BlockedEntry -Name 'display' -Reason "GUI binary is missing for $DisplayMode mode: $GuiPath"))
    } else {
        $displayArgs = @{
            Mode = $DisplayMode
            ExpectedTopologyChange = $DisplayExpectedTopologyChange
            StateFile = $DisplayStateFile
            OutputJson = $displayReport
        }
        if ($RequireMixedDpi) { $displayArgs.RequireMixedDpi = $true }
        if (Test-Path -LiteralPath $GuiPath -PathType Leaf) { $displayArgs.GuiPath = $GuiPath }
        $gates.Add((Invoke-Validator -Name 'display' -ReportPath $displayReport -Action {
            & (Join-Path $root 'scripts\display-validation.ps1') @displayArgs
        }))
    }

    $webReport = Join-Path $OutputDir "web-resolver-$stamp.json"
    if ($SkipWebResolver) {
        $gates.Add((New-SkippedEntry -Name 'web_resolver'))
    } else {
        $webKeyPresent = -not [string]::IsNullOrWhiteSpace($env:SEARCH_TOOL_GOOGLE_KEY)
        $webCxPresent = -not [string]::IsNullOrWhiteSpace($env:SEARCH_TOOL_GOOGLE_CX)
        if (-not $webKeyPresent -or -not $webCxPresent) {
            $webPreflight = [ordered]@{
                schema = 1
                timestamp_utc = [DateTime]::UtcNow.ToString('o')
                computer_name = [Environment]::MachineName
                cli = $Cli
                credentials = [ordered]@{
                    google_key_present = [bool]$webKeyPresent
                    google_cx_present = [bool]$webCxPresent
                }
                result = 'BLOCKED'
                reason = 'SEARCH_TOOL_GOOGLE_KEY and SEARCH_TOOL_GOOGLE_CX are required for a real provider validation.'
            }
            $webJson = $webPreflight | ConvertTo-Json -Depth 5
            $webJson | Set-Content -LiteralPath $webReport -Encoding UTF8
            $gates.Add((New-GateEntry -Name 'web_resolver' -Result 'BLOCKED' -ReportPath $webReport -Reason $webPreflight.reason -Report ([pscustomobject]$webPreflight)))
        } elseif (-not (Test-Path -LiteralPath $Cli -PathType Leaf)) {
            $gates.Add((New-BlockedEntry -Name 'web_resolver' -Reason "CLI binary is missing: $Cli"))
        } else {
            $webArgs = @{
                Cli = $Cli
                OutputJson = $webReport
            }
            $gates.Add((Invoke-Validator -Name 'web_resolver' -ReportPath $webReport -Action {
                & (Join-Path $root 'scripts\web-resolver-validation.ps1') @webArgs
            }))
        }
    }
}

$knownPass = @('PASS')
$knownBlocked = @('BLOCKED','UNAVAILABLE','EXCLUDED')
$knownReady = @('READY','PREPARED','READY_FOR_INTERACTIVE_CHECK')
$knownFail = @('FAIL','DETECTION')
$knownSkipped = @('SKIPPED')
$known = @($knownPass + $knownBlocked + $knownReady + $knownFail + $knownSkipped)

$unknownGates = @($gates | Where-Object { $known -notcontains ([string]$_.result).ToUpperInvariant() })
$failedGates = @($gates | Where-Object { $knownFail -contains ([string]$_.result).ToUpperInvariant() })
$blockedGates = @($gates | Where-Object { $knownBlocked -contains ([string]$_.result).ToUpperInvariant() })
$readyGates = @($gates | Where-Object { $knownReady -contains ([string]$_.result).ToUpperInvariant() })
$skippedGates = @($gates | Where-Object { $knownSkipped -contains ([string]$_.result).ToUpperInvariant() })
$passedGates = @($gates | Where-Object { $knownPass -contains ([string]$_.result).ToUpperInvariant() })

$overall = if ($unknownGates.Count -gt 0 -or $failedGates.Count -gt 0) {
    'FAIL'
} elseif ($blockedGates.Count -gt 0) {
    'BLOCKED'
} elseif ($skippedGates.Count -gt 0) {
    'PARTIAL'
} elseif ($readyGates.Count -gt 0) {
    'READY'
} elseif ($passedGates.Count -eq $gates.Count -and $gates.Count -gt 0) {
    'PASS'
} else {
    'FAIL'
}

$nextActions = @($gates | ForEach-Object {
    $action = Get-NextAction -Gate $_
    if ($action) {
        [ordered]@{
            gate = [string]$_.name
            action = $action
        }
    }
})

$summary = [ordered]@{
    schema = 1
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    result = $overall
    computer_name = [Environment]::MachineName
    enforce_all = [bool]$EnforceAll
    inputs = [ordered]@{
        artifact = $Artifact
        cli = $Cli
        gui = $GuiPath
        display_mode = $DisplayMode
        display_expected_topology_change = $DisplayExpectedTopologyChange
        display_state_file = $DisplayStateFile
        observed_smartscreen_outcome = $ObservedSmartScreenOutcome
        defender_custom_scan = $DefenderCustomScan
        require_smartscreen_enabled = $RequireSmartScreenEnabled
        require_smartscreen_motw = $RequireSmartScreenMotw
        require_mixed_dpi = $RequireMixedDpi
    }
    counts = [ordered]@{
        pass = $passedGates.Count
        ready = $readyGates.Count
        blocked = $blockedGates.Count
        failed = $failedGates.Count + $unknownGates.Count
        skipped = $skippedGates.Count
    }
    gates = $gates
    next_actions = $nextActions
}

$json = $summary | ConvertTo-Json -Depth 12
$json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
$json
Write-Host "Summary=$OutputJson"

if ($EnforceAll -and $overall -ne 'PASS') {
    throw "EXTERNAL_VALIDATION=$overall Summary=$OutputJson"
}

if ($overall -eq 'FAIL') {
    throw "EXTERNAL_VALIDATION=FAIL Summary=$OutputJson"
}

Write-Host "EXTERNAL_VALIDATION=$overall"
