# Requirements: Lovelace.Proofs — White-Paper Arithmetic in Lean

> Lifted requirements for a new Lean 4 project, `Lovelace.Proofs`, that formally proves the
> positional (base-`b`) arithmetic equations in `White Paper.pdf` — representation, addition with
> carry, subtraction with borrow, and multiplication via convolution. Proofs are stated over
> `Nat` using Lean 4 + Std only (no Mathlib). This document is the source of truth for what
> gets built; nothing is scaffolded or proven until it is approved.

---

## Purpose & Scope

`White Paper.pdf` derives the digit-wise algorithms of schoolbook arithmetic in an arbitrary
integer base `b`. The paper states the results informally (and with several notational
collisions and garbled general formulas). This project makes those results precise and proves
them in Lean 4.

**In scope** — for natural numbers `a, c : Nat` and a base `b : Nat` with `2 ≤ b`:

1. **Representation** — every `a` has a base-`b` digit expansion `a = Σ cᵢ bⁱ` with
   `0 ≤ cᵢ < b`, and the digit-extraction formula.
2. **Addition** — the carry-propagating digit addition equals `a + c`.
3. **Subtraction** — the borrow-propagating digit subtraction equals `a − c` when `a ≥ c`.
4. **Multiplication** — the convolution (Cauchy product) with carry equals `a · c`.

**Out of scope (for now):** `Integer`/signed numbers, `Real`/fractional & periodic
expansions, `sqrt`/division/inverse, arbitrary *change* of base between operands, and any
connection to the C# implementation's BCD encoding. These are follow-on projects.

---

## Lifted Mathematical Model

### Notation disambiguation (Paper → Lean)

The paper reuses symbols in conflicting ways. The following is the canonical reading used by
this project:

| Paper symbol | Meaning | Canonical name in Lean |
|---|---|---|
| `b` | base | `b : Nat`, with `2 ≤ b` |
| `a` | first operand | `a : Nat` |
| `c` (as a number) | second operand | **renamed** — stored as a digit list `cs`; the number is `ofDigits b cs` |
| `cᵢ` | digit of `a` at position `i` | `cs[i]` (list `cs : List Nat`) |
| `dⱼ` | digit of the second operand | `ds[j]` (list `ds : List Nat`) |
| `eᵢ` | digit of the result | `es[i]` (list `es : List Nat`) |
| `MOD b` / `% b` | remainder | `Nat.mod` / `% b` |
| `/ b` (integer) | floor quotient | `Nat.div` / `/ b` |
| overline `ēᵢ` (subtraction) | raw difference `b + cᵢ − dᵢ` | a local `Nat` term |
| overline `ēn` (multiplication) | column carry | a local `Nat` term |
| `Kᵢ` (subtraction) | borrow flag (`0` or `1`) | `borrow : Nat`, invariant `borrow ≤ 1` |

**Digit-list convention.** The paper writes `A(b) = cₙbⁿ + cₙ₋₁bⁿ⁻¹ + ⋯ + c₀b⁰`
(most-significant first). Lean lists are most naturally **least-significant first**, matching the
pseudocode loops (`i` from `0` upward). This project stores digits as
`[c₀, c₁, …, cₙ]` representing `Σ cᵢ bⁱ`. This is the same convention as Mathlib's
`Nat.digits` / `Nat.ofDigits`.

### 1. Representation

Paper:

> `∀ a ∈ ℕ → A(b), b ∈ ℕ`  and  `A(b) = cₙbⁿ + cₙ₋₁bⁿ⁻¹ + ⋯ + c₀b⁰ = a`
> with `cₙ = (a − cₙ₊₁bⁿ⁺¹) / bⁿ`

**Lifted (corrected).** The coefficient-extraction formula in the paper is garbled (off-by-one
and top-down). The canonical, well-defined form is:

```lean
def ofDigits (b : Nat) : List Nat → Nat
  | []      => 0
  | d :: ds => d + b * ofDigits b ds

def digits (b : Nat) (n : Nat) : List Nat := -- least-significant first
  if n = 0 then [] else (n % b) :: digits b (n / b)
```

with the digit-extraction identity

```lean
-- digit i of n is (n / b^i) % b   (this is the paper's "c_n = (a − c_{n+1} b^{n+1}) / b^n", corrected)
theorem digits_getD (hb : 2 ≤ b) (n i : Nat) :
  (digits b n).getD i 0 = (n / b ^ i) % b
```

