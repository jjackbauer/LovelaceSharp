Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error CS|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc=New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-33\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll=(Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$cases=@('inf - inf','0 * inf','inf + 1','x = symbol("x"); x - x','x = symbol("x"); 0 * x','inf / inf','0 / 0')
foreach ($c in $cases) {
  $n=($c -replace '[^A-Za-z0-9]','_'); $f=Join-Path $d ($n+'.ls'); [System.IO.File]::WriteAllText($f,$c,$enc)
  $out=[string]::Join([char]10,(& dotnet $dll --file $f --omit-functions))
  try{$o=$out|ConvertFrom-Json; if($o.ok){$s=$o.result.structured; $v= if($s.pretty){$s.pretty}elseif($s.value){$s.value}else{$s.type}; Write-Output ('{0,-32} -> {1}' -f $c,$v)}else{Write-Output ('{0,-32} -> {1}: {2}' -f $c,$o.code,$o.message)}}catch{Write-Output ('{0,-32} -> NOJSON' -f $c)} }