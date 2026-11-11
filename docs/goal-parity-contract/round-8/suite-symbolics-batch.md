# Round 8 — `Lovelace.Suite.Tests` and `Lovelace.Symbolics.Tests`: a verified batch

Oracle commit `1cc28f255eca6029849f880949fc7be1d40ab8af` (the census and the C# runner are the
same pinned reference the previous rounds used). This round ports **532 census cases** and
refuses **89** explicitly. Coverage moved from 2,542/6,209 (40.94 %) to
**3,074/6,209 (49.51 %)**; `Lovelace.Suite.Tests` moved from 0/929 to **435/929 (46.82 %)** and
`Lovelace.Symbolics.Tests` from 0/1,226 to **97/1,226 (7.91 %)**.

All 532 are new `#[test]`s in one test-only module, `rust/lovelace-lang/src/suite_parity_batch.rs`
(532 tests, 3416 lines), declared with `#[cfg(test)] mod suite_parity_batch;` at the
end of `rust/lovelace-lang/src/lib.rs`. No production line was changed.

## 1. Mappings per project

| C# test project | Cases | Ported this round | Explicit refusals |
|---|---:|---:|---:|
| Lovelace.Suite.Tests | 929 | 435 | 83 |
| Lovelace.Symbolics.Tests | 1,226 | 97 | 6 |
| **total** | **2,155** | **532** | **89** |

### 1.1 Lovelace.Suite.Tests, by class

| C# class | Census cases touched | Ported | Explicit refusals |
|---|---:|---:|---:|
| `LanguageDocumentationTests` | 111 | 111 | 0 |
| `StructuredRealExactnessTests` | 126 | 110 | 16 |
| `DescriptorExampleTests` | 142 | 99 | 43 |
| `ArrayTests` | 28 | 20 | 1 |
| `InterpreterBinaryArithmeticTests` | 14 | 12 | 1 |
| `LenZeroDimensionTests` | 12 | 11 | 1 |
| `StatementTests` | 14 | 11 | 1 |
| `InterpreterComparisonTests` | 8 | 8 | 0 |
| `InterpreterBuiltinIsEvenIsOddTests` | 6 | 6 | 0 |
| `InterpolationPrintTests` | 5 | 5 | 0 |
| `TypedArrayPromotionTests` | 10 | 5 | 5 |
| `FunctionTests` | 7 | 4 | 0 |
| `InterpreterBuiltinSignTests` | 4 | 4 | 0 |
| `EmptyReductionTests` | 7 | 3 | 2 |
| `InterpreterBuiltinAbsTests` | 3 | 3 | 0 |
| `InterpreterLiteralTests` | 3 | 3 | 0 |
| `InterpreterUnaryTests` | 6 | 3 | 0 |
| `SliceTests` | 5 | 3 | 2 |
| `BroadcastTests` | 4 | 2 | 1 |
| `ExactnessPreservationTests2` | 6 | 2 | 1 |
| `ExactRationalPayloadTests2` | 3 | 2 | 1 |
| `InterpreterBuiltinDivRemTests` | 4 | 2 | 1 |
| `InterpreterBuiltinPiTests` | 5 | 2 | 1 |
| `InterpreterBuiltinSqrtTests` | 5 | 2 | 2 |
| `InterpreterBuiltinInvTests` | 2 | 1 | 0 |
| `InterpreterFactorialTests` | 4 | 1 | 2 |
| `InterpreterBuiltinSetprecisionTests` | 9 | 0 | 2 |
| **subtotal** | **553** | **435** | **83** |

### 1.2 Lovelace.Symbolics.Tests, by class

| C# class | Census cases touched | Ported | Explicit refusals |
|---|---:|---:|---:|
| `UsageDocumentationTests` | 103 | 97 | 6 |
| **subtotal** | **103** | **97** | **6** |

## 2. How every row was verified

The C# assertion is the contract, so the work started from the assertion, not from the Rust
output. Three pools, three (overlapping) verifications:

