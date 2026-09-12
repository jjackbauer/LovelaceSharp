# Goal cycle 6 — round 1, falsification A: are the two stale `solve` help pins the whole CI failure?

Role: **Falsifier**. This file is the only file written. No product, test, workflow or fixture file was
edited. Every experiment ran in throwaway copies of the repository under WSL Ubuntu:
`~/ls-A6` (clean `git checkout -f 7f69f54`), `~/ls-A8` (same, then `git clean -xdf` for a pristine
checkout) and `~/ls-fixed6` (`git clone --depth 1 file:///mnt/c/Users/ricar/dev/LovelaceSharp`, i.e.
HEAD `70241dc` = `98a9049` "fix(console): re-pin the solve help contract to the text the product
declares" + the cycle-6 docs commit), plus a throwaway probe project at `/tmp/helpprobe` (outside the
repository) that references the built `Lovelace.Console` and drives the product's public `ReplSession`.

Environment: `wsl.exe -d Ubuntu`, .NET SDK 10.0.401, `export PATH="$HOME/.dotnet:$PATH"`,
`export DOTNET_ROOT="$HOME/.dotnet"`. CI commands reproduce `.github/workflows/ci.yml` verbatim
(same project order, `--configuration Release --nologo`, `--collect:"XPlat Code Coverage"`, the same
two `Lovelace.Real.Tests` filters, and the `set -euo pipefail` fail-fast behaviour).

## Claim table

| # | Claim | Verdict | Evidence (file:line / command) | Counterexample attempted | Reason |
|---|-------|---------|--------------------------------|--------------------------|--------|
| 1 | The only reason the CI job "Fast accuracy test suites" is red at commit 7f69f54 is that `Lovelace.Console.Tests/ReplOutputTests.cs` pins help strings the product no longer emits ("Solves an equation for x." at line 63 and "Returns: Vector \| Text" at line 65); correcting those two pins to the text the product actually declares is sufficient for the whole job — all 14 test projects in `.github/workflows/ci.yml` plus the two `Lovelace.Real.Tests` steps — to pass on GitHub's ubuntu-latest runner with .NET 10. | **Supported** | Pins at the claim commit (`sed -n '57,68p'` in `~/ls-A6`): line 63 `Assert.Contains("Solves an equation for x.", t);`, line 65 `Assert.Contains("Returns: Vector \| Text", t);`. Product: `Lovelace.Symbolics/SymbolicsPlugin.cs:453` declares the long summary ("Solves an equation for x and returns the SAME SolveResult record … pass real for real solutions only."), `:454` the return kind `"SolveResult"`; `Lovelace.Suite/HelpService.cs:167,176` emit both with `AppendLine`, unwrapped. GitHub API: run `34655500562` (head 7f69f54) job "Fast accuracy test suites" = `failure` at step 5 "Run fast test suites" (36 s), steps 6–8 `skipped`, the other two jobs `success`. Replica of all 16 executed steps at 7f69f54 (WSL): 15 exit 0, **only** `Lovelace.Console.Tests` exit 1 — `Failed: 1, Passed: 14, Total: 15`, `Failed ReplOutputTests.Help_Solve_ShowsSignatureAndSummary`, `Not found: "Solves an equation for x." … ReplOutputTests.cs:line 63`. Product probe: stale pins `MISS`, corrected pins `HAS`, the other four assertions of that test `HAS` (2 runs, identical). Cold pristine-tree replica of the literal loop: first failure = `Lovelace.Console.Tests` at +32 s cumulative (GitHub: 33–37 s; last green run `9e761ba`: 256 s). History: `git log 9e761ba..7f69f54 -- Lovelace.Console.Tests/ReplOutputTests.cs` is empty while the red streak starts exactly at `86a5fc8`, the commit that rewrote the `solve` descriptor. **Landed-fix run**: the same 16-step loop on `~/ls-fixed6` (HEAD 70241dc) passed the `symbench` build and 11 suites — including `Lovelace.Console.Tests`, the project the claim blames — and died only at `Lovelace.Suite.Tests` (project 12) on a load-sensitive cancellation test (column 5). | 1. Full job replica **continuing past failures** at 7f69f54 → no hidden second failure (13/14 projects + Real correctness 2475 + Real timing all exit 0, twice: warm tree and pristine cold tree). 2. Assertion-by-assertion probe of the failing test behind the line-63 throw → only the two claimed pins mismatch. 3. **Ran the literally corrected tree** (HEAD 70241dc) through the whole job → the job still went red, but at `Lovelace.Suite.Tests`, not at Console: `CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly`, "cancellation was not observed … still running after 30.0 s (test fence 30 s)", 6/6 failures while host loadavg was 15–20. The control kills it as a counterexample: the identical failure reproduces in the **unpatched** 7f69f54 tree (3/3) with identical md5s for `Interpreter.cs`, `SuiteEngine.cs` and the test file, while the same project passed 813/813 twice earlier at load 2–5. 4. Cold pristine-tree loop under `set -e` → stops at Console.Tests (32 s vs the runner's 33–37 s), not at an earlier suite. 5. `Category=Timing` repeated 3× → 0 failures (cosine ≈1 s against its 10 s budget). 6. SymPy oracle forced present (sympy 1.14.0 + mpmath 1.4.1 on `PYTHONPATH`, 0 skips) → 1030/1031, the one failure being the same class of load artefact (448.61 ms against a 250 ms ceiling whose passing measurement is 11.34 ms). 7. Cycle-6's recorded `Lovelace.Complex.Tests EXIT=1` → its own log is a Windows "Test host process crashed"; Linux gives 96/96 warm and cold. | Both legs survive on the evidence available. (a) *Only reason*: one non-zero step in a full 16-step replica, one failing test in it, and the failing assertion is the pinned string the product stopped emitting; the red streak starts at the exact commit that changed that contract while the test file did not move. (b) *Sufficiency*: the corrected tree runs the CI loop past Console.Tests and every suite up to project 12; the two corrected strings are literally present in the product transcript as are the test's other four assertions, and every remaining step was verified green at 7f69f54, whose product code is byte-identical to the corrected tree (only `ReplOutputTests.cs` differs). The one red step observed on the corrected tree is reproduced identically by the unpatched control and disappears when the host is idle, so it is host contention, not the claim's subject; it is nevertheless the residual flake risk named below. |

