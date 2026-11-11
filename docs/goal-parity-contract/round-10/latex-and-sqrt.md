# Round 10 — LaTeX, the complex square root, and the nested-power printer

Scope: `rust/lovelace-symbolics/src/lib.rs`, `rust/lovelace-lang/src/builtins.rs`,
`rust/lovelace-lang/src/lib.rs` (test-module declaration) and the new test module
`rust/lovelace-lang/src/latex_and_sqrt_parity.rs`. No `.cs`, fixture, `parity/`, or other doc
was changed. The two production defects were the `latex` builtin advertised but not dispatched,
and `sqrt` of a negative Real returning `0 (Real)`. A third, previously hidden printer defect
(a power used as a power base printing without parentheses) surfaced when the canonicalisation
was corrected and is fixed here as well.

Environment for every command:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
```

The C# oracle probes ran the pinned runner:

```powershell
dotnet exec Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file <script> --text --omit-functions
```

## 1. `latex` — advertised but not dispatched

### Root cause

`rust/lovelace-lang/src/registry.rs:855` already advertised `latex` (`Introspection`, one
argument, `Text`), but `builtins::call` had no arm for it, so every `latex(...)` script failed
with `Unknown function 'latex'.` The printer itself had no LaTeX mode at all: only the plain
infix `Expr::to_string` existed.

### Fix

`rust/lovelace-symbolics/src/lib.rs` now has the LaTeX arm of the reference printer:

- the same precedence table (`Or -1`, `And/Relation 0`, `Add 1`, `Mul 2`, `Pow 3`, `Atom 4`) and
  the same "parent > child" delimiter decision, applied as `\left(...\right)`.
- `latex_text_prec`: the precedence of rendered TEXT as written, so `\frac{1}{2}` is an atom while
  `1/2` is a division, a negative numeral is additive, and a control symbol/word is one token.
- the shared product decomposition: one `\frac` per division, nested for several denominator
  factors, rational coefficients folded into the fraction, `\cdot` between factors.
- radical forms for exponents `1/2` and `-1/2`, `\sqrt`, `abs` as `\left|...\right|`, and the
  documented function names (`\sin`, `\cos`, ...; `\operatorname{...}` otherwise).
- name escaping: all ten LaTeX specials (`# $ % & _ { } ~ ^ \`) use the LaTeX2e spellings, so
  `x_1` is `x\_1`, never a subscript.
- products are sorted by the reference kernel's total order before shaping, so the fraction
  nesting is independent of this crate's own `factor_order`.

`builtins::call` dispatches `latex`: `Value::Symbolic` prints through the new arm, an exact Real
is converted to its exact rational first (so `latex(0.5)` is `\frac{1}{2}`), an inexact Real is
published as its decimal, and any other kind reports `latex needs an expression.`

### Before / after probes (exact stdout)

Before = `rust/target/tmp/before-target/release/lovelace-run.exe` (SHA-256
`F8BEE52CE529AF33847ED307E93CCC95F347870B4FF90B4E7934656B8F627D70`; it also returns
`0 (Real)` for `sqrt(-1)`, so it is a genuine pre-fix baseline). After = the current
`rust/target/release/lovelace-run.exe` (SHA-256
`BF5D6B910B154520F3FAC5ACA03A0213665491D4647783B929BD4558F0CC2274`).

`latex((x + 1)/y)` with `x = symbol("x"); y = symbol("y");`

```text
before  stdout: (empty)
        stderr: Error [InvalidOperation/DomainError]: Unknown function 'latex'.
after   = \frac{x + 1}{y}
          _ = \frac{x + 1}{y}
          x = x
          y = y
oracle  = \frac{x + 1}{y}
          _ = \frac{x + 1}{y}
          x = x
          y = y
```

`latex((x + 1)^2)` and a symbol with an underscore:

