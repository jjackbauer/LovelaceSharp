$deadline = (Get-Date).AddMinutes(26)
while ((Get-Date) -lt $deadline) {
  $hits = @()
  foreach ($w in @('c5-limits2','c5-exactflag','c5-wire3b','c5-real2','c5-solver3','c5-depth2')) { if (Test-Path ".worktrees\$w\c5-report.md") { $hits += $w } }
  if ($hits.Count -gt 0) { 'READY ' + ($hits -join ',') ; exit 0 }
  Start-Sleep -Seconds 20
}
'TIMEOUT'