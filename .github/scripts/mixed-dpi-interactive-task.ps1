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
    [string]$OutputDir,
    [string]$InteractiveUser,
    [int]$TimeoutSeconds = 120
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$finalizer = Join-Path $root '.github\scripts\mixed-dpi-finalizer.ps1'
$releaseStatePath = Join-Path $root 'docs\RELEASE_STATE.json'

if ([string]::IsNullOrWhiteSpace($PackageZip)) {
    $PackageZip = Join-Path $root 'dist\SearchTool-Windows-x64.zip'
} elseif (-not [IO.Path]::IsPathRooted($PackageZip)) {
    $PackageZip = Join-Path $root $PackageZip
}
$PackageZip = [IO.Path]::GetFullPath($PackageZip)

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root 'dist\mixed-dpi-final-live'
} elseif (-not [IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $root $OutputDir
}
$OutputDir = [IO.Path]::GetFullPath($OutputDir)

function Assert-InteractiveTask {
    param(
        [Parameter(Mandatory)] [bool]$Condition,
        [Parameter(Mandatory)] [string]$Message
    )
    if (-not $Condition) {
        throw "MIXED_DPI_INTERACTIVE_TASK_FAIL: $Message"
    }
}

function Read-JsonFile {
    param([Parameter(Mandatory)] [string]$Path)
    Assert-InteractiveTask (Test-Path -LiteralPath $Path -PathType Leaf) "missing file: $Path"
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Get-ReleaseState {
    $state = Read-JsonFile $releaseStatePath
    Assert-InteractiveTask ([string]$state.package.status -eq 'VALIDATED') 'release package is not VALIDATED'
    $blockers = @($state.external_blockers)
    Assert-InteractiveTask ($blockers.Count -eq 1 -and [string]$blockers[0].name -eq 'mixed_dpi') 'mixed_dpi must be the sole unresolved external blocker'
    return $state
}

function Resolve-InteractiveAccount {
    if (-not [string]::IsNullOrWhiteSpace($InteractiveUser)) {
        return $InteractiveUser
    }

    $console = [string](Get-CimInstance Win32_ComputerSystem -ErrorAction Stop).UserName
    Assert-InteractiveTask (-not [string]::IsNullOrWhiteSpace($console)) 'no logged-in console user was detected'
    return $console
}

function Resolve-AccountSid {
    param([Parameter(Mandatory)] [string]$Account)
    $sid = (New-Object System.Security.Principal.NTAccount($Account)).Translate(
        [System.Security.Principal.SecurityIdentifier]
    ).Value
    Assert-InteractiveTask (-not [string]::IsNullOrWhiteSpace($sid)) "unable to resolve SID for $Account"
    return $sid
}

function Get-RunnerContent {
    param(
        [Parameter(Mandatory)] [string]$FinalizerMode,
        [Parameter(Mandatory)] [string]$RunnerOutputDir,
        [Parameter(Mandatory)] [string]$RunnerPackageZip,
        [Parameter(Mandatory)] [string]$DoneFile,
        [Parameter(Mandatory)] [string]$LogFile
    )

    $template = @'
$ErrorActionPreference = 'Stop'
$repo = '__ROOT__'
$finalizer = '__FINALIZER__'
$outDir = '__OUTDIR__'
$package = '__PACKAGE__'
$done = '__DONE__'
$log = '__LOG__'
$mode = '__MODE__'

try {
    Set-Location $repo
    & pwsh.exe -NoLogo -NoProfile -File $finalizer -Mode $mode -PackageZip $package -OutputDir $outDir *>&1 |
        Tee-Object -FilePath $log | Out-Host
    $code = $LASTEXITCODE
    [ordered]@{
        schema = 1
        result = if ($code -eq 0) { 'PASS' } else { 'FAIL' }
        exit_code = $code
        user = [Environment]::UserName
        user_interactive = [Environment]::UserInteractive
        session_id = [Diagnostics.Process]::GetCurrentProcess().SessionId
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $done -Encoding UTF8
    exit $code
} catch {
    [ordered]@{
        schema = 1
        result = 'ERROR'
        message = $_.Exception.Message
        user = [Environment]::UserName
        user_interactive = [Environment]::UserInteractive
        session_id = [Diagnostics.Process]::GetCurrentProcess().SessionId
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $done -Encoding UTF8
    exit 1
}
'@

    $replacements = @{
        '__ROOT__' = $root.Replace("'","''")
        '__FINALIZER__' = $finalizer.Replace("'","''")
        '__OUTDIR__' = $RunnerOutputDir.Replace("'","''")
        '__PACKAGE__' = $RunnerPackageZip.Replace("'","''")
        '__DONE__' = $DoneFile.Replace("'","''")
        '__LOG__' = $LogFile.Replace("'","''")
        '__MODE__' = $FinalizerMode.Replace("'","''")
    }

    foreach ($key in $replacements.Keys) {
        $template = $template.Replace($key, [string]$replacements[$key])
    }
    return $template
}

function Invoke-SelfTest {
    Assert-InteractiveTask (Test-Path -LiteralPath $finalizer -PathType Leaf) "finalizer missing: $finalizer"
    $state = Get-ReleaseState
    Assert-InteractiveTask ([string]$state.package.sha256 -match '^[0-9A-Fa-f]{64}$') 'release-state package SHA-256 is invalid'
    Assert-InteractiveTask ([int64]$state.package.bytes -gt 0) 'release-state package byte-size is invalid'

    $sampleDone = Join-Path $env:TEMP 'mixed-dpi-interactive-task-selftest-done.json'
    $sampleLog = Join-Path $env:TEMP 'mixed-dpi-interactive-task-selftest.log'
    $runner = Get-RunnerContent -FinalizerMode 'Probe' -RunnerOutputDir $OutputDir -RunnerPackageZip $PackageZip -DoneFile $sampleDone -LogFile $sampleLog

    foreach ($marker in @(
        [Environment]::UserInteractive,
        'session_id',
        'mixed-dpi-finalizer.ps1',
        '-Mode $mode',
        '-PackageZip $package',
        '-OutputDir $outDir'
    )) {
        if ($marker -is [bool]) {
            continue
        }
        Assert-InteractiveTask ($runner.Contains([string]$marker)) "generated runner is missing marker: $marker"
    }

    [ordered]@{
        schema = 1
        result = 'PASS'
        package_status = [string]$state.package.status
        sole_external_blocker = 'mixed_dpi'
        finalizer_present = $true
        runner_generation = 'PASS'
        task_logon_type = 'InteractiveToken'
        task_is_temporary = $true
    } | ConvertTo-Json -Depth 4
}

if ($Mode -eq 'SelfTest') {
    Invoke-SelfTest
    exit 0
}

Assert-InteractiveTask ($TimeoutSeconds -ge 10 -and $TimeoutSeconds -le 900) 'TimeoutSeconds must be between 10 and 900'
Get-ReleaseState | Out-Null
Assert-InteractiveTask (Test-Path -LiteralPath $finalizer -PathType Leaf) "finalizer missing: $finalizer"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$account = Resolve-InteractiveAccount
$sid = Resolve-AccountSid $account
$pwsh = (Get-Command pwsh.exe -ErrorAction Stop).Source
$token = [Guid]::NewGuid().ToString('N')
$taskName = "SearchTool-MixedDpi-$Mode-$token"
$runnerPath = Join-Path $OutputDir ".interactive-task-$token.ps1"
$donePath = Join-Path $OutputDir ".interactive-task-$token-done.json"
$logPath = Join-Path $OutputDir ".interactive-task-$token.log"

$runnerContent = Get-RunnerContent -FinalizerMode $Mode -RunnerOutputDir $OutputDir -RunnerPackageZip $PackageZip -DoneFile $donePath -LogFile $logPath
[IO.File]::WriteAllText($runnerPath, $runnerContent, [Text.UTF8Encoding]::new($false))

$service = New-Object -ComObject 'Schedule.Service'
$service.Connect()
$folder = $service.GetFolder('\')

try {
    try { $folder.DeleteTask($taskName, 0) } catch {}

    $task = $service.NewTask(0)
    $task.RegistrationInfo.Description = "Temporary Search-Tool interactive mixed-DPI $Mode task"
    $task.Settings.Enabled = $true
    $task.Settings.StartWhenAvailable = $true
    $task.Settings.DisallowStartIfOnBatteries = $false
    $task.Settings.StopIfGoingOnBatteries = $false
    $task.Settings.ExecutionTimeLimit = 'PT15M'
    $task.Principal.UserId = $sid
    $task.Principal.LogonType = 3
    $task.Principal.RunLevel = 1

    $trigger = $task.Triggers.Create(1)
    $trigger.StartBoundary = (Get-Date).AddMinutes(5).ToString('s')
    $trigger.Enabled = $true

    $action = $task.Actions.Create(0)
    $action.Path = $pwsh
    $action.Arguments = '-NoLogo -NoProfile -ExecutionPolicy Bypass -File "' + $runnerPath + '"'
    $action.WorkingDirectory = $root

    $registered = $folder.RegisterTaskDefinition($taskName, $task, 6, $null, $null, 3, $null)
    $null = $registered.Run($null)

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        Start-Sleep -Milliseconds 500
        if (Test-Path -LiteralPath $donePath) {
            break
        }
    } while ((Get-Date) -lt $deadline)

    Assert-InteractiveTask (Test-Path -LiteralPath $donePath -PathType Leaf) "interactive task did not finish within $TimeoutSeconds seconds"

    $done = Read-JsonFile $donePath
    Assert-InteractiveTask ([bool]$done.user_interactive) 'scheduled finalizer did not run in an interactive desktop'
    Assert-InteractiveTask ([int]$done.session_id -gt 0) "scheduled finalizer ran in invalid session: $($done.session_id)"

    $logText = if (Test-Path -LiteralPath $logPath -PathType Leaf) {
        Get-Content -LiteralPath $logPath -Raw
    } else {
        ''
    }

    [ordered]@{
        schema = 1
        result = if ([int]$done.exit_code -eq 0) { 'PASS' } else { 'FAIL' }
        finalizer_mode = $Mode
        account = $account
        sid = $sid
        user = [string]$done.user
        user_interactive = [bool]$done.user_interactive
        session_id = [int]$done.session_id
        exit_code = [int]$done.exit_code
        output_dir = $OutputDir
        log = $logText
    } | ConvertTo-Json -Depth 6

    if ([int]$done.exit_code -ne 0) {
        throw "interactive finalizer mode $Mode failed with exit code $($done.exit_code)"
    }
} finally {
    try { $folder.DeleteTask($taskName, 0) } catch {}
    Remove-Item -LiteralPath $runnerPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $donePath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $logPath -Force -ErrorAction SilentlyContinue
}