1. **Documentation suites** (111 + 97 rows). `LanguageDocumentationTests.DocumentedExample_MatchesEngine`
   and `UsageDocumentationTests.DocumentedExample_MatchesEngine` parse a `lovelace`/`result` pair out
   of the guide and assert `ValueFormatter.FormatTyped(value) == expected` (or the exact error message,
   print text, or plot title). The Rust test asserts the same text for the same script. Each census
   row was tied to its guide block by the `(script, expected)` pair in the census display name, and the
   pair was cross-checked against `parity/corpus/lang.json` / `symbolics.json`. All 111 language rows
   and 97 of 103 symbolics rows are mapped. The 2 corpus failures are accounted for: `sym/076` is
   refused with its observed canonical shape, and `sym/071` is one of the five rows the census cannot
   identify (below), so it was not guessed.
2. **Descriptor examples** (99 rows). `DescriptorExampleTests.Example_Executes` only asserts that the
   snippet *executes* — a kind-only contract. The port therefore asserts the stronger statement the
   reference actually produces: the exact `FormatTyped` rendering of the same snippet, taken from the
   `Lovelace.Run` oracle at the pinned commit and required to equal the Rust rendering. A snippet whose
   two renderings differ is refused, and one that returns `Void` is refused as kind-only.
3. **Engine and structured-value tests** (Suite: `StructuredRealExactnessTests`, arrays, control flow,
   builtins, errors). Each C# input (`script`, `InlineData` row or `MemberData` row) was replayed through
   the C# oracle and required to reproduce the C# assertion before the Rust side was written; the Rust
   test then asserts the same concrete text, or the same field of the same structured projection the C#
   test reads (`kind`, `exact`, `numerator`, `denominator`, `shape`, element kinds).

Two pools were checked and **refused wholesale** rather than weakened:

- `RecursionDepthGuardTests` (6) and `RecursionBudgetCompositionTests` (33): the documented deepest
  accepted call `f(85)` kills the Rust runner with a stack overflow (exit `-1073741571`), while C#
  answers and refuses `f(86)` with the documented budget message. 39 cases stay deferred.
- `SignPrecedenceTests` (16) and the sign-sensitive symbolic rows: Rust parses `-2^2` as `4`, C# as `-4`.

AST-level facts (`ValueTests`, `StructuralEqualityTests`, `ParserTests`, `TokenTests`, `Tokenizer*`)
assert C# type shapes (`Assert.IsType<…>`, `ValueKind` alone) and are refused as kind-only; the
`Interpreter*` facts were ported through the equivalent source program (`abs(-5)`, `divrem(17, 5)`,
`-(-5)!`, …) and the value assertion is kept exactly.

## 3. Refusals (89)

| Pool | Cases | Refused | Dominant reason |
|---|---:|---:|---|
| Suite — descriptor examples | 142 | 43 | 24 need a builtin this crate does not have (Dsp, `latex`, `assume_*`, `acosh`/`asinh`/`atanh`, `cancel_full`, `capabilities`, `ndims`, `scale`, `step`, `impulse`, `delay`, `movingavg`, `powerseries`, `cosine`/`exponential`, `filter`/`noise`/`fft`), 17 render differently, 2 are `Void` |
| Suite — structured exactness | 126 | 16 | 10 certify `exact=true` where Rust says `false`; 4 use `pi`/`e`, which do not exist as constants; 1 needs `LReal64`; 1 truncation width |
| Suite — engine tests | 139 | 24 | see the table below |
| Symbolics — usage guide | 103 | 6 | 1 canonical `linsolve` shape, 5 census display names truncated |

### 3.1 Suite engine tests refused (24)

