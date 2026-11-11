# Round 11 — the LaTeX/printing batch: 167 census cases ported, 285 refused with reasons

Scope: `rust/lovelace-lang/src`, `rust/lovelace-run/src` (tests only) and
`parity/manifest-map.json`. No production line, no `.cs`, no fixture changed. The C# oracle is
the pinned `Lovelace.Run` runner, replayed for every fixed string below; the C# test assertion is
the contract and no assertion was weakened.

## 1. Result

Coverage moved from **3,749/6,209 (60.38 %)** to **3,916/6,209 (63.07 %)** — **167 new
mappings**. `Lovelace.Symbolics.Tests` moved from 232/1,226 (18.92 %) to **399/1,226
(32.54 %)**. `Lovelace.Complex.Tests` stays 0/121, refused with a type-level reason (section 4).

| C# test project | Cases | Ported before | Ported now | New | Refusals recorded |
|---|---:|---:|---:|---:|---:|
| Lovelace.Symbolics.Tests — `LatexPrinterTests` | 164 | 0 | 106 | **106** | 58 |
| Lovelace.Symbolics.Tests — `LatexSymbolNameEscapingTests` | 49 | 0 | 48 | **48** | 1 |
| Lovelace.Symbolics.Tests — `PrettyParenthesesTests` | 198 | 80 | 93 | **13** | 105 |
| Lovelace.Complex.Tests | 121 | 0 | 0 | **0** | 121 |
| **total touched** | **532** | **80** | **247** | **167** | **285** |

The 167 new rows break down as: 41 `GroupingParity` + 49 `Delimiters_AreLoadBearing` +
11 `KnownRenderings` + 5 hand-reasoned renderings + 48 escaping cases (36 theory rows, the
underscore fact, 11 documented escapes) + 10 newly-round-tripping Pretty rows + 2 minimality rows
+ 1 reported rational-base rendering.

## 2. What was added

### 2.1 `rust/lovelace-lang/src/latex_printer_parity.rs` (new, 12 tests)

* `latex_grouping_parity_corpus_matches_the_pretty_rendering` — a line-for-line port of the C#
  test-only denoiser (`ShapeTree.TokenizeInfix/TokenizeLatex` + grammar + precedence) applied to
  the 41 corpus rows for which the premise holds. It compares the Pretty tree with the LaTeX tree,
  which is exactly what `LatexPrinterTests.GroupingParity…` asserts.
* `latex_delimiters_are_load_bearing_on_every_corpus_row` — the port of
  `Delimiters_AreLoadBearing`: every `\left(…\right)` / `\left|…\right|` pair is removed,
  the text re-parsed, and a pair whose removal leaves the tree unchanged fails. All 49 row
  renderings are oracle-identical, so this is the reference's own measurement.
* the five hand-reasoned renderings: rational constant, negative numeric base, rational base,
  flat denominators, and the "LaTeX arm is not the Pretty arm" guard (1 identical row here, the C#
  bound is ≤ 2).
* the six escaping tests: no bare special (ported control-sequence tokenizer), decodes back to the
  name, Pretty/Canonical keep the name verbatim, the underscore decision, and the 11 documented
  escapes. The C# tests build `Exprs.Symbol(name)`; Rust builds the same model value with
  `Expr::sym(name)`, including the trailing-backslash composite name `x_1%&#'{}~^\`.

### 2.2 `rust/lovelace-run/src/pretty_batch.rs` (extended)

* `ROUND_TRIP` gains the 10 rows whose display now re-parses to the source's canonical form
  (`(-x)^(1/2)`, `(-1)^x`, `(-2)^x`, `(-1/2)^x`, `(-1.5)^x`, `(-3)^(-x)`, `(1/2)^x`,
  `(2/3)^x`, `(-3/2)^x`, `(1/2)*x + 1/3`).
* `MINIMALITY` gains `(2/3)^x` and `(1/2)*x + 1/3`.
* `pretty_reported_case_power_of_a_rational_constant_has_a_delimited_base` — the port of
  `ReportedCase_PowerOfARationalConstant_HasADelimitedBase` (exact rendering, canonical form and
  the delimited-base property).

### 2.3 Oracle verification of every fixed string

```text
LaTeX renderings of the 49-row corpus:      49/49 identical to the oracle
Pretty renderings of the same 49 rows:      38/49 identical (11 differ — production gap, section 5)
round-trip rows newly passing:              10/16 probed
minimality rows newly passing:              2/2
awkward symbol names (`latex(symbol(...))`): 11/12 through the language, the composite
                                            trailing-backslash name through the model-level printer
