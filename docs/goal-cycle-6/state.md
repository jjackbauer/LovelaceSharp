# Harness State — cycle-6

- **Round**: 9 landed — rows 2/3/4, row 1's first route, O-B11 and the CI budget are all on `main`; **CI run #36 on `be89e55` is green in all three jobs**; round cap 40
- **Goal**: make CI green on GitHub's runners again, close the four open Tier-0/Tier-1 rows and their
  residual bounds, and claim A+ only if a fresh adversarial audit cannot falsify it.
- **Definition of done**: **D0 met** (run #36, all three jobs, on `be89e55`); **D2** rows 2/3/4 closed
  with control-failing tests and row 1's first route landed; **D3** section P written with nine rows
  CLOSED and seven awaiting the maintainer's words; **D4** re-measured (rebuild 0/0, sweep 5298/1 flake,
  AOT fresh + smoke 0, capabilities MATCH=19, round-trip ok=30); **D5** report written; **D1 NOT met** —
  P-B1 and P-B3 are open and the four-persona fresh audit was not run
- **Commits landed this cycle**: `98a9049`, `70241dc`, `300f6bb`, `1b70a32`, `e8638c0`, `d4d7ccf`,
  `b009dfe`, `e8b52b3`, `aa27753`, `c5c1437`, `55c8cab`, `be89e55` (all pushed)
- **Stop criteria**: not met; the goal stays active

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | EVD-237…EVD-259 each resolve to a command I ran or a `file:line` I read |
| G2 Falsification | **FAIL (open row)** | round 3's claims 1 and 2 were Falsified by both falsifiers; the cause is located (`ComplexMath`, OQ-003) but **not closed** |
| G3 Coverage | PASS | every dimension at least Partial except D1 (None) |
| G4 Reproduction | PASS | every landing re-run by me: wire probes, four suites, forced rebuild (0/0) |
| G5 Honesty | PASS | the stopped rounds, the reverted partial fix, the open defects and the missing maintainer acceptance are all recorded, not hidden |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 1 | Fix the red CI run (D0) | Adversarial | 3 | 11 | 0 | all pass | CI #28 green, all three jobs |
| 2 | Triage the three interrupted worktrees | Survey | 3 | 4 | 0 | all pass | r21/r22 landable; r20 needs a repair |
| 3 | Close row 1 (RealLiteral provenance) | Adversarial | 3 | 6 | **2** | G2 FAIL | route 1 landed (`e8638c0`); route 2 located in `ComplexMath` |
| 4 | Close row 1's special-angle route | Adversarial | 1 | 1 | — | — | **stopped mid-flight**, work preserved and reverted (OBS-013) |
| 5 | Re-measure every section-O bound | Probe | 1 | 3 | 0 | all pass | 3 rows close, 10 still true, 1 unmeasurable |
| 6 | Close rows 3 and 4 (`evalf(f,0)`, `capabilities()`) | — | — | 1 | 0 | all pass | landed `d4d7ccf` |
| 7 | Close row 2 (`--cancel-after` in the kernels) | — | — | 1 | 0 | all pass | landed `b009dfe` |
| 8 | D4 re-measure | — | — | 2 | 0 | all pass | rebuild 0/0; sweep 5298/1 (one unexplained flake, EVD-261); AOT fresh, smoke 0; capabilities MATCH=19; round-trip ok=30 bad=0 |
| 9 | Close O-B11 (recursion) + repair the CI budget | — | 1 | 2 | 0 | all pass | landed `c5c1437`, `aa27753`, `55c8cab` |

## Open defects that block D1 (recorded, with evidence)

| # | Defect | Evidence | Status |
|---|---|---|---|
| 1 | `evalf(cos(pi(30)), 100)` publishes `-1` as `exact:true` with `-1/1`; mpmath differs at the 61st decimal. Path: `ComplexMath.SinCosAtPrecision`; pinned by `ComplexMathProvenanceTests:83-84` | EVD-251, EVD-259, OQ-003 | **open P1** |
| 2 | ~~Deep user-function recursion kills the process~~ | EVD-254, EVD-262 | **CLOSED** in `c5c1437`: the evaluation walk is bounded at 512 units, `f(85)` answers, `f(86)` and deeper are typed `DepthExceeded` refusals (the trade is recorded as RISK-005) |
| 3 | `1/(3*10^1000)` returns `0`, and `evalf(1/(3*10^1000),30)` returns Integer `0` **exact:true**; mpmath gives 3.33e-1001 | EVD-255 | **open P1** (O-B10a) |

## Next objectives, in order

1. Finish D4 (sweep, AOT + smoke, capabilities, round-trip) and land the memory.
2. Put section P in writing: the three rows that are now CLOSED (O-B7, O-B8a…i, O-B10b) plus rows 3/4's
   own bound (O-B14) with evidence, and the still-true bounds for the maintainer's acceptance.
3. Ask the maintainer once more for the acceptance words (asked twice so far; no answer recorded).
4. Close defects 2 and 3 if the round budget allows, then run the fresh audit (D1) and write the report.