| Census case | Reason |
|---|---|
| `Squeeze_RemovesSingletonDims#0` | squeeze([[[1, 2]]]) renders "[1, 2] (Array)" in Rust but "[1, 2] (Vector)" in C#: the squeezed 1x1x2 array keeps Array rank in this crate. |
| `Evaluate_GivenSteppedRange_ProducesProgression#0` | 1..2..7 element [3] is Natural(7) in Rust but Integer(7) in C#: the stepped range does not widen its elements. |
| `VectorSlice_WithStep#0` | [0, 1, 2, 3, 4][::2] is refused by the Rust parser (slice syntax not supported); C# yields [0, 2, 4] (Vector). |
| `SliceView_FeedsElementwise#0` | [1, 2, 3, 4, 5][::2] * 2 is refused by the Rust parser (slice syntax not supported); C# yields [2, 6, 10] (Vector). |
| `ColumnVector_BroadcastsToMatrix#0` | [[1], [2]] + [[10, 20], [30, 40]] is refused ("Operands could not be broadcast together with shapes [2, 1] and [2, 2]"); C# broadcasts the column to [[11, 21], [32, 42]]. |
| `Sum_Empty_ReturnsZero#0` | sum([]) is refused in Rust ("Reduction needs at least one element."); C# returns 0 (Natural) (D6 empty reduction). |
| `Prod_Empty_ReturnsOne#0` | prod([]) is refused in Rust ("Reduction needs at least one element."); C# returns 1 (Natural). |
| `Len_IsTheFirstDimensionOfTheValue#10` | len(1..0) is 2 (Natural) in Rust (the range materialises [1, 0]) but 0 in C# (empty range). |
| `MixedNumericLiteral_PromotesToMaxKind#0` | [1, 2.5, 3] keeps Natural elements in Rust; C# promotes every element to Real (D3). |
| `NestedMixedLiteral_PromotesHomogeneously#0` | [[1, 2.5], [3, 4]] keeps Natural elements in Rust; C# promotes the whole array to Real. |
| `NaturalDivision_Exact_NarrowsBackToNatural#0` | [4, 6] / [2, 2] yields Real elements in Rust; C# narrows exact quotients back to Natural (D2). |
| `RealDivision_ExactIntegerQuotient_NarrowsToNatural#0` | [2.5] / [0.25] yields Real 10 in Rust; C# narrows the exact quotient to Natural(10). |
| `IntegerDivision_ExactNonNegative_NarrowsToNatural#0` | [-4, -6] / [-2, -2] yields Real elements in Rust; C# narrows to Natural. |
| `Substitution_PreservesTheExactRational#0` | subs(1/3, x, 2) is refused in Rust ("subs needs an expression."); C# returns a Symbolic with canonical (rat 1 3). |
| `Approximation_CarriesNoRationalFormAndIsNotExact#0` | evalf(1/3, 50) publishes exact=true with numerator/denominator in Rust; C# publishes exact=false and no rational form. |
| `Evaluate_GivenSqrtWithNoArguments_ThrowsBuiltinArityException#0` | the arity refusal text differs: Rust "'sqrt' expects 1 argument(s), but 0 were given." vs C# "sqrt(): expected 1 argument; got 0." |
| `Evaluate_GivenSqrtWithTooManyArguments_ThrowsBuiltinArityException#0` | arity refusal text differs: Rust "'sqrt' expects 1 argument(s), but 2 were given." vs C# "sqrt(): expected 1 argument; got 2." |
| `Evaluate_GivenIntegerFactorial_ReturnsInteger#0` | the Integer factorial path is refused in Rust ("Factorial needs a Natural"); C# returns Integer(120). |
| `Evaluate_GivenNegativeIntegerFactorial_ThrowsInvalidOperation#0` | refusal text differs: Rust "Factorial needs a Natural, but got 'Integer'." vs C# "Factorial is not defined for negative integers." |
| `Evaluate_GivenDivRemOfIntegers_KeepsTheSameSignQuotientPositive#0` | divrem(-7, -3) gives remainder -1 in Rust but 1 in C#: the remainder sign rule differs. |
| `Evaluate_GivenDivide_ReturnsCorrectQuotient#0` | 100 / 5 is 20 (Real) in Rust but 20 (Natural) in C#: exact Natural division does not narrow back. |
| `Evaluate_GivenTooManyArguments_ThrowsBuiltinArityException#0` | arity refusal text differs: Rust "'setprecision' expects 1 argument(s), but 2 were given." vs C# "setprecision(): expected 1 argument; got 2." |
| `Evaluate_GivenACountWiderThanInt64_NamesTheCount#0` | refusal text differs: Rust does not name the wide count the way C# does ("123456789012345678901234567890"). |
| `Evaluate_GivenPiWithTooManyArguments_ThrowsBuiltinArityException#0` | arity refusal text differs: Rust "'pi' expects 1 argument(s), but 2 were given." vs C# "pi(): expected 0 to 1 arguments; got 2." |

### 3.2 Structured exactness refused (16)

