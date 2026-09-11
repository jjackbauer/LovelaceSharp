Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-21\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @('2^-1','3^-2','asinh(0)','acosh(1)','atanh(0)','0^-1','sqrt(-1)','(-1)^(1/2)')
foreach ($c in $cases) {
  $n = ($c -replace '[^A-Za-z0-9]','_')
  $f = Join-Path $d ($n + '.ls')
  [System.IO.File]::WriteAllText($f, $c, $enc)
  $out = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))
  try { $o = $out | ConvertFrom-Json; if ($o.ok) { $s=$o.result.structured; $note= if ($s.pretty) { $s.pretty } else { $s.value }; if ($s.numerator) { $note = $note + ' exact=' + $s.numerator + '/' + $s.denominator }; Write-Output ('{0,-14} -> {1}' -f $c, $note) } else { Write-Output ('{0,-14} -> {1}/{2}: {3}' -f $c, $o.code, $o.category, $o.message) } } catch { Write-Output ('{0,-14} -> NOJSON' -f $c) } }