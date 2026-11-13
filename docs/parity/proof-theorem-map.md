# PROOF-002 — Lean theorems mapped to Rust implementation-boundary tests

The pinned Lean project is [Lovelace.Proofs/LovelaceProofs](../../Lovelace.Proofs/LovelaceProofs.lean) with six
modules. Its theorems are stated for an abstract base `b` over mathematical digit lists. The
Rust implementation is [rust/lovelace-nat](../../rust/lovelace-nat/src/lib.rs): a value is a
little-endian array of base-2^64 limbs with no most-significant zero limb, plus a lazily
materialised canonical decimal rendering.

This document maps each theorem to the Rust boundary it constrains and to the test that exercises
that boundary. The mapping is deliberately two-sided:

* **algorithm-level** — the Rust code implements the same digit algorithm as the Lean definition,
  so the theorem's proof steps apply to the code shape (carry/borrow propagation);
* **contract-level** — the Rust code implements a *different* algorithm (Karatsuba/NTT
  multiplication, Knuth/Newton/BZ division) but the theorem's *statement* is the contract the code
  must satisfy;
* **proof-internal** — the lemma only serves another theorem's proof and has no independent
  implementation boundary;
* **no counterpart** — the Rust design has no such operation (e.g. fixed result lengths).

The exhaustive small-domain tests live in two modules and are also the PROOF-004 artifact:

* [rust/lovelace-nat/src/proof_small_domain.rs](../../rust/lovelace-nat/src/proof_small_domain.rs) —
  the digit-algorithm instances of the Lean theorems (7 tests);
* [rust/lovelace-int/src/proof_small_domain.rs](../../rust/lovelace-int/src/proof_small_domain.rs) —
  the signed tower on top of those magnitudes: every `a, b` in `[-64, 64]` for `+`, `-`, `*`,
  `div_rem_signed` and ordering against an `i128` oracle, plus ring laws and transitivity over
  every `a, b, c` in `[-16, 16]` (6 tests). `Int` has no Lean module; these tests exercise the
  laws the language exposes, not a theorem statement.

Run them with:

    cargo test -p lovelace-nat proof_
    cargo test -p lovelace-int proof_

Last run on this machine: **7 passed** (nat) and **6 passed** (int), 0 failed (see the round report
for the recorded output). The full crate suites are `cargo test -p lovelace-nat` and
`cargo test -p lovelace-int`.

## 1. Mapping

### Basic.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `ofDigits_nil`, `ofDigits_cons` | evaluation of a digit list is structural | `Nat::limbs` / `Nat::from_limbs` (base-2^64 digits) | `proof_representation_digits_get_digit` recombines limbs and compares | algorithm-level |
| `ofDigits_map_mul` | scaling every digit scales the value | not a public operation (proof-internal) | — | proof-internal |
| `ofDigits_addLists` | adding digit lists adds their values | `Nat::add` | `proof_addition_of_digits_add_lists` (exhaustive 0..256 pairs; carry case `(2^64-1)+1`) | algorithm-level |
| `ofDigits_lt_pow` | a k-digit list is below `b^k` | limb count vs value bound | `proof_basic_of_digits_lt_pow` (exhaustive 0..65 536; boundary limbs at positions 0..3) | algorithm-level |

### Representation.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `digits_zero` | zero has no digits | `Nat::zero()`, empty limb array, `to_decimal() == "0"` | `proof_representation_of_digits_digits` (value 0 included) | algorithm-level |
| `ofDigits_digits` | expanding then evaluating reconstructs the value | decimal parse/format round trip; limb recombination | `proof_representation_of_digits_digits` (0..65 536), `proof_representation_digits_get_digit` (b = 2^64) | algorithm-level |
| `digits_lt_base` | every digit is below the base | `u64` limbs are below 2^64 by type; decimal digits are ASCII 0-9 | `proof_representation_of_digits_digits` (digit character check) | algorithm-level |
| `digits_getDigit` | digit i is `(n / b^i) % b` | limb i / decimal digit i | `proof_representation_digits_get_digit` (boundary-limb cross-product) | algorithm-level (at b = 2^64) |
| `digits_injective` | the digit expansion is unique | canonical decimal text is unique (no leading zeros) | `proof_representation_of_digits_digits` (render == input text) | algorithm-level |

### Addition.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `addDigits_correct` | carry-propagating addition equals the sum with the final carry | `Nat::add` | `proof_addition_of_digits_add_lists`; existing `round39_natural_api::natural_implements_iadditionoperators` | algorithm-level |
| `addDigits_no_carry` | with no final carry the result is exactly the sum | `Nat::add` (result keeps the natural width) | same test (all exhaustive pairs have no final carry) | algorithm-level |

### Subtraction.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `subColumn` | one-column borrow identity | `Nat::sub` limb step | `proof_subtraction_sub_digits_correct` (borrow cases) | algorithm-level |
| `subDigits_lt_base` | every result digit is below the base | result limbs are `u64` by type | `proof_subtraction_sub_digits_correct` (base-2^64 digit expansion) | algorithm-level |
| `subDigits_length` | the result has the input length | Rust normalises away high zero limbs | — | no counterpart |
| `subDigits_correct` | subtraction with the final borrow is exact | `Nat::sub` returns `None` on underflow, `Some` otherwise | `proof_subtraction_sub_digits_correct` (exhaustive 0..256 pairs + `2^192 - 1`) | algorithm-level |
| `subDigits_no_borrow` | with `ds ≤ cs` subtraction yields the difference | `Nat::sub` | same test | algorithm-level |

### Multiplication.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `ofDigits_conv` | the digit convolution evaluates to the product | `Nat::mul` | `proof_multiplication_mul_digits_correct` (two-limb convolution with independent `u128` coefficients) | contract-level (Rust multiplies by schoolbook/Karatsuba/NTT, not by `conv`) |
| `normalizeAux_spec`, `normalize_correct` | normalising a coefficient list preserves its value and bounds the digits | limb normalisation / carry propagation | `proof_multiplication_mul_digits_correct` (`(2^64-1)^2 = 2^128 - 2^65 + 1`) | contract-level |
| `mulDigits_correct` | normalised convolution equals the product | `Nat::mul` | `proof_multiplication_mul_digits_correct`; existing `ntt::agrees_with_the_existing_nat_multiplication`, `round39_natural_api::div_rem_given_large_operands_produces_correct_quotient_and_remainder` | contract-level |

### Division.lean

| Theorem | Statement (informal) | Rust boundary | Test | Relation |
|---|---|---|---|---|
| `ofDigits_append`, `ofDigitsMSB_eq_reverse` | MSB evaluation equals reversed LSB evaluation | representation conversion only | `proof_representation_digits_get_digit` (digit order) | proof-internal |
| `ofDigits_mod_eq_getDigit`, `ofDigits_div_eq_ofDigits_tail`, `getDigit_ofDigits` | digit i is `(n / b^i) % b` | limb/decimal digit extraction | `proof_representation_digits_get_digit` | algorithm-level |
| `mul_pow_add`, `divStep`, `divDigitsMSB_correct` | the long-division fold reconstructs the dividend | `Nat::div_rem` quotient/remainder identity | `proof_division_div_digits_correct` (`q*d + r == a` over 0..4096 × 1..64, multi-limb dividends) | contract-level (Rust divides by Knuth/Newton/BZ) |
| `divColumn` | one column keeps `q < b` and `r < d` | quotient digit and remainder bounds | `proof_division_div_digits_correct` (remainder < divisor everywhere) | contract-level |
| `divDigitsMSB_quotient_lt_base`, `divDigits_quotient_lt_base` | every quotient digit is below the base | result limbs are `u64` | `proof_division_div_digits_correct` (small quotients cross-checked against `u128`) | contract-level |
| `divDigitsMSB_remainder_lt`, `divRemMSB_eq_snd`, `divRemMSB_remainder_lt`, `divDigits_remainder_lt` | the running remainder stays below the divisor | `Nat::div_rem` remainder | `proof_division_div_digits_correct` (`r < d` assertion in every block) | contract-level |
| `divDigits_correct` | `ofDigits q * d + r = a` | `Nat::div_rem` | `proof_division_div_digits_correct`; existing `round39_natural_api::div_rem_given_large_operands_produces_correct_quotient_and_remainder` | contract-level |
| `divDigits_quotient_eq_div` | quotient equals `a / d` | `Nat::div_rem` | `proof_division_div_digits_correct` (u128 quotient) | contract-level |
| `divDigits_remainder_eq_mod` | remainder equals `a % d` | `Nat::div_rem` | `proof_division_div_digits_correct` (u128 remainder) | contract-level |
| `divDigits_getDigit` | quotient digit i is `(a / d / b^i) % b` | quotient limb i | **gap** — no direct test; the quotient value is tested, the per-digit extraction is not | contract-level |
| `divRemMSB_eq_snd`, `divDigitsMSB_length` | the MSB fold equals the quotient fold and keeps length | Rust does not use an MSB fold and does not keep fixed lengths | — | no counterpart |

## 2. What this mapping does and does not prove

* The Lean project proves properties of **mathematical digit algorithms**. It does not model Rust
  memory, `OnceLock` caches, thread safety, or the `DigitStore` type.
* Addition and subtraction are algorithm-level matches: the Rust carry/borrow loops follow the same
  recurrence the theorems describe, and the exhaustive domains include every carry and borrow case
  for one byte and every limb boundary.
* Multiplication and division are **contract-level** matches. Rust uses Karatsuba/NTT for large
  products and Knuth/Newton/BZ for large quotients; those algorithms have their own exactness
  obligations (NTT modular reconstruction, Newton convergence) that the Lean project does not
  model. The tests verify the theorem's **statement** on exhaustive small domains and on
  limb-boundary values, not the fast-path internals.
* The unsigned convention is the only one in the Lean model. `Int` (sign + magnitude) is
  mapped to `Nat` and covered by the integer crate's own exhaustive small-domain tests
  (`cargo test -p lovelace-int proof_`), not by these theorems; see
  [numeric-representation.md](numeric-representation.md) for the complete tower correspondence.
* These boundary tests are evidence, not a proof. The contract's own wording is recorded here:
  "These Rust property tests do not, by themselves, constitute a formal proof of the Rust code."

## 3. Reproduce

    cargo test -p lovelace-nat proof_          # 7 tests, exhaustive small domains
    cargo test -p lovelace-int proof_          # 6 tests, exhaustive signed domains
    cargo test -p lovelace-nat                 # the whole crate, including the fast-path tests following the same contract
    cargo clippy -p lovelace-nat --all-targets -- -D warnings

The CI workflow [.github/workflows/rust-ci.yml](../../.github/workflows/rust-ci.yml) runs
`cargo test -p lovelace-nat proof_` and `cargo test -p lovelace-int proof_` as their own steps, so
the exhaustive verification is part of CI and not only of this document.
