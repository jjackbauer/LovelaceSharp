# Harness State — cycle-4

- **Round**: 2 of 30
- **Goal**: Resolve the three Cycle-4 blockers, close or obtain written maintainer acceptance for
  the four remaining below-A+ gaps, and re-establish every cycle claim on the final tree.
- **Definition of done**: D1 met; D2 partial (oracle now executes; one kernel disagreement found and
  under repair); D3 met; D4 measured on the as-received tree (2439/0/6), re-measure pending on the
  final tree; D5 met on the as-received tree; D6–D9 pending.
- **Stop criteria**: all nine met at high confidence, zero Falsified rows, zero P0 open questions.

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 24 EVD rows (EVD-148…EVD-171), every one personally reproduced |
| G2 Falsification | PASS | VAL-001 and VAL-002 both recorded; 0 falsified rows outstanding |
| G3 Coverage | FAIL | D1/D3 met, D2 partial, D4–D9 partial or open |
| G4 Reproduction | PASS | every claim re-run by the orchestrator; the one subagent delivery so far is still in flight |
| G5 Honesty | PASS | N21/N22/N23, RISK-002, the complex-sweep bound and the CI-job caveat are all recorded |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 1 | Baseline rebuild + sweep; classify the 78 paths into 5 stages | Probe | 0 | 7 | 0 | G3✗ | verified; 0 warnings, 2439/0/6 |
| 1b | Commit the cycle in five stages | Probe | 0 | 1 | 0 | G3✗ | 5 commits, tree clean (EVD-159) |
| 1c | Make the SymPy oracle execute (blocker 2) | Probe | 0 | 3 | 1 | G2✓ | **executed: found N20, then a real kernel disagreement** |
| 1d | Browser-verify the Studio UI (blocker 3) | Probe | 0 | 2 | 0 | G3✗ | verified: Chrome 152, 0 exceptions (EVD-169) |
| 1e | Diagnose the oracle disagreement | Probe | 0 | 9 | 0 | G2✓ | cause located: `Lovelace.Real` division (N21), pre-existing |
| 2 | Fix N21/N22 under the located cause | Implementer | 1 | 0 | 0 | — | in flight |
| 3 | §4.1 exhaustive capabilities + §4.3 complex roots | Implementer | 1 | 0 | 0 | — | in flight |
| 3 | §4.2 complex treatment for all nine rules | Implementer | 1 | 0 | 0 | — | in flight |

## Blocker board

| # | Blocker | State |
|---|---|---|
| 1 | Cycle-3 work uncommitted | **RESOLVED** — `b1fb67f`→`3992d80`, tree clean; per-commit verification still pending (TODO-004) |
| 2 | SymPy oracle never executed | **RESOLVED** — executes against sympy 1.14.0; it immediately found N20 (oracle) and N21 (kernel). Residual: the CI job itself still has not run |
| 3 | Studio UI never browser-verified | **RESOLVED** — headless Chrome, 2 real UI round trips, 0 exceptions, screenshot + DOM captured |

## Maintainer decisions taken this session

1. Five staged commits, with the P0 wire revisions revertible on their own (OQ-007) — delivered as
   `160ba4e`, reordered so every commit is green (DEC-003).
2. Fetch a standalone Python + sympy and run the oracle for real — done (sympy 1.14.0).
3. Run the browser check and keep the claim — done.
4. **§4 scope, decided in favour of the ambitious option on all five**: exhaustive capability list;
   complex treatment for all nine falsification rules; reopen `sqrt(-1)`/`(-1)^(1/2)` scope;
   add error bars to the ShortRun benchmark baseline; fix the high-precision `cos` path.
   The §4.2 literal reading is mathematically unsound and is implemented as the sound maximum
   (5 sampled + 4 exclusion-proven) — flagged to the maintainer in the round-2 report.

## Next objective

Land the three in-flight implementer rounds, each verified by the orchestrator in this same round:
run the oracle (expect `Failed 0 / Passed 6 / Skipped 0`), the Symbolics and Suite suites, and the
falsification partition test.
