$ErrorActionPreference='Continue'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = Join-Path $env:TEMP 'b1-attrib'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$probes = @(
  @('f1-evalf-sin1','evalf(sin(1), 40)'),
  @('f2-explog2','evalf(exp(log(2)), 40)'),
  @('f3-sin-pi100','evalf(sin(pi(100)*2), 40)'),
  @('f4-periodic9','x = (0.(9) == 1)'),
  @('f5-evalf-zero-digits','evalf(sin(1), 0)'),
  @('f6-tiny-quotient','evalf(1/(3*10^1000), 30)')
)
foreach ($p in $probes) {
  $f = Join-Path $d ($p[0] + '.ls')
  [System.IO.File]::WriteAllText($f, $p[1], $enc)
  if ($env:PROBE_MODE -eq 'aot') { $raw = [string]::Join([char]10, (& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --file $f --omit-functions 2>&1)) }
  else { $raw = [string]::Join([char]10, (& dotnet run --project 'C:\Users\ricar\dev\LovelaceSharp\.worktrees\c5-orch-control\Lovelace.Run' -c Release -- --file $f --omit-functions 2>&1)) }
  $j = $null; try { $j = $raw | ConvertFrom-Json } catch { $p[0] + ' ENVELOPE-FAIL'; continue }
  if (-not $j.ok) { '{0,-22} ERR {1}/{2} :: {3}' -f $p[0], $j.code, $j.category, ([string]$j.message).Substring(0,[Math]::Min(60,([string]$j.message).Length)); continue }
  $st = $j.result.structured
  $x = if ($st.exact -ne $null) { ' exact=' + $st.exact } else { '' }
  '{0,-22} {1}{2}' -f $p[0], ([string]$j.result.display).Substring(0,[Math]::Min(44,([string]$j.result.display).Length)), $x
}