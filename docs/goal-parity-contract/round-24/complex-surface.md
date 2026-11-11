# Round 24 - the Complex value surface

Status: **implemented in tree; gates re-run**. Scope: `rust/lovelace-lang/src` (parser, builtins,
the new `complex` module, structured projection is unchanged) and tests. No `.cs`, fixture,
corpus, or `parity/` source/configuration was edited; the differential harness wrote only its run
directory. No documentation outside this report was edited.

## 1. Premise corrections (probed, not assumed)

The brief says "in Rust `1+2i` is a parse error and `complex` is a domain constant; the C#
language has a first-class arbitrary-precision Complex value". The probes confirm the first half
and correct the second:

1. **The oracle has no complex literal grammar.** `1+2i`, `2i`, `3i`, `(1+2i)*(3-4i)` and
   `abs(3+4i)` are all `ParseError`s; `i`, `i*i` and `-i` are `InvalidOperation` "Undefined
   variable 'i'." There is no form the oracle accepts that the Rust parser must newly accept.
   What was wrong on the Rust side was the *refusal*: the Rust parser allowed whitespace to
   separate statements, so `2i` silently meant the two statements `2` and `i` (and `x = 3; 2x`
   answered `3`). This round makes the separator mandatory and reproduces the reference sentences
   and positions byte for byte.
2. **`complex` is a zero-arity BUILTIN, not only a variable.** `complex` reads the Domain value
   and `complex(1, 2)` is the declared-arity refusal `complex(): expected 0 arguments; got 2.`
   The same holds for `real`, `integer` and `rational`. Rust previously answered
   `Unknown function 'complex'.`
3. **The 121 `Lovelace.Complex.Tests` rows are library-level, not language-level.** 69 of them
   assert `ComplexMath` (Exp/Ln/Sin/Cos/Tan/Atan/Atan2/Acosh/Asinh/Atanh/Pow/Sqrt/Log) with
   Real digit-provenance rules; 52 assert the `Complex` value type, the fixed-width
   `LComplex64`/`LComplex128` forms, and `DenseArray<Complex>`. This round implements the value
   surface (and the language bridge that was broken); `ComplexMath` remains out of reach because
   the Rust `Real` has no exp/ln/trig kernel at all (see section 8).

## 2. The literal grammar discovered

Probes (`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval <script>
--omit-functions`):

| Input | Oracle |
|---|---|
| `1+2i` | `ParseError: Expected ';' or end of input but found 'i' at position 3.` |
| `2i` | `ParseError: ... found 'i' at position 1.` |
| `3i` | `ParseError: ... found 'i' at position 1.` |
| `i` | `InvalidOperation: Undefined variable 'i'.` |
| `i*i` | `InvalidOperation: Undefined variable 'i'.` |
| `-i` | `InvalidOperation: Undefined variable 'i'.` |
| `(1+2i)*(3-4i)` | `ParseError: Expected 'RParen' but found 'i' at position 4.` |
| `abs(3+4i)` | `ParseError: Expected 'RParen' but found 'i' at position 7.` |
| `complex` | the Domain value `complex` |
| `complex(1, 2)` | `InvalidArgument: complex(): expected 0 arguments; got 2.` |
| `sqrt(-1)` | Symbolic `i` (the symbolic complex path, not a Complex value) |

So the complex-value surface is:

* **No literal.** A complex value is produced only by a DSP builtin (`fft`, `dft`, `filter`,
  `conv`, `impulse`, `step`, `delay`, `movingavg`, `noise`, `exponential`, `powerseries`,
  `cosine`) and rendered by the reference `Complex.ToString` rules: `re` if the imaginary part is
  zero, `imi` if the real part is zero, else `re + |im|i` / `re - |im|i`.
* **Text forms exist only in the library.** `Complex.Parse` (and `LComplex64/LComplex128.Parse`)
  accept `2`, `4i`, `-3i`, `i`, `-i`, `1.5+0.5i`, `1.5 - 0.5i`, `-2+3i`: trim; if the text does
  not end in `i`, the whole text is the real part; a bare `i` body (an empty body, or one that is
  just `+` or `-`) is the imaginary unit with that sign; otherwise the last `+`/`-` that is not a
  leading sign splits the components, and an empty imaginary-digits field means `1`. These are
  ported and tested.
* **Statement separation.** The host normalizes top-level newlines to `;`
  (`ScriptSource.ToSemicolonStatements`), and the parser requires one separator between
  statements: `Expected ';' or end of input but found '<text>' at position <pos>.` at top
  level and `Expected ';' or '}' but found '<text>' at position <pos>.` in a block. A stray
  `}` at top level is the primary-token refusal instead. `Expect` publishes the reference token
  kind (`RParen`, `RBrace`, `Comma`, ...), not the punctuation glyph.

