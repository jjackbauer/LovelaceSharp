# Triage — r21 `--cancel-after` in the numeric kernels (goal-cycle-6, round 2, row 2)

**VERDICT: LANDABLE AS-IS** — the patch applies clean to HEAD `300f6bb` with exit 0, the four affected
projects build in Release with 0 warnings / 0 errors, every pre-existing test still passes
(180/180 Run.Tests, 819/819 Suite.Tests, 61/61 Dsp.Tests), and the control run proves the new tests have teeth:
9 of the 11 new test methods fail on the pristine tree while all 11 pass on the patched one.

Shorthands used in the commands below (all read-only w.r.t. the main checkout):

| name | value |
|---|---|
| `$MAIN` | `C:\Users\ricar\dev\LovelaceSharp` |
| `$P` | `$MAIN\docs\goal-cycle-5\patches\wip-r21-cancel-UNVERIFIED.diff` (SHA256 `8F63092DC1D6B13F183CA4F9F3D9D9C6C26886BB7516B6D15E7D512F8B0F5E4F`) |
| `$T` | `$MAIN\.worktrees\c6-r21` (scratch tree, `git worktree add .worktrees/c6-r21 HEAD`) |
| `$C` | `$MAIN\.worktrees\c6-r21-control` (pristine control: HEAD **without** the product changes) |

## Step / command / observed result

| # | exact command | observed result |
|---|---|---|
| 1 | `git -C $MAIN worktree add .worktrees/c6-r21 HEAD` | exit 0; `HEAD is now at 300f6bb`; `git rev-parse HEAD` → `300f6bb639d70233b4fcc060aa2731979fed42f9`; `status --porcelain` empty |
| 2 | `git -C $T apply --check --verbose $P` | exit 0; `Checking patch` × 11, no error line. Applies as-is. |
| 3 | `git -C $T apply $P` | exit 0. Warning only: `$P:1290: new blank line at EOF.` + `warning: 1 line adds whitespace errors.` Result: 8 modified, 3 untracked (`M Lovelace.Dsp/DspMath.cs, DspPlugin.cs, FixedDsp.cs, Signals.cs; Lovelace.Run/RunProtocol.cs, Runner.cs; Lovelace.Suite/Interpreter.cs, TypedArrayOps.cs` + `?? Lovelace.Dsp/KernelCancellation.cs, Lovelace.Run.Tests/CancellationBudgetTests.cs, Lovelace.Suite.Tests/ArrayKernelCancellationTests.cs`). `git diff --stat` = 8 files, 346 insertions(+), 7 deletions(-). |
| 4 | `dotnet build $T\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)`; pulled in Lovelace.Dsp, Lovelace.Suite, Lovelace.Run. |
| 5 | `dotnet build $T\Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` |
| 6 | `dotnet build $T\Lovelace.Dsp.Tests\Lovelace.Dsp.Tests.csproj -c Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` |
| 7 | `dotnet test $T\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --no-build --logger "console;verbosity=normal"` | exit 0. `Total tests: 180 / Passed: 180`, `Total time: 27.1266 Seconds`. The 5 new ones: `Run_GivenBudgetFarBelowTheSum_… [693 ms]`, `Run_GivenExceededDeadline_… [122 ms]`, `Run_GivenWorkInsideTheBudget_… [4 ms]`, `Run_GivenBudgetFarBelowTheMatrixProduct_… [102 ms]`, `Run_GivenNoBudget_… [1 ms]` — all Passed. |
| 8 | `dotnet test $T\Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --no-build --logger "console;verbosity=normal"` | exit 0. `Total tests: 819 / Passed: 819`, `Total time: 15.5241 Seconds`. The 6 new ones: `SumOverRange… [130 ms]`, `ProductOverRange… [206 ms]`, `MatrixProduct… [107 ms]`, `WhileLoop… [202 ms]`, `ElementwiseArrayArithmetic… [116 ms]`, `RunInsideBudget… [1 ms]` — all Passed. |
| 9 | `dotnet test $T\Lovelace.Dsp.Tests\Lovelace.Dsp.Tests.csproj -c Release --no-build --logger "console;verbosity=minimal"` (regression: the patch edits the Dsp product) | exit 0. `Passed! - Failed: 0, Passed: 61, Skipped: 0, Total: 61, Duration: 4 s` |
| 10 | CLI probe, patched: `dotnet $T\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval "sum(1..10000000)" --json --omit-functions --omit-variables --print-budget 20 --cancel-after 100` (same for `matmul(eye(400), eye(400))`/100 ms and `prod(1..200000)`/200 ms) | `sum`: **wall 289 ms**, exit 1, `ok=false code=Cancelled category=BudgetExceeded cancellation={"budgetMs":100,"elapsedMs":124.081,"stopped":true,"exceeded":true,"excessMs":24.081}`. `matmul`: **wall 228 ms**, exit 1, `Cancelled/BudgetExceeded`, `elapsedMs 108.408`. `prod`: **wall 338 ms**, exit 1, `Cancelled/BudgetExceeded`, `elapsedMs 209.878`. |
| 11 | `git -C $MAIN worktree add .worktrees/c6-r21-control HEAD` then `Copy-Item` of exactly the two new test files from `$T` to `$C` | exit 0; `git status` in `$C` = `?? Lovelace.Run.Tests/CancellationBudgetTests.cs`, `?? Lovelace.Suite.Tests/ArrayKernelCancellationTests.cs` only. Copied bytes verified identical (SHA256 `486EA5EA…7769` and `E43CD103…E091` on both sides). No product file copied. |
| 12 | `dotnet build $C\Lovelace.Run.Tests\…csproj -c Release`; `dotnet build $C\Lovelace.Suite.Tests\…csproj -c Release` | both exit 0, `0 Warning(s) 0 Error(s)` — the new tests **compile against the unpatched product** (they use only pre-existing public API: `SuiteEngine.EvaluateAsync(string, TextWriter?, CancellationToken)` at `Lovelace.Suite/SuiteEngine.cs:226`, `EvaluationCancelledException` at `Lovelace.Suite/EngineExceptions.cs:25`, `Runner.RunAsync`). |
| 13 | `dotnet test $C\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --no-build --logger "console;verbosity=normal"` | **exit 1**. `Total tests: 180 / Passed: 176 / Failed: 4`. First failure line: `CancellationBudgetTests.cs(41,0)` → `Assert.Equal() Failure: Values differ / Expected: 1 / Actual: 0` (the run whose kernel never sees the budget exits 0). Failing: `Run_GivenBudgetFarBelowTheSum_…` (7 s), `Run_GivenBudgetFarBelowTheMatrixProduct_…` (9 s), `Run_GivenExceededDeadline_…` (7 s), `Run_GivenWorkInsideTheBudget_…` (1 ms). |
| 14 | `dotnet test $C\Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --no-build --logger "console;verbosity=normal"` | **exit 1**. `Total tests: 819 / Passed: 815 / Failed: 4`. First failure: `ArrayKernelCancellationTests.cs(39,0)` → `cancellation was not observed: the 200 ms budget expired but 'prod(1..200000)' was still running after 8.0 s (fence 8 s)`; likewise `sum(1..10000000)`, `matmul(eye(400), eye(400))` and the 100M-iteration while loop (four 8 s fence trips). |
| 15 | CLI probe, control (same three commands, `$C` dll) | `sum`: **wall 4104 ms, exit 0, ok=true, no cancellation key**. `matmul`: **wall 4153 ms, exit 0, ok=true, no cancellation key**. `prod`: **wall 27658 ms, exit 0, ok=true, no cancellation key**. |