```text
after   = \left(x + 1\right)^{2}      oracle = \left(x + 1\right)^{2}
          _ = \left(x + 1\right)^{2}            _ = \left(x + 1\right)^{2}

v = symbol("x_1"); latex(v)
after   = x\_1                       oracle = x\_1
          _ = x\_1                             _ = x\_1
          v = x_1                              v = x_1

latex(1/2)
after   = \frac{1}{2}                oracle = \frac{1}{2}
          _ = \frac{1}{2}                      _ = \frac{1}{2}
```

All 49 precedence-corpus sources of `LatexPrinterTests` (the same sources the three theories
`GroupingParity`, `Delimiters_AreLoadBearing` and `CanonicalAndPretty` iterate) were replayed
through the oracle and the fixed runner: **49/49 identical**. That includes the flat/nested
fraction shapes (`(x/y)/z`, `x/(y*z)`, `(x/y)/(z/w)`, `x/y/z/w`), negative and rational bases,
`apart`, `abs`, radicals and the 12-power polynomial.

Escaping: the 11 awkward single-character/underscore names and the 6 larger-expression contexts
were replayed against the oracle: all identical (`x\_1`, `p\%q`, `a\&b`, `h\#1`, `c\{d\}`,
`o\}e`, `b\textbackslash{}s`, `e\textasciicircum{}f`, `t\textasciitilde{}u`, `m\$n`,
`\_\_`, and in context `x\_1^{2}`, `a\%b \cdot c\&d`, `2 \cdot x\_1`,
`\left|x\_1\right|`, `x\_1^{-1}`, `\sin(x\_1)`).

## 2. `sqrt` of a negative value

### Root cause

The builtin sent every non-symbolic argument to `Real::sqrt`. `Real` is a real-domain type, so a
negative radicand produced the "no real root" value `0` (pre-fix stdout below), while the oracle
returns the exact principal complex root. The symbolic `Expr::sqrt` had the symmetric bug: a
negative exact value returned its negative real root instead of the imaginary branch.

### Fix

- `builtins::sqrt` keeps `Symbolic` and non-negative numeric arguments on their old paths. A
  negative Real is converted to its exact rational and routed through the symbolic power
  `base^(1/2)`.
- `Expr::sqrt` now returns `i*sqrt(|q|)` for a negative rational `q`, folding perfect squares
  (`sqrt(-4) = 2*i`, `sqrt(-1/4) = 1/2*i`) and keeping non-squares as radicals
  (`sqrt(-2) = i*sqrt(2)`).
- `Expr::pow` implements the reference kernel's principal half-integer branch for a negative
  rational base (`(-4)^(1/2) = 2*i`, `(-4)^(-1/2) = -i/2`).
- The imaginary unit is the named constant `Fun("i", [])`, not a user symbol spelled `i`, and its
  integer powers fold (`i^2 = -1`, `i^3 = -i`). `i = symbol("i")` remains an ordinary symbol.
- `builtins::as_expr` converts an exact Real through `Real::to_rational` (including periodic
  values such as `1/3`), which is what lets numeric values participate in symbolic expressions.

### Before / after probes (exact stdout)

```text
script          before                        after / oracle
sqrt(-1)        = 0 (Real)                    = i (Symbolic)
                _ = 0                           _ = i
sqrt(-4)        = 0 (Real)                    = 2*i (Symbolic)
                _ = 0                           _ = 2*i
sqrt(-2)        = 0 (Real)                    = i*sqrt(2) (Symbolic)
                _ = 0                           _ = i*sqrt(2)
sqrt(-1)^2      = 0 (Real)                    = -1 (Symbolic)
                _ = 0                           _ = -1
```

Every `sqrt(-...)` probe in the C# tests was replayed against the oracle and matched, including
`-3`, `-9`, `-16`, `-1/4`, `-4/9`, `-0.25`, `-2.5`, `-1/2`, plus the compositions
`sqrt(-4) + sqrt(-1) = 3*i` and `sqrt(-4)*sqrt(-9) = -6`. The 25-case negative/control comparison
matched 25/25. The only observed difference in the wider sqrt probe was the pre-existing display
width of the POSITIVE `sqrt(2)` decimal (`Real::sqrt` is untouched).

## 3. Nested-power printer regression (found while fixing the pool)

Correcting `Expr::pow` to stop combining `(x^y)^2` into `x^(2*y)` exposed a printer defect that
the old canonicalisation had hidden: `power_text` parenthesised `Add`/`Mul` bases but not a
power base, so `Pow(Pow(x,y),2)` printed `x^y^2` and re-parsed as `x^(y^2)`. The same hole
printed `-2^x`, `1/2^x` and `x^1/2` for bases/exponents that need grouping.

`power_text` now parenthesises a power, negative or non-integer numeric base, and a power or
rational exponent. The required round trips are exact:

```text
(x^y)^2    -> (x^y)^2       x^(y^2) -> x^(y^2)
(x + y)^2  -> (x + y)^2     (x*y)^2 -> (x*y)^2
(-x)^2     -> (-x)^2        2^(3^2) -> 512 (Natural)
(-2)^x     -> (-2)^x        (1/2)^x -> (1/2)^x
```

`cargo test -p lovelace-run pretty_round_trips -- --nocapture` is green (the test re-parses
each display and compares the canonical form). Before this fix it failed at
`lovelace-run/src/pretty_batch.rs:115` with
`'(x^y)^2' rendered as 'x^y^2' and re-parsed to a different canonical form`.

## 4. Tests added

`rust/lovelace-lang/src/latex_and_sqrt_parity.rs` (7 tests, 200+ assertions):

1. `latex_precedence_corpus_matches_the_oracle_exactly` — all 49 corpus sources.
2. `latex_known_renderings_match_the_oracle_exactly` — the documented exact renderings plus
   rational constants and reals.
3. `latex_symbol_name_escaping_matches_the_oracle_exactly` — 11 awkward names and 6 contexts.
4. `latex_documented_escapes_are_the_single_characters` — the ten documented escapes.
5. `sqrt_of_negative_values_is_the_principal_complex_root` — 14 negative/composed roots.
6. `sqrt_of_non_negative_and_symbolic_values_is_unchanged` — the real/symbolic controls,
   including a user symbol literally named `i`.
7. `nested_power_renderings_round_trip_through_the_parser` — the six required shapes plus
   negative/rational bases. Each display is re-evaluated and its canonical form compared.

`rust/lovelace-symbolics/src/lib.rs` (6 unit tests): `Latex_GivenRationalConstant_IsAFraction`,
`Latex_GivenPowerAndProduct_UsesTheSharedGrouping`, `Latex_GivenSymbolName_EscapesTheTenSpecials`,
`Sqrt_GivenNegativeValue_IsTheExactImaginaryMultiple`,
`Pow_GivenNegativeHalfInteger_IsThePrincipalImaginaryValue`,
`Pow_GivenImaginaryUnit_IntegerPowersFold`.

```text
cargo test -p lovelace-lang latex_and_sqrt_parity
  test result: ok. 7 passed; 0 failed

cargo test -p lovelace-symbolics
  test result: ok. 33 passed; 0 failed   (27 before; 6 added)
```

## 5. Gates

### Workspace — green

```text
cargo test --workspace   (exit 0)
  58 + 67 + 125 + 7 + 638 + 134 + 2328 + 137 + 1 + 33 = 3,528 passed; 0 failed
```

The task started from 3,404 tests. 13 of the increase are the tests added here (7 language,
6 symbolics); the rest of the increase comes from the concurrent porting agent's test modules
already present in the tree. No test was weakened, ignored or removed.

### Documentation corpus — unchanged 212/214

```text
cargo run --release -p lovelace-corpus
FAIL sym/071
FAIL sym/076
corpus: 212 passed, 2 failed, 214 total      (exit 1, the same two pre-existing failures)
```

### Golden fixtures — 20/20

```text
node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust: 20/20
PASS: oracle 20/20, rust 20/20                  (exit 0)
```

