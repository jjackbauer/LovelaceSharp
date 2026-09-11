# Journal — cycle-5 (append-only)

> Entry templates and the confidence vocabulary: `references/evidence-protocol.md` of the
> stretch-goal-harness skill. `OBS` is what was read; `EVD` (in `evidence.md`) is what the
> orchestrator personally reproduced. Only HYP/TODO `Status` fields may be edited in place.

---

### OBS-001: The cycle-4 tree is intact and clean

- **Source**: `git rev-parse HEAD`, `git status --porcelain`, `git log --oneline -5`
- **Fact**: HEAD is `92283054d52f3e64538a3656d76b047bd3d37f56` with an empty `git status --porcelain`;
  the five stage commits of cycle 4 (`b1fb67f`…`3992d80`) are present below it.
- **Implications**: the baseline is a real commit, so every pre-fix test can be run in a worktree at
  `9228305` without in-flight edits.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-201

### OBS-002: T0-2's root cause, read in source

- **Source**: `Lovelace.Symbolics/Printing.cs:509`, `:529`, `:612-637`
- **Fact**: the power-base delimiter decision is `NeedsPowerBaseDelimiter(b) => Prec(b) <= PowPrec`
  with `PowPrec = 3` and every constant falling to `AtomPrec = 4`, so a negative numeric constant
  base is never delimited; the base text itself is `RationalConstantExpr r => r.Value.ToString()`,
  which renders `-1` with a leading unary minus. `-1^x` is therefore emitted for `(-1)^x`.
- **Implications**: the fix belongs in the ONE power-base rule (constraint: one parenthesis decision
  shared by pretty and LaTeX), not in a second special case.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-203

### OBS-003: T0-1's root cause, read in source

- **Source**: `Lovelace.Symbolics/Solvers/Solve.cs:567-599` (SolveElementary) and `:601-687`
  (SolveInverse), decision at `:638-660` and `:665-687`
- **Fact**: for `sqrt(x) + 2 == 0`, `SolveElementary` takes the additive constant term as `rhs = -2`
  and calls `SolveInverse(sqrt(x), -2)`; `SolveInverse` matches `PowerExpr p when p.Exponent is
  RationalConstantExpr pe` with `pe.Value = 1/2`, computes `principal = c^(1/n) = (-2)^2 = 4`, and
  returns `Solved` (line 667) without ever substituting the candidate back into the original
  equation. The residual check that exists in this file (`SatisfiesAll`, `:1002`, and the tolerance
  loop at `:1008-1021`) is used only by the *system* solver.
- **Implications**: the unsound step is the branch inversion at `:647`; the general, bounded repair
  is to verify every candidate produced by an inverse branch against the original equation and to
  stop claiming completeness for any set that lost a candidate.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-204

### DEC-001: Cycle 5 treats the cycle-4 audit's 94 FINDING rows as a work list, not a verdict

- **Decision**: work items are taken from `a-plus-cycle-4-report.md` §5–§6 and the three audit files;
  the audit itself is **not** re-run as evidence — D1 requires a *fresh* audit with new agents and new
  probes.
- **Rationale**: the cycle-4 report explicitly refuses to grade around its P0s; re-running the same
  probes would prove only that the old findings are fixed, which D1 says is insufficient.
- **Alternatives considered**: re-running the cycle-4 probe corpus as the D1 audit — rejected, it is
  a regression check (valuable, and it will be run) but cannot satisfy "new agents, new probes".
- **Related**: OBS-001

### DEC-002: Implementers work in isolated git worktrees and deliver patches

- **Decision**: parallel implementer rounds get their own `git worktree` at `9228305`
  (`.worktrees/c5-*`), build and test there, and deliver a patch plus their raw test output; the
  orchestrator applies the patch to the main tree and re-runs the test itself.
- **Rationale**: concurrent `dotnet` builds in one tree produced `MSB3061` file-lock warnings in
  cycle 4 and cost real time; a worktree cannot be contaminated by in-flight edits (cycle-4 lesson),
  and the patch boundary is what makes "the orchestrator re-ran it" possible.
