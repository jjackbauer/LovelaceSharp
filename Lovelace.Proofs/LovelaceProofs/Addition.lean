import LovelaceProofs.Basic

namespace Lovelace.Proofs

open Lovelace.Proofs

set_option linter.unusedVariables false

/-- Carry-propagating digit addition (equal-length lists). Returns (result, final carry). -/
def addDigits (b : Nat) : List Nat → List Nat → Nat → List Nat × Nat
  | [],      [],      c => ([], c)
  | x :: xs, y :: ys, c =>
      let s := x + y + c
      let (r, c') := addDigits b xs ys (s / b)
      (s % b :: r, c')
  | _, _, c => ([], c)

theorem addDigits_correct (b : Nat) (cs ds : List Nat) (k : Nat)
    (hlen : cs.length = ds.length) :
    let z := addDigits b cs ds k
    ofDigits b z.1 + z.2 * b ^ cs.length = ofDigits b cs + ofDigits b ds + k := by
  induction cs generalizing ds k with
  | nil =>
      cases ds with
      | nil => simp [addDigits]
      | cons d ds => simp at hlen
  | cons x xs ih =>
      cases ds with
      | nil => simp at hlen
      | cons y ys =>
          have hlen' : xs.length = ys.length := Nat.succ.inj hlen
          simp [addDigits]
          let s := x + y + k
          let z := addDigits b xs ys (s / b)
          change (s % b + b * ofDigits b z.1 + z.2 * b ^ (xs.length + 1)) =
                 x + b * ofDigits b xs + (y + b * ofDigits b ys) + k
          have hrec : ofDigits b z.1 + z.2 * b ^ xs.length =
              ofDigits b xs + ofDigits b ys + s / b :=
            ih ys (s / b) hlen'
          have hfac : b * ofDigits b z.1 + z.2 * b ^ (xs.length + 1)
                      = b * (ofDigits b z.1 + z.2 * b ^ xs.length) := by
            rw [Nat.pow_succ, ← Nat.mul_assoc, Nat.mul_comm (z.2 * b ^ xs.length) b, ← Nat.mul_add]
          calc
            s % b + b * ofDigits b z.1 + z.2 * b ^ (xs.length + 1)
                = s % b + b * (ofDigits b z.1 + z.2 * b ^ xs.length) := by
                    rw [Nat.add_assoc, hfac]
            _ = s % b + b * (ofDigits b xs + ofDigits b ys + s / b) := by rw [hrec]
            _ = s % b + (b * ofDigits b xs + b * ofDigits b ys + b * (s / b)) := by
                    rw [Nat.mul_add, Nat.mul_add]
            _ = (s % b + b * (s / b)) + b * ofDigits b xs + b * ofDigits b ys := by ac_rfl
            _ = s + b * ofDigits b xs + b * ofDigits b ys := by rw [Nat.mod_add_div s b]
            _ = x + b * ofDigits b xs + (y + b * ofDigits b ys) + k := by
                    rw [show s = x + y + k by rfl]
                    ac_rfl

/-- With no final carry, the result is exactly the sum. -/
theorem addDigits_no_carry (b : Nat) (cs ds : List Nat) (hb : 2 ≤ b)
    (hltc : ∀ d ∈ cs, d < b) (hltd : ∀ d ∈ ds, d < b) (hlen : cs.length = ds.length)
    (hcarry : (addDigits b cs ds 0).2 = 0) :
    ofDigits b (addDigits b cs ds 0).1 = ofDigits b cs + ofDigits b ds := by
  have h := addDigits_correct b cs ds 0 hlen
  dsimp at h
  rw [hcarry] at h
  simpa using h

end Lovelace.Proofs
