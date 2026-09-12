# Harness State — cycle-6

- **HANDOFF (read this before anything else)**: the cycle is **one decision away** and everything needed for it is on disk. (1) **Landed and verified**: the four rows (D2, rows 3 and 4 re-verified on a pre-fix tree at 31/41 and 7/10 failing — EVD-326), every P0/P1 of waves 3 and 4 (H-1, G-2, I-1, K-1, K-2, K-3), D4's whole checklist on the published binary (forced rebuild 0/0; sweep **5591/0/0**; AOT freshness with zero `.cs` files changed since; five smoke scenarios **24/24**; AOT round-trip **ok=30 bad=0**; capabilities **MATCH=19**) — EVD-323…EVD-327. (2) **Running now**: **audit J** (document conformance) and **audit M** (composed programs), the fourth and fifth personas, against `out/aot/Lovelace.Run.exe` whose product code IS HEAD's (EVD-324). Their deliverables land in `docs/goal-cycle-6/round-22/`. (3) **To finish D1**: triage those two reports against the criteria fixed in `round-20/wave-4-plan.md` — every finding reproduced, each new P0/P1 closed with a control-failing test and then ANOTHER fresh wave; if both come back clean, D1 is met on the standard this cycle set (three personas against the binary matching the final product code, zero open P0/P1). (4) **D3 cannot be finished by any agent**: the seven §P.3 acceptance requests stand unanswered after six written attempts. **A+ must not be claimed until both are resolved** — that is the goal's own condition, not caution. (5) **Loose ends**: two commits are committed locally and unpushed (`9ea10ba`, `882bb88`) so they would not cancel CI #83; re-run `dotnet build --no-incremental` and the sweep after any further product change, and re-publish the AOT binary if a product commit lands.
- **Round**: 22 — **wave 3 is closed end to end and a fourth strategy wave has already found more.** Landed and verified by me: H-1's kink repair (`9852a2f`), G-2's one-text reader (`effd054`), the two documentation findings (`5e85c28`, `8d61b4b`), I-1's cancel-ledger verdict (`fd658d0`, **corrected by `d3a1f50` after my own sweep falsified my first attempt and its test**), the two evalf/printer P2s (`7df2a68`), and the pole/branch-point `series` cluster (`6f7e371`). The 15-project sweep on the tree with all of that is **5556 passed / 0 failed / 0 skipped in 132 s**. **Audit K** — the first wave to drive the product assemblies instead of the CLI — found three new P1s and one P2 in the engine's own host surface, and measurement has since re-graded them: **K-2 CLOSED** (`523240b`: the projection cut 1100 digits to 100 while reporting `Truncated: null`; Studio had the same defect with no scope at all; 15 tests, 6 failing on the control, sweep 5571/0/0), **K-1 RE-GRADED to P2** (EVD-320: the refusal is already `InvalidArgument`/`TypeMismatch` and `setprecision`'s persistence is documented engine contract — what remains is the raw .NET message naming neither number; fix in flight), **K-3** (assumptions shared between engines through one plugin — the last potential P1) and **K-4** (a one-way precision latch) **in flight**. **D1 is therefore not met on two counts**: K-3 is still open, and the gate needs a wave of three to four new personas against the binary published from the FINAL commit — wave 4 has run one persona against an intermediate one. **D4 is MET** on the tree whose product code is HEAD's, against the PUBLISHED binary: forced rebuild 0 warnings / 0 errors; 15-project sweep **5591/0/0 in 130.3 s**; AOT re-publish with freshness (238 s newer than the newest source) and **zero .cs files changed since** — so the wave audits the artefact the final commit builds; the **five CI smoke scenarios 24/24**; the printer round-trip through the published binary **ok=30 bad=0**; capability honesty **MATCH=19 MISMATCH=0** (EVD-323, EVD-324, EVD-325). **Wave 4's third persona ran as audit K** (library-embedding) and its findings are all closed; the **fourth and fifth (J: document conformance; M: composed programs) are running against that binary now** — they decide D1. **D3** is unchanged: seven §P.3 acceptances, five written requests, no answer. A+ not claimed.
- **Round**: 21 — **every wave-3 finding is closed and CI is verifying them**: H-1 (`9852a2f`), G-2 (`effd054`), G-1 (`5e85c28`), H-2 (`8d61b4b`), records `d9095d5`; the 15-project sweep on the fixed tree is **5535/0/0 in 180.5 s** (5497 + the 38 new tests); run **#69** has the AOT-publish and SymPy-oracle jobs `success` with the fast suite still running. **New, found while verifying H-1, and NOT closed: a pre-existing `series` cluster across a POLE or BRANCH POINT** — `series(1/x, x, 0, 2)` = `1 + O(x^2)` where SymPy says `1/x`; `series(1/x^2, x, 0, 2)` errors; `series(sin(x)/x, x, 0, 3)` = `1 + O(x^3)` where SymPy says `1 - x^2/6 + O(x^3)`; `series(sqrt(x), x, 0, 2)` publishes `1/sqrt(0)`. Root cause derived by the orchestrator (`Series.Divide` drops the `(la - lb)` leading-power shift, and the fraction path never asks for enough terms to survive the shift) and dispatched to a fresh fixer worktree with a control-tree requirement. **D0 pending run #69; D1 NOT met (this cluster is open and no clean wave 4 has run); D3 PARTIAL** — the seven §P.3 acceptances are still unanswered after five written requests. A+ not claimed.
- **Round**: 20 — **the third audit wave (new strategies: cross-surface consistency, determinism/idempotence, budget honesty) falsified the closure claim again**: **H-1 (P0)** `series(abs(x), x, 0, 3)` returns a different value every run (`__t<guid>` from `Series.cs:56` survives into the published `piecewise`) **and denotes 0 where SymPy says x**; **G-2 (P1)** byte-identical BOM text reports `position 12` via `--file` but `13` via `--eval`/`--stdin`; two P2s (`SymbolicsPlugin.cs` stale capability prose — **fixed by me**; `--omit-*` emitting `[]` instead of omitting the key). Fixers dispatched for H-1 and G-2 with control trees; **D1 is NOT MET at `d87e910`**. A+ not claimed.
- **Round**: 19 — the second wave's P1s are closed too (published positions `63774da`; print output and `ParseError` `6424e27`), so **all fourteen P0/P1s from both audit waves are closed with control-failing tests**. **CI run #66 on `3d27d61` is green in all three jobs.** **D1 remains NOT MET** because no CLEAN fresh wave has been run against the closure tree, and **D3 remains PARTIAL** (seven bound acceptances unanswered). A+ not claimed.

> **Round 21 objective: close the pole/branch-point `series` cluster (dispatched with the derived root
> cause and a control-tree requirement), then re-measure D4 on the final tree — forced rebuild, 15-project
> sweep, AOT re-publish plus freshness and the five smoke scenarios, capability honesty, printer round-trip
> — and run wave 4 against THAT binary (`round-20/wave-4-plan.md`: protocol conformance, library/embedding
> API, composed programs). D1 needs a wave that comes back clean; D3 needs the maintainer's words for the
> seven §P.3 bounds. A+ stays unclaimed until both resolve.**
>
> Background, round 20: **run the third fresh audit wave against the binary published from HEAD, then
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
