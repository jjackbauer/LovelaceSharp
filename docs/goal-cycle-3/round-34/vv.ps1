Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error CS|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc=New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-34\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll=(Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases=@(
 @{N='squeeze';S='x = symbol("x"); limit_full(x*sin(1/x), x, 0)'},
 @{N='sinc';S='x = symbol("x"); limit_full(sin(x)/x, x, 0)'},
 @{N='dne';S='x = symbol("x"); limit_full(1/x, x, 0)'},
 @{N='absx';S='x = symbol("x"); limit_full(abs(x)/x, x, 0)'}
)
foreach ($c in $cases) {
  $f=Join-Path $d ($c.N+'.ls'); [System.IO.File]::WriteAllText($f,$c.S,$enc)
  $out=[string]::Join([char]10,(& dotnet $dll --file $f --omit-functions))
  try{$o=$out|ConvertFrom-Json; $s=$o.result.structured; $f2=@{}; foreach($y in $s.fields){$f2[$y.name]=$y.value}
    Write-Output ('{0,-8} status={1,-12} exists.kind={2,-6} exists.value={3}' -f $c.N,$f2['status'].value,$f2['exists'].kind,$f2['exists'].value) }catch{Write-Output ('{0,-8} NOJSON' -f $c.N)} }