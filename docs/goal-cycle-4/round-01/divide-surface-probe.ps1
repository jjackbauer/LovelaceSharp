# Cycle-4: is the Real division defect reachable from the user-facing runner?
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-4\round-01\divide-surface'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = [ordered]@{
  'one_over_0_9'      = '1 / 0.9'
  'ten_over_9'        = '10 / 9'
  'one_over_0_99'     = '1 / 0.99'
  'one_over_near1'    = '1 / 0.9993142073433579945'
  'cos_neg2'          = 'cos(1/27)^(-2)'
  'two_pow_neg2'      = '2^-2'
  'one_over_2'        = '1 / 2'
  'reciprocal_builtin'= '1 / 3'
}
foreach ($k in $cases.Keys) {
  $f = Join-Path $d ($k + '.ls')
  [System.IO.File]::WriteAllText($f, $cases[$k], $enc)
  $raw = [string]::Join([char]10, (& dotnet $dll --file $f --omit-functions 2>&1))
  $shown = 'NOJSON'
  try {
    $o = $raw | ConvertFrom-Json
    if ($o.ok) {
      $s = $o.result.structured
      $val = if ($s.pretty) { $s.pretty } elseif ($s.display) { $s.display } else { $s | ConvertTo-Json -Compress -Depth 3 }
      $shown = "ok  $val"
    } else {
      $shown = "ERR $($o.message)"
    }
  } catch { $shown = 'NOJSON: ' + ($raw.Substring(0, [Math]::Min(160, $raw.Length))) }
  Write-Output ('{0,-20} {1,-8} -> {2}' -f $k, $cases[$k], $shown)
}
