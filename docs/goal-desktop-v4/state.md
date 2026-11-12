# Harness State — desktop-v4

- **Round**: 8 (round 7 hardening verified; real-window gesture proof still missing)
- **Goal**: Lovelace Desktop ships as a native Rust scientific scripting workbench that satisfies every
  P0 row of the v4.0 contract with fresh, source-bound GUI evidence and passes all 22 mandatory
  journeys.
- **Definition of done**: 0/19 dimensions MET; 14/171 P0 rows PASS, 1 waived (TST-012), 0 FAIL,
  156 NOT_TESTED.
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
| 3 | Async execution core with typed events and cancellation | Implementer → orchestrator | 1 implementer (stopped) + 1 repairer | 2 | 1 (red suite) | G1✓ G2✓ G3✗ G4✓ G5✓ | **green: 81/81**; submit 154 µs, RunStarted 276 µs, cancel→terminal 20.21 ms; not yet wired into a window |
| 4 | ARC-001 candidate comparison completed (iced, slint) | Probe | 1 implementer | 1 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | iced rejected (no accessibility); slint viable but unmeasured for 5k lines |
| 5 | Cut the core + prototype into the product binary | Implementer → orchestrator | 1 implementer | 3 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | default binary is the new workbench; 90 tests, `--gui-selftest` 7/7, legacy shell behind a flag |
| 6 | Verify the product window and record GUI rows | Orchestrator probe | 0 | 2 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | real engine run observed from the window; LAY-008 + PLT-011 PASS; plot-directory defect found |
| 7 | Harden the product surface (plot path, run/cancel, long run) | Implementer → orchestrator | 1 implementer | 3 | 1 | G1✓ G2✓ G3✗ G4✓ G5✓ | 105 tests; PLF-006 PASS; gui-selftest 8/10 (plot checks env-blocked); kernel cancel gap recorded |
| 8 | Real-window gesture proof (single-process runner) | Orchestrator | 0 | 0 | 0 | G1✓ G2✓ G3✗ G4✗ G5✓ | **attempt failed**: 0 state samples, 136-byte capture; VIS-010/RUN-009 stay open |

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

Round 8: make the real-window gesture provable, then make cancellation reach kernels.
1. **Single-process scenario runner.** Add a `-Action scenario` to `tools/gui-harness.ps1` that does
   launch → wait → focus → set script → Ctrl+Enter → sample UIA → Esc → sample UIA → capture → close
   inside ONE PowerShell process (the foreground is lost between separate tool calls, and UIA
   `ValuePattern` is unsupported on the editor, so the gesture must be driven by real keystrokes in a
   single invocation). Use it to certify VIS-010 (Cancel visible/Run disabled while running) and
   RUN-009 (distinct statuses) with real-window artifacts.
2. **Kernel cancellation (ARC-009 / RUN-006).** Add a live shared cancel flag polled at the same seams
   as the thread-local deadline in `lovelace-symbolics/src/cancel.rs`, set it from the desktop's
   Cancel path, and prove a single long `sqrt(2)` at high precision stops within 2 s. Coordinate with
   the other session, which is editing `lovelace-lang` concurrently: prefer a new module plus the
   smallest possible hook, and never revert its work.
3. Then EDT-002/EDT-015 from the product window with a typing capture and a caret UIA dump.