## Control result and what it proves

- Patched vs control, same platform, same commands: the three CLI probes go from 4104 / 4153 / 27658 ms
  with `ok=true` and no ledger to 289 / 228 / 338 ms with `exit=1, code=Cancelled, category=BudgetExceeded`
  and a populated `cancellation` block. Row 2's contract ("must stop sum/prod/matmul **and be reported**")
  is observed directly, not inferred.
- New tests with teeth (fail on the pristine tree, pass on the patched one): **9 of 11**.
  - `CancellationBudgetTests`: 4/5 (sum, matmul, exceeded-diagnostic, in-budget-ledger).
  - `ArrayKernelCancellationTests`: 4/6 (sum, prod, matmul, while loop).
- **No teeth (pass in both trees), reported as required:**
  - `ArrayKernelCancellationTests.ElementwiseArrayArithmetic_GivenShortBudget_CancelsInsideTheMap`
    (`a = zeros(5000000); b = a + a; sum(b)`, 100 ms) — passes on the pristine tree in 1 s because the
    multi-statement script lets the pre-existing statement-boundary check cancel between statements; the
    map polling it claims to cover is therefore **not** what makes it pass.
  - `CancellationBudgetTests.Run_GivenNoBudget_CarriesNoCancellationBlockAtAll` — an assertion about the
    absence of the new field, true on both trees by construction.
- Two further new tests are declared negative controls and also pass on both trees:
  `ArrayKernelCancellationTests.RunInsideBudget_GivenGenerousBudget_StillReturnsTheResult` and (on the
  patched tree) `CancellationBudgetTests.Run_GivenWorkInsideTheBudget_SucceedsNormallyAndReportsNoExcess` —
  the latter does fail on the pristine tree (it asserts the new ledger), so only the former is a pure
  no-signal control.

