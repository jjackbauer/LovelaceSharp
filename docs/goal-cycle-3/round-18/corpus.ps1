Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-3\round-18\corpus'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$corpus = @(
 '-x^2 + 1','-x^3 + x','-x^4 + 5*x^2 - 4','-2*x^2 + 8','-6*x^3 + 6*x','-x^2 + 2',
 '-x^3 - x^2 + x + 1','-x^2 - 2*x - 1','-3*x^2 + 3',
 'x^2 - 1','x^3 + 1','x^4 - 5*x^2 + 4','2*x^2 - 8','6*x^3 - 6*x','x^2 - 2',
 'x^3 + x^2 - x - 1','x^3 - 3*x^2 + 3*x - 1','x^4 - 1')
$i = 0
$pass = 0; $fail = 0; $hang = 0
foreach ($p in $corpus) {
  $i++
  $safe = ($p -replace '[^A-Za-z0-9]', '_')
  $name = '{0:d2}_{1}' -f $i, $safe
  $src = 'x = symbol("x"); expand(expand(factor(' + $p + ')) - expand(' + $p + '))'
  $f = Join-Path $d ($name + '.ls')
  [System.IO.File]::WriteAllText($f, $src, $enc)
  $outFile = Join-Path $d ($name + '.out.txt')
  $p2 = Start-Process -FilePath 'dotnet' -ArgumentList @($dll, '--file', $f, '--omit-functions') -NoNewWindow -PassThru -RedirectStandardOutput $outFile -RedirectStandardError (Join-Path $d ($name + '.err.txt'))
  if (-not $p2.WaitForExit(15000)) {
    try { $p2.Kill() } catch {}
    Write-Output ('{0,-32} HANG   factor({1})' -f $name, $p)
    $hang++
    continue
  }
  $raw = [string]::Join([char]10, (Get-Content $outFile -ErrorAction SilentlyContinue))
  try {
    $o = $raw | ConvertFrom-Json
    if (-not $o.ok) { Write-Output ('{0,-32} ERROR  {1}' -f $name, $o.message); $fail++; continue }
    $pretty = $o.result.structured.pretty
    if ($pretty -eq '0') { Write-Output ('{0,-32} PASS   factor({1}) ok' -f $name, $p); $pass++ }
    else { Write-Output ('{0,-32} FAIL   factor({1}) off by {2}' -f $name, $p, $pretty); $fail++ }
  } catch {
    Write-Output ('{0,-32} NOJSON {1}' -f $name, ($raw.Trim()))
    $fail++
  }
}
Write-Output ''
Write-Output ("TOTAL {0}: pass={1} fail={2} hang={3}" -f $corpus.Count, $pass, $fail, $hang)
