$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 3 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-13'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$cases = @(@{N='neg_quad';S='x = symbol("x"); expand(factor(-x^2 + 1))'}, @{N='neg_cubic';S='x = symbol("x"); expand(factor(-x^3 + x))'}, @{N='control';S='x = symbol("x"); expand(factor(x^2 - 1))'}, @{N='neg_factor';S='x = symbol("x"); factor(-x^2 + 1)'})
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $f --omit-functions))
  $o = $raw | ConvertFrom-Json
  if ($o.ok) { Write-Output ('{0,-12} -> {1}' -f $c.N, $o.result.structured.pretty) } else { Write-Output ('{0,-12} -> ERR {1}' -f $c.N, $o.message) } }
& dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }