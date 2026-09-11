# Cycle-4 / round-09 — P1 numeric-boundary adversarial audit of the published AOT binary

**Role:** P1, the numeric-boundary attacker (falsifier). **Goal: break the binary, not praise it.**

**Target (exact, unchanged):** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5,673,984 bytes, LastWriteTime `09/11/2026 12:41:36`, published by round-08 §3 from HEAD `12cc863`).

**Ground truth:** SymPy 1.14.0 at `C:\Users\ricar\dev\.lovelace-tools\python\python3.exe`.
Every value called wrong below is checked against SymPy exact rationals (`Rational`, `N(..., k)`),
with the SymPy script and its raw output kept in `raw/sympy-verify.txt`, `raw/final-check.txt`,
`raw/final-check5.txt`.

**Method.** 222 probes, one `.ls` file each, run one process per probe through
`run-probes.ps1` (hard 60 s timeout, exit code captured via `System.Diagnostics.Process`).
Raw envelopes: `raw/batch1.txt` … `raw/batch5.txt`; the per-probe `.ls` sources are in
`probes/<batch>/`. Nothing in the repository's source or tests was touched; every file written
lives under `docs\goal-cycle-4\round-09\`.

In the table below, `EXE` abbreviates
`C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`, and every command has the form
`EXE --file docs\goal-cycle-4\round-09\probes\<batch>\<probe>.ls --omit-functions`.
`exact:` is the `result.structured.exact` field of the JSON envelope; `[Nch]` means the display
string is N characters long (head…tail shown). FINDINGS are reproduced with full commands and
verbatim output in the FINDINGS section.

## Summary

<!--COUNTS-->

**Headline:** the previously-fixed division defect is genuinely fixed — **every** `a/b` probe held,
including the neighbours of the old failure (`1/0.9`, `1/0.95`, `1/0.99`, `1/0.999`,
`1/0.999…9` at 36/37/38/39 nines, `1/1.000…01`, `1/(-1.0000001)`, `1/(1e-30)`, period-1/2/3/6
divisors 3, 11, 27, 7, 13, 21, 37, 101, and 39-significant-digit dividend/divisor literals).
**What is still broken is the inverse direction:** multiplying a periodic quotient back by its
divisor no longer round-trips. The shortest failing bodies are three tokens —
`(1/17)*17`, `(1/22)*22`, `(1/26)*26` all return `0.999…984` / `0.999…(8)` / `0.999…984` — and
even the decimal-free `(4/3)*(3/4)` returns `0.999…975`, while
`(1/0.95)*0.95` returns a 1001-digit rational `(225·10^1000 − 1)/(225·10^1000)`
that the envelope marks **`"exact":true`**. 62 of 222 probes are findings.

<!--TABLE-->

<!--PROSE-TAIL-->
## FINDINGS

### F1 — `"exact":true` on a value that is not the true value (false exactness), 9 probes
The envelope publishes a numerator/denominator pair and asserts exactness. SymPy shows the
published rational differs from the mathematically exact result and, in the `(a/b)*b` cases,
differs from `a`.

Reproduction (smallest command first):

```
> C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch5\i34_one_over_22_times_22.ls --omit-functions
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,
 "result":{"kind":"Real","display":"0.999…[1003 chars]…(8)",
 "structured":{"kind":"Real","value":"0.999…[1003 chars]…(8)","exact":true,
 "numerator":"899999…[999 digits]…9999","denominator":"9000000…[999 digits]…0000"}},…}
```

* `(1/22)*22` → published `899…9/900…0` (999 digits each, numerator = denominator − 1),
  i.e. `1 − 1/(9·10^999)`; the display ends `999…(8)`.
  SymPy: `Rational(1,22)*22 == 1`; `Rational(899…9, 900…0) == 1` is **False**;
  `1 − value = 1.11111e-999`. **Wrong value, falsely exact.**

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch1\d36_identity_one_over_0_95_times_0_95.ls --omit-functions
{"…","result":{"kind":"Real","display":"0.999999999999999999999999999999999999999999999999999999999999[1005 chars]9999999999999999999999(5)",
 "structured":{"kind":"Real","value":"0.999…[1005 chars]…(5)","exact":true,
 "numerator":"224999…[1001 digits]…9999","denominator":"225000…[1001 digits]…0000"}},…}
```

* `(1/0.95)*0.95` → `(225·10^1000 − 1)/(225·10^1000)` = `1 − 4.44444e-1001`, marked `exact:true`.
  SymPy: `Rational(1,Rational(95,100))*Rational(95,100) == 1`, `Rational(20,19)*Rational(19,20) == 1`;
  the published rational `== 1` is **False**. Full envelope in `raw/batch1.txt` (6,515 bytes).
