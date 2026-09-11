$ErrorActionPreference='Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$d = 'docs\goal-cycle-5\probes\r10r11'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$cases = @(
  @('pow-neg100000','2^-100000'),
  @('zero-pow-realneg1','0^(-1.0)'),
  @('trunc-1-over-1009','setprecision(18); 1/1009'),
  @('exact-1-over-1e19','1/(10^19)'),
  @('error-envelope','x = symbol("x"); solve(x^2 + 1 == 0, x, integer)'),
  @('arity-abs','abs(1,2)'),
  @('arity-mean','mean(1)'),
  @('arity-min','min()'),
  @('arity-sum','sum(1,2,3)'),
  @('print-cr','print("A"); print("B")'),
  @('solve-empty','x = symbol("x"); solve(0 == 1, x)'),
  @('solvefull-empty','x = symbol("x"); solve_full(2 == 3, x)')
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c[0] + '.ls')
  [System.IO.File]::WriteAllText($f, $c[1], $enc)
  $o = & dotnet run --project Lovelace.Run -c Release -- --file (Resolve-Path $f) --omit-functions 2>&1 | Out-String
  $j = $null; try { $j = $o | ConvertFrom-Json } catch { $c[0] + ' : ENVELOPE-PARSE-FAIL'; continue }
  if (-not $j.ok) { '{0,-20} ERR {1}/{2} recoverable={3} :: {4}' -f $c[0], $j.code, $j.category, $j.recoverable, ([string]$j.message).Substring(0,[Math]::Min(70,([string]$j.message).Length)); if ($c[0] -eq 'error-envelope') { '   elapsedTime=' + ($j.elapsedTime | ConvertTo-Json -Compress) + ' timings=' + ($j.timings | Measure-Object).Count }; continue }
  $st = $j.result.structured
  $extra = ''
  if ($st.exact -ne $null) { $extra += ' exact=' + $st.exact }
  if ($st.numerator) { $extra += ' num=' + ([string]$st.numerator).Substring(0,[Math]::Min(12,([string]$st.numerator).Length)) + ' denLen=' + ([string]$st.denominator).Length }
  if ($c[0] -eq 'print-cr') { $extra += ' output=' + ([string]::Join('|', $j.output)) }
  if ($st.type -eq 'SolveResult') { foreach ($x in $st.fields) { if ($x.name -eq 'status') { $extra += ' status=' + $x.value.value } } }
  '{0,-20} {1}{2}' -f $c[0], ([string]$j.result.display).Substring(0,[Math]::Min(46,([string]$j.result.display).Length)), $extra
}