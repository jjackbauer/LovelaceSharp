# Cycle-4 final verification sequence, on the frozen final tree, in one job:
#   1. forced full rebuild  -> D5 (0 warnings / 0 errors)
#   2. full 15-project sweep -> D4 (comparable to EVD-156's 2439)
#   3. Native AOT publish + freshness guard + the five CI smoke scenarios -> D6
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-08'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'final-sequence.txt'
$lines = @()
$lines += "started: $(Get-Date -Format o)"
$lines += "HEAD: $(git rev-parse --short HEAD)"
$lines += "dirty entries: $((git status --porcelain | Measure-Object).Count)"
$lines += ""

$lines += "=================== 1. forced full rebuild ==================="
$b = & dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental 2>&1 | Out-String
$lines += "exit=$LASTEXITCODE"
$lines += "warning lines=$(([regex]::Matches($b, ': warning ')).Count)  error lines=$(([regex]::Matches($b, ': error ')).Count)"
$lines += ($b -split "`n" | Select-String -Pattern 'Warning\(s\)|Error\(s\)|Build succeeded|Build FAILED' | ForEach-Object { '  ' + $_.Line.Trim() })
$lines | ForEach-Object { Write-Output $_ }

$lines += ""
$lines += "=================== 2. full 15-project sweep ==================="
& 'docs\goal-cycle-4\round-01\sweep.ps1' | Tee-Object -Variable sweepOut | Out-Null
$lines += $sweepOut
$lines | ForEach-Object { Write-Output $_ }

$lines += ""
$lines += "=================== 3. Native AOT publish + smoke ==================="
$env:PATH = "C:\Users\ricar\dev\.lovelace-tools\python;" + $env:PATH
$r = & 'docs\goal-cycle-4\round-06\aot-and-smoke.ps1' 2>&1 | Out-String
$lines += $r
$lines += ""
$lines += "finished: $(Get-Date -Format o)"
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $log), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
$lines | ForEach-Object { Write-Output $_ }
