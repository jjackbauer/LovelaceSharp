# Goal — cycle-6

**Owner**: orchestrator (stretch-goal-harness). **Started**: 2026-09-11. **Round cap**: 40.

## The goal, in one sentence

Make LovelaceSharp's CI green on GitHub's runners again, close the four open Tier-0/Tier-1 rows and
their residual bounds, and claim A+ only if a *fresh* adversarial audit cannot falsify it.

## Definition of done — falsifiable, each with a command and an expected observable

| ID | Dimension | Command | Expected observable |
|---|---|---|---|
| **D0** | CI green on a GitHub runner | push, then read the run | all three jobs `success` on the pushed HEAD |
| **D1** | Zero open P0/P1 from a **fresh** adversarial audit (new agents, new probes) | dispatch 3–4 new personas with new attack strategies against the binary published from the final commit | no finding in the P0/P1 classes |
| **D2** | Every Tier-0 and Tier-1 defect closed by a test asserting the *correct* behaviour against independent ground truth, **that fails on the pre-fix tree** | `dotnet test <project> -c Release`, and the same test in a pristine control worktree | passes on the fixed tree, fails on the control |
| **D3** | Every remaining bound accepted in writing by the maintainer, or closed | amend `docs/symbolics/a-plus-convergence-alignment-plan.md` (cycle 5 added section O; add **P**) | each bound `CLOSED` with evidence or `ACCEPTED` with the maintainer's own words |
| **D4** | The final tree re-measures green | forced `--no-incremental` rebuild; full 15-project sweep with `LOVELACE_REQUIRE_SYMPY=1`; AOT publish + freshness + the five CI smoke scenarios; `verify-capabilities.ps1`; `run-roundtrip2.ps1` | 0 warnings; 0 failed; a binary newer than the newest **code** file; smoke 0 failures; `MATCH=16 MISMATCH=0`; round-trip `ok=30 bad=0` |
| **D5** | Every claim in the report traces to a numbered EVD row the orchestrator reproduced | read `evidence.md` and re-run each cited command | no claim without its row |

## Independent ground truth

- SymPy 1.14.0 at `C:\Users\ricar\dev\.lovelace-tools\python` (python 3.12.14).
- The published Native AOT binary `out/aot/Lovelace.Run.exe` for every wire claim.
- **New in cycle 6**: WSL2 Ubuntu 24.04 with .NET 10.0.401 at `~/ls` (provisioned by
  `docs/goal-cycle-6/wsl-setup.sh`) reproduces the GitHub `ubuntu-latest` environment locally, so a
  Linux-only CI failure can be diagnosed without spending runner minutes.

## Open work list (four rows, each already reproduced and located by cycle 5)

1. `RealLiteral.FromRealExact` drops provenance — `Lovelace.Symbolics/Expr.cs:82` stores digits +
   exponent and no exactness bit, so `evalf(sin(pi(1)*1/6), 40)` reports `exact:true` **with a rational
   numerator/denominator** while `pi/6` at 30 places is a truncation.
2. `--cancel-after` is ignored inside the numeric kernels — no `Cancellation` reference in
   `Lovelace.Array/Dsp/Real/Integer/Rational/Complex/Statistics`.
3. `evalf(f, 0)` raises `InternalError/InternalInvariantFailure` (NullReferenceException) on valid
   input; `evalf(1/3, 0)` returns `0` declared exact; `evalf(sqrt(2), 0)` ignores the count.
4. `capabilities()` under-claims — it advertises `pow.non-integer-exponent` while `(-4)^(1/2)` = `2i`
   succeeds, and three reachable refusals are unlisted (`solve.unevaluated`,
   `system-solve.unevaluated`, `integration.no-closed-form`).

Recorded and **not** fixed by cycle 5: `Real.Sin` of a *periodic* argument ignores the period; three
special-angle lines show true residuals instead of a hard zero (deliberate, documented);
`ComplexMath`'s series is quadratic in the ambient precision.

## Hard constraints (gates, carried forward from cycle 5 section 7)

1. **Native AOT must keep publishing and running**; re-publish before any audit and check freshness as
   "exe newer than the newest **code** file" (`*.cs/*.csproj/*.json/*.js/*.css/*.html`; documentation
   and `.worktrees/` excluded — recorded in `final-aot.ps1`).
2. Sections **C–J** of `docs/symbolics/a-plus-convergence-alignment-plan.md` are **frozen law**. Amend
   the document (sections N, O, then **P**) — never reduce scope silently.
3. No semantically meaningful string in a full machine API.
4. **No broad `catch (Exception)` in `Lovelace.Symbolics`** (currently 0 — re-verify). The one broad
   catch in `Lovelace.Suite/SuiteEngine.cs` records a diagnostic and rethrows; leave it.
5. `Assumptions.cs` carries a 324-query bound matrix and a 324-pair contradiction matrix; any change
   there requires the **full** suite.
6. **No test weakened, skipped or deleted to reach green.** Golden fixtures may be regenerated only when
   the contract deliberately changed, and then from the binary with the diff reviewed.
7. One bounded change per round, verified in the same round.
8. **Pushing is authorized** (D0 requires it). Do not force-push, do not rewrite history.

## Environment traps (carried forward)

- Windows PowerShell **5.1** only; there is no `pwsh`. `&&` and `||` are parse errors — use `;`.
  Never `"` inside a double-quoted PowerShell string; never name a parameter `$args`.
- `>` redirection writes UTF-16, which `git apply` rejects. Patches are produced with
  `git add -A ; git diff --cached --output=<name>.diff`; agents never stage their own report or patch.
- Bare `python3` is a Microsoft Store stub. Prepend
  `$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH` and set
  `$env:LOVELACE_REQUIRE_SYMPY='1'`.
- Kill stale `testhost` processes before measuring a zero-warning rebuild.
- A wall-clock test (`CancellationObservationTests`) fails under load and passes in isolation; the
  `Category=Timing` assertion is excluded from coverage runs **by design**.
- The `read` tool truncates lines at 2 000 characters; `write`/`edit` require a prior `read` in-session.

## Stop criteria

`Done` when D0–D5 all hold on the final tree, each verified by the orchestrator, and
`docs/symbolics/a-plus-cycle-6-report.md` exists. Claim **A+** only if a fresh adversarial audit could
not falsify it; otherwise the report names the open P0s and says why. Round cap **40**.

## Round plan (one narrow objective per round; re-planned after each landing)

| Round | Objective | Dimension |
|---|---|---|
| 1 | D0 — identify and fix the CI failure, push, poll every job to `success` | D0 |
| 2 | Register the three interrupted worktrees (c5-exact2, c5-cancel, c5-caps); land or discard each | D2 |
| 3 | Land `RealLiteral` provenance (`Expr.cs:82`) with a control-tree failure | D2 |
| 4 | Land `--cancel-after` inside the numeric kernels | D2 |
| 5 | Land `evalf(f, 0)` correctness | D2 |
| 6 | Land `capabilities()` honesty | D2 |
| 7 | Amendment section **P**: every residual bound CLOSED or ACCEPTED in writing | D3 |
| 8 | Fresh adversarial audit (new personas, new strategies) against the published binary | D1 |
| 9 | Close every P0/P1 the fresh audit found | D1 |
| 10 | Final re-measure: forced rebuild, 15-project sweep, AOT + smoke, capabilities, round-trip | D4 |
| 11 | Write `a-plus-cycle-6-report.md`; verify every claim traces to an EVD row | D5 |
