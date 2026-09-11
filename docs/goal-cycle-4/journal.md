# Goal Journal — cycle-4

> Append-only. See the stretch-goal-harness evidence protocol for entry format.
> Only in-place edits permitted: a HYP `Status` flip and a TODO `Status` flip.

---

<!-- New entries go below this line -->

### OBS-001: Cycle-3 ended with 78 dirty paths, not the ~40 the brief estimated

- **Source**: `git status --porcelain` at `C:\Users\ricar\dev\LovelaceSharp`
- **Fact**: HEAD is `35e2609f541c14d3933987657ec27909e7424d11` on branch `main`; the working tree
  carries 78 dirty entries (44 modified tracked files plus 34 untracked entries, several of which
  are directories expanding to hundreds of files).
- **Implications**: The brief's "~40 modified or new paths" understates the exposure. Any staging
  plan must enumerate by path, not by the brief's estimate.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-148, EVD-149, RISK-001

### OBS-002: Two of the three "blockers" are not environmental impossibilities

- **Source**: `Invoke-WebRequest` HEAD to `pypi.org`/`github.com` (both HTTP 200); `Test-Path` on
  Chrome and Edge executables (both FOUND); `Get-Command winget` (present).
- **Fact**: The machine has working network egress, a package manager, and two installed
  browsers. The Cycle-3 report's premise — "no Python in this environment" and "the panels have
  never been loaded in a browser from this environment" — is true of the *absent tools*, not of the
  *capability*. A self-contained Python and a headless browser are both obtainable/usable here.
- **Implications**: Blocker 2 and Blocker 3 are self-resolvable without a GitHub runner. Only
  Blocker 1 (the commit) genuinely required a human decision.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-151, EVD-152, DEC-002

### DEC-001: Lay a non-destructive safety net before any commit work

- **Decision**: Before touching the commit plan, create a real commit object holding the entire
  working tree (`git add -A` → `git write-tree` → `git commit-tree -p HEAD` → tag
  `cycle-3-safety-net` → `git reset --mixed HEAD`), leaving the working tree byte-identical.
- **Rationale**: RISK-005 has been open for two cycles; the largest exposure is an accident, not a
  defect. This net captures tracked *and* untracked paths without ever moving a file, so no
  failure mode of the net itself can lose work. A `git stash -u` would have moved files and could
  strand a half-applied tree.
- **Alternatives considered**: (a) `git stash push -u` + `apply` — rejected, it mutates the tree
  and a failed apply leaves a mixed state; (b) one immediate WIP commit on `main` — rejected, it
  pre-empts the maintainer's chosen split; (c) filesystem copy of the repo — rejected as slower
  and blind to git's own object model.
- **Related**: EVD-150, RISK-005 → RISK-001

### DEC-002: Maintainer decisions recorded for the three blockers

- **Decision**: (1) Commit split = **5 staged commits** per brief §3's suggested split. (2) SymPy
  oracle = **fetch a standalone Python + sympy locally** and run the corpora for real. (3) Studio UI
  = **run the headless browser check** and keep the claim.
- **Rationale**: Direct answers to `ask_user_question` in this session. The brief's own note says
  Blocker 1 must be the user's call (OQ-007) and that the browser question must be asked rather
  than silently dropped.
- **Alternatives considered**: single commit; CI-only oracle; retiring the browser claim. All
  offered to the maintainer and not selected.
- **Related**: OBS-002, OQ-007 (cycle-3), TODO-001, TODO-002, TODO-003

### OQ-001: Are the five commit stages independently buildable?

- **Question**: Stage 3 (P0 wire revisions) and stage 4 (correctness fixes) touch kernel and
  protocol code that stage 2's golden fixtures and stage 5's tests consume. Does each intermediate
  tree compile and pass its own suites on its own, or does the split need a file moved between
  stages?
- **Needed evidence**: `git worktree add` at each stage commit, then build there and run that
  stage's affected suites.
- **Priority**: P1 important
- **Raised by**: Round 1, orchestrator
- **Related**: DEC-002, TODO-001

### RISK-001: Committing in five stages can strand a non-building intermediate tree

- **Risk**: A reviewer checking out stage 3 alone finds a tree that does not compile, which is
  worse than one honest commit. Stage boundaries run through shared kernel/protocol code.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: OQ-001; brief §3 "verify each stage builds and its suites pass before committing it"
- **Mitigation**: Before each stage commit, build the cumulative tree; after the series, verify each
  intermediate commit in a separate `git worktree` and move files between stages if a boundary
  does not stand alone. The safety net (`cycle-3-safety-net`) makes the whole series redoable.
- **Gate**: —

### TODO-001: Commit the cycle in five stages, each verified

- **Task**: Stage 1 harness/cycle docs → 2 runner test project + goldens + slnx/CI → 3 P0 wire
  revisions with regenerated goldens → 4 the six correctness fixes (N13/N14/N15/N17/N18/N19) →
  5 remaining items and their tests. Build and run affected suites before each commit.
