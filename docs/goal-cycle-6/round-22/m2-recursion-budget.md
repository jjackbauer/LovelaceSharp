# Cycle-6 · round-22 · M-2 — the recursion budget is spent by a preceding statement (P1) — **FIXED**

| | |
|---|---|
| **Finding** | audit M, M-2 (P1): "the recursion budget is spent by *unrelated preceding statements*, so the documented `f(85)` is refused inside ordinary programs, and the refusal names 'nesting' with a depth number that is not monotone" — `docs/goal-cycle-6/round-22/audit-M-programs.md:121-205`, reproduced by the orchestrator as EVD-329 |
| **Tree** | `.worktrees/r22-m2`, branch `r22-m2`, based on `19ae61e` |
| **Commit** | `92ffe33912c29bc52780562de637b854337744fa` (not pushed, not merged) |
| **Control tree** | `.worktrees/r22-m2-ctl`, detached at `19ae61e` — the new tests fail there, 12 of 33 in-process and 2 of 4 on the wire |
| **Files** | `Lovelace.Suite/Interpreter.cs` (the fix), `Lovelace.Abstractions/InputDepth.cs` (the doc paragraph that described the credit), `Lovelace.Suite.Tests/RecursionBudgetCompositionTests.cs` (new, 33 tests), `Lovelace.Run.Tests/RecursionBudgetCompositionEnvelopeTests.cs` (new, 4 tests) |
| **Status** | fixed, test-first proven, all pins kept, whole-solution build 0 warnings / 0 errors, every test project green |

---

## 1. What was measured first, on the control tree

Command (both trees; `dotnet exec` on the JIT runner the tests spawn, same flags as audit M):

```powershell
& dotnet exec <worktree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file <script> --json --omit-functions --omit-variables
```

`DEF` is audit M's `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` on its own line; the probe ran every composition at N = 84, 85, 86, 87 in a fresh process. Audit M's own pair, verbatim, on the control tree:

```
CONTROL (19ae61e) — DEF + `{ 1; f(85) }`   (audit M's m02)
{"protocolVersion":1,...,"ok":false,"code":"DepthExceeded","category":"BudgetExceeded","message":"evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","recoverable":true,"diagnostics":[{"...":"...","position":56,"line":2,"column":1}],...}
exit=1
```

and the same bytes on the fixed tree:

```
FIXED (92ffe33) — DEF + `{ 1; f(85) }`
{"protocolVersion":1,...,"ok":true,"revision":82,"result":{"kind":"Natural","display":"0","typed":"0 (Natural)","structured":{"kind":"Natural","value":"0","exact":true}},"output":[],"variables":[],"functions":[],...}
exit=0
```

The full boundary scan (`max N accepted` per composition, and the depth the refusal published at `max N + 1`):

```
Case                   ctl maxN fix maxN ctl refuse fix refuse
----                   -------- -------- ---------- ----------
flat-top-level               85       85 depth=513  depth=513
block-single                 85       85 depth=513  depth=513
block-trailing-sibling       85       85 depth=513  depth=513
block-leading-1              84       85 depth=513  depth=513
block-leading-2              84       85 depth=513  depth=513
block-leading-3              84       85 depth=513  depth=513
block-leading-4              84       85 depth=514  depth=513
block-leading-5              84       85 depth=515  depth=513
block-leading-6              84       85 depth=513  depth=513
block-leading-8              84       85 depth=513  depth=513
block-leading-20             84       85 depth=513  depth=513
print-argument               84       85 depth=513  depth=513
plus-zero                    84       85 depth=513  depth=513
ident-argument               84       85 depth=513  depth=513
inside-if                    84       85 depth=513  depth=513
inside-for                   84       85 depth=513  depth=513
inside-while                 84       85 depth=515  depth=513
nested-blocks-3              85       85 depth=513  depth=513
four-top-level-before        85       85 depth=513  depth=513
wrapper-g (`func g() { return f(N) }; g()`)   84   84    depth=513  depth=513
```

That reproduces audit M's table exactly (including the non-monotone 513/514/515 and the 84 for every scaffold), and shows the fix: **every composition whose call is not nested under a live user frame now admits exactly the documented 85 and refuses 86 with the same published depth, 513.**

