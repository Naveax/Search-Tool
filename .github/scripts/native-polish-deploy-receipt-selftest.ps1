[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$gate = Join-Path $PSScriptRoot 'native-polish-deploy-receipt-check.ps1'
$temp = Join-Path ([IO.Path]::GetTempPath()) ('SearchToolReceiptGate-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp -Force | Out-Null
$zip = 'AFA884E6133E16E371820DA668E426BB80266D7E11A96553AEF77603131834FC'
$bins = [ordered]@{
    'search-tool.exe' = '4276C825C49A013CF526A1F5A204BE42479C84EADD849F90305CEAE41E4DB59D'
    'search-tool-gui.exe' = 'EC8926E44185EB3F4E0A84E1A07C6864CDDC18C4969479FB34026EE33013131F'
    'search-tool-service.exe' = 'F2F045EBBA70E73FDA71E5D9838EA2A3C0F59C38F5E60EF0A8D0EFBFD867DFBB'
    'search-tool-worker.exe' = 'BA762363D9B4101603C41FA6195E70E8796C5787514A920D9A952632085F6BC7'
}
function New-BinaryEntries([bool]$WithMatch) {
    $out = [ordered]@{}
    foreach($n in $bins.Keys) {
        if ($WithMatch) { $out[$n] = [ordered]@{sha256=$bins[$n];match=$true} }
        else { $out[$n] = [ordered]@{sha256=$bins[$n];bytes=1} }
    }
    return $out
}
$idxBefore = [ordered]@{path='C:\ProgramData\SearchTool\index\C.stidx';sha256=('A'*64)}
$idxAfter = [ordered]@{path='C:\ProgramData\SearchTool\index\C.stidx';sha256=('B'*64)}
$svc = [ordered]@{state='Running';start_mode='Auto';path='"C:\Program Files\Search Tool\search-tool-service.exe" --service-name "SearchToolIndexer"'}
$baseDeploy = [ordered]@{
    schema=1;host='DESKTOP-ONDD84S';ok=$true;
    started_utc='2026-10-08T08:00:00Z';completed_utc='2026-10-08T08:00:20Z';
    package_sha256=$zip;package_sha256_verified=$true;
    packaged_binaries=(New-BinaryEntries $false);
    installed_binaries=(New-BinaryEntries $false);
    installed_binary_hashes_match_package=$true;
    service_after=$svc;index_before=$idxBefore;index_after=$idxAfter;index_verify_status='ok'
}
$basePost = [ordered]@{
    schema=1;host='DESKTOP-ONDD84S';result='PASS';timestamp_utc='2026-10-08T08:00:30Z';
    deployment=[ordered]@{
        package_sha256_verified=$true;
        installed_binaries=(New-BinaryEntries $true);
        installed_binary_hashes_match_package=$true;
        service_after=$svc;
        index_continuity=[ordered]@{
            existing_index_preserved=$true;before=$idxBefore;after=$idxAfter;verify_status='ok';verify_exit=0
        };
        resident_gui=[ordered]@{
            running=$true;pid=12345;sha256=$bins['search-tool-gui.exe'];binary_sha256_matches_package=$true
        }
    }
}
$deployPath = Join-Path $temp 'deploy-result.json'
$postPath = Join-Path $temp 'post-deploy-verify.json'
function Write-Receipts($d,$p) {
    $d|ConvertTo-Json -Depth 20|Set-Content -LiteralPath $deployPath -Encoding UTF8
    $p|ConvertTo-Json -Depth 20|Set-Content -LiteralPath $postPath -Encoding UTF8
}
function Deep-Copy($obj){ return ($obj|ConvertTo-Json -Depth 20|ConvertFrom-Json) }
$count=0
function Expect-Failure([string]$Label,[string]$Fragment,$d,$p) {
    Write-Receipts $d $p
    $failed=$false
    try{& $gate -DeployResult $deployPath -PostResult $postPath|Out-Null}
    catch{
        $failed=$true
        if(-not $_.Exception.Message.Contains($Fragment)){throw "Unexpected failure for $Label : $($_.Exception.Message)"}
    }
    if(-not $failed){throw "Rejected fixture unexpectedly accepted: $Label"}
    $script:count++
}
try {
    Write-Receipts $baseDeploy $basePost
    $pass=& $gate -DeployResult $deployPath -PostResult $postPath|ConvertFrom-Json
    if([string]$pass.result -ne 'PASS' -or [int]$pass.binary_hashes_verified -ne 4 -or [string]$pass.scope -ne 'RECEIPTS_ONLY_NOT_LIVE_INSTALLATION'){throw 'Positive synthetic receipt fixture rejected'}
    $count++
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$d.ok=$false
    Expect-Failure 'deploy_not_pass' 'elevated deployment is not PASS' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.result='PASSFAKE'
    Expect-Failure 'post_not_pass' 'post-deployment is not PASS' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$d.package_sha256=('F'*64)
    Expect-Failure 'package_mismatch' 'deployment package hash mismatch' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.deployment.installed_binaries.'search-tool-gui.exe'.sha256=('F'*64)
    Expect-Failure 'gui_installed_mismatch' 'post-deployment installed search-tool-gui.exe hash mismatch' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.deployment.index_continuity.verify_status='fail'
    Expect-Failure 'index_verify_mismatch' 'post-deployment index verifier not ok' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.deployment.service_after.start_mode='Manual'
    Expect-Failure 'service_not_auto' 'post-deployment service not Auto' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.timestamp_utc='2026-10-08T07:59:59Z'
    Expect-Failure 'time_travel' 'receipt timestamps out of order' $d $p
    $d=Deep-Copy $baseDeploy;$p=Deep-Copy $basePost;$p.host='WRONGHOST'
    Expect-Failure 'host_mismatch' 'post-deployment host mismatch' $d $p
    Write-Receipts $baseDeploy $basePost
    Remove-Item -LiteralPath $postPath -Force
    $failed=$false
    try{& $gate -DeployResult $deployPath -PostResult $postPath|Out-Null}catch{if($_.Exception.Message -match 'post-deployment receipt missing'){$failed=$true}else{throw}}
    if(-not $failed){throw 'Missing post receipt was accepted'}
    $count++
    [ordered]@{schema=1;result='PASS';synthetic_tests=$count;negative_tests=($count-1);scope='SYNTHETIC_RECEIPTS_ONLY';live_install_validation='NOT_ATTEMPTED'}|ConvertTo-Json -Depth 4
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