- **Priority**: P0
- **Depends on**: EVD-150 (net in place); D4 baseline sweep
- **Status**: Open
- **Related**: DEC-002, OQ-001, RISK-001

### TODO-002: Make the SymPy oracle execute its corpora for real

- **Task**: Obtain a self-contained Python + sympy, run the differential oracle with
  `LOVELACE_REQUIRE_SYMPY=1`, and record the observed pass/fail/skip counts and the sympy version.
- **Priority**: P0
- **Depends on**: nothing
- **Status**: Open
- **Related**: DEC-002, OBS-002

### TODO-003: Browser-verify the Studio UI

- **Task**: Start `Lovelace.Studio`, drive headless Chrome/Edge at it, capture console errors, DOM
  state and screenshots for each panel as evidence.
- **Priority**: P0
- **Depends on**: a release build
- **Status**: Open
- **Related**: DEC-002, OBS-002

### OBS-003: The tree as received is green and 53 tests larger than the Cycle-3 report records

- **Source**: `docs/goal-cycle-4/round-01/suite-counts.txt` (my own sweep, 15 projects) and
  `docs/goal-cycle-4/round-01/forced-rebuild.log`.
- **Fact**: 2439 passed / 0 failed / 6 skipped in 99.4 s, against the Cycle-3 report's 2386. The
  forced full rebuild is 0 warnings / 0 errors in 12.08 s. Project-level deltas: Symbolics 740 → 791,
  Suite 640 → 642; every other project unchanged. The 6 skips are the SymPy differential oracle.
- **Implications**: The brief's warning not to quote 2386 as current is correct and now quantified.
  The extra 53 tests arrived without a full sweep behind them; this is the first end-to-end
  measurement of that state, and it is green. D4's baseline is now a measured number, not an inherited
  one.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-155, EVD-156, EVD-157

### VAL-001: OQ-001 — the brief's stage order cannot be green (Falsified)

- **Target**: OQ-001 ("are the five commit stages independently buildable?")
- **Method**: Read the runner test project's assertions and corpus directly, then compare against
  which stage the brief assigns them to.
- **Evidence examined**: `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:84` asserts
  `fields["status"]["kind"] == "Enum"`; `:39` pins a `capabilities` fixture;
  `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj:24` adds a `Lovelace.Run` reference for
  the capabilities honesty test; the diff keyword map shows `Lovelace.Symbolics/SymbolicsPlugin.cs`
  carrying CANCEL + CAPS + LATEX + LIMIT-EXISTS + N2 + ARITY in one file.
- **Result**: Falsified — for the brief's order (runner project as stage 2, wire revisions as stage 3).
  The runner test project cannot pass before the wire revision *and* the `capabilities()` item exist,
  so committing it second produces a knowingly-red commit.
- **Conclusion**: The series is reordered (DEC-003). Every commit is to build and pass its own
  suites; the wire revision stays a single revertible commit, which is what OQ-007 actually asked for.
  A fully green file-level partition is still only a hypothesis and is verified per commit in a
  `git worktree` (TODO-004).
- **Related**: EVD-158, DEC-003, TODO-001

### DEC-003: Order the five commits so each one is green, not as the brief listed them

- **Decision**: Order = (1) harness/cycle records → (2) P0 wire revisions → (3) the correctness fixes
  N13/N14/N15/N17/N18/N19/N2 → (4) remaining items 6–13, 17 + analyzer hygiene → (5) the runner test
  project, golden fixtures and slnx/CI wiring **last**. Implementation files go to the earliest stage
  that needs any part of them; test files go to the latest stage their assertions touch.
- **Rationale**: VAL-001 — the brief's order makes stage 2 unable to pass. The approved option's own
  wording was "verify each stage builds and its suites pass before committing it", and the harness's
  G2/G4 gates forbid shipping a knowingly-red commit. The substantive requirement from OQ-007 — the
  P0 wire revisions as their own revertible commit — is preserved exactly.
- **Alternatives considered**: (a) keep the brief's order and document a red stage 2 — rejected, a
  knowingly-failing commit is worse than a reordered green one; (b) hunk-level splitting of
  `SymbolicsPlugin.cs` and `Printing.cs` — rejected for now because it creates intermediate file
  states that were never compiled as such, trading buildability for tidiness; each such file's commit
  message names every concern it carries instead; (c) one single commit — offered to the maintainer
  and not selected.
- **Related**: EVD-158, VAL-001, RISK-001

### TODO-004: Verify each stage commit in a git worktree

- **Task**: For stages 2–5, `git worktree add` at the commit, `dotnet build -c Release`, then run the
  affected test projects in that clean checkout. A red stage means the offending file moves to the
  next stage and the two commits are rewritten.
- **Priority**: P0
- **Depends on**: TODO-001
- **Status**: Open
- **Related**: DEC-003, VAL-001, RISK-001

### OBS-004: The SymPy oracle's first-ever execution found a defect — N20, in the oracle itself

- **Source**: `docs/goal-cycle-4/round-01/oracle-run.log`; the traceback names
  `File "<string>", line 3` and `NameError: name 'sin' is not defined`; the failing program is
  `sympy.diff(sin(x**2), x)` and `sympy.limit((sin(x)/x), x, 0, dir='+')`.
- **Fact**: With python3 3.12.14 + sympy 1.14.0 on PATH and `LOVELACE_REQUIRE_SYMPY=1`, the run
  reports **Failed 2 / Passed 4 / Skipped 0 / Total 6** in 12 s. Both failures are the same root
  cause: the generated SymPy program prelude does `import sympy` only, so every name the corpora use
  unqualified (`sin`, `cos`, `Matrix`, …) raises `NameError`. The derivative and limit corpora
  therefore compared **nothing** at all. Solve, roots, factorisation and matrix operations passed.
- **Implications**: The differential oracle had never executed anywhere, and the first execution
  proves why that mattered: two of its six corpora were incapable of comparing, and the CI job that
  was supposed to run them has never run. It also demonstrates the no-silent-pass design working —
  the failure surfaced as a hard failure, not a green run.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-159, EVD-160, DEC-004, HYP-001, TODO-002

### DEC-004: Fix N20 in the oracle prelude, not in the corpora

- **Decision**: `SympyOracle.cs` (`SympySession.Eval`) now emits
  `import sympy` **+ `from sympy import *`** before the generated program, so the corpora's natural
  SymPy syntax resolves. The corpora themselves are untouched.
- **Rationale**: OBS-004. The defect is in the generated program, not in the corpus data; qualifying
  every function name across six corpora would be a larger, more error-prone change that would also
  make the corpus unreadable as mathematics. The symbols are bound *after* the prelude, so the star
  import cannot shadow `x`/`y`.
- **Alternatives considered**: (a) rewrite the corpora to `sympy.sin(...)` form — rejected as
  invasive and no more correct; (b) delete the two failing corpora — rejected outright: that is
  exactly the "weaken a test to reach green" move constraint 6 forbids, and it would have hidden
  N20.
- **Note**: this widens what the oracle can compare and weakens no assertion. It is scaffolding,
  not product code, and it is the orchestrator's own change — recorded as such rather than
  attributed to a subagent.
- **Related**: OBS-004, HYP-001

### HYP-001: With the prelude fixed, all six corpora compare against SymPy and agree

- **Claim**: Re-running with `LOVELACE_REQUIRE_SYMPY=1` yields 6 passed / 0 failed / 0 skipped, i.e.
  the kernel agrees with SymPy on derivatives, solve, roots, factorisation, limits and matrix
  operations.
- **Supporting OBS**: OBS-004 (4 of 6 already passed; the 2 failures were a NameError, not a
  mismatch).
- **Why it matters**: If true, the project's most exposed unverified surface becomes verified. If
  false, the disagreements are the most consequential findings left in the project, and each one
  needs triage: kernel wrong, or corpus's SymPy expression wrong.
- **Falsification strategy**: Run the oracle again and read the summary line; then, for any
  non-zero failed count, read the `MISMATCH`/`does not satisfy` triage messages, which by
  construction name both forms, the sample point and both values.
- **Status**: Falsified — see VAL-002
- **Confidence**: Medium

### VAL-002: HYP-001 — the oracle's first real comparison found a genuine disagreement (Falsified)

- **Target**: HYP-001 ("with the prelude fixed, all six corpora compare and agree")
- **Method**: Re-ran the oracle; for the one mismatch, walked the failing expression node by node
  with a standalone console probe outside the repo (six probes, each isolating one hypothesis), then
  compared the value against SymPy ground truth using digit-exact comparison that never renders.
- **Evidence examined**: `oracle-run-after-N20-fix.log` (Failed 1 / Passed 5);
  `docs/goal-cycle-4/round-01/probe-negative-power.log`; probe output showing
  `Divide(NumRat 1, NumReal 0.99862888…)` = 0; the minimal-pair table in EVD-164; the identical probe
  run against a `35e2609` worktree (EVD-165).
- **Result**: Falsified. Five of the six corpora agree. The derivatives corpus disagrees on exactly
  one case, and the cause is **not** the calculus: the kernel's `3*x^2*cos(x^3)^(-2)` is correct, but
  `Lovelace.Real` division returns 1 or 0 instead of ≈1.0014 when the divisor is just below 1.
- **Conclusion**: The oracle earned its keep on its first run: it found a core numeric defect that
  every unit suite had missed, and it is pre-existing rather than a Cycle-3 regression.
