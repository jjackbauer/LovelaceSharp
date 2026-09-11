$ErrorActionPreference='Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-2'
$enc = New-Object System.Text.UTF8Encoding($false)
function Run-Step {
  param([string]$Label, [string[]]$DotnetArgs, [string]$LogName)
  $sw=[System.Diagnostics.Stopwatch]::StartNew()
  & dotnet @DotnetArgs *>&1 | Out-File (Join-Path $out $LogName) -Encoding utf8
  $code=$LASTEXITCODE; $sw.Stop()
  Write-Output ('{0} exit={1} {2}s' -f $Label,$code,[int]$sw.Elapsed.TotalSeconds)
}
Run-Step -Label 'build-solution' -DotnetArgs @('build','LovelaceSharp.slnx','-c','Release','--nologo') -LogName 'verify-build.log'
Run-Step -Label 'test-Symbolics' -DotnetArgs @('test','Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj','-c','Release','--nologo') -LogName 'verify-symbolics.log'
Run-Step -Label 'test-Suite' -DotnetArgs @('test','Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj','-c','Release','--nologo') -LogName 'verify-suite.log'
Run-Step -Label 'test-Console' -DotnetArgs @('test','Lovelace.Console.Tests/Lovelace.Console.Tests.csproj','-c','Release','--nologo') -LogName 'verify-console.log'
Run-Step -Label 'publish-aot' -DotnetArgs @('publish','Lovelace.Run/Lovelace.Run.csproj','-c','Release','-p:PublishAot=true','-p:InvariantGlobalization=true','-o','out/aot') -LogName 'verify-aot.log'
Select-String -Path (Join-Path $out 'verify-symbolics.log'),(Join-Path $out 'verify-suite.log'),(Join-Path $out 'verify-console.log') -Pattern 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$srcNewest = Get-ChildItem -Recurse -Include *.cs -File | Where-Object { $_.FullName -notmatch '\\(obj|bin|out)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$e = Get-Item 'out\aot\Lovelace.Run.exe'
Write-Output ('aot exe={0} bytes mtime={1} newest-src={2} STALE={3}' -f $e.Length, $e.LastWriteTime.ToString('s'), $srcNewest.LastWriteTime.ToString('s'), ($e.LastWriteTime -lt $srcNewest.LastWriteTime))
$script = Join-Path $out 'aot-smoke.ls'
[System.IO.File]::WriteAllText($script, 'x = symbol("x"); solve_full(x^2 - 4 == 0, x)', $enc)
$raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $script --omit-functions))
$code = $LASTEXITCODE
$o = $raw | ConvertFrom-Json
$s = $o.result.structured; $f=@{}; foreach ($x in $s.fields) { $f[$x.name]=$x.value }
Write-Output ('aot-smoke exit={0} ok={1} type={2} status={3} complete={4} domain={5}' -f $code,$o.ok,$s.type,$f['status'].value,$f['complete'].value,$f['domain'].domain)