[CmdletBinding()]
param(
    [ValidateSet(
        'Probe',
        'Exercise',
        'PreparePrimaryChanged',
        'VerifyPrimaryChanged',
        'PrepareMonitorRemoved',
        'VerifyMonitorRemoved',
        'Bundle',
        'SelfTest'
    )]
    [string]$Mode = 'Probe',

    [string]$PackageZip,
    [string]$OutputDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# PowerShell hosts are typically DPI unaware (0); GetDpiForMonitor then
# reports 96 for each display regardless of mixed physical scaling.
# Enable per-monitor DPI awareness before invoking the sealed validator.
if (-not ('SearchToolMixedDpiHostAwareness' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class SearchToolMixedDpiHostAwareness {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")]
    public static extern IntPtr GetThreadDpiAwarenessContext();
    [DllImport("user32.dll")]
    public static extern int GetAwarenessFromDpiAwarenessContext(IntPtr context);
    public static int Current() {
        return GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext());
    }
    public static bool EnsurePerMonitorAware() {
        if (Current() == 2) return true;
        return SetProcessDpiAwarenessContext(new IntPtr(-4)) && Current() == 2;
    }
}
"@
}
if (-not [SearchToolMixedDpiHostAwareness]::EnsurePerMonitorAware()) {
    throw 'MIXED_DPI_DPI_CONTEXT_BLOCKED: PowerShell must be per-monitor DPI aware before collecting physical DPI evidence.'
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$displayValidator = Join-Path $root 'scripts\display-validation.ps1'
$releaseStatePath = Join-Path $root 'docs\RELEASE_STATE.json'

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root 'dist\mixed-dpi-final'
} elseif (-not [IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $root $OutputDir
}
$OutputDir = [IO.Path]::GetFullPath($OutputDir)
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($PackageZip)) {
    $PackageZip = Join-Path $root 'dist\SearchTool-Windows-x64.zip'
} elseif (-not [IO.Path]::IsPathRooted($PackageZip)) {
    $PackageZip = Join-Path $root $PackageZip
}
$PackageZip = [IO.Path]::GetFullPath($PackageZip)

$probeReport = Join-Path $OutputDir 'probe.json'
$exerciseReport = Join-Path $OutputDir 'exercise.json'
$primaryState = Join-Path $OutputDir 'primary-change-state.json'
$primaryPrepareReport = Join-Path $OutputDir 'primary-change-prepare.json'
$primaryVerifyReport = Join-Path $OutputDir 'primary-change-verify.json'
$removalState = Join-Path $OutputDir 'monitor-removal-state.json'
$removalPrepareReport = Join-Path $OutputDir 'monitor-removal-prepare.json'
$removalVerifyReport = Join-Path $OutputDir 'monitor-removal-verify.json'
$removalMeta = Join-Path $OutputDir 'monitor-removal-meta.json'
$bundleReport = Join-Path $OutputDir 'mixed-dpi-final-bundle.json'

function Read-JsonFile {
    param([Parameter(Mandatory)] [string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing required JSON file: $Path"
    }
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Get-ReleaseState {
    $state = Read-JsonFile $releaseStatePath
    if ([string]$state.package.status -ne 'VALIDATED') {
        throw 'Release package must be VALIDATED before final mixed-DPI evidence is collected.'
    }

    $blockers = @($state.external_blockers | ForEach-Object { [string]$_.name })
    if ($blockers.Count -ne 1 -or $blockers[0] -ne 'mixed_dpi') {
        throw 'Finalizer requires mixed_dpi to be the only unresolved external blocker.'
    }

    $completed = @($state.completed_external_gates | ForEach-Object { [string]$_.name })
    foreach ($required in @('smartscreen','defender','web_resolver')) {
        if ($completed -notcontains $required) {
            throw "Required completed external gate is missing: $required"
        }
    }
    return $state
}

function Get-DesktopContext {
    $process = [Diagnostics.Process]::GetCurrentProcess()
    return [pscustomobject]@{
        user_interactive = [bool][Environment]::UserInteractive
        session_id = [int]$process.SessionId
        user_name = [string][Environment]::UserName
        session_name = [string][Environment]::GetEnvironmentVariable('SESSIONNAME')
    }
}

function Get-DesktopContextBlockReason {
    param([Parameter(Mandatory)] [object]$Context)

    if (-not [bool]$Context.user_interactive) {
        return 'Physical mixed-DPI evidence requires an interactive Windows user desktop; this process is non-interactive.'
    }
    if ([int]$Context.session_id -le 0) {
        return 'Physical mixed-DPI evidence must not run from Windows Session 0/service context.'
    }
    return $null
}

function Test-StandardDisplayDeviceName {
    param([string]$Device)

    if ([string]::IsNullOrWhiteSpace($Device)) {
        return $false
    }
    return ($Device -match '^\\\\\.\\DISPLAY[0-9]+$')
}

function Test-StandardDisplayDevices {
    param([Parameter(Mandatory)] [object[]]$Monitors)

    if ($Monitors.Count -eq 0) {
        return $false
    }
    foreach ($monitor in $Monitors) {
        if (-not (Test-StandardDisplayDeviceName -Device ([string]$monitor.device))) {
            return $false
        }
    }
    return $true
}

function Test-InteractiveDesktopContextEvidence {
    param([object]$Context)

    if ($null -eq $Context) {
        return $false
    }
    return [string]::IsNullOrWhiteSpace([string](Get-DesktopContextBlockReason -Context $Context))
}

function Bind-DesktopContextToReport {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [object]$Context
    )

    $report = Read-JsonFile $Path
    $report | Add-Member -NotePropertyName desktop_context -NotePropertyValue $Context -Force
    $standardDevices = Test-StandardDisplayDevices -Monitors @($report.monitors)
    if (-not $standardDevices) {
        $report.result = 'FAIL'
        $report | Add-Member -NotePropertyName reason -NotePropertyValue 'Physical mixed-DPI evidence contains a non-standard/session/virtual display device.' -Force
    }
    $report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Path -Encoding UTF8
    if (-not $standardDevices) {
        throw "MIXED_DPI_CONTEXT_BLOCKED: report contains a non-standard/session/virtual display device. Report=$Path"
    }
    return $report
}

function Assert-InteractiveDesktopContext {
    $context = Get-DesktopContext
    $reason = Get-DesktopContextBlockReason -Context $context
    if ($reason) {
        throw "MIXED_DPI_CONTEXT_BLOCKED: $reason user=$($context.user_name) session_id=$($context.session_id)"
    }
    return $context
}

function Invoke-Probe {
    Remove-Item -LiteralPath $probeReport -Force -ErrorAction SilentlyContinue
    $context = Get-DesktopContext
    $contextReason = Get-DesktopContextBlockReason -Context $context

    if ($contextReason) {
        $blocked = [ordered]@{
            schema = 1
            timestamp_utc = [DateTime]::UtcNow.ToString('o')
            mode = 'Probe'
            computer_name = [Environment]::MachineName
            monitor_count = 0
            distinct_dpi_count = 0
            mixed_dpi = $false
            require_mixed_dpi = $true
            monitors = @()
            desktop_context = $context
            host_dpi_awareness = 'PER_MONITOR_AWARE'
            result = 'BLOCKED'
            reason = $contextReason
        }
        $blocked | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $probeReport -Encoding UTF8
        $blocked | ConvertTo-Json -Depth 8 | Write-Host
        Write-Host "Report=$probeReport"
        return [pscustomobject]$blocked
    }

    $args = @{
        Mode = 'Probe'
        RequireMixedDpi = $true
        OutputJson = $probeReport
    }
    & $displayValidator @args | Out-Host
    $probe = Read-JsonFile $probeReport
    $probe | Add-Member -NotePropertyName desktop_context -NotePropertyValue $context -Force
    $probe | Add-Member -NotePropertyName host_dpi_awareness -NotePropertyValue 'PER_MONITOR_AWARE' -Force

    if (-not (Test-StandardDisplayDevices -Monitors @($probe.monitors))) {
        $probe.result = 'BLOCKED'
        $probe.reason = 'Physical mixed-DPI evidence requires standard interactive display devices such as \\.\DISPLAY1; a session or virtual display device was observed.'
    }

    $probe | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $probeReport -Encoding UTF8
    return $probe
}

function Assert-MixedDpiReady {
    $probe = Invoke-Probe
    if ([string]$probe.result -ne 'READY') {
        throw "MIXED_DPI_BLOCKED: $([string]$probe.reason) Report=$probeReport"
    }
    return $probe
}
function Get-SealedGui {
    $state = Get-ReleaseState
    if (-not (Test-Path -LiteralPath $PackageZip -PathType Leaf)) {
        throw "Sealed package ZIP is missing: $PackageZip"
    }

    $file = Get-Item -LiteralPath $PackageZip
    $actualHash = (Get-FileHash -LiteralPath $PackageZip -Algorithm SHA256).Hash.ToUpperInvariant()
    $expectedHash = ([string]$state.package.sha256).ToUpperInvariant()
    $expectedBytes = [int64]$state.package.bytes

    if ($actualHash -ne $expectedHash) {
        throw "Package SHA-256 mismatch. expected=$expectedHash actual=$actualHash"
    }
    if ([int64]$file.Length -ne $expectedBytes) {
        throw "Package byte-size mismatch. expected=$expectedBytes actual=$($file.Length)"
    }

    $stage = Join-Path $env:TEMP ("SearchTool-MixedDpi-" + $expectedHash.Substring(0,12))
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Expand-Archive -LiteralPath $PackageZip -DestinationPath $stage -Force

    $gui = Join-Path $stage 'search-tool-gui.exe'
    if (-not (Test-Path -LiteralPath $gui -PathType Leaf)) {
        throw 'search-tool-gui.exe is missing from the sealed package.'
    }

    return [pscustomobject]@{
        state = $state
        gui = $gui
        package_sha256 = $actualHash
        package_bytes = [int64]$file.Length
        stage = $stage
    }
}

function Assert-NoExistingGui {
    $existing = @(Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue)
    if ($existing.Count -gt 0) {
        $ids = ($existing | ForEach-Object { $_.Id }) -join ','
        throw "Close existing search-tool-gui processes before final validation. PIDs=$ids"
    }
}

function Wait-GuiWindow {
    param([Parameter(Mandatory)] [System.Diagnostics.Process]$Process)

    $deadline = (Get-Date).AddSeconds(15)
    do {
        $Process.Refresh()
        if ($Process.HasExited) {
            throw "GUI exited before its window became available. pid=$($Process.Id)"
        }
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
        Start-Sleep -Milliseconds 200
    } while ((Get-Date) -lt $deadline)

    throw "Timed out waiting for GUI window. pid=$($Process.Id)"
}

if (-not ('SearchToolFinalizerWindow' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class SearchToolFinalizerWindow {
    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
}
'@
}

function Test-RectIntersectsMonitor {
    param(
        [Parameter(Mandatory)] [object]$Window,
        [Parameter(Mandatory)] [object]$Monitor
    )
    return (
        [int]$Window.right -gt [int]$Monitor.left -and
        [int]$Window.left -lt [int]$Monitor.right -and
        [int]$Window.bottom -gt [int]$Monitor.top -and
        [int]$Window.top -lt [int]$Monitor.bottom
    )
}

function Stop-ValidatedGui {
    param([Parameter(Mandatory)] [string]$ReportPath)
    $report = Read-JsonFile $ReportPath
    if ([string]$report.result -ne 'PASS') {
        return
    }

    $pidValue = 0
    if ($null -ne $report.PSObject.Properties['observed_gui_pid']) {
        $pidValue = [int]$report.observed_gui_pid
    } elseif ($null -ne $report.PSObject.Properties['gui_pid']) {
        $pidValue = [int]$report.gui_pid
    }

    if ($pidValue -gt 0) {
        $proc = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
        if ($proc -and $proc.ProcessName -eq 'search-tool-gui') {
            Stop-Process -Id $pidValue -Force -ErrorAction Stop
            Write-Host "Stopped validated GUI pid=$pidValue"
        }
    }
}
function Prepare-MonitorRemoval {
    $probe = Assert-MixedDpiReady
    $context = $probe.desktop_context
    $sealed = Get-SealedGui
    Assert-NoExistingGui

    $primary = @($probe.monitors | Where-Object { $_.primary } | Select-Object -First 1)
    if ($primary.Count -ne 1) {
        throw 'Unable to identify exactly one primary monitor.'
    }

    $targets = @($probe.monitors | Where-Object {
        -not $_.primary -and [int]$_.dpi_x -ne [int]$primary[0].dpi_x
    })
    if ($targets.Count -eq 0) {
        $targets = @($probe.monitors | Where-Object { -not $_.primary })
    }
    if ($targets.Count -eq 0) {
        throw 'No non-primary monitor is available for the removal exercise.'
    }
    $target = $targets[0]

    $proc = Start-Process -FilePath $sealed.gui -PassThru
    $hwnd = Wait-GuiWindow -Process $proc

    $width = [Math]::Min(900, [Math]::Max(320, [int]$target.width - 80))
    $height = [Math]::Min(700, [Math]::Max(240, [int]$target.height - 80))
    $x = [int]$target.left + [Math]::Max(20, [int](([int]$target.width - $width) / 2))
    $y = [int]$target.top + [Math]::Max(20, [int](([int]$target.height - $height) / 2))

    $moved = [SearchToolFinalizerWindow]::SetWindowPos(
        $hwnd, [IntPtr]::Zero, $x, $y, $width, $height, 0x0014)
    if (-not $moved) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        throw "SetWindowPos failed while staging the GUI on $($target.device)."
    }
    Start-Sleep -Milliseconds 800

    Remove-Item -LiteralPath $removalState,$removalPrepareReport -Force -ErrorAction SilentlyContinue
    $prepareArgs = @{
        Mode = 'PrepareTopology'
        ExpectedTopologyChange = 'MonitorRemoved'
        RequireMixedDpi = $true
        StateFile = $removalState
        GuiPath = $sealed.gui
        OutputJson = $removalPrepareReport
        Enforce = $true
    }
    & $displayValidator @prepareArgs | Out-Host
    Bind-DesktopContextToReport -Path $removalState -Context $context | Out-Null
    Bind-DesktopContextToReport -Path $removalPrepareReport -Context $context | Out-Null

    $prepared = Read-JsonFile $removalPrepareReport
    if ([int]$prepared.gui_pid -ne $proc.Id) {
        throw 'Prepared GUI PID does not match the sealed GUI process.'
    }
    if (-not (Test-RectIntersectsMonitor -Window $prepared.window -Monitor $target)) {
        throw 'Prepared GUI is not on the monitor selected for physical removal.'
    }

    [ordered]@{
        schema = 1
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        result = 'PREPARED'
        package_sha256 = $sealed.package_sha256
        package_bytes = $sealed.package_bytes
        desktop_context = $context
        gui_pid = $proc.Id
        target_device = [string]$target.device
        target_dpi = [int]$target.dpi_x
        target_bounds = [ordered]@{
            left = [int]$target.left
            top = [int]$target.top
            right = [int]$target.right
            bottom = [int]$target.bottom
        }
        state_file = $removalState
        next_action = 'Physically disconnect target_device, keep the GUI process alive, then run -Mode VerifyMonitorRemoved.'
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalMeta -Encoding UTF8

    Write-Host "TARGET_DEVICE=$($target.device)"
    Write-Host "TARGET_DPI=$($target.dpi_x)"
    Write-Host 'NEXT=Physically disconnect that display, then run -Mode VerifyMonitorRemoved'
    Write-Host "Meta=$removalMeta"
}

function Build-Bundle {
    $state = Get-ReleaseState
    $exercise = Read-JsonFile $exerciseReport
    $primary = Read-JsonFile $primaryVerifyReport
    $removal = Read-JsonFile $removalVerifyReport
    $meta = Read-JsonFile $removalMeta

    $checks = [ordered]@{
        exercise_pass = ([string]$exercise.result -eq 'PASS')
        exercise_mixed_dpi = [bool]$exercise.mixed_dpi
        exercise_multiple_monitors = ([int]$exercise.monitor_count -ge 2)
        exercise_distinct_dpi = ([int]$exercise.distinct_dpi_count -ge 2)
        exercise_all_moves_pass = (@($exercise.moves | Where-Object {
            -not $_.move_api_ok -or -not $_.intersects_target -or -not $_.dpi_match
        }).Count -eq 0)
        exercise_context_interactive = (Test-InteractiveDesktopContextEvidence -Context $exercise.desktop_context)
        exercise_standard_devices = (Test-StandardDisplayDevices -Monitors @($exercise.monitors))
        primary_change_pass = ([string]$primary.result -eq 'PASS')
        primary_change_observed = (
            [string]$primary.expected_topology_change -eq 'PrimaryChanged' -and
            [bool]$primary.expected_change_observed
        )
        primary_gui_survived = [bool]$primary.gui_process_survived
        primary_window_recovered = [bool]$primary.window_recovered_to_active_monitor
        primary_dpi_match = [bool]$primary.window_dpi_matches_active_monitor
        primary_context_interactive = (Test-InteractiveDesktopContextEvidence -Context $primary.desktop_context)
        primary_standard_devices = (Test-StandardDisplayDevices -Monitors @($primary.monitors))
        monitor_removal_pass = ([string]$removal.result -eq 'PASS')
        monitor_removal_observed = (
            [string]$removal.expected_topology_change -eq 'MonitorRemoved' -and
            [bool]$removal.expected_change_observed
        )
        removal_window_was_on_removed_monitor = [bool]$removal.window_was_on_removed_monitor
        removal_gui_survived = [bool]$removal.gui_process_survived
        removal_window_recovered = [bool]$removal.window_recovered_to_active_monitor
        removal_dpi_match = [bool]$removal.window_dpi_matches_active_monitor
        removal_context_interactive = (Test-InteractiveDesktopContextEvidence -Context $removal.desktop_context)
        removal_standard_devices = (Test-StandardDisplayDevices -Monitors @($removal.monitors))
        removal_prepare_context_interactive = (Test-InteractiveDesktopContextEvidence -Context $meta.desktop_context)
        removal_target_recorded = (-not [string]::IsNullOrWhiteSpace([string]$meta.target_device))
        removal_target_standard_device = (Test-StandardDisplayDeviceName -Device ([string]$meta.target_device))
    }
    $passed = -not ($checks.Values -contains $false)

    $head = (& git -C $root rev-parse HEAD).Trim()
    $bundle = [ordered]@{
        schema = 1
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        result = if ($passed) { 'PASS' } else { 'FAIL' }
        gate = 'mixed_dpi'
        current_head_sha = $head
        packaged_source_sha = [string]$state.package.packaged_source_sha
        package_sha256 = ([string]$state.package.sha256).ToUpperInvariant()
        package_bytes = [int64]$state.package.bytes
        checks = $checks
        evidence = [ordered]@{
            exercise = $exerciseReport
            primary_change = $primaryVerifyReport
            monitor_removal = $removalVerifyReport
            monitor_removal_meta = $removalMeta
        }
        evidence_sha256 = [ordered]@{
            exercise = (Get-FileHash -LiteralPath $exerciseReport -Algorithm SHA256).Hash
            primary_change = (Get-FileHash -LiteralPath $primaryVerifyReport -Algorithm SHA256).Hash
            monitor_removal = (Get-FileHash -LiteralPath $removalVerifyReport -Algorithm SHA256).Hash
            monitor_removal_meta = (Get-FileHash -LiteralPath $removalMeta -Algorithm SHA256).Hash
        }
    }
    $bundle | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $bundleReport -Encoding UTF8
    $bundle | ConvertTo-Json -Depth 8
    Write-Host "Bundle=$bundleReport"
    if (-not $passed) {
        throw 'Mixed-DPI final bundle is not PASS.'
    }
}
function Invoke-SelfTest {
    $nonInteractiveContext = [pscustomobject]@{
        user_interactive = $false
        session_id = 3
        user_name = 'synthetic'
        session_name = 'Console'
    }
    $sessionZeroContext = [pscustomobject]@{
        user_interactive = $true
        session_id = 0
        user_name = 'synthetic'
        session_name = 'Services'
    }
    $interactiveContext = [pscustomobject]@{
        user_interactive = $true
        session_id = 2
        user_name = 'synthetic'
        session_name = 'Console'
    }

    $nonInteractiveReason = Get-DesktopContextBlockReason -Context $nonInteractiveContext
    if ([string]::IsNullOrWhiteSpace([string]$nonInteractiveReason)) {
        throw 'Mixed-DPI finalizer self-test failed to reject non-interactive desktop context.'
    }

    $sessionZeroReason = Get-DesktopContextBlockReason -Context $sessionZeroContext
    if ([string]::IsNullOrWhiteSpace([string]$sessionZeroReason)) {
        throw 'Mixed-DPI finalizer self-test failed to reject Session 0 desktop context.'
    }

    $interactiveReason = Get-DesktopContextBlockReason -Context $interactiveContext
    if (-not [string]::IsNullOrWhiteSpace([string]$interactiveReason)) {
        throw "Mixed-DPI finalizer self-test rejected a valid interactive desktop context: $interactiveReason"
    }

    $standardDevices = @(
        [pscustomobject]@{ device = '\\.\DISPLAY1' },
        [pscustomobject]@{ device = '\\.\DISPLAY2' }
    )
    if (-not (Test-StandardDisplayDevices -Monitors $standardDevices)) {
        throw 'Mixed-DPI finalizer self-test rejected standard Windows display devices.'
    }

    $sessionDevice = @([pscustomobject]@{ device = 'WinDisc' })
    if (Test-StandardDisplayDevices -Monitors $sessionDevice) {
        throw 'Mixed-DPI finalizer self-test accepted a session display device.'
    }

    $prefixSpoofDevice = @([pscustomobject]@{ device = '\\.\DISPLAY1VIRTUAL' })
    if (Test-StandardDisplayDevices -Monitors $prefixSpoofDevice) {
        throw 'Mixed-DPI finalizer self-test accepted a prefix-spoof display device.'
    }

    $exercisePayload = [ordered]@{
        schema = 1
        result = 'PASS'
        mixed_dpi = $true
        monitor_count = 2
        distinct_dpi_count = 2
        desktop_context = $interactiveContext
        monitors = $standardDevices
        moves = @(
            [ordered]@{
                move_api_ok = $true
                intersects_target = $true
                dpi_match = $true
            },
            [ordered]@{
                move_api_ok = $true
                intersects_target = $true
                dpi_match = $true
            }
        )
    }
    $primaryPayload = [ordered]@{
        schema = 1
        result = 'PASS'
        desktop_context = $interactiveContext
        monitors = $standardDevices
        expected_topology_change = 'PrimaryChanged'
        expected_change_observed = $true
        gui_process_survived = $true
        window_recovered_to_active_monitor = $true
        window_dpi_matches_active_monitor = $true
    }
    $removalPayload = [ordered]@{
        schema = 1
        result = 'PASS'
        desktop_context = $interactiveContext
        monitors = @([pscustomobject]@{ device = '\\.\DISPLAY1' })
        expected_topology_change = 'MonitorRemoved'
        expected_change_observed = $true
        window_was_on_removed_monitor = $true
        gui_process_survived = $true
        window_recovered_to_active_monitor = $true
        window_dpi_matches_active_monitor = $true
    }
    $metaPayload = [ordered]@{
        schema = 1
        result = 'PREPARED'
        desktop_context = $interactiveContext
        target_device = '\\.\DISPLAY2'
    }

    $exercisePayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $exerciseReport -Encoding UTF8
    $primaryPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $primaryVerifyReport -Encoding UTF8
    $removalPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalVerifyReport -Encoding UTF8
    $metaPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalMeta -Encoding UTF8

    Build-Bundle | Out-Host
    $positive = Read-JsonFile $bundleReport
    if ([string]$positive.result -ne 'PASS') {
        throw 'Mixed-DPI finalizer self-test positive control did not PASS.'
    }

    $removalPayload.window_was_on_removed_monitor = $false
    $removalPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalVerifyReport -Encoding UTF8
    $negativeRejected = $false
    try {
        Build-Bundle | Out-Null
    } catch {
        $negativeRejected = $true
    }
    if (-not $negativeRejected) {
        throw 'Mixed-DPI finalizer self-test failed to reject invalid removal provenance.'
    }

    $removalPayload.window_was_on_removed_monitor = $true
    $removalPayload.desktop_context = $nonInteractiveContext
    $removalPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalVerifyReport -Encoding UTF8
    $contextRejected = $false
    try {
        Build-Bundle | Out-Null
    } catch {
        $contextRejected = $true
    }
    if (-not $contextRejected) {
        throw 'Mixed-DPI finalizer self-test failed to reject non-interactive context embedded in evidence.'
    }

    $removalPayload.desktop_context = $interactiveContext
    $removalPayload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $removalVerifyReport -Encoding UTF8
    Build-Bundle | Out-Null

    [ordered]@{
        schema = 1
        result = 'PASS'
        host_dpi_awareness = 'PER_MONITOR_AWARE'
        non_interactive_context_rejected = $true
        session_zero_context_rejected = $true
        valid_interactive_context_accepted = $true
        standard_display_devices_accepted = $true
        session_display_device_rejected = $true
        prefix_spoof_display_device_rejected = $true
        bundle_non_interactive_context_rejected = $true
        positive_bundle_pass = $true
        invalid_removal_provenance_rejected = $true
        final_bundle_restored_to_pass = $true
        bundle = $bundleReport
    } | ConvertTo-Json -Depth 4
}

Get-ReleaseState | Out-Null

switch ($Mode) {
    'Probe' {
        $probe = Invoke-Probe
        if ([string]$probe.result -eq 'READY') {
            Write-Host 'NEXT=Run -Mode Exercise with the exact sealed package ZIP available.'
        } else {
            Write-Host 'NEXT=Attach at least two active displays with distinct effective DPI.'
        }
    }

    'Exercise' {
        $probe = Assert-MixedDpiReady
        $context = $probe.desktop_context
        $sealed = Get-SealedGui
        Assert-NoExistingGui
        Remove-Item -LiteralPath $exerciseReport -Force -ErrorAction SilentlyContinue
        $args = @{
            Mode = 'Exercise'
            RequireMixedDpi = $true
            GuiPath = $sealed.gui
            OutputJson = $exerciseReport
            Enforce = $true
        }
        & $displayValidator @args | Out-Host
        Bind-DesktopContextToReport -Path $exerciseReport -Context $context | Out-Null
        Write-Host "Exercise=$exerciseReport"
        Write-Host 'NEXT=Run -Mode PreparePrimaryChanged, change the Windows primary display, then run -Mode VerifyPrimaryChanged.'
    }

    'PreparePrimaryChanged' {
        $probe = Assert-MixedDpiReady
        $context = $probe.desktop_context
        $sealed = Get-SealedGui
        Assert-NoExistingGui
        Remove-Item -LiteralPath $primaryState,$primaryPrepareReport -Force -ErrorAction SilentlyContinue
        $args = @{
            Mode = 'PrepareTopology'
            ExpectedTopologyChange = 'PrimaryChanged'
            RequireMixedDpi = $true
            StateFile = $primaryState
            GuiPath = $sealed.gui
            OutputJson = $primaryPrepareReport
            Enforce = $true
        }
        & $displayValidator @args | Out-Host
        Bind-DesktopContextToReport -Path $primaryState -Context $context | Out-Null
        Bind-DesktopContextToReport -Path $primaryPrepareReport -Context $context | Out-Null
        Write-Host "State=$primaryState"
        Write-Host 'NEXT=Change which active display is Windows primary without closing the GUI, then run -Mode VerifyPrimaryChanged.'
    }

    'VerifyPrimaryChanged' {
        $context = Assert-InteractiveDesktopContext
        Remove-Item -LiteralPath $primaryVerifyReport -Force -ErrorAction SilentlyContinue
        $args = @{
            Mode = 'VerifyTopology'
            ExpectedTopologyChange = 'PrimaryChanged'
            RequireMixedDpi = $true
            StateFile = $primaryState
            OutputJson = $primaryVerifyReport
            Enforce = $true
        }
        & $displayValidator @args | Out-Host
        Bind-DesktopContextToReport -Path $primaryVerifyReport -Context $context | Out-Null
        Stop-ValidatedGui -ReportPath $primaryVerifyReport
        Write-Host "PrimaryChange=$primaryVerifyReport"
        Write-Host 'NEXT=Run -Mode PrepareMonitorRemoved.'
    }

    'PrepareMonitorRemoved' {
        Prepare-MonitorRemoval
    }

    'VerifyMonitorRemoved' {
        $context = Assert-InteractiveDesktopContext
        Remove-Item -LiteralPath $removalVerifyReport -Force -ErrorAction SilentlyContinue
        $args = @{
            Mode = 'VerifyTopology'
            ExpectedTopologyChange = 'MonitorRemoved'
            RequireMixedDpi = $true
            StateFile = $removalState
            OutputJson = $removalVerifyReport
            Enforce = $true
        }
        & $displayValidator @args | Out-Host
        Bind-DesktopContextToReport -Path $removalVerifyReport -Context $context | Out-Null
        Stop-ValidatedGui -ReportPath $removalVerifyReport
        Write-Host "MonitorRemoval=$removalVerifyReport"
        Write-Host 'NEXT=Run -Mode Bundle.'
    }

    'Bundle' {
        Build-Bundle
    }

    'SelfTest' {
        Invoke-SelfTest
    }
}
