# Deliverables — cycle-3

| ID | Artifact | Path | Evidence | Confidence | Verified by |
|---|---|---|---|---|---|
| ART-001 | Cycle-3 alignment addendum (approved; amended with section 9.1 wire resolutions) | `docs/symbolics/a-plus-cycle-3-alignment.md` | EVD-001..EVD-029 | High | read back; approval received |
| ART-002 | Pre-change baseline transcript | `docs/goal-cycle-3/round-1/baseline-summary.txt` | EVD-001..EVD-005 | High | produced by baseline.ps1, exit 0 |
| ART-003 | Reproduction probe corpus (60 envelopes) | `docs/goal-cycle-3/round-1/probes/` | EVD-006..EVD-026 | High | each envelope parsed and reduced to a shape digest |
| ART-004 | Independent source observations A, B, C | `docs/goal-cycle-3/round-1/obs-{A,B,C}-*.md` | EVD-007, 014, 019, 021, 028, 029 | High | spot-checked each observer's most consequential citation |
| ART-005 | Runner test project + 16 golden fixtures, wired into the slnx and CI | `Lovelace.Run.Tests/` | EVD-030..EVD-036, EVD-046..EVD-050 | High | re-ran 28/28; mutated a golden and watched it fail with a JSON path |
| ART-006 | Structural solver emission (bindings, Symbol parameter/variable, system domain/complete/exactness) | `Lovelace.Symbolics/SymbolicsPlugin.cs`, `Solvers/Solve.cs`, `Lovelace.Suite/RecordSchemas.cs` | EVD-037..EVD-040 | High | read the live envelope; diffed every changed golden |
| ART-007 | Enum structured kind across the protocol | `Lovelace.Abstractions/EnumValue.cs`, `Lovelace.Suite/StructuredProjection.cs`, `docs/symbolics/dsh-protocol.md`, `.github/workflows/ci.yml` | EVD-041..EVD-045 | High | read the published binary's raw JSON; re-ran six suites and the AOT publish |
| ART-008 | Diagnostic vocabulary and structured diagnostics on all seven rich records | `Lovelace.Abstractions/Diagnostic.cs`, `Lovelace.Symbolics/SymbolicsPlugin.cs`, `Lovelace.MathIR/Evaluator.cs`, `Lovelace.Suite/RecordSchemas.cs` | EVD-046..EVD-051 | High | read the published binary's diagnostics record field by field; 0 Text diagnostics remain |
