Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-11'
$cases = @(@{N='f1_factor';S='x = symbol("x"); factor(-x^2 + 1)'}, @{N='f1_expand_back';S='x = symbol("x"); expand(factor(-x^2 + 1))'}, @{N='f1_control';S='x = symbol("x"); factor(x^2 - 1)'}, @{N='f1_negcube';S='x = symbol("x"); factor(-x^3 + x)'})
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $f --omit-functions))
  $o = $raw | ConvertFrom-Json
  if ($o.ok) { Write-Output ('{0,-16} -> {1}' -f $c.N, $o.result.structured.pretty) } else { Write-Output ('{0,-16} -> ERR {1}' -f $c.N, $o.message) } }