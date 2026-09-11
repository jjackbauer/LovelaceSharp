# Harness State — cycle-4

- **Round**: 10 of 30 — **objective complete; the cycle converged**
- **Goal**: Resolve the three Cycle-4 blockers, close or obtain written maintainer acceptance for the
  four remaining below-A+ gaps, and re-establish every cycle claim on the final tree.
- **Definition of done**: **D1 ✓ D2 ✓ D3 ✓ D4 ✓ D5 ✓ D6 ✓ D7 ✓ D8 ✓ D9 ✓** — all nine met, each with
  a command the orchestrator ran on the final tree.
- **Stop criteria**: met for the cycle's *claims* (zero unremediated falsified claims, every dimension
  met or bounded in writing). The project itself remains **below A+** because the adversarial audit
  found P0-class product defects; the report says so and names them rather than awarding a grade.

## Objective for this round (recorded before dispatch)

> Close §4.1 honestly: make `capabilities()` advertise every refusal class the adversarial audit
> found, transcribing each `code`/`category`/`message` from the live call, and **repair the two
> refusals that are currently malformed** — integration reports `Unevaluated` with *empty*
> diagnostics, and the symbolic `plot` path dies as an `InternalInvariantFailure` — rather than
> advertising a bug as a capability. Acceptance: every advertised entry is asserted against the live
> envelope, and a re-probe of all eight classes confirms it.

Baseline captured before any change: `round-10/pre-change-baseline.txt` (EVD-197).

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 53 EVD rows (EVD-148…EVD-200), every one personally reproduced |
| G2 Falsification | PASS | the two falsified claims of this cycle (the stage order; §4.1's exhaustiveness) are both remediated and re-tested; the audit's 94 FINDING rows are *product defects*, recorded as evidence and reported, not unremediated claims |
| G3 Coverage | PASS | all nine dimensions met or explicitly bounded in writing |
| G4 Reproduction | PASS | every claim re-run by the orchestrator; three agent deliveries re-verified in isolated worktrees, and the §4.1 closure verified by my own falsification test rather than the implementer's suite |
| G5 Honesty | PASS | falsified claims, pre-existing defects, the non-idle benchmark caveat, the unrun CI jobs and the three not-advertised refusal classes are all recorded |

## Objective ledger (Cycle 4)

| Round | Objective | Mode | Agents | New EVD | Falsified | Outcome |
|---|---|---|---|---|---|---|
| 1 | Re-measure the tree; classify 78 paths | Probe | 0 | 7 | 0 | 0 warnings, 2439/0/6 |
| 1b | Commit the cycle in five stages | Probe | 0 | 1 | 0 | 5 commits, tree clean |
| 1c | Make the SymPy oracle execute | Probe | 0 | 12 | 1 | executed; found N20, then N21 |
| 1d | Browser-verify the Studio UI | Probe | 0 | 2 | 0 | Chrome 152, 0 page exceptions |
| 2 | Fix N21/N22 | Implementer | 1 | 5 | 0 | verified; its own regression then caught |
| 3 | §4.3 complex closed forms + §4.1 list | Implementer | 1 | 2 | 0 | delivered; §4.1 later falsified |
| 3 | §4.2 complex treatment, all nine rules | Implementer | 1 | 1 | 0 | 9 = 5 sampled + 4 exclusion-proven |
| 4 | Rewrite the stage series (DEC-006) | Probe | 0 | 3 | 0 | stage 3 builds; trees identical |
| 5 | N23 high-precision `cos` | Implementer | 1 | 4 | 0 | 66× faster, digits byte-identical |
| 6 | Final tree: rebuild, sweep, AOT, smoke | Probe | 0 | 5 | 0 | 2494/0/6, 0 warnings, 28/28 smoke |
| 7 | Adversarial audit vs the re-published binary | Adversarial | 3 | 6 | 2 | 94 FINDING rows; §4.1 falsified |
| 9 | Close §4.1 (this round) | Implementer | 1 | 1 | 0 | closed |
| 10 | Republish, re-measure, re-check the audit on the final binary | Probe | 1 | 3 | 0 | 2503/0/0, 0 warnings, 28/28 smoke, 16/16 capabilities, findings still reproduce |

## Blocker board — all three closed

| # | Blocker | State |
|---|---|---|
| 1 | Cycle-3 work uncommitted | **CLOSED** — five verified stages (`b1fb67f`→`fca8277`), rewritten once for a non-building wire commit and re-verified in clean worktrees |
| 2 | SymPy oracle never executed | **CLOSED** — six corpora, 57 cases, `Failed 0 / Passed 6 / Skipped 0` |
| 3 | Studio UI never browser-verified | **CLOSED** — two real UI round trips, 0 page exceptions, screenshot and DOM captured |

## Next objective

None for Cycle 4 — the objective is complete and every dimension is verified on the final tree. The
work that remains belongs to a Cycle 5 and is listed below; the goal is being marked complete with the
project explicitly **not** awarded A+.

## What a Cycle 5 must pick up first (unchanged by this round)

1. The two P0-class defects the audit found and I reproduced: the solver claiming `complete: true` for
   an unsatisfiable equation, and the printer rendering `(-1)^x` as `-1^x`, which re-parses to a
   different value.
2. Wrong values labelled `"exact":true` (`2^-100000` → 0; `0^(-1.0)` → 0) and the failing round trip
   `(1/17)*17 != 1` — all pre-existing.
3. `solve_system`'s documented call form raising `InternalInvariantFailure`; the native stack overflow
   on 3000-deep nesting; the `\r` leaking into `print()` output.
4. The gap this cycle introduced: complex closed forms render as `i`, which the parser rejects, so
   they do not round-trip.
