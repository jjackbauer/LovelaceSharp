Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$p='Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'
Write-Output '=== new metamorphic tests ==='
& dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~MetamorphicSetClosure' 2>&1 | Select-String 'Passed!|Failed!|No test' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== full Symbolics ==='
& dotnet test $p -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== test names in the new file ==='
Select-String -Path 'Lovelace.Symbolics.Tests\MetamorphicSetClosureTests.cs' -Pattern 'public void ' | ForEach-Object { $_.Line.Trim() }