Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$exe=(Resolve-Path 'out\aot\Lovelace.Run.exe').Path
$src=Get-ChildItem -Recurse -Include *.cs -File | Where-Object { $_.FullName -notmatch '\\(obj|bin|out)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Output ('aot STALE=' + ((Get-Item $exe).LastWriteTime -lt $src.LastWriteTime))
$enc=New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-32\vv'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$f=Join-Path $d 'cancel.ls'; [System.IO.File]::WriteAllText($f,'2^1000000000',$enc)
$psi=New-Object System.Diagnostics.ProcessStartInfo; $psi.FileName=$exe
$psi.Arguments='--file "'+$f+'" --omit-functions --cancel-after 2000'
$psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true; $psi.UseShellExecute=$false
$sw=[System.Diagnostics.Stopwatch]::StartNew(); $p=[System.Diagnostics.Process]::Start($psi); $o=$p.StandardOutput.ReadToEndAsync()
if(-not $p.WaitForExit(60000)){ $p.Kill(); Write-Output 'RESULT: STILL RUNNING AT 60000 ms - NOT FIXED' } else { $sw.Stop(); $t=$o.Result; try{$j=$t|ConvertFrom-Json; Write-Output ('RESULT: wall=' + [int]$sw.Elapsed.TotalMilliseconds + 'ms exit=' + $p.ExitCode + ' code=' + $j.code + ' category=' + $j.category + ' recoverable=' + $j.recoverable)}catch{ Write-Output ('RESULT: wall=' + [int]$sw.Elapsed.TotalMilliseconds + 'ms exit=' + $p.ExitCode) } }
$f2=Join-Path $d 'hot.ls'; [System.IO.File]::WriteAllText($f2,'2^2000000',$enc)
$sw2=[System.Diagnostics.Stopwatch]::StartNew(); $null=[string]::Join([char]10,(& $exe --file $f2 --omit-functions)); $sw2.Stop()
Write-Output ('hot path 2^2000000 wall=' + [int]$sw2.Elapsed.TotalMilliseconds + 'ms')