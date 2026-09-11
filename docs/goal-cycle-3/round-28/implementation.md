# Round 28 — item 13: `PrintMode.Latex`, derived from the ONE expression model

Outcome: **done**. `PrintMode.Latex` is a new arm of the EXISTING printer (not a second printer),
reachable from the language as `latex(expr)`, with a real `BuiltinDescriptor`. All suites green,
Run.Tests goldens unchanged, no golden regenerated, nothing outside SCOPE edited.

## 1. Files touched (SCOPE only)

| File | Change |
| --- | --- |
| `Lovelace.Symbolics/Printing.cs` | `PrintMode.Latex` + the Latex arm + the shared precedence/structure helpers factored out of the Pretty arm |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | the `latex` builtin + `BuiltinDescriptor` |
| `Lovelace.Symbolics.Tests/LatexPrinterTests.cs` | new: parity, load-bearing delimiters, known renderings, mode-coherence |
| `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs` | the pinned builtin count, 105 -> 106 (tripwire, not a weakened assertion) |
| `docs/goal-cycle-3/round-28/implementation.md` | this file |

## 2. Step A — the assertions first, observed failing output

Step A was: add the `PrintMode.Latex` enum member with **no renderer** behind it (it fell through
the dispatch switch to the Pretty arm), write the tests, run. Observed:

```
  Failed ...KnownRenderings_AreExact(source: "x^2", expected: "x^{2}") [< 1 ms]
   Assert.Equal() Failure: Strings differ
           ↓ (pos 0)
Expected: "x^{2}"
Actual:   "x^2"
  Failed ...KnownRenderings_AreExact(source: "sqrt(x)", expected: "\sqrt{x}") [< 1 ms]
Expected: "\sqrt{x}"
Actual:   "sqrt(x)"
  Failed ...KnownRenderings_AreExact(source: "(x + 1)^2", expected: "\left(x + 1\right)^{2}")
Expected: "\left(x + 1\right)^{2}"
Actual:   "(x + 1)^2"
  Failed ...KnownRenderings_AreExact(source: "x - (y - a)", expected: "x - \left(y - a\right)")
Expected: "x - \left(y - a\right)"
Actual:   "x - (y - a)"
  Failed ...KnownRenderings_AreExact(source: "1/2", expected: "\frac{1}{2}")
   System.InvalidCastException : Unable to cast object of type 'Lovelace.Real.Real' to type 'Lovelace.Symbolics.Expr'.

Failed!  - Failed:    12, Passed:   102, Skipped:     0, Total:   114, Duration: 107 ms
```

**What passed immediately, stated plainly:** the two structural tests
(`GroupingParity_...`, `Delimiters_AreLoadBearing`) passed in Step A *vacuously* — with no renderer
the LaTeX text was the Pretty text, so both trivially agreed. That is a false green, so a
non-vacuity guard (`LatexArm_IsNotThePrettyArm`: the precedence-sensitive corpus must render
differently from Pretty on all but at most two rows) was added and **did** fail while the
implementation was incomplete:

```
  Failed ...LatexArm_IsNotThePrettyArm [4 ms]
   the LaTeX rendering is the pretty rendering for 4 corpus rows:
   x - (y - a), x - y - a, a - (x + y), -(x + y)
```

The one `InvalidCastException` is a corpus fact, not a printer defect: a bare `1/2` is a numeric
value at the language level, so that row was moved to a direct-model assertion
(`KnownRendering_RationalConstant_IsAFraction` on `Exprs.Rational`).

Then the load-bearing test caught a real defect in my first LaTeX design (decorative parentheses
inside `\frac` groups), which is exactly what it exists for:

```
  Failed ...Delimiters_AreLoadBearing(source: "x/(y + 1)") [< 1 ms]
   "x/(y + 1)" -> \frac{x}{\left(y + 1\right)} has delimiters that change nothing: \left(y + 1\right)
  Failed ...Delimiters_AreLoadBearing(source: "log(x + 1)/(x - 1)") [< 1 ms]
   "log(x + 1)/(x - 1)" -> \frac{\log(x + 1)}{\left(x - 1\right)} has delimiters that change nothing: \left(x - 1\right)
Failed!  - Failed:     4, Passed:   111, Skipped:     0, Total:   115
```

Fix: a side of `\frac` that is a single part is rendered at `RootPrec` (the brace group IS its
delimiter); a side of several parts is a text-level `\cdot` product, so its sum factors keep the
delimiters `Delimit` asks for.

## 3. What was REUSED versus DUPLICATED

`PrintMode` is now exactly `{Canonical, Pretty, Debug, Latex}`. The Latex arm makes **every**
precedence and structural decision through the same code the Pretty arm calls. Reused (one place):

| Shared thing | Role |
| --- | --- |
| `Prec(Expr)` + the `OrPrec/AndPrec/RelationPrec/AddPrec/MulPrec/PowPrec/AtomPrec` constants | THE precedence table. `Prec` now returns those constants and no other precedence number exists. |
| `NeedsDelimiter(parentPrec, nodePrec)` | THE parenthesis decision (formerly `parentPrec > N` inline in eight arms of Pretty). `Delimit` (ASCII `(...)`) and `LatexDelimit` (`\left(...\right)`) are the two one-line notations of that one decision. |
| `ShapeProduct(MultiplyExpr)` | THE product decomposition: signed rational coefficient, numerator factors, and which factors are denominators (`^ -1`). Extracted from Pretty's multiply arm, now called by both. |
| `IsNegativeTerm`, `IsNegativeConstant` | THE sum sign decisions (which term renders as `- magnitude`). |
| `SubtrahendPrec` | THE precedence demanded of a subtracted magnitude — `x - (y - a)`, both notations. |
| `OrderTermsForDisplay`, `CompareSeriesTerms`, `TryMonomial`, `DegreeOf` | THE display term order — both renderings list terms identically. |
| `OrderShape(OrderExpr)` | THE O-term decomposition (zero point / degree one), extracted from Pretty. |
| `NeedsPowerBaseDelimiter`, `OneHalf`, `MinusOneHalf`, `ArgPrec`, `RootPrec` | THE power-base rule, the radical decision (`x^(1/2) -> \sqrt{}`, `x^(-1/2) -> 1/\sqrt{}`), the call-argument precedence. |

**Duplicated precedence rules: none.** The refactor replaced Pretty's inline `parentPrec > 1/2/3/4/-1/0`
comparisons with `Delimit(...)`/`NeedsDelimiter(...)` and its inline `Prec(p.Base) <= 3` with
`NeedsPowerBaseDelimiter` — Pretty's behaviour is byte-identical (proved by the untouched
Pretty/canonical tests, the golden envelopes and `PrettyParenthesesTests`).

**Notation-specific, deliberately not shared:** LaTeX's own grouping constructs — the braces of
`\frac{A}{B}`, `\sqrt{A}` and `x^{A}` — already delimit their child, so those children are
rendered at `RootPrec` and no extra delimiter is emitted. Pretty has no such construct, so its
power-exponent parenthesisation (a flat-infix necessity) is Pretty-only. This is not a second
precedence table: it is a strictly *weaker* delimiter demand in three positions, and the tests
prove both halves — the nesting is still identical, and every delimiter that IS emitted is
load-bearing.

## 4. Grouping parity: method and result

LaTeX cannot be re-parsed into the kernel, so the acceptance property is structural, measured on
the rendered text in two independent ways, over a 34-row precedence-sensitive corpus (nested
subtraction, powers of sums, powers of negatives, products of sums, nested fractions, negative
exponents, unary minus, function application, abs, radicals). The corpus is built **through the
language** (`SuiteEngine` + `SymbolicsPlugin`), not through the kernel API.

1. **Grouping parity** (`GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting`). Both
   renderings are denoised into a structural tree by a recursive-descent denoiser written for the
   test. The LaTeX denoiser resolves LaTeX's OWN grouping — `{...}` groups, `\frac{A}{B}` as a
   quotient of its two groups, `\sqrt{A}` and `\left|A\right|` as the calls they denote,
   `x^{A}` as a superscript group, `\left(\right)` as parentheses — and the infix denoiser resolves
   Pretty's parentheses; `*` vs `\cdot`, `sqrt(x)` vs `\sqrt{x}`, `1/2` vs `\frac{1}{2}` are
   normalised to one vocabulary. The trees (fully parenthesised shape strings, parentheses
   transparent because they only affect grouping) are compared for equality. **A delimiter dropped
   in either rendering changes its tree**, so this is not a substring check: no test here asserts
   that a string "contains `\frac`". Result: 34/34 rows equal.
2. **Delimiters are load-bearing** (`Delimiters_AreLoadBearing`). For every `\left(...\right)` /
   `\left|...\right|` pair in the LaTeX text, the pair is removed and the text re-parsed: if the
   tree is unchanged the delimiter was decoration and the test fails. This is the "exactly where
   the Pretty rendering needs parentheses" half, measured without an oracle. Result: 34/34 rows
   have zero decorative delimiters (it failed on 4 rows before the `\frac`-group fix above).
3. **Non-vacuity** (`LatexArm_IsNotThePrettyArm`): at most 2 corpus rows may render identically in
   both notations. Result: 1 row (`x - y - a`, which genuinely has no grouping in either form).

## 5. Known renderings (asserted exactly, hand-reasoned)

```
1/2                -> \frac{1}{2}          (Exprs.Rational(1,2); -1/2 -> -\frac{1}{2}, 7 -> 7)
x^2                -> x^{2}
sqrt(x)            -> \sqrt{x}
(x + 1)^2          -> \left(x + 1\right)^{2}
x - (y - a)        -> x - \left(y - a\right)
(x + y)*(x - y)    -> \left(x + y\right) \cdot \left(x - y\right)
x^(-2)             -> x^{-2}
1/(x + 1)          -> \left(x + 1\right)^{-1}     (a power of a sum; the fraction form needs a product)
abs(x)             -> \left|x\right|
x^(y^2)            -> x^{y^{2}}
sin(x)             -> \sin(x)
```

Observed from the JIT runner (`Lovelace.Run --file latex.ls`, envelope `output` array):

```
latex(1/2)     -> \frac{1}{2}
latex(x^2)     -> x^{2}
latex(sqrt(x)) -> \sqrt{x}
latex((x+1)^2) -> \left(x + 1\right)^{2}
latex(x-(y-a)) -> x - \left(y - a\right)
```

and from the corpus run (`latex2.ls`, 18 rows, JSON `+` unescaped):

```
x - (y - a)          -> x - \left(y - a\right)
x - y - a            -> x - a - y                       (display order, same as Pretty)
-x^2                 -> -x^{2}
(-x)^2               -> \left(-x\right)^{2}
(x + y)*(x - y)      -> \left(x + y\right) \cdot \left(x - y\right)
x^(-2)               -> x^{-2}
1/(2*(x + 1))        -> \left(2 \cdot \left(x + 1\right)\right)^{-1}
-1/(2*(x + 1))       -> -\frac{1}{2 \cdot \left(x + 1\right)}
x/(y + 1)            -> \frac{x}{y + 1}
abs(x)               -> \left|x\right|
x^(y^2)              -> x^{y^{2}}
(x^y)^2              -> \left(x^{y}\right)^{2}
(-x)^(1/2)           -> \sqrt{-x}
x^(-1/2)             -> \frac{1}{\sqrt{x}}
(1/2)*x + 1/3        -> \frac{1}{2} \cdot x + \frac{1}{3}
exp(-x^2)            -> \exp(-x^{2})
x/(2*y)              -> \frac{x}{2 \cdot y}
(x + 1)^12           -> \left(x + 1\right)^{12}
```

## 6. The builtin

```
Add("latex", new[] { "expr" }, args => Printing.PrettyPrint(AsExpr(args[0]),
        new Printing.PrintOptions(Printing.PrintMode.Latex)),
    new BuiltinDescriptor("latex", new[] { "expr" }, BuiltinCategories.Introspection,
        "Renders an expression as LaTeX source. This is the printer's LaTeX mode, not a second
         printer: it shares the precedence table and the structural decisions of the pretty form,
         so the two can never describe different expressions.",
        ["latex((x + 1)/y)"], "Text", ["print", "type"]));
```

Category `Introspection`, one real runnable example (`latex((x + 1)/y)` — executed by
`Lovelace.Suite.Tests.DescriptorExampleTests` as a doctest), return kind `Text`, cross-references
`print` and `type`. Metadata completeness: `HelpMetadataCompletenessTests` walks every builtin
through `help` and finds no `(no summary registered)` / undeclared return kind; both suites pass.

**New builtin count: 106** (was 105). The pinned tripwire
`HyperbolicBuiltinsAndNegativeIntegerPowersTests.BuiltinCount_IncludesTheThreeHyperbolicFunctions`
was updated 105 -> 106 (it asserts the registry size; the change adds exactly one builtin).

## 7. Passing output (observed)

```
dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    1 Warning(s)        (pre-existing xUnit2013 in Lovelace.Suite.Tests/EmptyReductionTests.cs)
    0 Error(s)
Time Elapsed 00:00:03.05

dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   723, Skipped:     6, Total:   729, Duration: 15 s
(the 6 skips are the pre-existing Sympy-oracle skips; the Latex file alone is 115/115)

dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   638, Skipped:     0, Total:   638, Duration: 8 s

dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 436 ms
```

Goldens: every `Lovelace.Run.Tests/fixtures/*.json` envelope still matches
(`GoldenEnvelopeTests` is inside the 39 passing Run.Tests), `git status` shows no fixture file
touched, and no golden was regenerated. Pretty/Canonical are additionally pinned by
`CanonicalAndPretty_AreUntouchedByTheLatexArm` over the 34-row corpus and by the untouched
`PrettyParenthesesTests` (all green).

## 8. Not done / limits

- **Symbol names are emitted verbatim** (as Pretty does): a symbol named `x_1` renders as `x_1`
  (LaTeX reads it as a subscript, which is usually what is meant, but a multi-character subscript
  would need braces and there is no escaping of `#`, `%`, `&`). Escaping is a follow-up, not a
  correctness issue: the parity corpus uses plain names.
- **Parity/load-bearing coverage is the algebraic + elementary-function surface.** The relational,
  logical, piecewise, calculus and O-term arms of the Latex renderer are implemented and share
  `Delimit`/`Prec`, but the denoiser corpus does not include them (piecewise renders as
  `\begin{cases}...\end{cases}`, which the test denoiser deliberately does not model). No claim
  is made about their delimiter minimality.
- The `Unicode` option is ignored by the Latex arm (LaTeX has its own commands); documented on
  `PrintOptions`.
