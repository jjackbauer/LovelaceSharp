$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\probes\roundtrip'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$exe = 'out\aot\Lovelace.Run.exe'
$prelude = 'x = symbol("x"); y = symbol("y"); z = symbol("z"); a = symbol("a");'
$srcs = @(
  '(-1)^x', '(-2)^x', '(-1/2)^x', '(-1.5)^x', '(-x)^2', '(-x - 1)^2',
  '(x/y)/z', 'x/(y*z)', 'x/y/z', '(x/y)*z', 'x*y/z',
  'x - (y - a)', '-(x + y)', 'x^(-2)', '(x + 1)^(-1)', 'sqrt(x + y)',
  '1/2*sqrt(8)', '2^(1/3)', 'sqrt(-1)', 'sqrt(-4)', 'abs(sqrt(-1))',
  'x^y^2', '(x^y)^2', '-x^2', 'exp(-x^2)', '1/(2*(x + 1))'
)
function Run([string]$text) {
  $f = Join-Path $out '_tmp.ls'
  [System.IO.File]::WriteAllText($f, $prelude + $text, $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  $code = $LASTEXITCODE
  try { $j = $raw | ConvertFrom-Json } catch { return @{ ok = $false; code = 'ENVELOPE'; pretty = ''; canonical = ''; exit = $code } }
  if ($j.ok -ne $true) { return @{ ok = $false; code = $j.code + '/' + $j.category; pretty = ''; canonical = ''; exit = $code } }
  $s = $j.result.structured
  return @{ ok = $true; code = ''; pretty = [string]$s.pretty; canonical = [string]$s.canonical; kind = [string]$s.kind; exit = $code }
}
$rows = @()
foreach ($s in $srcs) {
  $one = Run $s
  if (-not $one.ok) { $rows += ('{0,-18} | FIRST-RUN-ERROR {1}' -f $s, $one.code); continue }
  if ($one.kind -ne 'Symbolic') { $rows += ('{0,-18} | NOT-SYMBOLIC kind={1} pretty={2}' -f $s, $one.kind, $one.pretty); continue }
  $two = Run $one.pretty
  if (-not $two.ok) { $rows += ('{0,-18} | REPARSE-ERROR {1} | pretty={2}' -f $s, $two.code, $one.pretty); continue }
  $verdict = if ($one.canonical -eq $two.canonical) { 'ROUNDTRIP-OK' } else { 'ROUNDTRIP-DIFF' }
  $rows += ('{0,-18} | {1} | pretty={2} | c1={3} | c2={4}' -f $s, $verdict, $one.pretty, $one.canonical, $two.canonical)
}
$rows | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path $out 'roundtrip-pre.txt'), ($rows -join ([char]10)), $enc)