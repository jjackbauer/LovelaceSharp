# Requirements: Lovelace.Proofs.Division — Digit-by-Digit Long Division in Lean

> Lifted requirements for the **division** module of `Lovelace.Proofs`, extending the existing
> core-Lean project (`Lovelace.Proofs` lake package, namespace `Lovelace.Proofs`). This document
> is the source of truth for what gets built for division; the Representation / Addition /
> Subtraction / Multiplication modules are already proven and are **not** re-derived here.

---

## Purpose & Scope

The existing project proves the White Paper's digit-wise *representation*, *addition*, *subtraction*,
and *multiplication* over `Nat` in an arbitrary base `b` (`2 ≤ b`), with digits stored
least-significant-first. This module adds **long division**: given a dividend written as base-`b`
digits and a positive divisor `d`, produce the quotient's base-`b` digits and the remainder, and
prove the result is exactly `Nat.div` / `Nat.mod`.

**In scope:**

1. **Long division** — the schoolbook digit-by-digit algorithm (most-significant first).
2. **Quotient digits are valid** — every emitted quotient digit is `< b` (no post-normalization
   carry needed).
3. **Remainder bound** — the final remainder is `< d`.
4. **Correctness** — `a = q · d + r` where `a` is the dividend value, `q` the quotient value,
   `r` the remainder.
5. **Connection to `Nat.div`/`Nat.mod`** — the quotient value equals `a / d` and the remainder
   equals `a % d`.

**Out of scope (for now):** signed division, fractional/repeating expansion, division of two
arbitrary digit lists (the divisor is a plain `Nat`), and any C#-implementation detail.

---

## Lifted Mathematical Model

### Digit form

The computation is the running-remainder recurrence — one column needs only `r < d` and `cᵢ < b`:

```text
t_i     = r_i * b + cᵢ
q_i     = t_i div d      (and q_i < b)
r_{i+1} = t_i mod d      (and r_{i+1} < d),   r_0 = 0
```

`divColumn` proves `q_i < b` and `r_{i+1} < d`, so the state never leaves the `(b, d)`-bounded range
(no whole-number value is materialized). The closed-form extraction

```text
getDigit (divDigits b ds d).1 i  =  (ofDigits b ds / d / b^i) % b
```

is a derived consequence of the recurrence, not the algorithm's definition.

### Algorithm (the long-division loop, MSB-first)

Take the dividend digits `[cₙ, cₙ₋₁, …, c₀]` **most-significant first**, and a divisor `d > 0`:

```text
r := 0
for each digit cᵢ  (MSB → LSB):
    t   := r * b + cᵢ
    qᵢ  := t div d
    r   := t mod d
result: quotient digit list [qₙ, …, q₀] (MSB-first) and remainder r
```

### Convention and data model

The rest of the library stores digits **least-significant first** and evaluates them with
`ofDigits` (Horner). Division is naturally a **most-significant-first** fold, so the module:

- runs the fold over the *reversed* input digit list,
- returns the quotient digits reversed back to least-significant-first,

so that the public interface and all theorems are stated against the **same** `ofDigits` as every
other module. A small MSB-first evaluator `ofDigitsMSB` (`ofDigitsMSB b l = ofDigits b l.reverse`)
is introduced purely as a proof device for the MSB-first fold.

### Core invariant

For the MSB-first fold with a running remainder `r` (satisfying `0 ≤ r < d`):

```text
ofDigitsMSB (quotient so far) · d + r  =  ofDigitsMSB (processed prefix) + r_in · b^(prefix length)
```

At the end (`r_in = 0`) this is exactly `a = q · d + r` with `0 ≤ r < d`.

---

## Proof Worktree (Completeness Checklist)

> Each item is one named Lean declaration. `d` is positive (`0 < d`) throughout; the dividend
> digits are assumed valid (`∀ c ∈ ds, c < b`) only where a digit bound is claimed.

### Module: `Division.lean`

