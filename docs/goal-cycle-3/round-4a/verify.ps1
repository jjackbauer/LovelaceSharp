$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-4a'
function Run-Step { param([string]$Label,[string[]]$DotnetArgs,[string]$LogName)
  & dotnet @DotnetArgs *>&1 | Out-File (Join-Path $out $LogName) -Encoding utf8
  Write-Output ('{0} exit={1}' -f $Label,$LASTEXITCODE) }
Run-Step -Label 'build' -DotnetArgs @('build','LovelaceSharp.slnx','-c','Release','--nologo') -LogName 'v-build.log'
foreach ($p in @(@('run','Lovelace.Run.Tests/Lovelace.Run.Tests.csproj'),@('sym','Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj'),@('suite','Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj'),@('studio','Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj'),@('console','Lovelace.Console.Tests/Lovelace.Console.Tests.csproj'),@('abs','Lovelace.Abstractions.Tests/Lovelace.Abstractions.Tests.csproj'))) {
  Run-Step -Label $p[0] -DotnetArgs @('test',$p[1],'-c','Release','--nologo') -LogName ('v-' + $p[0] + '.log') }
Run-Step -Label 'aot' -DotnetArgs @('publish','Lovelace.Run/Lovelace.Run.csproj','-c','Release','-p:PublishAot=true','-p:InvariantGlobalization=true','-o','out/aot') -LogName 'v-aot.log'
Select-String -Path (Join-Path $out 'v-run.log'),(Join-Path $out 'v-sym.log'),(Join-Path $out 'v-suite.log'),(Join-Path $out 'v-studio.log'),(Join-Path $out 'v-console.log'),(Join-Path $out 'v-abs.log') -Pattern 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$sc = Join-Path $out 'v-probe.ls'
[System.IO.File]::WriteAllText($sc, 'x = symbol("x"); solve_full(x^2 - 4 == 0, x)', (New-Object System.Text.UTF8Encoding($false)))
$raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $sc --omit-functions))
Write-Output '--- raw JSON fragments ---'
Write-Output (([regex]::Match($raw, '"name":"status".{0,80}')).Value)
Write-Output (([regex]::Match($raw, '"name":"complete".{0,80}')).Value)
Write-Output (([regex]::Match($raw, '"name":"exactness".{0,80}')).Value)
Write-Output '--- type() probe ---'
[System.IO.File]::WriteAllText($sc, 'x = symbol("x"); t = type(solve_full(x^2 - 4 == 0, x).status); t', (New-Object System.Text.UTF8Encoding($false)))
$raw2 = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $sc --omit-functions))
Write-Output (([regex]::Match($raw2, '"structured":.{0,120}')).Value)
Write-Output '--- ci.yml changed asserts ---'
Select-String -Path .github\workflows\ci.yml -Pattern "kind'\] == 'Enum|complete'\]\['value'\]|status'\]\['value'" | ForEach-Object { $_.LineNumber.ToString() + ': ' + $_.Line.Trim() }