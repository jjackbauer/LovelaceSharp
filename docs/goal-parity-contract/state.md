# Harness State — parity contract

- **Round**: 2 of 14 (closing)
- **Goal**: an enforced, reproducible parity contract for the C# → Rust migration
- **Definition of done**: 5/10 dimensions met at best (D1, D2, D3, D8 met; D4/D5/D6/D7/D9/D10 incomplete)
- **Stop criteria**: not met; the cycle reports the outstanding dimensions rather than claiming convergence

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G0 Intent | PASS | VAL-001 reproduced; goal corrected; DEC-001 records the user-authorised scope; 18 cycle removals named |
| G1 Evidence | PASS | EVD-001..022, each reproducible |
| G2 Falsification | PASS | 2 claims falsified and fixed (the "nothing removed" claim; the --cases count); 3 product defects found by gates |
| G3 Coverage | FAIL | D5 19/20, D6 14/16, D7 partially validated, D9 212/214, D10 unavailable |
| G4 Reproduction | PASS | every artifact was run by the orchestrator; agent claims re-run |
| G5 Honesty | PASS | OQ-001..013 and CAP-001/002 recorded; unavailable checks never counted as passes |

## Verified baselines (round 2)

| Fact | Value | Source |
|---|---|---|
| Golden fixtures, C# oracle | 20/20 | EVD-006 |
| Golden fixtures, Rust runner | 19/20 | EVD-021 |
| CLI matrix | 14/16 | EVD-022 |
| C# census | 2 105 methods / 6 209 cases | EVD-007 |
| Manifest | 6 209 dispositioned, 20 ported (0.32 %) | EVD-008, EVD-017 |
| Requirement ledger | 42 met / 22 partial / 198 unmet | EVD-009, gate-requirements |
| Project ledger | 4 port / 1 consolidated / 8 partial / 6 absent | EVD-010 |
| Documentation corpus | 212/214 | gate D9 |
| Rust test suite | 293 passed, exit 0 | EVD-005 |
| Differential | see parity/scorecard.json | gate D4 |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 0 | Contract, freeze, measure the gap | Probe | 0 | 4 | 0 | G1✓ | contract written |
| 1 | Oracle, census, ledgers, differential, protocol wave 1 | Adversarial + Implementers | 4 | 8 | 1 | G0✓ G1✓ G2✓ G3✗ G4✓ G5✓ | wave 1 stopped at 4/20, tree fixed by wave 2 |
| 2 | Coverage pilot, protocol wave 2, final gates | Triangulate | 3 | 10 | 1 | G2✓ G3✗ G4✓ G5✓ | 19/20, 14/16, corpus restored |

## Next objective

Round 3 (next cycle): close RUN-010/RUN-012 (cancellation committed-state semantics; Plotting.cs port), then fixtures/complex, then make D5/D6 green and validate the remaining ten mutations.
## Final scorecard (5 000 differential cases)

| Dimension | Status | Detail |
|---|---|---|
| D1 Oracle | PASS | pinned SHA, clean sources, runner answers protocol v1 |
| D2 Census | PASS | 2 105 methods / 6 209 cases, deterministic |
| D3 Ledgers | PASS | manifest 6 209/20 ported; projects 19; requirements 43 met / 28 partial / 191 unmet |
| D4 Differential | FAILED | 5 000 cases: 3 296 match, 1 704 mismatch, 0 errors |
| D5 Fixtures | PASS | oracle 20/20, rust 20/20 |
| D6 CLI | FAILED | 14/16 (RUN-010 cancellation, RUN-012 plot rendering) |
| D7 Mutation | FAILED | 5/7 detected, 6 reverted green; GATE-006 undetected, GATE-008 anchor stale |
| D9 Corpus | FAILED | 212/214 (sym/071, sym/076, unapproved) |
| D10 Rust quality | FAILED | UNAVAILABLE: rustfmt/clippy not installed |
## Round 4 — objective audit

