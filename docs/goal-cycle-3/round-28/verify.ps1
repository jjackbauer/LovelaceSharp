Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-28\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases = @(@{N='half';S='latex(1/2)'},@{N='pow';S='x = symbol("x"); latex(x^2)'},@{N='sqrt';S='x = symbol("x"); latex(sqrt(x))'},@{N='sum2';S='x = symbol("x"); latex((x+1)^2)'},@{N='sub';S='x = symbol("x"); y = symbol("y"); a = symbol("a"); latex(x-(y-a))'})
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $out = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))
  try { $o = $out | ConvertFrom-Json; if ($o.ok) { Write-Output ('{0,-6} -> {1}' -f $c.N, $o.result.structured.value) } else { Write-Output ('{0,-6} -> {1}: {2}' -f $c.N, $o.code, $o.message) } } catch { Write-Output ('{0,-6} -> NOJSON' -f $c.N) } }
Write-Output '=== Pretty/Canonical regression: goldens + Run.Tests ==='
& dotnet test 'Lovelace.Run.Tests/Lovelace.Run.Tests.csproj' -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }