$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\probes\pre3'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$exe = 'out\aot\Lovelace.Run.exe'
$probes = [ordered]@{}
$probes['b01-nested-fraction'] = 'x = symbol("x"); y = symbol("y"); z = symbol("z"); e = (x/y)/z; e'
$probes['b01b-canonical'] = 'x = symbol("x"); y = symbol("y"); z = symbol("z"); inspect((x/y)/z)'
$probes['b02-cubic-print'] = 'x = symbol("x"); solve_full(x^3 - 2 == 0, x)'
$probes['b03-rootof-print'] = 'x = symbol("x"); solve_full(x^5 - x - 1 == 0, x)'
$probes['b04-sqrt-neg'] = 'sqrt(-1)'
$probes['b05-abs-i'] = 'abs(sqrt(-1))'
$probes['b06-re-im'] = 're(sqrt(-1))'
$probes['b07-im'] = 'im(sqrt(-1))'
$probes['b08-conj'] = 'conj(sqrt(-1))'
$probes['b09-free-symbols'] = 'x = symbol("x"); y = symbol("y"); inspect(x*y)'
$probes['b10-solve-partial-text'] = 'x = symbol("x"); solve(x^4 - x^2 - 1 == 0, x)'
$probes['b11-limit-text'] = 'x = symbol("x"); limit(sin(x)/x, x, 0)'
$probes['b12-limit-unevaluated'] = 'x = symbol("x"); limit(sin(x), x, inf)'
$probes['b13-capabilities'] = 'capabilities()'
$probes['b14-exact-half-sqrt8'] = '1/2*sqrt(8)'
$probes['b15-two-thirds'] = '2^(1/3)'
$probes['b16-zero-pow-real-cap'] = '2^(-1.0)'
$probes['b17-latex'] = 'x = symbol("x"); latex((-1)^x)'
$summary = @()
foreach ($k in $probes.Keys) {
  $f = Join-Path $out ($k + '.ls')
  [System.IO.File]::WriteAllText($f, $probes[$k], $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $out ($k + '.out.txt')), $raw, $enc)
  $summary += ('{0,-26} exit={1,-12} bytes={2}' -f $k, $code, $raw.Length)
}
$summary | ForEach-Object { Write-Output $_ }