| Census case | Reason |
|---|---|
| `ExactPayload_AlwaysCarriesTheExactQuotient#5` | the payload flags differ: C# exact=false with no rational form, Rust exact=true with 1/17. |
| `ExactPayload_AlwaysCarriesTheExactQuotient#6` | the payload flags differ: C# exact=false with no rational form, Rust exact=true with 22/7. |
| `FixedWidthTruncation_IsNotPublishedAsAnExactReal#0` | the fact exercises LReal64 (fixed-width tier) directly; this crate has no LReal64 type. |
| `TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#0` | C# certifies "2^-5000" as exact with the full power-of-two/five denominator; Rust publishes exact=false. |
| `TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#1` | C# certifies "10^-1001" as exact with the full power-of-two/five denominator; Rust publishes exact=false. |
| `TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | C# certifies "2^-100000" as exact with the full power-of-two/five denominator; Rust publishes exact=false. |
| `TerminatingQuotient_IsAlwaysCertifiedExact#19` | C# certifies 1/158456325028528675187087900672000 exact at every budget; Rust publishes exact=false for the 2^100·5^3 denominator at this budget. |
| `TerminatingQuotient_IsAlwaysCertifiedExact#29` | C# certifies 1/158456325028528675187087900672000 exact at every budget; Rust publishes exact=false for the 2^100·5^3 denominator at this budget. |
| `TerminatingQuotient_IsAlwaysCertifiedExact#39` | C# certifies 1/158456325028528675187087900672000 exact at every budget; Rust publishes exact=false for the 2^100·5^3 denominator at this budget. |
| `TerminatingQuotient_IsAlwaysCertifiedExact#9` | C# certifies 1/158456325028528675187087900672000 exact at every budget; Rust publishes exact=false for the 2^100·5^3 denominator at this budget. |
| `TruncatedIrrational_IsNotCertifiedExact#0` | Rust produced no structured payload: Undefined variable 'pi'. |
| `TruncatedIrrational_IsNotCertifiedExact#1` | Rust produced no structured payload: Undefined variable 'e'. |
| `TruncatedIrrational_IsNotCertifiedExact#3` | Rust produced no structured payload: Undefined variable 'pi'. |
| `TruncatedIrrational_IsNotCertifiedExact#4` | Rust produced no structured payload: Undefined variable 'e'. |
| `TruncatedIrrational_IsNotCertifiedExact#6` | C# publishes exact=false with no rational form for "evalf(1/3, 50)"; Rust: exact=true with numerator 33333333333333333333… |
| `TruncatedQuotientAtTheBudget_IsNotCertifiedExact#0` | C# asserts value "0.000991080277502477" (18-place truncation at setprecision(18)); Rust renders "0.0009910802775024777006937561942517343904856293359762140733…" (display truncation width differs). |

### 3.3 Descriptor examples refused (43)

