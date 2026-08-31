import LovelaceProofs.Representation

namespace Lovelace.Proofs

open Lovelace.Proofs

/-- Coefficients of the (Cauchy) product of two digit lists, least-significant first. -/
def conv : List Nat → List Nat → List Nat
  | [],      _ => []
  | c :: cs, ds => addLists (ds.map fun d => c * d) (0 :: conv cs ds)

/-- Carry propagation of a coefficient list into canonical base-`b` digits. -/
private def normalizeAux (b : Nat) : List Nat → Nat → List Nat
  | [],      c => digits b c
  | x :: xs, c =>
      let t := x + c
      (t % b) :: normalizeAux b xs (t / b)

def normalize (b : Nat) (s : List Nat) : List Nat := normalizeAux b s 0

theorem ofDigits_conv (b : Nat) (cs ds : List Nat) :
    ofDigits b (conv cs ds) = ofDigits b cs * ofDigits b ds := by
  induction cs generalizing ds with
  | nil => simp [conv, ofDigits]
  | cons c cs ih =>
      simp [conv, ofDigits]
      rw [ofDigits_addLists]
      rw [ofDigits_map_mul]
      simp [ofDigits]
      rw [ih ds]
      rw [Nat.add_mul]
      ac_rfl

theorem normalizeAux_spec (b : Nat) (s : List Nat) (c : Nat) (hb : 2 ≤ b) :
    (∀ d ∈ normalizeAux b s c, d < b) ∧ ofDigits b (normalizeAux b s c) = ofDigits b s + c := by
  induction s generalizing c with
  | nil =>
      constructor
      · exact digits_lt_base b c hb
      · simp [normalizeAux, ofDigits_digits b c hb]
  | cons x xs ih =>
      have hbpos : 0 < b := Nat.lt_trans (by decide : 0 < 1) (Nat.lt_of_succ_le hb)
      constructor
      · intro d hd
        simp [normalizeAux] at hd
        rcases hd with rfl | hd
        · exact Nat.mod_lt (x + c) hbpos
        · exact (ih ((x + c) / b)).1 d hd
      · simp [normalizeAux]
        have hrec := (ih ((x + c) / b)).2
        rw [hrec]
        rw [Nat.mul_add]
        rw [Nat.add_assoc, Nat.add_comm (b * ofDigits b xs), ← Nat.add_assoc]
        rw [Nat.mod_add_div (x + c) b]
        ac_rfl

theorem normalize_correct (b : Nat) (s : List Nat) (hb : 2 ≤ b) :
    (∀ d ∈ normalize b s, d < b) ∧ ofDigits b (normalize b s) = ofDigits b s := by
  have h := normalizeAux_spec b s 0 hb
  simpa [normalize] using h

theorem mulDigits_correct (b : Nat) (cs ds : List Nat) (hb : 2 ≤ b) :
    ofDigits b (normalize b (conv cs ds)) = ofDigits b cs * ofDigits b ds := by
  rw [(normalize_correct b (conv cs ds) hb).2]
  exact ofDigits_conv b cs ds

end Lovelace.Proofs
