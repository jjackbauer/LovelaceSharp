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