* Same shape: `0.95*(1/0.95)` (m02), `x = 1/0.95; x*0.95` (m03), `(20/19)*(19/20)` (m04),
  `(1/0.55)*0.55` (m09), `((-1)/0.95)*0.95` (m45) — all publish `225·10^1000 − 1` over
  `225·10^1000` with `exact:true`;
  `(1/0.94)*0.94` (m19) publishes `(180·10^1000 − 1)/(180·10^1000)`, `exact:true`.
  Each is `≠ 1` by SymPy (`raw/final-check.txt`, section 1).

### F2 — wrong value (flagged `exact:false`, but still wrong), 32 probes
Same defect without the exactness lie: the published number is not the mathematical result.
Shortest failing bodies: `(1/17)*17`, `(1/22)*22`, `(1/26)*26` (all `1` by SymPy). Example pasted
in full (the 1004-character displays are in `raw/batch2.txt`):

```
> …\Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch4\k01_four_thirds_times_three_quarters.ls --omit-functions
  (body: (4/3)*(3/4))
  result.display = 0.999999999999999999999999999999999999999999999999999999999999999999999999999999[1004 chars]9999999999999999999999975
  structured.exact = false
```

* SymPy: `Rational(4,3)*Rational(3,4) == 1`; observed − 1 = `−2.5e-1001`.
* `(1/0.75)*0.75` (m07), `(1/0.35)*0.35` (m11), `(1/0.98)*0.98` (m16), `(1/0.97)*0.97` (m17),
  `(1/0.96)*0.96` (m18), `(1/0.71)*0.71` (i02), `(1/0.49)*0.49` (i06), `0.75*(1/0.75)` (i16),
  `(1/(0.75))*0.75` (i20), `(1/0.750)*0.750` (i26), `(1/0.75)*(3/4)` (i27), `(4/3)*0.75` (i28):
  all return `1 − δ`, `δ ∈ [1.5e-1001, 2.5e-1001]`.
* `(1/17)*17` (m29), `(1/23)*23` (m30), `(1/26)*26` (i35): `1 − 1.6e-999`.
  `(1/97)*97` (m34): `1 − 9.1e-999`. Full 1002-character displays in `raw/batch2.txt`, `raw/batch4.txt`.
* Short (102-char) reproductions, pasted verbatim:

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch5\i17_seven_over_0_75_times.ls --omit-functions
  (body: (7/0.75)*0.75)
  {"result":{"kind":"Real","display":"6.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999","structured":{"kind":"Real","value":"6.999…","exact":false}}}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch4\k05_two_over_17_times_17.ls --omit-functions
  (body: (2/17)*17)
  {"result":{"kind":"Real","display":"1.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999","structured":{"kind":"Real","value":"1.999…","exact":false}}}
```

  SymPy: `7` and `2` exactly; observed − truth = `−1.0e-100` in both.
* `(123/0.97)*0.97` (i19) → `122.999…9` [104 chars]; `((1/17)*17)*17` (k12) → `16.999…9`;
  `((1/0.75)*0.75)*1000000` (k10) → `999999.999…9`; `((1/17)*17) + 0` (k11) → `0.999…984`;
  `sqrt((1/0.75)*0.75)` (k19) → `0.999…9`; `((1/0.75)*0.75)^2` (k20) → `0.999…0625` [2006 chars];
  `a = 1/17; a*17` (k07) → `0.999…984`.
* `evalf((1/17)*17, 30)` (k13) and `evalf((1/17)*17, 40)` (k14) return the *same* 1002-character
  wrong value — the error is not a rounding artefact of the requested precision.
  Likewise `evalf((4/3)*(3/4), 19|20|40)` (i21–i23) are byte-identical to each other and all ≠ 1.

### F3 — `evalf` of the F1 value returns the wrong digits, 2 probes

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch2\m36_evalf_bad_product_30.ls --omit-functions
  (body: evalf((1/0.95)*0.95, 30))
  {"result":{"kind":"Real","display":"0.999999999999999999999999999999","structured":{"kind":"Real","value":"0.999999999999999999999999999999","exact":false}}}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch2\m37_evalf_bad_product_50.ls --omit-functions
  (body: evalf((1/0.95)*0.95, 50))
  {"result":{"kind":"Real","display":"0.99999999999999999999999999999999999999999999999999","structured":{"kind":"Real","value":"0.999…","exact":false}}}
```

SymPy: `N(1, 30) = 1.00000000000000000000000000000`,
`N(1, 50) = 1.0000000000000000000000000000000000000000000000000`. The engine answers
30 nines and 50 nines respectively: wrong in the last digit at both precisions.

### F4 — the same value is 1 for division/subtraction but ≠ 1 for comparison, 2 probes

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch2\m38_bad_product_minus_one.ls --omit-functions
  (body: ((1/0.95)*0.95) - 1)
  {"result":{"kind":"Real","display":"0","structured":{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}}}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch2\m44_one_over_0_95_times_0_95_div_2.ls --omit-functions
  (body: ((1/0.95)*0.95)/2)
  {"result":{"kind":"Real","display":"0.5","structured":{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}}}
