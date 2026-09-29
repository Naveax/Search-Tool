[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Artifact,
    [ValidateSet('NotObserved','Warned','Blocked','Allowed')] [string]$ObservedOutcome = 'NotObserved',
    [switch]$RequireEnabled,
    [switch]$RequireMotw,
    [switch]$Enforce,
    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$resolved = (Resolve-Path -LiteralPath $Artifact).Path

function Read-RegValue([string]$Path, [string]$Name) {
    try {
        $item = Get-ItemProperty -LiteralPath $Path -Name $Name -ErrorAction Stop
        return $item.$Name
    } catch {
        return $null
    }
}

function Read-ZoneIdentifier([string]$Path) {
    try {
        $raw = (Get-Content -LiteralPath $Path -Stream Zone.Identifier -Raw -ErrorAction Stop)
        $zone = $null
        if ($raw -match '(?im)^ZoneId\s*=\s*(\d+)\s*$') { $zone = [int]$Matches[1] }
        return [ordered]@{ present = $true; zone_id = $zone; raw = $raw.Trim() }
    } catch {
        return [ordered]@{ present = $false; zone_id = $null; raw = $null }
    }
}

$machineExplorer = Read-RegValue 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer' 'SmartScreenEnabled'
$userExplorer = Read-RegValue 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer' 'SmartScreenEnabled'
$policyEnabled = Read-RegValue 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' 'EnableSmartScreen'
$policyLevel = Read-RegValue 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' 'ShellSmartScreenLevel'

$enabled = $null
$source = 'unknown'
if ($null -ne $policyEnabled) {
    $enabled = ([int]$policyEnabled -ne 0)
    $source = 'policy'
} elseif ($machineExplorer -in @('Warn','RequireAdmin')) {
    $enabled = $true
    $source = 'machine-explorer'
} elseif ($machineExplorer -eq 'Off') {
    $enabled = $false
    $source = 'machine-explorer'
} elseif ($userExplorer -in @('Warn','RequireAdmin')) {
    $enabled = $true
    $source = 'user-explorer'
} elseif ($userExplorer -eq 'Off') {
    $enabled = $false
    $source = 'user-explorer'
}

$motw = Read-ZoneIdentifier $resolved
$signature = Get-AuthenticodeSignature -FilePath $resolved
$unsignedOrUntrusted = $signature.Status -ne 'Valid'
$motwInternet = $motw.present -and $null -ne $motw.zone_id -and [int]$motw.zone_id -ge 3
$observedProtective = $ObservedOutcome -in @('Warned','Blocked')

$requirements = [ordered]@{
    smartscreen_enabled = ($enabled -eq $true)
    motw_internet_zone = [bool]$motwInternet
    unsigned_or_untrusted_artifact = [bool]$unsignedOrUntrusted
    protective_outcome_observed = [bool]$observedProtective
}
$ready = (-not $RequireEnabled -or $requirements.smartscreen_enabled) -and
    (-not $RequireMotw -or $requirements.motw_internet_zone) -and
    $requirements.unsigned_or_untrusted_artifact
$result = if (-not $ready) {
    'BLOCKED'
} elseif ($ObservedOutcome -eq 'NotObserved') {
    'READY_FOR_INTERACTIVE_CHECK'
} elseif ($observedProtective) {
    'PASS'
} else {
    'FAIL'
}

$report = [ordered]@{
    schema = 1
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    computer_name = [Environment]::MachineName
    artifact = $resolved
    sha256 = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash
    signature_status = [string]$signature.Status
    signer_subject = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { $null }
    motw = $motw
    configuration = [ordered]@{
        effective_enabled = $enabled
        source = $source
        machine_explorer = $machineExplorer
        user_explorer = $userExplorer
        policy_enable_smartscreen = $policyEnabled
        policy_shell_level = $policyLevel
    }
    requirements = $requirements
    observed_outcome = $ObservedOutcome
    result = $result
}

$json = $report | ConvertTo-Json -Depth 7
$json
if ($OutputJson) {
    $parent = Split-Path -Parent $OutputJson
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json | Set-Content -LiteralPath $OutputJson -Encoding UTF8
    Write-Host "Report=$OutputJson"
}
if ($Enforce -and $result -ne 'PASS') {
    throw "SmartScreen validation did not produce PASS: result=$result enabled=$enabled motw=$motwInternet outcome=$ObservedOutcome"
}
