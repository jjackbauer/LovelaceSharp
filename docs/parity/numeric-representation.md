# PROOF-003 — Rust numeric representation and the Lean proof model

This document records how the representation used by the Rust numeric tower corresponds to the
model in the pinned Lean project, and which parts of the tower that model does **not** cover. It
is the companion to [proof-theorem-map.md](proof-theorem-map.md) (PROOF-002) and to the exhaustive
small-domain tests in [proof_small_domain.rs](../../rust/lovelace-nat/src/proof_small_domain.rs)
(PROOF-004).

## 1. The proof model

[Lovelace.Proofs/LovelaceProofs/Basic.lean](../../Lovelace.Proofs/LovelaceProofs/Basic.lean)
defines a value from a least-significant-first digit list at an arbitrary base `b`:

    ofDigits b []       = 0
    ofDigits b (d::ds)  = d + b * ofDigits b ds

A digit list is *valid* when every digit is below `b` (`∀ d ∈ l, d < b`), which the theorems
carry as an explicit hypothesis. `digits b n` is the canonical expansion (no most-significant
zero; `digits b 0 = []`). Every theorem is quantified over `b` — the premises are `2 ≤ b` or
`0 < b`, never a concrete base — so the model is not tied to decimal digits.

## 2. The Rust representation

[rust/lovelace-nat](../../rust/lovelace-nat/src/lib.rs) stores a natural as:

* a little-endian `Vec<u64>` of base-2^64 limbs, with no most-significant zero limb and zero as
  the empty vector;
* plus a lazily materialised canonical decimal string (`Arc<str>`) that is shared by clones. At
  least one form is always present; the other is computed on demand and cached.

Two derived representations matter for this correspondence:

* **base-2^64 digits** — `Nat::limbs()` in the proof tests, i.e. `digits (2^64) n`;
* **base-10 digits** — `Nat::to_decimal()` / `Nat::from_decimal()`, i.e. `digits 10 n` and
  `ofDigits 10` over the canonical decimal text.

[rust/lovelace-int](../../rust/lovelace-int/src/lib.rs) is a sign plus a `Nat` magnitude with no
negative zero. [rust/lovelace-real](../../rust/lovelace-real/src/lib.rs) is an exact
`int * 10^exp` with optional period metadata (`period_start` / `period_len`), so the stored digits
of a repeating value are the period block plus the pre-period.

## 3. Correspondence

Let `B = 2^64`.

