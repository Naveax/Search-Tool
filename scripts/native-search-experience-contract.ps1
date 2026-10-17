# Read-only, repository-local contract for the no-companion-UI release mode.
# DOES NOT install extensions, modify registry/services, open Search, or send input.
[CmdletBinding()]
param(
    [switch]$SelfTest,
    [string]$RepositoryRoot = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}

function Test-NoNativeEntrypointTakeover([string]$Source) {
    # Parse command AST rather than case-sensitive source substrings.
    # Never execute the installer. Dynamic/evasive changes still require
    # separate code review, isolated installation and physical acceptance.
    $tokens = $null
    $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseInput(
        $Source, [ref]$tokens, [ref]$parseErrors)
    if (@($parseErrors).Count -gt 0 -or $null -eq $ast) { return $false }
    $mutations = '^(?:New-Item|Set-Item|New-ItemProperty|Set-ItemProperty|Remove-Item|Remove-ItemProperty|Rename-Item|Move-Item|Copy-Item|reg(?:\.exe)?)$'
    $protectedShell = '(?i)(?:(?:HKLM|HKCU):\\(?:SOFTWARE\\)?Classes\\search(?:-ms)?|HKCR:\\search(?:-ms)?)(?=\\|[''"]|\s|$)'
    $startupRun = '(?i)(?:HKLM|HKCU):\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run(?:Once)?(?=\\|[''"]|\s|$)'
    $commands = $ast.FindAll({
        param($n) $n -is [System.Management.Automation.Language.CommandAst]
    }, $true)
    foreach ($command in $commands) {
        $name = [string]$command.GetCommandName()
        if ($name -match '(?i)^(?:Register-ScheduledTask|Set-ScheduledTask|New-ScheduledTask|schtasks(?:\.exe)?)$') {
            return $false
        }
        if ($name -match "(?i)$mutations" -and
            ($command.Extent.Text -match $protectedShell -or
             $command.Extent.Text -match $startupRun)) {
            # The installer intentionally removes ONLY our historic
            # SearchTool.Search OpenWithProgids value. It must never
            # delete or replace the Windows-owned search/search-ms class.
            $legacyOwnedOpenWithRemoval =
                $name -ieq 'Remove-ItemProperty' -and
                $command.Extent.Text -match '(?i)Classes\\search\\OpenWithProgids' -and
                $command.Extent.Text -match '(?i)-Name\s+[''"]?SearchTool\.Search[''"]?(?=\s|$)' -and
                $command.Extent.Text -notmatch '(?i)search-ms'
            if (-not $legacyOwnedOpenWithRemoval) { return $false }
        }
        # A separately visible GUI shortcut cannot be created by the
        # default installer, including when its .Save() call is renamed.
        if ($name -match '(?i)^(?:New-Shortcut|Create-Shortcut)$') {
            return $false
        }
    }
    $methods = $ast.FindAll({
        param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst]
    }, $true)
    foreach ($method in $methods) {
        if ($method.Member.Extent.Text -match "(?i)^['\x22]?(?:CreateShortcut|RegisterHotKey)['\x22]?$") {
            return $false
        }
    }
    return $true
}

function Test-NativeOnlyInstaller([string]$Source) {
    $requiredRemoval = $Source.Contains('Remove-Item -LiteralPath (Join-Path $startup ''Search Tool.lnk'')')
    $legacyAutoStart = $Source.Contains('$shortcut.Save()') -or
        $Source.Contains('--resident --no-shell-bridge') -or
        $Source.Contains('--resident --shell-bridge')
    $startMenuAbsent = $Source.Contains('Remove-Item -LiteralPath (Join-Path $programs ''Search Tool.lnk'')')
    # A private, explicitly invoked searchtool: URI is permitted. A
    # competing default search: registration or keyboard takeover is not.
    $shellTakeover = $Source.Contains("New-Item -Path 'HKLM:\SOFTWARE\Classes\search'") -or
        $Source.Contains("Set-Item -Path 'HKLM:\SOFTWARE\Classes\search'") -or
        $Source.Contains("Set-ItemProperty -Path 'HKLM:\SOFTWARE\Classes\search'")
    return ($requiredRemoval -and -not $legacyAutoStart -and
        $startMenuAbsent -and -not $shellTakeover -and
        (Test-NoNativeEntrypointTakeover $Source))
}

function Get-NativeExperienceStatus([bool]$NoCompanionAutoStart,
                                     [bool]$SupportedProviderValidated,
                                     [bool]$PhysicalInteractionAccepted) {
    if (-not $NoCompanionAutoStart) {
        return [ordered]@{result='BLOCKED'; reason='COMPANION_GUI_DEFAULT'}
    }
    if (-not $SupportedProviderValidated) {
        return [ordered]@{result='BLOCKED'; reason='NATIVE_SEARCH_RESULT_INTEGRATION_UNVERIFIED'}
    }
    if (-not $PhysicalInteractionAccepted) {
        return [ordered]@{result='BLOCKED'; reason='PHYSICAL_WIN10_WIN11_INTERACTION_UNVERIFIED'}
    }
    return [ordered]@{result='PASS'; reason='ALL_EVIDENCE_PRESENT'}
}

