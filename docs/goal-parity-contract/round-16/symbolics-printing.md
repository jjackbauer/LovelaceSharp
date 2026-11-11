# Round 16 — the Symbolics printing cluster: canonical parse, Debug print and print-mode dispatch

Scope: `rust/lovelace-symbolics/src/lib.rs`, `rust/lovelace-lang/src/builtins.rs`,
`rust/lovelace-lang/src/lib.rs` and the two new test modules
`rust/lovelace-symbolics/src/printing_round16.rs` and
`rust/lovelace-lang/src/printing_round16.rs`. No `.cs`, fixture, corpus example or `parity/`
artifact was changed. The C# oracle is the pinned `Lovelace.Run` runner plus a temporary probe
program that calls the reference `Lovelace.Symbolics.Printing` class directly (no repository
source was modified to build it).

## 1. Result

The three refused pools in the round-11 refusal table are now backed by a real printing surface:

| Refused pool | Rows | New coverage |
|---|---:|---|
| `PrettyParenthesesTests.CanonicalMode_StillRoundTripsThroughCanonicalParse` | 48 | `canonical_parse` + 48-row round-trip test |
| `PrettyParenthesesTests.DebugMode_IsStructuralAndContainsNoPrecedenceParentheses` | 48 | `Expr::debug_print` + 48-row coherence/no-`((` test |
| `LatexPrinterTests.CanonicalAndPretty_AreUntouchedByTheLatexArm` | 49 | `PrintMode` dispatch through `print(expr, mode)` + 49-row test |
| **total** | **145** | 17 new Rust tests (10 model, 7 language) |

Measured gates:

- `cargo test --workspace`: **3,747 passed, 0 failed, exit 0** (3,730 before; +17).
- Corpus: **212/214**, the same two failures (`sym/071`, `sym/076`) as the baseline.
- Golden fixtures: **oracle 20/20, Rust 20/20**.
- Differential (`--cases 200 --seed 1 --no-minimize`): **154 match / 46 mismatch / 0 error**,
  identical to the pre-change baseline (generated cases), regressions **600 / 1,152**, identical.
  **Delta: 0** (the same 154/46 and 600/1,152 as the baseline).

## 2. What was implemented

### 2.1 Canonical-text parser (`lovelace_symbolics::canonical_parse`)

`pub fn canonical_parse(text: &str) -> Result<Expr, CanonicalParseError>` is a tokenizer plus
recursive-descent parser for the tagged prefix language the wire form uses: `(rat N D)`,
`(int N)` (oracle alias), `(sym NAME)`, `(add ...)`, `(mul ...)`, `(pow A B)`,
`(fn NAME ARG...)`, `(eq|ne|lt|le|gt|ge A B)`, `(rootof POLY INDEX)` and `(pi)`/`(e)`/
`(i)`/`(inf)`. Refusals carry the byte offset and the exact expectation (bad integer, zero
denominator, unbalanced parentheses, trailing tokens, unknown tag). Tags the expression model
cannot represent as canonical output (`real`, `cplx`, `der`, `integ`, `pw`, `and`, `or`,
`not`, `order`) are refused rather than guessed.

The round-trip contract is over this crate's canonical output:
`canonical_parse(e.canonical()).unwrap().canonical() == e.canonical()` for every `Expr` shape.
The parser is a codec, not a re-canonicaliser: operands keep the text's order, so a `(add A B)`
whose text order differs from the display order still reprints identically. `(int N)` is accepted
because the oracle emits it for integer-constant nodes; the Rust model has one numeric leaf, so it
normalises to `(rat N 1)` (documented in the API docs).

`Expr::canonical()` for the explicit `Div` node now emits the two multiply-by-reciprocal factors
in `canonical_order`, so the parsed `Mul` reprints the same text (an unsorted `Div` broke the
round-trip when the denominator sorted before the numerator).

### 2.2 Debug print (`Expr::debug_print`)

