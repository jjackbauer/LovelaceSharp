# Harness State — cycle-5 (final)

- **Round**: 8 of 40 — the cycle closed at the round cap of its own budget, not of the harness
- **Goal**: close every Tier-0/Tier-1 defect with a pre-fix-failing test against independent ground
  truth, accept or close every residual bound in writing, and claim A+ only if a fresh adversarial
  audit cannot falsify it.
- **Result**: **A+ is NOT claimed.** Five of six Tier-0 defects are closed and verified; T0-3 and the
  wire/complex clusters are open, and D1 was deliberately not attempted because open P1 rows make it
  vacuously falsifiable. The report says so; section O of the alignment document lists every bound.

## Gate status (final)

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | EVD-201…EVD-222, each reproduced by the orchestrator |
| G2 Falsification | **FAIL** | audit #1's triage table still has open P0/P1 rows (T0-3, the indeterminate limit, O-B7…O-B9) |
| G3 Coverage | PARTIAL | D2 met for five of six Tier-0 items and for the `solve_system` contract; D1/D3 unmet |
| G4 Reproduction | PASS | every landed round re-run by me in the main tree; every pre-fix claim reproduced in a control worktree I controlled |
| G5 Honesty | PASS | the two rounds that did not return, the flaky test, the patch-hygiene failures and every open bound are recorded, not smoothed |

## Commits (nine; `git log --oneline 9228305..HEAD`)

`34c970c` `3ffc782` `bb9746f` `2bd8785` `8502e5f` `b908d9a` `a03553a` `5382861` `a94e535`

## Final measurements (commit `a94e535`)

Forced rebuild **0 warnings / 0 errors**; full sweep **4849 passed / 0 failed / 0 skipped** with the
SymPy oracle required; AOT published and fresh; the five CI smoke scenarios **28/28**; capability
honesty **16/16**; the round-trip property **30/30** through the published binary.

## Open at the close

T0-3 (`2^-100000` → 0 exact; `0^(-1.0)` → 0; the shape-guessed `exact` flag); the indeterminate limit
(`(1+1/x)^x → 1`, true value `e`); the wire-contract cluster (O-B8); the complex round-trip (O-B9);
`IsExact`'s narrowness (O-B7); user-function recursion depth (O-B11). All are stated in
`docs/symbolics/a-plus-cycle-5-amendment.md` §O.2 and in `audit1-triage.md`.