### Differential — no delta

Before and after were measured with the same command and seed:

```text
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
before: generated 200 — 134 match / 66 mismatch / 0 error; replayed 1752 — 162 / 1590 / 0
after : generated 200 — 134 match / 66 mismatch / 0 error; replayed 1752 — 162 / 1590 / 0
delta : 0 / 0; no category changed (arithmetic 35/1, precision 16/4, arrays 20/12,
        symbolic 24/12, records 16/12, errors 10/14, syntax 5/9, registry 8/2)
```

Both runs exit 1 because the generated corpus itself has mismatches; that is the gate's normal
"mismatches exist" signal, and it is identical before and after. The generated corpus is
unaffected by these two paths: every category count is identical before and after, so the two
pools improve without a differential delta. The regression replay is also identical (no new
mismatches;
`stillMismatching=1590`, `resolved=162`, `new=0` in both runs).

## 6. What the 213 LaTeX census cases mean here

The 213 census cases are `LatexPrinterTests` (164) and `LatexSymbolNameEscapingTests` (49):

- 164 = 49 corpus rows × 3 theories (`GroupingParity`, `Delimiters_AreLoadBearing`,
  `CanonicalAndPretty`) = 147, plus 11 `KnownRenderings` rows and 6 facts.
- 49 = 12 awkward names × 3 theories = 36, plus the underscore fact, 11 documented escapes and
  the larger-expression fact.

This round verifies the PRINTED CONTRACT of all 49 corpus inputs and of every awkward-name
rendering that is expressible in the language here (11 of the 12; the terminal-backslash name is
covered at the printer level) against the oracle, not the C# test harness's private shape parser. The three property theories assert a
structural relation (a denoised shape tree of Pretty vs LaTeX) and would require porting those
two test-only parsers; the production arm they measure is what changed here. The one
`KnownRendering` shape that cannot be constructed in this crate is the direct
`Power(ComplexConstantExpr, x)` fact: Rust `Expr` has no complex-constant leaf (language-level
complex values are a separate `Value` variant). The complex value itself is rendered
best-effort by the builtin (`(re + im i)`), but that specific model-level fact is not ported.

## 7. Not verified / limits

- **C# tests were not executed** (the VSTest host cannot run in this environment, as recorded in
  `parity/README.md`); every expected string above came from the pinned executable oracle.
- The all-ten-specials name `x_1%&\#'{}~^\` cannot be placed in a LANGUAGE string literal
  identically in both runners: a terminal backslash is tokenized differently (the oracle accepts
  one trailing `\`, this crate's tokenizer needs `\\`). That is a parser divergence, not a
  LaTeX divergence. The ten individual escapes and the larger-expression contexts are verified
  through the language, and the full ten-special name (including its trailing backslash) is
  verified at the printer level by `Latex_GivenSymbolName_EscapesTheTenSpecials`.
- Positive `sqrt(2)` still prints a longer decimal than the oracle (pre-existing Real display
  width); the real square-root path was not touched.
- The differential before baseline is the pre-fix reconstruction
  `rust/target/tmp/before-target/release/lovelace-run.exe` (it returns `0 (Real)` for `sqrt(-1)`
  and refuses `latex`); its source predates round 9. The 200 generated cases are unaffected by
  that difference, and both runs report exactly the same counts.

## 8. Files changed

| file | change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | LaTeX arm, name escaping, C#-order product shaping, exact negative sqrt, principal half-integer power, `i` named constant and power folding, numeric power fold, nested-power pretty grouping |
| `rust/lovelace-lang/src/builtins.rs` | dispatch `latex`; route negative Real sqrt to the principal complex branch; exact-Real `as_expr`; symbolic `abs` |
| `rust/lovelace-lang/src/lib.rs` | declare the test module |
| `rust/lovelace-lang/src/latex_and_sqrt_parity.rs` | new 7-test parity module |
| `docs/goal-parity-contract/round-10/latex-and-sqrt.md` | this report |
