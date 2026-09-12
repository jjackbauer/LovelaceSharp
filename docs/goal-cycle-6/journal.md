# Goal Journal — cycle-6

> Append-only. See the stretch-goal-harness evidence protocol for entry format.
> IDs `OBS/HYP/VAL/DEC/TODO/RISK/OQ` continue from cycle 5's maximum in the same type where a cycle-5
> entry is being superseded; otherwise they start at 001 for this run.

---

<!-- New entries go below this line -->

### OBS-001: CI is red, and only one of the three jobs is red

- **Source**: `https://api.github.com/repos/jjackbauer/LovelaceSharp/actions/runs/34655500562/jobs`
- **Fact**: Run **#26** on `7f69f54` (the pushed HEAD): `Native AOT publish + runner smoke` **success**,
  `Differential oracle (SymPy installed)` **success**, `Fast accuracy test suites` **failure**, the
  failing step being **#5 `Run fast test suites`**; steps 6–8 (Real subset, Timing, Codecov) are
  `skipped`.
- **Implications**: The breakage is inside the 14-project loop, not in packaging, not in the oracle.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: —

### OBS-002: The failure starts at `86a5fc8` and is stable in duration

- **Source**: `actions/runs?per_page=20` + the jobs endpoint for runs 18, 20, 22, 23, 24, 25, 26
- **Fact**: Run **#17** on `9e761ba` is **success** with step 5 taking **256 s**. Every run from **#18**
  (`86a5fc8`) through **#26** (`7f69f54`) fails at step 5 after **32–37 s**.
- **Implications**: The break is deterministic, was introduced between `9e761ba` and `86a5fc8`
  (only `fa282b6` (docs) and `86a5fc8` are in that window), and dies early in the loop.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: OBS-001

### OBS-003: A Linux reproduction environment was built, and it reproduces the CI failure

- **Source**: `docs/goal-cycle-6/wsl-setup.sh`, `docs/goal-cycle-6/linux-ci-loop.sh`, job `pwsh-105`
- **Fact**: WSL2 Ubuntu 24.04 with .NET **10.0.401** run over a copy of HEAD at `~/ls` reproduces the CI
  loop: Abstractions 20/20, Array 19/19, Complex 96/96 pass, then **`Lovelace.Console.Tests` exits 1**
  with `Failed: 1, Passed: 14` after 8+3+10+9 s — the same position in the loop and the same elapsed
  budget as the runner.
- **Implications**: The failure can be diagnosed locally on Linux without spending runner minutes.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: OBS-002

### OBS-004: The failing test and message

- **Source**: `/tmp/ci/Lovelace.Console.Tests.log` (WSL), `docs/goal-cycle-6/ci-repro/*.log` (Windows)
- **Fact**: `Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary` fails with
  `Assert.Contains() Failure: Sub-string not found / Not found: "Solves an equation for x."` at
  `Lovelace.Console.Tests/ReplOutputTests.cs:line 63`. It fails **identically on Windows**
  (`Failed: 1, Passed: 14`), so it is not a Linux artifact.
- **Implications**: This is a stale pinned string, not a platform difference.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: OBS-003

### OBS-005: The product text moved deliberately; the test pin was not updated with it

- **Source**: `Lovelace.Symbolics/SymbolicsPlugin.cs:451-455`, `Lovelace.Suite/HelpService.cs:167,176`,
  `docs/goal-cycle-5/patches/r13-wire3b-symbolics.diff:896-900`
- **Fact**: `86a5fc8` (patch r13) rewrote the `solve` descriptor's summary to "Solves an equation for x
  and returns the SAME SolveResult record solve_full returns: …" and its `ReturnKind` from
  `Vector | Text` to `SolveResult` — the documented point of the round, that `solve` now returns the
  same record `solve_full` does. `HelpService.Function` appends `descriptor.Summary` verbatim
  (line 167) and `Returns: {descriptor.ReturnKind}` (line 176). The Console test still pins the old
  sentence at line 63 and `Returns: Vector | Text` at line 65.
- **Implications**: The fix is to update two stale pins in the test to the product's declared text; the
  product is correct and must not be reverted (that would restore a false return-kind claim).
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: OBS-004

### HYP-001: The whole CI failure is the two stale pins in `ReplOutputTests`

- **Claim**: After updating `ReplOutputTests.cs:63` and `:65` to the product's declared summary and
  return kind, all three CI jobs pass on the pushed HEAD.
