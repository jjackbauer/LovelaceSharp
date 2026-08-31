import LovelaceProofs.Basic

namespace Lovelace.Proofs

open Lovelace.Proofs

set_option linter.unusedVariables false
set_option linter.unusedSimpArgs false

/-- Evaluate a most-significant-first digit list in base `b`. -/
def ofDigitsMSB (b : Nat) : List Nat → Nat
  | []      => 0
  | d :: ds => d * b ^ ds.length + ofDigitsMSB b ds

/-- MSB-first long division with a running remainder accumulator. -/
def divDigitsMSB (b : Nat) : List Nat → Nat → Nat → List Nat × Nat
  | [],      d, r => ([], r)
  | c :: cs, d, r =>
      let t := r * b + c
      let z := divDigitsMSB b cs d (t % d)
      (t / d :: z.1, z.2)

/-- Long division, least-significant-first in and out (reverses internally). -/
def divDigits (b : Nat) (ds : List Nat) (d : Nat) : List Nat × Nat :=
  let z := divDigitsMSB b ds.reverse d 0
  (z.1.reverse, z.2)

/-- Running remainder of MSB-first long division of `ds` by `d`, starting from `r`. -/
def divRemMSB (b d : Nat) : List Nat → Nat → Nat
  | [],      r => r
  | c :: cs, r => divRemMSB b d cs ((r * b + c) % d)

theorem ofDigits_append (b : Nat) (xs ys : List Nat) :
    ofDigits b (xs ++ ys) = ofDigits b xs + b ^ xs.length * ofDigits b ys := by
  induction xs with
  | nil => simp [ofDigits]
  | cons x xs ih =>
      simp [ofDigits, ih, Nat.mul_add]
      rw [Nat.pow_succ]
      ac_rfl

theorem ofDigitsMSB_eq_reverse (b : Nat) (l : List Nat) :
    ofDigitsMSB b l = ofDigits b l.reverse := by
  induction l with
  | nil => simp [ofDigitsMSB]
  | cons d ds ih =>
      simp [ofDigitsMSB, ofDigits_append, ih, List.length_reverse]
      rw [Nat.mul_comm]
      ac_rfl

/-- The units digit of a valid digit list is the value modulo the base. -/
theorem ofDigits_mod_eq_getDigit (b : Nat) (l : List Nat) (hlt : ∀ d ∈ l, d < b) :
    ofDigits b l % b = getDigit l 0 := by
  cases l with
  | nil => simp [ofDigits, getDigit]
  | cons d ds =>
      have hd : d < b := hlt d (by simp)
      simp [ofDigits, getDigit]
      exact Nat.mod_eq_of_lt hd

/-- Dividing a valid digit list's value by the base drops the units digit. -/
theorem ofDigits_div_eq_ofDigits_tail (b : Nat) (l : List Nat) (hb : 2 ≤ b) (hlt : ∀ d ∈ l, d < b) :
    ofDigits b l / b = ofDigits b l.tail := by
  cases l with
  | nil => simp [ofDigits]
  | cons d ds =>
      have hd : d < b := hlt d (by simp)
      have hbpos : 0 < b := Nat.lt_trans (by decide : 0 < 1) (Nat.lt_of_succ_le hb)
      simp [ofDigits]
      rw [Nat.add_mul_div_left d (ofDigits b ds) hbpos]
      rw [Nat.div_eq_of_lt hd]
      simp

/-- The i-th digit of a valid digit list is (value / b^i) % b. -/
theorem getDigit_ofDigits (b : Nat) (l : List Nat) (i : Nat) (hb : 2 ≤ b) (hlt : ∀ d ∈ l, d < b) :
    getDigit l i = (ofDigits b l / b ^ i) % b := by
  induction i generalizing l with
  | zero =>
      simpa using (ofDigits_mod_eq_getDigit b l hlt).symm
  | succ i ih =>
      cases l with
      | nil => simp [ofDigits, getDigit]
      | cons d ds =>
          have hd : d < b := hlt d (by simp)
          have hds : ∀ e ∈ ds, e < b := by intro e he; exact hlt e (by simp [he])
          have hbpos : 0 < b := Nat.lt_trans (by decide : 0 < 1) (Nat.lt_of_succ_le hb)
          simp [ofDigits, getDigit]
          rw [Nat.pow_succ, Nat.mul_comm (b ^ i) b]
          rw [← Nat.div_div_eq_div_mul (d + b * ofDigits b ds) b (b ^ i)]
          rw [Nat.add_mul_div_left d (ofDigits b ds) hbpos]
          rw [Nat.div_eq_of_lt hd]
          simp
          exact ih ds hds

@[simp] theorem divDigitsMSB_length (b : Nat) (ds : List Nat) (d r : Nat) :
    (divDigitsMSB b ds d r).1.length = ds.length := by
  induction ds generalizing r with
  | nil => simp [divDigitsMSB]
  | cons c cs ih => simp [divDigitsMSB, ih]