## Attacks attempted that did not succeed

1. **Hunt for a second failing project at 7f69f54.** Ran the exact CI suite list (`symbench` build +
   14 projects + the 2 Real.Tests steps) with failure-continue. Observed: every step exit 0 except
   `Lovelace.Console.Tests` (exit 1, 1 failed / 14 passed, only `Help_Solve_ShowsSignatureAndSummary`).
2. **Hunt for a third stale pin hidden behind the line-63 throw.** Drove the product's own
   `ReplSession` from an out-of-repo probe and tested all six assertions of the failing test. Observed:
   signature line, `Examples:`, `See also: solve_full, solve_system, linsolve` and
   `Plugin: Lovelace.Symbolics` are present; only the two claimed pins are absent.
3. **Claim the corrected pin text cannot match (wrapping).** `HelpService.cs:164-181` writes
   `descriptor.Summary` and `Returns: {ReturnKind}` with `AppendLine` on single lines; the probe shows
   both corrected strings verbatim.
4. **Run the corrected tree end to end and catch it red.** Did exactly that (HEAD `70241dc`): the job
   got past Console.Tests and 11 suites, then failed at `Lovelace.Suite.Tests` on the load-sensitive
   cancellation test. The unpatched-tree control (same machine, same minute) fails identically, so the
   failure is not attributable to the correction; it is recorded as the residual risk instead.
5. **Claim the runner failed in an earlier suite.** Cold pristine-tree loop under `set -e`: first
   failure at Console.Tests, +32 s cumulative, inside the 33–37 s of every red GitHub run.
6. **Claim an unrelated wall-clock flake reddens the job.** `Category=Timing` 3× → 3× exit 0 (~1 s
   against a 10 s budget); `Lovelace.Console.Tests` run twice → the same single failure both times.
7. **Claim a runner-only SymPy condition breaks the suite.** WSL matches the runner (`python3`, no
   `sympy`). Forcing the oracle present still passed 1030/1031; the one failure was the load artefact.
8. **Claim cycle 6's recorded Complex EXIT=1 is real.** Its own log is a Windows test-host crash; Linux
   gives 96/96 in warm and cold trees.
9. **Read the runner's own failing log.** GitHub's job-log endpoint answers
   `403 {"message":"Must have admin rights to Repository."}` anonymously and no token or `gh` CLI is
   available, so the failing suite was identified from the replica's timing/order instead.
10. **Claim local untracked files change a suite's outcome.** The pristine (`untracked=0`) tree
    reproduces the same results for the 12 re-run steps.

## What I could not check, and the residual risk

* The claim names GitHub's ubuntu-latest runner; I can only exercise the Linux replica WSL provides.
  No push, workflow trigger or OIDC token is available from this session.
* The job's last step, "Upload coverage to Codecov" (`use_oidc: true`, `fail_ci_if_error: true`), is
  unreachable here; at 7f69f54 it was `skipped`. The most recent run that reached it, `9e761ba`,
  uploaded successfully in 3 s.
* The GitHub job log itself (403, above), so the identity of the suite inside step 5 is inferred from the
  duration match and the replica rather than read from the runner's output.
* **Residual risk, not a counterexample:** on the corrected tree the CI step does go red at
  `Lovelace.Suite.Tests` while this host is loaded (loadavg 15–20): `CancellationObservationTests`'s
  30 s fence trips for `2000000!`. It reproduces identically without the fix and the suite passed at low
  load, so it is attributed to contention — but it is a wall-clock tripwire inside the very step the claim
  says will pass, and I could not observe a fully green run of the corrected tree on this machine.
