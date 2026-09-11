Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-13\bisect'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @('-x^2 + 1','-x^3 + x','-x^4 + 5*x^2 - 4','-6*x^3 + 6*x','-x^3 - x^2 + x + 1','-x^2 - 2*x - 1','x^4 - 5*x^2 + 4','x^3 - 3*x^2 + 3*x - 1')
foreach ($c in $cases) {
  $n = ($c -replace '[^A-Za-z0-9]','_')
  $f = Join-Path $d ($n + '.ls')
  [System.IO.File]::WriteAllText($f, ('x = symbol("x"); factor(' + $c + ')'), $enc)
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = 'dotnet'
  $psi.Arguments = ('"' + $dll + '" --file "' + $f + '" --omit-functions')
  $psi.RedirectStandardOutput = $true; $psi.RedirectStandardError = $true; $psi.UseShellExecute = $false
  $p = [System.Diagnostics.Process]::Start($psi)
  $out = $p.StandardOutput.ReadToEndAsync()
  $done = $p.WaitForExit(15000)
  if (-not $done) { $p.Kill(); Write-Output ('{0,-24} HANG (>15s)' -f $c) } else { $txt = $out.Result; $pretty=''; try { $pretty = ($txt | ConvertFrom-Json).result.structured.pretty } catch { $pretty='PARSE?' }; Write-Output ('{0,-24} exit={1} -> {2}' -f $c, $p.ExitCode, $pretty) } }