| Census case | Reason |
|---|---|
| `Corpus_ReachesCoreMetadataAndEveryPlugin#0` | the fact asserts the C# descriptor catalogue (corpus size, core/plugin membership), which is registry state of the C# engine rather than a language behaviour; there is no Rust equivalent to compare. |
| `Example_Executes#1` | Rust refuses the snippet: abs needs a number, but got 'Vector'. |
| `Example_Executes#10` | Rust renders "null" where C# renders "x < 10 and x > 0 (Symbolic)". |
| `Example_Executes#100` | the snippet returns Void; the C# contract only requires it to execute, which would be a weaker (kind-only) assertion. |
| `Example_Executes#107` | Rust refuses the snippet: Unknown function 'scale'. |
| `Example_Executes#108` | Rust refuses the snippet: Unknown function 'scale'. |
| `Example_Executes#11` | Rust refuses the snippet: Unknown function 'assume_clear'. |
| `Example_Executes#110` | the snippet returns Void; the C# contract only requires it to execute, which would be a weaker (kind-only) assertion. |
| `Example_Executes#116` | Rust renders "TransformResult(status: Satisfied, original: exp(log(x)), expression: exp(log(x)), changed: False, conditions: [], steps…" where C# renders "TransformResult(status: Satisfied, original: exp(log(x)), expression: x, changed: Tru… |
| `Example_Executes#12` | Rust refuses the snippet: Unknown function 'assume_integer'. |
| `Example_Executes#127` | Rust renders "1 (Array)" where C# renders "[1] (Vector)". |
| `Example_Executes#128` | Rust refuses the snippet: Unknown function 'step'. |
| `Example_Executes#13` | Rust refuses the snippet: Unknown function 'assume_negative'. |
| `Example_Executes#14` | Rust evaluates the snippet to Void (the assume_* builtin returns no value); C# returns the symbol expression x. |
| `Example_Executes#15` | Rust evaluates the snippet to Void (the assume_* builtin returns no value); C# returns the symbol expression x. |
| `Example_Executes#16` | Rust refuses the snippet: Unknown function 'assume_real'. |
| `Example_Executes#19` | Rust refuses the snippet: Unknown function 'atanh'. |
| `Example_Executes#21` | Rust refuses the snippet: Unknown function 'cancel_full'. |
| `Example_Executes#22` | Rust refuses the snippet: Unknown function 'cancel_full'. |
| `Example_Executes#24` | Rust renders "y + 1 + x*(y + 1) (Symbolic)" where C# renders "1 + y + x*(y + 1) (Symbolic)". |
| `Example_Executes#25` | Rust renders "CompilationResult(ir: #!mathir 2\nparam x\nconst (rat 1 1)\nconst (rat 2 1)\nConstant 0 0\nParameter 0 0\nConstant 0 1\nPowInt …" where C# renders "#!mathir 2\nparam x\nconst (rat 1 1)\nconst (rat 2 1)\nConstant 0 0\nParameter… |
| `Example_Executes#29` | Rust renders "-1i (Complex)" where C# renders "1i (Complex)". |
| `Example_Executes#3` | Rust refuses the snippet: Unknown function 'acosh'. |
| `Example_Executes#33` | Rust refuses the snippet: Unknown function 'cosine'. |
| `Example_Executes#35` | Rust refuses the snippet: Unknown function 'delay'. |
| `Example_Executes#42` | Rust refuses the snippet: Unknown function 'e'. |
| `Example_Executes#49` | Rust refuses the snippet: Unknown function 'exponential'. |
| `Example_Executes#53` | Rust refuses the snippet: Unknown function 'filter'. |
| `Example_Executes#57` | Rust refuses the snippet: Unknown function 'impulse'. |
| `Example_Executes#65` | Rust renders "MatrixInverseResult(status: Solved, complete: True, completeness: Complete, inverse: [[x/(x^2 - 1), -1/(x^2 - 1)], [-1/(…" where C# renders "MatrixInverseResult(status: Solved, complete: True, completeness: Complete, inverse: … |
| `Example_Executes#69` | Rust refuses the snippet: Unknown function 'latex'. |
| `Example_Executes#76` | Rust renders "[(5*x - 4)/(x^2 - 1), (4*x - 5)/(x^2 - 1)] (Vector)" where C# renders "[(5 - (4*x - 5)/(x^2 - 1))/x, (4*x - 5)/(x^2 - 1)] (Vector)". |
| `Example_Executes#77` | Rust renders "MatrixSolveResult(status: Solved, complete: True, completeness: Complete, solutions: [[(5*x - 4)/(x^2 - 1), (4*x - 5)/(x…" where C# renders "MatrixSolveResult(status: Solved, complete: True, completeness: Complete, solutions: … |
| `Example_Executes#8` | Rust refuses the snippet: Unknown function 'asinh'. |
| `Example_Executes#83` | Rust renders "2 (Real)" where C# renders "2 (Natural)". |
| `Example_Executes#85` | Rust refuses the snippet: Unknown function 'movingavg'. |
| `Example_Executes#86` | Rust refuses the snippet: Unknown function 'ndims'. |
| `Example_Executes#87` | Rust refuses the snippet: Unknown function 'noise'. |
| `Example_Executes#94` | Rust renders "x < -1 or x > 0 (Symbolic)" where C# renders "(x < -1 or x > 0) (Symbolic)". |
| `Example_Executes#95` | Rust renders "3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132823066…" where C# renders "3.14159265358979323846264338327950288419716939937510582097494459230781640628620899862… |
| `Example_Executes#97` | Rust renders "C:\\Users\\ricar\\dev\\LovelaceSharp\\rust\\target\\tmp\\lovelace-plots\\plot.svg" where C# renders "C:\\Users\\ricar\\dev\\LovelaceSharp\\plot.svg". |
| `Example_Executes#98` | Rust renders "C:\\Users\\ricar\\dev\\LovelaceSharp\\rust\\target\\tmp\\lovelace-plots\\plot.svg" where C# renders "C:\\Users\\ricar\\dev\\LovelaceSharp\\plot.svg". |
| `Example_Executes#99` | Rust refuses the snippet: Unknown function 'powerseries'. |

