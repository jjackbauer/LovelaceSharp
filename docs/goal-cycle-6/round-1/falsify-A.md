# Goal cycle 6 — round 1, falsification A: are the two stale `solve` help pins the whole CI failure?

Role: **Falsifier**. This file is the only file written. No product, test, workflow or fixture file was
edited. Every experiment ran in throwaway copies of the repository under WSL Ubuntu
(`~/ls-A6` = clean `git checkout -f 7f69f54`; `~/ls-A8` = same, then `git clean -xdf` for a pristine
checkout) or in a throwaway probe project at `/tmp/helpprobe` (outside the repository) that references
the built `Lovelace.Console` and drives the product's public `ReplSession` API directly.

Environment: `wsl.exe -d Ubuntu`, .NET SDK 10.0.401, `export PATH="$HOME/.dotnet:$PATH"`,
`export DOTNET_ROOT="$HOME/.dotnet"`. CI commands were reproduced verbatim from
`.github/workflows/ci.yml` (same project order, same `--configuration Release --nologo`,
same `--collect:"XPlat Code Coverage"`, same two `Lovelace.Real.Tests` filters, same
`set -euo pipefail` semantics in the cold-tree replica).

## Claim table

| # | Claim | Verdict | Evidence (file:line / command) | Counterexample attempted | Reason |
|---|-------|---------|--------------------------------|--------------------------|--------|
| 1 | The only reason the CI job "Fast accuracy test suites" is red at commit 7f69f54 is that `Lovelace.Console.Tests/ReplOutputTests.cs` pins help strings the product no longer emits ("Solves an equation for x." at line 63 and "Returns: Vector \| Text" at line 65); correcting those two pins to the text the product actually declares is sufficient for the whole job — all 14 test projects in `.github/workflows/ci.yml` plus the two `Lovelace.Real.Tests` steps — to pass on GitHub's ubuntu-latest runner with .NET 10. | **Supported** | Pins at the claim commit: `sed -n '57,68p' Lovelace.Console.Tests/ReplOutputTests.cs` in `~/ls-A6` at 7f69f54 → line 63 `Assert.Contains("Solves an equation for x.", t);`, line 65 `Assert.Contains("Returns: Vector \| Text", t);`. Product: `Lovelace.Symbolics/SymbolicsPlugin.cs:453` declares the long summary ("Solves an equation for x and returns the SAME SolveResult record … pass real for real solutions only."), `:454` declares return kind `"SolveResult"`; `Lovelace.Suite/HelpService.cs:167,176` emits the summary and `Returns: {ReturnKind}` with `AppendLine` (no wrapping). GitHub API: run `34655500562` (head 7f69f54) job "Fast accuracy test suites" = `failure` at step 5 "Run fast test suites" (36 s), steps 6–8 `skipped`; the other two jobs `success`. WSL replica of all 16 executed steps at 7f69f54: 15 exit 0, **only** `Lovelace.Console.Tests` exit 1 — `Failed: 1, Passed: 14, Total: 15`, `Failed ReplOutputTests.Help_Solve_ShowsSignatureAndSummary`, `Assert.Contains() Failure: Sub-string not found … Not found: "Solves an equation for x." … ReplOutputTests.cs:line 63`. Product probe (no edit): stale pins `MISS`, corrected pins `HAS`, the other four assertions of that test `HAS`, identical over 2 runs. Cold pristine-tree replica of the literal CI loop (`set -euo pipefail`): first failure = `Lovelace.Console.Tests` at cumulative **+32 s** (GitHub red runs step 5: 33–37 s; last green run `9e761ba` step 5: 256 s). History: `git log 9e761ba..7f69f54 -- Lovelace.Console.Tests/ReplOutputTests.cs` = empty, while the redness starts exactly at `86a5fc8` ("solve returns the record"), the commit that rewrote the `solve` descriptor. | 1. Full job replica **continuing past failures** to expose a hidden second failure → none: 13/14 projects + Real correctness (2475) + Real timing (1) + the `symbench` build all exit 0, twice (warm tree, pristine cold tree). 2. Assertion-by-assertion probe of the failing test behind the line-63 throw → only the two pins named in the claim mismatch. 3. Cold pristine-tree loop under `set -e` → stops at `Lovelace.Console.Tests`, matching the runner's step-5 duration (32 s vs 33–37 s), not at an earlier suite. 4. `Category=Timing` repeated 3× → 0 failures (measured cosine ≈1 s against its 10 s budget). 5. `Symbolics.Tests` with the SymPy oracle made available (sympy 1.14.0 + mpmath 1.4.1 on `PYTHONPATH`; 0 skips / 1031 executed) → 1030 pass; one wall-clock tripwire test failed once at 448.61 ms against its 250 ms ceiling, then passed 8/8 single-test repeats and 3/4 full-project runs; its own passing log measures 11.34 ms and the file is unchanged since the last green run, so the 448 ms reading is CPU starvation in this shared VM, not a defect. 6. Prior cycle-6 artefact claiming `Lovelace.Complex.Tests EXIT=1` → refuted: that log is a Windows "Test host process crashed"; Linux gives 96/96 in warm and cold trees. | Both legs survive. (a) *Only reason*: the sole non-zero step of a full 16-step replica is the Console project, its sole failing test fails on the pinned string the product stopped emitting, and the red streak begins at the very commit that changed that contract while the test file did not change. (b) *Sufficiency*: every step except that one test already exits 0, and the corrected pins ("Solves an equation for x and returns the SAME SolveResult record … only." and "Returns: SolveResult") are literally present in the transcript the product writes, as are the test's other four assertions, so the corrected test passes. The 8 skipped tests are the SymPy oracle; the separate `sympy-oracle` job ran exactly those comparisons green on the runner at this commit. |

