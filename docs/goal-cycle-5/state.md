# Harness State — cycle-5

- **Round**: goal round 2 closed. **21 commits** on top of `9228305`; local == `origin/main` at `40b96d9`; CI green on run #15 (and #14 caught a real CI-only failure that is now fixed).
- **Goal**: close every Tier-0/Tier-1 defect with independent-ground-truth tests, accept or close every
  residual bound in writing, and claim A+ only if a fresh adversarial audit cannot falsify it.

## Landed and verified this round

| Commit | Round | What |
|---|---|---|
| `a0a70a6` | wire-3a | `variables[].structured`, `functions[]` arity metadata, total `--print-budget`, `divrem` as a record, `inspect(<Real>).exact` |
| `86a5fc8` | wire-3b | `evalf` honours its digit count, structured `assumptions()`, `solve` returns the record, protocol examples match the binary |
| `49f5a4c` | exact-flag | exactness is a property of the leaves: `sin(x)`, `sqrt(x)`, `x^(3/2)` are exact, a Real leaf stays inexact |
| `952c761` | solver | the infinite family `log(c) + 2*pi*k*i` is published as a family; a solution that mentions the solved symbol is refused |
| `40b96d9` | limits | `limit((1+1/x)^x, x, inf)` = e (was 1); a record with no value reports `SolutionExactness.None` |

Verification totals on the merged tree: Symbolics **1031/0** (SymPy oracle required), Suite **805/0**, Run **159/0**, Real **2456/0**.

## Audit wave 2 (four reports, ~2200 probes)

**Fixed**: the infinite-family completeness claim; the circular "solution"; `evalf`'s digit count; the
unevaluated-limit exactness claim; print-budget totality; arity metadata.

**Open** (dispatched or recorded):
1. **A regression cycle 5 introduced**: `evalf(sin(1), 40)` reports `exact:true` with a rational form
   where `9228305` reported `exact:false`; traced to `4ef7edf` (provenance defaults to exact while the
   Symbolics numeric path builds its Real without clearing it). Diagnosis handed over with file:line.
2. `sin`/`tan` at multiples of the runner's own `pi(100)` are wrong by ~70 orders of magnitude
   (round `c5-real2` in flight).
3. `0.(9) == 1` is false while `0.(9) - 1 == 0` is true (round `c5-real2` in flight).
4. `x = 1; x = [x];` ×12000 kills the process with no envelope (round `c5-depth2` in flight).
5. `--cancel-after` is ignored inside array/numeric kernels (recorded, no round).
6. `evalf(f, 0)` raises an internal invariant failure (pre-existing; recorded).
7. `capabilities()` still under-claims `(-4)^(1/2)`; three refusals remain unlisted.

## Gate status

| Gate | State |
|---|---|
| G1 Evidence | PASS — EVD-201…EVD-234, each reproduced by me |
| G2 Falsification | **open** — audit wave 2 produced ~15 P0/P1 defects; five are fixed, the rest listed above |
| G3 Coverage | Tier-0: all six closed. Tier-1: the machine-API cluster closed except the items above |
| G4 Reproduction | PASS — every landed round re-run by me; every pre-fix claim reproduced in a control worktree |
| G5 Honesty | PASS — the regression this cycle caused, the CI-only failure, the golden regenerations and every open bound are recorded |

**A+ is not claimed.** D1 requires a fresh audit to find no P0/P1; audit wave 2 found ~15 and seven are
still open. The goal stays active.
