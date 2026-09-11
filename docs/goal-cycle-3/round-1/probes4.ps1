$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$dir = Join-Path $root 'docs\goal-cycle-3\round-1\probes'
$enc = New-Object System.Text.UTF8Encoding($false)
$p = @(
  @{ N='r3_budget_expand'; S='x = symbol("x"); expand((x+1)^12)'; A=@('--print-budget','6') },
  @{ N='r3_noexp_budget';  S='x = symbol("x"); expand((x+1)^12)'; A=@() },
  @{ N='r3_timing';        S='x = symbol("x"); solve_full(x^8 - 3*x^5 + x - 1 == 0, x)'; A=@() },
  @{ N='r3_cancel_40';     S='x = symbol("x"); solve_full(x^8 - 3*x^5 + x - 1 == 0, x)'; A=@('--cancel-after','40') },
  @{ N='r3_cancel_0';      S='x = symbol("x"); solve_full(x^8 - 3*x^5 + x - 1 == 0, x)'; A=@('--cancel-after','1') }
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