The reference shape, transcribed node for node from `Lovelace.Symbolics/Printing.cs` and verified
against the oracle: `rat(1/2)`, `sym(x)`, `add[...]`, `mul[...]`, `pow[a, b]`,
`fn:sin(sym(x))`, `rel:Lt(sym(x), rat(1))`, `named(Pi)`/`named(E)`/`named(I)`/
`named(Infinity)`, `rootof[poly, 0]`, `and[...]`/`or[...]`/`not[...]`, and the quotient's
multiply-by-reciprocal shape. `sqrt` debug-renders as the power the canonical form prints
(`pow[arg, rat(1/2)]`), and add/multiply operands are visited in canonical order, which is the
order the reference expression stores. The C# invariant "DebugPrint contains no `((`" is asserted
over the whole corpus.

### 2.3 Print-mode dispatch

`pub enum PrintMode { Pretty, Canonical, Latex, Debug }` with `PrintMode::from_name`
(case-insensitive, exact otherwise), `Expr::print(mode)` and `Expr::print_named(name)`; the four
arms are the reference `PrettyPrint(expr, options)` dispatch. The language surface is the
documented `print` builtin: `print(expr, "pretty"|"canonical"|"latex"|"debug")` writes that
mode's rendering. The one-argument form and every non-`(Symbolic, Text)` call keep the reference's
plain `print(value, ...)` behaviour. An unknown name is an argument refusal on the wire:

```text
code: InvalidArgument, category: TypeMismatch
print(): unknown print mode 'wibble'; expected one of: pretty, canonical, latex, debug.
```

The registry metadata is deliberately unchanged (no new builtin, no changed summary/arity): the
four modes are a Rust-surface extension of the existing varargs `print`, and adding a builtin or
changing its descriptor would have changed the `functions` payload the differential gate compares
(the 3 registry cases currently match). There is no C# string-mode API, so the unknown-mode
refusal itself has no oracle counterpart; the dispatch it belongs to is the oracle's enum dispatch.

## 3. Oracle probes — 28 expressions, before and after

Method: the pinned oracle runner evaluated the same language sources; a temporary C# probe called
`Printing.CanonicalPrint`, `Printing.DebugPrint` and `Printing.PrettyPrint(..., PrintMode.Latex)`
on the expression each source builds. The "before" column is the runner binary built from the
pre-change source copy (the mode argument did not exist, so `print(e, "debug")` printed the
pretty value followed by the literal mode word). The probe JSON is
`rust/target/tmp/r16-probes.json` (28 rows, SHA-256 of the row array
`c89d6a7e0ec82ce88f3cb4d54b747beed138ae24337fe33d5ac9c93a8e697398`).

Summary over these 28 rows:

- canonical: oracle = Rust after **28/28**; Rust before matched **0/28**.
- DebugPrint: oracle = Rust after **28/28**; Rust before was the plain print **0/28**.
- LaTeX: oracle = Rust after **28/28**.

### 3.1 Canonical mode