## 3. What was implemented

| File | Change |
|---|---|
| `rust/lovelace-lang/src/syntax.rs` | Mandatory statement separators in `program()` and block bodies, with the reference sentences/positions; `expect_punct` now names the token kind; function-name, parameter, `for`-variable and `in` messages ported; the `if` handler no longer steals the enclosing statement separator. |
| `rust/lovelace-lang/src/builtins.rs` | `re`/`im`/`conj` are the DspPlugin `MapElements` shape (scalar through ToComplex; arrays mapped elementwise and flattened to a rank-1 vector); `conj` now negates the imaginary part; `abs` of a complex scalar uses the engine precision instead of 64 digits; `abs` of an array is the elementwise magnitude spectrum with the reference DType refusal; `real`/`integer`/`rational`/`complex` are zero-arity domain builtins; a domain payload is named `MathDomain`. |
| `rust/lovelace-lang/src/complex.rs` (new) | The `Lovelace.Complex` value surface: `Complex` (arbitrary-precision components, add/sub/mul/div/neg/scale, conjugate, `|z|^2`, `|z|`, reciprocal, integer power, component-wise equality, Parse/ToString, exactness, and the reference fixed-width fast-path dispatch at a budget <= 37 places); `LComplex64`/`LComplex128` (19/38 significant exact decimals, checked arithmetic, conversion, conjugate, reciprocal, promotion) and `LRealPromoteException`. |
| `rust/lovelace-lang/src/lib.rs` | Registers `pub mod complex;` and the round-24 test module. |
| `rust/lovelace-lang/src/complex_round24.rs` (new) | 24 regression tests pinning every probe below. |

CPLX mapping: CPLX-001 arithmetic/rendering (value type + `ToString`/`Parse`, language re/im/conj/abs);
CPLX-002 arbitrary precision (74-digit products and 1000-place magnitudes without rounding);
CPLX-003 fixed-width optimized representations (LComplex64/LComplex128 and the <=37-place fast path);
CPLX-004 promotion without saturation (checked fixed-width arithmetic promotes; the exact path
answers); CPLX-005 optimized vs arbitrary (the fast path is compared with the exact result in
three tests); CPLX-006 real-to-complex coercion and domain behavior (`re`/`im`/`conj` accept
Natural/Integer/Real/Complex, `abs` only Complex arrays, domain constants refused by the bridge).

## 4. Before/after oracle-vs-Rust probes

The 29 probes captured in `rust/target/tmp/round24-probes.json` now match the oracle on
ok/code/category/message/position/kind/display/structured (29/29 identical after canonical object
ordering); the two extra rows below (`conj`/`abs` of one FFT element) were measured separately.
The table marks what was wrong before this round; the rest are controls that already matched and
stayed matched.