Live-frame wrappers were probed separately at 82–85 on both trees: `g` admits 84 and `h∘g` (`func h() { return g() }`) admits 83, on the control tree and on the fixed tree, unchanged. The boundary is now a monotone function of the call's **live** nesting: 85 (no live user frame above), 84 (one), 83 (two).

---

## 2. Where the baseline came from

All line numbers are the control tree's (`19ae61e`), `Lovelace.Suite/Interpreter.cs`:

```csharp
57  private const int CallLevelUnits = 4;
59  /// ... That call is the script's entry into user code — the BASE of the measurement ...
63  /// It is a CREDIT consumed by the statement's first charges rather than a per-call exemption, ...
66  private const int EntryCredit = CallLevelUnits + 2;      // 6
78  private int _entryCredit = EntryCredit;
87  private int EnterEvaluation(int units)
89      if (_entryCredit > 0)                                 // the credit is eaten FIRST
91          int free = Math.Min(_entryCredit, units);
92          _entryCredit -= free;
93          units -= free;
94          if (units == 0) return 0;
98      _evaluationDepth += units;
99      if (_evaluationDepth > InputDepth.MaxEvaluationDepth) throw ...;
114 private void ResetEvaluationBudget() { _entryCredit = EntryCredit; }
315 ResetEvaluationBudget();                                   // once per TOP-LEVEL statement
881 int charged = EnterEvaluation(CallLevelUnits);             // one user-function frame
```

The intended design is written in the doc comment: the six-unit credit is "the statement itself (one unit), the script's own outermost call expression (one unit) and the frame it pushes (4)" — i.e. the base of the measurement, so that the budget bounds "the depth the script's function bodies add below their entry". The documented f(85) is exactly 512 measured units, so the base must be subtracted *exactly once, uniformly*, for the boundary to sit at 85/86.

The implementation made the base a **consumable pool restored per top-level statement**, and every statement *inside* that top-level statement draws from the same pool:

* `{ f(85); 1 }` — the call runs first: statement (1) + call expression (1) + first frame (4) consume the six units → measured = raw − 6 → peak 512 → **answers**. The trailing `1` runs after the counter has unwound, so it costs nothing.
* `{ 1; f(85) }` — the sibling `1` consumes one unit of the pool *before* the call. The pool is never refilled (only the next **top-level** statement refills it), so the whole recursion is now measured at raw − 5 → the same call peaks at **513** → refused. This is the finding, exactly.
* k leading siblings drain min(k, 4) more units for the first four and, past that, shift *which* charge crosses 512 — hence the audit's non-monotone 513/514/515 for the same failing call.
* The same pool is drained by anything the statement evaluates on the way to the call — a builtin call (`print(f(N))`, one unit for the call expression), a binary operator (`f(N) + 0`, one), an argument (`ident(f(N))`, one), an `if` (one), a loop body (one) — which is why the audit's table shows **84** for all of those: the entire recursion was measured one unit deeper, for a program whose nesting had not changed.

So the baseline was `EntryCredit - (units the statement had already burned)`: the credit *was* a per-statement base, but it was consumed by the statement's own earlier charges instead of being applied once at the entry into user code. The counter itself (`_evaluationDepth`) was already balanced by its own `finally`s — the leak was in the credit, not in the depth.

---

## 3. The fix

The credit is replaced by an explicit, non-consumable **base of the measurement**, found once per top-level statement, at the statement's entry into user code (`Lovelace.Suite/Interpreter.cs`):

```csharp
    /// ... A SIBLING statement is not part of the measurement. A statement that has already run has
    /// returned every unit it charged before this one starts, so it cannot spend this call's budget ...
    /// What DOES move the measurement is a user-function frame that is still LIVE when the call runs ...
    private void EstablishEvaluationBase()
    {
        if (_userFrameDepth == 0 && !_evaluationBaseEstablished)
        {
            _evaluationBaseEstablished = true;
            _evaluationBase = _evaluationDepth + CallLevelUnits;
        }
    }

    private int EnterEvaluation(int units)
    {
        _evaluationDepth += units;
        int depth = _evaluationDepth - _evaluationBase;
        if (depth > InputDepth.MaxEvaluationDepth)
        {
            _evaluationDepth -= units;   // leave the counter exactly as it was found
            throw new InputDepthExceededException("evaluation", depth, InputDepth.MaxEvaluationDepth, "evaluation");
        }
        return units;
    }

    private void ResetEvaluationBudget()   // per top-level statement, and per EvaluateAsync(Expr)
    {
        _evaluationBase = 0;
        _evaluationBaseEstablished = false;
    }
```