### 2. Addition (carry)

Paper:

> `e₀ = (c₀ + d₀) MOD b`
> `e₁ = ((c₁ + d₁) + (c₀ + d₀)/b) MOD b`
> …
> `eₙ = (Σᵢ₌₀ⁿ (cₙ₋ᵢ + dₙ₋ᵢ)/bⁱ) MOD b`  *(correct as the closed form of the carry recursion — equals ⌊Sₙ/bⁿ⌋ mod b for Sₙ = Σ(cᵢ+dᵢ)bⁱ; the only caveat is that "/" is rational division, floor the whole sum before mod)*

and pseudocode:

```
for (Carry = 0 ; i < n ; i++) { Ei = (Ci + Di + Carry); Carry = Ei / b; Ei = Ei % b }
```

**Lifted (corrected).** The correct recursion (matching the pseudocode exactly):

```lean
carry₀   = 0
sᵢ      = cᵢ + dᵢ + carryᵢ
eᵢ      = sᵢ % b
carryᵢ₊₁ = sᵢ / b
```

```lean
def addDigits (b : Nat) : List Nat → List Nat → Nat → List Nat × Nat
  | [],      [],      carry => ([], carry)
  | x :: xs, y :: ys, carry =>
      let s := x + y + carry
      let (rest, c) := addDigits b xs ys (s / b)
      (s % b :: rest, c)
  | x :: xs, [],      carry =>
      let s := x + carry
      let (rest, c) := addDigits b xs [] (s / b)
      (s % b :: rest, c)
  | [],      y :: ys, carry =>
      let s := y + carry
      let (rest, c) := addDigits b [] ys (s / b)
      (s % b :: rest, c)
```

### 3. Subtraction (borrow)

Paper (for `a ≥ c`):

> `e₀ = (b + c₀ − d₀) MOD b`
> `e₁ = (b + c₁ − d₁ − (1 − (b + c₀ − d₀)/b)) MOD b`
> …
> `eₙ = (ēn − (1 − Kₙ)) MOD b`

and pseudocode:

```
for (Carry = 1 ; i < n ; i++) { Ei = (b + Ci − Di − (1 − Carry)); Carry = Ei / b; Ei = Ei % b }
```

**Lifted (corrected).** Re-express in terms of a borrow flag `borrow = 1 − Carry ∈ {0,1}`:

```lean
borrow₀   = 0
eᵢ       = (b + cᵢ − dᵢ − borrowᵢ) % b
borrowᵢ₊₁ = 1 − (b + cᵢ − dᵢ − borrowᵢ) / b
```

```lean
def subDigits (b : Nat) : List Nat → List Nat → Nat → List Nat × Nat
  | [],      [],      borrow => ([], borrow)
  | x :: xs, y :: ys, borrow =>
      let t := b + x - y - borrow
      let (rest, c) := subDigits b xs ys (1 - t / b)
      (t % b :: rest, c)
  | x :: xs, [],      borrow =>
      let t := b + x - borrow
      let (rest, c) := subDigits b xs [] (1 - t / b)
      (t % b :: rest, c)
  | [],      y :: ys, borrow =>
      let t := b + 0 - y - borrow
      let (rest, c) := subDigits b [] ys (1 - t / b)
      (t % b :: rest, c)
```

### 4. Multiplication (convolution + carry)

Paper:

> `eₙ = (c₀dₙ + c₁dₙ₋₁ + ⋯ + cₙd₀ + ēₙ₋₁) MOD b`
> `ēn = (c₀dₙ + c₁dₙ₋₁ + ⋯ + cₙd₀ + ēₙ₋₁) / b`

**Lifted.** Decompose into two independent facts:

**(a) Cauchy product** (no carry — pure algebra):

```lean
-- column sum s_n = Σ_{k=0..n} c_k · d_{n-k}
theorem convolution_mul (b : Nat) (cs ds : List Nat) :
  ofDigits b cs * ofDigits b ds =
    ∑ n < cs.length + ds.length - 1, (∑ k ≤ n, cs.getD k 0 * ds.getD (n - k) 0) * b ^ n
```

**(b) Carry normalization** (fold the column sums into canonical digits):

