# Round 36 — implementation: N16a (bare cancel carries no condition) and N16b (power of a sum)

**Status: DONE.** No git commit was made. Primary artifacts changed:
`Lovelace.Symbolics/Algebra/RationalFunctions.cs`, `Lovelace.Symbolics/SymbolicsPlugin.cs`,
`Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs`,
`Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs`.
Nothing outside SCOPE was touched (`Polynomial.cs` was NOT modified; the rewrite-rule path in
`Simplify.cs` was NOT modified).

## 1. Decisions

### N16a — a bare expression CANNOT carry a condition, so the structured form is the answer

The kernel value model has one slot per result: a bare builtin returns an `Expr` (surfaced as a
`Symbolic` value). There is no place on that value to hang a side condition, and bolting one on
(for example by replacing the result with a conditional expression) would be exactly the "second
convention" the brief forbids. So:

* **Bare `cancel` keeps returning the reduced expression** (unchanged value semantics:
  `cancel(x/x) = 1`), and its `BuiltinDescriptor` now says so explicitly — the result is a bare
  value that cannot carry the side conditions, holds only off the pole it removes, and callers who
  need the definedness delta must use `cancel_full`. That descriptor text is the "say what a caller
  should use instead" part.
* **New `cancel_full(f)` builtin** returns a `CancelResult` record with
  `status` (`CancelStatus`), `original`, `expression`, `changed`, `conditions` — the same shape
  and the same `ConditionExprs` projection the other `*_full` builtins use
  (`simplify_full`, `integrate_full`, `limit_full`), so no new convention was invented.
* **The condition atoms are literally the rewrite path's atoms.** `RationalFunctions.NonZeroCondition`
  mirrors `Simplify.NonZeroOf`: a symbol becomes `SymbolPropertyAssumption(s, NonZero)`, anything else
  `ExpressionPropertyAssumption(e, NonZero)`. For `x/x` the structured cancel emits the *same*
  atom object the `rat.cancel-x-over-x` rule attaches (asserted atom-for-atom in
  `CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath`), so the machine API and the
  rule path now state the identical condition. The atoms, not the helper, are the convention; the
  helper is mirrored because `Simplify.cs` is out of scope this round.
* **The condition is not decoration.** Cancelling g out of n/d yields a form that agrees with the
  input exactly where g ≠ 0; at a zero of g the input is undefined while the reduced form may be
  defined, i.e. cancellation *extends* the domain. That is why `status = Conditional` is reported
  rather than an everywhere-identity claim.

**A caller should use `cancel_full` (or the kernel `RationalFunctions.CancelWithConditions`) whenever
the definedness delta matters; the bare `cancel` remains the convenience value form.**

### N16b — cancel expands before parsing; the parser contract is NOT changed

Chosen: **make cancel expand such an input before attempting the reduction**, in
`RationalFunctions.CancelWithConditions`, via a private `TryPolynomial` that retries
`Polynomial.TryFromExpr` on `Algebra.Expand(e, ctx)`. The alternative — teaching
`Polynomial.TryFromExpr` to accept `Pow(sum, k)` — was rejected because:

1. `TryFromExpr` is a shared parser with many callers (apart, factoring, series branches, the
   polynomial-identity checks). Widening it changes branch selection and cost everywhere, not just in
   cancel; the round's blast radius would be the whole algebra layer for a defect observed in one
   builtin.
2. Expansion is **value-preserving on every input** — it adds no definedness delta and therefore
   needs no condition — so doing it inside cancel is unconditionally sound. Cancellation itself is
   conditional (N16a); keeping the unconditional step local and explicit preserves that distinction.
3. The parser's documented contract ("integer powers of variables") stays true and remains pinned.

Behaviour is pinned by a converted test, and the parser boundary is still asserted exactly as before
(`TryFromExpr(Pow(sum,2)) == false`) so the test states *where* the fix lives. The expanded spelling
still reduces to the same result, so both spellings now agree.

## 2. Step A — failing output BEFORE the fix (test-first)

Command:
`dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~MetamorphicSetClosureTests"`

