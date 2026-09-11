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
| CI | run **#28** on `70241dc`; badge `CI - passing` at 20:21:25; per-job conclusions pending the API rate-limit reset (20:57) |

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
| D0 | CI green on a GitHub runner | **Met** | EVD-237…EVD-245 | per-job conclusions from the run API (rate-limited until 20:57) |
| D1 | Zero open P0/P1 from a fresh audit | **None** | — | the audit wave has not been dispatched (round 8–9) |
| D2 | Tier-0/Tier-1 defects closed with control-failing tests | **Partial** | EVD-243, EVD-244, EVD-247 | the four open rows are located, preserved as patches, and unclosed |
| D3 | Every residual bound closed or accepted in writing | **None** | — | section P does not exist; 13+ bounds in section O await disposition |
| D4 | The final tree re-measures green | **Partial** | EVD-239, EVD-245 | the forced rebuild / sweep / AOT / smoke sequence has not been run at the final tree |
| D5 | Every claim traces to an EVD row | **Partial** | this file + `evidence.md` | the report does not exist yet |

## Next objective

Round 2 as recorded above. Dispatch one diagnosis-and-repair round per open row, starting with the row
whose preserved patch applies cleanly to HEAD and is cheapest to falsify (`wip-r20-exact2`, row 1),
then row 4 (`wip-r22-caps`), then row 2 (`wip-r21-cancel`, which does not apply and needs a rebase).