### 3.4 Symbolics usage guide refused (6)

| Census case | Reason |
|---|---|
| `DocumentedExample_MatchesEngine#61` | the census display name truncates this example's script and result (xUnit shortens very long arguments), so the row cannot be tied to a guide result block without guessing; left unmapped rather than guessed. |
| `DocumentedExample_MatchesEngine#70` | the census display name truncates this example's script and result (xUnit shortens very long arguments), so the row cannot be tied to a guide result block without guessing; left unmapped rather than guessed. |
| `DocumentedExample_MatchesEngine#71` | the census display name truncates this example's script and result (xUnit shortens very long arguments), so the row cannot be tied to a guide result block without guessing; left unmapped rather than guessed. |
| `DocumentedExample_MatchesEngine#73` | the census display name truncates this example's script and result (xUnit shortens very long arguments), so the row cannot be tied to a guide result block without guessing; left unmapped rather than guessed. |
| `DocumentedExample_MatchesEngine#75` | the C# test asserts the documented rendering [-x/x/(x*y), x/(x*y)] for linsolve([[x, 1], [0, y]], [0, 1]); Rust renders [-1/(x*y), x/(x*y)] (canonical shape differs; corpus sym/076). |
| `DocumentedExample_MatchesEngine#99` | the census display name truncates this example's script and result (xUnit shortens very long arguments), so the row cannot be tied to a guide result block without guessing; left unmapped rather than guessed. |

## 4. Divergences this batch found

These are observed Rust/C# differences. Some are refused above; the ones marked *(not asserted)*
are differences the C# test does not constrain, listed so they are not mistaken for parity.

| # | Behaviour | C# | Rust | Affects |
|---|---|---|---|---|
| 1 | unary minus vs `^` | `-2^2` = `-4 (Integer)` | `4 (Integer)` | `SignPrecedenceTests` (16), symbolic sign rows |
| 2 | deep recursion | `f(85)` answers, `f(86)` refused with the budget message | `f(85)` aborts the process (stack overflow) | `RecursionDepthGuardTests` + `RecursionBudgetCompositionTests` (39) |
| 3 | `pi` / `e` constants | defined (`pi`, `e`) | undefined; only `pi(n)` exists | `TruncatedIrrational` (4), descriptor examples |
| 4 | exact Natural division | `100 / 5` = `20 (Natural)` | `20 (Real)` | `InterpreterBinaryArithmeticTests` (1) |
| 5 | D3 mixed-literal promotion | `[1, 2.5, 3]` promotes all elements to Real | elements keep Natural | `TypedArrayPromotionTests` (2) |
| 6 | D2 division narrowing | `[4, 6] / [2, 2]` narrows to Natural | stays Real | `TypedArrayPromotionTests` (3) |
| 7 | empty reductions (D6) | `sum([])` = 0, `prod([])` = 1 | refused “Reduction needs at least one element.” | `EmptyReductionTests` (2) |
| 8 | column broadcasting | `[[1],[2]] + [[10,20],[30,40]]` works | refused, shapes `[2, 1]` vs `[2, 2]` | `BroadcastTests` (1) |
| 9 | slice syntax | `[0,1,2,3,4][::2]` = `[0, 2, 4]` | parse error | `SliceTests` (2) |
| 10 | empty range | `len(1..0)` = 0 | 2 (materialises `[1, 0]`) | `LenZeroDimensionTests` (1) |
| 11 | stepped-range element kind | `1..2..7[3]` = `Integer(7)` | `Natural(7)` | `StatementTests` (1) |
| 12 | `squeeze` rank | `squeeze([[[1, 2]]])` = `[1, 2] (Vector)` | `[1, 2] (Array)` | `ArrayTests` (1) |
| 13 | `divrem` remainder sign | `divrem(-7, -3)` = q 2, r 1 | q 2, r **-1** | `InterpreterBuiltinDivRemTests` (1) |
| 14 | arity-refusal wording | `sqrt(): expected 1 argument; got 0.` | `'sqrt' expects 1 argument(s), but 0 were given.` | 5 rows |
| 15 | `setprecision` on non-terminating quotients | display truncated to the budget | renders the full expansion | `StructuredRealExactnessTests` value row; `evalf` exact flag *(not asserted)* |
| 16 | exactness of huge terminating quotients | `2^-5000` and `1/(2^100·5^3)` certified exact | `exact=false` | 7 structured rows |
| 17 | `linsolve` canonical shape | `[-x/x/(x*y), x/(x*y)]` | `[-1/(x*y), x/(x*y)]` | corpus `sym/076`, 1 doc row |
| 18 | complex rendering | `-1i` | `0 - 1i` | Dsp descriptor examples *(not asserted)* |
| 19 | `mean([1, 2, 3])` kind | `2 (Natural)` | `2 (Real)` | `ArrayTests` fact `Mean_Exact` (refused whole) |
| 20 | `subs` with a numeric first argument | returns a Symbolic `(rat 1 3)` | “subs needs an expression.” | `ExactnessPreservationTests2` (1) |
| 21 | `plot()` result | `<cwd>\plot.svg` | `<tmp>\lovelace-plots\plot.svg` | 2 descriptor examples |

