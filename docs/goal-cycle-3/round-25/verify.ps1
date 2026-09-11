Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$p='Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'
Write-Output '=== the two matrices, by name ==='
& dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~BoundReasoning_MatchesReferenceTruthTable|FullyQualifiedName~ContradictionDetection_IsNeitherMissedNorInvented' 2>&1 | Select-String 'Passed!|Failed!|No test' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== scaling tests ==='
& dotnet test $p -c Release --nologo --filter 'FullyQualifiedName~AssumptionAddScaling' 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
Write-Output '=== full Symbolics suite ==='
& dotnet test $p -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }