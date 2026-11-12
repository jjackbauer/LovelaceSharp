# Round 67 — the polynomial algebra block (content / primitive part / gcd / lcm / resultant / discriminant / factor)

**Status: complete.** The five deferred `PolynomialsApiTests` rows in the round-66 ranked scope
(section 5, the "5 rows" polynomial family) are now backed by a Rust façade with the reference's
exact values, and the edges the reference decides — zero polynomial, constants, non-monic inputs,
negative leading coefficients — are covered by their own tests. The C# oracle and the Rust façade
agree on all 59 probe lines (the five row scripts plus every edge probe).

The five census rows were deferred because no Rust API existed (round 45 and round 38 measured the
same absence; round 66 section 5 carried the family as 5 of the 130 UNRUNNABLE rows). This round
adds that surface, corrects the two square-free edges the factor algorithm depends on, and maps each
row to its own test. No `.cs`, fixture, `parity/` file or other document was edited; the only
document written is this report.

## 1. Result

| Measure | Before | After |
|---|---|---:|
| Census rows in this family deferred | 5 | 0 |
| Rust `polynomials` façade | absent | `content`, `primitive_part`, `gcd`, `lcm`, `resultant`, `discriminant`, `factor` |
| C# oracle vs Rust probe lines | n/a (no API) | **59 / 59 identical** |
| `square_free_univariate` constant / non-monic / negative-lc edges | constant `[]`; factors carried the input leading coefficient | constant `[(1, 1)]`; every factor monic |
| New Rust tests | 0 | 16 (5 rows + 11 edge) |
| `cargo test -p lovelace-symbolics` | 152 passed | **168 passed, 0 failed** |

## 2. The five census rows and their tests

All tests live in `rust/lovelace-symbolics/src/round67_polynomial_algebra.rs`; the production
surface is `rust/lovelace-symbolics/src/polynomials.rs`.

| Census row | Rust test | C# contract asserted |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Content_PrimitivePart_SplitCoefficients#0` | `content_primitive_part_split_coefficients` | `2x^2 + 4x + 6` has content `2` and primitive part `[3, 2, 1]` (`x^2 + 2x + 3`). |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Gcd_Lcm_AreConsistent#0` | `gcd_lcm_are_consistent` | `gcd(x^2 - 1, x^2 + 2x + 1)` is the monic `x + 1`; `gcd * lcm` equals the product of the inputs. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Resultant_MatchesKnownValues#0` | `resultant_matches_known_values` | `res(x^2 - 2, x - 1) = -1`, `res(x - 1, x - 1) = 0`, `res(x - 1, x - 2) = 1`. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Discriminant_MatchesKnownValues#0` | `discriminant_matches_known_values` | `disc(x^2 + 3x + 2) = 1`, `disc(x^2 - 2) = 8`. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Factor_QuarticWithRationalRoots_FourLinearFactors#0` | `factor_quartic_with_rational_roots_four_linear_factors` | `x^4 - 5x^2 + 4` has content `1` and four distinct monic linear factors, multiplicities `1`; the product reconstructs the input. The exact factor order `[2, 1], [1, 1], [-1, 1], [-2, 1]` is also pinned (the oracle's ascending rational-root order). |

## 3. Oracle probe — before and after

### 3.1 Method

A temporary console project **outside the tracked tree** (`rust/target/tmp/poly-probe`, a
`ProjectReference` to `Lovelace.Symbolics`) called the reference's `Polynomials` and
`Factoring` static API on each row script and on the selected edge cases: zero polynomial,
positive and negative constants, non-monic inputs, negative leading coefficients, fractions,
zero/constant gcd/lcm/resultant operands, linear and constant discriminants, and repeated roots.
It printed the content, primitive-part coefficient vector, gcd/lcm coefficient vectors, resultant,
discriminant, and `factor` content/count/factors.

The same coefficient vectors (ascending degree) were then run through the Rust façade; both logs
were filtered to the 59 probe lines and compared line by line. **59 / 59 lines identical** after
the probe's own multiplicity print form was normalised; there is no remaining differing value.

Before this round the five rows had no API at all, and the square-free edge probe below measured
the two behavioural gaps the factor algorithm inherits.

### 3.2 Before — `square_free_univariate` edge probe

Run through the existing public API (raw output of a temporary test before the fix):

| Input (ascending) | Polynomial | Before | After (reference) |
|---|---|---|---|
| `[0]` | `0` | `[]` | `[]` |
| `[6]` | `6` | `[]` | `[([1], 1)]` |
| `[-6]` | `-6` | `[]` | `[([1], 1)]` |
| `[6, 4, 2]` | `2x^2 + 4x + 6` | `[([6, 4, 2], 1)]` | `[([3, 2, 1], 1)]` |
| `[1, 0, -1]` | `-x^2 + 1` | `[([1, 0, -1], 1)]` | `[([-1, 0, 1], 1)]` |
| `[0, 1, 2]` | `2x^2 + x` | `[([0, 1, 2], 1)]` | `[([0, 1/2, 1], 1)]` |
| `[2, -4, 2]` | `2(x - 1)^2` | `[([-1, 1], 2)]` | `[([-1, 1], 2)]` (unchanged) |
| `[0, 0, -1, 1]` | `x^3 - x^2` | `[([-1, 1], 1), ([0, 1], 2)]` | `[([-1, 1], 1), ([0, 1], 2)]` (unchanged) |

### 3.3 After — the full 59-line oracle/Rust probe

Each line is the oracle's value and the Rust value (they are equal). Coefficient vectors are
ascending; `[] (zero)` is the zero polynomial; `FACTOR` prints `content`, factor count, and
each monic factor with its multiplicity.

```text
CONTENT 2x^2+4x+6 = 2
PRIM    2x^2+4x+6 = [3, 2, 1]
CONTENT zero = 1
PRIM    zero = [] (zero)
CONTENT 6 = 6
PRIM    6 = [1]
CONTENT -6 = 6
PRIM    -6 = [-1]
CONTENT -2x^2+4x+6 = 2
PRIM    -2x^2+4x+6 = [3, 2, -1]
CONTENT x/2+1/3 = 1/6
PRIM    x/2+1/3 = [2, 3]
CONTENT 2x+2 = 2
PRIM    2x+2 = [1, 1]
GCD x^2-1, x^2+2x+1 = [1, 1]
LCM x^2-1, x^2+2x+1 = [-1, -1, 1, 1]
GCD 0, x^2-1 = [-1, 0, 1]
GCD x^2-1, 0 = [-1, 0, 1]
GCD 0, 0 = [] (zero)
LCM 0, x^2-1 = [] (zero)
GCD 2x+2, 4x+4 = [1, 1]
LCM 2x+2, 4x+4 = [8, 8]
GCD -x^2+1, x+1 = [1, 1]
GCD 6, 4 = [1]
LCM 6, 4 = [24]
GCD 6, 0 = [1]
RES x^2-2, x-1 = -1
RES x-1, x-1 = 0
RES x-1, x-2 = 1
RES 0, x-1 = 0
RES x-1, 0 = 0
RES 0, 0 = 0
RES 2x^2-4, x-1 = -2
RES -x^2+2, x-1 = 1
RES 3, x-1 = 3
RES x-1, 3 = 3
RES x^2-2, 3 = 9
RES 3, 4 = 1
DISC x^2+3x+2 = 1
DISC x^2-2 = 8
DISC 0 = 0
DISC 6 = 1
DISC -6 = 1
DISC 2x^2+6x+4 = 4
DISC -2x^2+6x-4 = 4
DISC x-1 = 1
DISC 3x-3 = 1
DISC x^3-x = 4
DISC 2x^3-2x = 64
FACTOR x^4-5x^2+4 = content=1 count=4 factors=[[2, 1]^1; [1, 1]^1; [-1, 1]^1; [-2, 1]^1]
FACTOR 0 = content=0 count=0 factors=[]
FACTOR 6 = content=6 count=1 factors=[[1]^1]
FACTOR -6 = content=6 count=1 factors=[[-1]^1]
FACTOR -x^2+1 = content=-1 count=2 factors=[[1, 1]^1; [-1, 1]^1]
FACTOR 2x^2+x = content=2 count=2 factors=[[1/2, 1]^1; [0, 1]^1]
FACTOR 2x^2-4x+2 = content=2 count=1 factors=[[-1, 1]^2]
FACTOR x^3-x^2 = content=1 count=2 factors=[[-1, 1]^1; [0, 1]^2]
FACTOR 4x^2-4 = content=4 count=2 factors=[[1, 1]^1; [-1, 1]^1]
FACTOR 2x^4-10x^2+8 = content=2 count=4 factors=[[2, 1]^1; [1, 1]^1; [-1, 1]^1; [-2, 1]^1]
```

## 4. Fixes

### 4.1 New façade: `rust/lovelace-symbolics/src/polynomials.rs`

The reference's `Polynomials` static class is ported over the crate's existing ascending
coefficient-vector carrier (the same carrier as `poly_coeffs` / `poly_div` /
`square_free_univariate`), so the five rows become library-assertable:

| Rust function | Reference branch | Exact semantics |
|---|---|---|
| `content(coeffs) -> Rat` | `Polynomials.Content` -> `Factoring.PrimitivePart` | gcd of the nonzero coefficient magnitudes over the denominators; zero polynomial answers `1`. |
| `primitive_part(coeffs) -> Vec<Rat>` | `Polynomials.PrimitivePart` | divides every coefficient by the content; zero stays zero; a negative leading coefficient stays in the primitive part. |
| `gcd(a, b) -> Vec<Rat>` | `Polynomial.GcdUnivariate` | monic Euclidean gcd over Q; zero operands follow the reference (`gcd(0, p)` is the monic `p`, `gcd(0, 0)` is zero). |
| `lcm(a, b) -> Vec<Rat>` | `Polynomials.Lcm` | zero if either operand is zero, else `(a / gcd(a, b)) * b`; for constants this is `a*b/g` (`lcm(6, 4) = 24`). |
| `resultant(f, g) -> Rat` | `Polynomials.Resultant` | determinant of the Sylvester matrix by exact fraction-free Bareiss elimination; zero operand -> `0`, two constants -> `1`, constant against degree-n -> `c^n`. |
| `discriminant(f) -> Rat` | `Polynomials.Discriminant` | `(-1)^(n(n-1)/2) * res(f, f') / lc(f)`; zero -> `0`, constant -> `1`, linear -> `1`. |
| `factor(coeffs) -> PolynomialFactors` | `Factoring.FactorPoly` | content + Yun square-free factors + one monic linear factor per rational root; residual factors whole; zero -> content `0` and no factors. |

The resultant/determinant and discriminant steps are the reference's own algorithm, not a
reconstruction from roots, so the zero/constant/non-monic/negative-lc cases fall out exactly as the
oracle prints them.

### 4.2 `square_free_univariate` corrections

Two edge behaviours diverged from `Polynomial.SquareFreeUnivariate` and are fixed in
`rust/lovelace-symbolics/src/lib.rs`:

1. **Constant input.** The reference normalizes a nonzero constant to the unit and returns
   `[(1, 1)]`; the Rust function returned `[]` (it treated every `len <= 1` input as the zero
   polynomial). Zero still returns `[]`.
2. **Monic factors.** The reference normalizes every emitted factor; the Rust decomposition
   emitted the input's leading coefficient on a square-free factor (`2x^2 + 4x + 6` came back as
   `[6, 4, 2]`, `-x^2 + 1` as `[1, 0, -1]`). Each emitted factor is now passed through the
   crate's monic normalization, so the decomposition matches the reference (`[3, 2, 1]`,
   `[-1, 0, 1]`, `[0, 1/2, 1]`).

This is what lets `factor` fold the primitive part's leading coefficient back into the content,
exactly as `FactorPoly` does.

### 4.3 `factor` content rule

`FactorPoly` returns monic factors, so the primitive part's leading coefficient is a unit the
decomposition does not carry. When at least one factor is emitted the Rust `factor` multiplies the
content by that leading coefficient; when nothing is emitted (the primitive part is itself a unit)
the content is left untouched and the primitive part is returned as the one factor. The negative
constant and negative-leading-coefficient probes (`-6`, `-x^2 + 1`, `-2(x - 1)^2`) distinguish
the two branches and match the oracle.

## 5. Tests

`rust/lovelace-symbolics/src/round67_polynomial_algebra.rs` adds 16 tests. The five row tests are
in section 2; the eleven edge tests are:

| Test | Edge exercised | Oracle value pinned |
|---|---|---|
| `content_and_primitive_part_of_zero_and_constants` | zero, constants, fractions, non-monic linear | content `1/6`, primitive `[2, 3]`; zero -> `(1, [0])`; `±6` -> `(6, [±1])`; `2x + 2` -> `(2, [1, 1])`. |
| `primitive_part_keeps_the_sign_of_a_negative_leading_coefficient` | negative leading coefficient | `-2x^2 + 4x + 6` -> content `2`, primitive `[3, 2, -1]`. |
| `gcd_and_lcm_on_zero_constants_and_non_monic_inputs` | zero, constants, non-monic, negative lc | `gcd(0, x^2 - 1) = [-1, 0, 1]`, `lcm` with zero = `[0]`, `gcd(2x + 2, 4x + 4) = [1, 1]`, `lcm = [8, 8]`, `gcd(6, 4) = [1]`, `lcm(6, 4) = [24]`. |
| `resultant_of_zero_and_constant_operands` | zero, constants | zero operand -> `0`; two constants -> `1`; `res(3, x - 1) = 3`; `res(x^2 - 2, 3) = 9`. |
| `resultant_tracks_the_leading_coefficient_sign_and_scale` | non-monic, negative lc | `res(2x^2 - 4, x - 1) = -2`; `res(-x^2 + 2, x - 1) = 1`. |
| `discriminant_of_zero_constant_and_linear_polynomials` | zero, constants, linear | zero -> `0`; `±6` -> `1`; `x - 1` -> `1`; `3x - 3` -> `1`. |
| `discriminant_tracks_the_leading_coefficient_sign_and_scale` | non-monic, negative lc | `2x^2 + 6x + 4` and `-2x^2 + 6x - 4` -> `4`; `x^3 - x` -> `4`; `2x^3 - 2x` -> `64`. |
| `factor_of_zero_and_constants` | zero, constants | zero -> content `0`, no factors; `6` -> content `6`, `[(1, 1)]`; `-6` -> content `6`, `[(-1, 1)]`. |
| `factor_keeps_the_leading_coefficient_in_the_content` | non-monic, negative lc, repeated roots | `-x^2 + 1` -> content `-1`, `x + 1, x - 1`; `2x^2 + x` -> content `2`, `x + 1/2, x`; `4x^2 - 4` -> content `4`; `±2(x - 1)^2` -> multiplicity `2`; `2x^4 - 10x^2 + 8` -> content `2`, four linear factors. |
| `square_free_of_a_constant_is_the_unit` | zero, constants | zero -> `[]`; `±6` -> `[([1], 1)]`. |
| `square_free_normalises_non_monic_and_negative_inputs` | non-monic, negative lc, repeated roots | `2x^2 + 4x + 6` -> `[([3, 2, 1], 1)]`; `-x^2 + 1` -> `[([-1, 0, 1], 1)]`; `2x^2 + x` -> `[([0, 1/2, 1], 1)]`; `2(x - 1)^2` -> `[([-1, 1], 2)]`. |

No existing test expectation was changed; the two `round66_resweep.rs` and `round39_residue.rs`
square-free tests still pass unchanged.

## 6. Gates

| Gate | Command (from the stated directory) | Observed | Exit |
|---|---|---|---:|
| Oracle vs Rust probe | temporary C# probe + matching Rust probe, 59 lines | **59 / 59 identical** | 0 |
| Symbolics crate | `cargo test -p lovelace-symbolics` (`rust/`) | `running 168 tests ... 168 passed; 0 failed` | 0 |
| Full suite | `cargo test --workspace` (`rust/`) | `21 binaries: 5136 passed; 0 failed; 1 ignored` | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` (`rust/`) | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` (repo root) | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` (repo root) | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` (repo root) | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error** (1958 executions, 115 duplicates skipped); `harness exit: 0` | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` (`rust/`) | `Finished dev profile`; no diagnostics | 0 |
| Formatting | `cargo fmt --all -- --check` (`rust/`) | no diff | 0 |

The workspace suite and the fmt/clippy gates were rerun after the concurrent `lovelace-lang` lane
settled; the observed lines above are from that final run. The baseline was 5,098 passed / 1
ignored; this round adds the 16 tests (5,098 + 16 = 5,114) and the concurrent lane adds the rest
(5,136 total).

## 7. Scope and residual

* Edited: `rust/lovelace-symbolics/src/polynomials.rs` (new), `rust/lovelace-symbolics/src/lib.rs`
  (module registration plus the two square-free corrections),
  `rust/lovelace-symbolics/src/round67_polynomial_algebra.rs` (new), and this report.
  `rust/lovelace-symbolics/src/round66_resweep.rs` was reformatted by `cargo fmt` (import order
  and line wraps only; no assertion changed) so the symbolics package is fmt-clean.
* No `.cs` file, fixture, corpus file, `parity/` file or other document was edited.
* The `parity/manifest-map.json` mapping is intentionally left to the parent lane: this report lists
  the five ids and their test names, and the tests are in the measured suite.
* Both workspace-wide quality gates were run on the settled tree after the concurrent
  `rust/lovelace-lang` plugin lane stopped editing: `cargo clippy --workspace --all-targets -- -D
  warnings` and `cargo fmt --all -- --check` both exit 0 (section 6).