## Seam check (new public member / new file / new wire field)

- **New files (3):** `Lovelace.Dsp/KernelCancellation.cs` (whole file), `Lovelace.Run.Tests/CancellationBudgetTests.cs`,
  `Lovelace.Suite.Tests/ArrayKernelCancellationTests.cs`.
- **New type, internal only:** `internal readonly struct KernelCancellation` — **duplicated**:
  `$T/Lovelace.Dsp/KernelCancellation.cs:20` (namespace `Lovelace.Dsp`) and appended at
  `$T/Lovelace.Suite/TypedArrayOps.cs:564` (namespace `Lovelace.Suite`; patch lines 1256-1288). Its members
  (`Interval`, `Capture`, `Poll`, `PollNow`) are `public` **inside an internal type**, so no assembly's public
  surface changes: a `Select-String '^\+.*\b(public|internal)\b'` sweep over the patch shows no added `public`
  type or member on a public type.
- **New wire fields (the real seam):** `cancellation` on **both** envelopes and `diagnostics` on the **success**
  envelope — `RunEnvelopeDto` gains `CancellationDto? Cancellation = null` and `DiagnosticDto[]? Diagnostics = null`
  (`$T/Lovelace.Run/RunProtocol.cs:80,83`), `RunErrorDto` gains `CancellationDto? Cancellation = null` (`:106`), new
  `internal sealed record CancellationDto` (`:62`) and `[JsonSerializable(typeof(CancellationDto))]` (`:119`). Both
  are omitted when null (`JsonIgnoreCondition.WhenWritingNull`), which is why the golden-envelope tests still
  pass. `ProtocolVersion` stays `1` (`Lovelace.Run/Runner.cs:37`).
- Product wiring: `CancellationLedger` / `OverrunDiagnostic` / `ComputeLineColumn` / `FormatMs`
  (`$T/Lovelace.Run/Runner.cs:333,352,370,391`) are all `private static`; the success path attaches the ledger at
  `:214`, the failure path at `:249`.
- Kernel polling added in `$T/Lovelace.Suite/Interpreter.cs:397,413,424,464,930,1045,1083`,
  `$T/Lovelace.Suite/TypedArrayOps.cs` (23 call sites) and the Dsp product (`DspMath.cs`, `FixedDsp.cs`,
  `DspPlugin.cs`, `Signals.cs`).
- Documentation: the patch does **not** touch `docs/symbolics/dsh-protocol.md`, whose error-envelope example
  (`:189-199`) does not mention `cancellation`; whether that doc is expected to move with a wire field was not
  determined (see below).

## What I could not determine

- Whether the project's convention requires `docs/symbolics/dsh-protocol.md` (unchanged, `:189-199`) to be updated
  when a wire field is added; the patch extends only the `--help` text.
- Whether the promptness fences are stable on a saturated machine. All patched observations sat far inside the
  fences (`Run.Tests` 693 ms vs a 2500 ms fence; `Suite.Tests` 107-206 ms vs an 8 s fence), but each suite was run
  **once**, and other agents' worktrees were active on this box during the runs.
- Whether anything outside the built set consumes the envelope: I built `Lovelace.Dsp`, `Lovelace.Suite`,
  `Lovelace.Run` and the three touched test projects, **not** the whole `LovelaceSharp.slnx` (`Lovelace.Console`,
  `Lovelace.Studio`, `Lovelace.Knowledge.*` were not built).
- Whether the two duplicated `KernelCancellation` copies are intended to stay in sync (their XML docs
  cross-reference each other); no drift is observable today, and they differ only in doc text and `using` lines.
- Whether the `ElementwiseArrayArithmetic` test's premise (that its map is what runs past the budget) can be made
  to have teeth; as written it is green on the unfixed tree.
- The author's intent for covering `prod` only in `Lovelace.Suite.Tests` and not in the end-to-end
  `Lovelace.Run.Tests` budget tests.

## Hygiene / scope notes

- Only reads and builds happened in `$MAIN`; all writes landed in `$T`, `$C` and this file. Nothing was committed,
  pushed or repaired, and no product or test file was edited.
- `$MAIN` also carries uncommitted edits to `docs/goal-cycle-6/evidence.md` and `docs/goal-cycle-6/state.md`; those
  are **not** from this run (another agent's activity), and `docs/goal-cycle-5/waiter6.ps1` was already untracked
  before the run started.
- Raw evidence logs: `$T\scratch-logs\patched-run-tests.log`, `patched-suite-tests.log`, `patched-dsp-tests.log`,
  `probe-cancel.ps1`; `$C\scratch-logs\control-run-tests.log`, `control-suite-tests.log`.
