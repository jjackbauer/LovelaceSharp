$deadline = (Get-Date).AddMinutes(28)
while ((Get-Date) -lt $deadline) {
  $hits = @()
  foreach ($w in @('c5-limits','c5-realexact')) {
    foreach ($f in @(".worktrees\$w\c5-report.md", ".worktrees\$w\c5-report2.md")) { if (Test-Path $f) { $hits += ($w + ':' + (Split-Path $f -Leaf)) } }
  }
  if ($hits.Count -gt 0) { 'READY ' + ($hits -join ' ') ; exit 0 }
  Start-Sleep -Seconds 20
}
'TIMEOUT'