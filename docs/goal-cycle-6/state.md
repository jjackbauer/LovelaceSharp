# Harness State — cycle-6

- **Round**: 20 — **the third audit wave (new strategies: cross-surface consistency, determinism/idempotence, budget honesty) falsified the closure claim again**: **H-1 (P0)** `series(abs(x), x, 0, 3)` returns a different value every run (`__t<guid>` from `Series.cs:56` survives into the published `piecewise`) **and denotes 0 where SymPy says x**; **G-2 (P1)** byte-identical BOM text reports `position 12` via `--file` but `13` via `--eval`/`--stdin`; two P2s (`SymbolicsPlugin.cs` stale capability prose — **fixed by me**; `--omit-*` emitting `[]` instead of omitting the key). Fixers dispatched for H-1 and G-2 with control trees; **D1 is NOT MET at `d87e910`**. A+ not claimed.
- **Round**: 19 — the second wave's P1s are closed too (published positions `63774da`; print output and `ParseError` `6424e27`), so **all fourteen P0/P1s from both audit waves are closed with control-failing tests**. **CI run #66 on `3d27d61` is green in all three jobs.** **D1 remains NOT MET** because no CLEAN fresh wave has been run against the closure tree, and **D3 remains PARTIAL** (seven bound acceptances unanswered). A+ not claimed.

> **Round 20 objective: run the third fresh audit wave against the binary published from HEAD, then
> close whatever it finds before the tree is called clean.** Wave 3 (three new strategies: cross-surface
> consistency, determinism/idempotence, budget/limit honesty) found **one P0 and one P1**, both NEW:
> (a) **H-1 (P0, two defects in one)** — `series(abs(x), x, 0, 3)` is **nondeterministic** (the
> substitution variable `"__t" + Guid.NewGuid().ToString("N")[..6]` from `Lovelace.Symbolics/Calculus/
> Series.cs:56` survives into the published `piecewise` node, so 4/4 runs differ) **and wrong**: both
> branches are `diff(0, t)` = 0 under the always-false condition `0 != 0`, so it denotes `0 + O(x^3)`
> where SymPy 1.14.0 says `x`. The family is every series at a non-smooth point — `x*abs(x)`,
> `abs(sin(x))`, `abs(x-1)`, and `sqrt(abs(x))` which even publishes `1/sqrt(0)`. The
> `x0 = 1` case is *correct* (condition `1 != 0` is true), so the fix must pin the correct value per
> shape rather than assume uniform wrongness. `Limits.cs:402`'s deterministic `"__t"` never leaks —
> leave its four answers alone.
> (b) **G-2 (P1)** — the same bytes report different positions per surface (`--file` strips the BOM →
> `12`; `--eval`/`--stdin` keep it → `13`); the reading must be taken from the documents and enforced in
> one place so the surfaces cannot drift again.
> Two P2s ride along: the stale capability prose in `SymbolicsPlugin.cs:824-829` (**already corrected by
> the orchestrator**, with the four live refusals re-measured) and `--omit-functions`/`--omit-variables`
> emitting `[]` instead of omitting the key.
> **Then**: re-verify both fixes on the wire myself, re-measure D4, and run a fourth fresh wave with new
> strategies against the final binary — the closure wave is not a substitute for a fresh audit (OBS-022,
> OBS-024).

- **Round**: 18 — the second audit wave (CLI-surface differential fuzzing, against the binary published from the FINAL tree) falsified the "no P0/P1 outstanding" claim: **D1 is NOT MET** with three new P1s and two live cycle-5 P1s recorded; an attack-the-fixes persona is still running. **CI run #59 on `9fa52c5` is green in all three jobs**; D4 re-measured clean on the final tree. A+ not claimed. Historical: round 17 — the four-persona fresh audit ran and is triaged: **four of its findings are closed** (the `--plot-dir` P0 abort, the wrong-shaped-argument cluster, the short `limit` family, the evalf working precision) and **one P0 and five P1s remain open** (named in the report §4). **CI run #56 on `dd436c4` is green in all three jobs**; round cap 40; **A+ is not claimed**
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
| G2 Falsification | **PASS for the work, FAIL for the claim** | the round-3 falsification was closed in `78c19d8`, and the fresh audit's own findings were triaged and four closed; the audit's remaining P0/P1 keep the A+ claim falsified |
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

## Round 10 objective, recorded as dispatched

> **Close the two defects cycle 6 still names, each with a test that fails on the pre-fix tree.**
> (a) **P-B1** — the special-angle path hands an exact value to an inexact argument: the fix is the
> FLAG, not the digits (an inexact argument must produce an inexact result on every path out of
> `ComplexMath.SinCosAtPrecision`), keeping the pinned VALUES `Sin(Rl.Pi) == 0` and
> `Cos(Rl.Pi) == -1` intact and proving the wire now answers `exact:false` for
> `evalf(cos(pi(30)), 40)`. (b) **P-B3** — `1/(3*10^1000)` is returned as `0` and
> `evalf(1/(3*10^1000), 30)` as an Integer `0` declared exact: `Real.Divide` must place the quotient's
> decimal point correctly (the representation can hold the value), with the boundary table checked
> against mpmath and the in-budget neighbours unchanged.

## Round 12 objective, recorded as dispatched

> **Re-measure the one remaining candidate defect (P-B4) properly, and then run the gate that decides the
> cycle: a fresh four-persona adversarial audit against the published binary built from this tree.**
> The audit personas are metamorphic relations (solver residuals, calculus round-trips, rewrite
> soundness, printer round-trip), the precision-and-exactness lattice, hostile input shapes (the classes
> the project forbids: internal invariant failures, crashes, empty stdout), and agent
> workflow/protocol conformance. No persona may re-run cycle 5's probe lists.

## Next objectives, in order

1. Verify and land round 10 (P-B1, P-B3), then re-run the D4 sequence on the new final tree.
2. Run the fresh adversarial audit (D1): four personas with new strategies against the published binary —
   metamorphic/generative, precision-and-exactness lattice, hostile input shapes, agent workflow and the published AOT binary's protocol conformance.
3. Ask the maintainer once more for the seven acceptance words (asked twice; no answer recorded).
4. Reconcile the report and section P with whatever round 10 and the audit produce; claim A+ only if the
   audit cannot falsify it.