and in `CallUserFunctionAsync`, immediately before the frame's units are charged:

```csharp
        EstablishEvaluationBase();
        int charged = EnterEvaluation(CallLevelUnits);
```

Why this shape, and not the three obvious alternatives:

* **Restore the credit per statement rather than per top-level statement.** That would hand a fresh six-unit exemption to *every* nesting level: 120 nested `if`s in a recursive body (the shape whose pre-fix death is pinned by `RecursionDepthEnvelopeTests.NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying`) would get 120 × 6 free units and stop being refused. The base has to be found once.
* **Stop charging anything above the first user frame (measure only inside user functions).** That loses the guard on a walk with no user call at all — including a host-built `Expr` handed to `Interpreter.EvaluateAsync(Expr)`, which never passes `AstDepth`. The chosen form keeps it: a statement that never enters user code measures from 0, exactly as before (`_evaluationBase` stays 0), so nothing is weakened; the only change there is that the 6-unit credit is gone, which *tightens* that walk by 6 units — and the parser's own `InputDepth.MaxParsedTreeDepth = 512` bounds what can reach the interpreter (`AstDepth` walks the whole program, statements included), so no input the parser admits is newly refused. Measured: `ADeepFlatChain_IsStillEvaluated` (500-term top-level chain, 501 units) still answers; a 510-term chain is the deepest the 512 parsed-tree budget admits, at 511 units.
* **Keep the credit and reset it at every *call*.** Same as the per-statement variant, one level down: each recursion level would collect a fresh exemption and nothing would ever be refused.

`Lovelace.Abstractions/InputDepth.cs` carried the prose that described the credit ("starts each top-level statement with a six-unit credit"); it now describes the base, and records why a sibling cannot spend it and what does move it.

### The residual, stated loudly

A composition that keeps a user-function frame **live** when the call runs (`func g() { return f(N) }; g()`, mutual recursion, a chain of wrappers) still refuses `f(85)` and admits 84 — one call level per live frame (85 / 84 / 83 measured, on both trees). This is not a leftover of the bug: it is the metric. The call is genuinely one user frame deeper, and the budget is 512. It is also arithmetically forced: `f(85)` and `f(86)` differ by exactly one frame (6 units), and `f(85)` *is* the limit (512 measured), so any composition that adds even one live frame must cross it; admitting `f(85)` under a live wrapper while refusing `f(86)` at top level would require an effective budget larger than 512, which would move the pinned boundary the other way. What the fix removes is exactly what the finding named: siblings, statement order and dead scaffold cannot move the boundary — the probe shows `max N = 85` for all 19 of those compositions, and the boundary now decreases monotonically with live nesting (85 → 84 → 83).

---

## 4. Test-first proof

Two new files, byte-identical in both trees:

* `Lovelace.Suite.Tests/RecursionBudgetCompositionTests.cs` — 33 tests: a 16-shape composition table × (the deepest accepted call answers / one past the boundary is refused, typed and named), plus `TheBoundary_MovesOnlyWithLiveNesting` (85 for every scaffold composition, 84 and 83 for one and two live frames).
* `Lovelace.Run.Tests/RecursionBudgetCompositionEnvelopeTests.cs` — 4 wire tests through a real runner process: audit M's m02 answers 0; m01 and m02 at 85 both answer (1 and 0); `f(86)` and `{ 1; f(86) }` publish **the same** depth; a live wrapper frame still refuses.

Control tree (`git worktree add --detach .worktrees/r22-m2-ctl 19ae61e`), with the new files copied in:

```
PS> cd .worktrees/r22-m2-ctl; dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionTests"
  Failed ...TheDeepestAcceptedCall_Answers_InEveryComposition(name: "a block with one preceding sibling", body: "{ 1; f({0}) }", answer: "0") [10 ms]
  Error Message:
   Lovelace.Abstractions.InputDepthExceededException : evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).
  Stack Trace:
     at Lovelace.Suite.Interpreter.EnterEvaluation(Int32 units) in ...\.worktrees\r22-m2-ctl\Lovelace.Suite\Interpreter.cs:line 103
   at Lovelace.Suite.Interpreter.EvaluateAsync(Expr expr, Scope scope) in ...\Interpreter.cs:line 375
   ...

Failed!  - Failed:    12, Passed:    21, Skipped:     0, Total:    33, Duration: 825 ms - Lovelace.Suite.Tests.dll (net10.0)
```

