$ErrorActionPreference='Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$g = 'Lovelace.Run.Tests\fixtures\record.json'
$bak = 'Lovelace.Run.Tests\fixtures\record.json.bak'
Copy-Item $g $bak -Force
$txt = [System.IO.File]::ReadAllText($g)
$mut = $txt -replace '"Solved"', '"SolvedX"'
if ($mut -eq $txt) { Write-Output 'MUTATION DID NOT APPLY' } else { Write-Output 'mutation applied' }
[System.IO.File]::WriteAllText($g, $mut, (New-Object System.Text.UTF8Encoding($false)))
Write-Output '=== RED RUN (mutated golden) ==='
& dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo 2>&1 | Select-String -Pattern 'Failed|Passed!|error|\$.result' | Select-Object -First 20 | ForEach-Object { $_.Line.Trim() }
Write-Output '=== restoring ==='
Copy-Item $bak $g -Force
Remove-Item $bak -Force
Write-Output '=== GREEN RUN (restored golden) ==='
& dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo 2>&1 | Select-String -Pattern 'Passed!|Failed:|Skipped' | ForEach-Object { $_.Line.Trim() }