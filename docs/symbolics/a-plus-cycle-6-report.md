# A+ Convergence — Cycle 6 report

**Repository**: `jjackbauer/LovelaceSharp` · **HEAD at writing**: `185d2e6` (pushed; `origin/main` matches)
**Commits this cycle**: `98a9049`, `70241dc`, `300f6bb`, `1b70a32`, `e8638c0`, `d4d7ccf`, `b009dfe`, `e8b52b3`, `aa27753`, `c5c1437`, `55c8cab`, `be89e55`
**Harness memory**: `docs/goal-cycle-6/{goal,journal,evidence,state,deliverables}.md`
**Amendment**: section **P** at `docs/symbolics/a-plus-cycle-6-amendment.md` (summarised in the plan)
**A+ is not claimed.** Three defects are open and are named in §4 rather than graded around.

---

## 1. What this cycle was handed

A red CI (three jobs; the last green run the previous cycle had confirmed was #15, and twelve commits
had been pushed without re-checking), four open rows each located but unclosed, three interrupted
worktrees with no report, and two audit waves on disk. Everything below was re-measured; nothing was
inherited.

## 2. Definition of done

| ID | Dimension | Status | The command and what it printed |
|---|---|---|---|
| **D0** | CI green on a GitHub runner | **MET** | push then read the run: **#28** on `70241dc` (the fix), **#36** on `be89e55`, **#40** on `d945133` and **#41** on the final HEAD `185d2e6` (all three jobs `success`, 567 s) — each with `Fast accuracy test suites` **success**, `Differential oracle (SymPy installed)` **success** and `Native AOT publish + runner smoke` **success**. Between them the record shows the whole arc: #27 failure (the stale pin), #28 green, #30/#33 cancelled by the 30-minute job timeout, #32 failed at 189 s on a coverage-inflated cancellation assertion, #36 green with the costly corpus running uninstrumented. EVD-237, EVD-238, EVD-248, EVD-265 |
| **D1** | Zero open P0/P1 from a **fresh** adversarial audit | **NOT MET** | Two waves ran against the published binary with new personas and new strategies. **Wave 1** (metamorphic relations, precision/exactness lattice, hostile input shapes, agent workflow/protocol) found **2 P0s and 7 P1s**. **Wave 2** (CLI-surface differential fuzzing, attack-the-fixes), run against the binary published from the FINAL tree, found **3 more P1s and two live cycle-5 P1s**, falsifying the "no P0/P1 outstanding" claim I had written an hour earlier (OBS-022). **All fourteen are now closed**, each with a test that fails on the pre-fix tree and each re-verified by me on the wire: `d9f2a7c` (the `--plot-dir` process abort), `9a671c0` (wrong-shaped arguments; the short `limit` family), `c373583` (the evalf working precision), `c7111de` (the leading digit at the precision boundary), `b3b74b8` (the structured payload's silent 100-decimal cut), `1e6fd72` (the refused allocation, the failed plot write, `len` of a zero-dimension array), `63774da` (every published position, LF/CRLF/CR/BOM), `6424e27` (print output kept on a failing run; `ParseError` on its own layer). D1's bar is a CLEAN fresh audit, and **no third wave has been run against the closure tree** — the rule this cycle learned twice is that a closure wave is not a substitute for one. Five P2s remain and are listed in §4. |
| **D2** | Every Tier-0/Tier-1 defect closed by a test that fails on the pre-fix tree | **3 of 4 rows + 3 defects the cycle found** | row 2: 9 of 11 new cases fail on a pristine control tree, and the CLI probes go from 4104/27 658/4153 ms with `ok:true` to 284/362/268 ms with `Cancelled`; rows 3+4: 50 control-tree failures; row 1: 13 of 32 cases fail on the control tree, but its **second route is open** (P-B1) |
| **D3** | Every residual bound closed with evidence or accepted in the maintainer's words | **PARTIAL** | section P written: nine rows CLOSED with evidence; three rows OPEN as defects; seven rows are scope decisions put to the maintainer **twice in writing** with no answer recorded, so none is marked ACCEPTED |
| **D4** | The final tree re-measures green | **MET** | **re-measured on the final tree with a fresh published binary** (EVD-289): forced `--no-incremental` rebuild **0 warnings / 0 errors**; full 15-project sweep with `LOVELACE_REQUIRE_SYMPY=1` **passed=5473 failed=0 skipped=0** in 134 s (Suite 848, Symbolics 1148, Run 262, Complex 121, Real non-Heavy 2482); AOT publish exit 0, 0 warnings / 0 errors, `out/aot/Lovelace.Run.exe` **5 801 472 bytes, 864 s newer** than the newest code file, the five CI smoke scenarios **SMOKE FAILURES: 0** (run #59's own aot-smoke job is green on this tree as well); capability honesty **19 advertised entries, MATCH=19 MISMATCH=0**; printer round-trip through the **fresh AOT binary** `ok=30 bad=0 other=0`. The earlier sweep's single unexplained failure (EVD-261, §5) did not recur in this run, and its identity is now covered by the workflow's `::error` annotations |
| **D5** | Every claim traces to an EVD row I reproduced | **MET** | `docs/goal-cycle-6/evidence.md` EVD-237…EVD-275; every number in this report is from a transcript cited there. **Provenance note**: the D4 local numbers above were measured on `b009dfe`; the tree has since gained rounds 10–11 (six more test files, three product files), so its end-to-end validation on the FINAL tree is **run #46** — the same three jobs, including the AOT publish, the five smoke scenarios and the full instrumented loop — not the older local sweep. A local re-measure at the final HEAD is the first item of the next round. |

## 3. What closed

1. **The CI breakage.** The `fast-tests` job was red from run #18 to #26 because the round that made
   `solve()` return the same `SolveResult` record `solve_full()` does reworded the builtin's summary and
   its declared return kind, and updated the Suite-side help test but not
   `Lovelace.Console.Tests/ReplOutputTests.cs`, which still pinned "Solves an equation for x." and
   "Returns: Vector | Text". The pins now assert what the product declares, whole-line, and reverting
   either product string fails the test. Reproduced on Windows and on Linux (WSL Ubuntu 24.04, .NET
   10.0.401) before being fixed.
2. **Row 1, first route** (`e8638c0`): `RealLiteral` now carries the route it was built from, so a
   truncation that leaves the numeric tier and comes back through `FromRealExact`/`ToReal` stays
   inexact. The wire went from `evalf(sin(pi(30)/6), 40)` = `0.499…999` `exact:true` with a 30-digit
   rational form to `exact:false` with no numerator/denominator, with the exact controls unchanged.
3. **Row 2** (`b009dfe`): the numeric kernels observe `--cancel-after`; a cancelled statement raises
   `EvaluationCancelledException` and both envelopes carry a `cancellation` ledger reporting budget,
   elapsed, stopped, exceeded and excess.
4. **Row 3** (`d4d7ccf`): `evalf(f, digits)` refuses a count it cannot honour as a recoverable
   `InvalidArgument/TypeMismatch` instead of answering `0.8`/`0`-declared-exact, and `digits = 1` still
   answers.
5. **Row 4** (`d4d7ccf`): `capabilities()` no longer advertises a class whose trigger succeeds, names
   the narrowed class, and lists the three reachable refusals that were missing. Nineteen advertised
   entries, **19/19** reproduce their advertised code and category.
6. **O-B11** (`c5c1437`): user-function recursion is budgeted. `f(436)` and `f(448)` used to kill the
   process with `0xC00000FD` and **0 bytes on stdout**; they now answer a well-formed envelope with
   `DepthExceeded/BudgetExceeded`, and `f(85)` still answers. The trade is explicit — recursion deeper
   than ~85 call levels is refused with a typed error where it used to work up to ~432 and then crash —
   and it is the same trade cycle 5 made for nested input.
7. **The CI budget** (`aa27753`, `55c8cab`, `be89e55`, `1ba8e16`): run #32 failed at 189 s on a
   wall-clock cancellation assertion measured at 29 s under the coverage collector, runs #30/#33 were
   cancelled by the 30-minute job timeout, and run #37 reddened again on a 2.5 s promptness fence around
   a spawned runner. The job now separates the two causes: **`Category=Timing`** (wall-clock verdicts -
   `RealTrigFastPathTests`, `CancellationObservationTests`, `ArrayKernelCancellationTests`,
   `CancellationBudgetTests`) and **`Category=Costly`** (legitimate work the collector multiplies
   seventeen-fold - the truncation corpus, 23 s uninstrumented against 404 s instrumented) leave the
   instrumented loop and run, uninstrumented, in their own step. Nothing is skipped: the totals are
   preserved (Suite 817 + 8 = 825; Run 201 + 5 = 206; Symbolics 1064 + 8 skips + 32 = the project's
   own count).
8. **Section-O bounds that are no longer true**: O-B7, O-B8a…i, O-B10 (second half), and the
   under-claim inside O-B14 — each re-measured on this tree, each CLOSED in section P with its probe.
9. **P-B1 and P-B3** (`78c19d8`): the special-angle path no longer hands an exact value to an inexact
   argument, so `evalf(cos(pi(30)), 40|100)` crosses as `{"value":"-1","exact":false}` with no rational
   form while the pinned VALUES stay; and `1/(3*10^1000)` is the exact periodic `0.000…0(3)` instead of
   `0`. Teeth on both trees: 12/0 vs 8-failed/4-passed, and 6/0 vs 3-failed/3-passed.
10. **The last two flag leaks** (`762aa8c`): a truncated constant (`pi(30)`, `pi(1)`, `e(30)`) is no
    longer published as `exact:true` with a rational form — same digits, honest claim — and
    `ComplexMath.Exp`/`Sqrt` follow the argument's provenance. Teeth: 22/0 vs 15-failed/7-passed and
    7/0 vs 4-failed/3-passed.
12. **Four defects the fresh audit found** (rounds 14–16): the `--plot-dir` process abort
    (`d9f2a7c`: `0xC0000409` with 0 bytes on stdout → a 443-byte `PlotDirectoryError/TypeMismatch`
    envelope, 3 of 4 new cases failing on a pristine control); the wrong-shaped-argument cluster
    (`9a671c0`: 52 internal failures of a 345-probe sweep → **0**, 26 builtins, one central guard);
    the short `limit` family returning prose (`9a671c0`: now the same `LimitResult` record
    `limit_full` returns); and the evalf working precision (`c373583`: `evalf(sinh(34/3), 30)` was
    wrong from the 26th decimal, now exact against mpmath at 20/30/50/100 decimals and for a 145-digit
    integer part).
14. **The audit's remaining P0 and P1s** (round 17): the ambient-precision boundary
    (`c7111de` — `setprecision(31); evalf(sin(pi(30)), 40)` was `…4` and is now `…5`, with the pinned
    `Sin(Rl.Pi) == 0` / `Cos(Rl.Pi) == -1` intact and the cost still ~2 ms per statement); the
    structured payload's silent 100-decimal cut (`b3b74b8` — `pi(1100)` carries 1100 decimals in
    `result.structured` and `variables[]` while the display keeps its documented 100); and the trio
    `zeros(10^9)` → `AllocationRefused/BudgetExceeded`, a failed plot write → `PlotFileError`, and
    `len` of a zero-dimension array (`1e6fd72`).
15. **The CI knife edges** (`7bb34d0`): the Timing step's two red runs were test-side — a promptness
    fence that included the runner's process start, and a demand that a stop INSIDE its budget report an
    excess. The verdict now comes from the kernel's own ledger (cross-checked against the envelope) and
    the excess assertion matches the contract. Verified on Linux, the platform that failed: 5/5 three
    times.

## 4. What is open — and why A+ is not claimed

| # | Defect | The measurement | Class |
|---|---|---|---|
| — | **No P0 and no P1 is open.** The audit's two P0s and seven P1s are closed (see the D1 row and §3.12-16). What remains from the audit is the P2 class: |
| ~~A-P0~~ | **The leading significant digit can be wrong at the ambient-precision boundary.** `setprecision(31); evalf(sin(pi(30)), 40)` → `0.0000000000000000000000000000004` where the true value is `5.0288419716939937…e-31` — a 20 % error in the first meaningful digit, inside the requested 40 places (`setprecision(30)` honestly answers 0; at 32 the last digit is `9` where `0` is correct). Truncation where rounding is required | EVD-280 (audit B) | **OPEN — P0** |
| **A-P1a** | **The machine-readable payload silently truncates every Real at 100 decimals.** `setprecision(1100); pi(1100)` renders 1100 digits in the display but `result.structured.value` carries exactly 100 and no truncation marker — an agent reading the structured value loses the digits it asked for | EVD-281 (audit B) | **OPEN — P1** |
| **A-P1b** | **The last delivered decimal is truncated, not rounded** (`…974943` where mpmath gives `…974944`) — which also corrects my own EVD-276, whose "matching to the digits printed" was wrong | EVD-280 (audit B) | **OPEN — P1** |
| **A-P1c** | `zeros(1000000000)` raises `InternalError/InternalInvariantFailure` ("Insufficient memory…", `recoverable:false`) — an OutOfMemoryException reaching the generic handler where a typed budget refusal belongs (39 GB was free) | EVD-282 (audit C) | **OPEN — P1** |
| **A-P1d** | A failed plot WRITE (`--plot-file ""`, a missing subdirectory) is an internal invariant failure, while a failed file READ stays a typed, recoverable `FileReadError` | EVD-282 (audit C) | **OPEN — P1** |
| **A-P1e** | `len([[],[]])` / `len(zeros(2,0))` are refused with "Array dimensions must be positive, but got 0" although `shape([[],[]]) == [2,0]` and the value prints | EVD-282 (audit C) | **OPEN — P1** |
| — | Also recorded, P2: `--print-budget` is non-monotone (budget 12 → 73 chars, 16 → 31); the protocol document's own `solve` example shows one `timings` entry where the live run has two; `evalf` over-delivers 2000 decimals for a 40-place request; `pi(30)` at `setprecision(20)` leaks a raw .NET message; `subs` at a pole returns `0^-1` symbolically where a direct `0^-1` is typed | audits A and B | **OPEN — P2** |

Seven further bounds (O-B1…O-B6, O-B9, O-B12) are scope decisions. The maintainer was asked twice, in
writing, with the measured behaviour quoted; **no answer was recorded**, so section P marks them OPEN
and none is claimed as accepted. If the maintainer accepts them, D3 becomes MET; if any is rejected,
each is a round of work with a located home.

## 5. The one blemish in D4

Two full sweeps report `Lovelace.Run.Tests passed=190 failed=1` while the same project run standalone
reports exit 0 three times out of three (with and without `LOVELACE_REQUIRE_SYMPY=1`). The case was
never captured because the sweep script writes a raw log only when its summary regex fails — a defect
in my own harness, recorded rather than papered over, and part of why the CI workflow now prints failing
test names as `::error` annotations. That diagnostic has since paid for itself twice: it named run #45's
failure (a stop INSIDE its budget reported as an overrun — a test-side knife edge, `7bb34d0`) and run
#54's (a Windows-only "invalid characters" case that is legal on Linux — `c9bcf0b`). The class is the one
the cycle carries three times now (RISK-002, RISK-003, RISK-006): wall-clock or platform assumptions
inside instrumented runs.

## 6. Method, and where it was adapted

- **The harness ran as designed for rounds 1–3**: one narrow objective per round, recorded before
  dispatch; an Implementer plus two Falsifiers on identical prompts for the claims that gate a
  dimension; my own reproduction of every number before it entered `evidence.md`.
- **The falsification gate earned its keep.** Round 3's claim ("row 1 is closed") was **Falsified by
  both falsifiers**, independently, with the same counterexample from different routes; I reproduced it
  and quantified it against mpmath. Round 1's claim survived two full 16-command CI replicas.
- **Three agents were stopped mid-flight** (rounds 4, 4b and 7). Their work was preserved as patches
  and file copies under `docs/goal-cycle-6/round-4/preserved/` and `.worktrees/c6-ob11`, and the two
  patches that were already independently triaged (r21, r22) were applied, verified, committed and
  pushed by me (DEC-007 records why: the rule's purpose — no unverified delivery enters the record — is
  preserved, the Implementer/Falsifier separation for a mechanical patch application is what is lost).
- **The full four-persona fresh audit (D1) was not run.** The cycle did not have the round budget or a
  stable agent window left after the stops, and running it against a tree with three known open defects
  would have produced confirmation, not information — the same reasoning Cycle 5 recorded in its own
  O.3. D1 is therefore **not met**, and the report says so.
- **Two process lapses are recorded, not hidden**: round 5's objective was written to `state.md` after
  its dispatch (OBS-010), and an early patch-applicability verdict of mine was a PowerShell pipeline
  artifact that I later corrected in a superseding row (OBS-008, EVD-249).

## 7. What a reader should not conclude

- That the exactness problem is solved. One route is closed and guarded by tests that fail on the
  pre-fix tree; the wire still over-claims on the special-angle route (P-B1).
- That CI is green *by construction*. It is green on `70241dc` and on `b009dfe`'s predecessors; the
  final commit's run is cited in §2 when it completes.
- That the four rows were closed by one agent each. Two came from preserved work that had never been
  verified (cycles 5's stopped rounds), one needed a repaired assertion, and one — row 1 — turned out to
  have a second route that only falsification found.

## 7. Rounds 20–22 — the third and fourth waves, and where the cycle actually stands

This section SUPERSEDES the status table in §2, which was written before the third wave ran. The
dimensions are the same; the numbers are the ones measured after it.

| ID | Dimension | Status at the time of writing | Evidence |
|---|---|---|---|
| **D0** | CI green on a GitHub runner | **MET** | **#69** on `9852a2f` green in all three jobs in 10.7 min, after the series kink repair and the positions contract; the runs the fix cycle superseded read `cancelled` because `ci.yml:17-19` cancels in progress, which is policy and not a failure (EVD-307, EVD-311) |
| **D1** | Zero open P0/P1 from a **fresh** adversarial audit | **NOT MET** | **Wave 3** (cross-surface consistency, determinism/idempotence, budget honesty) found **1 P0 + 2 P1 + 4 P2** — all but audit K's below are now closed. **Wave 4's first persona** (library-embedding, the first wave to drive the product assemblies instead of the CLI) found **3 P1 + 1 P2** in the engine's own host surface, and two fixers are on them. **No wave has yet come back clean against a final binary**, so the gate is open (EVD-297, EVD-298, EVD-308, EVD-315) |
| **D2** | Every Tier-0/Tier-1 defect closed by a test that fails on the pre-fix tree | **MET** | rows 1–4 closed in rounds 1–14; every closure carries a pristine-control failure: 33 kink cases, 5 of 6 surface-agreement cases, 5 of 6 pole/branch-point cases, 8 of 14 evalf/printer cases, the cancellation cases, and — re-measured by me on a worktree at the pre-fix commit `1b70a32` — **row 3 FAILED 31 of 41 and row 4 FAILED 7 of 10, against 51/51 on HEAD** (EVD-294, EVD-305, EVD-306, EVD-313, EVD-315, EVD-326) |
| **D3** | Every residual bound closed with evidence or accepted in the maintainer's words | **PARTIAL** | section P now carries **P-1…P-23** closed with evidence, and §P.2 names what is not: audit K's P1s while their fixers run, the recorded P2s, and the **seven §P.3 scope bounds**, for which six written requests have produced no answer. Nothing is marked ACCEPTED, and nothing is silently reduced (EVD-304, DEC-009) |
| **D4** | The final tree re-measures green | **MET** | re-measured on the tree whose product code is HEAD's, against the PUBLISHED binary: forced `--no-incremental` rebuild **0 warnings / 0 errors**; full 15-project sweep with `LOVELACE_REQUIRE_SYMPY=1` **5591 passed / 0 failed / 0 skipped in 130.3 s** (EVD-323); AOT re-publish with **freshness** (238 s newer than the newest source) and a smoke answer (EVD-324); the **five CI smoke scenarios 24/24 checks** and the **printer round-trip through the published binary `ok=30 bad=0`** (EVD-325); **capability honesty MATCH=19 MISMATCH=0** (EVD-318); and `git diff --name-only <publish-tree>..HEAD -- '*.cs'` lists **0 files**, so the publish and the final commit carry the same product code (EVD-324) |
| **D5** | Every claim traces to an EVD row I reproduced | **MET so far** | `docs/goal-cycle-6/evidence.md` EVD-237…EVD-315; every number above is from a transcript cited there, including the two rows that correct my own work (EVD-299, the false finding a shell quoting trap produced; EVD-314, the fix my own sweep falsified) |

### 7.1 The third wave, and why a new strategy is what finds these

Wave 3's two serious findings share one blind spot: a value that is self-consistent **per route and per
run** until you compare two runs or two routes. **H-1 (P0)** — `series(abs(x), x, 0, 3)` returned a
different value on every run (`Guid`-named substitution variable) *and* denoted `0 + O(x^3)` where
`|x|` is `x`; closed in `9852a2f` by resolving the kink the way SymPy resolves it. **G-2 (P1)** —
byte-identical text reported `position 12` through `--file` and `13` through `--eval`/`--stdin`,
because only `--file` consumed the BOM; closed in `effd054` by one reader all three surfaces share.
**I-1 (P1)** — the `--cancel-after` ledger could report a stop that denied its own overrun; closed in
`fd658d0` and corrected in `d3a1f50`. Four P2s followed (stale capability prose, `--help` claiming an
omission the envelope does not perform, `evalf`'s silent clamp, a truncation that split an identifier).

### 7.2 The mistake worth keeping

My first I-1 fix was wrong twice, and **my own sweep is what caught it**: `Lovelace.Run.Tests passed=291
failed=1`, the single failure being the test I had just written. One half was a clock started on the wrong
side of the token; the other half was a test demanding a verdict no clock can guarantee — the OS timer can
fire a little before the engine's stopwatch reaches the budget, which is what the auditor's own 11
instances actually show (95.7–99.7 ms of a 100 ms budget). Both are corrected in `d3a1f50`, and the
disposition is written into §P (P-21) rather than smoothed over. Two further rows correct earlier claims
of mine: EVD-299 (a "defect" that was PowerShell 5.1 stripping quotes before `argv`) and EVD-314.

### 7.3 What the fourth wave adds

Audit K drove the product assemblies directly — outside the CLI — the way Studio and any other host does,
and found what no CLI wave could: **K-1**, one engine where a `setprecision(20)` evaluation permanently
lowers the computation cap so a later *unrelated* evaluation is refused with a raw
`ArgumentOutOfRangeException`, with interleaving changing a valid script's outcome; **K-2**, the library
projection truncating 1100 digits to 100 while reporting `Truncated: null`; **K-3**, assumptions leaking
between two engines through one shared plugin; **K-4**, a one-way precision latch. It also documented what
it could not falsify: 100× determinism, interleaving, 8-thread safety, cancellation-then-reuse, and all
1000 published digits of `sqrt(2)`/`pi`/`e`/`sqrt(3)`/`sqrt(2)+sqrt(3)` matching mpmath.

### 7.4 The A+ decision, in the maintainer's absence

**A+ is not claimed, and the reason is now two-sided.** D1 requires a fresh wave that comes back clean
against a final binary; wave 4's first persona has just come back with three P1s, so that gate is open by
measurement, not by caution. D3 requires the maintainer's own words for seven scope bounds; six written
requests — each quoting the measured behaviour and each offering Accept/Reject/Partial — have produced no
answer, so none is marked ACCEPTED and §P.3 stands as written. What the cycle can say on its own evidence:
**D0 met, D2 met, D5 met so far, D4 partially re-measured, D1 and D3 open**, with every open item named in
§P.2 and every closed one carrying a control-failing test and a command.

### 7.5 Wave 4 is complete, and its third persona falsifies the closure claim

| persona | strategy | result |
|---|---|---|
| **K** | library/embedding — the first wave to drive the product assemblies instead of the CLI | 3 P1 + 1 P2: all closed or dispositioned (K-2 `523240b`, K-3 `34ee4b3`, K-1 re-graded to P2 and closed `2572c59`, K-4 recorded as a monotone flag) |
| **J** | document conformance — every normative sentence of `dsh-protocol.md` and `--help` as a probe | **0 P0, 0 P1**, four P2s, all corrected from measurement (EVD-328) |
| **M** | composed programs — the seams rather than the calls | **3 P1 + 1 P2, all NEW and all still OPEN** (EVD-329) |

**So D1 is not met, and this time the reason is not caution.** M's three P1s are real and I reproduced each
of them on the published binary: a cancelled statement's printed output is lost while `timings[].hasOutput`
says it produced none (the machine-readable flag is false); the documented `f(85)` answers or is refused
depending on whether a sibling statement precedes it inside the same block; and `solve` ignores a session
`assume` that `simplify` honours. None of them is a documentation defect, and none is a variant of
anything the earlier waves found — the composed-program strategy is the fifth in a row to find something
new, which is the strongest argument this cycle has produced for never treating a closure wave as an
audit.

What that leaves: **D0 met** (#83 on `61de73a` green in all three jobs), **D2 met**, **D4 met**, **D5 met
so far** (EVD-237…EVD-329), **D1 open on M-1/M-2/M-3**, **D3 open on the maintainer's silence** — six
written requests, and a seventh one-click form that also went unanswered. **A+ is not claimed.**


