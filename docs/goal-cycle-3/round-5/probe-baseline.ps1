
$ErrorActionPreference = 'Continue'
$runner = "Lovelace.Run/bin/Release/net10.0/Lovelace.Run.exe"
function Probe([string]$label, [string]$file) {
  Write-Output "---- $label"
  $json = & $runner --file $file --omit-functions --omit-variables 2>&1 | Out-String
  try {
    $o = $json | ConvertFrom-Json
    if ($o.ok) {
      $f = @{}
      foreach ($fld in $o.result.structured.fields) { $f[$fld.name] = $fld.value }
      $status = $f['status'].value; $complete = $f['complete'].value; $comp = $f['completeness'].value
      Write-Output ("ok=true kind={4} status={0} complete={1} completeness={2} unrepresented_count={3}" -f $status, $complete, $comp, ($f['unrepresented_count'].value), $o.result.kind)
    } else {
      Write-Output ("ok=false code={0} category={1} recoverable={2} message={3}" -f $o.code, $o.category, $o.recoverable, $o.message)
    }
  } catch { Write-Output ("RAW: " + $json) }
}
Probe "a1_ns_ratio" "docs/goal-cycle-3/round-5/probes/a1_ns_ratio.ls"
Probe "a2_ns_xoverx" "docs/goal-cycle-3/round-5/probes/a2_ns_xoverx.ls"
Probe "a3_ns_real" "docs/goal-cycle-3/round-5/probes/a3_ns_real.ls"
Probe "a4_partial" "docs/goal-cycle-3/round-5/probes/a4_partial.ls"
Probe "a5_solved" "docs/goal-cycle-3/round-5/probes/a5_solved.ls"
Probe "a6_unevaluated" "docs/goal-cycle-3/round-5/probes/a6_unevaluated.ls"
Probe "a7_system_ns" "docs/goal-cycle-3/round-5/probes/a7_system_ns.ls"
Probe "a8_system_inconsistent2" "docs/goal-cycle-3/round-5/probes/a8_system_inconsistent2.ls"
Probe "b1_arity" "docs/goal-cycle-3/round-5/probes/b1_arity.ls"
Probe "b2_arity_ok" "docs/goal-cycle-3/round-5/probes/b2_arity_ok.ls"
Probe "b3_dft_short" "docs/goal-cycle-3/round-5/probes/b3_dft_short.ls"
Probe "b4_symbol_short" "docs/goal-cycle-3/round-5/probes/b4_symbol_short.ls"
Probe "b5_solve_short" "docs/goal-cycle-3/round-5/probes/b5_solve_short.ls"