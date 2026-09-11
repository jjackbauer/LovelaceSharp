$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-4b'
function Run-Step { param([string]$Label,[string[]]$DotnetArgs,[string]$LogName)
  & dotnet @DotnetArgs *>&1 | Out-File (Join-Path $out $LogName) -Encoding utf8
  Write-Output ('{0} exit={1}' -f $Label,$LASTEXITCODE) }
Run-Step -Label 'build' -DotnetArgs @('build','LovelaceSharp.slnx','-c','Release','--nologo') -LogName 'v-build.log'
foreach ($p in @(@('run','Lovelace.Run.Tests/Lovelace.Run.Tests.csproj'),@('sym','Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'),@('suite','Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj'),@('studio','Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj'),@('console','Lovelace.Console.Tests/Lovelace.Console.Tests.csproj'))) {
  Run-Step -Label $p[0] -DotnetArgs @('test',$p[1],'-c','Release','--nologo') -LogName ('v-' + $p[0] + '.log') }
Run-Step -Label 'aot' -DotnetArgs @('publish','Lovelace.Run/Lovelace.Run.csproj','-c','Release','-p:PublishAot=true','-p:InvariantGlobalization=true','-o','out/aot') -LogName 'v-aot.log'
Select-String -Path (Join-Path $out 'v-run.log'),(Join-Path $out 'v-sym.log'),(Join-Path $out 'v-suite.log'),(Join-Path $out 'v-studio.log'),(Join-Path $out 'v-console.log') -Pattern 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$sc = Join-Path $out 'v-solve.ls'
[System.IO.File]::WriteAllText($sc, 'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)', $enc)
$raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $sc --omit-functions))
Write-Output '--- SolveResult diagnostics ---'
Write-Output (([regex]::Match($raw, '"name":"diagnostics".{0,420}')).Value)
Write-Output '--- unrepresented_reason still present? ---'
Write-Output (([regex]::Match($raw, '"name":"unrepresented_reason".{0,90}')).Value)
Write-Output '--- any Text diagnostics left on the seven? ---'
Write-Output ([regex]::Matches($raw, '"name":"diagnostics","value":{"kind":"Text"').Count)
Write-Output '--- matrix records still free text? ---'
[System.IO.File]::WriteAllText($sc, 'a = [[1, 2], [2, 4]]; inv_full(a)', $enc)
$raw2 = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $sc --omit-functions))
Write-Output (([regex]::Match($raw2, '"name":"diagnostics".{0,140}')).Value)