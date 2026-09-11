$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\probes\pre'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$exe = 'out\aot\Lovelace.Run.exe'
$probes = [ordered]@{}
$probes['t01-solver-completeness'] = 'x = symbol("x"); solve_full(sqrt(x) + 2 == 0, x); subs(sqrt(x) + 2, x, 4)'
$probes['t02-printer-negbase'] = 'x = symbol("x"); e = (-1)^x; e; subs(e, x, 2); subs(-1^x, x, 2); (-2)^x; subs((-2)^x, x, 2)'
$probes['t03-exact-lies'] = '2^-100000; 2^-5000; 10^-1001; inspect(2^-100000); 0^(-1.0); 0^-1'
$probes['t04-roundtrip'] = '(1/17)*17; x = (1/17)*17; x == 1; inspect((1/17)*17)'
$probes['t05-solve-system'] = 'x = symbol("x"); y = symbol("y"); solve_system(x + y == 2, x - y == 0)'
$probes['t05b-solve-system-list'] = 'x = symbol("x"); y = symbol("y"); solve_system([x + y == 2, x - y == 0], [x, y])'
$probes['t06-depth-3000'] = ('(' * 3000) + '1' + (')' * 3000)
$probes['t06b-depth-2000'] = ('(' * 2000) + '1' + (')' * 2000)
$probes['t1-arity'] = 'sin(1,2)'
$probes['t1-arity-abs'] = 'abs(1,2)'
$probes['t1-arity-det'] = 'det()'
$probes['t1-arity-len'] = 'len(1,2)'
$probes['t1-arity-min'] = 'min()'
$probes['t1-arity-sum3'] = 'sum(1,2,3)'
$probes['t1-arity-mean1'] = 'mean(1)'
$probes['t1-arity-max'] = 'max(1,2)'
$probes['t1-arity-sumx'] = 'x = symbol("x"); sum(x,5)'
$probes['t1-print-cr'] = 'print("A"); print("B")'
$probes['t2-parse-i'] = 'i'
$probes['t2-print-i'] = 'solve_full(x^2 + 1 == 0, x)'
$probes['t2-parse-2pow13'] = 'solve_full(x^3 - 2 == 0, x)'
$probes['t2-evalf-digits'] = 'evalf(sqrt(2),5)'
$probes['t2-abs-i'] = 'abs(i)'
$probes['t2-reimconj'] = 're(i); im(i); conj(i)'
$probes['t2-parse-fail'] = '1 +;'
$probes['t1-envelope'] = 'x = symbol("x"); solve(x^2 + 1 == 0, x, integer)'
$summary = @()
foreach ($k in $probes.Keys) {
  $f = Join-Path $out ($k + '.ls')
  [System.IO.File]::WriteAllText($f, $probes[$k], $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $out ($k + '.out.txt')), $raw, $enc)
  $len = $raw.Length
  $summary += ('{0,-24} exit={1,-12} bytes={2}' -f $k, $code, $len)
}
$summary | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path $out '_summary.txt'), ($summary -join ([char]10)), $enc)