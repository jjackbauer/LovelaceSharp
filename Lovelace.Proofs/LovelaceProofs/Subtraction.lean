import LovelaceProofs.Basic

namespace Lovelace.Proofs

open Lovelace.Proofs

set_option linter.unusedVariables false
set_option linter.unusedSimpArgs false

/-- Borrow-propagating digit subtraction (equal-length lists). Returns (result, final borrow). -/
def subDigits (b : Nat) : List Nat → List Nat → Nat → List Nat × Nat
  | [],      [],      borrow => ([], borrow)
  | x :: xs, y :: ys, borrow =>
      let t := b + x - y - borrow
      let (r, c) := subDigits b xs ys (1 - t / b)
      (t % b :: r, c)
  | _, _, borrow => ([], borrow)

/-- Single-column subtraction identity. -/
theorem subColumn (b x y borrow : Nat) (hx : x < b) (hy : y < b) (hb : borrow ≤ 1) :
    x + b * (1 - (b + x - y - borrow) / b) = y + borrow + (b + x - y - borrow) % b := by
  let t := b + x - y - borrow
  have hbpos : 0 < b := by omega
  have hyle : y + borrow ≤ b + x := by omega
  have ht_le : t / b ≤ 1 := by
    have ht : t < 2 * b := by omega
    have hdiv : t / b < 2 := (Nat.div_lt_iff_lt_mul hbpos).2 (by omega)
    omega
  have hQb : b * (t / b) ≤ b := by
    simpa using Nat.mul_le_mul_left b ht_le
  have hQt : b * (t / b) ≤ t := by
    simpa [Nat.mul_comm] using Nat.div_mul_le_self t b
  have hsub : b * (1 - t / b) = b - b * (t / b) := by
    simpa using Nat.mul_sub_left_distrib b 1 (t / b)
  have hmod : t % b = t - b * (t / b) := by
    have hm := Nat.mod_add_div t b
    omega
  have hkey : x + b = y + borrow + t := by omega
  calc
    x + b * (1 - t / b)
        = x + (b - b * (t / b)) := by rw [hsub]
    _ = x + b - b * (t / b) := by omega
    _ = (y + borrow + t) - b * (t / b) := by rw [hkey]
    _ = y + borrow + (t - b * (t / b)) := by omega
    _ = y + borrow + t % b := by rw [hmod]

theorem subDigits_lt_base (b : Nat) (cs ds : List Nat) (borrow : Nat) (hbpos : 0 < b) :
    ∀ d ∈ (subDigits b cs ds borrow).1, d < b := by
  induction cs generalizing ds borrow with
  | nil => cases ds <;> simp [subDigits]
  | cons x xs ih =>
      cases ds with
      | nil => simp [subDigits]
      | cons y ys =>
          intro d hd
          simp [subDigits] at hd
          rcases hd with rfl | hd
          · exact Nat.mod_lt (b + x - y - borrow) hbpos
          · exact ih ys (1 - (b + x - y - borrow) / b) d hd

