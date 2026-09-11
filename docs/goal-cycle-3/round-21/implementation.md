# Round 21 — exposing asinh/acosh/atanh, and negative integer exponents

Status: CHANGE COMPLETE; full-suite verification observed (see "VERIFY"). This file was written
before the change and updated as evidence was observed (Step A before, Step C after).

## What was wrong (measured, not assumed)

* `asinh`/`acosh`/`atanh` are registered **kernel** functions — `Lovelace.Symbolics/Functions.cs:93/95/97`
  (derivative templates plus evaluators `NumOps.Asinh/Acosh/Atanh`), `Evaluation.cs:341-345`, domain
  rule `Assumptions.cs:723-725` — but they were **not exported as builtins**:
  `SymbolicsPlugin.ElementaryFunctions` listed 11 names and `CoreBuiltinMetadata` has no hyperbolic
  inverse entry. The mathematics existed; the language could not reach it: `atanh(0.5)` →
  `Unknown function 'atanh'.` **Exposure gap, not missing mathematics.**
* `2^-1` → `Exponent must be positive. (Parameter 'exponent')` from `Lovelace.Integer/Integer.cs:378`
  via `Lovelace.Suite/NumericOps.cs:56` (integer arm of `BinaryOp.Power`). The kernel's symbolic
  numeric layer already did the right thing (`NumOps.Pow` → `PowInt` → `1/x^n`); only the language
  numeric path refused.
* `0^-1` → `Base cannot be zero. (Parameter 'exponent')`: the value out of range is the BASE, while
  the reported parameter name was the exponent.

## Step A — tests first (observed FAILING output, BEFORE the change)

New test file `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs`
(14 test cases). Full log: `docs/goal-cycle-3/round-21/step-a-tests.txt`.

```
  Failed ...HyperbolicBuiltins_AreRegistered_AndFoldAtTheirDefiningPoint(name: "asinh", call: "asinh(0)")
  Error Message:
   asinh is not a known function
  Failed ...HyperbolicBuiltins_AreRegistered_AndFoldAtTheirDefiningPoint(name: "acosh", call: "acosh(1)")
  Error Message:
   acosh is not a known function
  Failed ...HyperbolicBuiltins_HaveCompleteDescriptors(name: "atanh", inverse: "tanh", example: "atanh(0)")
  Error Message:
   Assert.NotNull() Failure: Value is null
  Failed ...HyperbolicBuiltins_KeepTheirSymbolicApplicationForm
  Failed ...TwoToTheMinusOne_IsTheExactValueOfOneHalf
  Error Message:
   System.ArgumentOutOfRangeException : Exponent must be positive. (Parameter 'exponent')
     at Lovelace.Integer.Integer.Pow(Integer exponent) in ...\Lovelace.Integer\Integer.cs:line 378
     at Lovelace.Suite.NumericOps.Apply(BinaryOp op, Value left, Value right) in ...\Lovelace.Suite\NumericOps.cs:line 56
  Failed ...ThreeToTheMinusTwo_IsTheExactValueOfOneNinth
  Error Message:
   System.ArgumentOutOfRangeException : Exponent must be positive. (Parameter 'exponent')
  Failed ...ZeroToTheMinusOne_IsATypedRecoverableErrorThatNamesTheBase
  Error Message:
   Assert.Equal() Failure: Strings differ
Expected: "base"
Actual:   "exponent"
  Failed ...BuiltinCount_IncludesTheThreeHyperbolicFunctions
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 104
Actual:   101
  Failed ...PositiveAndExactIntegerPowers_AreUnchanged
  Error Message:
   System.ArgumentOutOfRangeException : Exponent must be positive. (Parameter 'exponent')

Failed!  - Failed:    12, Passed:     2, Skipped:     0, Total:    14, Duration: 78 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Baseline probes (stale `bin/Release/net10.0/publish` binary, i.e. pre-change) reproduced all four
symptoms:

```
PROBE[2^-1]        => {"ok":false,"message":"Exponent must be positive. (Parameter 'exponent')"}
PROBE[3^-2]        => {"ok":false,"message":"Exponent must be positive. (Parameter 'exponent')"}
PROBE[asinh(0)]    => {"ok":false,"message":"Unknown function 'asinh'."}
PROBE[acosh(1)]    => {"ok":false,"message":"Unknown function 'acosh'."}
PROBE[atanh(0)]    => {"ok":false,"message":"Unknown function 'atanh'."}
PROBE[0^-1]        => {"ok":false,"message":"Base cannot be zero. (Parameter 'exponent')"}
PROBE[sqrt(-1)]    => {"ok":false,"message":"Square root is not defined for negative numbers."}
PROBE[(-1)^(1/2)]  => {"ok":false,"message":"Non-integer exponents are not yet supported."}
```

Two Step-A corrections are recorded honestly rather than hidden:

1. My first draft of the `sqrt(-1)`/`(-1)^(1/2)` assertions guessed `InvalidOperationException`; the
   observed types are `ArithmeticException` (`Lovelace.Real/Real.cs:818`) and
   `NotImplementedException` (`Lovelace.Real/Real.cs:730`), so the assertions name the types actually
   thrown. The **messages** are unchanged in either direction.
2. The builtin-count assertion was drafted as 120 on the parent's figure. Two configurations exist
   and they differ — see "Builtin count" — so the assertion pins the configuration the existing
   completeness test walks (104).

## Step B — the change (3 source files)

1. `Lovelace.Symbolics/SymbolicsPlugin.cs`
   * `ElementaryFunctions` gains `asinh`, `acosh`, `atanh`. That one list both **registers** the
     three builtins and gives them descriptors, so the existing metadata-completeness contract
     covers them with no second catalogue.
   * `ElementarySummary` gains the three summaries (principal-branch domains spelled out).
   * `ElementaryRelated(fn)` is new: every hyperbolic inverse names the hyperbolic it inverts, and
     each hyperbolic names its inverse, so help's "See also" reaches across the family from either
     end (the elementary descriptors previously pointed only at simplify/diff/integrate).
   * `ElementaryExample(fn)` is new: the three ship the exact doctests `asinh(0)` / `acosh(1)` /
     `atanh(0)` instead of the generic `fn(x)` placeholder.

2. `Lovelace.Suite/NumericOps.cs` — the numeric evaluation path for integer powers. The
   `(BinaryOp.Power, ValueKind.Integer)` arm now calls a new `IntegerPower(baseValue, exponent)`
   helper. A negative exponent is the **reciprocal power**: `x^-n` is `1 / x^n` computed in exact
   Real arithmetic through the very same `Rl.Divide` the literal `1/2` uses, so `2^-1` and `1/2`
   are the same value, not merely similar-looking. A zero base is **delegated to `Int.Pow`**, so
   there is exactly one spelling of the `0^-n` failure in the codebase.

3. `Lovelace.Integer/Integer.cs:376` — the base-zero guard now throws
   `ArgumentOutOfRangeException("base", "Base cannot be zero.")` instead of naming `exponent`, so the
   message names the quantity actually out of range: `Base cannot be zero. (Parameter 'base')`. The
   exponent guard keeps `nameof(exponent)`, which is correct there.

## Step C — the required assertions and their observed values

```
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~HyperbolicBuiltinsAndNegativeIntegerPowersTests"
Passed!  - Failed:     0, Passed:    14, Skipped:     0, Total:    14, Duration: 77 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

* `asinh(0)`, `acosh(1)`, `atanh(0)` each evaluate to the **Symbolic `0`** (kind `Symbolic`, rendered
  `0`) — the same folding `sin(0)` already performed. All three are registered builtins and all
  three have complete descriptors (summary, signature `asinh(x)`, a doctest example, a declared
  return kind, and cross-references that resolve both ways).
* `2^-1` is value-equal to the literal `1/2`: kind `Real`, `value.AsReal() == half.AsReal()`,
  rendered `0.5`, and the **structured projection is the exact rational 1/2**
  (`Exact == true`, `Numerator == "1"`, `Denominator == "2"`, identical to `1/2`'s).
* `3^-2` is value-equal to the literal `1/9`: rendered `0.(1)`, structured `Exact == true`,
  `Numerator == "1"`, `Denominator == "9"`.
* `x^-1` with `x = symbol("x")` returns the **symbolic reciprocal power** `x^-1` (kind `Symbolic`,
  rendered `x^-1`). It is not rewritten to `1/x` and it does not throw.
* `0^-1` throws the typed `ArgumentOutOfRangeException` with `ParamName == "base"` and message
  `Base cannot be zero. (Parameter 'base')` (asserted NOT to contain "exponent"); it is recorded in
  that evaluation's diagnostics; and the session survives it (`1 + 1` → `2` afterwards). The runner
  reports it as `code: InvalidArgument, category: TypeMismatch, recoverable: true`.
* `sqrt(-1)` and `(-1)^(1/2)` keep their typed errors **unchanged** (types and messages asserted).

## VERIFY (observed output)

Command 1 — `dotnet build LovelaceSharp.slnx -c Release --nologo`:

```
Build succeeded.
C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite.Tests\EmptyReductionTests.cs(47,9): warning xUnit2013: ...
    1 Warning(s)
BUILD EXIT: 0
```
(The single warning is pre-existing and unrelated — `EmptyReductionTests.cs`, untouched here.)

Command 2 — `dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo`:

```
Passed!  - Failed:     0, Passed:   449, Skipped:     6, Total:   455, Duration: 14 s - Lovelace.Symbolics.Tests.dll (net10.0)
SYMBOLICS TEST EXIT: 0
```

Command 3 — `dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo`:

```
Passed!  - Failed:     0, Passed:   636, Skipped:     0, Total:   636, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
SUITE TEST EXIT: 0
```

Full logs: `verify.txt` (state the assertions were strengthened from) and `verify-rerun.txt`
(identical commands re-run after the test-only strengthening edit). The re-run observed the same
numbers — `Build succeeded` / `BUILD EXIT: 0`, `Failed: 0, Passed: 449, Skipped: 6` /
`SYMBOLICS TEST EXIT: 0`, `Failed: 0, Passed: 636` / `SUITE TEST EXIT: 0` — so **0 failed in
every suite** both before and after the strengthening edit.

Command 4 — the runner on the freshly built JIT binary
(`Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe`, i.e. the code under test; the checked-in
`...\publish\Lovelace.Run.exe` copy is STALE and was only used for the pre-change baseline):

```
PROBE[2^-1]       => {"ok":true,"result":{"kind":"Real","display":"0.5","typed":"0.5 (Real)","structured":{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}}}
PROBE[3^-2]       => {"ok":true,"result":{"kind":"Real","display":"0.(1)","typed":"0.(1) (Real)","structured":{"kind":"Real","value":"0.(1)","exact":true,"numerator":"1","denominator":"9"}}}
PROBE[asinh(0)]   => {"ok":true,"result":{"kind":"Symbolic","display":"0","typed":"0 (Symbolic)","structured":{"kind":"Symbolic","pretty":"0","canonical":"(rat 0 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}}}
PROBE[acosh(1)]   => {"ok":true,"result":{"kind":"Symbolic","display":"0","...":"(identical shape)"}}
PROBE[atanh(0)]   => {"ok":true,"result":{"kind":"Symbolic","display":"0","...":"(identical shape)"}}
PROBE[0^-1]       => {"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Base cannot be zero. (Parameter 'base')","recoverable":true,"diagnostics":[{"message":"Base cannot be zero. (Parameter 'base')","position":0,"line":1,"column":1}]}
PROBE[sqrt(-1)]   => {"ok":false,"code":"ArithmeticError","category":"DomainError","message":"Square root is not defined for negative numbers.","recoverable":true}
PROBE[(-1)^(1/2)] => {"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true}
PROBE[atanh(0.5)] => {"ok":true,"result":{"kind":"Symbolic","display":"atanh(1/2)","canonical":"(fn atanh (rat 1 2))","domain":"real"}}
PROBE[2^3]        => {"ok":true,"result":{"kind":"Natural","display":"8","exact":true}}
PROBE[1/2]        => {"ok":true,"result":{"kind":"Real","display":"0.5","exact":true,"numerator":"1","denominator":"2"}}
```
Verbatim (unabridged) probe JSON: `probes.txt`.

## Builtin count

Two configurations, both measured:

| configuration | before | after |
| --- | --- | --- |
| fresh JIT `Lovelace.Run` (the runner probes above) | 117 (parent's figure; see note) | **120** (measured: `functions total: 120, builtin true: 120`) |
| the engine `HelpMetadataCompletenessTests` builds (`SuiteEngine` + `DspPlugin` + `SymbolicsPlugin` + `MathIRPlugin`), counted as `CaptureState().Functions.Values.Where(f => f.IsBuiltin)` | **101** (measured, Step A) | **104** (asserted by `BuiltinCount_IncludesTheThreeHyperbolicFunctions`) |

Both are +3, and the +3 is exactly `asinh`/`acosh`/`atanh`. The existing completeness test walks the
second configuration, so 101 → 104 is the count that test covers, and all three new names carry
plugin descriptors — no "(no summary registered)" hole is possible. The 120 figure matches the
parent's 117 → 120 exactly; the 117 itself was not re-measured after the change (reverting to
re-measure was not worth the time-box), it follows from 120 − 3 and from the parent's own
observation. Note also that `...\publish\Lovelace.Run.exe` is stale (102 builtins) and is NOT the
baseline for either figure.

## NOT done this round (explicitly out of scope)

* Complex square roots of negative reals: `sqrt(-1)` still fails with its typed, unchanged error
  (`ArithmeticError` / "Square root is not defined for negative numbers.").
* Rational/fractional powers of negative bases: `(-1)^(1/2)` still fails with its typed, unchanged
  error (`UnsupportedOperation` / "Non-integer exponents are not yet supported.").
  Both remain explicit unsupported operations — never a silently wrong answer.
* `capabilities()` (a later round) and everything else outside the files below.
* `CoreBuiltinMetadata` was NOT touched: a plugin-registered descriptor wins the help lookup, so the
  three functions need no core-table entry (and that table is out of this round's scope).
* No complex-number or rational `ValueKind` was introduced: `2^-1` yields the language's existing
  exact `Real` (structured as an exact rational), which is what `1/2` already yields.

## Files changed

* `Lovelace.Symbolics/SymbolicsPlugin.cs`
* `Lovelace.Suite/NumericOps.cs`
* `Lovelace.Integer/Integer.cs`
* `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs` (new)
* `docs/goal-cycle-3/round-21/{implementation.md, step-a-tests.txt, verify.txt, verify-rerun.txt, probes.txt, probe.ls}`

No test was deleted, skipped or weakened; no expected value was changed to match old output; no git
commit was made.
