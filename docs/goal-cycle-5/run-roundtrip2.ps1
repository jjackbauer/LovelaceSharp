$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\probes\roundtrip'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$exe = 'out\aot\Lovelace.Run.exe'
$useAot = (Test-Path $exe) -and ($env:RT_USE_AOT -eq '1')
$prelude = 'x = symbol("x"); y = symbol("y"); z = symbol("z"); w = symbol("w"); a = symbol("a");'
$srcs = @(
  '(-1)^x', '(-2)^x', '(-1/2)^x', '(-1.5)^x', '(-x)^2', '(-x - 1)^2', '(-3)^(-x)',
  '(1/2)^x', '(2/3)^x', '(-3/2)^x',
  '(x/y)/z', 'x/y/z', 'x/y/z/w', '(x/y)/(z/w)', 'x/(y*z)', '(x*y)/(z*w)', '(x/y)*z', 'x*y/z', 'x/y/x',
  'x - (y - a)', '-(x + y)', 'x^(-2)', '(x + 1)^(-1)', 'sqrt(x + y)',
  'x^y^2', '(x^y)^2', '-x^2', 'exp(-x^2)', '1/(2*(x + 1))', 'apart(1/(x^2 - 1), x)'
)
function Run([string]$text) {
  $f = Join-Path $out '_tmp2.ls'
  [System.IO.File]::WriteAllText($f, $prelude + $text, $enc)
  if ($script:useAot) { $raw = [string]::Join([char]10, (& $script:exe --file $f --omit-functions 2>&1)) }
  else { $raw = [string]::Join([char]10, (& dotnet run --project Lovelace.Run -c Release --no-build -- --file $f --omit-functions 2>&1)) }
  $code = $LASTEXITCODE
  try { $j = $raw | ConvertFrom-Json } catch { return @{ ok = $false; code = 'ENVELOPE'; pretty = ''; canonical = '' } }
  if ($j.ok -ne $true) { return @{ ok = $false; code = $j.code + '/' + $j.category; pretty = ''; canonical = '' } }
  $s = $j.result.structured
  return @{ ok = $true; code = ''; pretty = [string]$s.pretty; canonical = [string]$s.canonical; kind = [string]$s.kind }
}
$rows = @(); $ok = 0; $bad = 0; $other = 0
foreach ($s in $srcs) {
  $one = Run $s
  if (-not $one.ok) { $rows += ('{0,-20} | FIRST-RUN-ERROR {1}' -f $s, $one.code); $other++; continue }
  if ($one.kind -ne 'Symbolic') { $rows += ('{0,-20} | NOT-SYMBOLIC kind={1} pretty={2}' -f $s, $one.kind, $one.pretty); $other++; continue }
  $two = Run $one.pretty
  if (-not $two.ok) { $rows += ('{0,-20} | REPARSE-ERROR {1} | pretty={2}' -f $s, $two.code, $one.pretty); $bad++; continue }
  if ($one.canonical -eq $two.canonical) { $rows += ('{0,-20} | OK | {1}' -f $s, $one.pretty); $ok++ }
  else { $rows += ('{0,-20} | DIFF | pretty={1} | c1={2} | c2={3}' -f $s, $one.pretty, $one.canonical, $two.canonical); $bad++ }
}
$rows | ForEach-Object { Write-Output $_ }
$sum = ('SUMMARY ok={0} bad={1} other={2} of {3}; source={4}' -f $ok, $bad, $other, $srcs.Count, $(if ($useAot) { 'AOT published binary' } else { 'JIT dotnet run' }))
Write-Output $sum
[System.IO.File]::WriteAllText((Join-Path $out 'roundtrip-post.txt'), (($rows + $sum) -join ([char]10)), $enc)