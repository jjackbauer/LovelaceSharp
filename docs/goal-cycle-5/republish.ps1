$ErrorActionPreference='Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-5\final'
$enc = New-Object System.Text.UTF8Encoding($false)
$lines = @()
$lines += 'republish started ' + (Get-Date -Format o) + ' HEAD ' + (git rev-parse --short HEAD)
Remove-Item 'out\aot\Lovelace.Run.exe' -Force -ErrorAction SilentlyContinue
Get-Process testhost -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
$a = & 'docs\goal-cycle-5\final-aot.ps1' 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'aot-and-smoke-republish.txt'), $a, $enc)
$lines += (($a -split ([char]10)) | Select-String -Pattern 'publish exit|PASS|FAIL|SMOKE FAILURES|newest source|exe LastWriteTime' | ForEach-Object { $_.Line.Trim() })
$env:RT_USE_AOT = '1'
$rt = & 'docs\goal-cycle-5\run-roundtrip2.ps1' 2>&1 | Out-String
$lines += (($rt -split ([char]10)) | Select-String -Pattern 'SUMMARY' | ForEach-Object { $_.Line.Trim() })
$cap = & 'docs\goal-cycle-5\verify-capabilities.ps1' 2>&1 | Out-String
$lines += (($cap -split ([char]10)) | Select-String -Pattern 'advertised entries|MATCH=' | ForEach-Object { $_.Line.Trim() })
$lines += 'republish finished ' + (Get-Date -Format o)
[System.IO.File]::WriteAllText((Join-Path $out 'republish.txt'), ($lines -join ([char]10)), $enc)
$lines | ForEach-Object { Write-Output $_ }