> …\Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch2\m42_compare_product_to_one.ls --omit-functions
  (body: ((1/0.95)*0.95) == 1)   →  result.structured.value = false
```

The first two treat the F1 value as exactly 1 (`x − 1 = 0` `exact:true`, `x/2 = 1/2` `exact:true`)
while the product's own published numerator/denominator is `1 − 4.4e-1001` and `x == 1` is `false`.
Note the asymmetry inside F2 itself: for the 0.75 case the same subtraction does *not* collapse —
`((1/0.75)*0.75) - 1 = −2.5e-1001 [1005 chars]` (k08), i.e. correct subtraction, whereas the
0.95 case collapses to 0. Both cannot be right about the same identity.

### F5 — `0^(-1.0)` returns 0 with `"exact":true` while `0^(-1)` is an error, 1 probe

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p06_zero_pow_real_neg1.ls --omit-functions
  (body: 0^(-1.0))
  {"result":{"kind":"Real","display":"0","structured":{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}}}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p01_zero_pow_neg1.ls --omit-functions
  (body: 0^-1)    → {"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Base cannot be zero. (Parameter 'base')"}   exit 1
```

SymPy 1.14.0: `S.Zero**-1 = zoo`, `S.Zero**-1.0 = zoo`. The engine both refuses and "exactly
answers 0" for the same mathematical object, depending on whether the exponent is written as an
integer or a decimal. The `0` envelope is a numeric claim that is false.

### F6 — `^` rejects negative/fractional exponents inconsistently, 11 probes
Reproductions (each an error envelope, exit 1, verbatim message):

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p17_two_pow_half.ls --omit-functions
  (body: 2^0.5)          → {"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported."}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p18_two_pow_half_30.ls --omit-functions
  (body: evalf(2^0.5, 30)) → same error  (and evalf(sqrt(2), 60) works, 100 correct decimals)
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p07_neg8_pow_third.ls --omit-functions
  (body: (-8)^(1/3))     → same error
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p34_cube_root_negative_real.ls --omit-functions
  (body: (-27)^(1/3))    → same error
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p29_pow_neg1_nearzero.ls --omit-functions
  (body: (0.00000000000000000000000000000000000001)^-1) → {"…","message":"Negative exponents are not yet supported."}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p32_pow_periodic_base_neg1.ls --omit-functions
  (body: (1/3)^-1)       → "Negative exponents are not yet supported."
```

Yet the same engine answers `2^-1 = 0.5`, `2^-2 = 0.25`, `2^-1000` (1000 correct decimals),
`(-8)^0.5 = i*sqrt(8)` (p09), `(-4)^(1/2) = 2i` (p30). So a value is expected and refused for
`2^0.5` (SymPy `sqrt(2)`), `2^-3`-style Reals and `(1/3)^-1` (SymPy `3`), while the *harder*
negative-base fractional cases succeed. No wrong value is produced here — these are error
envelopes where a value is expected, which the audit brief counts as findings.

### F7 — values needing more than 1000 fractional digits silently become exactly 0, 5 probes

```
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch3\p14_two_pow_neg_100000.ls --omit-functions
  (body: 2^-100000)
  {"result":{"kind":"Real","display":"0","structured":{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}}}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch4\n08_two_pow_neg5000.ls --omit-functions
  (body: 2^-5000)   → {"…","value":"0","exact":true,"numerator":"0","denominator":"1"}
> …Lovelace.Run.exe --file docs\goal-cycle-4\round-09\probes\batch4\n12_ten_pow_neg1001.ls --omit-functions
  (body: 10^-1001)  → {"…","value":"0","exact":true,"numerator":"0","denominator":"1"}
