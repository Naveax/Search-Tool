[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$DeployResult,
    [Parameter(Mandatory)] [string]$PostResult,
    [string]$ExpectedHost = 'DESKTOP-ONDD84S'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$expectedPackage = 'AFA884E6133E16E371820DA668E426BB80266D7E11A96553AEF77603131834FC'
$expectedBinaries = [ordered]@{
    'search-tool.exe' = '4276C825C49A013CF526A1F5A204BE42479C84EADD849F90305CEAE41E4DB59D'
    'search-tool-gui.exe' = 'EC8926E44185EB3F4E0A84E1A07C6864CDDC18C4969479FB34026EE33013131F'
    'search-tool-service.exe' = 'F2F045EBBA70E73FDA71E5D9838EA2A3C0F59C38F5E60EF0A8D0EFBFD867DFBB'
    'search-tool-worker.exe' = 'BA762363D9B4101603C41FA6195E70E8796C5787514A920D9A952632085F6BC7'
}

function Require([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "DEPLOY_RECEIPT_FAIL: $Message" }
}

function Require-Hash([object]$Value, [string]$Expected, [string]$Field) {
    Require ([string]$Value -ieq $Expected) "$Field hash mismatch"
}

function Read-Receipt([string]$Path, [string]$Label) {
    Require (Test-Path -LiteralPath $Path -PathType Leaf) "$Label receipt missing"
    try { return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json -ErrorAction Stop) }
    catch { throw "DEPLOY_RECEIPT_FAIL: $Label receipt is invalid JSON" }
}

function Read-BinarySha([object]$Container, [string]$Name, [string]$Label) {
    Require ($null -ne $Container) "$Label binary collection missing"
    $property = $Container.PSObject.Properties[$Name]
    Require ($null -ne $property) "$Label binary missing: $Name"
    $item = $property.Value
    Require ($null -ne $item -and $null -ne $item.PSObject.Properties['sha256']) "$Label binary SHA missing: $Name"
    return [string]$item.sha256
}

$deploy = Read-Receipt $DeployResult 'deployment'
$post = Read-Receipt $PostResult 'post-deployment'
Require ([int]$deploy.schema -eq 1 -and [int]$post.schema -eq 1) 'receipt schema mismatch'
Require ($null -ne $deploy.PSObject.Properties['installer_exit']) 'installer exit status missing'
Require ([int]$deploy.installer_exit -eq 0) 'installer exit status not zero'
Require ([bool]$deploy.ok) 'elevated deployment is not PASS'
Require ([string]$post.result -ceq 'PASS') 'post-deployment is not PASS'
Require ([string]$deploy.host -ceq $ExpectedHost) 'deployment host mismatch'
Require ([string]$post.host -ceq $ExpectedHost) 'post-deployment host mismatch'
Require-Hash $deploy.package_sha256 $expectedPackage 'deployment package'
Require ([bool]$deploy.package_sha256_verified) 'deployment package not SHA-verified'
Require ([bool]$post.deployment.package_sha256_verified) 'post-deployment package not SHA-verified'
Require ([bool]$deploy.installed_binary_hashes_match_package) 'deployment binary parity not verified'
Require ([bool]$post.deployment.installed_binary_hashes_match_package) 'post-deployment binary parity not verified'

foreach ($name in $expectedBinaries.Keys) {
    $sha = [string]$expectedBinaries[$name]
    Require-Hash (Read-BinarySha $deploy.packaged_binaries $name 'packaged') $sha "packaged $name"
    Require-Hash (Read-BinarySha $deploy.installed_binaries $name 'deployment installed') $sha "deployment installed $name"
    Require-Hash (Read-BinarySha $post.deployment.installed_binaries $name 'post-deployment installed') $sha "post-deployment installed $name"
    $postBinary = $post.deployment.installed_binaries.PSObject.Properties[$name].Value
    Require ([bool]$postBinary.match) "post-deployment binary match flag is false: $name"
}

Require ([string]$deploy.service_after.state -eq 'Running') 'deployment service not Running'
Require ([string]$deploy.service_after.start_mode -eq 'Auto') 'deployment service not Auto'
Require ([string]$post.deployment.service_after.state -eq 'Running') 'post-deployment service not Running'
Require ([string]$post.deployment.service_after.start_mode -eq 'Auto') 'post-deployment service not Auto'
Require ([string]$deploy.service_after.path -ceq [string]$post.deployment.service_after.path) 'service executable changed between receipts'

