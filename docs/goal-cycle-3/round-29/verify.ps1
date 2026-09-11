Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
$codes = [regex]::Matches($out, 'warning (CS\d+|xUnit\d+)') | ForEach-Object { $_.Groups[1].Value } | Group-Object | Sort-Object Name
Write-Output ('warning summary: ' + (($codes | ForEach-Object { $_.Name + ' x' + $_.Count }) -join ', '))
Write-Output ('CS0109 remaining: ' + ([regex]::Matches($out, 'warning CS0109')).Count)
Write-Output ('CS0108 (would indicate rebinding): ' + ([regex]::Matches($out, 'warning CS0108')).Count)
Write-Output ('errors: ' + ([regex]::Matches($out, ': error ')).Count)
Write-Output '=== key suites ==='
& dotnet test Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
& dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj -c Release --nologo --filter 'Category!=Heavy' 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
& dotnet test Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }