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

**222 probes: 156 HELD, 62 FINDING, 4 INCONCLUSIVE.**

| finding | probes | one line |
|---|---|---|
| F1 | 9 | `(a/b)*b` returns a wrong rational that the envelope marks `"exact":true` |
| F2 | 32 | `(a/b)*b` returns a wrong rational (flagged `exact:false`) |
| F3 | 2 | `evalf` of an F1 value returns nines instead of 1 at the requested precision |
| F4 | 2 | the F1 value behaves as exactly 1 in `-1` and `/2` but `== 1` is false |
| F5 | 1 | `0^(-1.0)` returns 0 with `"exact":true` while `0^-1` is an error |
| F6 | 11 | `^` refuses negative/fractional exponents it elsewhere supports |
| F7 | 5 | values needing >1000 fractional digits return 0 with `"exact":true` |

**`exact:true` audit:** 130 probes returned `"exact":true`. 15 of them are demonstrated wrong against SymPy (F1, F5, F7). 115 more were compared digit-for-digit to SymPy and are exactly right. 2 are symbolic results (`i`, `2i`) whose `exact:true` matches SymPy's `I`/`2*I`. 

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

| probe | command | observed | SymPy value (if applicable) | verdict |
|---|---|---|---|---|
| d01_one_over_0_9 | `EXE --file …/probes/batch1/d01_one_over_0_9.ls --omit-functions` | Real `1.(1)` exact:`true` | 10/9 = 1.11111111111111111 | HELD |
| d02_one_over_0_99 | `EXE --file …/probes/batch1/d02_one_over_0_99.ls --omit-functions` | Real `1.(01)` exact:`true` | 100/99 = 1.01010101010101010 | HELD |
| d03_one_over_0_999 | `EXE --file …/probes/batch1/d03_one_over_0_999.ls --omit-functions` | Real `1.(001)` exact:`true` | 1000/999 = 1.00100100100100100 | HELD |
| d04_one_over_0_9999 | `EXE --file …/probes/batch1/d04_one_over_0_9999.ls --omit-functions` | Real `1.(0001)` exact:`true` | 10000/9999 = 1.00010001000100010 | HELD |
| d05_one_over_0_95 | `EXE --file …/probes/batch1/d05_one_over_0_95.ls --omit-functions` | Real `1.(052631578947368421)` exact:`true` | 20/19 = 1.05263157894736842 | HELD |
| d06_one_over_near1_38nines | `EXE --file …/probes/batch1/d06_one_over_near1_38nines.ls --omit-functions` | Real `1.(00000000000000000000000000000000000001)` exact:`true` | 10^38/(10^38-1) | HELD |
| d07_one_over_near1_39nines | `EXE --file …/probes/batch1/d07_one_over_near1_39nines.ls --omit-functions` | Real `1.(000000000000000000000000000000000000001)` exact:`true` | 10^39/(10^39-1) | HELD |
| d08_one_over_1_1 | `EXE --file …/probes/batch1/d08_one_over_1_1.ls --omit-functions` | Real `0.(90)` exact:`true` | 10/11 = 0.909090909090909091 | HELD |
| d09_one_over_1_01 | `EXE --file …/probes/batch1/d09_one_over_1_01.ls --omit-functions` | Real `0.(9900)` exact:`true` | 100/101 = 0.990099009900990099 | HELD |
| d10_one_over_1_plus_1e38 | `EXE --file …/probes/batch1/d10_one_over_1_plus_1e38.ls --omit-functions` | Real `0.(9999999999999999999…000000000) [80ch]` exact:`true` | 10^38/(10^38+1) | HELD |
| d11_one_over_1_plus_tiny | `EXE --file …/probes/batch1/d11_one_over_1_plus_tiny.ls --omit-functions` | Real `0.(9999999999999999999…000000000) [82ch]` exact:`true` | 10^39/(10^39+1) | HELD |
| d12_one_over_1 | `EXE --file …/probes/batch1/d12_one_over_1.ls --omit-functions` | Natural `1` exact:`true` | 1 | HELD |
| d13_one_over_0_5 | `EXE --file …/probes/batch1/d13_one_over_0_5.ls --omit-functions` | Real `2` exact:`true` | 2 | HELD |
| d14_one_over_2 | `EXE --file …/probes/batch1/d14_one_over_2.ls --omit-functions` | Real `0.5` exact:`true` | 1/2 = 0.500000000000000000 | HELD |
| d15_one_over_3 | `EXE --file …/probes/batch1/d15_one_over_3.ls --omit-functions` | Real `0.(3)` exact:`true` | 1/3 = 0.333333333333333333 | HELD |
| d16_one_over_11 | `EXE --file …/probes/batch1/d16_one_over_11.ls --omit-functions` | Real `0.(09)` exact:`true` | 1/11 = 0.0909090909090909091 | HELD |
| d17_one_over_27 | `EXE --file …/probes/batch1/d17_one_over_27.ls --omit-functions` | Real `0.(037)` exact:`true` | 1/27 = 0.0370370370370370370 | HELD |
| d18_one_over_7 | `EXE --file …/probes/batch1/d18_one_over_7.ls --omit-functions` | Real `0.(142857)` exact:`true` | 1/7 = 0.142857142857142857 | HELD |
| d19_one_over_13 | `EXE --file …/probes/batch1/d19_one_over_13.ls --omit-functions` | Real `0.(076923)` exact:`true` | 1/13 = 0.0769230769230769231 | HELD |
| d20_one_over_21 | `EXE --file …/probes/batch1/d20_one_over_21.ls --omit-functions` | Real `0.(047619)` exact:`true` | 1/21 = 0.0476190476190476190 | HELD |
| d21_one_over_37 | `EXE --file …/probes/batch1/d21_one_over_37.ls --omit-functions` | Real `0.(027)` exact:`true` | 1/37 = 0.0270270270270270270 | HELD |
| d22_one_over_101 | `EXE --file …/probes/batch1/d22_one_over_101.ls --omit-functions` | Real `0.(0099)` exact:`true` | 1/101 = 0.00990099009900990099 | HELD |
| d23_ten_over_9 | `EXE --file …/probes/batch1/d23_ten_over_9.ls --omit-functions` | Real `1.(1)` exact:`true` | 10/9 = 1.11111111111111111 | HELD |
| d24_hundred_over_99 | `EXE --file …/probes/batch1/d24_hundred_over_99.ls --omit-functions` | Real `1.(01)` exact:`true` | 100/99 = 1.01010101010101010 | HELD |
| d25_thousand_over_999 | `EXE --file …/probes/batch1/d25_thousand_over_999.ls --omit-functions` | Real `1.(001)` exact:`true` | 1000/999 = 1.00100100100100100 | HELD |
| d26_two_over_3 | `EXE --file …/probes/batch1/d26_two_over_3.ls --omit-functions` | Real `0.(6)` exact:`true` | 2/3 = 0.666666666666666667 | HELD |
| d27_one_over_neg_0_9 | `EXE --file …/probes/batch1/d27_one_over_neg_0_9.ls --omit-functions` | Real `-1.(1)` exact:`true` | -10/9 = -1.11111111111111111 | HELD |
| d28_neg_one_over_0_9 | `EXE --file …/probes/batch1/d28_neg_one_over_0_9.ls --omit-functions` | Real `-1.(1)` exact:`true` | -10/9 = -1.11111111111111111 | HELD |
| d29_one_over_neg_1_0000001 | `EXE --file …/probes/batch1/d29_one_over_neg_1_0000001.ls --omit-functions` | Real `-0.(99999990000000)` exact:`true` | -10^7/(10^7+1) | HELD |
| d30_one_over_1e-30 | `EXE --file …/probes/batch1/d30_one_over_1e-30.ls --omit-functions` | Real `1000000000000000000000000000000` exact:`true` | 10000000000000… = 1.00000000000000000E+30 | HELD |
| d31_one_over_0_999999999999999999999999999999999999 | `EXE --file …/probes/batch1/d31_one_over_0_999999999999999999999999999999999999.ls --omit-functions` | Real `1.(000000000000000000000000000000000001)` exact:`true` | 10^36/(10^36-1) | HELD |
| d32_0_9_over_0_9999 | `EXE --file …/probes/batch1/d32_0_9_over_0_9999.ls --omit-functions` | Real `0.(9000)` exact:`true` | 1000/1111 = 0.900090009000900090 | HELD |
| d33_identity_a_over_b_mul_b_period3 | `EXE --file …/probes/batch1/d33_identity_a_over_b_mul_b_period3.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| d34_identity_a_mul_recip_period3 | `EXE --file …/probes/batch1/d34_identity_a_mul_recip_period3.ls --omit-functions` | Real `0.(037)` exact:`true` | 1/27 = 0.0370370370370370370 | HELD |
| d35_identity_one_over_0_9_times_0_9 | `EXE --file …/probes/batch1/d35_identity_one_over_0_9_times_0_9.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| d36_identity_one_over_0_95_times_0_95 | `EXE --file …/probes/batch1/d36_identity_one_over_0_95_times_0_95.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| d37_identity_one_over_7_times_7 | `EXE --file …/probes/batch1/d37_identity_one_over_7_times_7.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| d38_identity_one_over_3_times_3 | `EXE --file …/probes/batch1/d38_identity_one_over_3_times_3.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| d39_a_over_b_times_b_mixed | `EXE --file …/probes/batch1/d39_a_over_b_times_b_mixed.ls --omit-functions` | Real `7` exact:`true` | 7 | HELD |
| d40_div_exactness_rational | `EXE --file …/probes/batch1/d40_div_exactness_rational.ls --omit-functions` | Real `0.25` exact:`true` | 1/4 = 0.250000000000000000 | HELD |
| d41_div_zero | `EXE --file …/probes/batch1/d41_div_zero.ls --omit-functions` | ERROR `DivisionByZero` — "Cannot divide by zero." (exit ) | zoo (undefined) | HELD |
| d42_div_zero_point_zero | `EXE --file …/probes/batch1/d42_div_zero_point_zero.ls --omit-functions` | ERROR `DivisionByZero` — "Cannot divide a Real by zero." (exit ) | zoo (undefined) | HELD |
| m01_repro_one_over_0_95_times_0_95 | `EXE --file …/probes/batch2/m01_repro_one_over_0_95_times_0_95.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m02_swapped_0_95_times_recip | `EXE --file …/probes/batch2/m02_swapped_0_95_times_recip.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m03_bound_then_multiply | `EXE --file …/probes/batch2/m03_bound_then_multiply.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m04_exact_rat_20_19_times_19_20 | `EXE --file …/probes/batch2/m04_exact_rat_20_19_times_19_20.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m05_one_over_19_times_19 | `EXE --file …/probes/batch2/m05_one_over_19_times_19.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m06_one_over_0_85_times_0_85 | `EXE --file …/probes/batch2/m06_one_over_0_85_times_0_85.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m07_one_over_0_75_times_0_75 | `EXE --file …/probes/batch2/m07_one_over_0_75_times_0_75.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| m08_one_over_0_65_times_0_65 | `EXE --file …/probes/batch2/m08_one_over_0_65_times_0_65.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m09_one_over_0_55_times_0_55 | `EXE --file …/probes/batch2/m09_one_over_0_55_times_0_55.ls --omit-functions` | Real `0.99999999999999999999…9999999(5) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m10_one_over_0_45_times_0_45 | `EXE --file …/probes/batch2/m10_one_over_0_45_times_0_45.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m11_one_over_0_35_times_0_35 | `EXE --file …/probes/batch2/m11_one_over_0_35_times_0_35.ls --omit-functions` | Real `0.99999999999999999999…9999999985 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| m12_one_over_0_25_times_0_25 | `EXE --file …/probes/batch2/m12_one_over_0_25_times_0_25.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m13_one_over_0_15_times_0_15 | `EXE --file …/probes/batch2/m13_one_over_0_15_times_0_15.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m14_one_over_0_05_times_0_05 | `EXE --file …/probes/batch2/m14_one_over_0_05_times_0_05.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m15_one_over_0_99_times_0_99 | `EXE --file …/probes/batch2/m15_one_over_0_99_times_0_99.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m16_one_over_0_98_times_0_98 | `EXE --file …/probes/batch2/m16_one_over_0_98_times_0_98.ls --omit-functions` | Real `0.99999999999999999999…9999999908 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| m17_one_over_0_97_times_0_97 | `EXE --file …/probes/batch2/m17_one_over_0_97_times_0_97.ls --omit-functions` | Real `0.99999999999999999999…9999999921 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| m18_one_over_0_96_times_0_96 | `EXE --file …/probes/batch2/m18_one_over_0_96_times_0_96.ls --omit-functions` | Real `0.99999999999999999999…9999999936 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| m19_one_over_0_94_times_0_94 | `EXE --file …/probes/batch2/m19_one_over_0_94_times_0_94.ls --omit-functions` | Real `0.99999999999999999999…9999999(4) [1005ch]` exact:`true` | 1 | **FINDING F1** |
| m20_one_over_0_90_times_0_90 | `EXE --file …/probes/batch2/m20_one_over_0_90_times_0_90.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m21_one_over_0_8_times_0_8 | `EXE --file …/probes/batch2/m21_one_over_0_8_times_0_8.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m22_one_over_0_6_times_0_6 | `EXE --file …/probes/batch2/m22_one_over_0_6_times_0_6.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m23_one_over_0_3_times_0_3 | `EXE --file …/probes/batch2/m23_one_over_0_3_times_0_3.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m24_one_over_0_2_times_0_2 | `EXE --file …/probes/batch2/m24_one_over_0_2_times_0_2.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m25_one_over_0_1_times_0_1 | `EXE --file …/probes/batch2/m25_one_over_0_1_times_0_1.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m26_one_over_3_times_2 | `EXE --file …/probes/batch2/m26_one_over_3_times_2.ls --omit-functions` | Real `0.(6)` exact:`true` | 2/3 = 0.666666666666666667 | HELD |
| m27_two_over_3_times_3 | `EXE --file …/probes/batch2/m27_two_over_3_times_3.ls --omit-functions` | Real `2` exact:`true` | 2 | HELD |
| m28_one_over_13_times_13 | `EXE --file …/probes/batch2/m28_one_over_13_times_13.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m29_one_over_17_times_17 | `EXE --file …/probes/batch2/m29_one_over_17_times_17.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| m30_one_over_23_times_23 | `EXE --file …/probes/batch2/m30_one_over_23_times_23.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| m31_one_over_29_times_29 | `EXE --file …/probes/batch2/m31_one_over_29_times_29.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m32_one_over_31_times_31 | `EXE --file …/probes/batch2/m32_one_over_31_times_31.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m33_one_over_37_times_37 | `EXE --file …/probes/batch2/m33_one_over_37_times_37.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m34_one_over_97_times_97 | `EXE --file …/probes/batch2/m34_one_over_97_times_97.ls --omit-functions` | Real `0.99999999999999999999…9999999909 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| m35_one_over_0_9999_times_0_9999 | `EXE --file …/probes/batch2/m35_one_over_0_9999_times_0_9999.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m36_evalf_bad_product_30 | `EXE --file …/probes/batch2/m36_evalf_bad_product_30.ls --omit-functions` | Real `0.999999999999999999999999999999` exact:`false` | 1 | **FINDING F3** |
| m37_evalf_bad_product_50 | `EXE --file …/probes/batch2/m37_evalf_bad_product_50.ls --omit-functions` | Real `0.99999999999999999999999999999999999999999999999999` exact:`false` | 1 | **FINDING F3** |
| m38_bad_product_minus_one | `EXE --file …/probes/batch2/m38_bad_product_minus_one.ls --omit-functions` | Real `0` exact:`true` | 0 | **FINDING F4** |
| m39_add_thirds | `EXE --file …/probes/batch2/m39_add_thirds.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m40_add_sevenths | `EXE --file …/probes/batch2/m40_add_sevenths.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m41_one_over_3_times_3_again | `EXE --file …/probes/batch2/m41_one_over_3_times_3_again.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| m42_compare_product_to_one | `EXE --file …/probes/batch2/m42_compare_product_to_one.ls --omit-functions` | Boolean `false` | true | HELD |
| m43_one_over_0_95_plus_1_over_0_95 | `EXE --file …/probes/batch2/m43_one_over_0_95_plus_1_over_0_95.ls --omit-functions` | Real `2.(105263157894736842)` exact:`true` | 40/19 = 2.10526315789473684 | HELD |
| m44_one_over_0_95_times_0_95_div_2 | `EXE --file …/probes/batch2/m44_one_over_0_95_times_0_95_div_2.ls --omit-functions` | Real `0.5` exact:`true` | 1/2 = 0.500000000000000000 | **FINDING F4** |
| m45_neg_recip_times | `EXE --file …/probes/batch2/m45_neg_recip_times.ls --omit-functions` | Real `-0.9999999999999999999…9999999(5) [1006ch]` exact:`true` | -1 | **FINDING F1** |
| m46_evalf_one_third_38 | `EXE --file …/probes/batch2/m46_evalf_one_third_38.ls --omit-functions` | Real `0.33333333333333333333333333333333333333` exact:`false` | 1/3 to the requested digits | HELD |
| m47_evalf_one_third_50 | `EXE --file …/probes/batch2/m47_evalf_one_third_50.ls --omit-functions` | Real `0.33333333333333333333333333333333333333333333333333` exact:`false` | 1/3 to the requested digits | HELD |
| m48_evalf_one_third_51 | `EXE --file …/probes/batch2/m48_evalf_one_third_51.ls --omit-functions` | Real `0.33333333333333333333…3333333333 [53ch]` exact:`false` | 1/3 to the requested digits | HELD |
| m49_evalf_one_third_60 | `EXE --file …/probes/batch2/m49_evalf_one_third_60.ls --omit-functions` | Real `0.33333333333333333333…3333333333 [62ch]` exact:`false` | 1/3 to the requested digits | HELD |
| m50_evalf_one_third_100 | `EXE --file …/probes/batch2/m50_evalf_one_third_100.ls --omit-functions` | Real `0.33333333333333333333…3333333333 [102ch]` exact:`false` | 1/3 to the requested digits | HELD |
| m51_evalf_sqrt2_60 | `EXE --file …/probes/batch2/m51_evalf_sqrt2_60.ls --omit-functions` | Real `1.41421356237309504880…3276415727 [102ch]` exact:`false` | sqrt(2) to 100 decimals (checked) | HELD |
| m52_evalf_one_seventh_60 | `EXE --file …/probes/batch2/m52_evalf_one_seventh_60.ls --omit-functions` | Real `0.14285714285714285714…2857142857 [62ch]` exact:`false` | 1/7 = 0.142857142857142857 | HELD |
| m53_evalf_guard_9_tail | `EXE --file …/probes/batch2/m53_evalf_guard_9_tail.ls --omit-functions` | Real `0.00000000000000000000…0000000001 [53ch]` exact:`false` | 1/100000000000… = 1.00000000000000000E-51 | HELD |
| m54_evalf_2_over_3_60 | `EXE --file …/probes/batch2/m54_evalf_2_over_3_60.ls --omit-functions` | Real `0.66666666666666666666…6666666666 [62ch]` exact:`false` | 2/3 = 0.666666666666666667 | HELD |
| p01_zero_pow_neg1 | `EXE --file …/probes/batch3/p01_zero_pow_neg1.ls --omit-functions` | ERROR `InvalidArgument` — "Base cannot be zero. (Parameter 'base')" (exit 1) | zoo | HELD |
| p02_zero_pow_zero | `EXE --file …/probes/batch3/p02_zero_pow_zero.ls --omit-functions` | Natural `1` exact:`true` | 1 | HELD |
| p03_zero_pow_zero_real | `EXE --file …/probes/batch3/p03_zero_pow_zero_real.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| p04_zero_pow_neg2 | `EXE --file …/probes/batch3/p04_zero_pow_neg2.ls --omit-functions` | ERROR `InvalidArgument` — "Base cannot be zero. (Parameter 'base')" (exit 1) | zoo | HELD |
| p05_zero_pow_half | `EXE --file …/probes/batch3/p05_zero_pow_half.ls --omit-functions` | Real `0` exact:`true` | 0 | HELD |
| p06_zero_pow_real_neg1 | `EXE --file …/probes/batch3/p06_zero_pow_real_neg1.ls --omit-functions` | Real `0` exact:`true` | zoo | **FINDING F5** |
| p07_neg8_pow_third | `EXE --file …/probes/batch3/p07_neg8_pow_third.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | 2*(-1)^(1/3) = 1+1.732i | **FINDING F6** |
| p08_neg8_pow_real_third | `EXE --file …/probes/batch3/p08_neg8_pow_real_third.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | ~1+1.732i | **FINDING F6** |
| p09_neg8_pow_half | `EXE --file …/probes/batch3/p09_neg8_pow_half.ls --omit-functions` | Symbolic `i*sqrt(8)` exact:`false` | sqrt(-8) = 2.828i | HELD |
| p10_neg2_pow_3_5 | `EXE --file …/probes/batch3/p10_neg2_pow_3_5.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | (-2)^3.5 = -8*sqrt(2)*i | **FINDING F6** |
| p11_neg8_pow_neg_third | `EXE --file …/probes/batch3/p11_neg8_pow_neg_third.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | (-8)^(-1/3) = -0.5 (real root) | **FINDING F6** |
| p12_neg1_pow_half | `EXE --file …/probes/batch3/p12_neg1_pow_half.ls --omit-functions` | Symbolic `i` exact:`true` | i | HELD |
| p13_two_pow_100000 | `EXE --file …/probes/batch3/p13_two_pow_100000.ls --omit-functions` | Natural `9990020930143845079440…9883109376 [30103ch]` exact:`true` | 2^100000 (30103 digits; engine string compared EQUAL) | HELD |
| p14_two_pow_neg_100000 | `EXE --file …/probes/batch3/p14_two_pow_neg_100000.ls --omit-functions` | Real `0` exact:`true` | 1.2418e-30103 | **FINDING F7** |
| p15_ten_pow_40 | `EXE --file …/probes/batch3/p15_ten_pow_40.ls --omit-functions` | Natural `10000000000000000000000000000000000000000` exact:`true` | 10000000000000… = 1.00000000000000000E+40 | HELD |
| p16_ten_pow_neg_40 | `EXE --file …/probes/batch3/p16_ten_pow_neg_40.ls --omit-functions` | Real `0.0000000000000000000000000000000000000001` exact:`false` | 1/100000000000… = 1.00000000000000000E-40 | HELD |
| p17_two_pow_half | `EXE --file …/probes/batch3/p17_two_pow_half.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | sqrt(2) = 1.4142... | **FINDING F6** |
| p18_two_pow_half_30 | `EXE --file …/probes/batch3/p18_two_pow_half_30.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | sqrt(2) = 1.4142... | **FINDING F6** |
| p19_two_pow_half_50 | `EXE --file …/probes/batch3/p19_two_pow_half_50.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | sqrt(2) = 1.4142... | **FINDING F6** |
| p20_inf_only | `EXE --file …/probes/batch3/p20_inf_only.ls --omit-functions` | Symbolic `inf` exact:`false` | oo | HELD |
| p21_inf_minus_inf | `EXE --file …/probes/batch3/p21_inf_minus_inf.ls --omit-functions` | Symbolic `0*inf` exact:`false` | nan | INCONCLUSIVE |
| p22_zero_times_inf | `EXE --file …/probes/batch3/p22_zero_times_inf.ls --omit-functions` | Symbolic `0*inf` exact:`false` | nan | INCONCLUSIVE |
| p23_inf_plus_inf | `EXE --file …/probes/batch3/p23_inf_plus_inf.ls --omit-functions` | Symbolic `2*inf` exact:`false` | oo | INCONCLUSIVE |
| p24_inf_times_neg | `EXE --file …/probes/batch3/p24_inf_times_neg.ls --omit-functions` | Symbolic `-inf` exact:`false` | -oo | HELD |
| p25_one_over_inf | `EXE --file …/probes/batch3/p25_one_over_inf.ls --omit-functions` | Symbolic `inf^-1` exact:`false` | 0 | INCONCLUSIVE |
| p26_ten_pow_38 | `EXE --file …/probes/batch3/p26_ten_pow_38.ls --omit-functions` | Natural `100000000000000000000000000000000000000` exact:`true` | 10000000000000… = 1.00000000000000000E+38 | HELD |
| p27_ten_pow_39 | `EXE --file …/probes/batch3/p27_ten_pow_39.ls --omit-functions` | Natural `1000000000000000000000000000000000000000` exact:`true` | 10000000000000… = 1.00000000000000000E+39 | HELD |
| p28_pow_guard_9999999999 | `EXE --file …/probes/batch3/p28_pow_guard_9999999999.ls --omit-functions` | Real `0.99999999999999999999…0000000001 [78ch]` exact:`false` | (1-10^-38)^2 | HELD |
| p29_pow_neg1_nearzero | `EXE --file …/probes/batch3/p29_pow_neg1_nearzero.ls --omit-functions` | ERROR `UnsupportedOperation` — "Negative exponents are not yet supported." (exit 1) | 10000000000000… = 1.00000000000000000E+38 | **FINDING F6** |
| p30_pow_half_negative_base_frac | `EXE --file …/probes/batch3/p30_pow_half_negative_base_frac.ls --omit-functions` | Symbolic `2*i` exact:`true` | 2i | HELD |
| p31_pow_periodic_base | `EXE --file …/probes/batch3/p31_pow_periodic_base.ls --omit-functions` | Real `0.(1)` exact:`true` | 1/9 = 0.111111111111111111 | HELD |
| p32_pow_periodic_base_neg1 | `EXE --file …/probes/batch3/p32_pow_periodic_base_neg1.ls --omit-functions` | ERROR `UnsupportedOperation` — "Negative exponents are not yet supported." (exit 1) | 3 | **FINDING F6** |
| p33_pow_periodic_base_neg2 | `EXE --file …/probes/batch3/p33_pow_periodic_base_neg2.ls --omit-functions` | ERROR `UnsupportedOperation` — "Negative exponents are not yet supported." (exit 1) | 9 | **FINDING F6** |
| p34_cube_root_negative_real | `EXE --file …/probes/batch3/p34_cube_root_negative_real.ls --omit-functions` | ERROR `UnsupportedOperation` — "Non-integer exponents are not yet supported." (exit 1) | real_root(-27,3) = -3 | **FINDING F6** |
| p35_pow_two_precisions_third | `EXE --file …/probes/batch3/p35_pow_two_precisions_third.ls --omit-functions` | Real `-0.0000000000000000000011111111111111111111` exact:`false` | -1111111111111… = -1.11111111111111111E-21 | HELD |
| p36_exp_zero | `EXE --file …/probes/batch3/p36_exp_zero.ls --omit-functions` | Symbolic `1` exact:`true` | 1 | HELD |
| p37_ln_zero | `EXE --file …/probes/batch3/p37_ln_zero.ls --omit-functions` | ERROR `InvalidOperation` — "Unknown function 'ln'." (exit 1) | -oo (ln is not a builtin here) | HELD |
| p38_ln_neg | `EXE --file …/probes/batch3/p38_ln_neg.ls --omit-functions` | ERROR `InvalidOperation` — "Unknown function 'ln'." (exit 1) | i*pi (ln is not a builtin here) | HELD |
| p39_sqrt_zero | `EXE --file …/probes/batch3/p39_sqrt_zero.ls --omit-functions` | Real `0` exact:`true` | 0 | HELD |
| p40_sqrt_neg_zero | `EXE --file …/probes/batch3/p40_sqrt_neg_zero.ls --omit-functions` | Real `0` exact:`true` | 0 | HELD |
| n01_two_pow_neg1 | `EXE --file …/probes/batch4/n01_two_pow_neg1.ls --omit-functions` | Real `0.5` exact:`true` | 1/2 = 0.500000000000000000 | HELD |
| n02_two_pow_neg2 | `EXE --file …/probes/batch4/n02_two_pow_neg2.ls --omit-functions` | Real `0.25` exact:`true` | 1/4 = 0.250000000000000000 | HELD |
| n03_two_pow_neg10 | `EXE --file …/probes/batch4/n03_two_pow_neg10.ls --omit-functions` | Real `0.0009765625` exact:`true` | 1/1024 = 0.000976562500000000000 | HELD |
| n04_two_pow_neg100 | `EXE --file …/probes/batch4/n04_two_pow_neg100.ls --omit-functions` | Real `0.00000000000000000000…9306640625 [102ch]` exact:`false` | 2^-100 (100 decimals) | HELD |
| n05_two_pow_neg500 | `EXE --file …/probes/batch4/n05_two_pow_neg500.ls --omit-functions` | Real `0.00000000000000000000…8681640625 [502ch]` exact:`false` | 2^-500 | HELD |
| n06_two_pow_neg1000 | `EXE --file …/probes/batch4/n06_two_pow_neg1000.ls --omit-functions` | Real `0.00000000000000000000…1650390625 [1002ch]` exact:`false` | 2^-1000 (1000 decimals) | HELD |
| n07_two_pow_neg1001 | `EXE --file …/probes/batch4/n07_two_pow_neg1001.ls --omit-functions` | Real `0.00000000000000000000…0825195312 [1002ch]` exact:`false` | 2^-1001 (needs 1001 decimals) | HELD |
| n08_two_pow_neg5000 | `EXE --file …/probes/batch4/n08_two_pow_neg5000.ls --omit-functions` | Real `0` exact:`true` | 7.0798e-1506 | **FINDING F7** |
| n09_two_pow_neg30103 | `EXE --file …/probes/batch4/n09_two_pow_neg30103.ls --omit-functions` | Real `0` exact:`true` | 1.2418e-9062 | **FINDING F7** |
| n10_one_over_two_pow_1000 | `EXE --file …/probes/batch4/n10_one_over_two_pow_1000.ls --omit-functions` | Real `0.00000000000000000000…1650390625 [1002ch]` exact:`false` | 2^-1000 | HELD |
| n11_one_over_two_pow_1001 | `EXE --file …/probes/batch4/n11_one_over_two_pow_1001.ls --omit-functions` | Real `0.00000000000000000000…0825195312 [1002ch]` exact:`false` | 2^-1001 | HELD |
| n12_ten_pow_neg1001 | `EXE --file …/probes/batch4/n12_ten_pow_neg1001.ls --omit-functions` | Real `0` exact:`true` | 1.0e-1001 | **FINDING F7** |
| n13_ten_pow_neg1000 | `EXE --file …/probes/batch4/n13_ten_pow_neg1000.ls --omit-functions` | Real `0.00000000000000000000…0000000001 [1002ch]` exact:`false` | 1.0e-1000 | HELD |
| n14_ten_pow_neg999 | `EXE --file …/probes/batch4/n14_ten_pow_neg999.ls --omit-functions` | Real `0.00000000000000000000…0000000001 [1001ch]` exact:`false` | 1.0e-999 | HELD |
| n15_evalf_two_pow_neg100 | `EXE --file …/probes/batch4/n15_evalf_two_pow_neg100.ls --omit-functions` | Real `0.00000000000000000000…9306640625 [102ch]` exact:`false` | 2^-100 | HELD |
| n16_evalf_two_pow_neg30103 | `EXE --file …/probes/batch4/n16_evalf_two_pow_neg30103.ls --omit-functions` | Real `0` exact:`true` | 1.2418e-9062 | **FINDING F7** |
| g01_one_over_37nines | `EXE --file …/probes/batch4/g01_one_over_37nines.ls --omit-functions` | Real `1.(0000000000000000000000000000000000001)` exact:`true` | 10^37/(10^37-1) | HELD |
| g02_parse_one_with_38zeros | `EXE --file …/probes/batch4/g02_parse_one_with_38zeros.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| g03_parse_38nines | `EXE --file …/probes/batch4/g03_parse_38nines.ls --omit-functions` | Real `0.99999999999999999999999999999999999999` exact:`false` | (10^38-1)/10^38 | HELD |
| g04_parse_38nines_then_zero | `EXE --file …/probes/batch4/g04_parse_38nines_then_zero.ls --omit-functions` | Real `0.99999999999999999999999999999999999999` exact:`false` | (10^38-1)/10^38 | HELD |
| g05_parse_39sig_ends9 | `EXE --file …/probes/batch4/g05_parse_39sig_ends9.ls --omit-functions` | Real `12345678901234567890123456789012345678.9` exact:`true` | 1234...678.9 | HELD |
| g06_parse_39sig_ends0 | `EXE --file …/probes/batch4/g06_parse_39sig_ends0.ls --omit-functions` | Real `12345678901234567890123456789012345678` exact:`true` | 1234...678 | HELD |
| g07_evalf_third_37 | `EXE --file …/probes/batch4/g07_evalf_third_37.ls --omit-functions` | Real `0.3333333333333333333333333333333333333` exact:`false` | 1/3 | HELD |
| g08_evalf_third_39 | `EXE --file …/probes/batch4/g08_evalf_third_39.ls --omit-functions` | Real `0.333333333333333333333333333333333333333` exact:`false` | 1/3 | HELD |
| g09_evalf_sixth_40 | `EXE --file …/probes/batch4/g09_evalf_sixth_40.ls --omit-functions` | Real `0.1666666666666666666666666666666666666666` exact:`false` | 1/6 | HELD |
| g10_evalf_ninth_40 | `EXE --file …/probes/batch4/g10_evalf_ninth_40.ls --omit-functions` | Real `0.1111111111111111111111111111111111111111` exact:`false` | 1/9 | HELD |
| g11_evalf_1998_40 | `EXE --file …/probes/batch4/g11_evalf_1998_40.ls --omit-functions` | Real `0.00000100200300400500…3303313323 [1002ch]` exact:`false` | 1/998001 (first 1001 decimals checked) | HELD |
| k01_four_thirds_times_three_quarters | `EXE --file …/probes/batch4/k01_four_thirds_times_three_quarters.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| k02_one_over_six_times_six | `EXE --file …/probes/batch4/k02_one_over_six_times_six.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| k03_one_over_nine_times_nine | `EXE --file …/probes/batch4/k03_one_over_nine_times_nine.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| k04_five_sixths_times_six | `EXE --file …/probes/batch4/k04_five_sixths_times_six.ls --omit-functions` | Real `5` exact:`true` | 5 | HELD |
| k05_two_over_17_times_17 | `EXE --file …/probes/batch4/k05_two_over_17_times_17.ls --omit-functions` | Real `1.99999999999999999999…9999999999 [102ch]` exact:`false` | 2 | **FINDING F2** |
| k06_three_over_seven_times_seven | `EXE --file …/probes/batch4/k06_three_over_seven_times_seven.ls --omit-functions` | Real `3` exact:`true` | 3 | HELD |
| k07_seventeen_times_one_over_17_var | `EXE --file …/probes/batch4/k07_seventeen_times_one_over_17_var.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| k08_bad_product_minus_one_75 | `EXE --file …/probes/batch4/k08_bad_product_minus_one_75.ls --omit-functions` | Real `-0.0000000000000000000…0000000025 [1005ch]` exact:`false` | 0 | HELD |
| k09_bad_product_eq_one_75 | `EXE --file …/probes/batch4/k09_bad_product_eq_one_75.ls --omit-functions` | Boolean `false` | false | HELD |
| k10_bad_product_scaled | `EXE --file …/probes/batch4/k10_bad_product_scaled.ls --omit-functions` | Real `999999.999999999999999…9999999999 [107ch]` exact:`false` | 1000000 | **FINDING F2** |
| k11_bad_17_product_plus_zero | `EXE --file …/probes/batch4/k11_bad_17_product_plus_zero.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| k12_bad_17_product_times_17 | `EXE --file …/probes/batch4/k12_bad_17_product_times_17.ls --omit-functions` | Real `16.9999999999999999999…9999999999 [103ch]` exact:`false` | 17 | **FINDING F2** |
| k13_evalf_bad_17_product_30 | `EXE --file …/probes/batch4/k13_evalf_bad_17_product_30.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| k14_evalf_bad_17_product_40 | `EXE --file …/probes/batch4/k14_evalf_bad_17_product_40.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| k15_one_over_0_75_only | `EXE --file …/probes/batch4/k15_one_over_0_75_only.ls --omit-functions` | Real `1.(3)` exact:`true` | 4/3 = 1.33333333333333333 | HELD |
| k16_one_over_0_75_plus_itself | `EXE --file …/probes/batch4/k16_one_over_0_75_plus_itself.ls --omit-functions` | Real `2.(6)` exact:`true` | 8/3 = 2.66666666666666667 | HELD |
| k17_quarter_recips | `EXE --file …/probes/batch4/k17_quarter_recips.ls --omit-functions` | Real `16` exact:`true` | 16 | HELD |
| k18_bad_product_div_self | `EXE --file …/probes/batch4/k18_bad_product_div_self.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| k19_bad_product_sqrt | `EXE --file …/probes/batch4/k19_bad_product_sqrt.ls --omit-functions` | Real `0.99999999999999999999…9999999999 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| k20_bad_product_squared | `EXE --file …/probes/batch4/k20_bad_product_squared.ls --omit-functions` | Real `0.99999999999999999999…0000000625 [2006ch]` exact:`false` | 1 | **FINDING F2** |
| i01_one_over_0_7_times_0_7 | `EXE --file …/probes/batch5/i01_one_over_0_7_times_0_7.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i02_one_over_0_71_times_0_71 | `EXE --file …/probes/batch5/i02_one_over_0_71_times_0_71.ls --omit-functions` | Real `0.99999999999999999999…9999999957 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i03_one_over_0_91_times_0_91 | `EXE --file …/probes/batch5/i03_one_over_0_91_times_0_91.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i04_one_over_0_87_times_0_87 | `EXE --file …/probes/batch5/i04_one_over_0_87_times_0_87.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i05_one_over_0_63_times_0_63 | `EXE --file …/probes/batch5/i05_one_over_0_63_times_0_63.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i06_one_over_0_49_times_0_49 | `EXE --file …/probes/batch5/i06_one_over_0_49_times_0_49.ls --omit-functions` | Real `0.99999999999999999999…9999999957 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i07_one_over_0_21_times_0_21 | `EXE --file …/probes/batch5/i07_one_over_0_21_times_0_21.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i08_one_over_0_13_times_0_13 | `EXE --file …/probes/batch5/i08_one_over_0_13_times_0_13.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i09_one_over_0_11_times_0_11 | `EXE --file …/probes/batch5/i09_one_over_0_11_times_0_11.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i10_one_over_0_09_times_0_09 | `EXE --file …/probes/batch5/i10_one_over_0_09_times_0_09.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i11_one_over_0_07_times_0_07 | `EXE --file …/probes/batch5/i11_one_over_0_07_times_0_07.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i12_one_over_0_03_times_0_03 | `EXE --file …/probes/batch5/i12_one_over_0_03_times_0_03.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i13_one_over_0_01_times_0_01 | `EXE --file …/probes/batch5/i13_one_over_0_01_times_0_01.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i14_one_over_0_999_times_0_999 | `EXE --file …/probes/batch5/i14_one_over_0_999_times_0_999.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i15_one_over_0_99999_times | `EXE --file …/probes/batch5/i15_one_over_0_99999_times.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i16_swapped_0_75 | `EXE --file …/probes/batch5/i16_swapped_0_75.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 3/4 = 0.750000000000000000 | **FINDING F2** |
| i17_seven_over_0_75_times | `EXE --file …/probes/batch5/i17_seven_over_0_75_times.ls --omit-functions` | Real `6.99999999999999999999…9999999999 [102ch]` exact:`false` | 7 | **FINDING F2** |
| i18_seven_over_0_95_times | `EXE --file …/probes/batch5/i18_seven_over_0_95_times.ls --omit-functions` | Real `7` exact:`true` | 7 | HELD |
| i19_123_over_0_97_times | `EXE --file …/probes/batch5/i19_123_over_0_97_times.ls --omit-functions` | Real `122.999999999999999999…9999999999 [104ch]` exact:`false` | 123 | **FINDING F2** |
| i20_alt_bracket_0_75 | `EXE --file …/probes/batch5/i20_alt_bracket_0_75.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i21_evalf_four_thirds_product_20 | `EXE --file …/probes/batch5/i21_evalf_four_thirds_product_20.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i22_evalf_four_thirds_product_40 | `EXE --file …/probes/batch5/i22_evalf_four_thirds_product_40.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i23_evalf_four_thirds_product_19 | `EXE --file …/probes/batch5/i23_evalf_four_thirds_product_19.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i24_four_thirds_product_eq_one | `EXE --file …/probes/batch5/i24_four_thirds_product_eq_one.ls --omit-functions` | Boolean `false` | false | HELD |
| i25_four_thirds_product_minus_one | `EXE --file …/probes/batch5/i25_four_thirds_product_minus_one.ls --omit-functions` | Real `-0.0000000000000000000…0000000025 [1005ch]` exact:`false` | 0 | **FINDING F2** |
| i26_one_over_0_750_times | `EXE --file …/probes/batch5/i26_one_over_0_750_times.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i27_one_over_0_75_times_three_quarters | `EXE --file …/probes/batch5/i27_one_over_0_75_times_three_quarters.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i28_four_thirds_times_0_75 | `EXE --file …/probes/batch5/i28_four_thirds_times_0_75.ls --omit-functions` | Real `0.99999999999999999999…9999999975 [1004ch]` exact:`false` | 1 | **FINDING F2** |
| i29_long_literal_times_0_75 | `EXE --file …/probes/batch5/i29_long_literal_times_0_75.ls --omit-functions` | Real `0.999999999999999999999999999999999999975` exact:`false` | 39999999999999… = 1.00000000000000000 | HELD |
| i30_one_over_6_times_3 | `EXE --file …/probes/batch5/i30_one_over_6_times_3.ls --omit-functions` | Real `0.5` exact:`true` | 1/2 = 0.500000000000000000 | HELD |
| i31_one_over_12_times_12 | `EXE --file …/probes/batch5/i31_one_over_12_times_12.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i32_one_over_14_times_14 | `EXE --file …/probes/batch5/i32_one_over_14_times_14.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i33_one_over_18_times_18 | `EXE --file …/probes/batch5/i33_one_over_18_times_18.ls --omit-functions` | Real `1` exact:`true` | 1 | HELD |
| i34_one_over_22_times_22 | `EXE --file …/probes/batch5/i34_one_over_22_times_22.ls --omit-functions` | Real `0.99999999999999999999…9999999(8) [1003ch]` exact:`true` | 1 | **FINDING F1** |
| i35_one_over_26_times_26 | `EXE --file …/probes/batch5/i35_one_over_26_times_26.ls --omit-functions` | Real `0.99999999999999999999…9999999984 [1002ch]` exact:`false` | 1 | **FINDING F2** |
| i36_one_over_0_7_times_0_7_eq | `EXE --file …/probes/batch5/i36_one_over_0_7_times_0_7_eq.ls --omit-functions` | Boolean `true` | true | HELD |
| i37_one_over_0_75_times_1_5 | `EXE --file …/probes/batch5/i37_one_over_0_75_times_1_5.ls --omit-functions` | Real `2` exact:`true` | 2 | HELD |
| i38_zero_times_bad | `EXE --file …/probes/batch5/i38_zero_times_bad.ls --omit-functions` | Real `0` exact:`true` | 0 | HELD |
| i39_one_over_0_75_minus_three_quarters | `EXE --file …/probes/batch5/i39_one_over_0_75_minus_three_quarters.ls --omit-functions` | Real `0.0000000000000000000000000000000000000(3)` exact:`true` | 1/300000000000… = 3.33333333333333333E-38 | HELD |


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
