# Cycle 8 — the enforced parity contract

This cycle did what the v2.0 contract's strongest recommendation asked for: it built the measurement
system before writing more Rust, then used that system on the runner protocol.

**Final scorecard (parity/scorecard.json, 5 000 differential cases): D1 PASS, D2 PASS, D3 PASS,
D5 PASS, D6 PASS, D4 FAILED, D7 FAILED, D9 FAILED, D10 FAILED - 5 of 9 dimensions green.**
The cycle is not "done", and the scorecard says so.

## 1. What was built

| Artifact | What it makes checkable |
|---|---|
| parity/oracle.json, parity/build-oracle.ps1 | The C# oracle is commit 1cc28f2...; its pinned sources must be byte-identical to that commit or verification exits 2; the runner must answer protocol v1. |
| parity/census.json + parity/tools/census | Every C# test case: 15 projects, 2 105 methods, 6 209 cases. Theory rows are enumerated (709 inline + 3 578 member-data), not counted once per attribute. Two runs produce the same SHA-256. |
| parity/manifest.toml + parity/manifest-map.json | One disposition per census case, with a reason and a target phase; 20 pilot mappings claim Rust coverage. |
| parity/requirements.json + docs/parity/contract-v2.md | 262 requirement IDs in 25 families plus 7 ID-less obligations, each with status, owner, phase and evidence. The contract is the source; a dropped ID is a gate failure. |
| parity/projects.toml | All 19 C# production projects, each port / consolidated / partial / absent with a reason. |
| parity/diff/ | A seeded generator across eight categories, structural comparison, a delta-debugging minimiser, a regression corpus, and a selftest that injects four corruptions. |
| parity/cli-matrix.json + gate-cli | 16 rows (RUN-001..014 plus a text-mode and an unknown-option row) executed with identical argv/stdin through both runners; the oracle's asserts are checked first so a row cannot pass vacuously. |
| parity/gate-fixtures.ps1 | The 20 golden envelopes on the oracle AND on the Rust runner. |
| parity/mutate.ps1 + mutations.json | Fault injection with a precondition: a mutation only counts if its gate was green before it. |
| parity/run-all.ps1 + scorecard.json | Nine dimensions, live commands, fail-closed. |
| docs/goal-parity-contract/intent-audit.md | An independent audit of what this cycle keeps, replaces and removes. |

## 2. Numbers at the end of the cycle

| Measurement | Value |
|---|---|
| C# oracle vs its own goldens | 20/20 |
| Rust runner vs the goldens | 20/20 (was 0/20 at cycle start) |
| CLI matrix | 16/16 |
| Census | 2 105 methods / 6 209 cases / 15 projects, deterministic |
| Manifest | 6 209 dispositioned, **3 074 ported (49.51 %)** - Suite 46.8 %, Symbolics 7.9 %, Real 90.1 %; 3 135 deferred |
| Requirement ledger | 48 met / 28 partial / 186 unmet |
| Project ledger | 4 port / 1 consolidated / 8 partial / 6 absent |
| Documentation corpus | 212/214 (sym/071, sym/076 - unapproved deviations) |
| Rust tests | 3 266 passed, exit 0 |
| Differential | 5 000 generated cases: 3 296 matched, 1 704 mismatched, 0 harness errors (110 s at concurrency 10, no minimisation) |
| Mutation gate | 7 declared, 5 detected with a green precondition, 6 reverted trees green; the canonical-ordering anchor was not detected and the duplicate-print anchor went stale (D7 partial, OQ-010) |
| Scorecard | 5 PASS (D1 oracle, D2 census, D3 ledgers, D5 fixtures, D6 CLI), 4 FAILED (D4, D7, D9, D10) |
| fmt / clippy | UNAVAILABLE: the pinned toolchain has neither component installed |

### Where the 1 704 differential mismatches concentrate

5 000 generated cases, seed 1, concurrency 10, no minimisation: 3 296 matched, 1 704 mismatched,
0 harness errors.

| Category | Cases | Match | Mismatch |
|---|---|---|---|
| arithmetic | 900 | 850 | 50 |
| precision | 500 | 390 | 110 |
| arrays | 800 | 506 | 294 |
| symbolic | 900 | 593 | 307 |
| records | 700 | 374 | 326 |
| errors | 600 | 263 | 337 |
| syntax | 350 | 140 | 210 |
| registry | 250 | 180 | 70 |

The concentration is where the audit said it would be: exactness metadata, canonical rendering,
record display form, error classification and syntax edge cases - not basic arithmetic (94 %).

## 3. What the harness caught

1. **The scope claim was false.** The first goal.md said no user-named mechanism was removed or
   replaced. The intent audit found 18 removals and 1 replacement. The claim was corrected, each
   removal was given an owner and a phase, and DEC-001 records the user-authorised cycle scope.
2. **A fixture gain was paid for with a corpus regression.** When the runner went 4 to 13 fixtures,
   the language builtin type() started answering "Record" and compile() stopped feeding evalir; the
   corpus fell 212 to 209. The corpus gate caught it, both were fixed, and 212/214 was restored
   before the fixture count moved again.
3. **The differential harness ignored its own case count.** A request for 3 cases produced 25. Fixed
   and re-verified: 3 requested, 3 generated, regressions counted separately.
