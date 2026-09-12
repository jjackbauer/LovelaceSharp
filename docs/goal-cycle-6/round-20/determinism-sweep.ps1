param(
  [string]$Exe = "C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe",
  [int]$Repeats = 3
)
# Determinism sweep. Scripts go through --file, NEVER through --eval: Windows PowerShell 5.1 strips
# embedded double quotes from a native command's arguments, so --eval 'symbol("x")' silently becomes
# symbol(x) and the script fails for the wrong reason (EVD-299). A file cannot be mangled that way.
$ErrorActionPreference = 'Continue'
$corpus = @(
  'pi(30)', 'e(20)', 'sqrt(2)', '1/3', '2^1000', 'factor(123456789)',
  'sin(1)', 'cos(pi(30)/3)', 'tan(1)', 'evalf(sin(1), 40)', 'evalf(1/7, 100)',
  'setprecision(50); pi(50)', 'setprecision(1100); pi(1100)',
  'x = symbol("x"); solve(x^2 - 2 == 0, x)',
  'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)',
  'x = symbol("x"); series(sin(x), x, 0, 5)',
  'x = symbol("x"); series(exp(x), x, 0, 5)',
  'x = symbol("x"); series(abs(x), x, 0, 3)',
  'x = symbol("x"); limit(sin(x)/x, x, 0)',
  'x = symbol("x"); diff(x^3, x)',
  'x = symbol("x"); integrate(x^2, x)',
  'x = symbol("x"); expand((x + 1)^5)',
  'x = symbol("x"); factor(x^4 - 1)',
  'x = symbol("x"); inspect(sin(x))',
  '[1, 2, 3] + [4, 5, 6]', 'det([[1, 2], [3, 4]])', 'sum(1..1000)', 'mean([1, 2, 3, 4])',
  'sort([3, 1, 2])', 'sqrt(-1)', 'exp(i*pi)', 'print("hello")',
  'det(1)', 'mean(1)', '1/0', 'abs(1, 2)', 'nosuchfn(1)', 'i_do_not_exist'
)
$tmp = Join-Path $env:TEMP ("det-sweep-" + [Guid]::NewGuid().ToString("N") + ".ls")
$results = @()
foreach ($s in $corpus) {
  Set-Content -LiteralPath $tmp -Value $s -Encoding UTF8
  $forms = @()
  for ($i = 0; $i -lt $Repeats; $i++) {
    $raw = & $Exe --file $tmp --json --omit-functions --omit-variables 2>&1 | Out-String
    $line = ($raw -split "\n" | Where-Object { $_.Trim().StartsWith('{') } | Select-Object -First 1)
    if (-not $line) { $forms += "NO-ENVELOPE"; continue }
    try {
      $o = $line | ConvertFrom-Json
      foreach ($p in @('elapsed', 'elapsedTime', 'timings')) { $o.PSObject.Properties.Remove($p) }
      $forms += ($o | ConvertTo-Json -Depth 100 -Compress)
    } catch { $forms += "UNPARSEABLE" }
  }
  $distinct = ($forms | Sort-Object -Unique).Count
  $results += [pscustomobject]@{ Script = $s; Distinct = $distinct; Stable = ($distinct -eq 1) }
}
Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
$bad = @($results | Where-Object { -not $_.Stable })
Write-Output ("DETERMINISM SWEEP (--file): {0} scripts x {1} runs; unstable = {2}" -f $corpus.Count, $Repeats, $bad.Count)
foreach ($b in $bad) { Write-Output ("  UNSTABLE ({0} distinct)  {1}" -f $b.Distinct, $b.Script) }
$stable = @($results | Where-Object { $_.Stable }).Count
Write-Output ("stable = {0}/{1}" -f $stable, $corpus.Count)
