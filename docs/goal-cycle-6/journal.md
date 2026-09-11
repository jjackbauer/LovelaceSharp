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

