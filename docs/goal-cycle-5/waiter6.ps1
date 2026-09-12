$deadline = (Get-Date).AddMinutes(26)
while ((Get-Date) -lt $deadline) {
  $hits = @()
  foreach ($w in @('c5-exact2','c5-cancel','c5-caps')) { if (Test-Path ".worktrees\$w\c5-report.md") { $hits += $w } }
  if ($hits.Count -gt 0) { 'READY ' + ($hits -join ',') ; exit 0 }
  Start-Sleep -Seconds 20
}
'TIMEOUT'