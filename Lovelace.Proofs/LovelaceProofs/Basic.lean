namespace Lovelace.Proofs

/-- Evaluate a least-significant-first digit list in base `b`. -/
def ofDigits (b : Nat) : List Nat → Nat
  | []      => 0
  | d :: ds => d + b * ofDigits b ds

/-- The `i`-th element of `l`, or `0` if out of range. -/
def getDigit (l : List Nat) (i : Nat) : Nat :=
  (l[i]?).getD 0

/-- Elementwise sum of two coefficient lists, zero-padding the shorter side. -/
def addLists : List Nat → List Nat → List Nat
  | [],      ys      => ys
  | xs,      []      => xs
  | x :: xs, y :: ys => (x + y) :: addLists xs ys

@[simp] theorem ofDigits_nil (b : Nat) : ofDigits b [] = 0 := rfl

@[simp] theorem ofDigits_cons (b : Nat) (d : Nat) (ds : List Nat) :
    ofDigits b (d :: ds) = d + b * ofDigits b ds := rfl

theorem ofDigits_map_mul (b c : Nat) (ds : List Nat) :
    ofDigits b (ds.map fun d => c * d) = c * ofDigits b ds := by
  induction ds with
  | nil => simp
  | cons d ds ih =>
      simp [ofDigits, ih, Nat.mul_add]
      ac_rfl

theorem ofDigits_addLists (b : Nat) (xs ys : List Nat) :
    ofDigits b (addLists xs ys) = ofDigits b xs + ofDigits b ys := by
  induction xs generalizing ys with
  | nil => simp [addLists]
  | cons x xs ih =>
      cases ys with
      | nil => simp [addLists]
      | cons y ys =>
          simp [addLists, ofDigits, ih, Nat.mul_add]
          ac_rfl

/-- A digit list of length `n` with every digit `< b` has value `< b^n`. -/
theorem ofDigits_lt_pow (b : Nat) (l : List Nat) (_hb : 0 < b) (hlt : ∀ d ∈ l, d < b) :
    ofDigits b l < b ^ l.length := by
  induction l with
  | nil => simp [ofDigits]
  | cons d ds ih =>
      have hd : d < b := hlt d (by simp)
      have hds : ∀ e ∈ ds, e < b := by intro e he; exact hlt e (by simp [he])
      have ih' : ofDigits b ds < b ^ ds.length := ih hds
      have hsucc : ofDigits b ds + 1 ≤ b ^ ds.length := Nat.succ_le_of_lt ih'
      have hmul : b * ofDigits b ds + b ≤ b * b ^ ds.length := by
        simpa [Nat.mul_add, Nat.mul_one] using Nat.mul_le_mul_left b hsucc
      have hmid : b + b * ofDigits b ds ≤ b * b ^ ds.length := by
        simpa [Nat.add_comm] using hmul
      have hlast : b * b ^ ds.length = b ^ (ds.length + 1) := by
        rw [Nat.pow_succ, Nat.mul_comm]
      simp [ofDigits]
      calc
        d + b * ofDigits b ds < b + b * ofDigits b ds := Nat.add_lt_add_right hd _
        _ ≤ b * b ^ ds.length := hmid
        _ = b ^ (ds.length + 1) := hlast

end Lovelace.Proofs
