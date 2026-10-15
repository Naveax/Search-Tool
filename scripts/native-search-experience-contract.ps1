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
        $startMenuAbsent -and -not $shellTakeover)
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
