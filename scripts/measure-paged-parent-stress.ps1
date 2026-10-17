# Synthetic scoped-search stress with Windows process counters and safeguards.
[CmdletBinding()]
param(
 [ValidateSet('2M','4M','20M')][string]$Size='4M',
 [int]$MaxWorkingSetMB=12288,
 [int]$MinFreeGB=80,
 [int]$TimeoutSeconds=1200,
 [int]$PollMilliseconds=1500,
 [switch]$SkipBuild
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$perf=Join-Path $root 'target\paged-parent-perf'
New-Item -ItemType Directory -Path $perf -Force | Out-Null
if($MaxWorkingSetMB -lt 1024 -or $MinFreeGB -lt 1 -or $TimeoutSeconds -lt 60 -or $PollMilliseconds -lt 250){throw 'Unsafe watchdog limits'}
if(-not $SkipBuild){
 Push-Location $root
 try {
  & cargo test -p search-core --release --no-run
  if($LASTEXITCODE -ne 0){throw "Rust test compilation exit $LASTEXITCODE"}
 } finally {Pop-Location}
}
$exe=Get-ChildItem (Join-Path $root 'target\release\deps') -Filter 'search_core-*.exe' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if($null -eq $exe){throw 'Release search-core test binary not found'}
$drive=[System.IO.DriveInfo]::new([System.IO.Path]::GetPathRoot($root))
if(($drive.AvailableFreeSpace/1GB) -lt $MinFreeGB){throw 'Insufficient available storage'}
$stamp=(Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$base=Join-Path $perf "paged-$Size-$stamp"
$stdout="$base.stdout.log"
$stderr="$base.stderr.log"
$metrics="$base.metrics.json"
$setting=switch($Size){
 '2M' {'SEARCH_TOOL_BENCH_OVER_2M'}
 '4M' {'SEARCH_TOOL_BENCH_OVER_4M'}
 '20M' {'SEARCH_TOOL_BENCH_20M'}
}
$previous=[Environment]::GetEnvironmentVariable($setting,'Process')
[Environment]::SetEnvironmentVariable($setting,'1','Process')
$start=Get-Date
$samples=[System.Collections.Generic.List[object]]::new()
$peakRss=[int64]0
$peakPrivate=[int64]0
$lastRead=$null
$lastWrite=$null
$lastCpu=0.0
$minFree=[int64]$drive.AvailableFreeSpace
$abort=$null
$lastProgress=-1
try {
 $p=Start-Process -FilePath $exe.FullName -ArgumentList @('--exact','live::tests::bench_large_base_async_parent_index_boundary','--ignored','--nocapture','--test-threads=1') -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
 Write-Output "STRESS_STARTED size=$Size pid=$($p.Id) exe=$($exe.Name)"
 while($true){
  $p.Refresh()
  if($p.HasExited){break}
  $seconds=((Get-Date)-$start).TotalSeconds
  $peakRss=[Math]::Max($peakRss,[int64]$p.PeakWorkingSet64)
  $peakPrivate=[Math]::Max($peakPrivate,[int64]$p.PrivateMemorySize64)
  $lastCpu=[Math]::Max($lastCpu,$p.TotalProcessorTime.TotalSeconds)
  $free=[int64]$drive.AvailableFreeSpace
  $minFree=[Math]::Min($minFree,$free)
  try {
   $wmi=Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$($p.Id)" -ErrorAction Stop
   if($null -ne $wmi){
    if($null -ne $wmi.ReadTransferCount){$lastRead=[uint64]$wmi.ReadTransferCount}
    if($null -ne $wmi.WriteTransferCount){$lastWrite=[uint64]$wmi.WriteTransferCount}
   }
  }catch {}
  $samples.Add([ordered]@{
   elapsedSeconds=[Math]::Round($seconds,2)
   workingSetBytes=[int64]$p.WorkingSet64
   peakWorkingSetBytes=$peakRss
   privateBytes=[int64]$p.PrivateMemorySize64
   cumulativeReadTransferBytes=$lastRead
   cumulativeWriteTransferBytes=$lastWrite
   volumeFreeBytes=$free
  })
  $bucket=[int][Math]::Floor($seconds/30)
  if($bucket -ne $lastProgress){
   $lastProgress=$bucket
   Write-Output ('STRESS_PROGRESS elapsed_s={0:N0} peak_rss_mib={1:N1} peak_private_mib={2:N1} free_gib={3:N1}' -f $seconds,($peakRss/1MB),($peakPrivate/1MB),($free/1GB))
  }
  if($peakRss -gt ($MaxWorkingSetMB*1MB)){$abort='Memory watchdog';break}
  if($free -lt ($MinFreeGB*1GB)){$abort='Disk watchdog';break}
  if($seconds -gt $TimeoutSeconds){$abort='Duration watchdog';break}
  Start-Sleep -Milliseconds $PollMilliseconds
 }
 if($null -ne $abort){
  Write-Output "STRESS_ABORT reason=$abort"
  if(-not $p.HasExited){Stop-Process -Id $p.Id -Force}
 }
 $p.WaitForExit()
 $p.Refresh()
 $peakRss=[Math]::Max($peakRss,[int64]$p.PeakWorkingSet64)
 $lastCpu=[Math]::Max($lastCpu,$p.TotalProcessorTime.TotalSeconds)
 $seconds=[Math]::Round(((Get-Date)-$start).TotalSeconds,2)
 $out=if(Test-Path $stdout){Get-Content $stdout -Raw}else{''}
 $err=if(Test-Path $stderr){Get-Content $stderr -Raw}else{''}
 $passed=($p.ExitCode -eq 0) -and ($null -eq $abort) -and $out.Contains('SYNTHETIC_SCOPED_ASYNC_BASE') -and $out.Contains('test result: ok.')
 $result=[ordered]@{
  schema=1
  purpose='Synthetic test only. No production Windows Search access.'
  size=$Size
  testExecutable=$exe.Name
  passed=$passed
  exitCode=$p.ExitCode
  abortReason=$abort
  wallSeconds=$seconds
  peakWorkingSetBytes=$peakRss
  peakPrivateBytesSampled=$peakPrivate
  cpuSecondsObserved=[Math]::Round($lastCpu,2)
  processReadTransferBytesLastSample=$lastRead
  processWriteTransferBytesLastSample=$lastWrite
  lowestFreeVolumeBytesSampled=$minFree
  stdout=$stdout
  stderr=$stderr
  samples=$samples
 }
 $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $metrics -Encoding utf8
 Write-Output "STRESS_RESULT metrics=$metrics"
 Write-Output ('STRESS_SUMMARY passed={0} exit={1} wall_s={2} peak_rss_mib={3:N1} private_sampled_mib={4:N1} cpu_s={5} last_read_gib={6:N2} last_write_gib={7:N2}' -f $passed,$p.ExitCode,$seconds,($peakRss/1MB),($peakPrivate/1MB),$lastCpu,($lastRead/1GB),($lastWrite/1GB))
 $out -split '\r?\n' | Where-Object {$_ -match 'SYNTHETIC_SCOPED_ASYNC_BASE|test result:'} | ForEach-Object {Write-Output $_}
 if(-not $passed){
  if($err){Write-Output $err.Substring(0,[Math]::Min(2500,$err.Length))}
  exit 1
 }
}finally{
 [Environment]::SetEnvironmentVariable($setting,$previous,'Process')
}
