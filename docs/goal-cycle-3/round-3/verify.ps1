$ErrorActionPreference='Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-3'
function Run-Step { param([string]$Label,[string[]]$DotnetArgs,[string]$LogName)
  & dotnet @DotnetArgs *>&1 | Out-File (Join-Path $out $LogName) -Encoding utf8
  Write-Output ('{0} exit={1}' -f $Label,$LASTEXITCODE) }
Run-Step -Label 'build' -DotnetArgs @('build','LovelaceSharp.slnx','-c','Release','--nologo') -LogName 'verify-build.log'
Run-Step -Label 'run-tests' -DotnetArgs @('test','Lovelace.Run.Tests/Lovelace.Run.Tests.csproj','-c','Release','--nologo') -LogName 'verify-run.log'
Run-Step -Label 'symbolics' -DotnetArgs @('test','Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj','-c','Release','--nologo') -LogName 'verify-symbolics.log'
Run-Step -Label 'suite' -DotnetArgs @('test','Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj','-c','Release','--nologo') -LogName 'verify-suite.log'
Run-Step -Label 'studio' -DotnetArgs @('test','Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj','-c','Release','--nologo') -LogName 'verify-studio.log'
Run-Step -Label 'aot' -DotnetArgs @('publish','Lovelace.Run/Lovelace.Run.csproj','-c','Release','-p:PublishAot=true','-p:InvariantGlobalization=true','-o','out/aot') -LogName 'verify-aot.log'
Select-String -Path (Join-Path $out 'verify-run.log'),(Join-Path $out 'verify-symbolics.log'),(Join-Path $out 'verify-suite.log'),(Join-Path $out 'verify-studio.log') -Pattern 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$e=Get-Item 'out\aot\Lovelace.Run.exe'
$src=Get-ChildItem -Recurse -Include *.cs -File | Where-Object { $_.FullName -notmatch '\\(obj|bin|out)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Output ('aot bytes={0} STALE={1}' -f $e.Length, ($e.LastWriteTime -lt $src.LastWriteTime))
$sc = Join-Path $out 'live.ls'
[System.IO.File]::WriteAllText($sc, 'x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])', (New-Object System.Text.UTF8Encoding($false)))
$raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $sc --omit-functions))
$o = $raw | ConvertFrom-Json
$s = $o.result.structured
$top = ($s.fields | ForEach-Object { $_.name + ':' + $_.value.kind }) -join ', '
$sol = $s.fields | Where-Object { $_.name -eq 'solutions' }
$inner = $sol.value.elements[0]
$innerFields = ($inner.fields | ForEach-Object { $_.name + ':' + $_.value.kind }) -join ', '
$b0 = ($inner.fields | Where-Object { $_.name -eq 'bindings' }).value.elements[0]
$bf = ($b0.fields | ForEach-Object { $_.name + '=' + $_.value.pretty + '(' + $_.value.kind + ')' }) -join ', '
Write-Output ('SystemSolveResult fields: ' + $top)
Write-Output ('SystemSolution fields: ' + $innerFields)
Write-Output ('Binding[0]: ' + $bf)