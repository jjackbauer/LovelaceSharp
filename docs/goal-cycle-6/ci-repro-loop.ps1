$ErrorActionPreference = 'Continue'
$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$outdir = Join-Path $root 'docs\goal-cycle-6\ci-repro'
New-Item -ItemType Directory -Force -Path $outdir | Out-Null
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
$summary = @()
foreach ($p in $suites) {
  $name = ($p -split '/')[0]
  $log = Join-Path $outdir ("$name.log")
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $out = & dotnet test $p --configuration Release --nologo --collect:"XPlat Code Coverage" 2>&1
  $code = $LASTEXITCODE
  $sw.Stop()
  $out | Out-File -FilePath $log -Encoding utf8
  $line = "RESULT {0} EXIT={1} SECONDS={2:N1}" -f $name, $code, $sw.Elapsed.TotalSeconds
  $summary += $line
  Write-Output $line
  if ($code -ne 0) { Write-Output "FIRST_FAILURE $name"; break }
}
$summary | Out-File -FilePath (Join-Path $outdir 'summary.txt') -Encoding utf8
Write-Output 'REPRO_LOOP_DONE'