- **Alternatives considered**: one-at-a-time implementers in the main tree — rejected as too slow for
  a 15-item backlog inside a 40-round cap.
- **Related**: RISK-001

### TODO-001: Close the six Tier-0 defects

- **Task**: T0-1 solver false completeness; T0-2 printer `-1^x`; T0-3 wrong values marked
  `"exact":true`; T0-4 `(1/17)*17 != 1`; T0-5 `solve_system` internal invariant failure; T0-6 stack
  overflow on deep input.
- **Priority**: P0
- **Depends on**: the diagnosis dossiers under `diagnosis/`
- **Status**: Open
- **Related**: OBS-002, OBS-003

### TODO-002: Close the Tier-1 wire-contract defects

- **Task**: `solve_system` must return the documented `SystemSolveResult` (with a `completeness`
  field); short-form `solve`/`limit` must not degrade to `Text`; one arity-error contract; error
  envelopes must carry `elapsedTime`/`timings`; `variables[]` must carry enough to recover meaning.
- **Priority**: P1
- **Depends on**: TODO-001 (T0-5 shares the code path)
- **Status**: Open
- **Related**: —

### TODO-003: Close the Tier-2 round-trip gaps cycle 4 introduced

- **Task**: complex results print as `i` and do not re-parse; `2^(1/3)` and `rootof(...)` print but do
  not parse; `(x/y)/z` re-parses to a different canonical form; `evalf(sqrt(2),5)` returns 100
  decimals; exact `±½√8` is flagged `exact:false`; `abs(i)` is unevaluated; `re`/`im`/`conj` leak an
  internal type name.
- **Priority**: P1
- **Depends on**: TODO-001 (the round-trip property test is the shared instrument)
- **Status**: Open
- **Related**: —

### RISK-001: Parallel implementers produce patches that do not merge

- **Risk**: two worktrees edit the same file (e.g. `Printing.cs` is touched by both the negative-base
  fix and the Tier-2 printer items) and the second patch fails to apply or silently reverts the first.
- **Likelihood**: Medium
- **Impact**: High
- **Evidence**: `Printing.cs` is 57 KB and is the natural home of five backlog items.
- **Mitigation**: assign **disjoint files** per worktree; keep printer work in exactly one worktree at
  a time; apply each patch to the main tree and run that project's suite before dispatching the next.
- **Gate**: G4

### RISK-002: A numeric "fix" changes the meaning of a documented contract

- **Risk**: `Real`'s periodic-decimal representation is load-bearing for the `Real.Tests` suite,
  `Lovelace.Dsp` and the wire's `exact` flag; a repair for `(1/17)*17` could move Dsp results.
- **Likelihood**: Medium
- **Impact**: High
- **Evidence**: cycle 4's `Real` division fix passed 295 `Real.Tests` and broke `Lovelace.Dsp.Tests`.
- **Mitigation**: any change under `Lovelace.Real` re-runs `Lovelace.Dsp.Tests`, `precbench.Tests` and
  the SymPy oracle in the same round, not just its own suite.
- **Gate**: G3

### OQ-001: What is the correct answer for `sqrt(x) + 2 == 0`?

- **Question**: the protocol permits `NoSolutions` and `Unevaluated`; which does the kernel owe when
  every candidate root fails substitution verification — a *proved* empty set, or an honest refusal?
- **Needed evidence**: SymPy's answer for the same equation over the complex field, plus the protocol
  text's own wording on `complete: true`; and a decision on whether a numeric residual check may ever
  license `NoSolutions`.
- **Priority**: P0 blocking
- **Raised by**: Round 1, orchestrator
- **Related**: OBS-003

### OQ-002: Which of the six Tier-0 defects are pre-existing vs cycle-4 regressions?

- **Question**: cycle 4 attributed T0-3/T0-4 to the pre-cycle tree; T0-5, T0-6 and the Tier-1 wire
  items were never attributed.
