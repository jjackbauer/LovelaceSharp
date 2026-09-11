$deadline = (Get-Date).AddMinutes(26)
while ((Get-Date) -lt $deadline) {
  $hits = @()
  foreach ($w in @('c5-limits2','c5-exactflag','c5-wire3a','c5-wire3b','c5-real2')) {
    foreach ($f in @(".worktrees\$w\c5-report.md", ".worktrees\$w\c5-report2.md")) { if (Test-Path $f) { $hits += ($w + ':' + (Split-Path $f -Leaf)) } }
  }
  $a = Get-ChildItem docs\goal-cycle-5\audit2 -Filter *.md -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne 'B1-differential.md' }
  if ($hits.Count -gt 0 -or $a) { 'READY ' + ($hits -join ' ') + ' audits=' + (($a | ForEach-Object { $_.Name }) -join ',') ; exit 0 }
  Start-Sleep -Seconds 20
}
'TIMEOUT'