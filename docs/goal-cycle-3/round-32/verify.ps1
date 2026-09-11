$ErrorActionPreference = 'Continue'
Set-Location C:\Users\ricar\dev\LovelaceSharp
Write-Output "=== dotnet build LovelaceSharp.slnx -c Release --nologo ==="
dotnet build LovelaceSharp.slnx -c Release --nologo 2>&1 | Select-Object -Last 8
Write-Output "BUILD_EXIT=$LASTEXITCODE"
foreach ($p in @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Run.Tests')) {
  Write-Output "=== dotnet test $p -c Release --nologo ==="
  dotnet test "$p/$p.csproj" -c Release --nologo 2>&1 | Select-String -Pattern 'Passed!|Failed!|error' | Select-Object -Last 6
  Write-Output "TEST_EXIT=$LASTEXITCODE"
}
Write-Output "=== dotnet publish Lovelace.Run -c Release -p:PublishAot=true ==="
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot 2>&1 | Select-Object -Last 6
Write-Output "PUBLISH_EXIT=$LASTEXITCODE"
Write-Output "=== published binary: 2^1000000000 --cancel-after 2000 ==="
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$out = & ./out/aot/Lovelace.Run.exe --eval "2^1000000000" --cancel-after 2000 2>&1 | Out-String
$code = $LASTEXITCODE
$sw.Stop()
Write-Output ("WALL_MS=" + $sw.ElapsedMilliseconds)
Write-Output ("PROCESS_EXIT=" + $code)
Write-Output "ENVELOPE_START"
Write-Output $out
Write-Output "ENVELOPE_END"
Write-Output "=== hot path (same expressions as the pre-fix baseline) ==="
foreach ($expr in @("2^2000000")) {
  $sw2 = [System.Diagnostics.Stopwatch]::StartNew()
  $o2 = & ./out/aot/Lovelace.Run.exe --eval $expr 2>&1 | Out-String
  $sw2.Stop()
  Write-Output ("HOTPATH expr={0} wall={1} ms exit={2}" -f $expr, $sw2.ElapsedMilliseconds, $LASTEXITCODE)
}
Write-Output "DONE"