| # | expression | oracle `CanonicalPrint` | Rust before `print(e,"canonical")` | Rust after | match |
|---:|---|---|---|---|:--:|
| 1 | `(x + 1)^12` | `(pow (add (rat 1 1) (sym x)) (rat 12 1))` | `(x + 1)^12 canonical` | `(pow (add (rat 1 1) (sym x)) (rat 12 1))` | ✓ |
| 2 | `exp(-x^2)` | `(fn exp (mul (rat -1 1) (pow (sym x) (rat 2 1))))` | `exp(-x^2) canonical` | `(fn exp (mul (rat -1 1) (pow (sym x) (rat 2 1))))` | ✓ |
| 3 | `factor(-x^2 - 2*x - 1)` | `(mul (rat -1 1) (pow (add (rat 1 1) (sym x)) (rat 2 1)))` | `-(x + 1)^2 canonical` | `(mul (rat -1 1) (pow (add (rat 1 1) (sym x)) (rat 2 1)))` | ✓ |
| 4 | `x - (y - a)` | `(add (sym x) (mul (rat -1 1) (add (sym y) (mul (rat -1 1) (sym a)))))` | `x - y - a canonical` | `(add (sym x) (mul (rat -1 1) (add (sym y) (mul (rat -1 1) (sym a)))))` | ✓ |
| 5 | `x - y - a` | `(add (sym x) (mul (rat -1 1) (sym a)) (mul (rat -1 1) (sym y)))` | `x - a - y canonical` | `(add (sym x) (mul (rat -1 1) (sym a)) (mul (rat -1 1) (sym y)))` | ✓ |
| 6 | `a - (x + y)` | `(add (sym a) (mul (rat -1 1) (add (sym x) (sym y))))` | `a - x + y canonical` | `(add (sym a) (mul (rat -1 1) (add (sym x) (sym y))))` | ✓ |
| 7 | `-x^2` | `(mul (rat -1 1) (pow (sym x) (rat 2 1)))` | `-x^2 canonical` | `(mul (rat -1 1) (pow (sym x) (rat 2 1)))` | ✓ |
| 8 | `(-x)^2` | `(pow (mul (rat -1 1) (sym x)) (rat 2 1))` | `(-x)^2 canonical` | `(pow (mul (rat -1 1) (sym x)) (rat 2 1))` | ✓ |
| 9 | `(-x)^3` | `(pow (mul (rat -1 1) (sym x)) (rat 3 1))` | `(-x)^3 canonical` | `(pow (mul (rat -1 1) (sym x)) (rat 3 1))` | ✓ |
| 10 | `-(x + y)` | `(mul (rat -1 1) (add (sym x) (sym y)))` | `-(x + y) canonical` | `(mul (rat -1 1) (add (sym x) (sym y)))` | ✓ |
| 11 | `(-x - 1)^2` | `(pow (add (rat -1 1) (mul (rat -1 1) (sym x))) (rat 2 1))` | `(-x - 1)^2 canonical` | `(pow (add (rat -1 1) (mul (rat -1 1) (sym x))) (rat 2 1))` | ✓ |
| 12 | `(-x)^(1/2)` | `(pow (mul (rat -1 1) (sym x)) (rat 1 2))` | `(-x)^(1/2) canonical` | `(pow (mul (rat -1 1) (sym x)) (rat 1 2))` | ✓ |
| 13 | `(-1)^x` | `(pow (rat -1 1) (sym x))` | `(-1)^x canonical` | `(pow (rat -1 1) (sym x))` | ✓ |
| 14 | `(-2)^x` | `(pow (rat -2 1) (sym x))` | `(-2)^x canonical` | `(pow (rat -2 1) (sym x))` | ✓ |
| 15 | `(-1/2)^x` | `(pow (rat -1 2) (sym x))` | `(-1/2)^x canonical` | `(pow (rat -1 2) (sym x))` | ✓ |
| 16 | `(-1.5)^x` | `(pow (rat -3 2) (sym x))` | `(-3/2)^x canonical` | `(pow (rat -3 2) (sym x))` | ✓ |
| 17 | `(-3)^(-x)` | `(pow (rat -3 1) (mul (rat -1 1) (sym x)))` | `(-3)^(-x) canonical` | `(pow (rat -3 1) (mul (rat -1 1) (sym x)))` | ✓ |
| 18 | `(1/2)^x` | `(pow (rat 1 2) (sym x))` | `(1/2)^x canonical` | `(pow (rat 1 2) (sym x))` | ✓ |
| 19 | `(2/3)^x` | `(pow (rat 2 3) (sym x))` | `(2/3)^x canonical` | `(pow (rat 2 3) (sym x))` | ✓ |
| 20 | `(-3/2)^x` | `(pow (rat -3 2) (sym x))` | `(-3/2)^x canonical` | `(pow (rat -3 2) (sym x))` | ✓ |
| 21 | `(x/y)/z` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | `x/(y*z) canonical` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | ✓ |
| 22 | `x/y/z` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | `x/(y*z) canonical` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | ✓ |
| 23 | `x/y/z/w` | `(mul (sym x) (pow (sym w) (rat -1 1)) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | `x/(w*y*z) canonical` | `(mul (sym x) (pow (sym w) (rat -1 1)) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))` | ✓ |
| 24 | `(x/y)/(z/w)` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (mul (sym z) (pow (sym w) (rat -1 1))) (rat -1 1)))` | `x/((z/w)*y) canonical` | `(mul (sym x) (pow (sym y) (rat -1 1)) (pow (mul (sym z) (pow (sym w) (rat -1 1))) (rat -1 1)))` | ✓ |
| 25 | `x/(y*z)` | `(mul (sym x) (pow (mul (sym y) (sym z)) (rat -1 1)))` | `x/(y*z) canonical` | `(mul (sym x) (pow (mul (sym y) (sym z)) (rat -1 1)))` | ✓ |
| 26 | `(x*y)/(z*w)` | `(mul (sym x) (sym y) (pow (mul (sym w) (sym z)) (rat -1 1)))` | `x*y/(w*z) canonical` | `(mul (sym x) (sym y) (pow (mul (sym w) (sym z)) (rat -1 1)))` | ✓ |
| 27 | `(x/y)*z` | `(mul (sym x) (sym z) (pow (sym y) (rat -1 1)))` | `x*z/y canonical` | `(mul (sym x) (sym z) (pow (sym y) (rat -1 1)))` | ✓ |
| 28 | `apart(1/(x^2 - 1), x)` | `(add (mul (rat -1 2) (pow (add (rat 1 1) (sym x)) (rat -1 1))) (mul (rat 1 2) (pow (add (rat -1 1) (sym x)) (rat -1 1))))` | `-1/2/(x + 1) + 1/2/(x - 1) canonical` | `(add (mul (rat -1 2) (pow (add (rat 1 1) (sym x)) (rat -1 1))) (mul (rat 1 2) (pow (add (rat -1 1) (sym x)) (rat -1 1))))` | ✓ |

