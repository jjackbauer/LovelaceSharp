$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$p='Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'
Remove-Item Env:\LOVELACE_REQUIRE_SYMPY -ErrorAction SilentlyContinue
Write-Output '=== RUN A: no env var (must be SKIPPED, never passed) ==='
$a = & dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~DifferentialOracle' 2>&1 | Out-String
[System.IO.File]::WriteAllText('docs\goal-cycle-3\round-11\v-runA.log', $a, (New-Object System.Text.UTF8Encoding($false)))
($a -split [char]10) | Where-Object { $_ -match 'Passed!|Failed!|Skipped:|No test matches' } | ForEach-Object { $_.Trim() }
$env:LOVELACE_REQUIRE_SYMPY='1'
Write-Output '=== RUN B: LOVELACE_REQUIRE_SYMPY=1 (must FAIL) ==='
$b = & dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~DifferentialOracle' 2>&1 | Out-String
[System.IO.File]::WriteAllText('docs\goal-cycle-3\round-11\v-runB.log', $b, (New-Object System.Text.UTF8Encoding($false)))
($b -split [char]10) | Where-Object { $_ -match 'Passed!|Failed!' } | ForEach-Object { $_.Trim() }
($b -split [char]10) | Where-Object { $_ -match 'requires the SymPy oracle' } | Select-Object -First 1 | ForEach-Object { $_.Trim() }
Remove-Item Env:\LOVELACE_REQUIRE_SYMPY -ErrorAction SilentlyContinue
Write-Output '=== silent-pass path remaining? ==='
(Select-String -Path Lovelace.Symbolics.Tests\DifferentialOracleTests.cs -Pattern 'is null\) return|SympyProbe\(\) is null' | Measure-Object).Count