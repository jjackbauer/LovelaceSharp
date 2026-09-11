$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-7'
New-Item -ItemType Directory -Force -Path $out | Out-Null
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
 'Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj',
 'Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj',
 'Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj',
 'Lovelace.Run.Tests/Lovelace.Run.Tests.csproj',
 'precbench.Tests/precbench.Tests.csproj')
$total=0; $failed=0
foreach ($s in $suites) {
  $name = ($s -split '/')[0]
  $txt = & dotnet test $s -c Release --nologo 2>&1 | Out-String
  [System.IO.File]::WriteAllText((Join-Path $out ('sweep-' + $name + '.log')), $txt, (New-Object System.Text.UTF8Encoding($false)))
  $m = [regex]::Match($txt, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+), Total:\s+(\d+)')
  if ($m.Success) { $f=[int]$m.Groups[1].Value; $p=[int]$m.Groups[2].Value; $sk=[int]$m.Groups[3].Value; $total+=$p; $failed+=$f; Write-Output ('{0,-38} passed={1,-5} failed={2,-3} skipped={3}' -f $name,$p,$f,$sk) } else { Write-Output ('{0,-38} NO-SUMMARY' -f $name) } }
$txt = & dotnet test 'Lovelace.Real.Tests/Lovelace.Real.Tests.csproj' -c Release --nologo --filter 'Category!=Heavy' 2>&1 | Out-String
$m = [regex]::Match($txt, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+), Total:\s+(\d+)')
if ($m.Success) { $f=[int]$m.Groups[1].Value; $p=[int]$m.Groups[2].Value; $total+=$p; $failed+=$f; Write-Output ('{0,-38} passed={1,-5} failed={2,-3} skipped={3}' -f 'Lovelace.Real.Tests(nonHeavy)',$p,$f,[int]$m.Groups[3].Value) }
Write-Output ('TOTAL passed={0} failed={1}' -f $total,$failed)