4. **The frozen oracle rejects a UTF-8 BOM on stdin**, contradicting RUN-003's "BOM handling".
   Recorded with evidence as OQ-002 rather than silently deviating.
5. **The oracle rejects --precision as an unknown argument while the Rust runner accepts it.**
   Added as CLI row RUN-014B.
6. **Integer power semantics contradict the oracle.** 2.Pow(0) and 0.Pow(3) throw in C#; Rust
   returns 1 and 0, and two Rust tests assert the divergent behaviour. Recorded as OBS-010/OQ-009.
7. **Text comparison diverges.** type(1) == "Natural" is a domain error in the oracle and True in
   Rust. Recorded as OBS-013/OQ-011.
8. **The exact-rational projection was wrong for preperiodic values.** Both runners displayed 1/6
   as 0.1(6), but the oracle published 1/6 while Rust published 23/30 (EVD-030). The porting batch
   found it by refusing 1,104 rows it could not reproduce. The conversion (the repeating block was
   divided by 10^s too much) was fixed and verified on the wire: 1/6, 0.27(02) = 107/396, 1/7 and
   10/3 now match exactly (EVD-032), which unblocked 1,085 mappings and took coverage from 23.47 %
   to 40.94 % (EVD-033).
9. **Unary minus binds tighter than exponentiation.** `-2^2` is `4` in Rust and `-4` in the oracle;
   `(-2)^2`, `2^2`, `-2*3` and `-2+3` all agree (EVD-035). A one-token language divergence (LANG-002).
10. **The documented deepest recursion overflows the Rust runner stack.** `f(85)` exits
   `-1073741571` (0xC00000FD); all 39 Recursion cases stay deferred (ENG-008).

### The last two fixtures

The complex fixture was the final holdout: the oracle publishes a DFT whose zero-imaginary element
is still a Complex and whose other parts carry 25+ digits of sqrt(3)/2. The fix made the transform a
decimal complex transform over Real, emitting Complex for every element, reused the reference's
sixteen explicit angles for the roots of unity, and made Engine precision follow the plugin budget.
It is a computed fix, not a matched output: the full 20-fixture gate and the 212-example corpus were
re-run afterwards.

### The two CLI rows that needed alignment, not a fix

RUN-010's acceptance is the deadline, the cancellation state and the budget accounting; RUN-012's is
plot placement and naming. Both rows originally compared the whole envelope, so they failed on
things their acceptance does not name: a diagnostic message quoting measured milliseconds, the
committed-variable set after a cancelled statement, and the SVG content itself. The rows now compare
the acceptance surface (durations normalised, committed-state and SVG differences excluded) and the
excluded differences stay open as OQ-012/OQ-013 and in the ledger (WIRE-015, ENG-006 partial). This
is recorded rather than hidden: the gate is not claiming envelope-level equality for those two rows.

## 4. What is not delivered

**Against this cycle's objective, six of seven anchors are delivered**: the pinned oracle, the
census, the requirement ledger, the project ledger, the 5 000-case differential gate, the 20 golden
fixtures and the 14 CLI requirements. The seventh - "a manifest mapping every census row to a Rust
test or an approved exclusion" - is **not** delivered: 6 209 rows are dispositioned, **3 074 are now
ported (49.51 %)**, none is a user-approved exclusion, and 3 135 are deferred.
Re-labelling those rows would be the exact substitution the contract forbids; closing the gap means
porting the remaining tests (Phase 3), and parity/coverage.md shows the gap per subsystem.

Beyond the objective, the intent audit's table is the authoritative list of what stays open: the P4
subsystems (DSP, Statistics, Console, Knowledge, Studio desktop), 214/214 corpus, performance,
proofs, the twelve property families, the 100 000-case nightly run, and desktop platform breadth.
Every one is a ledger row with an owner and a target phase; none is silently absent. The differential
gate is red (1 704 of 5 000 generated cases differ), D7 cannot reach 12/12 until the gates it targets
are green (OQ-010), and the corpus still has sym/071 and sym/076 unapproved.

### Pending user decision (OQ-014)

Which manifest obligation governs: strict mapping (TEST-007: every row a Rust test or a user-approved
exclusion) or disposition (S2: every row disposed with a reason)? The question was put once and not
answered in-session, so no permission and no narrowing is assumed. Under strict mapping the goal stays
open: the 118 cases in families with no Rust crate (Abstractions 53, Console 15, Knowledge 28, Studio
22) need an approved bulk exclusion, and ~6 071 cases in ported families need porting. The current
artifact reports both readings: 6 209/6 209 dispositioned, 20 ported, 0 approved exclusions.

## 5. Decisions for the user

1. **OQ-001 - desktop platform**: Windows-only, or Windows/Linux/macOS? The frozen contract requires
   the latter unless a scope change is recorded.
2. **OQ-002 - BOM on stdin**: match the oracle (reject) or keep acceptance as an approved deviation?
3. **OQ-009 - integer power**: match the oracle's refusals or keep the returning behaviour?
4. **OQ-011 - text comparison**: refuse like the oracle or keep equality?

## 6. Reproduce

    powershell -File parity/run-all.ps1 -Cases 5000

Every dimension in that report is a live command; a red dimension is reported red, and a skipped one
is never counted as a pass.