```lean
carry₋₁ = 0
eₙ      = (sₙ + carryₙ₋₁) % b
carryₙ  = (sₙ + carryₙ₋₁) / b
```

```lean
def normalize (b : Nat) (s : List Nat) : List Nat :=
  let rec go : List Nat → Nat → List Nat
    | [],      0 => []
    | [],      c => (c % b) :: go [] (c / b)
    | x :: xs, c =>
        let t := x + c
        (t % b) :: go xs (t / b)
  go s 0
```

> **Note (important).** Unlike addition/subtraction, where carry/borrow is `0` or `1`, the
> multiplication column sum `sₙ = Σ cₖ dₙ₋ₖ` can be as large as `(n+1)·(b−1)²`, so the
> multiplication carry is a **full `Nat`**, not a single bit. The paper's `ēn` is this
> multi-digit carry.

---

## Toolchain Prerequisites

| Requirement | Current state | Action needed before build |
|---|---|---|
| `elan` (Lean version manager) | **not installed** | install via <https://lean-lang.org/lean4/doc/quickstart.html> or `winget install elan` |
| `lean` (Lean 4, latest stable) | **not installed** | `elan toolchain install leanprover/lean4:stable` |
| `lake` (Lean build tool) | ships with `elan` | included |
| `Std` (Lean standard library) | bundled dependency | declared in `lakefile.lean`; no extra download beyond the toolchain |

> **Dependency decision (default): stdlib-only.** The target theorems are elementary `Nat`
> arithmetic (`+`, `*`, truncated `-`, `/`, `%`, `^ᵢ`, `List`, finite sums), all of
> which live in Lean core + `Std`. This avoids Mathlib's large toolchain/build and keeps the
> project self-contained. **Alternative:** depend on Mathlib and restate the results against
> `Nat.digits` / `Nat.ofDigits` / `Nat.ofDigits_digits` (already proven there) — heavier,
> but buys pre-proven round-trip lemmas. *See Open Questions.*

## Project Layout (proposed)

```
Lovelace.Proofs/               # lake package name: LovelaceProofs
├── lakefile.lean              # package LovelaceProofs; lean_lib Lovelace.Proofs
├── lean-toolchain             # leanprover/lean4:stable
├── Lovelace/
│   └── Proofs/
│       ├── Representation.lean   # ofDigits, digits, round-trip, extraction, uniqueness
│       ├── Addition.lean         # addDigits + correctness
│       ├── Subtraction.lean      # subDigits + correctness
│       ├── Multiplication.lean   # convolution, normalize + correctness
│       └── Basic.lean            # shared lemmas on /, %, ^, List sums
```

Lean namespace: `Lovelace.Proofs`. The lake package identifier is `LovelaceProofs` (Lean
identifiers cannot contain a bare `.`; the dot lives in the namespace).

---

## Proof Worktree (Completeness Checklist)

> Each unchecked item is one named Lean declaration to write and prove. Items are ordered by
> dependency (definitions before their theorems).

### Module: `Representation.lean`

- [ ] `ofDigits` — Horner evaluation of a least-significant-first digit list in base `b`.
- [ ] `digits` — base-`b` digit expansion of a natural number (well-founded on `n`).
- [ ] `ofDigits_digits` — **existence/round-trip**: `ofDigits b (digits b n) = n` for `2 ≤ b`.
- [ ] `digits_lt_base` — **valid digits**: `∀ d ∈ digits b n, d < b`.
- [ ] `digits_getD` — **extraction**: `(digits b n).getD i 0 = (n / b^i) % b`.
- [ ] `digits_injective` — **uniqueness**: `digits b` is injective (for `2 ≤ b`).
- [ ] `ofDigits_mul_base_add` — `ofDigits b (d :: ds) = d + b * ofDigits b ds` (helper).

### Module: `Addition.lean`

- [ ] `addDigits` — carry-propagating addition (with zero-padding for unequal lengths).
- [ ] `addDigits_carry_le_one` — the carry out of each column is `0` or `1` (needs `2 ≤ b`).
- [ ] `addDigits_correct` — **value preservation**:
  `ofDigits b (addDigits b cs ds k).1 + (addDigits b cs ds k).2 * b^len = ofDigits b cs + ofDigits b ds + k`.