The 12 failures are the sibling compositions (1, 2, 4, 5 and 20 preceding siblings), `print(f(85))`, `f(85) + 0`, `ident(f(85))`, the `if`/`for`/`while` bodies, and `TheBoundary_MovesOnlyWithLiveNesting`; the refusal texts in that run are `evaluation depth 513`, **514** (four siblings) and **515** (five siblings) — the audit's non-monotone number, reproduced by the new test. The 21 that pass are the shapes the control tree already got right (top level, single-statement block, trailing sibling, nested blocks, four top-level statements before, and all 16 `one past the boundary` cases, which refuse on both trees).

```
PS> cd .worktrees/r22-m2-ctl; dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionEnvelopeTests"
  Failed ...RecursionBudgetCompositionEnvelopeTests.APrecedingSiblingStatement_DoesNotSpendTheCallBudget [153 ms]
  Error Message:
Expected: 0
Actual:   1
  Failed ...RecursionBudgetCompositionEnvelopeTests.BothBlockOrders_AnswerAtTheDocumentedBoundary [452 ms]
  Error Message:
Expected: 0
Actual:   1

Failed!  - Failed:     2, Passed:     2, Skipped:     0, Total:     4, Duration: 1 s - Lovelace.Run.Tests.dll (net10.0)
```

(the exit code the control runner returned for m02 is 1 where the test wants 0; the two that pass there are the ones the bug does not touch — the shared depth of the two refusal shapes and the wrapper refusal).

Fixed tree, same files:

```
PS> cd .worktrees/r22-m2; dotnet test Lovelace.Suite.Tests/... --filter "...RecursionBudgetCompositionTests"
Passed!  - Failed:     0, Passed:    33, Skipped:     0, Total:    33, Duration: 994 ms - Lovelace.Suite.Tests.dll (net10.0)

PS> cd .worktrees/r22-m2; dotnet test Lovelace.Run.Tests/... --filter "...RecursionBudgetCompositionEnvelopeTests"
Passed!  - Failed:     0, Passed:     4, Skipped:     0, Total:     4, Duration: 1 s - Lovelace.Run.Tests.dll (net10.0)
```

---

## 5. The existing pins — checked, none of them pins the bug

`Lovelace.Suite.Tests/RecursionDepthGuardTests.cs` and `Lovelace.Run.Tests/RecursionDepthEnvelopeTests.cs` pin the round-7 closure of O-B11. Read line by line: **every** shape they build puts the call at the top level (`"func f(n) {...}; f(N)"` in-process, `"func f(n) {...}\nf(N)"` on the wire) and nothing varies the surrounding statement, so **no existing test pins the composition-dependent behaviour** — nothing had to be weakened, skipped, deleted, or re-pinned, and none was. The specific pins that had to keep passing, and do, on the fixed tree:

| pin | where | result |
|---|---|---|
| `f(85)` answers, `f(86)` refuses, with `Limit == 512` and the message shape | `RecursionDepthGuardTests` (in-process) | pass |
| same boundary on the wire, plus `f(0/1/8/64/85)` answer and `f(86/200/432/433/448/2000/100000)` refuse | `RecursionDepthEnvelopeTests` | pass |
| `f(448)` is a well-formed `DepthExceeded`/`BudgetExceeded` envelope, not a process death | `RecursionDepthEnvelopeTests` | pass |
| 5 call levels × 120 nested `if`s in the body are refused (the shape a call-level cap cannot see) | `RecursionDepthEnvelopeTests.NestedBlocksInsideARecursiveBody_...` | pass |
| `f(10)` with a 100-term chain in the body is refused | `RecursionDepthGuardTests.ADeepExpressionInsideTheRecursiveBody_IsRefused` | pass |
| `fact(10)`, `even(40)`, `fib(20)` still answer; a 500-term flat chain still evaluates | `RecursionDepthGuardTests.OrdinaryRecursion_KeepsWorking`, `.ADeepFlatChain_IsStillEvaluated` | pass |
| a parse-phase depth refusal still crosses as `DepthExceeded` | `ParseErrorClassificationTests`, `InputDepthGuardTests`, `ValueDepthGuardTests`, `ValueDepthEnvelopeTests` | pass |

