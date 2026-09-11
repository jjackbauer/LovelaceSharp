$dir = 'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-3\round-1\probes'
$enc = New-Object System.Text.UTF8Encoding($false)
Get-ChildItem $dir -Filter *.json | ForEach-Object {
  $raw = [System.IO.File]::ReadAllText($_.FullName).TrimEnd()
  $sb = New-Object System.Text.StringBuilder
  for ($i = 0; $i -lt $raw.Length; $i += 1500) {
    $len = [Math]::Min(1500, $raw.Length - $i)
    [void]$sb.AppendLine($raw.Substring($i, $len))
  }
  [System.IO.File]::WriteAllText((Join-Path $dir ($_.BaseName + '.chunked')), $sb.ToString(), $enc)
}
Write-Output ('chunked ' + (Get-ChildItem $dir -Filter *.chunked).Count + ' files')