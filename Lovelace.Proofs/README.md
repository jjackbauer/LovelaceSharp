# Lovelace.Proofs

Formal proofs in [Lean 4](https://lean-lang.org/) of the positional (base-`b`) arithmetic
equations in the repository's `White Paper.pdf`: representation, addition with carry,
subtraction with borrow, multiplication via convolution, and division via long division.

The project is **core-Lean only** (no Mathlib, no `Std` dependency): every theorem is stated
and proved over `Nat` using `Lean 4.33.1`.

## What is proved

The canonical targets are the White Paper's *digit-wise pseudocode loops*, restated as
least-significant-first digit lists and proved correct against ordinary `Nat` arithmetic.
See `.github/requirements/Lovelace.Proofs.md` for the full lifted specification.

### Representation (`LovelaceProofs/Representation.lean`)

| Theorem | Statement |
|---|---|
| `ofDigits_digits` | `ofDigits b (digits b n) = n` (existence/round-trip) |
| `digits_lt_base` | every digit of `digits b n` is `< b` |
| `digits_getDigit` | `getDigit (digits b n) i = (n / b ^ i) % b` (extraction) |
| `digits_injective` | `digits b` is injective (uniqueness) |

### Addition (`LovelaceProofs/Addition.lean`)

| Theorem | Statement |
|---|---|
| `addDigits_correct` | `ofDigits (addDigits cs ds k).1 + carry · b^len = ofDigits cs + ofDigits ds + k` |
| `addDigits_no_carry` | no final carry ⇒ result is exactly the sum |

### Subtraction (`LovelaceProofs/Subtraction.lean`)

| Theorem | Statement |
|---|---|
| `subDigits_correct` | `ofDigits cs + borrow · b^len = ofDigits ds + borrow0 + ofDigits result` |
| `subDigits_no_borrow` | `ofDigits ds ≤ ofDigits cs` ⇒ no final borrow and result is the difference |

### Multiplication (`LovelaceProofs/Multiplication.lean`)

| Theorem | Statement |
|---|---|
| `ofDigits_conv` | Cauchy product: `ofDigits (conv cs ds) = ofDigits cs * ofDigits ds` |
| `normalize_correct` | carry propagation preserves value and yields digits `< b` |
| `mulDigits_correct` | `ofDigits (normalize (conv cs ds)) = ofDigits cs * ofDigits ds` |

### Division (`LovelaceProofs/Division.lean`)

Digit-wise long division: a running remainder `r < d` plus one dividend digit `cᵢ < b` compute each
column `qᵢ = (r·b + cᵢ) / d`, `r := (r·b + cᵢ) % d` — the state stays `(b, d)`-bounded.

| Theorem | Statement |
|---|---|
| `divColumn` | `r < d → c < b → (r·b + c) / d < b ∧ (r·b + c) % d < d` |
| `divDigits_correct` | `ofDigits b (divDigits b ds d).1 · d + (divDigits b ds d).2 = ofDigits b ds` |
| `divDigits_quotient_lt_base` | every quotient digit of `divDigits b ds d` is `< b` |
| `divDigits_remainder_lt` | `(divDigits b ds d).2 < d` |
| `divDigits_quotient_eq_div` | `ofDigits b (divDigits b ds d).1 = ofDigits b ds / d` |
| `divDigits_remainder_eq_mod` | `(divDigits b ds d).2 = ofDigits b ds % d` |
| `divDigits_getDigit` | i-th quotient digit: `getDigit (divDigits b ds d).1 i = (ofDigits b ds / d / b^i) % b` |

All statements carry the hypothesis `2 ≤ b` (base at least 2) — except division, whose divisor
carries `0 < d` — matching the corrected reading of the paper's loose `b ∈ ℕ`.

## Building

Requires Lean 4.33.1 (`lake` + `lean` on `PATH`):

```
cd Lovelace.Proofs
lake build
```

The build produces the compiled library with **zero `sorry`/`admit`**.

## Layout

```
Lovelace.Proofs/
├── lakefile.lean              # package LovelaceProofs; core-only (no requires)
├── lean-toolchain             # leanprover/lean4:v4.33.1
├── LovelaceProofs.lean        # root module (imports all submodules)
└── LovelaceProofs/
    ├── Basic.lean             # ofDigits, getDigit, addLists + helper lemmas
    ├── Representation.lean    # digits + round-trip/extraction/uniqueness
    ├── Addition.lean          # addDigits + correctness
    ├── Subtraction.lean       # subDigits + borrow correctness
    ├── Multiplication.lean    # conv, normalize + product correctness
    └── Division.lean          # divDigits + long-division correctness
```

Lean namespace: `Lovelace.Proofs`.