- [ ] `ofDigitsMSB` — evaluate a **most-significant-first** digit list in base `b`.
- [ ] `ofDigits_append` — `ofDigits b (xs ++ ys) = ofDigits b xs + b^xs.length · ofDigits b ys`.
- [ ] `ofDigitsMSB_eq_reverse` — `ofDigitsMSB b l = ofDigits b l.reverse`.
- [ ] `divDigitsMSB` — MSB-first long division fold with a running-remainder accumulator.
- [ ] `divDigits` — public entry point: LSB-first input/output, reverses internally.
- [ ] `divDigitsMSB_length` — the quotient list is as long as the (MSB-first) input.
- [ ] `divDigitsMSB_correct` — the running-remainder invariant (value preservation).
- [ ] `divDigitsMSB_quotient_lt_base` — every quotient digit of the fold is `< b`.
- [ ] `divDigitsMSB_remainder_lt` — the running remainder stays `< d`.
- [ ] `divRemMSB` — first-class running-remainder function (the recurrence state).
- [ ] `divRemMSB_eq_snd` — `divRemMSB b d ds r = (divDigitsMSB b ds d r).2`.
- [ ] `divRemMSB_remainder_lt` — the running remainder always stays `< d`.
- [ ] `divColumn` — one column step: `r < d → c < b → (r·b+c)/d < b ∧ (r·b+c)%d < d`.
- [ ] `divDigits_quotient_lt_base` — every quotient digit of `divDigits` is `< b`.
- [ ] `divDigits_remainder_lt` — `(divDigits b ds d).2 < d`.
- [ ] `divDigits_correct` — `ofDigits b (divDigits b ds d).1 · d + (divDigits b ds d).2 = ofDigits b ds`.
- [ ] `divDigits_quotient_eq_div` — `ofDigits b (divDigits b ds d).1 = ofDigits b ds / d`.
- [ ] `divDigits_remainder_eq_mod` — `(divDigits b ds d).2 = ofDigits b ds % d`.
- [ ] `getDigit_ofDigits` — `getDigit l i = (ofDigits b l / b^i) % b` (digit extraction for any valid digit list).
- [ ] `divDigits_getDigit` — the i-th quotient digit: `getDigit (divDigits b ds d).1 i = (ofDigits b ds / d / b^i) % b`.

---

## Proof Plan (named theorems)

1. `divDigitsMSB_correct (b : Nat) (ds : List Nat) (d r : Nat)` — induction on the MSB-first list.
   The step is pure algebra: with `t = r·b + c`,

   ```text
   (t/d) · b^n · d + (t % d) · b^n  =  ((t/d)·d + (t % d)) · b^n
                                    =  t · b^n                              -- Nat.mod_add_div
                                    =  (r·b + c) · b^n
                                    =  c · b^n + r · b^(n+1)
   ```

2. `divColumn` — one column step with `r < d` and `cᵢ < b`: `r·b + cᵢ < d·b`, hence
   `(r·b + cᵢ) div d < b` (via `Nat.div_lt_of_lt_mul`) and `(r·b + cᵢ) mod d < d` (via `Nat.mod_lt`).
   This is the *small-state* claim: the recurrence never leaves the `(b, d)`-bounded range.
   `divDigits_quotient_lt_base` / `divDigitsMSB_remainder_lt` are the induction of this over the list.

3. `divDigits_correct` — compose `divDigitsMSB_correct` (at `r = 0`) with
   `ofDigitsMSB_eq_reverse` on both the quotient and the dividend.

4. `divDigits_quotient_eq_div` / `divDigits_remainder_eq_mod` — from `a = q·d + r` with
   `r < d`, apply `Nat.mul_add_div` and `Nat.mul_add_mod` (plus `Nat.div_eq_of_lt`,
   `Nat.mod_eq_of_lt`) — the uniqueness half of Euclidean division.

5. `divDigits_getDigit` — compose `getDigit_ofDigits` (on the quotient digit list, which is valid
   by `divDigits_quotient_lt_base`) with `divDigits_quotient_eq_div`, mirroring Representation's
   `digits_getDigit` for the quotient.

---

## Open Questions / Risks

1. **Hypothesis shape.** Unlike add/sub/mul, division's divisor is a plain `Nat`, so the module's
   theorems carry `0 < d` (divisor positive) rather than a second digit-list bound. The base-`b`
   digit-validity hypothesis `∀ c ∈ ds, c < b` appears only in `divDigits_quotient_lt_base`.
2. **MSB-first vs LSB-first.** The fold is MSB-first, but the public interface is LSB-first to match
   `ofDigits`; `ofDigitsMSB` + `List.reverse` bridge the two. This is a proof-device choice,
   not a change to the library's digit convention.
3. **No normalization.** Because the running remainder `r < d` and each dividend digit `cᵢ < b`
   imply `t < d·b`, the quotient digit `t div d` is automatically a single base-`b` digit; no
   post-pass (like multiplication's `normalize`) is required.
