# Harness State — cycle-6

- **Round**: 1 landed; round 2 selected (not yet dispatched)
- **Goal**: make CI green on GitHub's runners again, close the four open Tier-0/Tier-1 rows and their
  residual bounds, and claim A+ only if a fresh adversarial audit cannot falsify it.
- **Definition of done**: **1/6 met** (D0 met on `70241dc`), 4 Partial, D3 None
- **Stop criteria**: all six dimensions verified by me (unmet); round cap 40 (39 unused)

## Round 1 — landed

| Item | Value |
|---|---|
| Objective | D0 — identify and fix the CI failure, push, poll every job to `success` |
| Mode | Adversarial — 1 Implementer + 2 Falsifiers (identical prompts) |
| Commits | `98a9049` (two-pin correction), `70241dc` (this memory) |
| Evidence | EVD-237…EVD-247 (11 new rows) |
| Falsified | 0 — both Falsifiers `Supported`, my 2×2 control agrees |
| Gates | G1 PASS · G2 PASS · G3 PASS · G4 PASS · G5 PASS |
| CI | run **#28** on `70241dc` — **all three jobs `success`** (EVD-248), step #5 ran the full 14-project loop in 435 s |

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | every round-1 claim resolves to `file:line` or a command with its observed output; the two falsifier artifacts were read in full and their citations spot-checked |
| G2 Falsification | PASS | 0 Falsified rows in round 1; the one contrary datum (EVD-246) is a *different* fact about a different tree and is recorded as RISK-002, not averaged |
| G3 Coverage | PASS | D0 met; D1/D2/D3/D5 None-or-Partial and scheduled in rounds 3–11 |
| G4 Reproduction | PASS | I ran the fix on Windows and Linux, built the 2×2 control myself, and read the CI badge flip |
| G5 Honesty | PASS | OQ-001 answered (no hidden failure: both falsifiers ran the full job twice), OQ-002 opened; the second writer (OBS-006) recorded, not hidden |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 1 | Diagnose the red CI run and fix it so all three jobs pass on the pushed HEAD | Adversarial | 3 | 11 | 0 | all pass | **landed** — CI #28 green |

## Round 2 objective, recorded verbatim before dispatch

> **Register the three interrupted worktrees and give each of the four open rows one owner.** For each of
> `c5-exact2` (row 1, `RealLiteral` provenance), `c5-cancel` (row 2, `--cancel-after` in the kernels) and
> `c5-caps` (rows 3 and 4, `evalf(f,0)` and `capabilities()` honesty): take the preserved WIP patch,
> decide **land**, **repair** or **discard** on reproduced evidence only, and record the decision with
> the measurement that justifies it. No WIP patch enters `main` because it exists — each is a hypothesis
> that must fail on a control tree and pass on the fixed one before this round ends.

## Coverage of the definition of done

