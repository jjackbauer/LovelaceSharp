$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\final'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$lines = @()
$lines += 'final verification started ' + (Get-Date -Format o) + ' HEAD ' + (git rev-parse --short HEAD)
Get-Process testhost -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3
$lines += '=== git status (must show no source file modified) ==='
$lines += (git status --porcelain)
$lines += '=== forced full rebuild ==='
$b = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'forced-rebuild.log'), $b, $enc)
$lines += 'exit=' + $LASTEXITCODE + ' warninglines=' + ([regex]::Matches($b,': warning ')).Count + ' errorlines=' + ([regex]::Matches($b,': error ')).Count
$lines += ($b -split ([char]10) | Select-String -Pattern 'Warning\(s\)|Error\(s\)|Build succeeded|Build FAILED' | ForEach-Object { '  ' + $_.Line.Trim() })
$lines += '=== full sweep with sympy REQUIRED ==='
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = '1'
$sw = & 'docs\goal-cycle-5\final-sweep.ps1' 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'sweep.txt'), $sw, $enc)
$lines += $sw
$lines += '=== AOT publish + freshness + CI smoke scenarios ==='
$a = & 'docs\goal-cycle-5\final-aot.ps1' 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'aot-and-smoke-run.txt'), $a, $enc)
$lines += $a
$lines += '=== capability honesty (published binary) ==='
$cap = & 'docs\goal-cycle-5\verify-capabilities.ps1' 2>&1 | Out-String
$lines += (($cap -split ([char]10)) | Select-String -Pattern 'advertised entries|MATCH=' | ForEach-Object { $_.Line.Trim() })
$lines += '=== round-trip property harness (published binary) ==='
$env:RT_USE_AOT = '1'
$rt = & 'docs\goal-cycle-5\run-roundtrip2.ps1' 2>&1 | Out-String
$lines += (($rt -split ([char]10)) | Select-String -Pattern 'SUMMARY' | ForEach-Object { $_.Line.Trim() })
$lines += 'final verification finished ' + (Get-Date -Format o)
[System.IO.File]::WriteAllText((Join-Path $out 'final.txt'), ($lines -join ([char]10)), $enc)
$lines | ForEach-Object { Write-Output $_ }