### 3.2 Debug mode

| # | expression | oracle `DebugPrint` | Rust before `print(e,"debug")` | Rust after | match |
|---:|---|---|---|---|:--:|
| 1 | `(x + 1)^12` | `pow[add[rat(1), sym(x)], rat(12)]` | `(x + 1)^12 debug` | `pow[add[rat(1), sym(x)], rat(12)]` | ✓ |
| 2 | `exp(-x^2)` | `fn:exp(mul[rat(-1), pow[sym(x), rat(2)]])` | `exp(-x^2) debug` | `fn:exp(mul[rat(-1), pow[sym(x), rat(2)]])` | ✓ |
| 3 | `factor(-x^2 - 2*x - 1)` | `mul[rat(-1), pow[add[rat(1), sym(x)], rat(2)]]` | `-(x + 1)^2 debug` | `mul[rat(-1), pow[add[rat(1), sym(x)], rat(2)]]` | ✓ |
| 4 | `x - (y - a)` | `add[sym(x), mul[rat(-1), add[sym(y), mul[rat(-1), sym(a)]]]]` | `x - y - a debug` | `add[sym(x), mul[rat(-1), add[sym(y), mul[rat(-1), sym(a)]]]]` | ✓ |
| 5 | `x - y - a` | `add[sym(x), mul[rat(-1), sym(a)], mul[rat(-1), sym(y)]]` | `x - a - y debug` | `add[sym(x), mul[rat(-1), sym(a)], mul[rat(-1), sym(y)]]` | ✓ |
| 6 | `a - (x + y)` | `add[sym(a), mul[rat(-1), add[sym(x), sym(y)]]]` | `a - x + y debug` | `add[sym(a), mul[rat(-1), add[sym(x), sym(y)]]]` | ✓ |
| 7 | `-x^2` | `mul[rat(-1), pow[sym(x), rat(2)]]` | `-x^2 debug` | `mul[rat(-1), pow[sym(x), rat(2)]]` | ✓ |
| 8 | `(-x)^2` | `pow[mul[rat(-1), sym(x)], rat(2)]` | `(-x)^2 debug` | `pow[mul[rat(-1), sym(x)], rat(2)]` | ✓ |
| 9 | `(-x)^3` | `pow[mul[rat(-1), sym(x)], rat(3)]` | `(-x)^3 debug` | `pow[mul[rat(-1), sym(x)], rat(3)]` | ✓ |
| 10 | `-(x + y)` | `mul[rat(-1), add[sym(x), sym(y)]]` | `-(x + y) debug` | `mul[rat(-1), add[sym(x), sym(y)]]` | ✓ |
| 11 | `(-x - 1)^2` | `pow[add[rat(-1), mul[rat(-1), sym(x)]], rat(2)]` | `(-x - 1)^2 debug` | `pow[add[rat(-1), mul[rat(-1), sym(x)]], rat(2)]` | ✓ |
| 12 | `(-x)^(1/2)` | `pow[mul[rat(-1), sym(x)], rat(1/2)]` | `(-x)^(1/2) debug` | `pow[mul[rat(-1), sym(x)], rat(1/2)]` | ✓ |
| 13 | `(-1)^x` | `pow[rat(-1), sym(x)]` | `(-1)^x debug` | `pow[rat(-1), sym(x)]` | ✓ |
| 14 | `(-2)^x` | `pow[rat(-2), sym(x)]` | `(-2)^x debug` | `pow[rat(-2), sym(x)]` | ✓ |
| 15 | `(-1/2)^x` | `pow[rat(-1/2), sym(x)]` | `(-1/2)^x debug` | `pow[rat(-1/2), sym(x)]` | ✓ |
| 16 | `(-1.5)^x` | `pow[rat(-3/2), sym(x)]` | `(-3/2)^x debug` | `pow[rat(-3/2), sym(x)]` | ✓ |
| 17 | `(-3)^(-x)` | `pow[rat(-3), mul[rat(-1), sym(x)]]` | `(-3)^(-x) debug` | `pow[rat(-3), mul[rat(-1), sym(x)]]` | ✓ |
| 18 | `(1/2)^x` | `pow[rat(1/2), sym(x)]` | `(1/2)^x debug` | `pow[rat(1/2), sym(x)]` | ✓ |
| 19 | `(2/3)^x` | `pow[rat(2/3), sym(x)]` | `(2/3)^x debug` | `pow[rat(2/3), sym(x)]` | ✓ |
| 20 | `(-3/2)^x` | `pow[rat(-3/2), sym(x)]` | `(-3/2)^x debug` | `pow[rat(-3/2), sym(x)]` | ✓ |
| 21 | `(x/y)/z` | `mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | `x/(y*z) debug` | `mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | ✓ |
| 22 | `x/y/z` | `mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | `x/(y*z) debug` | `mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | ✓ |
| 23 | `x/y/z/w` | `mul[sym(x), pow[sym(w), rat(-1)], pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | `x/(w*y*z) debug` | `mul[sym(x), pow[sym(w), rat(-1)], pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]` | ✓ |
| 24 | `(x/y)/(z/w)` | `mul[sym(x), pow[sym(y), rat(-1)], pow[mul[sym(z), pow[sym(w), rat(-1)]], rat(-1)]]` | `x/((z/w)*y) debug` | `mul[sym(x), pow[sym(y), rat(-1)], pow[mul[sym(z), pow[sym(w), rat(-1)]], rat(-1)]]` | ✓ |
| 25 | `x/(y*z)` | `mul[sym(x), pow[mul[sym(y), sym(z)], rat(-1)]]` | `x/(y*z) debug` | `mul[sym(x), pow[mul[sym(y), sym(z)], rat(-1)]]` | ✓ |
| 26 | `(x*y)/(z*w)` | `mul[sym(x), sym(y), pow[mul[sym(w), sym(z)], rat(-1)]]` | `x*y/(w*z) debug` | `mul[sym(x), sym(y), pow[mul[sym(w), sym(z)], rat(-1)]]` | ✓ |
| 27 | `(x/y)*z` | `mul[sym(x), sym(z), pow[sym(y), rat(-1)]]` | `x*z/y debug` | `mul[sym(x), sym(z), pow[sym(y), rat(-1)]]` | ✓ |
| 28 | `apart(1/(x^2 - 1), x)` | `add[mul[rat(-1/2), pow[add[rat(1), sym(x)], rat(-1)]], mul[rat(1/2), pow[add[rat(-1), sym(x)], rat(-1)]]]` | `-1/2/(x + 1) + 1/2/(x - 1) debug` | `add[mul[rat(-1/2), pow[add[rat(1), sym(x)], rat(-1)]], mul[rat(1/2), pow[add[rat(-1), sym(x)], rat(-1)]]]` | ✓ |

### 3.3 The full C# corpora, row by row

| corpus | rows | oracle canonical = Rust | oracle DebugPrint = Rust | oracle pretty = Rust (pre-existing) |
|---|---:|---:|---:|---:|
| `PrettyParenthesesTests.Corpus` | 48 | **46** | **46** | 37 |
| `LatexPrinterTests.Corpus` | 49 | **49** | **49** | 38 |

The two non-matching pretty-corpus rows are the two pre-existing structural divergences below,
not printing failures.

## 4. Gates and commands

```text
cargo test --workspace
  58 + 67 + 125 + 7 + 801 + 134 + 2,345 + 162 + 1 + 47 = 3,747 passed; 0 failed; exit 0