Nothing a correct run publishes changed: the result values, the envelope shape, the codes and categories are untouched — the only behaviour that changed is which programs are refused, and only in the direction the finding asked for (three contexts that used to refuse now answer, and no context that answered now refuses — verified by the before/after sweep, whose only *refusal*→*answer* rows are the 16 compositions plus their boundaries).

---

## 6. Totals on the fixed tree

```
dotnet build LovelaceSharp.slnx -c Release --no-incremental
    0 Warning(s)
    0 Error(s)
```

| project | result |
|---|---|
| Lovelace.Suite.Tests | **885 / 0** (852 existing + 33 new) |
| Lovelace.Run.Tests | **335 / 0** (331 existing + 4 new) |
| Lovelace.Symbolics.Tests | 1185 / 0 (8 skipped) |
| Lovelace.Abstractions.Tests | 20 / 0 |
| Lovelace.Console.Tests | 15 / 0 |
| Lovelace.Dsp.Tests | 61 / 0 |
| Lovelace.Array.Tests | 19 / 0 |
| Lovelace.Knowledge.Tests | 28 / 0 |
| Lovelace.Studio.Tests | 22 / 0 |
| Lovelace.Representation.Tests | 91 / 0 |
| Lovelace.Natural.Tests | 195 / 0 |
| Lovelace.Integer.Tests | 148 / 0 |
| Lovelace.Complex.Tests | 121 / 0 |
| Lovelace.Real.Tests | 2495 / 0 |
| precbench.Tests | 13 / 0 |

The sweep ran on the final product code; `Lovelace.Suite.Tests` and `Lovelace.Run.Tests` were re-run on the final commit `92ffe33` (`885/0` and `335/0`), and the whole-solution `--no-incremental` build was re-run on it (0/0). One run of the full `Lovelace.Suite.Tests` on a loaded machine flaked the wall-clock test `CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly` (`Category=Timing`, "cancellation was observed but only after 26271 ms") — it passed in the quiet re-run and is unrelated to the touched code (it evaluates `2000000!`, no user function).

---

## 7. What I could not do

* **The published binary is not re-published.** `out/aot/Lovelace.Run.exe` (and the REPL/Studio AOT binaries) still contain the pre-fix product code; the fix is in the worktree commit only, as instructed (no push, no merge). Re-publishing and re-running the wave is the orchestrator's step.
* **The `error[MSB4166]` in the first `--no-incremental` attempt** (an MSBuild child node exited while another build and the Real suite were running in parallel) is environmental, not a code error: the clean re-run is 0 warnings / 0 errors. It is recorded here so the number is not mistaken for a flake in this change.
* **The residual in §3** (a live wrapper frame still costs one call level, so `f(85)` under `func g() { return f(N) }; g()` is refused and the boundary there is 84) is impossible to remove without moving the published top-level boundary; it is stated here rather than hidden. The audit's table listed that row as part of the finding, so the orchestrator should re-point it at the metric (live nesting) rather than at composition.

---

## 8. Reproduce

```powershell
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/r22-m2 -b r22-m2 19ae61e
git -C C:\Users\ricar\dev\LovelaceSharp worktree add --detach .worktrees/r22-m2-ctl 19ae61e

# control: the new tests fail
cd .worktrees/r22-m2-ctl; git apply <the two test files: see the commit diff>
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionTests"   # 12 failed / 21 passed
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionEnvelopeTests"   # 2 failed / 2 passed

# fixed: the same tests pass
cd .worktrees/r22-m2; git checkout 92ffe33
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionTests"   # 33 / 0
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --filter "FullyQualifiedName~RecursionBudgetCompositionEnvelopeTests"   # 4 / 0
dotnet build LovelaceSharp.slnx -c Release --no-incremental   # 0 warnings / 0 errors
```

Scratch drivers (outside the repo, `%TEMP%\r22-m2\`): `probe.ps1` (the composition × N sweep through a real runner), `wrappers.ps1`, `compare.ps1` (the before/after table), `m02.ps1`. Nothing in the main tree was modified except this report.
