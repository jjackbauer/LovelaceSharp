$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$dir = Join-Path $root 'docs\goal-cycle-3\round-1\probes'
$enc = New-Object System.Text.UTF8Encoding($false)
$p = @(
  @{ N='ns_real_reject';  S='x = symbol("x"); solve_full(x^2 + 1 == 0, x, real)' },
  @{ N='ns_sqrt_neg';     S='x = symbol("x"); solve_full(sqrt(x) == -2, x)' },
  @{ N='ns_exp_zero';     S='x = symbol("x"); solve_full(exp(x) == 0, x)' },
  @{ N='ns_x_over_x';     S='x = symbol("x"); solve_full(x/x == 0, x)' },
  @{ N='ns_pole_all';     S='x = symbol("x"); solve_full((x^2 - 4)/(x^2 - 4) == 1, x)' }
)
foreach ($q in $p) {
  $f = Join-Path $dir ($q.N + '.ls')
  [System.IO.File]::WriteAllText($f, $q.S, $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.json')), $raw, $enc)
  $sb = New-Object System.Text.StringBuilder
  for ($i = 0; $i -lt $raw.Length; $i += 1500) { $len = [Math]::Min(1500, $raw.Length - $i); [void]$sb.AppendLine($raw.Substring($i, $len)) }
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.chunked')), $sb.ToString(), $enc)
  Write-Output ('{0} exit={1} bytes={2}' -f $q.N, $code, $raw.Length)
}