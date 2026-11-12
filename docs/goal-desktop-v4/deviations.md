# Recorded deviations — Lovelace Desktop v4.0

CON-006 requires every non-parity choice to have a human-approved rationale, an expiry or review
criterion, and evidence. This file is that record. A deviation here is not a failure: it is a
decision that is visible, owned and reviewable. Anything not listed here is expected to be ordinary
parity.

| ID | Deviation | Owner | Status | Rationale | Review criterion | Evidence |
|---|---|---|---|---|---|---|
| DEV-001 | TST-012 human usability sessions (≥5 target users, 8 tasks, ≥90 % success) are not performed for v1 | user | approved (DEC-006) | An autonomous agent run cannot recruit human subjects; the release rule allows a P0 to be human-waived | Run five real sessions before the next release claim; record results and replace this row | `gate/waivers.json`, `journal.md` DEC-006 |
| DEV-002 | v1 supports Windows only; Linux and macOS are later ports with no support claim | user | approved (DEC-006) | No Linux or macOS host is available to produce the built-and-run GUI evidence CON-009 requires | A GUI test artifact from a second platform before that platform is advertised | `platform-matrix.md` |
| DEV-003 | ARC-001's Linux half of the framework spike is deferred; the Windows spike is complete | user | approved (DEC-006) | Same host limitation as DEV-002 | Re-run the spike scenario set on Linux when a host exists | `round-2/spike-bench.json`, `round-2/arc-001-decision.md` |
| DEV-004 | The desktop terminates a script whose source fails to parse with a diagnostic and an idle run state, instead of reproducing the oracle's behaviour | contract (RUN-014) + agent | approved by the contract text | The oracle never reaches a terminal run state on a syntax error and locks the session (OD-1). Reproducing that would violate RUN-014 and make the product unusable | If the oracle is fixed, re-capture the reference and compare again | `reference/oracle-defects.md` OD-1, `reference/oracle-syntax-hang.json` |
| DEV-005 | Desktop diagnostics may show a code and category that the oracle leaves empty | contract (RUN-014) + agent | approved by the contract text | The oracle's `diagnostics[].code` is empty while the Rust engine carries a coded `LangError`; showing less would be a regression against the contract | Keep the message and 1-based line/column byte-identical to the oracle | `reference/oracle-defects.md` OD-2, `reference/REF-error.json` |
| DEV-006 | The ARC-001 spike was measured from a dev profile (`opt-level=2`), not from a signed release binary | agent | provisional | A release-profile measurement is required by PERF-001 and is not yet produced for the new stack | Re-measure startup, idle and typing from a release build before PERF-001 is claimed | `round-2/spike-bench.json` (`build_profile`) |
| DEV-007 | ARC-001 competitor probes (iced, slint) are run as minimal window/text probes, not full feature spikes | agent | provisional | Full spikes for every candidate would exceed the time box; the deciding criteria are text editing, accessibility, image rendering and window viability | Replace with a full spike if a competitor is chosen or if the egui decision is contested | `round-2/ui-candidates.md` |
| DEV-008 | A single opaque high-precision primitive (`sqrt(2)` at precision 200000) is budgeted as uninterruptible: the live cancellation flag cannot stop it, so Cancel waits for the evaluation to return | agent | provisional | `Real::sqrt` spends its whole time inside `lovelace-real`/`lovelace-nat` big-integer operations (`Nat::pow`, `Nat::to_decimal`, `isqrt`'s Newton loop of `Nat::div_rem`, `Int::cmp`) with no poll seam, and the token lives in `lovelace-symbolics`, which those lower crates cannot depend on without an upward dependency (OQ-003) | Retire when the big-int layer gains a poll seam and `sqrt(2)` at 200000 cancels inside 2 s; re-measure with the round-9 probe | `round-9/kernel-cancel.md` (phase profile, 15.2–63.48 s observations) |

## Rule

A row may only be added by the orchestrator, must cite evidence, and must name a review criterion.
`Status: approved (DEC-006)` means the user explicitly approved it in this session; `approved by the
contract text` means the contract itself requires the desktop to behave this way; `provisional`
means the deviation is a limitation of the current evidence and must be retired before the
corresponding requirement is claimed PASS.
