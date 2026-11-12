# Acceptance gates are separate — and the gate enforces it

CON-008 forbids substituting one kind of proof for another: engine semantics, AppState units, GUI
workflows, screenshots/theming, and performance/accessibility are independent gates. The v4 gate
implements that separation mechanically rather than by reviewer discipline.

## How the separation is enforced

Every requirement row carries an **evidence class** (`docs/goal-desktop-v4/goal.md`):

| Class | What may certify it | Typical rows |
|---|---|---|
| `unit` | a test binary with concrete input→output assertions | INC-001..010, DOC-001, EDT-014 |
| `file` | an on-disk artifact proving bytes | DOC-002..006, TST-001 |
| `static` | a source-level audit with citations, for absence/shape requirements | CON-001..006, PLF-002/005 |
| `gui` | a real-window screenshot, UIA dump, or GUI-driver log naming the window | VIS, LAY, EDT, WKS, RES, SYM, PLT, LOG, A11Y, RUN |
| `perf` | a measured numeric result with command and machine recorded | PERF, RUN-005/008/016, PLT-003/012 |

A `PASS` row must attach at least one artifact of its required class, with a SHA-256 recorded and a
commit that is an ancestor of the verified HEAD. A headless `--selftest` log therefore cannot certify
an editor row, and a screenshot cannot certify a cache-invariant row.

## Three scorecard gates

`tools/desktop-gate.ps1` reports the three gates independently in `gate/gate.json`:

- **functional** — rows whose class is `unit`, `file` or `static`;
- **visual** — rows whose class is `gui`;
- **performance** — rows whose class is `perf`.

Each gate reports `pass / rows / met`. No gate can borrow another's evidence, because a row only
counts toward the gate named by its class. `dimensions` separately reports the 19 product dimensions
(CON, ARC, …, journeys, release evidence), each `MET` only when all of its P0 rows pass.

## What is still missing for CON-008 itself

CON-008 is about the *scorecard existing and being honest*, not about every gate being green. The
scorecard is generated on every gate run; this note exists so a reviewer can see the separation rule.
Gate green-ness is tracked in `state.md`.
