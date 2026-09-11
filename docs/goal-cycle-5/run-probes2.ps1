$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\probes\pre2'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$exe = 'out\aot\Lovelace.Run.exe'
$probes = [ordered]@{}
$probes['a01-solve-only'] = 'x = symbol("x"); solve_full(sqrt(x) + 2 == 0, x)'
$probes['a02-subs-root'] = 'x = symbol("x"); subs(sqrt(x) + 2, x, 4)'
$probes['a03-pow-neg100000'] = '2^-100000'
$probes['a04-inspect-neg100000'] = 'inspect(2^-100000)'
$probes['a05-pow-neg5000'] = '2^-5000'
$probes['a06-pow-ten-neg1001'] = '10^-1001'
$probes['a07-zero-pow-realneg1'] = '0^(-1.0)'
$probes['a08-zero-pow-intneg1'] = '0^-1'
$probes['a09-recip17-times17'] = '(1/17)*17'
$probes['a10-inspect-recip'] = 'inspect((1/17)*17)'
$probes['a11-eq-one'] = 'x = (1/17)*17; x == 1'
$probes['a12-recip17'] = '1/17'
$probes['a13-i-parse'] = 'i'
$probes['a14-solve-complex-print'] = 'x = symbol("x"); solve_full(x^2 + 1 == 0, x)'
$probes['a15-solve-cubic-print'] = 'x = symbol("x"); solve_full(x^3 - 2 == 0, x)'
$probes['a16-abs-i'] = 'x = symbol("x"); abs(i)'
$probes['a17-re-im-conj'] = 'x = symbol("x"); re(i); im(i)'
$probes['a18-capabilities'] = 'capabilities()'
$probes['a19-two-half'] = '2^(1/2)'
$probes['a20-halfrac'] = '1/2*sqrt(8)'
$probes['a21-free-symbols'] = 'x = symbol("x"); inspect(x*y)'
# depth shapes
$shapes = [ordered]@{}
$shapes['d-paren']   = { param($n) ('(' * $n) + '1' + (')' * $n) }
$shapes['d-unary']   = { param($n) ('-' * $n) + '1' }
$shapes['d-call']    = { param($n) ('abs(' * $n) + '1' + (')' * $n) }
$shapes['d-addleft'] = { param($n) ('(' * $n) + '1' + ('+1)' * $n) }
$shapes['d-array']   = { param($n) ('[' * $n) + '1' + (']' * $n) }
$summary = @()
foreach ($k in $probes.Keys) {
  $f = Join-Path $out ($k + '.ls')
  [System.IO.File]::WriteAllText($f, $probes[$k], $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $out ($k + '.out.txt')), $raw, $enc)
  $summary += ('{0,-26} exit={1,-12} bytes={2}' -f $k, $code, $raw.Length)
}
foreach ($s in $shapes.Keys) {
  foreach ($n in @(1000, 2000, 3000)) {
    $name = ('{0}-{1}' -f $s, $n)
    $body = & $shapes[$s] $n
    $f = Join-Path $out ($name + '.ls')
    [System.IO.File]::WriteAllText($f, $body, $enc)
    $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
    $code = $LASTEXITCODE
    [System.IO.File]::WriteAllText((Join-Path $out ($name + '.out.txt')), $raw, $enc)
    $summary += ('{0,-26} exit={1,-12} bytes={2}' -f $name, $code, $raw.Length)
  }
}
$summary | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path $out '_summary.txt'), ($summary -join ([char]10)), $enc)