- **Supporting OBS**: OBS-002, OBS-004, OBS-005
- **Why it matters**: If false, further CI round-trips are wasted; if true, D0 closes in one round.
- **Falsification strategy**: Run the full 14-project loop plus both Real.Tests steps on Linux (WSL)
  and on Windows after the change; any second failure falsifies the claim.
- **Status**: Under review
- **Confidence**: High

### VAL-001: Round-1 claim batch — the failure is a stale pin, not a product defect

- **Target**: HYP-001
- **Method**: two independent renderings of the same suite (Windows PowerShell 5.1 and WSL Ubuntu
  24.04/.NET 10.0.401); read the product sources the renderer uses; read the r13 patch that moved the
  text.
- **Evidence examined**: `Lovelace.Console.Tests/ReplOutputTests.cs:63,65`;
  `Lovelace.Symbolics/SymbolicsPlugin.cs:451-455` (summary and `SolveResult` return kind);
  `Lovelace.Suite/HelpService.cs:167,176` (verbatim summary / `Returns:` line);
  `git`-read patch `r13-wire3b-symbolics.diff` showing the old string removed.
- **Result**: Supported
- **Conclusion**: The pinned strings are absent from the product because the product's descriptor text
  changed deliberately in `86a5fc8`; the console test was the one place the round missed. No product
  defect is involved.
- **Related**: OBS-004, OBS-005, HYP-001

### DEC-001: Fix the stale pins, do not revert the product text

- **Decision**: Round 1 changes only `Lovelace.Console.Tests/ReplOutputTests.cs` (the two pins), with the
  replacement strings taken from the product's live descriptors — never invented.
- **Rationale**: Reverting the summary would restore the false `Returns: Vector | Text` claim that
  `86a5fc8` corrected (VAL-001); the round's contract change is documented in the source comment at
  `SymbolicsPlugin.cs:445-450`.
- **Alternatives considered**: (a) revert the descriptor text — rejected, it re-introduces a wrong
  return-kind claim the cycle-5 round deliberately fixed; (b) drop the assertions — rejected as
  weakening; (c) tag the test `Category=Timing`/skip — rejected, the failure is not timing-dependent.
- **Related**: VAL-001

### TODO-001: Land the two-pin fix and push

- **Task**: Update the two stale pins, run `Lovelace.Console.Tests` on Windows and Linux, run a
  regression suite outside the changed project, commit, push, poll every CI job to `success`.
- **Priority**: P0
- **Depends on**: —
- **Status**: In progress
- **Related**: DEC-001

### RISK-001: The loop may hide a second failure behind the first

- **Risk**: The CI step aborts at the first failing project, so any failure later in the loop (Run,
  Studio, Suite, Symbolics, precbench, Real) is invisible until the first is fixed — and each blind
  push costs a runner round-trip.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: `.github/workflows/ci.yml:60` (`set -euo pipefail` inside the suite loop)
- **Mitigation**: Run the whole job (all 14 projects + both Real.Tests steps) with per-project
  continuation in WSL before pushing; take the answer from a transcript, not from hope.
- **Gate**: —

### OQ-001: Does anything after `Lovelace.Console.Tests` fail on Linux?

- **Question**: Do the remaining 10 projects plus the two `Lovelace.Real.Tests` steps pass on Linux at
  HEAD?
- **Needed evidence**: the tail of job `pwsh-106` (`docs/goal-cycle-6/linux-full-job.sh`), which runs
  every project with `--collect` and both Real.Tests filters, continuing past failures.
- **Priority**: P0 blocking
- **Raised by**: Round 1, orchestrator
- **Related**: RISK-001

### OBS-006: A second writer committed and pushed to `main` during this session

- **Source**: `git log --oneline`; `git show --stat 31d3e4a`; `git ls-remote origin refs/heads/main`
- **Fact**: At 20:00:26 local a commit `31d3e4a` ("docs(cycle-6): handoff - three rounds stopped and
  preserved unverified…") appeared on `main` and was **pushed** (`origin/main` was `31d3e4a` before my
  push). It adds `docs/goal-cycle-6/HANDOFF-from-cycle-5.md` and three WIP patches
  (`wip-r20-exact2`, `wip-r21-cancel`, `wip-r22-caps`, sizes 35 792 / 59 174 / 170 137 bytes). I did
  not write it; its author is the maintainer's git identity. Its checkable claim — the three patch
  sizes — matches exactly what is on disk.
- **Implications**: (a) the worktrees' content is preserved as patches, so round 2 can work from either;
  (b) two writers share this repository, so every push must re-check `origin/main` first;
  (c) the handoff's *prose* is external input, not evidence: I verified the patches exist, and its claim
  that the Complex.Tests `EXIT=1` was a probe artifact is superseded by my own measurement (EVD-242 —
  a real but flaky test-host crash) while its conclusion ("do not chase it") is correct.
