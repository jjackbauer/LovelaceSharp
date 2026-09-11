$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo *>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
& dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo *>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$e=Get-Item 'out\aot\Lovelace.Run.exe'
$src=Get-ChildItem -Recurse -Include *.cs -File | Where-Object { $_.FullName -notmatch '\\(obj|bin|out)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Output ('aot bytes={0} STALE={1}' -f $e.Length, ($e.LastWriteTime -lt $src.LastWriteTime))
foreach ($n in @('matrix_inverse_result','matrix_solve_result')) {
  $f = 'Lovelace.Run.Tests\fixtures\' + $n + '.ls'
  $raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $f --omit-functions))
  $o = $raw | ConvertFrom-Json
  $s = $o.result.structured; $f2=@{}; foreach ($x in $s.fields) { $f2[$x.name]=$x.value }
  $diag = $f2['diagnostics']
  $d0 = if ($diag.elements.Count -gt 0) { $df=@{}; foreach ($y in $diag.elements[0].fields) { $df[$y.name]=$y.value }; 'code=' + $df['code'].value + ' cat=' + $df['category'].value + '/' + $df['category'].type } else { 'none' }
  Write-Output ('{0}: type={1} status={2}/{3} complete={4} completeness={5}/{6} diagCount={7} {8}' -f $n, $s.type, $f2['status'].value, $f2['status'].type, $f2['complete'].value, $f2['completeness'].value, $f2['completeness'].type, $diag.elements.Count, $d0) }