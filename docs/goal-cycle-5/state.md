# Harness State — cycle-5

- **Round**: goal round 3 (memory reconciliation). **26 commits** on top of `9228305`; local ==
  `origin/main` at `1aa7182`. Nothing is in flight.
- **Goal**: close every Tier-0/Tier-1 defect with independent-ground-truth tests, accept or close every
  residual bound in writing, and claim A+ only if a fresh adversarial audit cannot falsify it.

## Why this file was rewritten

It was last written at `40b96d9` and said "seven open" and "two rounds in flight". Three commits have
landed since (`d8f03d4`, `0a75c3f`, `1aa7182`). `evidence.md` was kept current after every landing;
this file, `journal.md`, `deliverables.md` and the report were not. That is recorded as OBS-016 rather
than quietly repaired.

## Landed and verified (every patch applied, suites run and commit made by me)

| Commit | Round | What |
|---|---|---|
| `34c970c` `8502e5f` | T0-2 | the printer delimiters a base from the text it prints; 30/30 round-trip shapes |
| `3ffc782` `b908d9a` | T0-4 | periodic multiply/add through exact fractions; the fast path declines periodic operands |
| `bb9746f` | T0-1 | an inverse-branch candidate is verified before it enters a complete set |
| `2bd8785` | T0-5 | argument-shape errors are typed; `solve_system` publishes `SystemSolveResult` |
| `5382861` | T0-6 | deep input crosses as `DepthExceeded` |
| `4ef7edf` | T0-3 | exact terminating quotients; `0^negative` refuses; exactness is provenance |
| `a03553a` | wave-1 | an empty system solution set is published only when proved |
| `a0a70a6` `86a5fc8` `49f5a4c` | wave-2 wire | `variables[].structured`, arity metadata, total `--print-budget`, `divrem` record, `inspect(Real).exact`; `evalf` digits, structured `assumptions()`, `solve` returns the record, protocol examples match the binary; exactness is a property of the leaves |
| `952c761` `40b96d9` | wave-2 kernel | the infinite family `log(c)+2πik` is published as a family; a circular answer is refused; `limit((1+1/x)^x,x,∞)` = e; a valueless record reports `SolutionExactness.None` |
| `d8f03d4` | audit-2 | a deep runtime value crosses as `DepthExceeded` (was `0xC00000FD`, 0 bytes) |
| `0a75c3f` `1aa7182` | audit-2 | trig reduces against a π that resolves the argument; `0.(9)==1`; division keeps exactness; and through the wire `sin(pi(100)*2)` = −1.64e-100, `tan(pi(100)/2)` = 2.4346e100, `evalf` of a truncated transcendental is inexact again (the regression cycle 5 introduced) |

Latest verification on the merged tree: Complex 96/0, Real 2475/0, Suite 813/0, Symbolics 1031/0
(SymPy oracle required, 0 skipped), Run 175/0, Dsp 61/0, precbench 13/0.

## Open — the honest list (four items, each reproduced and located)

1. `RealLiteral.FromRealExact` (`Lovelace.Symbolics/Expr.cs:82`) drops provenance, so
   `evalf(sin(pi*1/6), 40)` still reports `exact:true` with a rational form. Needs a provenance bit on
   `RealLiteral` or a public inexact factory on `Real`.
2. `--cancel-after` is ignored inside array/numeric kernels (`sum(1..10000000)` with a 1 ms budget ran
   4.9–6.0 s and returned `ok:true`; no field reports the overrun).
3. `evalf(f, 0)` raises `InternalError/InternalInvariantFailure` (pre-existing, both trees).
4. `capabilities()` under-claims `(-4)^(1/2)`, which succeeds; three refusals remain unlisted.

Recorded, not fixed: `Real.Sin` of a *periodic* argument uses `DivideNonPeriodic` and ignores the period
(byte-identical on `9228305`); the three special-angle lines that now show true residuals instead of 0
(a deliberate trade documented by the round that made it).

## Gate status

| Gate | State |
|---|---|
| G1 Evidence | PASS — EVD-201…EVD-236, each reproduced by me |
| G2 Falsification | **open** — four P0/P1 rows above |
| G3 Coverage | Tier-0 all six closed; Tier-1 closed except item 4 |
| G4 Reproduction | PASS — every landed round re-run by me; every pre-fix claim reproduced in a control worktree |
| G5 Honesty | PASS — including this file's own staleness, the refuted delivery claim (EVD-236) and the regression cycle 5 caused |

**A+ is not claimed.** D1 asks for a fresh audit with no P0/P1; audit wave 2 produced ~15 and four are
still open. The goal stays active.
