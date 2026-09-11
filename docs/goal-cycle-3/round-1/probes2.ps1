$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$dir = Join-Path $root 'docs\goal-cycle-3\round-1\probes'
$enc = New-Object System.Text.UTF8Encoding($false)
$p = @(
  @{ N='diag_transform'; S='x = symbol("x"); simplify_full(sin(x)^2 + cos(x)^2)' },
  @{ N='diag_limit';     S='x = symbol("x"); limit_full(sin(x)/x, x, 0)' },
  @{ N='diag_compile';   S='x = symbol("x"); compile_full(x^2 + 1)' },
  @{ N='diag_optimize';  S='x = symbol("x"); optimize_full(exp(x)*exp(x))' },
  @{ N='abs_symbol';     S='x = symbol("x"); abs(x)' },
  @{ N='sqrt_sq_real';   S='x = symbol("x", real); simplify_full(sqrt(x^2))' },
  @{ N='budget_small';   S='x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)' },
  @{ N='heavy_cancel';   S='x = symbol("x"); simplify_full((x+1)^40)' }
)
foreach ($q in $p) {
  $f = Join-Path $dir ($q.N + '.ls')
  [System.IO.File]::WriteAllText($f, $q.S, $enc)
  $a = @('--file', $f, '--omit-functions')
  if ($q.N -eq 'budget_small') { $a += @('--print-budget','6') }
  if ($q.N -eq 'heavy_cancel') { $a += @('--cancel-after','1') }
  $raw = [string]::Join([char]10, (& $exe @a))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.json')), $raw, $enc)
  $sb = New-Object System.Text.StringBuilder
  for ($i = 0; $i -lt $raw.Length; $i += 1500) { $len = [Math]::Min(1500, $raw.Length - $i); [void]$sb.AppendLine($raw.Substring($i, $len)) }
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.chunked')), $sb.ToString(), $enc)
  Write-Output ('{0} exit={1} bytes={2}' -f $q.N, $code, $raw.Length)
}
$log = [System.IO.File]::ReadAllText((Join-Path $root 'docs\goal-cycle-3\round-1\build-release.log'))
$m = [regex]::Matches($log, 'warning (CS\d+)')
$g = $m | ForEach-Object { $_.Groups[1].Value } | Group-Object | Sort-Object Count -Descending
foreach ($x in $g) { Write-Output ('WARN {0} x{1}' -f $x.Name, $x.Count) }