- **Related**: EVD-161, EVD-163, EVD-164, EVD-165, OBS-005

### OBS-005: Two pre-existing defects in `Lovelace.Real`, plus one performance bound

- **Source**: EVD-164 (division), EVD-166 (`ToString`), EVD-168 (`cos` timing), EVD-167 (surface
  reachability).
- **Fact**: (1) **N21** — `Real` division returns 1 for `1/0.9`, `1/0.99`,
  `1/0.9993142073433579945` and 0 for `1/0.99862888499828389309965043538706203025`, while the same
  value computed as `10/9` is correct. Identical at `35e2609`. Its reach: the kernel's numeric
  evaluation (`Evaluation.EvaluateToNum`), because language *literals* stay exact rationals and are
  unaffected. (2) **N22** — `Real.ToString()` throws `ArgumentOutOfRangeException` at
  `Real.cs:1764` for a 2-digit repeating expansion, and it is user-visible: the runner returns an
  error envelope for `1 / 0.99`. (3) **N23** — `cos(1/2)` costs 44 s at the default 1000-place
  precision (1 ms at 20 places), which is what made two earlier probes look like hangs.
- **Implications**: N21 is a wrong answer in the numeric core and the sole cause of the oracle
  disagreement; N22 is a wrong answer *shape* on the published surface; N23 bounds interactive use of
  the Studio at its default precision. None is a Cycle-3 regression.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-164, EVD-165, EVD-166, EVD-167, EVD-168, RISK-002

### DEC-005: Fix N21 and N22 in one Implementer round, with the located cause in the prompt

- **Decision**: Dispatch one Implementer (not two) against `Lovelace.Real` for both defects, with the
  minimal pairs, the SymPy ground truth, the file:line of the `ToString` throw, and the exact
  acceptance command in the prompt. N23 is recorded as a bound, not fixed.
- **Rationale**: VAL-002 located both causes to one file, which is the condition under which Cycle 3
  saw five of five first-dispatch fixes succeed. One agent, not two, because both edits are in
  `Lovelace.Real/Real.cs` and parallel editors would collide. N23 is a performance characteristic
  rather than a wrong answer, and fixing high-precision `cos` is a different, larger piece of work
  than this round's objective.
- **Alternatives considered**: (a) fixing it myself — rejected: the orchestrator's probes established
  the cause, and an independent implementer is the right separation for the fix; (b) two agents, one
  per defect — rejected for the file collision; (c) also folding in N23 — rejected as scope creep
  inside one round.
- **Related**: OBS-005, VAL-002, TODO-005

### TODO-005: N21 (Real division) and N22 (Real.ToString) are fixed and regression-tested

- **Task**: Implementer round; tests written first and observed failing; fix must be principled digit
  arithmetic, not a special case for divisors near 1; acceptance = the SymPy oracle reports
  `Failed 0 / Passed 6 / Skipped 0` and `Lovelace.Real.Tests` (non-Heavy) stays green.
- **Priority**: P0
- **Depends on**: nothing
- **Status**: In progress
- **Related**: DEC-005, OBS-005

### RISK-002: The core numeric layer has defects no existing suite detects

- **Risk**: N21 and N22 are pre-existing and were invisible to 272 `Lovelace.Real` tests, the
  falsification sweeps and every other project's suite. There may be more of the same shape —
  wrong results at boundaries the suites do not sample.
- **Likelihood**: Medium
- **Impact**: High
- **Evidence**: EVD-164, EVD-165, EVD-166; 2439 tests were green while `1/0.9` was wrong in the
  numeric core.
- **Mitigation**: Fix N21/N22 with tests written against SymPy-derived ground truth (so the guard is
  independent of the implementation), keep the oracle running as a gate, and record the residual
  bound honestly in the final report rather than claiming numeric-core coverage.
- **Gate**: —

### VAL-003: RISK-001 materialized — the wire commit does not build on its own (Falsified)

- **Target**: RISK-001 ("committing in five stages can strand a non-building intermediate tree") and
  the series committed as `b1fb67f…3992d80`.
- **Method**: For each of stages 2, 3 and 4, `git worktree add` at that commit, build the solution
  there, then run that stage's suites — a clean checkout, so later stages cannot contaminate it.
- **Evidence examined**: `docs/goal-cycle-4/round-01/stage-verification.txt`; EVD-172 (stage 2 green),
  EVD-173 (stage 3 `CS0117`), EVD-174 (stage 4 green).
- **Result**: Falsified for the series as committed. Stage 2 builds and passes its suites; **stage 3
  does not compile** because `SymbolicsPlugin.cs:359` calls
  `RationalFunctions.CancelWithConditions`, and `RationalFunctions.cs` was committed one stage later;
  stage 4 builds at 0 warnings and passes.
- **Conclusion**: RISK-001's mitigation was correct and its realisation was caught by the check that
  exists for it, before any of this reached a reviewer. One file moves and two commits are rewritten
  (DEC-006). The verification is cheap: three clean builds.