cargo run --release -p lovelace-corpus        (exit 1, the two pre-existing failures)
  FAIL sym/071    FAIL sym/076
  corpus: 212 passed, 2 failed, 214 total

powershell -ExecutionPolicy Bypass -File parity/gate-fixtures.ps1
  oracle: 20/20
  rust: 20/20
  PASS: oracle 20/20, rust 20/20

node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize    (baseline -> after)
  generated:   154 match / 46 mismatch / 0 error  ->  154 / 46 / 0
  regressions: 600 match / 1,152 mismatch / 0 error -> 600 / 1,152 / 0
```

The differential run directory is `parity/diff/runs/20261110T213641Z-seed1`.

## 5. Divergences and limits

1. **Two canonical divergences remain in the 48-row corpus.** `integrate_full(exp(-x^2), x)`
   is an `(integ ...)` node in the reference and an `(fn integrate ...)` value here (the crate has
   no Integral node), and `diff(exp(x)*x^2, x)` has a nested `(mul 2 (mul x (exp x)))` factor
   shape versus the reference's flat product. Both rows still satisfy the ported property
   (their own canonical form round-trips and their Debug rendering is coherent); only the
   oracle-vs-Rust text differs.
2. **Pretty stays at 37/48 and 38/49.** The round-11 divergences (nested subtraction, flat
   division, negative exponents) are untouched; the `RoundTrip_Pretty` rows are out of this
   cluster.
3. **Canonical grammar is the Rust vocabulary plus `(int)`.** `real`/`cplx` leaves and the
   derivative/integral/piecewise/logic/order nodes have no Rust expression model, so the parser
   refuses them with a named reason instead of inventing a representation. This does not touch
   any of the 97 corpus rows (their canonical text uses only supported tags).
4. **The mode names are a language-surface extension.** The reference selects a mode with an
   enum, so the string form and its unknown-mode refusal have no oracle script counterpart; the
   dispatch arms, canonical text, debug text and LaTeX text they route to are all oracle-verified.
   `print(symbolic, "arbitrary text")` is now interpreted as a mode name and refused when
   unknown; this is the one intentional behaviour change of the extension and it is documented
   here and in the builtin comment.
5. `rustfmt`/`clippy` are not installed for this toolchain (same as rounds 11–15), so the D10
   Rust-quality check is unavailable, not passed.

## 6. Files changed

| file | change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | + `PrintMode`, `UnknownPrintMode`, `Expr::print`/`print_named`/`debug_print`, `canonical_parse` + grammar, canonical-order `Div` fix |
| `rust/lovelace-symbolics/src/printing_round16.rs` | new: 10 model-level tests (round-trip corpus, refusals, debug strings, dispatch, unknown mode) |
| `rust/lovelace-lang/src/builtins.rs` | `print` mode dispatch and unknown-mode argument refusal |
| `rust/lovelace-lang/src/printing_round16.rs` | new: 7 language tests over the 48+49 corpus rows and the 28-expression oracle table |
| `rust/lovelace-lang/src/lib.rs` | registers the new test module |
| `docs/goal-parity-contract/round-16/symbolics-printing.md` | this report |

The 145 census rows are not written to `parity/manifest-map.json` here: `parity/` is outside this
task's scope. The evidence a mapping needs is the test names above, each of which iterates the
C# theory's own corpus.