- **Needed evidence**: run the identical probe against a clean worktree at `35e2609` (pre-cycle-4) and
  compare envelopes byte for byte.
- **Priority**: P1 important
- **Raised by**: Round 1, orchestrator
- **Related**: —

---

## Round 1 — baseline, pre-fix evidence, and four diagnosis rounds

### OBS-004: T0-3's two root causes, located and independently confirmed

- **Source**: `Lovelace.Real/Real.cs:729-768` (`Pow`), `:613` (`Divide`'s loop bound),
  `Lovelace.Suite/NumericOps.cs:258-267`, `Lovelace.Suite/StructuredProjection.cs:128-131`
- **Fact**: (a) `0^(-1.0)` returns 0 because `Real.Pow` line 735 answers "zero base → zero" **before**
  reaching the negative-exponent refusal at line 754-755; the Integer path (`0^-1`) instead delegates
  to `Int.Pow`, which throws, which is why the two spellings disagree. (b) `2^-100000` returns 0
  because `NumericOps.IntegerPower` computes `Rl.Divide(1, 2^100000)` and `Divide`'s digit loop stops
  at `MaxComputationDecimalPlaces` (1000) with the remainder still non-zero — the true expansion needs
  30103 fractional digits, so every generated digit is '0'. (c) the wire's `exact:true` comes from
  `StructuredProjection.RealExact`, whose first clause is `Rl.IsZero(value)` — a *shape* heuristic that
  labels any zero exact however it was produced.
- **Implications**: three separate places, one lie. Fixing only the label leaves a wrong value; fixing
  only the value leaves the label heuristic able to lie about the next case.
- **Confidence**: High
- **Agent**: diagnosis D-C, verified against source by the orchestrator
- **Related**: EVD-205

### OBS-005: `Real` cannot reference `Rational` — the exact fix must live inside `Lovelace.Real`

- **Source**: `Lovelace.Real/Lovelace.Real.csproj` (references Abstractions + Integer only); D-C dossier
  §2.4
- **Fact**: `Real` stores magnitude + sign + exponent + one period block (`Real.cs:153/160/166/172`),
  so a periodic value is exactly recoverable as a fraction with `Int`/Nat arithmetic alone, but the
  `Lovelace.Rational`/`RationalReal` helpers are not on its reference list.
- **Implications**: the T0-4 repair is a `Lovelace.Real` change, which is also why `RealField.Multiply`
  and the limited-precision `LReal64`/`LReal128` twins stay in scope.
- **Confidence**: High
- **Agent**: orchestrator (steered to the numeric implementer)
- **Related**: OBS-006

### OBS-006: T0-6 is an *evaluator* recursion, not a parser recursion

- **Source**: D-D dossier; my own probe `docs/goal-cycle-5/probes/pre2/_summary.txt` and a
  stdout/stderr-separated re-run
- **Fact**: 3000 nested plain parentheses run fine (the parser collapses them; no AST node), and the
  published binary really does emit **0 bytes on stdout**, not 98 — the 98 bytes in my first transcript
  were PowerShell's merged stderr ("Process is terminating due to a StackOverflowException."). The four
  shapes that die at depth 3000 are exactly the four that build a *deep AST*
  (`-`, `abs(`, left-nested `+1)`, `[`), and the first unbounded walk over that AST is
  `Interpreter.cs:280 EvaluateAsync`, whose async frames are heavier than the parser's.
- **Implications**: one guard at the evaluator entry covers all four shapes plus user-function
  recursion; a parser-side limit must stay ≥ the depth the baseline already accepts (3000+).
- **Confidence**: High
- **Agent**: orchestrator (probe) + diagnosis D-D
- **Related**: EVD-208, EVD-213

### OBS-007: The "documented" two-relation `solve_system` form is documented nowhere

- **Source**: greps of `docs/symbolics/dsh-protocol.md` (zero `solve_system` occurrences) and of the
  whole repository (only the list form as an example, at `SymbolicsPlugin.cs:439`), by two agents
  independently
- **Fact**: the cycle-4 report's phrase "the documented call form" is not supported by the repository's
  documents. What *is* contractually binding is `dsh-protocol.md:187-195`: an argument problem "crosses
  as a recoverable argument error … never as an internal invariant failure".
- **Implications**: T0-5 is closed by making the argument-shape violation typed, not by inventing an
  undocumented call form; the final report must correct cycle 4's wording rather than repeat it.
- **Confidence**: High
- **Agent**: orchestrator + diagnosis D-A
- **Related**: EVD-207

### OBS-008: The round-trip property is now an instrument, and it fails in five places

- **Source**: `docs/goal-cycle-5/probes/roundtrip/roundtrip-pre.txt`, produced by
  `docs/goal-cycle-5/run-roundtrip.ps1`
- **Fact**: for each source expression the harness prints the pretty text, re-evaluates it in the same
  scope and compares canonical forms. Pre-fix: `(-1)^x`, `(-2)^x`, `(-1/2)^x`, `(-1.5)^x` — all
  ROUNDTRIP-DIFF (the T0-2 class); `(x/y)/z` and `x/y/z` — ROUNDTRIP-DIFF (rendered `x/(y*z)`, which
  re-parses to a *different* canonical form, because the canonicaliser keeps a flat product of negative
  powers); `sqrt(-1)`, `sqrt(-4)`, `abs(sqrt(-1))` — REPARSE-ERROR (the parser rejects `i`);
  `2^(1/3)` — FIRST-RUN-ERROR (the language cannot evaluate it); `1/2*sqrt(8)` — NOT-SYMBOLIC.
- **Implications**: the printer item is two shapes, not one, and both must be fixed by properties rather
  than by strings; the complex and rational-exponent items are separate rounds.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-203

### OBS-009: The real `sizeof` of the cycle-4 "94 findings" is smaller than the row count

- **Source**: the four repro batches under `docs/goal-cycle-5/probes/`
- **Fact**: the audit's rows cluster into far fewer root causes: 62 numeric rows are the single
  `Multiply`-on-periodic defect; the printer rows are one rule; the wire rows are the record/prose and
  envelope items; `mean`/`max`/`sum` are one missing argument guard and one bad cast.
- **Implications**: the backlog is ~12 bounded changes, not 94 fixes — which is what makes A+ reachable
  inside the round cap.
- **Confidence**: Medium (clustering is the orchestrator's judgement; the individual rows are High)
- **Agent**: orchestrator
- **Related**: DEC-001

### DEC-003: T0-3 is closed by *computing the exact value*, not by refusing

- **Decision**: for a quotient whose exact decimal expansion terminates, compute it exactly (the
  terminating case is decidable by stripping factors of 2 and 5 from the denominator), with a
  documented representation cap; only beyond that cap does the operation refuse with a typed error.
- **Rationale**: returning the exact `1/2^100000` satisfies both the value and the `exact` flag, needs
  no provenance bit on `Real`, and cannot be attacked as "the language used to return a number and now
  throws". SymPy answers the same expression with an exact rational.
- **Alternatives considered**: (a) keep truncating but set `exact:false` — rejected, the value is still
  wrong; (b) always throw on underflow — defensible but strictly weaker and a behaviour regression;
  (c) adding a provenance bit to `Real` — larger than the contract needs and touches every arithmetic
  path.
- **Related**: OBS-004, OBS-005

### DEC-004: the cycle-4 report's "documented form" wording will be corrected, not repeated

- **Decision**: the cycle-5 report states what the contract actually says (the argument-error contract
  at `dsh-protocol.md:187-195`) rather than inheriting cycle 4's claim that the two-relation call form is
  documented.
- **Rationale**: OBS-007; a claim that survives into two reports because neither checked it is exactly
  the failure mode this project's evidence discipline exists to prevent.
- **Related**: OBS-007

### RISK-003: the mid-cycle audit is running against the pre-fix binary

- **Risk**: audit #1's findings will include the six Tier-0 defects I already know about, so its
  cost/benefit is dominated by what it finds *beyond* them.
- **Likelihood**: High
- **Impact**: Low (the excess is confirmation, and anything new is gold)
- **Mitigation**: audit #2 — the D1 audit — runs against the final binary with *fresh agents you have
  never briefed on the fix list*; it is the one that decides the grade.
- **Gate**: G1

---

## Rounds 2–8 — audits, implementations, and what they cost

### OBS-010: The mid-cycle audit was worth its whole cost

- **Source**: `docs/goal-cycle-5/audit1/A1-numeric.md`, `A2-wire.md`, `A3-symbolic.md`, `A4-docs.md`
- **Fact**: four independent agents, given no list of known defects, ran 1188 probe inputs against the
  published binary and produced 24 P0/P1 findings. At least five defect classes were **not on the work
  list the brief carried**: the system solver's false `NoSolutions` for solvable systems; the
  indeterminate-limit answer of 1 where the limit is `e`; the limited-precision fast path certifying a
  truncation exact; the additive collapse to exactly zero; and the narrow `IsExact` in the kernel.
- **Implications**: the same lesson as cycles 3 and 4, a third time — working an item list produces a
  green board, and adversaries find live P0s in under an hour. The audit belongs at the start of a
  cycle, not the end.
- **Confidence**: High
- **Agent**: orchestrator (dispatch + triage)
- **Related**: `audit1-triage.md`

### OBS-011: Three of five implementers shipped an unusable patch file

- **Source**: my own `git apply`/patch inspection for rounds 3–8
- **Fact**: `git diff --cached > c5-patch.diff` in Windows PowerShell 5.1 writes **UTF-16**, which
  `git apply` rejects outright ("No valid patches in input"); and `git add -A` stages the deliverable
  files themselves, so two patches contained the report and the patch recursively. Four of eight patches
  had to be regenerated by the orchestrator with `git diff --cached --output=` and an explicit pathspec.
- **Implications**: a delivery protocol that says "produce a patch" without naming the writer will keep
  producing broken patches in this environment; the fix belongs in the dispatch template.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: RISK-004

### OBS-012: The published binary is the only artefact the audits can share, and it goes stale fast

- **Source**: three agents noted `revision` varying between runs while other agents republished; my own
  round-trip harness ran against the stale binary and reported 12 false failures.
- **Fact**: the AOT binary at `out/aot/Lovelace.Run.exe` was published once (13:41) and every audit
  after the first commits measured a tree that no longer existed. The harness's "prefer the AOT binary"
  rule silently measured the *old* product until I forced the JIT path.
- **Implications**: the freshness guard must name the commit the binary was published from, and any
  measurement must say which artefact it measured. Both are now in the report.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-218

### DEC-005: The endgame is reported honestly rather than stretched

- **Decision**: with the wire P1 cluster open, the complex round-trip unstarted, the `IsExact` defect
  identified but unfixed, and two rounds still in flight, the cycle **does not claim A+**. The report
  names each open row, points at the triage table, and states which of D1–D5 is met.
- **Rationale**: D1 is "a fresh audit cannot falsify the claim"; with eleven documented P1 rows open, a
  fresh audit would falsify it immediately. Cycle 4 set the precedent of refusing to grade around live
  P0s, and this cycle's whole evidence discipline exists to make that refusal mechanical.
- **Alternatives considered**: running a second full audit to "confirm" the open rows — rejected as
  expensive theatre: the rows are already reproduced twice each by independent agents, and a second audit
  that finds them again adds no information. What it *would* add is any defect class nobody has looked
  for; that is the one thing a Cycle 6 should buy with a fresh set of personas.
- **Related**: DEC-001, `audit1-triage.md`

### RISK-004: Patch hygiene, again

- **Risk**: a future round's patch is silently self-including or UTF-16 and the orchestrator applies a
  partial change believing it is complete.
- **Likelihood**: High (observed three times in five rounds)
- **Impact**: High (a partial patch is a wrong tree)
- **Evidence**: OBS-011
- **Mitigation**: every dispatch now names `git diff --cached --output=` and requires the exact
  `--name-status` list; the orchestrator regenerates from the worktree whenever the list is not exactly
  the intended paths.
- **Gate**: G4

### RISK-005: One suite is wall-clock sensitive under load

- **Risk**: `Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly`
  fails when the machine is saturated, which it was throughout this cycle (up to eight agents).
- **Likelihood**: High under load
- **Impact**: Medium (it makes a green tree look red, and it did once)
- **Evidence**: failed in the Suite suite twice under load; **passed 2/2 in isolation on the same tree**
  in 4 s (recorded in EVD-217)
- **Mitigation**: the final D4 sweep is run after every agent is stopped, and the test's behaviour is
  stated in the report rather than hidden.
- **Gate**: G3

### DEC-006: the cycle closes with two rounds outstanding, and says which

- **Decision**: the cycle ends at commit `a94e535` with **T0-3** and the **indeterminate-limit** rounds
  dispatched but not returned. Both are recorded as open, their diagnoses are preserved (`OBS-004`,
  `OBS-005`, and the limits round's located root cause in the dispatch record), and the final binary is
  re-probed so the report cannot claim them closed by silence.
- **Rationale**: a round that has not returned is not a round that succeeded. The alternative — waiting
  past the remaining budget and shipping no report, or writing the report from the agents' claims — is
  worse on both axes this project cares about.
- **Alternatives considered**: killing the two agents and re-doing their work myself — rejected, the
  remaining budget is smaller than one of those rounds; waiting indefinitely — rejected, a cycle with no
  written verdict is not a cycle.
- **Related**: EVD-222, DEC-005

### OBS-013: what the audits cost and what they bought

- **Source**: `audit1/` (4 reports) and the nine commits
- **Fact**: the mid-cycle audit cost roughly one hour of agent time and produced five defect classes the
  work list did not contain; two of them (the system solver's false `NoSolutions`, the additive collapse
  to zero) are now closed, one (the indeterminate limit) is diagnosed and open, and one (the fast path
  certifying a truncation exact) is closed. The audit cost less than the diagnosis rounds that preceded
  the fixes it motivated.
- **Implications**: for a Cycle 6, the audit should run in round 1, before any implementation round.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: OBS-010, DEC-005

### OBS-014: the push falsified one of the cycle's own residual bounds

- **Source**: GitHub Actions run #14 and #13, read through the public API (EVD-223)
- **Fact**: cycle 4 recorded "the CI jobs have never executed on a GitHub runner" and this cycle carried
  that forward into section O as O-B13. It is false: CI #13 was a **success** on the previous remote tip
  and run #14 executed all three jobs on this cycle's push — `sympy-oracle` **success**, `aot-smoke`
  **success**, `fast-tests` **failure**.
- **Implications**: (a) the residual bound is closed by measurement, not by acceptance; (b) **CI found
  something my local sweep could not** — a wall-clock assertion inflated past its budget by coverage
  instrumentation — which is precisely the value a third-party runner has, and the reason the bound
  should never have been stated as a caveat in the first place. A locally green tree is not a green tree.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-223, EVD-224

### DEC-007: a timing assertion gets an uninstrumented CI step, not a looser budget

- **Decision**: `Cos_AtDefaultPrecision_StaysInteractive` is tagged `Category=Timing`; the coverage
  steps in `.github/workflows/ci.yml` exclude that category and a new step runs it **without** a
  collector.
- **Rationale**: raising the 10 s budget would stop the test from catching the N23 class it exists for
  (the defect it guards against took 25 s); deleting or skipping it would remove a real guard. Running a
  wall-clock verdict uninstrumented keeps the assertion, keeps its meaning, and removes the false red.
- **Alternatives considered**: (a) raise the budget to 30-60 s — rejected, it would no longer catch the
  historical regression; (b) drop the test — rejected, N23 is a real class; (c) detect the collector in
  the test — rejected as unreviewable magic in a test.
- **Related**: EVD-224
