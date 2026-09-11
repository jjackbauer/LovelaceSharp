Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-20\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @('-x^2 + 1','-6*x^3 + 6*x','-2*x^2 + 8','-x^4 + 5*x^2 - 4','x^2 - 1','x^3 - 3*x^2 + 3*x - 1','-x^3 - x^2 + x + 1','-x^2 - 2*x - 1')
$ok=0; $tot=0
foreach ($c in $cases) {
  $tot++
  $n = ($c -replace '[^A-Za-z0-9]','_')
  $f = Join-Path $d ($n + '.ls')
  [System.IO.File]::WriteAllText($f, ('x = symbol("x"); expand(factor(' + $c + '))'), $enc)
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName='dotnet'; $psi.Arguments=('"' + $dll + '" --file "' + $f + '" --omit-functions')
  $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true; $psi.UseShellExecute=$false
  $p=[System.Diagnostics.Process]::Start($psi); $out=$p.StandardOutput.ReadToEndAsync()
  if (-not $p.WaitForExit(15000)) { $p.Kill(); Write-Output ('{0,-24} HANG' -f $c) } else { $t=$out.Result; $v='?'; try { $v=($t | ConvertFrom-Json).result.structured.pretty } catch {}; $same = ($v -eq $c); if ($same) { $ok++ }; Write-Output ('{0,-24} -> {1,-28} roundtrip={2}' -f $c, $v, $same) } }
Write-Output ('ROUNDTRIP {0}/{1}' -f $ok, $tot)