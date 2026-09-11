Set-Location "C:\Users\ricar\dev\LovelaceSharp"
$suites = @("Lovelace.Run.Tests","Lovelace.Symbolics.Tests","Lovelace.Suite.Tests","Lovelace.Studio.Tests","Lovelace.Console.Tests")
foreach ($s in $suites) {
  Write-Output "########## dotnet test $s"
  dotnet test "$s/$s.csproj" -c Release --nologo 2>&1 | Select-String -Pattern 'Passed!|Failed!|error ' | ForEach-Object { $_.Line }
}