- **Related**: EVD-172, EVD-173, EVD-174, RISK-001, DEC-006, TODO-006

### DEC-006: Move `RationalFunctions.cs` into the wire commit, then rewrite stages 3–5

- **Decision**: `Lovelace.Symbolics/Algebra/RationalFunctions.cs` moves from stage 4 (remaining items)
  into stage 3 (the P0 wire revisions), and commits `160ba4e`, `3367447`, `3992d80` are rebuilt with
  the corrected path lists. Stage 1 (`b1fb67f`) and stage 2 (`84a6a54`) are untouched.
- **Rationale**: VAL-003/EVD-173. `cancel_full`'s kernel half is `RationalFunctions.CancelWithConditions`
  and its facade half is in `SymbolicsPlugin.cs`, which the wire commit already carries; the same
  earliest-stage rule that put `SymbolicsPlugin.cs` in stage 3 also requires its dependency there.
- **Alternatives considered**: (a) leave the red commit and document it — rejected: a reviewer who
  checks out that commit gets a broken tree, and the whole point of the split is reviewability;
  (b) squash stages 3 and 4 — rejected: it destroys the wire-revisions-alone property that was the
  maintainer's actual OQ-007 requirement; (c) add a forward "fix the build" commit — rejected: it
  leaves the broken commit in history rather than removing it.
- **Sequencing constraint**: the rewrite must wait until every in-flight implementer round has
  reported and been verified. Re-staging a path while an agent is editing it would capture that
  agent's in-flight work into a historical commit (RISK-003).
- **Related**: EVD-173, VAL-003, RISK-003, TODO-006

### RISK-003: Rewriting history while implementer rounds are editing the same paths

- **Risk**: `git add <path>` during the DEC-006 rewrite captures an agent's *in-flight* edits into a
  commit that is supposed to represent the Cycle-3 state, corrupting both the history and the
  evidence chain.
- **Likelihood**: High if attempted now (three agents are editing `Lovelace.Real/Real.cs`,
  `Lovelace.Symbolics.Tests/**` and `Lovelace.Symbolics/**`).
- **Impact**: High — it would silently mix Cycle-4 work into Cycle-3 commits.
- **Evidence**: EVD-173; the three in-flight rounds (TODO-005, plus the §4.1/§4.3 and §4.2 rounds).
- **Mitigation**: hard barrier — no history rewrite until all three rounds have reported and their
  deliveries have been verified; the `cycle-3-safety-net` tag keeps the whole series redoable if the
  rewrite goes wrong.
- **Gate**: —

### TODO-006: Rewrite stages 3–5 with `RationalFunctions.cs` moved, then re-verify

- **Task**: After the in-flight rounds settle, `git reset --mixed 84a6a54`, re-stage the corrected
  path lists for stages 3, 4 and 5, re-commit, then re-run the worktree verification for stages 3, 4
  and 5 and record the result.
- **Priority**: P0
- **Depends on**: TODO-005 and both §4 rounds reporting
- **Status**: Open — blocked on RISK-003's barrier by design
- **Related**: DEC-006, RISK-003, VAL-003

### DEC-007: §4.2 is implemented as the mathematically sound maximum, not the literal reading

- **Decision**: The maintainer asked to "force complex probes onto all nine rewrite rules". Implement
  it as: the 5 rules whose assumptions admit complex arguments stay complex-sampled, and the other 4
  each gain a **negative control that proves the exclusion is necessary** — at a complex point that
  satisfies every non-order assumption, the rule's identity is shown to FAIL, so the real-only guard
  is demonstrated load-bearing rather than merely asserted. All 9 rules therefore receive a complex
  treatment; the guard in `AtomHolds` is NOT relaxed.