| Lean model | Rust representation | Why the correspondence holds |
|---|---|---|
| `Nat` (mathematical, unbounded) | limbs `Σ limbs[i] · B^i`, unbounded `Vec` | No fixed width; growth is by appending limbs. The value is exactly the finite sum. |
| `ofDigits b ds` | `Nat` value; for the decimal form `ofDigits 10` | Set `b = B`, `ds = limbs`; or `b = 10` over the canonical decimal text. |
| `digits b n` | `Nat::limbs()` (`b = B`) / `to_decimal()` (`b = 10`) | Both are canonical: no most-significant zero, zero is empty; least-significant first in the limb vector. |
| valid digit list (`d < b`) | every limb is `u64`; every decimal character is `0..=9` | The `u64` type enforces the bound at `B`; the renderer emits decimal digits only. |
| `getDigit l i` | limb `i` / decimal digit `i` | `digits_getDigit`: `(n / b^i) % b`; the test re-derives each limb with `div_rem`. |
| `addDigits` (carry) | `Nat::add` | The per-column sum is `x + y + carry`, at most `2B - 1`, computed in `u128`; carry is `sum / B`. |
| `subDigits` (borrow) | `Nat::sub` (`None` on underflow) | The per-column value is `B + x - y - borrow`; the new borrow is `1 - t / B`, exactly `subColumn`. |
| `conv` + `normalize` | `Nat::mul` (schoolbook → Karatsuba → NTT) | The theorem states the **product contract**; the fast paths are different algorithms with their own exactness arguments (see `4). |
| `divDigits` / `divDigitsMSB` | `Nat::div_rem` (Knuth → Newton → divide-and-conquer) | The theorem states the **quotient/remainder contract** `q·d + r = a`, `r < d`; the code does not run the MSB fold. |
| `ofDigits_lt_pow` | limb count exactly `ceil(bits / 64)` | A value below `B^k` has at most `k` limbs; the test asserts both bounds. |
| `digits_injective` | canonical decimal text | Parse + render round trip preserves the value and the text, with no leading zero. |

The instantiation is legitimate because the Lean theorems have no assumption about the size of
`b`: `b = 2^64` satisfies `2 ≤ b` and the proofs use only division/modulo by `b`. The decimal
instance is the second, independent instantiation (`b = 10`) and is what the language's public
text contract depends on.

## 4. Intermediate arithmetic: no overflow in the modelled operations

The Lean model computes in unbounded `Nat`. The Rust implementation uses `u128` for per-column
intermediates, which is sufficient by construction:

* **add / subtract**: one column is at most `2·(B-1) + 1 < 2^65`.
* **schoolbook multiply**: `(B-1)^2 + 2·(B-1) = B^2 - 1 < 2^128`, so one `u128` holds the
  product plus both carries.
* **limb normalisation**: after the convolution, a coefficient is reduced with `% B` and `/ B` in
  `u128` before it can exceed the bound.

The fast multiplication paths do **not** reduce to that argument: NTT multiplication reconstructs
coefficients from residues modulo three primes, and its exactness rests on the product of the
primes exceeding the largest coefficient (the threshold is enforced in
[ntt.rs](../../rust/lovelace-nat/src/ntt.rs) and cross-checked by
`agrees_with_the_existing_nat_multiplication`). Newton and divide-and-conquer division likewise
have their own convergence/recursion obligations. Those are outside the Lean model; the PROOF-002
mapping marks them contract-level for exactly this reason.

## 5. What the proof model does not cover

The pinned Lean project models the `Nat` digit algorithms only. The following parts of the Rust
numeric tower are deliberately outside PROOF-002's mapping:

* **Laziness and caching.** `Nat` can hold only a decimal string until arithmetic forces limb
  materialisation. The model has one representation; the implementation has two, proven mutually
  consistent by the round-trip tests (`ofDigits_digits` at `b = 10` and the limb recombination test
  at `b = B`).
* **DigitStore.** [digit_store.rs](../../rust/lovelace-nat/src/digit_store.rs) is a base-10 BCD
  buffer with two digits per byte, mirroring `Lovelace.Representation.DigitStore`. It is a
  separate value type, not the arithmetic path; its invariants (digit bound, zero
  canonicalisation, byte count) correspond to `digits 10`, and it is covered by its own tests, not
  by the theorems.
* **Int, Rational, Real, Complex.** Sign/magnitude, exact fractions, periodic decimals, precision
  caps and transcendental functions have no Lean model in this project. Their Rust contracts are
  covered by the crate tests and the parity corpus.
* **Thread safety and ownership.** `OnceLock` materialisation, `Arc` sharing and the `DigitStore`
  mutex are runtime concerns the proof model does not state.
* **Algorithm selection.** Thresholds (`KARATSUBA_THRESHOLD`, `BZ_THRESHOLD`, `ntt_threshold()`)
  choose among implementations; the theorems constrain the result, not the choice.
  NUM-008/NUM-009 cover threshold-independence separately.

## 6. Verifying the correspondence

    cargo test -p lovelace-nat proof_      # 7 exhaustive small-domain tests (PROOF-004)
    cargo test -p lovelace-nat             # the crate's own property/round-trip tests
    cargo clippy -p lovelace-nat --all-targets -- -D warnings

The tests instantiate both bases (`B = 2^64` for limbs, `10` for text), assert the digit bounds,
reconstruct values from digits, and compare every result against an independent `u128` oracle over
the exhaustive domains listed in the test module. The CI job
[rust-ci.yml](../../.github/workflows/rust-ci.yml) runs the `proof_` subset as a named step.