theorem mul_pow_add (r b c n : Nat) : (r * b + c) * b ^ n = c * b ^ n + r * b ^ (n + 1) := by
  have hpow : b * b ^ n = b ^ (n + 1) := by
    rw [Nat.mul_comm]
    exact (Nat.pow_succ b n).symm
  rw [Nat.add_mul, Nat.mul_assoc, hpow]
  ac_rfl

theorem divStep (b d r c n ofcs : Nat) :
    (r * b + c) / d * b ^ n * d + (ofcs + (r * b + c) % d * b ^ n)
      = c * b ^ n + ofcs + r * b ^ (n + 1) := by
  let t := r * b + c
  have ht : (t / d) * d + (t % d) = t := by
    rw [Nat.mul_comm]
    exact Nat.div_add_mod t d
  calc
    (t / d) * b ^ n * d + (ofcs + (t % d) * b ^ n)
        = ofcs + ((t / d) * b ^ n * d + (t % d) * b ^ n) := by ac_rfl
    _ = ofcs + (((t / d) * d + (t % d)) * b ^ n) := by
            rw [Nat.add_mul]
            ac_rfl
    _ = ofcs + (t * b ^ n) := by simp [ht]
    _ = ofcs + ((r * b + c) * b ^ n) := rfl
    _ = c * b ^ n + ofcs + r * b ^ (n + 1) := by
            rw [mul_pow_add]
            ac_rfl

theorem divDigitsMSB_correct (b : Nat) (ds : List Nat) (d r : Nat) :
    ofDigitsMSB b (divDigitsMSB b ds d r).1 * d + (divDigitsMSB b ds d r).2
      = ofDigitsMSB b ds + r * b ^ ds.length := by
  induction ds generalizing r with
  | nil => simp [divDigitsMSB, ofDigitsMSB]
  | cons c cs ih =>
      have hrec := ih ((r * b + c) % d)
      simp [divDigitsMSB, ofDigitsMSB] at hrec ⊢
      rw [Nat.add_mul, Nat.add_assoc, hrec]
      exact divStep b d r c cs.length (ofDigitsMSB b cs)

/-- One long-division column: with the running remainder `r < d` and the digit `c < b`, the
    quotient digit `(r·b + c) / d` is `< b` and the new remainder `(r·b + c) % d` is `< d`, so the
    running state never leaves the `(b, d)`-bounded range. -/
theorem divColumn (b d r c : Nat) (hd : 0 < d) (hr : r < d) (hc : c < b) :
    (r * b + c) / d < b ∧ (r * b + c) % d < d := by
  constructor
  · apply Nat.div_lt_of_lt_mul
    have hr1 : r + 1 ≤ d := Nat.succ_le_of_lt hr
    have h2 : r * b + b ≤ d * b := by
      have h := Nat.mul_le_mul_right b hr1
      simpa [Nat.add_mul, Nat.one_mul] using h
    have h1 : r * b + c < r * b + b := Nat.add_lt_add_left hc (r * b)
    exact Nat.lt_of_lt_of_le h1 h2
  · exact Nat.mod_lt (r * b + c) hd

theorem divDigitsMSB_quotient_lt_base (b : Nat) (ds : List Nat) (d r : Nat)
    (hd : 0 < d) (hr : r < d) (hlt : ∀ c ∈ ds, c < b) :
    ∀ q ∈ (divDigitsMSB b ds d r).1, q < b := by
  induction ds generalizing r with
  | nil => simp [divDigitsMSB]
  | cons c cs ih =>
      have hc : c < b := hlt c (by simp)
      have hcs : ∀ e ∈ cs, e < b := by
        intro e he
        exact hlt e (by simp [he])
      have hrec := ih ((r * b + c) % d) (Nat.mod_lt (r * b + c) hd) hcs
      intro q hq
      simp [divDigitsMSB] at hq
      rcases hq with rfl | hq
      · exact (divColumn b d r c hd hr hc).1
      · exact hrec q hq

theorem divDigitsMSB_remainder_lt (b : Nat) (ds : List Nat) (d r : Nat)
    (hd : 0 < d) (hr : r < d) :
    (divDigitsMSB b ds d r).2 < d := by
  induction ds generalizing r with
  | nil => simpa [divDigitsMSB] using hr
  | cons c cs ih =>
      simp [divDigitsMSB]
      exact ih ((r * b + c) % d) (Nat.mod_lt (r * b + c) hd)

@[simp] theorem divRemMSB_eq_snd (b d : Nat) (ds : List Nat) (r : Nat) :
    divRemMSB b d ds r = (divDigitsMSB b ds d r).2 := by
  induction ds generalizing r with
  | nil => simp [divRemMSB, divDigitsMSB]
  | cons c cs ih => simp [divRemMSB, divDigitsMSB, ih]

