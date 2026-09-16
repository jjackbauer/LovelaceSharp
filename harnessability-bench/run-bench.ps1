<#
.SYNOPSIS
  Execute the harnessability matrix with real DSH headless sessions and score it.

.EXAMPLE
  ./run-bench.ps1 -Modes L0,L2 -Tasks syntax-missing-brace,undefined-name
.EXAMPLE
  ./run-bench.ps1                       # 20 tasks x 4 modes x 1 replicate = 80 runs
#>
param(
  [string]$Modes = 'L0,L1,L2,L3',
  [string]$Tasks = 'all',
  [int]$Replicates = 1,
  [int]$Budget = 10,
  [int]$TimeoutSec = 1200,
  [switch]$Sequential,
  [string]$Tag = 'matrix'
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$py = Join-Path $root 'tools\python\python.exe'
$driver = Join-Path $root 'dsh\run_matrix.py'
$runner = Join-Path $root 'runner\Lovelace.Run.exe'
$runsDir = Join-Path $root 'runs'
New-Item -ItemType Directory -Force -Path $runsDir | Out-Null

$modeList = $Modes.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ }
$arms = @()
foreach ($mode in $modeList) {
  $arms += [pscustomobject]@{
    Mode = $mode
    Out  = Join-Path $runsDir "runs-$Tag-$mode.jsonl"
    Args = @($driver, '--tasks', $Tasks, '--modes', $mode, '--replicates', $Replicates,
              '--budget', $Budget, '--timeout', $TimeoutSec, '--runner', $runner,
              '--out', (Join-Path $runsDir "runs-$Tag-$mode.jsonl"))
  }
}

Write-Host "Running $($arms.Count) arm(s) over tasks=$Tasks budget=$Budget" -ForegroundColor Cyan
if ($Sequential) {
  foreach ($arm in $arms) {
    Write-Host "--- arm $($arm.Mode) ---" -ForegroundColor Yellow
    & $py @($arm.Args)
    if ($LASTEXITCODE -ne 0) { throw "arm $($arm.Mode) failed" }
  }
} else {
  $arms | ForEach-Object -Parallel {
    $py = $using:py
    $mode = $_.Mode
    Write-Host "--- arm $mode ---" -ForegroundColor Yellow
    & $py @($_.Args) 2>&1 | ForEach-Object { "[$mode] $_" }
  } -ThrottleLimit $arms.Count
}

$merged = Join-Path $runsDir "runs-$Tag.jsonl"
if (Test-Path $merged) { Remove-Item $merged }
Get-ChildItem $runsDir -Filter "runs-$Tag-L*.jsonl" | Sort-Object Name | ForEach-Object {
  Get-Content $_.FullName | Add-Content $merged
}
$count = (Get-Content $merged | Measure-Object -Line).Lines
Write-Host "merged $count trajectories -> $merged" -ForegroundColor Green

Write-Host '=== score ===' -ForegroundColor Cyan
& $py (Join-Path $root 'lovelace-harnessability-public-branch\bench.py') score `
    --runs $merged --runner $runner --budget $Budget | Tee-Object -FilePath (Join-Path $runsDir "score-$Tag.json")
