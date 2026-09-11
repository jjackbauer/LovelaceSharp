$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\baseline'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$lines = @()
$lines += 'baseline started ' + (Get-Date -Format o) + ' HEAD ' + (git rev-parse --short HEAD)
Get-Process testhost -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3
$lines += '=== forced full rebuild ==='
$b = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'forced-rebuild.log'), $b, $enc)
$lines += 'exit=' + $LASTEXITCODE + ' warninglines=' + ([regex]::Matches($b,': warning ')).Count + ' errorlines=' + ([regex]::Matches($b,': error ')).Count
$lines += ($b -split ([char]10) | Select-String -Pattern 'Warning\(s\)|Error\(s\)|Build succeeded|Build FAILED' | ForEach-Object { '  ' + $_.Line.Trim() })
$lines += '=== full sweep with sympy REQUIRED ==='
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = '1'
$sw = & 'docs\goal-cycle-5\sweep.ps1' 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'sweep.txt'), $sw, $enc)
$lines += $sw
$lines += '=== AOT publish + freshness + CI smoke scenarios ==='
$a = & 'docs\goal-cycle-5\aot-and-smoke.ps1' 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'aot-and-smoke-run.txt'), $a, $enc)
$lines += $a
$lines += 'baseline finished ' + (Get-Date -Format o)
[System.IO.File]::WriteAllText((Join-Path $out 'baseline.txt'), ($lines -join ([char]10)), $enc)
$lines | ForEach-Object { Write-Output $_ }