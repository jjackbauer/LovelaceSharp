$root='C:\Users\ricar\dev\LovelaceSharp'; Set-Location $root
$out = Join-Path $root 'docs\goal-cycle-6\final'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = '1'
$env:RT_USE_AOT = '1'
$lines = @()
function Say($s) { $script:lines += $s; Write-Output $s }

Say ("=== forced rebuild " + (Get-Date -Format o) + " ===")
$b = & dotnet build LovelaceSharp.slnx --configuration Release --no-incremental --nologo 2>&1 | Out-String
Say ("rebuild exit=" + $LASTEXITCODE + " warnings=" + ([regex]::Matches($b, ': warning ')).Count + " errors=" + ([regex]::Matches($b, ': error ')).Count)

Say "=== full sweep (LOVELACE_REQUIRE_SYMPY=1) ==="
& powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-cycle-5\sweep.ps1 2>&1 | ForEach-Object { Say $_ }

Say "=== AOT publish + freshness + five smoke scenarios ==="
& powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-cycle-5\final-aot.ps1 2>&1 | ForEach-Object { Say $_ }

Say "=== capability honesty ==="
& powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-cycle-5\verify-capabilities.ps1 2>&1 | ForEach-Object { Say $_ }

Say "=== printer round-trip through the published binary ==="
& powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-cycle-5\run-roundtrip2.ps1 2>&1 | ForEach-Object { Say $_ }

# also run the Real.Tests full suite including the Heavy cases
Say "=== Lovelace.Real.Tests, unfiltered (Heavy included) ==="
$rt = & dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj --configuration Release --nologo 2>&1 | Out-String
Say ("Real unrestricted exit=" + $LASTEXITCODE + " :: " + (($rt -split "`n" | Where-Object { $_ -match 'Passed!|Failed!' }) -join ' '))

$lines | Out-File -FilePath (Join-Path $out 'final.txt') -Encoding utf8
Say 'D4_DONE'
