# Round 32 — N17: cancellation is not observed by long-running numeric work

STATUS: DONE — fix implemented, all suites green, published binary verified (2208 ms vs the orchestrator's 60 000 ms).

Repo: `C:\Users\ricar\dev\LovelaceSharp`. No git commit was made.
Changed files: `Lovelace.Natural/Natural.cs`, `Lovelace.Suite.Tests/CancellationObservationTests.cs`
(new), `docs/goal-cycle-3/round-32/{implementation.md,verify.ps1}`.
Nothing outside SCOPE was edited: no edit to `Lovelace.Suite/Interpreter.cs` or
`Lovelace.Abstractions/Cancellation.cs` was needed — the seam was sufficient as-is.

## The seam reused (no second mechanism, no token parameters)

The ambient token in `Lovelace.Abstractions/Cancellation.cs:19`
(`Cancellation.ThrowIfCancellationRequested()`, backed by an `AsyncLocal<CancellationToken>`,
`Cancellation.cs:11-14`), installed once per evaluation by
`Lovelace.Suite/SuiteEngine.cs:223` `using var cancellation = Cancellation.Scope(cancellationToken);`
and polled today only at `Lovelace.Suite/Interpreter.cs:231` (per **statement**).
The typed end of the path already exists and is untouched:
`SuiteEngine.cs:255-261` turns the `OperationCanceledException` into
`EvaluationCancelledException`, and `Lovelace.Run/Runner.cs:258` classifies it as
`code = Cancelled, category = BudgetExceeded, recoverable = true` with a non-zero exit.

`SubProgress` (`Interpreter.cs:96-99`) carries an `IProgress<double>`, not the token, so it is the
wrong carrier; the ambient token is exactly what the symbolic kernels poll (Groebner, Solve,
Simplify, Expand, Rewriting, Diff, Integrate). **Reused that; no new cancellation mechanism, and no
arithmetic signature gained a parameter.**

## Stride chosen, and why

Polling every iteration of a hot loop would tax ordinary arithmetic; never polling is N17. All polls
live in `Lovelace.Natural/Natural.cs` and are gated by size or by a work stride:

| site | stride / gate | why |
| --- | --- | --- |
| `Natural.Pow` loop head (`Natural.cs`, binary exponentiation) | operand ≥ **4096 limbs** (≈ 78 000 decimal digits) | the loop is only O(log₂ e) iterations (~30 for 2^1000000000), but one squaring can run for seconds — this is the poll the statement-level loop never had |
| `MultiplyCore` recursion head | min(operand limbs) ≥ **4096** | the choke point of every product; each node above the gate is ≥ 4096² limb-multiplies of work, so an `AsyncLocal` read is unmeasurable, while no more than one such subtree can elapse between polls |
| `SchoolbookMultiply` row loop | one poll per **≈ 4 000 000 limb-multiplies** of work (`rowStride` computed once per call, not per row) | covers the unbounded one-sided product (e.g. 40 × 15M limbs ≈ 600M limb-multiplies) that the min-limb gate alone would miss |
| `Natural.Factorial` parallel factor range | every **4096 factors**, plus the value-sized polls inside each product | the factor range is genuinely unbounded (n can be astronomically large); the sequential branch is bounded by 2 × ProcessorCount and is documented as needing no poll |
| `Ntt` stage loop | once per transform **stage** (unconditional) | one stage of a 2²³-point transform is ~8M butterflies; Ntt is only entered at ≥ 100 000 combined limbs, so the poll is free there |
| `MultiplyCore` (`Parallel.Invoke`), `UnbalancedMultiply` (`Parallel.For`), `Factorial` (`Parallel.For`) | `AggregateException` unwrap (RethrowIfCancelled) | `Parallel` wraps a body exception in `AggregateException`, which the engine does **not** classify as cancellation; unwrapping keeps the *existing* typed path |

Small operands therefore pay one integer compare per call and never read the token.

## Step A — test-first, observed FAILING output (before the fix)

Test: `Lovelace.Suite.Tests/CancellationObservationTests.cs` — a 1500 ms
`CancellationTokenSource` around `2^1000000000` / `2000000!`, offloaded with `Task.Run` (so the
assertion really can fence) and a 30 s `Task.WhenAny` fence so the test cannot hang forever.

```
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~CancellationObservationTests"

  Failed Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly [30 s]
  Error Message:
   cancellation was not observed: the 1500 ms budget expired but 2000000! was still running after 30.0 s (test fence 30 s).
  Stack Trace:
     at Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite.Tests\CancellationObservationTests.cs:line 54

  Failed Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly [29 s]
  Error Message:
   Assert.Throws() Failure: No exception was thrown
Expected: typeof(Lovelace.Suite.EvaluationCancelledException)
  Stack Trace:
     at Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite.Tests\CancellationObservationTests.cs:line 37

Failed!  - Failed:     2, Passed:     0, Skipped:     0, Total:     2, Duration: 59 s - Lovelace.Suite.Tests.dll (net10.0)
```

Read: with a 1500 ms budget, `2^1000000000` ran to **completion after 29 s** and returned a value —
no exception, ~19× over budget; `2000000!` was still running when the 30 s fence fired. The
statement-level poll (`Interpreter.cs:231`) polls *between* statements, never inside one.

## Step C — same tests after the fix (observed PASSING output)

```
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~CancellationObservationTests"
Test run for C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite.Tests\bin\Release\net10.0\Lovelace.Suite.Tests.dll (.NETCoreApp,Version=v10.0)
Passed!  - Failed:     0, Passed:     2, Skipped:     0, Total:     2, Duration: 3 s - Lovelace.Suite.Tests.dll (net10.0)
```

Both tests hold a 1500 ms budget; the class (two tests, run in parallel) settles in **3 s total** and
each task returns `EvaluationCancelledException` instead of a value.

## Verification (observed output, `docs/goal-cycle-3/round-32/verify.ps1`)

```
=== dotnet build LovelaceSharp.slnx -c Release --nologo ===
    8 Warning(s)
    0 Error(s)
Time Elapsed 00:00:06.84
BUILD_EXIT=0
=== dotnet test Lovelace.Symbolics.Tests -c Release --nologo ===
Passed!  - Failed:     0, Passed:   723, Skipped:     6, Total:   729, Duration: 17 s - Lovelace.Symbolics.Tests.dll (net10.0)
TEST_EXIT=0
=== dotnet test Lovelace.Suite.Tests -c Release --nologo ===
Passed!  - Failed:     0, Passed:   640, Skipped:     0, Total:   640, Duration: 12 s - Lovelace.Suite.Tests.dll (net10.0)
TEST_EXIT=0
=== dotnet test Lovelace.Run.Tests -c Release --nologo ===
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 789 ms - Lovelace.Run.Tests.dll (net10.0)
TEST_EXIT=0
=== dotnet publish Lovelace.Run -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot ===
  Generating native code
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\
PUBLISH_EXIT=0
```

All suites end **0 failed** (723 + 640 + 39 passed, 6 pre-existing skips in Symbolics).

## Published binary vs the orchestrator's 60 000 ms

```
=== published binary: 2^1000000000 --cancel-after 2000 ===
WALL_MS=2208
PROCESS_EXIT=1
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"Cancelled","category":"BudgetExceeded",
 "message":"the evaluation was cancelled by the caller.","recoverable":true,
 "diagnostics":[],"elapsed":"2.11 s","partialOutput":[],"partialVariables":[]}
```

| | orchestrator (before) | after |
| --- | --- | --- |
| wall clock for `2^1000000000 --cancel-after 2000` | still running when killed at **60 000 ms** (30× budget) | **2 208 ms** (≈ 1.1× budget) |
| envelope | none (no cancellation observed) | `code=Cancelled`, `category=BudgetExceeded`, `recoverable=true`, exit 1 |

The published binary now stops **≈ 27× sooner** than the orchestrator's 60 000 ms observation and
reports the existing typed path: `Cancelled` / `BudgetExceeded` / recoverable `true`, non-zero exit,
with the partial state (`partialOutput`, `partialVariables`) available in the same envelope.

## Hot path

Pre-fix baseline with the previously published AOT binary: `2^2000000` = **606 ms**
(`out/aot/Lovelace.Run.exe`, same machine, same command). Post-fix publish of the same expression:

```
HOTPATH expr=2^2000000 wall=542 ms exit=0
```

No measurable slowdown (the 606 → 542 ms delta is run-to-run noise in the *faster* direction): the
size gates mean small-operand products execute one integer compare and never read the token.

## Loops audited but not instrumented

- `DivRemKnuth` quotient loop (`Natural.cs`) and the Newton-reciprocal division loops: unbounded in
  principle (a huge dividend divided by a small divisor), not instrumented — N17's acceptance case
  and the "unbounded numeric statement" class are products/powers/factorials; division by a value
  large enough to be slow already polls inside its own multiplies.
- `NttStage` butterfly inner loop: covered at stage granularity (poll once per stage), not per
  butterfly.
- `Natural.ToStringRecursive` / decimal conversion of an already-computed giant value: not
  instrumented (rendering, not arithmetic; it is entered only after the value exists).
- `Lovelace.Integer` and the interpreter layer add no loops of their own on this path:
  `Integer.Pow` delegates to `Natural.Pow`, `Integer.Factorial` to `Natural.Factorial`,
  and `Lovelace.Suite/NumericOps.cs:48` dispatches `^` straight to `Natural.Pow`.
