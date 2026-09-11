$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$dir = Join-Path $root 'docs\goal-cycle-3\round-1\probes'
$enc = New-Object System.Text.UTF8Encoding($false)
$p = @(
  @{ N='r2_compile';      S='x = symbol("x"); compile_full(x^2 + 1, [x])';      A=@() },
  @{ N='r2_optimize';     S='x = symbol("x"); optimize_full(x^2 + 2*x + 1, [x])'; A=@() },
  @{ N='r2_arity';        S='x = symbol("x"); compile_full(x^2 + 1)';            A=@() },
  @{ N='r2_budget_big';   S='x = symbol("x"); (x+1)^12';                          A=@('--print-budget','6') },
  @{ N='r2_budget_none';  S='x = symbol("x"); (x+1)^12';                          A=@() },
  @{ N='r2_cancel_heavy'; S='x = symbol("x"); solve_full(x^8 - 3*x^5 + x - 1 == 0, x)'; A=@('--cancel-after','1') }
)
foreach ($q in $p) {
  $f = Join-Path $dir ($q.N + '.ls')
  [System.IO.File]::WriteAllText($f, $q.S, $enc)
  $a = @('--file', $f, '--omit-functions') + $q.A
  $raw = [string]::Join([char]10, (& $exe @a))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.json')), $raw, $enc)
  $sb = New-Object System.Text.StringBuilder
  for ($i = 0; $i -lt $raw.Length; $i += 1500) { $len = [Math]::Min(1500, $raw.Length - $i); [void]$sb.AppendLine($raw.Substring($i, $len)) }
  [System.IO.File]::WriteAllText((Join-Path $dir ($q.N + '.chunked')), $sb.ToString(), $enc)
  Write-Output ('{0} exit={1} bytes={2}' -f $q.N, $code, $raw.Length)
}