## 5. Observed verification

```text
$ cargo test --manifest-path rust/Cargo.toml --workspace      # exit 0
   58 + 67 + 125 + 7 + 540 + 134 + 2308 + 27 = 3266 passed; 0 failed
   (lovelace-lang: 540 = 8 pre-existing + 532 new)

$ powershell -File parity/gen-rust-tests.ps1                    # exit 0
   crates=11 tests=3266 suiteExit=0 passed=3266 failed=0

$ node parity/gen-manifest.mjs                                  # exit 0
   { "rows": 6209, "dispositions": { "deferred": 3135, "ported": 3074 },
     "coveragePercent": 49.51, "rustTests": 3266 }

$ node parity/tools/gate-manifest.mjs                            # exit 0
   { "censusCases": 6209, "manifestRows": 6209,
     "dispositions": { "ported": 3074, "consolidated": 0, "excluded": 0, "deferred": 3135 },
     "coveragePercent": 49.51, "claimedRustTests": 3074, "projectsInCensus": 15, "failures": [] }
   PASS (structural): 6209 test cases dispositioned; 3074 claim Rust coverage (49.51%).

$ node parity/tools/coverage-report.mjs                          # exit 0
   coverage.md written: 3074/6209 ported (49.51%)

$ rust/target/debug/lovelace-corpus.exe                          # unchanged
   corpus: 212 passed, 2 failed, 214 total                      (sym/071, sym/076)

$ powershell -File parity/gate-fixtures.ps1                       # exit 0
   oracle: 20/20, rust: 20/20, PASS: oracle 20/20, rust 20/20
```

The coverage table after this round:

| C# test project | Cases | Ported | Deferred | Coverage |
|---|---:|---:|---:|---:|
| Lovelace.Suite.Tests | 929 | 435 | 494 | 46.82 % |
| Lovelace.Symbolics.Tests | 1,226 | 97 | 1,129 | 7.91 % |
| **TOTAL** | **6,209** | **3,074** | **3,135** | **49.51 %** |

## 6. What could not be verified

- **The C# tests were not executed.** `parity/README.md` records that the VSTest testhost cannot run
  in this environment (`OpenProcess` denied), so a C# assertion is verified by reading it in source and
  by replaying its input through the pinned `Lovelace.Run` oracle, which is the project’s documented
  behavioural oracle. Every mapped row’s input was replayed that way; no expected value was invented
  from the Rust output (except the two documented cases below).
- **Descriptor examples are a stronger contract.** The C# test only requires execution; the port pins
  the exact rendering. If that strengthening is not acceptable, the 99 rows should be re-dispositioned.
- **Five symbolics-guide census rows could not be tied to a guide block**: xUnit truncates long
  arguments in the display name (`…`), so the script/result pair is incomplete in the census. They are
  left deferred rather than guessed.
- **A stale `lovelace-run` binary** in `rust/target` was found before the comparisons and rebuilt
  (`cargo build -p lovelace-run -p lovelace-corpus`); without that, 10 descriptor rows that now map
  would have been refused for the wrong reason.
- **Not attempted** (outside this batch): the remaining 494 Suite cases (help service, plot files,
  cancellation, timing, records, plugins, tokenizer/parser type shapes, depth guards) and 1,129
  Symbolics cases (pretty-printer properties, LaTeX printer, falsification gates, solve records). The
  printers and most record shapes need APIs this crate does not expose.