Require ($null -ne $deploy.index_before -and $null -ne $deploy.index_after) 'deployment did not capture index continuity'
Require ([bool]$post.deployment.index_continuity.existing_index_preserved) 'index continuity not verified'
Require ([string]$deploy.index_verify_status -ceq 'ok') 'deployment index verifier not ok'
Require ([string]$post.deployment.index_continuity.verify_status -ceq 'ok') 'post-deployment index verifier not ok'
Require ([int]$post.deployment.index_continuity.verify_exit -eq 0) 'post-deployment index verifier exit not zero'
Require ([string]$deploy.index_before.path -ceq [string]$post.deployment.index_continuity.before.path) 'pre-deployment index path mismatch'
Require ([string]$deploy.index_after.path -ceq [string]$post.deployment.index_continuity.after.path) 'post-deployment index path mismatch'
$expectedIndex = 'C:\ProgramData\SearchTool\index\C.stidx'
Require ([string]$deploy.index_before.path -ieq $expectedIndex -and [string]$deploy.index_after.path -ieq $expectedIndex) 'unexpected index path'
foreach ($fingerprint in @($deploy.index_before, $deploy.index_after, $post.deployment.index_continuity.before, $post.deployment.index_continuity.after)) {
    Require ($null -ne $fingerprint) 'index fingerprint missing'
    Require ([string]$fingerprint.sha256 -match '^[0-9a-fA-F]{64}$') 'index fingerprint SHA invalid'
    Require ([int64]$fingerprint.bytes -gt 0) 'index fingerprint byte size invalid'
}
Require ([bool]$deploy.index_exact_hash_preserved -eq [bool]$post.deployment.index_continuity.exact_base_hash_preserved) 'index exact-preservation flags disagree'
Require-Hash $post.deployment.index_continuity.before.sha256 ([string]$deploy.index_before.sha256) 'pre-deployment index receipt'
Require-Hash $post.deployment.index_continuity.after.sha256 ([string]$deploy.index_after.sha256) 'post-deployment index receipt'
Require ([bool]$post.deployment.resident_gui.running) 'resident GUI not running'
Require ([bool]$post.deployment.resident_gui.binary_sha256_matches_package) 'resident GUI parity not verified'
Require-Hash $post.deployment.resident_gui.sha256 ([string]$expectedBinaries['search-tool-gui.exe']) 'resident GUI'
Require ([int]$post.deployment.resident_gui.pid -gt 0) 'resident GUI PID invalid'
Require ([string]$post.deployment.resident_gui.path -ieq 'C:\Program Files\Search Tool\search-tool-gui.exe') 'resident GUI executable path mismatch'
Require (@($deploy.index_verify_output | Where-Object { [string]$_ -ceq 'status=ok' }).Count -eq 1) 'deployment index output missing status=ok'
Require (@($post.deployment.index_continuity.verify_output | Where-Object { [string]$_ -ceq 'status=ok' }).Count -eq 1) 'post-deployment index output missing status=ok'

$start = [DateTimeOffset]::MinValue
$finish = [DateTimeOffset]::MinValue
$postTime = [DateTimeOffset]::MinValue
Require ([DateTimeOffset]::TryParse([string]$deploy.started_utc, [ref]$start)) 'deployment start timestamp invalid'
Require ([DateTimeOffset]::TryParse([string]$deploy.completed_utc, [ref]$finish)) 'deployment completion timestamp invalid'
Require ([DateTimeOffset]::TryParse([string]$post.timestamp_utc, [ref]$postTime)) 'post-deployment timestamp invalid'
Require ($start -le $finish -and $finish -le $postTime) 'receipt timestamps out of order'

[ordered]@{
    schema = 1
    result = 'PASS'
    scope = 'RECEIPTS_ONLY_NOT_LIVE_INSTALLATION'
    host = $ExpectedHost
    package_sha256 = $expectedPackage
    binary_hashes_verified = $expectedBinaries.Count
    service = 'Running/Auto (receipt)'
    index = 'ok (receipt)'
    gui = 'PASS (receipt)'
} | ConvertTo-Json -Depth 4