- **Confidence**: High (existence and sizes); the author's identity is reported by git, not verified.
- **Agent**: orchestrator
- **Related**: EVD-242, EVD-243

### VAL-002: Round-1 claim batch — two independent falsifiers, identical prompt, unanimous

- **Target**: HYP-001, VAL-001
- **Method**: two Falsifiers received the identical claim and prompt (neither told the other existed) and
  were free to attack it in their own way; each wrote its own table
  (`docs/goal-cycle-6/round-1/falsify-A.md`, `falsify-B.md`). I read both artifacts in full, checked
  their citations against the sources myself, and then reproduced the two decisive controls on my own
  tree.
- **Evidence examined**: A: full 16-command CI replica on a pristine `7f69f54` tree — 15 steps exit 0,
  only `Lovelace.Console.Tests` red at `ReplOutputTests.cs:63`, first failure at +32 s cumulative
  (CI's red runs: 33–37 s; green run #17: 256 s); it also drove the real `ReplSession` out-of-repo and
  showed no third stale pin. B: the same replica run **twice**, identical results (only Console red; all
  15 other commands exit 0, including `Lovelace.Suite.Tests` 813/0), plus the `symbench` build step
  green, plus a `git diff --name-only 7f69f54 31d3e4a` = four `docs/` files check on tree fidelity.
  My own controls: old test + old product (`9e761ba`'s `SymbolicsPlugin.cs`) → 15/15 pass; new test +
  old product → fails at `ReplOutputTests.cs:67` (EVD-244); CI badge for `ci.yml` on `main` flips to
  **passing** at 20:21:25 after my push (EVD-245).
- **Result**: Supported — unanimous (both Falsifiers `Supported`, zero `Falsified`, zero
  `NotCheckable` on the claim).
- **Conclusion**: The two stale pins were the whole CI failure; the corrected pins are sufficient for
  the job and are strictly stronger than the ones they replace. The one contrary datum I hold (EVD-246,
  a `Lovelace.Suite.Tests` wall-clock failure in a *loaded* WSL run) does not bear on this claim: it was
  measured on a different tree under concurrent load, and both falsifiers' unloaded replicas report
  Suite 813/0. It is recorded as RISK-002 instead of being averaged in.
- **Related**: HYP-001, VAL-001, EVD-244, EVD-245, EVD-246

### DEC-002: Round 1 lands as a test-pin correction; the CI fix is D0-closed

- **Decision**: `98a9049` (the two-pin correction) and `70241dc` (this memory) are pushed as round 1;
  D0 is recorded as met once the per-job conclusions are read from the run API. No product file changes.
- **Rationale**: VAL-002 unanimous; run #28 green (EVD-245); control matrix shows the new assertions
  have teeth (EVD-244).
- **Alternatives considered**: reverting the descriptor text (rejected, VAL-001); weakening the pins
  (rejected); tagging the Console test (rejected — it is not a timing assertion).
- **Related**: VAL-002, DEC-001

### RISK-002: `CancellationObservationTests` is a wall-clock assertion inside the coverage loop

- **Risk**: `Lovelace.Suite.Tests` runs **instrumented** in the CI loop and contains two wall-clock
  assertions (a 20 s "promptness" bound with a 30 s fence) around `2000000!` and `2^1000000000`. On a
  loaded machine it can cross the bound and redden the job for reasons unrelated to any change.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: EVD-246 (my WSL run: "cancellation was observed but only after 20996 ms; the budget was
  1500 ms", 813 tests in 56 s, while three agents were compiling and testing on the same host);
  falsifiers A and B both measured `Suite 813/0` unloaded; cycle-5's EVD-217/EVD-231 recorded the same
  test failing under agent load.
- **Mitigation**: Do not "fix" it by loosening the bound. If a CI run fails in `Lovelace.Suite.Tests`,
  treat it as this risk first, re-run, and only then investigate the kernel's cancellation latency —
  which is row 2 of the open work list.
- **Gate**: —

### OQ-002: Does the factorial kernel observe cancellation *promptly*, or only eventually?

- **Question**: `2000000!` cancelled at 1500 ms finished only after ~21 s under load and ~4 s when
  quiet. Is that latency load, or a coarse polling granularity in the factorial kernel (the same class
  as open row 2, "no `Cancellation` reference in the numeric kernels")?
- **Needed evidence**: a quiet-machine measurement of cancellation latency for `2000000!` and
  `sum(1..10000000)` with a 1 ms budget, before and after the round that closes row 2.
- **Priority**: P1
- **Raised by**: Round 1, orchestrator
- **Related**: RISK-002, EVD-246

### OBS-007: The three interrupted worktrees triaged — two landable, one needs a repair

- **Source**: `docs/goal-cycle-6/round-2/triage-r20.md`, `triage-r21.md`, `triage-r22.md` (three
  independent analysts, one patch each, each with its own scratch tree and pristine control)
- **Fact**: `wip-r20-exact2` (row 1) — applies exit 0, builds 0/0, 13 of 32 new cases fail on a pristine
  control tree, **but one of its own tests fails on the patched tree** (`TruncatingEvaluationStaysInexactTests.cs:360`,
  `Assert.IsType<NumInt>` → actual `NumRat`) and `C5Probe.cs` is an assertion-free 2 m 29 s probe:
  **NEEDS REPAIR**. `wip-r21-cancel` (row 2) — patched `sum`/`matmul`/`prod` under a 100 ms budget stop in
  289/228/338 ms with `exit 1, code=Cancelled, category=BudgetExceeded` and a `cancellation` ledger, control
  takes 4104/4153/27 658 ms with `ok:true` and no ledger; 9 of 11 new tests have teeth; Run 180/0,
  Suite 819/0, Dsp 61/0: **LANDABLE AS-IS**. `wip-r22-caps` (rows 3 and 4) — patched whole solution green
  (5261 passed / 8 skipped), control fails 50 cases, `evalf(sin(1),0)` answers `0.8` at base and is a typed
  `InvalidArgument/TypeMismatch` after: **LANDABLE AS-IS**.
- **Implications**: rows 2, 3 and 4 can be closed from preserved work; row 1 needs one repaired assertion
  and the removal of a probe file before it can land. All three reports name their own "no teeth" cases
  (2 for r21, 10 for r22, 18 for r20), which is what makes them usable.
- **Confidence**: High for the reports' internal consistency and the commands they paste; the suite totals
  are *their* observations and are re-measured by me in the landing rounds.
- **Agent**: three Triage analysts (round 2)
- **Related**: EVD-247, EVD-249, DEC-003

### DEC-003: Land the three patches in the order r20 (repaired), r22, r21 — each verified by me first

- **Decision**: Row 1 lands first (after the :360 repair and without `C5Probe.cs`), then rows 3+4
  (`wip-r22` as-is), then row 2 (`wip-r21` as-is). Each landing round re-runs the affected suites and the
  control tree itself before the commit; no patch lands on a triage verdict alone.
- **Rationale**: the triage reports are agent claims (OBS-007); the harness gate requires my own
  reproduction (G4), and each patch's control run is cheap to repeat because the control trees already
  exist at `.worktrees/c6-r2*-ctl`/`c6-r2*-control`.
- **Alternatives considered**: landing all three at once (rejected: one bounded change per round, and a
  single red suite would be unattributable); discarding the WIP work and re-implementing (rejected: the
  evidence shows the work is sound and the controls are already built).
- **Related**: OBS-007, EVD-249

### OBS-008: A PowerShell pipeline artifact produced a false "does not apply" verdict — mine

- **Source**: my own round-2 note in EVD-247 vs the re-measurement in EVD-249
- **Fact**: `git apply --check … 2>&1 | Select-Object -First 6` followed by reading `$LASTEXITCODE` reported
  exit `-1` for `wip-r21-cancel`; re-running the same check through `cmd /c` with `%errorlevel%` reports
  **exit 0** for all three patches. The short-circuit in the pipeline, not git, produced the -1.
- **Implications**: This is the same class of probe artifact the cycle-5 handoff warned about
  (`HANDOFF-from-cycle-5.md`: "most likely $LASTEXITCODE read after a pipeline"). Exit codes are now read
  through `cmd /c` or from `$LASTEXITCODE` immediately after the command with no pipeline.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-247, EVD-249

### OBS-009: Round 3 — both Falsifiers falsify the row-closure claim, and agree on the cause

- **Source**: `docs/goal-cycle-6/round-3/falsify-A.md`, `falsify-B.md`, `implementation.md`
- **Fact**: On the landed tree **both** Falsifiers returned `Falsified` for claims 1 and 2 and
  `Supported` for claims 3 and 4 (0 `NotCheckable`). Counterexample, found independently by both and
  reproduced by me: `evalf(cos(pi(30)), 40)` crosses the runner's wire as
  `{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}` while the same
  expression is `Symbolic exact:false` and `inspect(cos(pi(30))).exact` is `false`; at 100 digits the
  **value** is also wrong (the wire says `-1`; mpmath at 110 dps says
  −0.99999999999999999999999999999999999999999999999999999999999987355374…, differring at the 61st
  decimal). A's located cause: `Lovelace.Real/Real.cs:1742-1743` returns the special-angle table's value
  (`:1935`, `cos(pi)=negOne`) without propagating the input's inexactness, and its `ReducingPi` twin
  `:1749-1750` does the same.
- **Implications**: Row 1 has **two** routes; the round-3 change closes `FromRealExact`/ToReal but not the
  special-angle shortcut. The claim "no member of that family can still be published exact" is false.
- **Confidence**: High
- **Agent**: Falsifiers A and B; adjudicated by the orchestrator
- **Related**: VAL-003, DEC-004, EVD-251

### VAL-003: Round-3 claim batch — Falsified on two of four claims

- **Target**: HYP-002 ("row 1 is closed")
- **Method**: two Falsifiers with the identical claim list and prompt, independent scratch copies; then I
  re-ran every counterexample myself through the published runner's entry point and against mpmath.
- **Evidence examined**: `falsify-A.md` (own sweeps of 192 + 100 expressions, mechanism at
  `Real.cs:1742-1743`), `falsify-B.md` (same counterexample from a different route; pre-fix comparison);
  my own probes: `evalf(cos(pi(30)), 40)` → `exact:true, numerator -1, denominator 1`;
  `inspect(cos(pi(30))).exact` → `false`; `evalf(cos(pi(30)), 100)` → `-1` vs mpmath's
  −0.999…87355…; the pristine control tree (`.worktrees/c6-r20-ctl`) reproduces the same envelope, so the
  route is pre-existing, not introduced by this round.
- **Result**: **Falsified** (claims 1 and 2), Supported (claims 3 and 4: the 32-case file's 14-fail control,
  no weakening, `C5Probe.cs` absent, solution green).
- **Conclusion**: The round's change is a strict, verified improvement *and* insufficient to close row 1.
  Two sub-parts of the falsification need adjudication rather than acceptance: (a) `inspect(sin(1)).exact
  = true` beside `evalf(sin(1), 40).exact = false` is **not** a defect — the first asks whether the
  *expression* is exact, the second whether the *approximation* is; my claim 2's wording was ill-posed and
  is corrected here. (b) `subs(subs(x+y,x,pi(30)),y,1-pi(30)) = 1` moving from `exact:true` to
  `exact:false` is the **intended** provenance semantics (cycle 5's T0-3: "exactness is provenance"), not
  an over-correction. The `cos(pi(30))` counterexample, by contrast, is a real wrong-exactness **and**
  wrong-value claim, and it is the reason the row stays open.
- **Related**: OBS-009, DEC-004, EVD-251, EVD-252

### DEC-004: Land the first route, keep row 1 open, close the special-angle route next

- **Decision**: commit the round-3 change as `e8638c0` (it is a verified improvement with 13 teeth tests,
  the whole solution green and CI back to green), state in the commit message that row 1 is **not** closed,
  and make the special-angle route the next round's single objective.
- **Rationale**: VAL-003; `Real.cs:1742-1743`/`:1749-1750` is a located root cause with a reproduced,
  quantified counterexample (EVD-251), which is exactly the input a repair round needs.
- **Alternatives considered**: holding the change uncommitted until both routes were closed (rejected: it
  leaves verified work unsaved in a workspace a second writer is also committing to, and it makes the
  eventual diff unattributable); reopening the whole row from scratch (rejected: 13 of the 32 new cases
  already have teeth against the pristine tree).
- **Related**: VAL-003, OBS-009

### RISK-003: A second wall-clock assertion inside the coverage loop

- **Risk**: `Lovelace.Symbolics.Tests.AssumptionAddScalingTests.Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder`
  asserts a 250 ms wall-clock bound and runs **instrumented** in the CI loop; on a loaded machine it can
  redden the job for reasons unrelated to any change.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: EVD-252 — my own `dotnet test LovelaceSharp.slnx` with `LOVELACE_REQUIRE_SYMPY=1`, run
  while the implementer and two falsifiers were building, failed exactly this case ("[2 s]" against a
  250 ms bound); re-run alone it passes 4/4 in 625 ms, three times.
- **Mitigation**: same policy as RISK-002 — never loosen the bound; if CI reddens there, re-run first.
- **Gate**: —