if ($SelfTest) {
    $fixture = @'
function New-SearchToolShortcuts {
    $startup = 'x'
    Remove-Item -LiteralPath (Join-Path $startup 'Search Tool.lnk') -Force
    $programs = 'x'
    Remove-Item -LiteralPath (Join-Path $programs 'Search Tool.lnk') -Force
}
'@
    if (-not (Test-NativeOnlyInstaller $fixture)) { throw 'NATIVE_UX_REJECTED_SAFE_INSTALLER' }
    if (Test-NativeOnlyInstaller ($fixture + [Environment]::NewLine + '$shortcut.Save()')) {
        throw 'NATIVE_UX_ACCEPTED_AUTOSTART'
    }
    if (Test-NativeOnlyInstaller ($fixture + [Environment]::NewLine + 'Set-Item -Path ''HKLM:\SOFTWARE\Classes\search'' -Value hijack')) {
        throw 'NATIVE_UX_ACCEPTED_SEARCH_HANDLER_HIJACK'
    }
    # Adversarial regression controls: spelling, case, HKCU vs HKLM,
    # scheduled tasks and shortcut creation must not bypass protection.
    $unsafe = @(
        'nEw-ItEm -LiteralPath ''HKCU:\Software\Classes\search'' -Force'
        'SET-ITEMPROPERTY -Path ''HKLM:\SOFTWARE\Classes\search-ms'' -Name ''(Default)'' -Value hijack'
        'Remove-Item -LiteralPath ''HKLM:\SOFTWARE\Classes\search'' -Recurse -Force'
        'Remove-ItemProperty -Path ''HKCU:\SOFTWARE\Classes\search'' -Name ''(Default)'''
        'Set-Item -Path ''HKCR:\search'' -Value malicious'
        'New-ItemProperty -Path ''HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run'' -Name SearchTool -Value gui'
        'Set-ItemProperty -Path ''HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce'' -Name SearchTool -Value gui'
        'Register-ScheduledTask -TaskName SearchAutoStart -Action $action'
        'SCHTASKS.EXE /Create /TN SearchAutoStart /TR search-tool-gui.exe'
        '$shell.CreateShortcut(''Search Tool.lnk'')'
    )
    foreach ($snippet in $unsafe) {
        if (Test-NativeOnlyInstaller ($fixture + [Environment]::NewLine + $snippet)) {
            throw ('NATIVE_UX_AST_GUARD_ACCEPTED_' + ($unsafe.IndexOf($snippet)))
        }
    }
    $legitimate = @(
        'Remove-ItemProperty -Path ''HKLM:\SOFTWARE\Classes\search\OpenWithProgids'' -Name SearchTool.Search'
        'New-Item -Path ''HKLM:\SOFTWARE\Classes\searchtool'' -Force'
        '# Register-ScheduledTask and schtasks.exe are only mentioned in this comment'
    )
    foreach ($snippet in $legitimate) {
        if (-not (Test-NativeOnlyInstaller ($fixture + [Environment]::NewLine + $snippet))) {
            throw ('NATIVE_UX_AST_GUARD_REJECTED_ALLOWED_' + ($legitimate.IndexOf($snippet)))
        }
    }
    $missing = Get-NativeExperienceStatus $true $false $false
    if ($missing.result -cne 'BLOCKED' -or $missing.reason -cne 'NATIVE_SEARCH_RESULT_INTEGRATION_UNVERIFIED') {
        throw 'NATIVE_UX_FALSE_PROVIDER_ACCEPTANCE'
    }
    $missing = Get-NativeExperienceStatus $true $true $false
    if ($missing.result -cne 'BLOCKED' -or $missing.reason -cne 'PHYSICAL_WIN10_WIN11_INTERACTION_UNVERIFIED') {
        throw 'NATIVE_UX_FALSE_PHYSICAL_ACCEPTANCE'
    }
    if ((Get-NativeExperienceStatus $true $true $true).result -cne 'PASS') {
        throw 'NATIVE_UX_VALID_EVIDENCE_REJECTED'
    }
    'NATIVE_SEARCH_EXPERIENCE_CONTRACT_SELFTEST_PASS'
    return
}

$installerPath = Join-Path $RepositoryRoot 'scripts\install.ps1'
if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
    throw "Installer not found: $installerPath"
}
$installer = [IO.File]::ReadAllText($installerPath)
$installerSafe = Test-NativeOnlyInstaller $installer
if (-not $installerSafe) {
    throw 'NATIVE_UX_DEFAULT_INSTALLER_POLICY_FAILED'
}
# Critically: code inspection or a successful synthetic benchmark never
# establishes that native Win+S/Start/Explorer uses Search Tool's ranking.
# This status intentionally remains BLOCKED until separately approved,
# evidenced supported-native-provider and physical Win10/Win11 testing.
$status = Get-NativeExperienceStatus $installerSafe $false $false
[ordered]@{
    schema = 1
    result = $status.result
    default_companion_gui_autostart = $false
    native_search_result_integration = 'UNVERIFIED'
    physical_win10_win11_acceptance = 'UNVERIFIED'
    release_eligible = $false
    reason = $status.reason
    source_inspection = 'PASS'
} | ConvertTo-Json -Compress