- **Rationale**: The literal reading is unsound. `FalsificationGateTests.ComparePoint` returns early
  unless every assumption atom holds, and `AtomHolds` (`:486-489`) rejects a `NumComplex` for any
  order predicate. A complex number is not positive, so making a rule like `pow.sqrt-square` "compare"
  at a complex point would either manufacture a counterexample against a rule that is correct on its
  own domain, or force the gate to accept a claim the rule does not make. Cycle 3 documented exactly
  this boundary at `docs/goal-cycle-3/round-27/implementation.md:143-147` ("correctly, since
  |z|^2 != z^2 over C"). Manufacturing a false counterexample is the one failure mode a falsification
  gate must never have.
- **Alternatives considered**: (a) literal implementation — rejected as unsound, above; (b) leave the
  5-of-9 prose as-is — rejected, because the maintainer asked for more than prose and the negative
  controls make the bound a checked property rather than a claim.
- **Disclosure**: recorded here and in `state.md` so the maintainer can redirect if a different
  reading was intended. The literal option remains available and costs one more round.
- **Related**: todo for §4.2 (round-03 falsification round), OBS-005

### DEC-008: §4.3's reopened scope is bounded to exactly-representable complex values

- **Decision**: Authorise minimal `Lovelace.Suite` edits (the `sqrt` builtin at
  `Interpreter.cs:1451-1459`, the power path in `NumericOps.cs`/`ValueField.cs`) as a **fallback only**:
  when the Real-domain operation rejects the input, route to the Symbolics complex path. In scope:
  `sqrt(-a)` and `(-a)^(1/2)` for exact `a > 0`, returning exact `i*sqrt(a)` and exact complex
  constants where `sqrt(a)` is rational (`sqrt(-1)`=i, `sqrt(-4)`=2i), plus exact quadratic complex
  roots. Out of scope and still typed errors: general rational exponents of positive bases
  (`2^(1/2)`), denominator ≥ 3 over negative bases (`(-8)^(1/3)`, whose principal value is not exactly
  representable), and degree ≥ 4 complex algebraic roots.
- **Rationale**: The implementer established by direct probe that `2^(1/2)` fails for the SAME reason
  as `(-1)^(1/2)` ("Non-integer exponents are not yet supported"), so the reopened item is really
  "general rational exponents" — a much larger feature than the brief's §4.3 wording. Bounding to the
  exactly-representable subset delivers `sqrt(-1)`/`(-1)^(1/2)` end to end without inventing decimal
  approximations for values advertised as exact. `Lovelace.Real` is not touched: `Real.Sqrt(-1)`
  throwing remains correct for a real-domain type.
- **Alternatives considered**: (a) keep Suite frozen and land the kernel half only — rejected: it
  would leave `sqrt(-1)` failing at the published surface, which is the actual §4.3 complaint;
  (b) implement general rational exponents — rejected as a much larger, higher-risk change to a
  converged kernel, outside a single round's scope.
- **Consequence for §4.1**: the capability list must describe the resulting boundary accurately —
  `complex.sqrt-negative` leaves the list only if the live envelope now answers, and the residual
  rational-exponent class stays listed.
- **Related**: §4.3, §4.1, `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:202-208` (which asserts
  the old behaviour and is replaced, not deleted, under this decision)

### OBS-006: The differential oracle passes end to end, and it has already paid for itself

- **Source**: `docs/goal-cycle-4/round-02/orchestrator-verification.log`; EVD-175…EVD-179.
- **Fact**: With the N20 prelude fix and the N21/N22 fix, the oracle reports
  **`Failed 0 / Passed 6 / Skipped 0`** — six corpora, 57 cases, each sampled at multiple rational
  points, all agreeing with sympy 1.14.0. `Lovelace.Real.Tests` (non-Heavy) is 292/0 with the two test
  files at +136/−0. Through the published runner, `1 / 0.99` now returns `1.(01)` instead of an error
  envelope.
- **Implications**: D2 is met. The project's most-exposed unverified surface is now verified, and the
  cost/benefit is settled empirically: the oracle's first two real comparisons produced one defect in
  itself (N20) and one wrong answer in the numeric core (N21) that 2439 green tests had missed. The
  agreement is over the sampled corpus, not a proof over the domain — that bound stands.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-175, EVD-176, EVD-177, EVD-179, HYP-001, RISK-002

### VAL-004: HYP-001 re-tested after the N21 fix — Supported

- **Target**: HYP-001 ("with the prelude fixed, all six corpora compare against SymPy and agree"),
  recorded as Falsified at VAL-002.
- **Method**: Re-ran the oracle and the new regression guards in an isolated worktree at HEAD
  carrying only the two fixes; `LOVELACE_REQUIRE_SYMPY=1`, python3 3.12.14 + sympy 1.14.0.
- **Evidence examined**: `docs/goal-cycle-4/round-02/orchestrator-verification.log` —
  `Passed! Failed: 0, Passed: 13, Skipped: 0, Total: 13`.
- **Result**: Supported. The falsification at VAL-002 had a single cause (N21), and removing that cause
  removes the disagreement.
- **Conclusion**: The kernel's calculus agrees with SymPy across all six corpora at every sampled
  point. This is evidence over 57 sampled cases, not a proof; the sampled-point bound is stated in the
  report rather than implied away.
- **Related**: HYP-001, VAL-002, EVD-176

### DEC-009: A wrong expectation of mine was corrected against SymPy, not against the kernel

- **Decision**: On finding that `NegativePowerEvaluationTests` failed against a *correct* kernel value,
  I re-derived the expectation from SymPy 1.14.0 and edited only the expectation strings. The kernel
  was not touched, and no tolerance was widened.
- **Rationale**: EVD-178. The original expectation was doubly wrong: it used `1/cos` where the
  expression is `1/cos²`, and its `cos²` companion took the kernel's own 20-digit output as if it were
  independent ground truth — the precise failure mode this harness exists to prevent. Two independent
  facts caught it: the failure appeared in *my* file while the oracle's own six tests passed, and the
  kernel's value agreed with SymPy to 40 digits.
- **Alternatives considered**: relaxing the tolerance — rejected outright; changing the kernel to match
  the wrong expectation — rejected, and explicitly countermanded to the implementer before it could.
- **Related**: EVD-178, EVD-176, HYP-001

### VAL-005: The §4.2 implementer's Magnitude regression claim — Falsified

- **Target**: "the uncommitted `Lovelace.Real/Real.cs` edit makes `Complex.Magnitude` throw
  `DivideByZeroException` for tiny magnitudes (repro M = 2.26e-81); HEAD passes."
- **Method**: Wrote a probe that references only `Lovelace.Complex` (so it builds independently of the
  Symbolics projects other agents were editing), and ran it in two clean worktrees — one at `fca8277`
  without the fix, one with it.
- **Evidence examined**: EVD-183. At every scale from 1e−77 to 1e−89, `Magnitude` is correct in BOTH
  trees. The real difference is the reciprocal: `1/tiny` throws `ArgumentOutOfRangeException` at HEAD
  and returns the correct value with the fix.
- **Result**: Falsified. The claimed failure does not reproduce, and the direction of the effect is the
  opposite of the claim: the fix repairs that scale range rather than breaking it.
- **Conclusion**: No workaround is warranted, and the implementer was told not to chase it. The claim
  is kept in the journal rather than deleted, because a falsified finding is a result.
- **Related**: EVD-183, EVD-182, DEC-007

### DEC-010: The Ln ruling is flipped — the complex principal branch stays

- **Decision**: Keep the principal-branch complex `Ln` in `Evaluation.cs` and keep the `x = -1/2` point
  live in the `x*log(x)` oracle case. The earlier instruction to revert it and restrict the corpus to
  positive points is withdrawn.
- **Rationale**: That instruction rested on a **stale observation** — a failure reported *before* the
  implementer's Ln change landed, which I passed on as if it were current. The implementer's evidence is
  strictly better: it ran the case live and the kernel's `log(1/2) + i*pi` agrees with SymPy's
  `0.306852819440054690582767878542 + 3.14159265358979323846264338328*I`. This is my error, recorded
  as an error, and it is why the harness insists a claim be re-measured by the orchestrator before it
  becomes a ruling.
- **Alternatives considered**: holding the earlier line for consistency — rejected: consistency with a
  stale measurement is not a virtue, and the flipped ruling delivers more of what the original brief
  asked for (a live comparison under matched domains) without weakening any guard.
- **Consequence**: `capabilities()` must not advertise log-of-a-negative-real as unsupported. The
  `capabilities.json` golden must be regenerated from the new binary with its diff recorded.
- **Related**: VAL-005, §4.3, DEC-008

### RISK-004: A numeric-core fix perturbs neighbouring exact paths

- **Risk**: Fixing division changed the negative-argument path of `Real.Sin`, turning an exact `-0.5`
  into `-0.4999…4448…` and failing an untouched test. Numeric-core fixes move results everywhere that
  arithmetic feeds, so a fix verified only against its own symptom can regress an unrelated identity.
- **Likelihood**: Medium (it has already happened once, EVD-182)
- **Impact**: High — the affected identities (odd/even symmetry, exact special angles) are exactly what
  consumers depend on.
- **Evidence**: EVD-182; the pre-fix tree was 61/0 and the post-fix tree 60/1.
- **Mitigation**: (a) every numeric-core round must run at least one suite OUTSIDE the fixed project —
  this regression was invisible to `Lovelace.Real.Tests`'s own 292 tests; (b) the fix's owner is
  repairing the invariant and adding an odd-symmetry guard test; (c) the final full 15-project sweep is
  the gate that catches the next one.
- **Gate**: —

### OBS-007: The adversarial audit worked — and it falsified a claim Cycle 4 had just made

- **Source**: `docs/goal-cycle-4/round-09/audit-P1-numeric.md` (222 probes: 156 HELD / 62 FINDING /
  4 INCONCLUSIVE), `audit-P2P6-wire.md` (51 probes: 27 HELD / 20 FINDING / 4 INCONCLUSIVE), plus my own
  re-runs in EVD-193…EVD-196.
- **Fact**: Three independent falsifiers attacked the re-published binary. The numeric attacker found
  that `(a/b)*b` does not return `a` — `(1/17)*17` = `0.999…`, `x == 1` is `false` — and that
  `2^-100000` and `0^(-1.0)` return 0 **marked `exact:true`**. I verified all three against the
  pre-Cycle-4 tree and they are **pre-existing**, not regressions. The wire/capability attacker tripped
  every advertised unsupported-operation entry and found them byte-accurate, but found **eight**
  refused operations the statement omits — falsifying the exhaustiveness that §4.1 was closed on — plus
  an `InternalInvariantFailure` on `solve_system`'s documented call form.
- **Implications**: Cycle 4's own deliverables hold (blockers closed, §4.2–§4.4 and N23 delivered,
  final tree green and re-published), but the cycle **cannot claim A+**: the audit produced 20+ FINDING
  rows, including P0-class wrong values that are marked exact, and the §4.1 claim it was written to
  close is falsified. Saying so is the point of running the audit before the report rather than after.
