$ErrorActionPreference = 'Continue'
$suites = @(
  'Lovelace.Abstractions.Tests/Lovelace.Abstractions.Tests.csproj',
  'Lovelace.Array.Tests/Lovelace.Array.Tests.csproj',
  'Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj',
  'Lovelace.Console.Tests/Lovelace.Console.Tests.csproj',
  'Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.csproj',
  'Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj',
  'Lovelace.Knowledge.Tests/Lovelace.Knowledge.Tests.csproj',
  'Lovelace.Natural.Tests/Lovelace.Natural.Tests.csproj',
  'Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj',
  'Lovelace.Run.Tests/Lovelace.Run.Tests.csproj',
  'Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj',
  'Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj',
  'Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj',
  'precbench.Tests/precbench.Tests.csproj'
)
foreach ($p in $suites) {
  $name = ($p -split '/')[0]
  $log = 'docs/goal-cycle-3/round-5/final-' + $name + '.log'
  Write-Output ('===== ' + $p)
  dotnet test $p -c Release --nologo 2>&1 | Tee-Object -FilePath $log | Select-String -Pattern 'Passed!|Failed!|error CS' | Select-Object -Last 4
}
Write-Output '===== Lovelace.Real.Tests (Category!=Heavy)'
dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj -c Release --nologo --filter 'Category!=Heavy' 2>&1 | Tee-Object -FilePath docs/goal-cycle-3/round-5/final-Lovelace.Real.Tests-nonheavy.log | Select-String -Pattern 'Passed!|Failed!|error CS' | Select-Object -Last 4
Write-Output 'ALL SUITES DONE'