```

The corpus LaTeX strings are pinned in `rust/lovelace-lang/src/latex_and_sqrt_parity.rs`; this
round's tests start from those same oracle-verified renderings.

## 3. Observed gates

```text
cargo test --workspace                                   # exit 0
  58 + 67 + 125 + 7 + 650 + 134 + 2,328 + 138 + 1 + 33 = 3,541 passed; 0 failed
  (lovelace-lang 650 = 638 + 12 new; lovelace-run 138 = 137 + 1 new)

powershell -File parity/gen-rust-tests.ps1               # exit 0
  crates=11 tests=3541 suiteExit=0 passed=3541 failed=0

node parity/gen-manifest.mjs                             # exit 0
  { "rows": 6209, "dispositions": { "excluded": 118, "ported": 3916, "deferred": 2175 },
    "coveragePercent": 63.07, "rustTests": 3541 }

node parity/tools/gate-manifest.mjs                      # exit 0
  { "censusCases": 6209, "manifestRows": 6209,
    "dispositions": { "ported": 3916, "consolidated": 0, "excluded": 118, "deferred": 2175 },
    "coveragePercent": 63.07, "claimedRustTests": 3916, "projectsInCensus": 15, "failures": [] }
  PASS (structural): 6209 test cases dispositioned; 3916 claim Rust coverage (63.07%).

node parity/tools/coverage-report.mjs                    # exit 0
  coverage.md written: 3916/6209 ported (63.07%)

cargo run --release -p lovelace-corpus                   # exit 1, the two pre-existing failures
  FAIL sym/071
  FAIL sym/076
  corpus: 212 passed, 2 failed, 214 total

powershell -File parity/gate-fixtures.ps1                # exit 0
  oracle: 20/20
  rust: 20/20
  PASS: oracle 20/20, rust 20/20