| Anchor | Status | Evidence |
|---|---|---|
| SHA-pinned C# oracle | DELIVERED | D1 PASS; oracle.json + build-oracle.ps1; EVD-013/014 |
| census of every C# test case | DELIVERED | D2 PASS; 2 105 methods / 6 209 cases; EVD-007 |
| parity/manifest.toml mapping every row | **NOT DELIVERED** | 6 209 rows dispositioned, but 20 ported (0.32 %) and 0 user-approved exclusions; 6 189 are deferred |
| machine-readable requirement ledger | DELIVERED | D3 PASS; 262 IDs + 7 obligations; 45 met / 27 partial / 190 unmet; EVD-009 |
| every C# production project inventoried | DELIVERED | D3 PASS; 19/19; EVD-010 |
| 5,000-case differential gate | DELIVERED (gate runs; not clean) | D4: 5 000 cases, 3 296 match / 1 704 mismatch / 0 errors |
| 20 golden protocol fixtures | DELIVERED | D5 PASS; oracle 20/20, rust 20/20; EVD-025 |
| 14 CLI requirements | DELIVERED | D6 PASS; 16/16 rows; EVD-027 |

The objective is **not complete**: the manifest maps 20 of 6 209 cases to a Rust test. Closing that
requires porting tests (Phase 3), not re-labelling rows: a deferred row is not an approved exclusion,
and the gate refuses an exclusion whose owner is not the user.

Scorecard: D1 D2 D3 D5 D6 PASS; D4 D7 D9 D10 FAILED.
## Round 5 update — manifest coverage

- Manifest: 262 of 6 209 census cases ported (4.22 %), up from 20 (0.32 %). 242 added by a verified
  porting batch; 197 refusals documented; 0 approved exclusions.
- Rust suite: 450 tests, 0 failed. Corpus: 212/214 (unchanged).
- Proposed exclusions for the 118 no-crate cases: parity/proposed-exclusions.toml
  (status = pending-user-approval, inert until approved).
- The anchor remains unmet: 5 947 rows are deferred, neither a Rust test nor an approved exclusion.
## Round 6 update — Real batch and a public defect

- Manifest: 1 457 of 6 209 ported (23.47 %), up from 262 (4.22 %). Lovelace.Real.Tests alone:
  1 201/2 537 (47.34 %).
- Rust suite: 1 645 tests, 0 failed. Corpus 212/214, fixtures 20/20 unchanged.
- Porting refusals: 1 336, dominated by the exact-rational defect below.
- **New public defect:** the structured projection publishes 1/6 as numerator 23 / denominator 30
  (oracle: 1/6). It is a live wire defect (REAL-009, WIRE-005) and blocks 1 104 Real rows. OQ-015.
- The manifest anchor is still unmet: 4 752 rows deferred, 0 approved exclusions, OQ-014 open.
## Round 8 in flight

A Suite/Symbolics porting batch is still running (agent 0b80085c-65bf-4fda-9afc-dcb8a972cbb8).
At 14:17 the map held 2 561 entries (verified manifest coverage 40.94 % was measured at 2 542).
The batch writes its tests and map entries when it finishes; its report will appear at
docs/goal-parity-contract/round-8/suite-symbolics-batch.md. Before trusting any new coverage number,
regenerate and re-gate: gen-rust-tests.ps1, gen-manifest.mjs, gate-manifest.mjs, coverage-report.mjs.
Do not count mappings the gate has not resolved.
## Round 9 — user resumed; delegated exclusions; language fixes; coverage 58.40 %

- Goal resumed (armed) on the user's instruction; DEC-003 records the delegation and the 118
  delegated exclusions (owner user, absent subsystems only, reversible).
- Verified: -2^2 = -4 in both runners; f(85) = 0 in both (was 0xC00000FD); corpus 212/214;
  fixtures 20/20; suite 3 404 tests green; manifest 3 626 ported (58.40 %), 118 excluded,
  2 465 deferred; gate failures [].
- Open: f(86) is refused by the oracle but accepted by Rust (recursion boundary); a Run/Suite/
  Symbolics porting agent is still running and may add mappings; scorecard.json needs a re-run.
- Remaining to close the anchor: 2 465 rows (Symbolics/Run/Suite/Complex/Dsp/Representation/Real).
## Round 10 — coverage 60.38 %, printer regression closed

- Verified: pretty_round_trips passes; suite 3,515 green; manifest 3,749 ported (60.38 %),
  118 excluded, 2,342 deferred; gate failures [].
- RISK-001 (two writers): serialized to a single writer; the per-project deltas reconcile with the
  round-9 milestones; no id is missing from the manifest.
- The latex/complex-sqrt production agent is still in flight; its changes must not be counted until
  the suite/corpus/fixtures are green again.
- Remaining to close the anchor: 2,342 rows.