| # | Input | Before (Rust, pre-fix) | Oracle / after (Rust) |
|---|---|---|---|
| 1 | `1+2i` | `InvalidOperation: Undefined variable 'i'.` at 3 | `ParseError: Expected ';' or end of input but found 'i' at position 3.` |
| 2 | `2i` | `Undefined variable 'i'.` at 1 | `ParseError: ... found 'i' at position 1.` (was two statements) |
| 3 | `i` | `Undefined variable 'i'.` | same (control) |
| 4 | `(1+2i)*(3-4i)` | `ParseError: Expected ')'.` at 4 | `Expected 'RParen' but found 'i' at position 4.` |
| 5 | `abs(3+4i)` | `Expected ')'.` at 7 | `Expected 'RParen' but found 'i' at position 7.` |
| 6 | `re(1+2i)` | `Expected ')'.` at 6 | `Expected 'RParen' but found 'i' at position 6.` |
| 7 | `im(1+2i)` | `Expected ')'.` at 6 | `Expected 'RParen' but found 'i' at position 6.` |
| 8 | `conj(1+2i)` | `Expected ')'.` at 8 | `Expected 'RParen' but found 'i' at position 8.` |
| 9 | `sqrt(-1)` | Symbolic `i` | Symbolic `i` (control) |
| 10 | `complex` | Domain `complex` | Domain `complex` (control) |
| 11 | `complex(1,2)` | `UnsupportedOperation: Unknown function 'complex'. ...` | `InvalidArgument: complex(): expected 0 arguments; got 2.` |
| 12 | `fft([1,2,3,4])` | `[10, -2 + 2i, -2, -2 - 2i]` | same, structured Complex re/im/exact (control) |
| 13 | `re(fft([1,2,3,4]))` | `[10, -2 + 2i, -2, -2 - 2i]` (input returned) | `[10, -2, -2, -2]`, Real elements |
| 14 | `im(fft([1,2,3,4]))` | `0` (Natural) | `[0, 2, 0, -2]`, Real elements |
| 15 | `conj(fft([1,2,3,4]))` | `[10, -2 + 2i, -2, -2 - 2i]` (input returned) | `[10, -2 - 2i, -2, -2 + 2i]` |
| 16 | `abs(fft([1,2,3,4]))` | `DomainError: abs needs a number, but got 'Vector'.` | magnitude vector; structured Real elements and exact flags `[true,false,true,false]` are IDENTICAL to the oracle; the displayed digits differ (see the two residual facts below) |
| 17 | `abs(fft([1,2]))` | same refusal | `[3, 1]` (all exact; display and structure identical) |
| 18 | `re([1,2])` | `[1, 2]` with Natural elements | `[1, 2]` with Real elements (numerator/denominator) |
| 19 | `im([1,2])` | `0` (Natural) | `[0, 0]`, Real elements |
| 20 | `conj([1,2])` | `[1, 2]` with Natural elements (input returned) | `[1, 2]` with Complex `re/im` elements |
| 21 | `re(5)` | Natural `5` | Real `5` |
| 22 | `im(5)` | Natural `0` | Real `0` |
| 23 | `conj(5)` | Natural `5` | Complex `5 + 0i` |
| 24 | `abs([1,2])` | `abs needs a number, but got 'Vector'.` | `abs() on arrays requires a Complex array (got DType.Natural); use abs() on individual scalars for Natural/Integer/Real.` |
| 25 | `conj(complex)` | returned the Domain unchanged | `DSP builtins expect numeric/complex elements, but got 'MathDomain'.` |
| 26 | `conj(fft([1,2,3,4])[1])` | `-2 + 2i` | `-2 - 2i` |
| 27 | `abs(fft([1,2,3,4])[1])` | 64-digit magnitude (`...4759`) | 1002-character value equal to the oracle's `structured.value`; `exact:false` |
| 28 | `2 pi` | evaluated `pi` | `ParseError: Expected ';' or end of input but found 'pi' at position 2.` |
| 29 | `x = 3; 2x` | answered `3` | `ParseError: ... found 'x' at position 8.` |
| 30 | `1 2` | answered `2` | `ParseError: ... found '2' at position 2.` |
| 31 | `{1 2}` | answered `2` | `ParseError: Expected ';' or '}' but found '2' at position 3.` |

Two residual display facts, measured:

* `abs(fft([1,2,3,4]))` has **structured-equal** output (same `value` strings and
  `[true,false,true,false]` exactness) but the oracle display is 215 characters and the Rust
  display 2015: the reference `Real.ToString` truncates a non-periodic value at
  `Real.DisplayDecimalPlaces` (100 by default), while the Rust `Real::to_string` prints every
  stored place. This is a `rust/lovelace-real` renderer gap that predates the round and is outside
  this round's scope; the language value is the same.
* The reference number scanner consumes a following `(...)` as a periodic suffix even when it is
  empty, so `2(x+1)` is a separator error at `x` in the oracle. That lexer quirk is not ported;
  Rust answers `Only a named function can be called.` for that shape. No test, corpus case, or
  fixture in the workspace exercises it.

## 5. Library `Complex` surface (the 52 value-type rows)

`complex.rs` ports, and `complex_round24.rs` pins:

* `Complex::new/from_real/from_i64/zero/one/i/pi/e`, `re()/im()/parts()`, `is_exact()`.
* `add`, `sub`, `mul`, `div`, `neg`, `scale`, `conjugate`, `magnitude_squared`, `magnitude(digits)`,
  `reciprocal(digits)`, `pow_int(n, digits)`, component-wise `PartialEq`, `Display`, `parse`.
* The reference `Complex.Binary` fast path: at most 37 computation places, both operands are tried
  as 19-digit then 38-digit exact decimals, and a promotion falls through to the exact
  arbitrary-precision operation. `add_at`/`sub_at`/`mul_at`/`div_at` expose the budget; the
  plain operators use the reference default of 1000 places (so they take the slow path).
* `LComplex64` (19 digits) and `LComplex128` (38 digits): `parse`, `try_from_complex`,
  `to_complex`, `checked_add/sub/mul/div`, `neg`, `conjugate`, `magnitude_squared`, `reciprocal`,
  equality and display. Overflow answers `Err(LRealPromoteException)` and never rounds;
  `Complex::*_at` then takes the exact path.

Stated deviation: the fixed-width forms do not model the reference's periodic decimal expansions.
A periodic component does not convert (`try_from_complex` answers `None`), and a fixed-width
quotient that would need a period promotes instead of storing it. The arbitrary-precision type
computes the exact result, so no digit is lost (CPLX-004).

