# Cycle-4 TODO-004: verify each stage commit in its own clean worktree.
# A red stage means the offending file moves to the next stage and the two commits are rewritten.
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-01\stage-verification.txt'
$lines = @()
$stages = @(
  @{ Name = 'stage2-fixes';        Commit = '84a6a54'; Suites = @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Integer.Tests','Lovelace.Natural.Tests') },
  @{ Name = 'stage3-wire';         Commit = '160ba4e'; Suites = @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Studio.Tests') },
  @{ Name = 'stage4-remaining';    Commit = '3367447'; Suites = @('Lovelace.Symbolics.Tests','Lovelace.Suite.Tests','Lovelace.Console.Tests') }
)
foreach ($s in $stages) {
  $dir = "C:\Users\ricar\dev\.lovelace-check-$($s.Name)"
  if (Test-Path $dir) { git worktree remove --force $dir 2>$null | Out-Null }
  git worktree add $dir $s.Commit 2>$null | Out-Null
  $lines += "=================== $($s.Name)  commit $($s.Commit)  dir $dir"
  Push-Location $dir
  $build = & dotnet build LovelaceSharp.slnx -c Release --nologo 2>&1 | Out-String
  $bw = ([regex]::Matches($build, ': warning ')).Count
  $be = ([regex]::Matches($build, ': error ')).Count
  $bok = $build -match 'Build succeeded'
  $lines += "  build: succeeded=$bok warnings=$bw errors=$be"
  if (-not $bok) {
    $lines += ($build -split "`n" | Select-String -Pattern ': error ' | Select-Object -First 12 | ForEach-Object { '    ' + $_.Line.Trim() })
  }
  foreach ($suite in $s.Suites) {
    $proj = Join-Path $suite ($suite + '.csproj')
    if (-not (Test-Path $proj)) { $lines += "  $suite : MISSING at this commit"; continue }
    $t = & dotnet test $proj -c Release --nologo --no-build 2>&1 | Out-String
    $m = [regex]::Match($t, 'Failed:\s+(\d+), Passed:\s+(\d+), Skipped:\s+(\d+)')
    if ($m.Success) {
      $lines += ('  {0,-28} failed={1,-4} passed={2,-5} skipped={3}' -f $suite, $m.Groups[1].Value, $m.Groups[2].Value, $m.Groups[3].Value)
      if ([int]$m.Groups[1].Value -gt 0) {
        $lines += ($t -split "`n" | Select-String -Pattern '\[FAIL\]' | Select-Object -First 15 | ForEach-Object { '      ' + $_.Line.Trim() })
      }
    } else {
      $lines += "  $suite : NO-SUMMARY"
      $lines += ($t -split "`n" | Select-Object -Last 8 | ForEach-Object { '      ' + $_.Trim() })
    }
  }
  Pop-Location
}
$lines | ForEach-Object { Write-Output $_ }
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $out), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
