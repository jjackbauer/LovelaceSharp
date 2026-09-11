# Cycle-4 round-10: re-establish every measurement on the tree that closes §4.1.
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-10'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'final-verification.txt'
$lines = @()
$lines += "started: $(Get-Date -Format o)   HEAD $(git rev-parse --short HEAD)"
$lines += ""

$env:PATH = "C:\Users\ricar\dev\.lovelace-tools\python;" + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = '1'

$lines += "=== the suites that the §4.1 change touches, with the oracle REQUIRED ==="
foreach ($p in @(@('Lovelace.Symbolics.Tests', $null), @('Lovelace.Suite.Tests', $null), @('Lovelace.Run.Tests', $null), @('Lovelace.Real.Tests', 'Category!=Heavy'), @('Lovelace.Dsp.Tests', $null))) {
  $a = @('test', $p[0], '-c', 'Release', '--nologo')
  if ($p[1]) { $a += @('--filter', $p[1]) }
  $r = & dotnet @a 2>&1 | Select-String -Pattern 'Passed!|Failed!|\[FAIL\]|error CS' | Select-Object -First 4 | ForEach-Object { $_.Line.Trim() }
  $lines += ('{0,-28} {1}' -f $p[0], ($r -join ' | '))
}
$lines += ""
$lines += "=== oracle alone (must be Failed 0 / Passed 6 / Skipped 0) ==="
$lines += (& dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle" 2>&1 | Select-String -Pattern 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() })
$lines += ""
$lines += "=== forced full rebuild (D5) ==="
$b = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
$lines += "exit=$LASTEXITCODE  warning lines=$(([regex]::Matches($b, ': warning ')).Count)  error lines=$(([regex]::Matches($b, ': error ')).Count)"
$lines += ($b -split "`n" | Select-String -Pattern 'Warning\(s\)|Error\(s\)|Build succeeded|Build FAILED' | ForEach-Object { '  ' + $_.Line.Trim() })
$lines += ""
$lines += "=== full 15-project sweep (D4) ==="
$sweep = & 'docs\goal-cycle-4\round-01\sweep.ps1' 2>&1 | Out-String
$lines += $sweep
$lines += ""
$lines += "=== CI aot-smoke scenarios against the freshly published binary (D6) ==="
$lines += (& 'docs\goal-cycle-4\round-06\aot-and-smoke.ps1' 2>&1 | Out-String)
$lines += ""
$lines += "finished: $(Get-Date -Format o)"
$lines | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $log), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