| ID | Dimension | Status | Evidence | Gap |
|---|---|---|---|---|
| D0 | CI green on a GitHub runner | **Met** | EVD-237…EVD-248 | — (run #28, all three jobs `success` on `70241dc`) |
| D1 | Zero open P0/P1 from a fresh audit | **None** | — | the audit wave has not been dispatched (round 8–9) |
| D2 | Tier-0/Tier-1 defects closed with control-failing tests | **Partial** | EVD-243, EVD-244, EVD-247 | the four open rows are located, preserved as patches, and unclosed |
| D3 | Every residual bound closed or accepted in writing | **None** | — | section P does not exist; 13+ bounds in section O await disposition |
| D4 | The final tree re-measures green | **Partial** | EVD-239, EVD-245 | the forced rebuild / sweep / AOT / smoke sequence has not been run at the final tree |
| D5 | Every claim traces to an EVD row | **Partial** | this file + `evidence.md` | the report does not exist yet |

## Round 2 — landed (triage of the three interrupted worktrees)

| Worktree / row | Verdict | Decisive observation |
|---|---|---|
| `wip-r20-exact2` — row 1, `RealLiteral` provenance | **NEEDS REPAIR** | applies exit 0, builds 0/0, 13 of 32 new cases have teeth (base publishes `sin(pi(30)/6)` = 0.4999…9 **exact:true**); **but one of its own tests fails on the patched tree** (`TruncatingEvaluationStaysInexactTests.cs:360`, `Assert.IsType<NumInt>` vs `NumRat` — a hard-coded tier assertion the file contradicts 50 lines earlier), and `C5Probe.cs` is an assertion-free 2 m 29 s probe that must not land |
| `wip-r21-cancel` — row 2, `--cancel-after` | **LANDABLE AS-IS** | patched `sum`/`matmul`/`prod` stop in 289/228/338 ms with `Cancelled/BudgetExceeded` + a `cancellation` ledger; control 4104/4153/27 658 ms, `ok:true`, no ledger; 9 of 11 new tests have teeth; Run 180/0, Suite 819/0, Dsp 61/0 |
| `wip-r22-caps` — rows 3 and 4 | **LANDABLE AS-IS** | patched whole solution green (5261 passed / 8 skipped); control fails 50 cases; `evalf(sin(1),0)` answers `0.8` at base and is a typed `InvalidArgument/TypeMismatch` after; `capabilities()` 19 entries with `pow.non-integer-exponent` gone and a new `scope` field |

All three reports are in `docs/goal-cycle-6/round-2/`; the scratch trees are `.worktrees/c6-r20`,
`c6-r20-ctl`, `c6-r21`, `c6-r21-control`, `c6-r22`, `c6-r22-control` (each at base `300f6bb`).

## Round 3 objective, recorded verbatim before dispatch

> **Close row 1 — `RealLiteral` provenance — and land it on `main`.** Apply `wip-r20-exact2` **without**
> `Lovelace.Symbolics.Tests/C5Probe.cs`; repair the one broken assertion at
> `TruncatingEvaluationStaysInexactTests.cs:360` into an assertion that is *correct and no weaker* (it must
> still fail if the value stops being the exact rational 1/2); then show, with my own commands, that
> `evalf(sin(pi(30)/6), 40)` and `evalf(sin(pi(1)*1/6), 40)` cross the runner's wire as `exact:false` with no
> rational numerator/denominator, that genuinely exact values stay exact, that the whole 15-project solution
> is green with 0 warnings, and that the new tests still fail on the pristine control tree.

## Round 3 — landed with one claim falsified (row 1 still open)

| Item | Value |
|---|---|
| Objective | close row 1 (RealLiteral provenance) and land it |
| Mode | Adversarial — 1 Implementer + 2 Falsifiers |
| Commit | `e8638c0` — the first of row 1's two routes; the message states the row is not closed |
| Verified (me) | whole solution green except one load-induced wall-clock failure that passes 4/4 alone; wire before/after; control 14/32 fail; one-hunk test diff |
| Falsified | **2 claims** (A and B agree): `evalf(cos(pi(30)), 40|100)` still publishes `-1` as `exact:true` with `-1/1`, and at 100 digits the value is wrong from the 61st decimal (mpmath) |
| Gates | G1 PASS · **G2 FAIL** · G3 PASS · G4 PASS · G5 PASS |
| Cause | `Lovelace.Real/Real.cs:1742-1743` and `:1749-1750` return the special-angle table value for an inexact input that merely matches the angle |

## Round 4 objective, recorded verbatim before dispatch

> **Close row 1's second route: the special-angle fast path must not answer an inexact input with an
> exact special value.** `Lovelace.Real/Real.cs:1742-1743` (and its `ReducingPi` twin at `:1749-1750`,
> table entry `:1935`) returns `negOne`/`One`/`Zero` for an argument that only *matches* the angle
> within a tolerance, so `evalf(cos(pi(30)), 100)` publishes `-1` `exact:true` where the true value is
> −0.99999999999999999999999999999999999999999999999999999999999987355374… (mpmath, 110 dps) and
> `evalf(cos(2*pi(30)), 100)` publishes `1`. The fix must make an inexact input produce an inexact
> answer with the correct digits, keep genuinely exact inputs exact (`sin(0)`, `cos(0)`, integer
> angles), leave `Lovelace.Real.Tests` at 0 failures, and be guarded by a test that fails on the
> pre-fix tree.

## Next objective

Round 4 as recorded above.
