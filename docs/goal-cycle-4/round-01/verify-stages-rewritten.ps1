# Cycle-4 TODO-006: re-verify the REWRITTEN stages 3 and 4 (DEC-006). Stage 2 (84a6a54) is
# unchanged and was verified by EVD-172; stage 5 is the final tree, verified by the full sweep.
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-01\stage-verification-rewritten.txt'
$lines = @()
$stages = @(
  @{ Name = 'stage3-wire-rewritten';     Commit = 'd78e385'; Suites = @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Studio.Tests') },
  @{ Name = 'stage4-remaining-rewritten';Commit = '35e8b06'; Suites = @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Console.Tests') }
)
foreach ($s in $stages) {
  $dir = "C:\Users\ricar\dev\.lovelace-check-$($s.Name)"
  if (Test-Path $dir) { git worktree remove --force $dir 2>$null | Out-Null }
  git worktree add $dir $s.Commit 2>$null | Out-Null
  $lines += "=================== $($s.Name)  commit $($s.Commit)"
  Push-Location $dir
  $build = & dotnet build LovelaceSharp.slnx -c Release --nologo 2>&1 | Out-String
  $bw = ([regex]::Matches($build, ': warning ')).Count
  $be = ([regex]::Matches($build, ': error ')).Count
  $bok = $build -match 'Build succeeded'
  $lines += "  build: succeeded=$bok warnings=$bw errors=$be"
  if (-not $bok) { $lines += ($build -split "`n" | Select-String -Pattern ': error ' | Select-Object -First 12 | ForEach-Object { '    ' + $_.Line.Trim() }) }
  foreach ($suite in $s.Suites) {
    $proj = Join-Path $suite ($suite + '.csproj')
    if (-not (Test-Path $proj)) { $lines += "  $suite : MISSING at this commit"; continue }
    $t = & dotnet test $proj -c Release --nologo 2>&1 | Out-String
    $m = [regex]::Match($t, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+)')
    if ($m.Success) {
      $lines += ('  {0,-28} failed={1,-4} passed={2,-5} skipped={3}' -f $suite, $m.Groups[1].Value, $m.Groups[2].Value, $m.Groups[3].Value)
      if ([int]$m.Groups[1].Value -gt 0) { $lines += ($t -split "`n" | Select-String -Pattern '\[FAIL\]' | Select-Object -First 15 | ForEach-Object { '      ' + $_.Line.Trim() }) }
    } else {
      $lines += "  $suite : NO-SUMMARY"
      $lines += ($t -split "`n" | Select-Object -Last 6 | ForEach-Object { '      ' + $_.Trim() })
    }
  }
  Pop-Location
}
$lines | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $out), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