- [ ] `addDigits_no_carry` — if the final carry is `0`, the result is exactly `a + c`.
- [ ] `addDigits_digit` — **per-digit closed form** `eᵢ = (cᵢ + dᵢ + carryᵢ) % b` (the paper's `e₀, e₁, …`).

### Module: `Subtraction.lean`

- [ ] `subDigits` — borrow-propagating subtraction.
- [ ] `subDigits_borrow_le_one` — borrow stays in `{0,1}`.
- [ ] `subDigits_correct` — **value preservation with borrow**:
  `ofDigits b cs − ofDigits b ds − borrow₀ = ofDigits b (subDigits …).1 − (subDigits …).2 * b^len`.
- [ ] `subDigits_no_borrow` — **paper's theorem**: if `ofDigits b ds ≤ ofDigits b cs` then
  `(subDigits b cs ds 0).2 = 0` and `ofDigits b (subDigits b cs ds 0).1 = ofDigits b cs − ofDigits b ds`.
- [ ] `subDigits_digit` — **per-digit closed form** `eᵢ = (b + cᵢ − dᵢ − borrowᵢ) % b`.

### Module: `Multiplication.lean`

- [ ] `convolution` — column-sum coefficients `sₙ = Σₖ cₖ dₙ₋ₖ`.
- [ ] `convolution_mul` — **Cauchy product**: `(Σ cᵢ bⁱ)(Σ dⱼ bʲ) = Σₙ (Σₖ cₖ dₙ₋ₖ) bⁿ`.
- [ ] `normalize` — carry propagation of a coefficient list into canonical base-`b` digits.
- [ ] `normalize_correct` — **value preservation** `ofDigits b (normalize b s) = ofDigits b s`
  and `∀ d ∈ normalize b s, d < b`.
- [ ] `mulDigits_correct` — **paper's theorem**: `ofDigits b (normalize b (convolution cs ds)) = ofDigits b cs * ofDigits b ds`.
- [ ] `mulDigits_digit` — **per-digit closed form** `eₙ = (sₙ + carryₙ₋₁) % b`.

### Shared: `Basic.lean`

- [ ] Basic `Nat` lemmas used throughout: `Nat.mod_lt`, `Nat.div_add_mod`, `Nat.div_eq_of_lt`,
  `Nat.mul_div_right`, power identities (`pow_succ`, `pow_add`), `List` sum/range/`getD`
  helpers — *re-export/restate the Std versions; do not re-derive from scratch.*

---

## Proof Plan (named theorems to prove)

> In this project the **verification artifact is a Lean theorem + its proof**, replacing the
> xUnit "test plan" used by the C# projects. The naming convention mirrors it:
> `<operation>_<property>` — e.g. `addDigits_correct`. A theorem is "done" only when
> `#check`/`lake build` accepts it with no `sorry`/`admit` remaining.

### Representation

1. `ofDigits_digits (hb : 2 ≤ b) (n : Nat) : ofDigits b (digits b n) = n`
   *Reading*: every natural number equals the value of its base-`b` digits (existence).

2. `digits_lt_base (hb : 2 ≤ b) (n : Nat) : ∀ d ∈ digits b n, d < b`
   *Reading*: every emitted digit is a valid base-`b` digit.

3. `digits_getD (hb : 2 ≤ b) (n i : Nat) : (digits b n).getD i 0 = (n / b ^ i) % b`
   *Reading*: the `i`-th digit is `(n / bⁱ) mod b` — the paper's coefficient formula, corrected.

4. `digits_injective (hb : 2 ≤ b) : Function.Injective (digits b)`
   *Reading*: base-`b` representation is unique (no two distinct `digits` lists encode the same `n`).

### Addition

5. `addDigits_correct (b : Nat) (cs ds : List Nat) (k : Nat) :
   ofDigits b (addDigits b cs ds k).1 + (addDigits b cs ds k).2 * b ^ (addDigits b cs ds k).1.length
   = ofDigits b cs + ofDigits b ds + k`
   *Reading*: the carry algorithm preserves value — result plus shifted final carry equals the true sum.

6. `addDigits_digit (hb : 2 ≤ b) (cs ds : List Nat) (i : Nat) :
   (addDigits b cs ds 0).1.getD i 0 = (cs.getD i 0 + ds.getD i 0 + carryInto b cs ds i) % b`
   *Reading*: the paper's `eᵢ = (cᵢ + dᵢ + carryᵢ) mod b` per-column form (`carryInto` = the running carry).

### Subtraction

7. `subDigits_correct (b : Nat) (cs ds : List Nat) (borrow0 : Nat) (h : borrow0 ≤ 1) :
   ofDigits b cs − ofDigits b ds − borrow0
   = ofDigits b (subDigits b cs ds borrow0).1 − (subDigits b cs ds borrow0).2 * b ^ (subDigits b cs ds borrow0).1.length`
   *Reading*: borrow-preservation — the general value identity relating result and final borrow.

8. `subDigits_no_borrow (b : Nat) (cs ds : List Nat) (hle : ofDigits b ds ≤ ofDigits b cs) :
   (subDigits b cs ds 0).2 = 0 ∧ ofDigits b (subDigits b cs ds 0).1 = ofDigits b cs − ofDigits b ds`
   *Reading*: the paper's `A(b) − C(b) = e` when `a ≥ c` (no final borrow).

9. `subDigits_digit (hb : 2 ≤ b) (cs ds : List Nat) (i : Nat) :
   (subDigits b cs ds 0).1.getD i 0 = (b + cs.getD i 0 − ds.getD i 0 − borrowInto b cs ds i) % b`
   *Reading*: the paper's `eᵢ = (b + cᵢ − dᵢ − borrowᵢ) mod b` per-column form.

### Multiplication

10. `convolution_mul (b : Nat) (cs ds : List Nat) :
    ofDigits b cs * ofDigits b ds =
    ∑ n ∈ List.range (cs.length + ds.length - 1),
      (∑ k ∈ List.range (n + 1), cs.getD k 0 * ds.getD (n - k) 0) * b ^ n`
    *Reading*: the Cauchy product — the coefficient of `bⁿ` is `Σₖ cₖ dₙ₋ₖ`.

11. `normalize_correct (hb : 2 ≤ b) (s : List Nat) :
    (∀ d ∈ normalize b s, d < b) ∧ ofDigits b (normalize b s) = ofDigits b s`
    *Reading*: carry propagation preserves value and yields canonical digits.

12. `mulDigits_correct (hb : 2 ≤ b) (cs ds : List Nat) :
    ofDigits b (normalize b (convolution cs ds)) = ofDigits b cs * ofDigits b ds`
    *Reading*: the paper's multiplication — convolution then carry equals the true product.

13. `mulDigits_digit (hb : 2 ≤ b) (cs ds : List Nat) (i : Nat) :
    (normalize b (convolution cs ds)).getD i 0 = (convCoeff cs ds i + carryIntoMul b cs ds i) % b`
    *Reading*: the paper's `eₙ = (c₀dₙ + … + cₙd₀ + ēₙ₋₁) mod b` per-column form.

---

## Open Questions / Risks

1. **Dependency: Std-only (default) vs Mathlib.** Std-only keeps the toolchain small but requires
   restating a handful of `Nat`/list lemmas; Mathlib already proves `Nat.ofDigits_digits` etc.
   and would let the Representation module *restate* rather than *re-derive*. Default is Std-only.
2. **Uniqueness scope.** Uniqueness is stated as `digits_injective`; full "uniqueness up to
   trailing zeros of `ofDigits`" is deferred (not needed by the arithmetic theorems).
3. **Paper corrections are narrow, notational.** The paper's *sum* formulas are the closed-form
   unrolling of its own pseudocode and are essentially correct — e.g. the addition sum
   `eₙ = (Σ (cₙ₋ᵢ+dₙ₋ᵢ)/bⁱ) mod b` equals ⌊Sₙ/bⁿ⌋ mod b. The genuine defects are index typos in the
   *unrolled intermediate lines* (subtraction's e₂/e₃/K₂/K₃ and multiplication's generic one-liner
   use the wrong carry/borrow subscript). This document pins the *pseudocode recurrences*
   (unambiguous) as the canonical targets; the literal typo'd lines are not themselves proven.
4. **Base bound.** The paper says `b ∈ ℕ`; proofs require `2 ≤ b` (the `b = 0,1` cases are
   degenerate). All theorems carry the hypothesis `hb : 2 ≤ b`.
5. **Toolchain not installed.** Building requires installing `elan`/Lean 4 first; this is a
   prerequisite and is *not* part of the requirements-lift step.