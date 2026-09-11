Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error CS|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc=New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-36\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll=(Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases=@(@{N='xoverx';S='x = symbol("x"); cancel(x/x)'},@{N='sq';S='x = symbol("x"); cancel((x^2-1)/(x-1))'},@{N='pow';S='x = symbol("x"); cancel((x-1)^2/(x-1))'},@{N='full';S='x = symbol("x"); cancel_full((x^2-1)/(x-1))'})
foreach ($c in $cases) {
  $f=Join-Path $d ($c.N+'.ls'); [System.IO.File]::WriteAllText($f,$c.S,$enc)
  $out=[string]::Join([char]10,(& dotnet $dll --file $f --omit-functions))
  try{$o=$out|ConvertFrom-Json; if($o.ok){$s=$o.result.structured; if($s.type -eq 'CancelResult'){$f2=@{}; foreach($y in $s.fields){$f2[$y.name]=$y.value}; Write-Output ('{0,-8} CancelResult status={1} expr={2} conditions.kind={3}' -f $c.N,$f2['status'].value,$f2['expression'].pretty,$f2['conditions'].kind)}else{Write-Output ('{0,-8} -> {1}' -f $c.N,$s.pretty)}}else{Write-Output ('{0,-8} -> {1}: {2}' -f $c.N,$o.code,$o.message)}}catch{Write-Output ('{0,-8} NOJSON' -f $c.N)} }
Write-Output '=== suites ==='
& dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
& dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo 2>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }