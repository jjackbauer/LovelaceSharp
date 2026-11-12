# Harness State — desktop-v4

- **Round**: 3 of 40 (round 3 red and re-dispatched; round 4 cutover not started)
- **Goal**: Lovelace Desktop ships as a native Rust scientific scripting workbench that satisfies every
  P0 row of the v4.0 contract with fresh, source-bound GUI evidence and passes all 22 mandatory
  journeys.
- **Definition of done**: 0/19 dimensions MET; 11/171 P0 rows PASS, 1 waived (TST-012), 0 FAIL,
  159 NOT_TESTED.
- **Stop criteria**: (1) `-Verify` exit 0, all dimensions MET — unmet; (2) round 40 — not reached;
  (3) same blocking condition 3 rounds — not triggered.

## Gate status (last run)

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 45 EVD rows; every PASS row carries a hash-bound artifact of its required class |
| G2 Falsification | PASS | VAL-001 falsified the v1 contract; repaired. 0 open falsified rows |
| G3 Coverage | FAIL | 0/19 dimensions MET; 9/171 P0 PASS. Expected at round 3 |
| G4 Reproduction | PASS | desktop build, `--selftest`, spike bench, GUI harness, oracle capture and gate modes all re-run by the orchestrator |
| G5 Honesty | PASS | every observer/implementer "could not verify" item is retained; OD-1/OD-2 and the flaky spike bench are recorded |

## Scorecard (from `gate/gate.json`)

| Gate | Rows | PASS | MET |
|---|---|---|---|
| functional (unit/file/static) | 53 | 8 | no |
| visual (gui) | 98 | 0 | no |
| performance (perf) | 20 | 0 | no |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 0 | Contract, baseline, fail-closed gate | Probe | 0 | 18 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | gate live |
| 1 | Census all 17 groups + web reference | Survey | 1 auditor + 5 observers + 1 synthesizer | 10 | 1 (6 claims) | G1✓ G2✓ G3✗ G4✓ G5✓ | 784 OBS; register 229/229; contract repaired; DEC-006 |
| 2 | ARC-001 spike, measured | Implementer → orchestrator | 1 implementer + 1 candidate prober | 12 | 2 (flaky bench, mojibake) | G1✓ G2✓ G3✗ G4✓ G5✓ | working workbench measured; bench flakiness fixed by watchdog (pending report) |
| 3 | Async execution core with typed events and cancellation | Implementer → orchestrator | 1 implementer (stopped) + 1 repairer | 2 | 1 (red suite) | G1✓ G2✓ G3✗ G4✗ G5✓ | code written, **14/81 tests red**; slicer returned zero statements; repair re-dispatched with the observed failures |

## Contract rows closed this run

CON-001 baseline · CON-002 parity register · CON-003 behavioural parity note · CON-004 single engine ·
CON-005 fail-closed gate · CON-006 deviations register · CON-008 three-gate scorecard · CON-009
platform matrix · TST-017 visible status vocabulary. Each carries a class-correct, hash-bound
artifact; see `gate/register.json`.

## Coverage (definition-of-done dimensions)

D01 CON 8/9 P0 PASS (CON-007 open: golden desktop journeys need the product). D02..D19: no dimension
MET yet. D05 EDT and D12 PLT have strong prototype evidence but no product row has a GUI artifact at
a product window, which is the standard the gate enforces.

## Next objective

Round 4 (after round 3 lands): cut the tested execution core and the prototype shell together into
`lovelace-desktop`, so the shipped binary opens the new workbench, runs the real engine on a worker,
and emits the typed event stream — then certify EDT-001/002/015, LAY-002/008, VIS-002/010 and
RUN-001/004/009/011 with screenshots and the GUI driver.
