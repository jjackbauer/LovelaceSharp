# Harness State — cycle-4

- **Round**: 8 of 30 (the cycle converged; the remaining two jobs are measurements, not new objectives)
- **Goal**: Resolve the three Cycle-4 blockers, close or obtain written maintainer acceptance for
  the four remaining below-A+ gaps, and re-establish every cycle claim on the final tree.
- **Definition of done**: D1 ✓ D2 ✓ D3 ✓ D4 ✓ D5 ✓ D6 ✓ D7 ✓ (executed) D8 partially ✗ D9 ✓
- **Stop criteria**: **not all met** — §4.1 is falsified by the audit and the audit produced 82 FINDING
  rows. The report says so instead of awarding A+.

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 49 EVD rows (EVD-148…EVD-196), every one personally reproduced |
| G2 Falsification | **FAIL by design** | the audit produced 82 FINDING rows, including P0-class wrong values labelled exact; open by choice, not by oversight |
| G3 Coverage | PASS | all nine dimensions met or explicitly bounded in writing |
| G4 Reproduction | PASS | every claim re-run by the orchestrator; three agent deliveries re-verified in isolated worktrees |
| G5 Honesty | PASS | the falsified §4.1 claim, the three pre-existing numeric defects, the stack overflow, the non-idle benchmark caveat and the unrun CI jobs are all recorded |

## Objective ledger (Cycle 4)

| Round | Objective | Mode | Agents | New EVD | Falsified | Outcome |
|---|---|---|---|---|---|---|
| 1 | Re-measure the tree; classify 78 paths | Probe | 0 | 7 | 0 | 0 warnings, 2439/0/6 |
| 1b | Commit the cycle in five stages | Probe | 0 | 1 | 0 | 5 commits, tree clean |
| 1c | Make the SymPy oracle execute | Probe | 0 | 12 | 1 | executed; found N20 then N21 |
| 1d | Browser-verify the Studio UI | Probe | 0 | 2 | 0 | Chrome 152, 0 exceptions |
| 2 | Fix N21/N22 | Implementer | 1 | 5 | 0 | verified; then its regression was caught |
| 3 | §4.1 + §4.3 complex closed forms | Implementer | 1 | 2 | 0 | delivered; §4.1 later falsified |
| 3 | §4.2 complex treatment, all nine rules | Implementer | 1 | 1 | 0 | 9 = 5 + 4, partition asserted |
| 4 | Commit the stage rewrite | Probe | 0 | 3 | 0 | stage 3 now builds; stage rewrite tree-identical |
| 5 | N23 high-precision cos | Implementer | 1 | 4 | 0 | 66× faster, digits byte-identical |
| 6 | Final tree: rebuild, sweep, AOT, smoke | Probe | 0 | 5 | 0 | 2494/0/6, 0 warnings, 28/28 smoke |
| 7 | Adversarial audit vs the re-published binary | Adversarial | 3 | 6 | 2 | 82 FINDING rows; §4.1 falsified |

## Blocker board (all three closed)

| # | Blocker | State |
|---|---|---|
| 1 | Cycle-3 work uncommitted | **CLOSED** — five commits, each verified to build and pass in a clean worktree; the series was rewritten once (DEC-006) and re-verified |
| 2 | SymPy oracle never executed | **CLOSED** — sympy 1.14.0 locally; six corpora, `Failed 0 / Passed 6 / Skipped 0`; the CI job itself still has not run |
| 3 | Studio UI never browser-verified | **CLOSED** — headless Chrome, two real UI round trips, 0 page exceptions, screenshot and DOM captured |

## Next objective

None. The remaining work is measurement (the ShortRun error bars and the P4 symbolic audit) and the
report. Cycle 4 ends **below A+** by its own verdict: the §4.1 exhaustiveness claim is falsified, and
the audit's pre-existing wrong-value defects are named in the report rather than hidden.

## What a Cycle 5 would have to pick up first

1. §4.1: add the eight unlisted refused operations to `capabilities()`, with the same live-envelope
   assertion the existing four carry (`round-09/audit-P2P6-wire.md` has the list).
2. `(a/b)*b != a` and the wrong values flagged `"exact":true` (`2^-100000`, `0^(-1.0)`) — the highest
   severity findings in the audit, all pre-existing.
3. `solve_system`'s documented call form raising `InternalInvariantFailure`.
4. The stack overflow on deep nesting and the `\r` leaking into `print()` output.