- **Confidence**: High
- **Agent**: orchestrator (with three falsifier agents)
- **Related**: EVD-193, EVD-194, EVD-195, EVD-196, VAL-006

### VAL-006: §4.1's "exhaustive capability list" claim — Falsified

- **Target**: the §4.1 closure claim, and the maintainer's instruction to require an exhaustive
  operation list.
- **Method**: An independent auditor tripped all four advertised entries live and compared code,
  category and message; then attacked the other direction — searching for operations the kernel
  refuses that the statement does not list.
- **Evidence examined**: EVD-196. The honesty half **holds** (all four entries byte-accurate). The
  exhaustiveness half **fails**, eight ways, with reproductions.
- **Result**: Falsified. `capabilities()` advertises four unsupported classes and the kernel has at
  least twelve.
- **Conclusion**: §4.1 is *not* closed. The next round has the exact list to add; each addition needs
  the same live-envelope transcription and assertion the existing four already carry. Recorded as an
  open P0 question rather than quietly softened, and the report will not claim otherwise.
- **Related**: EVD-196, OBS-007

### RISK-005: Wrong values that are labelled exact

- **Risk**: `2^-100000` returns `0` with `"exact":true`. A consumer that trusts the exactness flag —
  which is precisely what the flag is for — will compute with a wrong value and never see an error.
