Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-13'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = @(@{N='neg_quad';S='x = symbol("x"); expand(factor(-x^2 + 1))'}, @{N='neg_cubic';S='x = symbol("x"); expand(factor(-x^3 + x))'}, @{N='control';S='x = symbol("x"); expand(factor(x^2 - 1))'}, @{N='neg_factor';S='x = symbol("x"); factor(-x^2 + 1)'})
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $raw = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))
  try { $o = $raw | ConvertFrom-Json; if ($o.ok) { Write-Output ('{0,-12} -> {1}' -f $c.N, $o.result.structured.pretty) } else { Write-Output ('{0,-12} -> ERR {1}' -f $c.N, $o.message) } } catch { Write-Output ('{0,-12} -> NOJSON' -f $c.N) } }