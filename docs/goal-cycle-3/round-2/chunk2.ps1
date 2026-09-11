$dir = 'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-3\round-2'
$enc = New-Object System.Text.UTF8Encoding($false)
foreach ($n in @('pre-refactor-envelope','post-refactor-envelope')) {
  $raw = [System.IO.File]::ReadAllText((Join-Path $dir ($n + '.json'))).TrimEnd()
  $sb = New-Object System.Text.StringBuilder
  for ($i = 0; $i -lt $raw.Length; $i += 1500) { $len = [Math]::Min(1500, $raw.Length - $i); [void]$sb.AppendLine($raw.Substring($i, $len)) }
  [System.IO.File]::WriteAllText((Join-Path $dir ($n + '.chunked')), $sb.ToString(), $enc)
  Write-Output ('{0} chars={1}' -f $n, $raw.Length)
}