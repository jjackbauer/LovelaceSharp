# Cycle-4 full sweep. REPRODUCES the Cycle-3 definition verbatim
# (docs/goal-cycle-3/final-sweep.ps1) so the totals are comparable:
# 14 projects unfiltered + Lovelace.Real.Tests with --filter 'Category!=Heavy'.
$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-01'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$suites = @('Lovelace.Abstractions.Tests','Lovelace.Array.Tests','Lovelace.Complex.Tests','Lovelace.Console.Tests','Lovelace.Dsp.Tests','Lovelace.Integer.Tests','Lovelace.Knowledge.Tests','Lovelace.Natural.Tests','Lovelace.Representation.Tests','Lovelace.Studio.Tests','Lovelace.Suite.Tests','Lovelace.Symbolics.Tests','Lovelace.Run.Tests','precbench.Tests')
$total = 0; $failed = 0; $skipped = 0; $lines = @()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
foreach ($s in $suites) {
  $t = & dotnet test ($s + '/' + $s + '.csproj') -c Release --nologo 2>&1 | Out-String
  $m = [regex]::Match($t, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+)')
  if ($m.Success) {
    $f = [int]$m.Groups[1].Value; $p = [int]$m.Groups[2].Value; $sk = [int]$m.Groups[3].Value
    $total += $p; $failed += $f; $skipped += $sk
    $lines += ('{0,-32} passed={1,-5} failed={2,-3} skipped={3}' -f $s, $p, $f, $sk)
  } else {
    $lines += ('{0,-32} NO-SUMMARY' -f $s)
    [System.IO.File]::WriteAllText((Join-Path $out ('raw-' + $s + '.log')), $t, (New-Object System.Text.UTF8Encoding($false)))
  }
}
$t = & dotnet test 'Lovelace.Real.Tests/Lovelace.Real.Tests.csproj' -c Release --nologo --filter 'Category!=Heavy' 2>&1 | Out-String
$m = [regex]::Match($t, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+)')
if ($m.Success) {
  $f = [int]$m.Groups[1].Value; $p = [int]$m.Groups[2].Value; $sk = [int]$m.Groups[3].Value
  $total += $p; $failed += $f; $skipped += $sk
  $lines += ('{0,-32} passed={1,-5} failed={2,-3} skipped={3}' -f 'Lovelace.Real.Tests(nonHeavy)', $p, $f, $sk)
} else {
  $lines += ('{0,-32} NO-SUMMARY' -f 'Lovelace.Real.Tests(nonHeavy)')
}
$sw.Stop()
$lines | ForEach-Object { Write-Output $_ }
$summary = 'TOTAL passed={0} failed={1} skipped={2}' -f $total, $failed, $skipped
Write-Output $summary
Write-Output ('elapsed={0}s' -f [math]::Round($sw.Elapsed.TotalSeconds, 1))
[System.IO.File]::WriteAllText(
  (Join-Path $out 'suite-counts.txt'),
  (($lines + $summary + ('elapsed={0}s' -f [math]::Round($sw.Elapsed.TotalSeconds, 1))) -join [char]10),
  (New-Object System.Text.UTF8Encoding($false)))
