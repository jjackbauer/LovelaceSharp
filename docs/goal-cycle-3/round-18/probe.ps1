Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-3\round-18'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = @(
  @{N='probeA_neg_quad';  S='x = symbol("x"); expand(factor(-x^2 + 1))'; T=20},
  @{N='probeB_neg_cubic'; S='x = symbol("x"); expand(factor(-6*x^3 + 6*x))'; T=20},
  @{N='control_pos';      S='x = symbol("x"); factor(x^2 - 1)'; T=20},
  @{N='hang_neg_perfect'; S='x = symbol("x"); expand(factor(-x^2 - 2*x - 1))'; T=15}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $outFile = Join-Path $d ($c.N + '.out.txt')
  $errFile = Join-Path $d ($c.N + '.err.txt')
  $p = Start-Process -FilePath 'dotnet' -ArgumentList @($dll, '--file', $f, '--omit-functions') -NoNewWindow -PassThru -RedirectStandardOutput $outFile -RedirectStandardError $errFile
  $exited = $p.WaitForExit($c.T * 1000)
  if (-not $exited) {
    try { $p.Kill() } catch {}
    Write-Output ('{0,-20} -> HANG (killed after {1}s)' -f $c.N, $c.T)
    continue
  }
  $raw = [string]::Join([char]10, (Get-Content $outFile -ErrorAction SilentlyContinue))
  try {
    $o = $raw | ConvertFrom-Json
    if ($o.ok) { Write-Output ('{0,-20} -> {1}' -f $c.N, $o.result.structured.pretty) }
    else { Write-Output ('{0,-20} -> ERR {1}' -f $c.N, $o.message) }
  } catch {
    Write-Output ('{0,-20} -> NOJSON: {1}' -f $c.N, ($raw.Trim()))
  }
}