## 6. Tests

`rust/lovelace-lang/src/complex_round24.rs`, registered in `lib.rs`, 24 tests:

1. `a_juxtaposed_complex_literal_is_the_oracles_separator_refusal`
2. `a_pure_imaginary_literal_is_refused_at_its_own_position`
3. `a_bare_i_is_an_undefined_variable_not_a_literal`
4. `complex_forms_inside_parentheses_are_the_rparen_refusal`
5. `complex_is_a_domain_constant_and_complex_takes_no_arguments`
6. `adjacent_statements_need_a_separator_like_the_oracle`
7. `fft_publishes_complex_elements_with_re_im_and_exactness`
8. `re_im_and_conj_map_a_complex_signal_like_the_oracle`
9. `scalar_re_im_conj_promote_every_numeric_kind`
10. `abs_of_a_complex_value_is_its_magnitude_at_the_ambient_precision`
11. `abs_of_a_complex_array_is_the_magnitude_spectrum`
12. `abs_of_a_non_complex_array_names_the_inferred_dtype`
13. `the_domain_constant_is_refused_by_the_complex_bridge`
14. `complex_arithmetic_is_component_wise_and_exact`
15. `complex_magnitude_is_the_three_four_five_triangle`
16. `complex_parse_and_display_round_trip_the_reference_forms`
17. `complex_exactness_is_both_parts`
18. `limited_precision_takes_the_fixed_path_and_stays_exact` (three `ComplexTests` rows)
19. `complex_integer_powers_are_exact`
20. `lcomplex64_parses_adds_and_conjugates_like_the_reference`
21. `lcomplex64_reciprocal_multiplies_to_one`
22. `lcomplex64_converts_to_and_from_the_arbitrary_precision_type`
23. `lcomplex64_promotes_on_overflow_instead_of_rounding`
24. `lcomplex128_covers_the_same_surface_at_38_digits`

## 7. Gates (after)

| Gate | Observed | Log |
|---|---|---|
| `cargo test --workspace` | **4 096 passed, 0 failed** (4 072 + 24), 19 suites | `rust/target/tmp/round24-cargo-after.log` |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** - only `sym/071`, `sym/076` (unchanged) | `rust/target/tmp/round24-corpus-after.log` |
| `node parity/tools/gate-fixtures.mjs` | oracle 20/20, rust 20/20, **PASS** | `rust/target/tmp/round24-fixtures-after.log` |
| `node parity/tools/gate-cli.mjs` | CLI matrix **16/16**, **PASS** | `rust/target/tmp/round24-cli-after.log` |
| `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated **154 match / 46 mismatch / 0 error**; replayed **600 match / 1 152 mismatch / 0 error**; **new=0**; delta **0** against the pre-fix row | `rust/target/tmp/round24-diff-after.log` |

Baseline before the round (same commands): 4 072 passed; corpus 212/214 (`sym/071`,
`sym/076`); fixtures 20/20 + 20/20; CLI 16/16; differential generated 154/46/0, replayed
600/1152/0, new=0. The round adds 24 tests and changes no gate count; it removes no test and
weakens no assertion.

## 8. What remains

1. **`ComplexMath` (69 of the 121 manifest rows).** Exp/Ln/Sin/Cos/Tan/Atan/Atan2/Asin/Acos/
   Asinh/Acosh/Atanh/Pow/Sqrt/Log with the reference digit-provenance rules need arbitrary-
   precision exp/ln/trig over `Real`. The Rust `Real` has no transcendental kernel (the language
   `sin`/`cos`/... stay symbolic), so those rows cannot be ported without a separate numeric
   round. They stay deferred; `parity/manifest.toml` was not touched (out of scope).
2. **The other 52 rows** now have a Rust value surface to map to (`complex.rs` and the 15
   value-type tests here), but the manifest dispositions were not rewritten in this round.
3. **Real display truncation.** `Real::to_string` prints all stored places while the oracle
   truncates at `DisplayDecimalPlaces` (100). This is why the non-exact `abs(fft(...))` display
   differs although the structured projection is equal; the fix belongs to `rust/lovelace-real`,
   outside this round's scope.
4. **Number-scanner `(` quirk** (`2(x+1)`) and newline-inside-block permissiveness (the Rust
   tokenizer treats every newline as a statement separator, the C# host only rewrites top-level
   ones) remain as lexer/host differences. No workspace test, corpus case, or fixture depends on
   either.
5. **Periodic fixed-width expansions** are not modelled; they promote to the exact
   arbitrary-precision path (documented in section 5).