/-- The running remainder of the MSB-first fold always stays `< d`. -/
theorem divRemMSB_remainder_lt (b d : Nat) (ds : List Nat) (r : Nat) (hd : 0 < d) (hr : r < d) :
    divRemMSB b d ds r < d := by
  simpa [divRemMSB_eq_snd] using divDigitsMSB_remainder_lt b ds d r hd hr

theorem divDigits_quotient_lt_base (b : Nat) (ds : List Nat) (d : Nat)
    (hd : 0 < d) (hlt : ∀ c ∈ ds, c < b) :
    ∀ q ∈ (divDigits b ds d).1, q < b := by
  change ∀ q ∈ (divDigitsMSB b ds.reverse d 0).1.reverse, q < b
  have hlt' : ∀ c ∈ ds.reverse, c < b := by
    intro c hc
    exact hlt c ((List.mem_reverse).1 hc)
  have hspec := divDigitsMSB_quotient_lt_base b ds.reverse d 0 hd hd hlt'
  intro q hq
  exact hspec q ((List.mem_reverse).1 hq)

theorem divDigits_remainder_lt (b : Nat) (ds : List Nat) (d : Nat) (hd : 0 < d) :
    (divDigits b ds d).2 < d := by
  change (divDigitsMSB b ds.reverse d 0).2 < d
  exact divDigitsMSB_remainder_lt b ds.reverse d 0 hd hd

theorem divDigits_correct (b : Nat) (ds : List Nat) (d : Nat) :
    ofDigits b (divDigits b ds d).1 * d + (divDigits b ds d).2 = ofDigits b ds := by
  change ofDigits b (divDigitsMSB b ds.reverse d 0).1.reverse * d + (divDigitsMSB b ds.reverse d 0).2 = ofDigits b ds
  calc
    ofDigits b (divDigitsMSB b ds.reverse d 0).1.reverse * d + (divDigitsMSB b ds.reverse d 0).2
        = ofDigitsMSB b (divDigitsMSB b ds.reverse d 0).1 * d + (divDigitsMSB b ds.reverse d 0).2 := by
            rw [← ofDigitsMSB_eq_reverse]
    _ = ofDigitsMSB b ds.reverse := by
            have h := divDigitsMSB_correct b ds.reverse d 0
            simpa using h
    _ = ofDigits b ds := by
            rw [ofDigitsMSB_eq_reverse]
            simp [List.reverse_reverse]

theorem divDigits_quotient_eq_div (b : Nat) (ds : List Nat) (d : Nat) (hd : 0 < d) :
    ofDigits b (divDigits b ds d).1 = ofDigits b ds / d := by
  let q := ofDigits b (divDigits b ds d).1
  let r := (divDigits b ds d).2
  let a := ofDigits b ds
  have hcorrect : q * d + r = a := divDigits_correct b ds d
  have hrem : r < d := divDigits_remainder_lt b ds d hd
  have ha : d * q + r = a := by
    simpa [Nat.mul_comm] using hcorrect
  have hdiv : (d * q + r) / d = q + r / d := Nat.mul_add_div hd q r
  have hrdiv : r / d = 0 := Nat.div_eq_of_lt hrem
  have hq : q = a / d := by
    calc
      q = q + 0 := by simp
      _ = q + r / d := by rw [hrdiv]
      _ = (d * q + r) / d := by rw [hdiv]
      _ = a / d := by rw [ha]
  simpa [q, a] using hq

theorem divDigits_remainder_eq_mod (b : Nat) (ds : List Nat) (d : Nat) (hd : 0 < d) :
    (divDigits b ds d).2 = ofDigits b ds % d := by
  let q := ofDigits b (divDigits b ds d).1
  let r := (divDigits b ds d).2
  let a := ofDigits b ds
  have hcorrect : q * d + r = a := divDigits_correct b ds d
  have hrem : r < d := divDigits_remainder_lt b ds d hd
  have ha : d * q + r = a := by
    simpa [Nat.mul_comm] using hcorrect
  have hmod : (d * q + r) % d = r % d := Nat.mul_add_mod d q r
  have hrmod : r % d = r := Nat.mod_eq_of_lt hrem
  have hr : r = a % d := by
    calc
      r = r % d := (hrmod).symm
      _ = (d * q + r) % d := by rw [hmod]
      _ = a % d := by rw [ha]
  simpa [r, a] using hr

/-- The i-th least-significant digit of the quotient equals (a / d / b^i) % b. -/
theorem divDigits_getDigit (b : Nat) (ds : List Nat) (d : Nat) (i : Nat)
    (hb : 2 ≤ b) (hd : 0 < d) (hlt : ∀ c ∈ ds, c < b) :
    getDigit (divDigits b ds d).1 i = (ofDigits b ds / d / b ^ i) % b := by
  have hlt_q : ∀ q ∈ (divDigits b ds d).1, q < b := divDigits_quotient_lt_base b ds d hd hlt
  rw [getDigit_ofDigits b (divDigits b ds d).1 i hb hlt_q]
  rw [divDigits_quotient_eq_div b ds d hd]

end Lovelace.Proofs
