# Harness State — desktop-v4

- **Round**: 1 of 40 (Phase 0 complete; round 1 baseline census in flight)
- **Goal**: Lovelace Desktop ships as a native Rust scientific scripting workbench that satisfies every
  P0 row of the v4.0 contract with fresh, source-bound GUI evidence and passes all 22 mandatory
  journeys.
- **Definition of done**: 0/19 dimensions MET (gate register initialised: 0/171 P0 PASS, 0/22 journeys)
- **Stop criteria**: (1) gate `-Verify` exit 0 with all dimensions MET — **unmet**; (2) round 40 —
  not reached; (3) same blocking condition 3 rounds — not triggered.

## Gate status (last run: round 0)

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 18 EVD rows; every round-0 claim resolves to a file:line, a command, or an observed output |
| G2 Falsification | PASS | 0 falsified claims so far (only observations, no hypotheses advanced yet) |
| G3 Coverage | FAIL | 18/19 dimensions at None — no requirement row has evidence yet. This is the expected starting state, not a regression |
| G4 Reproduction | PASS | desktop build reproduced (exit 0, isolated target); `--selftest` reproduced twice (14/15 default env, 15/15 redirected TMP) |
| G5 Honesty | PASS | every "could not determine" from the finished observer is recorded as OQ-002 (and the remaining observers are still running) |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 0 | Contract the goal, freeze the baseline, build a fail-closed gate | Probe (orchestrator) | 0 | 18 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | gate skeleton live; baseline pinned |
| 1 | Census the current implementation and the web reference for all 17 groups | Survey | 1 Intent Auditor + 5 Observers | 6 (so far) | 0 | in flight | in flight |

## Coverage (definition-of-done dimensions)

D01 CON None · D02 ARC None · D03 VIS None · D04 LAY None · D05 EDT None · D06 DOC None ·
D07 RUN None · D08 INC None · D09 WKS None · D10 RES None · D11 SYM None · D12 PLT None ·
D13 LOG None · D14 A11Y None · D15 PLF None · D16 PERF None · D17 TST None · D18 Journeys None ·
D19 Evidence None.

## Next objective

Round 1 (in flight): finish the five-surface baseline census, verify its claims, and synthesise it
into the CON-002 parity register. Round 2 then runs the ARC-001 framework/editor spike.
