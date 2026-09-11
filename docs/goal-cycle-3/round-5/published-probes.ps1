$ErrorActionPreference = 'Continue'
$exe = "out/aot/Lovelace.Run.exe"
if (-not (Test-Path $exe)) { $exe = "out/aot/Lovelace.Run" }
Write-Output ("published binary: " + (Resolve-Path $exe))
function Probe([string]$label, [string]$file) {
  Write-Output ("---- " + $label + "  [" + $file + "]")
  $json = & $exe --file $file --omit-functions --omit-variables 2>&1 | Out-String
  $code = $LASTEXITCODE
  try {
    $o = $json | ConvertFrom-Json
    if ($o.ok) {
      $s = $o.result.structured
      if ($s.kind -eq "Record") {
        $f = @{}
        foreach ($fld in $s.fields) { $f[$fld.name] = $fld.value }
        Write-Output ("exit=" + $code + " type=" + $s.type + " status=" + $f["status"].value + " complete=" + $f["complete"].value + " completeness=" + $f["completeness"].value)
      } else {
        Write-Output ("exit=" + $code + " kind=" + $s.kind + " (no record fields)")
      }
    } else {
      Write-Output ("exit=" + $code + " code=" + $o.code + " category=" + $o.category + " recoverable=" + $o.recoverable)
      Write-Output ("        message=" + $o.message)
    }
  } catch { Write-Output ("RAW: " + $json) }
}
Probe "NoSolutions (denominator removes every root)" "docs/goal-cycle-3/round-5/probes/a1_ns_ratio.ls"
Probe "NoSolutions (x/x == 0)" "docs/goal-cycle-3/round-5/probes/a2_ns_xoverx.ls"
Probe "NoSolutions (x^2 + 1 == 0, real)" "docs/goal-cycle-3/round-5/probes/a3_ns_real.ls"
Probe "Solved control (x^2 - 4 == 0)" "docs/goal-cycle-3/round-5/probes/a5_solved.ls"
Probe "Partial control (x^4 - x^2 - 1 == 0)" "docs/goal-cycle-3/round-5/probes/a4_partial.ls"
Probe "Unevaluated control (x^4 + 1 == 0)" "docs/goal-cycle-3/round-5/probes/a6_unevaluated.ls"
Probe "System NoSolutions (inconsistent)" "docs/goal-cycle-3/round-5/probes/a7_system_ns.ls"
Probe "Arity failure compile_full(x^2 + 1)" "docs/goal-cycle-3/round-5/probes/b1_arity.ls"
Probe "Arity OK compile_full(x^2 + 1, [x])" "docs/goal-cycle-3/round-5/probes/b2_arity_ok.ls"
Probe "Optional trailing args: dft([0,1,0,0])" "docs/goal-cycle-3/round-5/probes/b3_dft_short.ls"
Probe "Optional trailing args: solve(f, x)" "docs/goal-cycle-3/round-5/probes/b5_solve_short.ls"
Probe "Optional trailing args: symbol(name)" "docs/goal-cycle-3/round-5/probes/b4_symbol_short.ls"