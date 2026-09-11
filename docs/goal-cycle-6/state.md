# Harness State — cycle-6

- **Round**: 1 of 40
- **Goal**: make CI green on GitHub's runners again, close the four open Tier-0/Tier-1 rows and their
  residual bounds, and claim A+ only if a fresh adversarial audit cannot falsify it.
- **Definition of done**: 0/6 dimensions met (D0 Partial — failure identified and located, not yet green)
- **Stop criteria**: all six dimensions verified by me (unmet); round cap 40 (unused 39)

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | EVD-237…EVD-243 each resolve to a command output or `file:line` read in this session |
| G2 Falsification | PASS | 0 Falsified rows; VAL-001 Supported on two platforms and two sources |
| G3 Coverage | PASS | every D dimension at least Partial except D3 (None — no section P yet) |
| G4 Reproduction | PASS | CI conclusions fetched from the API, the failure re-run on Windows and Linux by me |
| G5 Honesty | PASS | OQ-001 (the loop may hide a later failure) recorded instead of assumed away |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 1 | Diagnose the red CI run and fix it so all three jobs pass on the pushed HEAD | Adversarial | 1 implementer + 2 falsifiers | 7 | 0 | all pass | in flight |

## Round 1 objective, recorded verbatim before dispatch

> **D0 — identify and fix the CI failure, push, poll every job to `success`.** The located objective:
> bring the pushed HEAD to three green jobs by correcting the stale help-contract pins in
> `Lovelace.Console.Tests/ReplOutputTests.cs` (`:63`, `:65`) to the text the product actually declares,
> after reproducing the failure on the pre-fix tree on both Windows and Linux, and with no product
> change and no assertion weakened.

## Coverage of the definition of done

| ID | Dimension | Status | Evidence | Gap |
|---|---|---|---|---|
| D0 | CI green on a GitHub runner | **Partial** | EVD-237, EVD-238, EVD-239, EVD-240, EVD-241 | the fix is not landed; no green run on the final commit yet |
| D1 | Zero open P0/P1 from a fresh audit | **None** | — | the audit wave has not been dispatched |
| D2 | Tier-0/Tier-1 defects closed with control-failing tests | **Partial** | EVD-243 | the four open rows are located but not closed; three worktrees unregistered |
| D3 | Every residual bound closed or accepted in writing | **None** | — | section P does not exist |
| D4 | The final tree re-measures green | **Partial** | EVD-239 (Linux loop) | forced rebuild / sweep / AOT / smoke not yet run at the final tree |
| D5 | Every claim traces to an EVD row | **Partial** | this file + `evidence.md` | the report does not exist yet |

## Next objective

Round 2: register the three interrupted worktrees (`c5-exact2`, `c5-cancel`, `c5-caps`) — take each
one's uncommitted diff, decide land-or-discard on evidence, and give the four open rows a single owner
each. (Round 1 continues until D0 is green.)
