import LovelaceProofs.Basic

namespace Lovelace.Proofs

open Lovelace.Proofs

set_option linter.unusedVariables false
set_option linter.unusedSimpArgs false

/-- Base-`b` digit expansion of `n`, least-significant first. -/
def digits (b : Nat) (n : Nat) : List Nat :=
  if h0 : n = 0 then []
  else
    if hb : b ≤ 1 then [n]
    else (n % b) :: digits b (n / b)
termination_by n
decreasing_by
  simp_wf
  exact Nat.div_lt_self (Nat.pos_of_ne_zero h0) (Nat.lt_of_not_ge hb)

theorem not_le_one_of_two_le {b : Nat} (hb : 2 ≤ b) : ¬ b ≤ 1 := by
  intro h
  exact (by decide : ¬ 2 ≤ 1) (Nat.le_trans hb h)

@[simp] theorem digits_zero (b : Nat) : digits b 0 = [] := by
  simp [digits]

theorem ofDigits_digits (b n : Nat) (hb : 2 ≤ b) : ofDigits b (digits b n) = n := by
  refine Nat.strongRecOn n ?_
  intro n ih
  by_cases h0 : n = 0
  · simp [digits, h0]
  · have hb1 := not_le_one_of_two_le hb
    unfold digits
    simp [h0, hb1]
    rw [ih (n / b) (Nat.div_lt_self (Nat.pos_of_ne_zero h0) (Nat.lt_of_not_ge hb1))]
    exact Nat.mod_add_div n b

theorem digits_lt_base (b n : Nat) (hb : 2 ≤ b) : ∀ d ∈ digits b n, d < b := by
  refine Nat.strongRecOn n ?_
  intro n ih
  by_cases h0 : n = 0
  · simp [digits, h0]
  · intro d hd
    have hb1 := not_le_one_of_two_le hb
    have hbg : 1 < b := Nat.lt_of_not_ge hb1
    unfold digits at hd
    simp [h0, hb1] at hd
    rcases hd with rfl | hd
    · exact Nat.mod_lt n (Nat.lt_trans (by decide : 0 < 1) hbg)
    · exact ih (n / b) (Nat.div_lt_self (Nat.pos_of_ne_zero h0) hbg) d hd

theorem digits_getDigit (b n i : Nat) (hb : 2 ≤ b) :
    getDigit (digits b n) i = (n / b ^ i) % b := by
  induction i generalizing n with
  | zero =>
      by_cases h0 : n = 0
      · simp [h0, getDigit]
      · have hb1 := not_le_one_of_two_le hb
        unfold digits
        simp [h0, hb1, getDigit]
  | succ i ih =>
      by_cases h0 : n = 0
      · simp [h0, getDigit]
      · have hb1 := not_le_one_of_two_le hb
        unfold digits
        simp [h0, hb1, getDigit]
        change getDigit (digits b (n / b)) i = n / b ^ (i + 1) % b
        rw [ih (n / b)]
        rw [Nat.div_div_eq_div_mul]
        rw [show b * b ^ i = b ^ (i + 1) by rw [Nat.pow_succ, Nat.mul_comm]]

theorem digits_injective (b : Nat) (hb : 2 ≤ b) : Function.Injective (digits b) := by
  intro n m h
  calc
    n = ofDigits b (digits b n) := (ofDigits_digits b n hb).symm
    _ = ofDigits b (digits b m) := by rw [h]
    _ = m := ofDigits_digits b m hb

end Lovelace.Proofs
