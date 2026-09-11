Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-3\round-18\extra'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = @(
  @{N='e1_int_content'; S='x = symbol("x"); expand(expand(factor(2*x + 2)) - (2*x + 2))'},
  @{N='e2_neg_content'; S='x = symbol("x"); expand(expand(factor(-2*x - 2)) - (-2*x - 2))'},
  @{N='e3_neg_rational'; S='x = symbol("x"); expand(expand(factor(-x^2 + 1/4)) - (-x^2 + 1/4))'},
  @{N='e4_neg_quad_nonroot'; S='x = symbol("x"); expand(expand(factor(-2*x^2 + 3*x - 5)) - (-2*x^2 + 3*x - 5))'},
  @{N='e5_neg_cube_mixed'; S='x = symbol("x"); expand(expand(factor(-x^3 + 2*x^2 + x - 2)) - (-x^3 + 2*x^2 + x - 2))'},
  @{N='e6_gcd_control'; S='x = symbol("x"); gcd(x^2 - 1, x - 1)'},
  @{N='e7_print_factor'; S='x = symbol("x"); factor(-x^2 + 1)'},
  @{N='e8_neg_higher'; S='x = symbol("x"); expand(expand(factor(-x^5 + x)) - (-x^5 + x))'}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls'); [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $outFile = Join-Path $d ($c.N + '.out.txt')
  $p = Start-Process -FilePath 'dotnet' -ArgumentList @($dll, '--file', $f, '--omit-functions') -NoNewWindow -PassThru -RedirectStandardOutput $outFile -RedirectStandardError (Join-Path $d ($c.N + '.err.txt'))
  if (-not $p.WaitForExit(20000)) { try { $p.Kill() } catch {}; Write-Output ('{0,-20} HANG' -f $c.N); continue }
  $raw = [string]::Join([char]10, (Get-Content $outFile -ErrorAction SilentlyContinue))
  try { $o = $raw | ConvertFrom-Json; if ($o.ok) { Write-Output ('{0,-20} -> {1}' -f $c.N, $o.result.structured.pretty) } else { Write-Output ('{0,-20} -> ERR {1}' -f $c.N, $o.message) } }
  catch { Write-Output ('{0,-20} -> NOJSON {1}' -f $c.N, ($raw.Trim())) }
}
