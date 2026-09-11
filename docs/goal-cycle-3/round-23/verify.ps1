Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-23\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @(
 @{N='abs_num';S='abs(-3)'},
 @{N='abs_sym';S='x = symbol("x"); abs(x)'},
 @{N='abs_diff';S='x = symbol("x"); diff(abs(x), x)'},
 @{N='abs_subs';S='x = symbol("x"); subs(abs(x), x, -3)'},
 @{N='rt_rewriter';S='x = symbol("x", real); simplify_full(sqrt(x^2)).expression'},
 @{N='rt_direct';S='x = symbol("x"); abs(x).canonical'}
)
$res=@{}
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $out = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))
  try { $o = $out | ConvertFrom-Json; if ($o.ok) { $s=$o.result.structured; $v = if ($s.canonical) { $s.canonical } elseif ($s.pretty) { $s.pretty } else { $s.value }; $res[$c.N]=$v; Write-Output ('{0,-12} -> {1}' -f $c.N, $v) } else { Write-Output ('{0,-12} -> {1}: {2}' -f $c.N, $o.code, $o.message) } } catch { Write-Output ('{0,-12} -> NOJSON' -f $c.N) } }
Write-Output ('ROUNDTRIP canonical equal: ' + ($res['rt_direct'] -eq $res['rt_rewriter']))