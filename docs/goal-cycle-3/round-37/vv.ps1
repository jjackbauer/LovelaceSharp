Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$o = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
Write-Output ('warnings=' + ([regex]::Matches($o,'warning [A-Za-z]+\d+')).Count + ' errors=' + ([regex]::Matches($o,': error ')).Count)
$enc=New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-37\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll=(Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
foreach ($n in @('x_1','p%q','a&b','h#k','br{ce','back\\slash','car^et')) {
  $f=Join-Path $d (($n -replace '[^A-Za-z0-9]','_')+'.ls')
  [System.IO.File]::WriteAllText($f, ('s = symbol("' + $n.Replace('\','\\') + '"); latex(s)'), $enc)
  $out=[string]::Join([char]10,(& dotnet $dll --file $f --omit-functions))
  try{$j=$out|ConvertFrom-Json; if($j.ok){Write-Output ('{0,-12} -> {1}' -f $n,$j.result.structured.value)}else{Write-Output ('{0,-12} -> {1}: {2}' -f $n,$j.code,$j.message)}}catch{Write-Output ('{0,-12} -> NOJSON' -f $n)} }