- **Likelihood**: High (it reproduces on demand, on both the current and the pre-Cycle-4 tree)
- **Impact**: High — the flag is a machine-readable promise, and it is false.
- **Evidence**: EVD-194
- **Mitigation**: Recorded as an open defect with a reproduction. Not fixed in Cycle 4: it is
  pre-existing, it is outside the five §4 items the maintainer approved, and a numeric-core change
  made late in a cycle has already produced one regression in this cycle. It is named in the report's
  below-A+ list.
- **Gate**: —

### VAL-007: §4.1's exhaustiveness claim, re-tested after the fix — Supported for every class exhibited

- **Target**: the falsified §4.1 claim (VAL-006), after the round-10 repairs.
- **Method**: I wrote and ran my own falsification test rather than trusting the implementer's suite.
  It decodes the advertised statement from the binary, executes **each entry's own `trigger` script**,
  and matches the advertised `code` and `category` against the live output — handling both surfaces,
  because three refusals ride inside a result record's `diagnostics` rather than an error envelope.
- **Evidence examined**: EVD-198 — `advertised entries: 16`, `MATCH=16  MISMATCH=0`; and all eight
  audit classes are present, including the two that needed kernel repairs.
- **Result**: Supported. Every advertised class is honest, and every class the audit exhibited is now
  advertised.
- **Conclusion**: §4.1 is closed as far as the claim can be tested: **by falsification over the classes
  an independent adversary found**, not by assertion. The implementer correctly left `exactness` at
  `BestEffort` rather than upgrading it, and added a test that drives live unlisted refusals so the
  verdict cannot drift into an overclaim. The honest statement of the boundary is therefore:
  exhaustive over the exhibited classes, with the remainder itemised.
- **Related**: VAL-006, EVD-196, EVD-197, EVD-198

### OBS-008: The round-10 repairs fixed a real bug, not just a documentation gap

- **Source**: EVD-198, EVD-199; `docs/goal-cycle-4/round-10/implementation.md`.
- **Fact**: Two of the eight classes could not honestly be advertised as they stood, because the
  refusals themselves were malformed: a refused integration returned `Unevaluated` with **empty**
  diagnostics (nothing for an agent to branch on, and invisible to the documented "did anything go
  wrong" check), and `plot(sin(x))` died as an `InternalInvariantFailure` — a bug wearing a refusal's
  clothes. Both were repaired at the source: integration now emits a structured
  `integration.unevaluated` diagnostic with the input-specific reason in `details`, and `plot`'s
  one-argument path checks its argument's kind before casting, yielding a typed
  `InvalidOperation`/`DomainError`. Advertising them without those repairs would have been the
  dishonest option, and the audit's probe is what made the difference visible.
- **Implications**: §4.1's closure produced a real defect fix in the integration path and a real cast
  bug fix in the plot path — the opposite of the usual "documentation-only" outcome of a capability
  exercise.
- **Confidence**: High
- **Agent**: orchestrator (verified), implementer (repaired)
- **Related**: EVD-197, EVD-198, VAL-007
