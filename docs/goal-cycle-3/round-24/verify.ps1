Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-24\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @(
 @{N='pow';S='x = symbol("x"); (x + 1)^12'},
 @{N='integ';S='x = symbol("x"); integrate_full(exp(-x^2), x).expression'},
 @{N='fact';S='x = symbol("x"); factor(-x^2 - 2*x - 1)'},
 @{N='sub1';S='x = symbol("x"); y = symbol("y"); a = symbol("a"); x - (y - a)'},
 @{N='sub2';S='x = symbol("x"); y = symbol("y"); a = symbol("a"); a - (x + y)'},
 @{N='sub3';S='x = symbol("x"); y = symbol("y"); a = symbol("a"); x + (y - a)'}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $out = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))
  try { $o = $out | ConvertFrom-Json; if ($o.ok) { $s=$o.result.structured; $v = if ($s.pretty) { $s.pretty } else { $s.value }; Write-Output ('{0,-8} -> {1}' -f $c.N, $v) } else { Write-Output ('{0,-8} -> {1}: {2}' -f $c.N, $o.code, $o.message) } } catch { Write-Output ('{0,-8} -> NOJSON' -f $c.N) } }