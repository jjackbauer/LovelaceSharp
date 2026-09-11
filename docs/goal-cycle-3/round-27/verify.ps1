Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$p='Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'
Write-Output '=== new gate tests ==='
& dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~FalsificationGate' 2>&1 | Select-String 'Passed!|Failed!|No test' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== all falsification tests (old + new) ==='
& dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~Falsification' 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== full Symbolics ==='
& dotnet test $p -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== gate test names ==='
Select-String -Path 'Lovelace.Symbolics.Tests\FalsificationGateTests.cs' -Pattern 'public void ' | ForEach-Object { $_.Line.Trim() }