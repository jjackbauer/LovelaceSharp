Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-3\round-33'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = @(
  @{N='n18_inf_minus_inf';  S='inf - inf'},
  @{N='n18_zero_times_inf';  S='0 * inf'},
  @{N='n18_control_inf_plus_one'; S='inf + 1'},
  @{N='ctrl_sym_minus_sym'; S='x = symbol("x"); x - x'},
  @{N='ctrl_zero_times_sym'; S='x = symbol("x"); 0 * x'},
  @{N='sib_inf_over_inf';   S='inf / inf'},
  @{N='sib_inf_pow_zero';   S='inf ^ 0'},
  @{N='sib_zero_over_zero'; S='0 / 0'},
  @{N='sib_sym_over_sym';   S='x = symbol("x"); x / x'},
  @{N='sib_sym_pow_zero';   S='x = symbol("x"); x ^ 0'},
  @{N='ctrl_sym_pole_identity'; S='x = symbol("x"); (1/x) - (1/x)'},
  @{N='n18_inf_minus_inf_plus_1'; S='(inf - inf) + 1'}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $errFile = Join-Path $d ($c.N + '.err.txt')
  $raw = (& dotnet $dll --file $f --omit-functions 2>$errFile | Out-String).Trim()
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $d ($c.N + '.out.txt')), $raw, $enc)
  try {
    $o = $raw | ConvertFrom-Json
    if ($o.ok) { Write-Output ('{0,-28} exit={1} -> {2}' -f $c.N, $code, $o.result.structured.pretty) }
    else { Write-Output ('{0,-28} exit={1} -> ERR {2}' -f $c.N, $code, $o.message) }
  } catch {
    Write-Output ('{0,-28} exit={1} -> NOJSON: {2}' -f $c.N, $code, $raw)
  }
}