```

## 4. Refusals (285) and why

| Pool | Cases | Refused | Dominant reason |
|---|---:|---:|---|
| `LatexPrinterTests.CanonicalAndPretty_AreUntouchedByTheLatexArm` | 49 | 49 | asserts the Canonical/Debug print MODES (`CanonicalPrint == PrintMode.Canonical`, `DebugPrint == PrintMode.Debug`); this crate has one canonical S-expression form (`Expr::canonical`) and no Debug printer or mode dispatch, so three of the four assertions have no counterpart |
| `LatexPrinterTests.GroupingParity…` | 49 | 8 | the Rust Pretty arm renders these rows differently from the reference, so the C# premise fails: `x - (y - a)` → `x - y - a`, `a - (x + y)` → `a - x + y`, the four flat-division rows → `x/(y*z)` etc., `(-x)^(1/2)` and `x^(-1/2)` keep the power form |
| `LatexPrinterTests` complex-constant fact | 1 | 1 | `Exprs.Complex(1,1)` has no counterpart: the Rust `Expr` enum has no complex constant leaf |
| `LatexSymbolNameEscapingTests.InsideLargerExpressions` | 1 | 1 | the fact also asserts the LaTeX of `Exprs.Derivative(f, x_1)`; Rust has no derivative node and `diff(f, x_1)` folds to `0` in both runners — half the fact is inexpressible |
| `PrettyParenthesesTests.CanonicalMode…` | 48 | 48 | asserts `Printing.CanonicalParse` round-trip; this crate has no canonical S-expression parser (only infix `parse_expression`) |
| `PrettyParenthesesTests.DebugMode…` | 48 | 48 | asserts `DebugPrint == PrintMode.Debug` and the absence of `((`; no structural Debug printer exists |
| `PrettyParenthesesTests.RoundTrip…` | 48 | 6 | the Rust display re-parses to a different canonical form: `x - (y - a)` → `x - y - a`, `a - (x + y)` → `a - x + y`, `(x/y)/z` / `x/y/z` → `x/(y*z)`, `x/y/z/w` → `x/(w*y*z)`, `(x/y)/(z/w)` → `x/(y*z/w)` |
| `PrettyParenthesesTests` reported/model facts | 3 | 3 | `factor(-x^2 - 2*x - 1)` is not factored by the Rust `factor`; the flat-denominator fact fails on two of three rows; the model-level complex base cannot be built |
| `Lovelace.Complex.Tests` | 121 | 121 | see below |

**Why the whole Complex project is refused.** Its 121 cases assert the `Lovelace.Complex` model
types: `Complex` (operators, `Parse`/`ToString`, `Magnitude`, `Reciprocal`, `Complex.Pi`/`E`,
fixed-width fast paths), `LComplex64`/`LComplex128` (fixed-width structs and
`LRealPromoteException`), `DenseArray<Complex>` (`DType.Complex`, `ElementType`), and the
`ComplexMath` static API (Exp/Sqrt/Ln/Sin/Cos/Tan/Atan/Atan2/Acosh/Asinh/Atanh/Pow plus its
provenance rules). This workspace has no Complex crate and no fixed-width complex type; complex
values exist only as the runtime `Value::Complex(re, im)` pair, and the language cannot construct
one — `1 + 2i` is a parse error in both runners (`Expected ';' or end of input but found 'i'`),
and `complex` is the domain constant, not a constructor (`complex(): expected 0 arguments; got 2`
in the oracle). The concrete inputs therefore cannot be built, so no assertion can be evaluated.

## 5. Divergences found (not fixed — production edits are forbidden)

| # | Behaviour | C# | Rust |
|---|---|---|---|
| 1 | nested subtraction in the Pretty arm | `x - (y - a)`, `a - (x + y)` (parenthesised) | `x - y - a`, `a - x + y` |
| 2 | flat division in the Pretty arm | `x/y/z`, `x/w/y/z`, `x/y/(z/w)` | `x/(y*z)`, `x/(w*y*z)`, `x/((z/w)*y)` |
| 3 | negative/rational exponents in the Pretty arm | `x^-2`, `(x + 1)^-1`, `sqrt(-x)`, `1/sqrt(x)` | `x^(-2)`, `(x + 1)^(-1)`, `(-x)^(1/2)`, `x^(-1/2)` |
| 4 | `factor(-x^2 - 2*x - 1)` | `-(x + 1)^2` | not factored |
| 5 | `diff(exp(x)*x^2, x)` term order | `2*x*exp(x) + exp(x)*x^2` | `exp(x)*x^2 + 2*x*exp(x)` |

These are the reasons the 8 grouping rows and 6 round-trip rows stay refused. The LaTeX arm itself
is exact: 49/49 corpus renderings and every escaping case match the oracle.

## 6. Shortfall against the 400 target

The three named Symbolics pools plus `Lovelace.Complex.Tests` contain 532 cases; 80 Pretty cases
were already ported, leaving 452. Of those, 167 are now ported and 285 are refused for the concrete
reasons in section 4. The remaining 233 mappings needed to reach 400 cannot be honestly produced
inside the rules: 121 need a Complex library type and a language-level complex constructor, 96
need a Canonical parser and a Debug printer, 49 need print-mode dispatch, and 19 are rows where the
Rust Pretty/algorithmic output differs from the reference. Weakening any of them (for example
mapping the Canonical-mode rows to a vacuous "canonical == canonical" check) would violate the
"weaker is a refusal" rule, so they stay deferred. No production change was made to close them.

## 7. Files changed

| file | change |
|---|---|
| `rust/lovelace-lang/src/latex_printer_parity.rs` | new test module: ported denoiser, 12 tests |
| `rust/lovelace-lang/src/lib.rs` | `#[cfg(test)] mod latex_printer_parity;` |
| `rust/lovelace-run/src/pretty_batch.rs` | +10 round-trip rows, +2 minimality rows, +1 reported fact test |
| `parity/manifest-map.json` | 167 ported rows and 285 refusal reasons |
| `parity/manifest.toml`, `parity/coverage.md` | regenerated (not hand-edited) |
| `docs/goal-parity-contract/round-11/latex-porting-batch.md` | this report |

## 8. Not verified / limits

- **The C# tests were not executed** (the VSTest host cannot run in this environment; see
  `parity/README.md`). Every expected string comes from the pinned `Lovelace.Run` oracle with the
  same input, not from running the C# tests.
- The two shape-parity theories are executed through the ported denoiser, not by running the C#
  test harness; the port is a translation of the same algorithm and is applied to the same sources.
- The composite name `x_1%&#'{}~^\` cannot be spelled in a Rust language string literal without
  the tokenizer divergence recorded in round 10 (the oracle treats a backslash literally, this
  crate's tokenizer escapes it). Its rendering is asserted at the model level (the same
  `Expr::sym` the C# fact uses); the oracle equivalent was probed with its single-backslash
  spelling and is identical.
- The C# `factor` and Pretty-divergence observations are reported, not fixed: production edits
  are outside this batch.