```

SymPy (`raw/final-check.txt`, section 1): `2^-5000 = 7.079811261e-1506`,
`2^-30103 = 1.241768181e-9062`, `10^-1001 = 1.0e-1001` — none of them is 0.
The boundary is sharp and observable: `10^-1000` (n13) and `2^-1000` (n06) are returned in full
(1002-character exact decimals) while `10^-1001` (n12), `2^-5000` (n08), `2^-30103` (n09) and
`evalf(2^-30103, 30)` (n16) collapse to `0` **with `exact:true`** — the envelope asserts exactness
for a 0 that is wrong by 1.0e-1001 / 7.1e-1506 / 1.2e-9062.

## Observations that did NOT reach FINDING (recorded so they are not mistaken for held claims)

* **N1.** `inf()` (p20) is `Symbolic "inf"` and `(-1)*inf()` (p24) is `"-inf"`; `inf() - inf()` (p21)
  and `0*inf()` (p22) return the *unevaluated* expression `0*inf`, `inf() + inf()` (p23) returns
  `2*inf`, `1/inf()` (p25) returns `inf^-1`. SymPy says `nan`/`oo`/`0`. The engine asserts no number
  in these cases, so there is no value claim to falsify — marked INCONCLUSIVE rather than HELD.
* **N2.** `2^-1001` (n07) and `1/2^1001` (n11) are printed as 1000-decimal truncations (last digit
  dropped) and flagged `exact:false`. The first 1000 decimals match SymPy's truncation exactly, so
  this is honest truncation, not a wrong-value claim.
* **N3.** `Exactness is conservative in the other direction for >18-fractional-digit terminating
  values.` `2^-100` (n04), `10^-40` (p16), `2^-1000` (n06), `10^-999`/`10^-1000` (n14/n13) and the
  literal `0.999…99` with 38 nines (g03) are returned *exactly* yet flagged `exact:false`.
  Reading `Lovelace.Suite\StructuredProjection.cs:128-131`, `RealExact` is
  `IsZero || IsPeriodic || -Exponent <= 18`, so the flag is a heuristic, not a correctness
  assertion. It is the `true` side of that heuristic that F1/F5/F7 falsify.
* **N4.** `evalf(1/998001, 40)` (g11) returns 1001 decimals although 40 digits were requested;
  all 1001 decimals checked equal SymPy's first 1001 decimals of `1/998001`. Over-delivery, not a
  wrong value. `evalf(sqrt(2), 60)` (m51) prints 100 decimals and all 100 match SymPy
  (`raw/sympy-verify.txt`, section A). `evalf(1/3, k)` for k = 30, 37, 38, 39, 50, 51, 60, 100 is
  all 3s — correct at every k.
* **N5.** `ln(0)`/`ln(-1)` (p37/p38) are `DomainError` "Unknown function 'ln'": the function simply
  does not exist in this surface, so no numeric claim is made.

## What I could not test

1. **No hang, no crash was observed** in 222 probes (all exited 0 or 1 well inside the 60 s
   timeout; slowest envelope 980.27 ms, `evalf((1/3)^2,20)-evalf((1/3)^2,40)`), but I could not
   prove the absence of hangs beyond this probe set — in particular for values that force the
   O(n³) period search (`FindSmallestPeriod`) on long periods at the 1000-digit cap.
2. **Precision >1000 fractional digits is unreachable from the script surface.** `evalf`'s
   computation cap is tied to `MaxComputationDecimalPlaces = 1000` and I found no `setprecision`
   builtin in the registered function list, so I could not test whether a value that *is*
   representable at 1001+ digits (e.g. `2^-1001`, `10^-1001`) can be produced correctly by any
   route. F7 is exactly the consequence.
3. **No direct `Lovelace.Real` API access.** I may not touch the repository's source or tests, so
   `Real`, `LReal64` and `LReal128` were only exercised through the runner's expression surface;
   `LReal128`'s 38-significant-digit `Promote()` path and the `WorkingFractionalDigits = 37`
   expansion are therefore untested directly.
4. **No complex-argument numerics, no transcendental boundary sweep.** Only `exp(0)`, `sqrt(0)`,
   `sqrt(-0.0)`, `(-1)^(1/2)`, `(-4)^(1/2)`, `(-8)^0.5`, `pi`-free cases were probed; `cos`/`log`/
   `exp` at guard-digit boundaries are outside this persona's brief (round-07 owned the `cos` cost
   defect) and were not re-tested.
5. **The trigger set for F2 is characterised, not derived.** I sampled 31 `(a/b)*b` identities and
   11 fail; e.g. `(1/0.7)*0.7`, `(1/0.91)*0.91`, `(1/0.21)*0.21`, `(1/0.999)*0.999`,
   `(7/0.95)*0.95`, `(1/6)*3`, `(1/12)*12`, `(1/18)*18` hold while `(4/3)*(3/4)`, `(1/0.75)*0.75`,
   `(1/17)*17`, `(1/22)*22`, `(7/0.75)*0.75`, `(1/26)*26` fail — I did not derive the exact
   condition, so I cannot promise that unlisted `(a/b)*b` pairs are safe.
6. **Reproducibility across processes.** Every probe was run once. I did not run repeats to test
   for nondeterminism (`AsyncLocal` precision scopes exist in `Lovelace.Real`), and the binary was
   not rebuilt or modified — the published artifact was audited as received.
7. **Locale/culture.** The AOT publish uses `InvariantGlobalization=true`; behaviour under another
   culture (decimal separator) was not tested.
8. **`0/0`, `0^-1` as values rather than errors**: the engine has no `zoo`/`nan` real value, so
   SymPy's `zoo`/`nan` answers could not be compared like-for-like in F5; I compared the *claims*
   the engine does make (F5) rather than the missing ones.
