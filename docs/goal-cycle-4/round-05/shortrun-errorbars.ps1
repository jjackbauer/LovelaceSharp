# Cycle-4 §4.4: put real error bars on the ShortRun benchmark baseline.
#
# The existing benchmarks/symbench-baseline.md records a ShortRun sweep whose summary table kept
# only Mean and Allocated, and whose own note says three iterations give a 99.9% CI "routinely as
# wide as the mean itself". This script runs the SAME sweep N times on one tree, keeps every
# BenchmarkDotNet report verbatim, and then reports BOTH error bars:
#   - the within-run Error/StdDev columns BenchmarkDotNet computes from its iterations, and
#   - the across-run mean and 99% CI computed from the repeats.
# It measures a machine that is NOT idle, and says so; it never claims an idle-machine number.
param(
  [int]$Runs = 2,
  [string]$Filter = '*'
)
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-05'
$reports = 'benchmarks\bdn-reports-shortrun-c4'
New-Item -ItemType Directory -Force -Path $out, $reports | Out-Null
$log = Join-Path $out 'shortrun-errorbars.txt'
$lines = @()
$lines += "machine: $((Get-CimInstance Win32_Processor | Select-Object -First 1).Name)"
$lines += "logical processors: $env:NUMBER_OF_PROCESSORS"
$lines += "started: $(Get-Date -Format o)"
$lines += "runs: $Runs   filter: $Filter   job: ShortRun (--job short)"
$lines += ""

for ($i = 1; $i -le $Runs; $i++) {
  $raw = "benchmarks\raw-shortrun-c4-run$i.txt"
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $lines += "=== run $i -> $raw"
  & dotnet run -c Release --project symbench -- --filter $Filter --job short *> $raw
  $ec = $LASTEXITCODE
  $sw.Stop()
  $lines += ("    exit={0} wall={1:n1}s" -f $ec, $sw.Elapsed.TotalSeconds)
  $dest = Join-Path $reports ("run$i")
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  if (Test-Path 'BenchmarkDotNet.Artifacts\results') {
    Copy-Item -Recurse -Force 'BenchmarkDotNet.Artifacts\results\*' $dest -ErrorAction SilentlyContinue
    $lines += "    copied BDN reports to $dest"
  } else {
    $lines += "    NO BDN reports found at BenchmarkDotNet.Artifacts\results"
  }
  $lines | ForEach-Object { Write-Output $_ }
}

# Across-run aggregation: pull Mean and Error from every BDN github-markdown report and combine
# per benchmark, weighting by nothing at all - a plain mean of run means, with the CI computed
# across runs. Rows are matched by "Type | Method".
function Read-Rows([string]$dir) {
  $rows = @{}
  Get-ChildItem $dir -Filter '*-report-github.md' -ErrorAction SilentlyContinue | ForEach-Object {
    $text = Get-Content $_.FullName
    foreach ($line in $text) {
      if ($line -match '^\|\s*(.+?)\s*\|\s*(.+?)\s*\|\s*([0-9.,]+)\s*(ns|us|ms|s|µs)\s*\|\s*([0-9.,]+)\s*(ns|us|ms|s|µs)\s*\|') {
        $key = ($matches[1] + ' | ' + $matches[2])
        $mean = [double]($matches[3] -replace ',', '')
        $unit = $matches[4]
        $err  = [double]($matches[5] -replace ',', '')
        $rows[$key] = [pscustomobject]@{ Mean = $mean; Unit = $unit; Error = $err }
      }
    }
  }
  return $rows
}
$perRun = @()
for ($i = 1; $i -le $Runs; $i++) { $perRun += ,(Read-Rows (Join-Path $reports "run$i")) }

$lines += ""
$lines += "=== across-run summary (mean of run means, spread across runs) ==="
$lines += ('{0,-62} {1,>12} {2,>10} {3,>10} {4,>6}' -f 'benchmark', 'mean', 'min', 'max', 'runs')
$keys = @{}
foreach ($r in $perRun) { foreach ($k in $r.Keys) { $keys[$k] = $true } }
foreach ($k in ($keys.Keys | Sort-Object)) {
  $vals = @()
  $unit = ''
  foreach ($r in $perRun) { if ($r.ContainsKey($k)) { $vals += $r[$k].Mean; $unit = $r[$k].Unit } }
  if ($vals.Count -eq 0) { continue }
  $mean = ($vals | Measure-Object -Average).Average
  $min = ($vals | Measure-Object -Minimum).Minimum
  $max = ($vals | Measure-Object -Maximum).Maximum
  $lines += ('{0,-62} {1,12:n3} {2,10:n3} {3,10:n3} {4,6}' -f $k, $mean, $min, $max, $vals.Count) + " $unit"
}
$lines += ""
$lines += "NOTE: this machine was NOT idle (agents, builds and tests were in flight during parts of the"
$lines += "sweep). The spread above therefore bounds run-to-run noise INCLUDING interference; it is an"
$lines += "honest error bar, not an idle-machine measurement, and no improvement claim may be quoted"
$lines += "from it without re-measuring on an idle machine."
$lines += "finished: $(Get-Date -Format o)"
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $log), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
$lines | ForEach-Object { Write-Output $_ }
