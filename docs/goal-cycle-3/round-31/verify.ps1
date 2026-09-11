Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-31\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$exe = (Resolve-Path 'out\aot\Lovelace.Run.exe').Path
foreach ($c in @('inf - inf','0 * inf','inf + 1','x = symbol("x"); limit_full(x*sin(1/x), x, 0)')) {
  $n=($c -replace '[^A-Za-z0-9]','_'); $f=Join-Path $d ($n+'.ls')
  [System.IO.File]::WriteAllText($f,$c,$enc)
  $raw=[string]::Join([char]10,(& $exe --file $f --omit-functions)); $code=$LASTEXITCODE
  try { $o=$raw|ConvertFrom-Json; if($o.ok){$s=$o.result.structured; $v= if($s.value){$s.value}elseif($s.pretty){$s.pretty}else{$s.type}; if($s.fields){ $f2=@{}; foreach($y in $s.fields){$f2[$y.name]=$y.value}; $v='status='+$f2['status'].value+' exists='+$f2['exists'].value } ; Write-Output ('{0,-34} exit={1} -> {2}' -f $c,$code,$v) } else { Write-Output ('{0,-34} exit={1} -> {2}: {3}' -f $c,$code,$o.code,$o.message) } } catch { Write-Output ('{0,-34} NOJSON' -f $c) } }
Write-Output '=== cancellation: 60s cap on a huge exponent ==='
$f=Join-Path $d 'cancel.ls'; [System.IO.File]::WriteAllText($f,'2^1000000000',$enc)
$psi=New-Object System.Diagnostics.ProcessStartInfo; $psi.FileName=$exe
$psi.Arguments='--file "'+$f+'" --omit-functions --cancel-after 2000'
$psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true; $psi.UseShellExecute=$false
$sw=[System.Diagnostics.Stopwatch]::StartNew(); $p=[System.Diagnostics.Process]::Start($psi); $o2=$p.StandardOutput.ReadToEndAsync()
if(-not $p.WaitForExit(60000)){ $p.Kill(); Write-Output ('CANCEL: still running at 60000 ms after --cancel-after 2000 -> NOT OBSERVED') } else { $sw.Stop(); $t=$o2.Result; try{$j=$t|ConvertFrom-Json; Write-Output ('CANCEL: returned in '+[int]$sw.Elapsed.TotalMilliseconds+' ms, code='+$j.code+' category='+$j.category)}catch{Write-Output ('CANCEL: returned in '+[int]$sw.Elapsed.TotalMilliseconds+' ms')} }