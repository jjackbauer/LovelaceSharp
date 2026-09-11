# Cycle-4 §4.1 falsification test, run by the orchestrator.
# For EVERY advertised unsupported_operations entry: execute the entry's own trigger script and
# compare the advertised code/category/message with what the live call actually produces.
# Handles both surfaces: an error envelope (ok:false) and a refusal that rides inside a result
# record's diagnostics (ok:true, as limit/integration/rootof do).
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-4\round-10'
$tmp = Join-Path $env:TEMP 'c4-caps-verify'
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'

function Envelope([string]$body, [string]$name) {
  $f = Join-Path $tmp ($name + '.ls')
  [System.IO.File]::WriteAllText($f, $body, $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  try { return ($raw | ConvertFrom-Json) } catch { return $null }
}
function FieldVal($fields, [string]$n) { foreach ($fl in $fields) { if ($fl.name -eq $n) { return $fl.value } } return $null }

# --- the advertised statement ---
$caps = Envelope 'capabilities()' 'caps'
$entries = @()
foreach ($fl in $caps.result.structured.fields) {
  if ($fl.name -eq 'unsupported_operations') { foreach ($e in $fl.value.elements) { $ef = @{}; foreach ($x in $e.fields) { $ef[$x.name] = $x.value }; $entries += [pscustomobject]@{ Class = $ef['operation_class'].value; Code = $ef['code'].value; Category = $ef['category'].value; Message = $ef['message'].value; Trigger = $ef['trigger'].value } } }
}
"advertised entries: $($entries.Count)"
$lines = @("advertised entries: $($entries.Count)")
$lines += ('{0,-44} {1,-34} {2,-20} {3}' -f 'operation_class', 'advertised code', 'category', 'verdict')
$match = 0; $mismatch = 0
foreach ($e in $entries) {
  $o = Envelope $e.Trigger ($e.Class -replace '[^A-Za-z0-9]', '_')
  $liveCode = $null; $liveCat = $null; $liveMsg = $null; $surface = ''
  if ($o -and -not $o.ok) {
    $surface = 'error'; $liveCode = $o.code; $liveCat = $o.category; $liveMsg = $o.message
  } elseif ($o) {
    $s = $o.result.structured
    if ($s.fields) {
      $diag = FieldVal $s.fields 'diagnostics'
      if ($diag -and $diag.elements) {
        foreach ($de in $diag.elements) {
          $df = @{}; foreach ($x in $de.fields) { $df[$x.name] = $x.value }
          if ($df['code'] -and $df['code'].value -eq $e.Code) { $surface = 'diagnostic'; $liveCode = $df['code'].value; $liveCat = $df['category'].value; $liveMsg = $df['message'].value }
        }
      }
      if (-not $surface) {
        # a refusal may also surface only as a status enum, with the message on the class
        foreach ($fl in $s.fields) { if ($fl.name -eq 'status') { $surface = 'status:' + $fl.value.value } }
      }
    }
  }
  $ok = ($liveCode -eq $e.Code) -and ($liveCat -eq $e.Category)
  if ($ok) { $match++ } else { $mismatch++ }
  $verdict = if ($ok) { "MATCH ($surface)" } else { "MISMATCH live=$liveCode/$liveCat [$surface]" }
  $row = '{0,-44} {1,-34} {2,-20} {3}' -f $e.Class, $e.Code, $e.Category, $verdict
  $lines += $row
  Write-Output $row
}
$lines += ""
$lines += "MATCH=$match  MISMATCH=$mismatch"
Write-Output ""
Write-Output "MATCH=$match  MISMATCH=$mismatch"
[System.IO.File]::WriteAllText((Join-Path (Get-Location) (Join-Path $d 'capability-honesty-verification.txt')), ($lines -join [char]10), $enc)

# --- the eight classes the round-09 audit found unlisted: are they advertised now? ---
""
$need = @('limit.unevaluated','integration','plot','solve','dsp','linsolve','fft')
"--- the eight audit classes present in the statement? ---"
foreach ($n in $need) {
  $hit = @($entries | Where-Object { $_.Class -like "*$n*" -or $_.Trigger -like "*$n*" })
  "{0,-22} -> {1}" -f $n, $(if ($hit) { ($hit | ForEach-Object { $_.Class }) -join ', ' } else { 'NOT ADVERTISED' })
}