theorem subDigits_length (b : Nat) (cs ds : List Nat) (borrow : Nat) (hlen : cs.length = ds.length) :
    (subDigits b cs ds borrow).1.length = cs.length := by
  induction cs generalizing ds borrow with
  | nil =>
      cases ds with
      | nil => simp [subDigits]
      | cons d ds => simp at hlen
  | cons x xs ih =>
      cases ds with
      | nil => simp at hlen
      | cons y ys =>
          have hlen' : xs.length = ys.length := Nat.succ.inj hlen
          simp [subDigits, ih ys (1 - (b + x - y - borrow) / b) hlen']

theorem subDigits_correct (b : Nat) (cs ds : List Nat) (borrow0 : Nat)
    (hb0 : borrow0 ≤ 1) (hltc : ∀ d ∈ cs, d < b) (hltd : ∀ d ∈ ds, d < b)
    (hlen : cs.length = ds.length) :
    let z := subDigits b cs ds borrow0
    ofDigits b cs + z.2 * b ^ cs.length = ofDigits b ds + borrow0 + ofDigits b z.1 := by
  induction cs generalizing ds borrow0 with
  | nil =>
      cases ds with
      | nil => simp [subDigits]
      | cons d ds => simp at hlen
  | cons x xs ih =>
      cases ds with
      | nil => simp at hlen
      | cons y ys =>
          have hlen' : xs.length = ys.length := Nat.succ.inj hlen
          have hx : x < b := hltc x (by simp)
          have hy : y < b := hltd y (by simp)
          have hxs : ∀ d ∈ xs, d < b := by intro d hd; exact hltc d (by simp [hd])
          have hys : ∀ d ∈ ys, d < b := by intro d hd; exact hltd d (by simp [hd])
          simp [subDigits]
          let t := b + x - y - borrow0
          let z := subDigits b xs ys (1 - t / b)
          change (x + b * ofDigits b xs + z.2 * b ^ (xs.length + 1)) =
                 y + b * ofDigits b ys + borrow0 + (t % b + b * ofDigits b z.1)
          have hnb : 1 - t / b ≤ 1 := Nat.sub_le 1 (t / b)
          have hrec : ofDigits b xs + z.2 * b ^ xs.length =
              ofDigits b ys + (1 - t / b) + ofDigits b z.1 :=
            ih ys (1 - t / b) hnb hxs hys hlen'
          have hcol : x + b * (1 - t / b) = y + borrow0 + t % b :=
            subColumn b x y borrow0 hx hy hb0
          have hfac : b * ofDigits b xs + z.2 * b ^ (xs.length + 1)
                      = b * (ofDigits b xs + z.2 * b ^ xs.length) := by
            rw [Nat.pow_succ, ← Nat.mul_assoc, Nat.mul_comm (z.2 * b ^ xs.length) b, ← Nat.mul_add]
          calc
            x + b * ofDigits b xs + z.2 * b ^ (xs.length + 1)
                = x + b * (ofDigits b xs + z.2 * b ^ xs.length) := by
                    rw [Nat.add_assoc, hfac]
            _ = x + b * (ofDigits b ys + (1 - t / b) + ofDigits b z.1) := by rw [hrec]
            _ = x + b * ofDigits b ys + b * (1 - t / b) + b * ofDigits b z.1 := by
                    rw [Nat.mul_add, Nat.mul_add]
                    ac_rfl
            _ = (x + b * (1 - t / b)) + b * ofDigits b ys + b * ofDigits b z.1 := by ac_rfl
            _ = (y + borrow0 + t % b) + b * ofDigits b ys + b * ofDigits b z.1 := by rw [hcol]
            _ = y + b * ofDigits b ys + borrow0 + (t % b + b * ofDigits b z.1) := by ac_rfl

/-- When the minuend is large enough, subtraction has no final borrow and gives the difference. -/
theorem subDigits_no_borrow (b : Nat) (cs ds : List Nat) (hb : 2 ≤ b)
    (hltc : ∀ d ∈ cs, d < b) (hltd : ∀ d ∈ ds, d < b) (hlen : cs.length = ds.length)
    (hle : ofDigits b ds ≤ ofDigits b cs) :
    (subDigits b cs ds 0).2 = 0 ∧
      ofDigits b (subDigits b cs ds 0).1 = ofDigits b cs - ofDigits b ds := by
  have hbpos : 0 < b := Nat.lt_trans (by decide : 0 < 1) (Nat.lt_of_succ_le hb)
  have h := subDigits_correct b cs ds 0 (by decide) hltc hltd hlen
  dsimp at h
  have hlenres : (subDigits b cs ds 0).fst.length = cs.length := subDigits_length b cs ds 0 hlen
  have hltres : ∀ d ∈ (subDigits b cs ds 0).fst, d < b := subDigits_lt_base b cs ds 0 hbpos
  have hltres' : ofDigits b (subDigits b cs ds 0).fst < b ^ cs.length := by
    simpa [hlenres] using ofDigits_lt_pow b (subDigits b cs ds 0).fst hbpos hltres
  have hbout0 : (subDigits b cs ds 0).snd = 0 := by
    by_cases hz : (subDigits b cs ds 0).snd = 0
    · exact hz
    · exfalso
      have hpos : 1 ≤ (subDigits b cs ds 0).snd := Nat.succ_le_of_lt (Nat.pos_of_ne_zero hz)
      have hmul : b ^ cs.length ≤ (subDigits b cs ds 0).snd * b ^ cs.length := by
        simpa [Nat.mul_comm] using Nat.mul_le_mul_right (b ^ cs.length) hpos
      have hle2 : ofDigits b cs + b ^ cs.length ≤ ofDigits b ds + ofDigits b (subDigits b cs ds 0).fst := by
        rw [← h]
        exact Nat.add_le_add_left hmul (ofDigits b cs)
      have hlt2 : ofDigits b ds + ofDigits b (subDigits b cs ds 0).fst < ofDigits b cs + b ^ cs.length := by
        omega
      omega
  constructor
  · exact hbout0
  · have h' : ofDigits b cs = ofDigits b ds + ofDigits b (subDigits b cs ds 0).fst := by
      rw [hbout0] at h
      simpa using h
    omega

end Lovelace.Proofs