```
  Failed Lovelace.Symbolics.Tests.MetamorphicSetClosureTests.CancelFull_ReportsTheConditionsTheBareValueCannotCarry [18 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'cancel_full'.
  Stack Trace:
     at Lovelace.Suite.Interpreter.EvaluateCallAsync(CallExpr call, Scope scope) in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite\Interpreter.cs:line 619
     ...
     at Lovelace.Symbolics.Tests.MetamorphicSetClosureTests.CancelFull_ReportsTheConditionsTheBareValueCannotCarry() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics.Tests\MetamorphicSetClosureTests.cs:line 566
  Failed Lovelace.Symbolics.Tests.MetamorphicSetClosureTests.Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing [4 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: AddExpr { IsExact = True, Kind = Add, NodeCount = 3, StructuralHash = -326327615, Terms = [RationalConstantExpr { ... Value = -1 }, SymbolExpr { ... Symbol = x }] }
Actual:   MultiplyExpr { Factors = [PowerExpr { Base = AddExpr { ... }, Exponent = RationalConstantExpr { ... }, ... }, PowerExpr { Base = AddExpr { ... }, Exponent = RationalConstantExpr { ... }, ... }], IsExact = True, Kind = Multiply, NodeCount = 11, StructuralHash = 979222751 }
  Stack Trace:
     at ...MetamorphicSetClosureTests.Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing() in ...MetamorphicSetClosureTests.cs:line 619
  Failed Lovelace.Symbolics.Tests.MetamorphicSetClosureTests.Cancel_PreservesValueOffPoles_AndThePoleIsExcluded [5 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: AddExpr { IsExact = True, Kind = Add, NodeCount = 3, StructuralHash = -326327615, Terms = [RationalConstantExpr { ... Value = -1 }, SymbolExpr { ... Symbol = x }] }
Actual:   MultiplyExpr { Factors = [ ... same unreduced product ... ], NodeCount = 11, StructuralHash = 979222751 }
  Stack Trace:
     at ...MetamorphicSetClosureTests.Cancel_PreservesValueOffPoles_AndThePoleIsExcluded() in ...MetamorphicSetClosureTests.cs:line 441

Failed!  - Failed:     3, Passed:     5, Skipped:     0, Total:     8, Duration: 201 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Three genuine failures: `cancel_full` did not exist ("Unknown function 'cancel_full'"), and both
`cancel((x-1)^2/(x-1))` assertions (the direct kernel call and the new corpus member) got the
unreduced product back instead of `x - 1`.

## 3. Characterisation tests converted (not deleted, not weakened)

| Test | Old assertion | New assertion |
| --- | --- | --- |
| `Cancel_UnexpandedPowerOfASum_IsNotReduced_KnownGap` (renamed `Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing`) | `Cancel((x-1)^2/(x-1)) == (x-1)^2/(x-1)` — the known gap, with the parser cause pinned | `Cancel(...) == x - 1`, and the same for the expanded spelling; the parser-boundary assertions (`TryFromExpr(Pow(sum,2))` false, `TryFromExpr(Pow(x,2))` true) are **kept verbatim** |
| `Cancel_PreservesValueOffPoles_AndThePoleIsExcluded` corpus | no member for the unexpanded spelling | **added** the `(x-1)^2/(x-1)` member (expected `x - 1`, pole 1, pole removed) — the corpus now covers the N16b shape; the existing members are untouched |
| `BuiltinCount_IncludesTheThreeHyperbolicFunctions` (tripwire) | 106 | 107, with the comment extended: "round 36 added exactly one more (`cancel_full` … — N16a)" — the same narrative the round-22/round-28 updates used, not a weakened assertion |

Kept as-is (still true, so converting would be wrong): the bare-builtin assertions
`cancel(x/x) = 1`, `cancel((x^2-1)/(x-1)) = x + 1`, `cancel((x^2+1)/(x-1))` unchanged, and the
whole rewrite-path test `Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath` (its stale
doc comment now points at `cancel_full` instead of "see the report").

New tests added: `CancelFull_ReportsTheConditionsTheBareValueCannotCarry` (engine surface: status,
original, expression, changed, structured `[x != 0]` condition, the `Exact`/empty-conditions case,
`x - 1 != 0` for the N16b shape, and property access `cancel_full(x/x).conditions[0]`) and
`CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath` (kernel surface: the atom-for-atom
equality with `Simplify.Transform`).

## 4. Step C — passing output

```
dotnet build LovelaceSharp.slnx -c Release --nologo
    5 Warning(s)
    0 Error(s)
Time Elapsed 00:00:05.37
```

```
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   742, Skipped:     6, Total:   748, Duration: 15 s - Lovelace.Symbolics.Tests.dll (net10.0)
```
(The 6 skips are the pre-existing Sympy-oracle skips `SympyOracle_*`, unrelated to this round.)

```
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   642, Skipped:     0, Total:   642, Duration: 11 s - Lovelace.Suite.Tests.dll (net10.0)

dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 447 ms - Lovelace.Run.Tests.dll (net10.0)
```

No Run.Tests golden changed (39/39 passed on the first run; `Run` tests never touched the cancel
surfaces).

## 5. JIT runner probes (observed output, functions array elided for readability)

```
--- cancel(x/x) ---
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Symbolic","display":"1","typed":"1 (Symbolic)","structured":{"kind":"Symbolic","pretty":"1","canonical":"(rat 1 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}},"output":[],"variables":[{"name":"_","kind":"Symbolic","display":"1"},{"name":"x","kind":"Symbolic","display":"x"}],...}
--- cancel((x^2-1)/(x-1)) ---
{"protocolVersion":1,...,"ok":true,"revision":82,"result":{"kind":"Symbolic","display":"x + 1","typed":"x + 1 (Symbolic)","structured":{"kind":"Symbolic","pretty":"x + 1","canonical":"(add (rat 1 1) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}},...}
--- cancel((x-1)^2/(x-1)) ---
{"protocolVersion":1,...,"ok":true,"revision":82,"result":{"kind":"Symbolic","display":"x - 1","typed":"x - 1 (Symbolic)","structured":{"kind":"Symbolic","pretty":"x - 1","canonical":"(add (rat -1 1) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}},...}
--- cancel_full((x-1)^2/(x-1)) ---
{"protocolVersion":1,...,"ok":true,"revision":82,"result":{"kind":"Record","display":"CancelResult(status: Conditional, original: (x - 1)^2/(x - 1), expression: x - 1, changed: True, conditions: [x - 1 != 0])","typed":"... (CancelResult)","structured":{"kind":"Record","type":"CancelResult","fields":[{"name":"status","value":{"kind":"Enum","type":"CancelStatus","value":"Conditional"}},{"name":"original","value":{"kind":"Symbolic","pretty":"(x - 1)^2/(x - 1)",...}},{"name":"expression","value":{"kind":"Symbolic","pretty":"x - 1","canonical":"(add (rat -1 1) (sym x))",...}},{"name":"changed","value":{"kind":"Boolean","value":"true"}},{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Symbolic","pretty":"x - 1 != 0","canonical":"(ne (add (rat -1 1) (sym x)) (rat 0 1))",...}]}}]}},...}

Run as: 'x = symbol("x"); <probe>' | dotnet run --project Lovelace.Run -c Release --no-build -- --stdin
(--eval with an embedded "x" is mangled by Windows PowerShell 5.1 native argument quoting; --stdin is exact.)
```

Reading: `cancel(x/x) = 1`, `cancel((x^2-1)/(x-1)) = x + 1`, **`cancel((x-1)^2/(x-1)) = x - 1`
(previously the input came back unchanged — N16b fixed)**, and `cancel_full` returns the structured
record with `status: CancelStatus.Conditional` and the structured condition `x - 1 != 0` (canonical
`(ne (add (rat -1 1) (sym x)) (rat 0 1))`), never a string.

## 6. Left undone, and why

1. **Bare `cancel` still returns a bare value with no inline condition.** That is the decision, not
   an omission: the value cannot carry one. The mitigation is discoverability — the descriptor now
   names the condition loss and points at `cancel_full` — and the structured surface itself.
2. **`Polynomial.TryFromExpr` still refuses `Pow(sum, k)`.** Deliberate (see N16b reasoning); the
   boundary is still asserted, and cancel compensates above it. If a later round wants the parser
   widened for *all* callers, that is a separate, wider-blast-radius decision.
3. **`cancel_full` carries no `steps` trace** (unlike `simplify_full`). The gcd cancellation is not
   performed by a registered rewrite rule, so there are no stable rule ids to report; inventing ids
   here would be a second provenance convention. `conditions` + `status` + `changed` are the
   contract for now.
4. **Historical docs still mention the old count (106)** in
   `docs/goal-cycle-3/round-28/implementation.md` — those are round-28 records of that round's state
   and docs outside this round's artifact are out of scope.
5. **`simplify(x/x)` still refuses in safe mode** while `cancel(x/x)` reduces: the asymmetry is
   intentional and pre-existing (safe simplify refuses conditional rewrites; `cancel` is the
   explicit request to cancel, now with an honest structured report). The rewrite path was out of
   scope and is unchanged.