## Attacks attempted that did not succeed

1. **Hunt for a second failing project.** Ran the exact CI suite list (14 projects + the 2 Real.Tests
   steps + the `symbench` build) with failure-continue at 7f69f54. Observed: every step exit 0 except
   `Lovelace.Console.Tests` (exit 1, 1 failed / 14 passed).
2. **Hunt for a third stale pin hidden behind the line-63 throw.** Drove the product's own
   `ReplSession` (script `help solve` + newline + `exit` + newline) from an out-of-repo probe and tested
   all six assertions of `Help_Solve_ShowsSignatureAndSummary`. Observed: the signature line, `Examples:`,
   `See also: solve_full, solve_system, linsolve`, `Plugin: Lovelace.Symbolics` are present; only the
   two claimed pins are absent. Repeated twice, identical.
3. **Claim the corrected pin text cannot match (wrapping).** Read `Lovelace.Suite/HelpService.cs:164-181`:
   the summary and `Returns: {ReturnKind}` are written with `AppendLine` on single lines. The probe shows
   both corrected strings present verbatim.
4. **Claim the runner failed in a different (earlier) suite.** Rebuilt the loop on a pristine tree
   (`git clean -xdf`) under `set -euo pipefail`: first failure at `Lovelace.Console.Tests`, +32 s
   cumulative — inside the 33–37 s of every red GitHub run, while the last full green run took 256 s for
   the same step.
5. **Claim an unrelated wall-clock flake reddens the job.** `Category=Timing`
   (`RealTrigFastPathTests.Cos_AtDefaultPrecision_StaysInteractive`, 10 s budget) run 3× → 3× exit 0,
   ~1 s measured. `Lovelace.Console.Tests` run twice → the same single failure both times.
6. **Claim a runner-only SymPy condition breaks the suite.** WSL has `python3` and no `sympy` (the same
   skip path as the runner). Forcing the oracle present still passed 1030/1031, and the one failure was
   the contention artefact reported in the counterexample column.
7. **Claim the Windows-only EXIT=1 recorded for `Lovelace.Complex.Tests` by cycle 6 is real.** Its own
   log says "The active test run was aborted. Reason: Test host process crashed"; Linux runs show 96/96
   twice.
8. **Read the runner's own failing log** to look for a cause other than the pins. GitHub's job-log
   endpoint answers `403 {"message":"Must have admin rights to Repository."}` anonymously, and no token
   or `gh` CLI is available, so the failing suite was identified by replica timing/order instead.
9. **Claim local untracked files change a suite's outcome.** The pristine (`untracked=0`) tree
   reproduces the same results for the 12 re-run steps.

## What I could not check

* The literal corrected test file executed on GitHub's runner: editing any test is forbidden, and no
  push / workflow trigger is available from this session. The sufficiency leg therefore rests on the
  assertion-by-assertion probe against the emitted transcript plus the 15/16 steps that already exit 0.
* The job's last step, "Upload coverage to Codecov" (`use_oidc: true`, `fail_ci_if_error: true`), is not
  reachable from SCOPE (needs a GitHub OIDC token; at 7f69f54 it was `skipped`). The most recent run
  that reached it, `9e761ba`, uploaded successfully in 3 s.
* The GitHub job log itself (403, above), so the identity of the suite inside step 5 is inferred from the
  duration match and the replica, not read from the runner's output.
