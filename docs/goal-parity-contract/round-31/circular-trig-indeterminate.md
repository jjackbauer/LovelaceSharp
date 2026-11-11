# Round 31 — circular/inverse families, indeterminate forms, real trigonometry

Scope: `rust/lovelace-lang/src`, `rust/lovelace-symbolics/src`, and — for the real-trig rows
only — `rust/lovelace-real/src` (see section 3.1 for why that crate had to be touched: the 12
rows assert `Lovelace.Real.Real.Sin/Cos` directly, and the Rust implementation of that API is in
`lovelace-real`). No `.cs` file, fixture, corpus file or `parity/` artifact was changed; this
report is the only non-Rust file written by this round.

Oracle for every probe: the pinned runner
```
dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script> --omit-functions --omit-variables
```
(`--file` is the same evaluation path as the task's `--eval`; it avoids PowerShell quoting of the
scripts). The Rust side is `rust/target/release/lovelace-run.exe` with the same arguments. The
comparison is byte-for-byte on the JSON envelope after dropping only the declared volatile fields
(`revision`, `elapsed`, `elapsedTime`, `timings`, `variables`, `functions`) and canonicalising
key order. For the real-trig rows — which are not wire-reachable, because the language leaves
`sin(10^20)` symbolic — the C# side was driven through its own kernel API with a scratch harness
compiled against the pinned `Lovelace.Real/bin/Release/net10.0` assembly (section 4).

Baseline at the start of this round: workspace suite 4,355 tests green, corpus 212/214
(sym/071, sym/076), fixtures 20/20, CLI 16/16, differential 200 cases / 157 match / 43 mismatch.
The working tree also carries the concurrent digit-store work (nat tests 226, a new
`rust/lovelace-nat/src/digit_store.rs`, and the regenerated `parity/` inventory); those changes are
not this round's and are not claimed here.

## 1. Family 1 — circular / inverse families (18 rows)

Cause (four distinct holes, one family):

1. `poly_coeffs` treated a function of the solved variable as a power of it: `degree_in` counts
   `exp(x)` as one degree and `divide_out` stripped it, so `x*exp(x) - c` looked like
   `x^2 - c` and the solver published `±sqrt(c)` as a Solved/Complete answer
   (`x*exp(x) == 1` was the reported repro).
2. The solver had no inverse-branch machinery: `exp(x) = 5` had a family only because an old
   hard-coded `parametric_family` recognised two literal shapes, and its imaginary unit was a
   user symbol `i` (canonical `(sym i)`, `free_symbols [i, k]`) instead of the named constant
   (canonical `(i)`, `free_symbols [k]`). `exp(x) = c`, the logarithmic inverse, all
   sin/cos/tan families and the real/complex split were absent.
3. There was no post-condition on published values: a candidate that still contains the solved
   variable (a circular answer) could enter the set, and the linear-coefficient path did not
   refuse a coefficient that mentions x.
4. The reference's branch verification (every inverse candidate must satisfy the ORIGINAL
   equation) was missing, so `sqrt(x) = -2`-shaped equations could publish an extraneous root.

Fix, ported from the reference (`Solvers.SolveElementary / SolveInverse / SolvePeriodic /
SolveExponentialFamily / EnforceSolutionValues`):

- `poly_coeffs` (lovelace-symbolics) refuses a term whose coefficient still mentions the variable
  (only the variable itself and an integer power of it are removable), and `solve_linear_symbolic`
  refuses variable-carrying coefficients — the reference's `TryCoefficients` guard.
- `solve_elementary` / `solve_inverse` (lovelace-lang, new section "Inverse-branch solving"):
  bare `h(u) = 0` and `h(u) ± c = 0` with `h` in {exp, log, sqrt, power, sin, cos, tan}; every
  candidate is verified numerically against the original `h - c` at 40 digits with the reference's
  1e-24 tolerance; a decidable non-zero residual rejects it and an undecidable one keeps it.
- `solve_periodic`: sin/cos/tan families with the reference's |c|-against-1 classification
  (two branches below 1, one at 1, `k*pi` / `pi/2 + k*pi` at 0, NoSolutions over the reals and
  Unevaluated over the complex field above 1), built in the reference's published term order.
- `solve_exponential_family`: `log(c) + 2*k*pi*i`, period `2*pi*i`, condition `c != 0` when c is
  not numerically decidable, with the principal member spot-checked at k = 0.
- `constant_real` (the reference's `ConstantToNum`): exp/real split and the |c| classification.
- The named imaginary unit and the NamedConstant ordinal (`Pi, E, I, Infinity`) in the canonical
  order, so `2*k*pi*i` is `(mul (rat 2 1) (sym k) (pi) (i))` exactly as the oracle prints it.
- `enforce_solution_values`: a published value or family template that mentions the variable is
  dropped; if nothing survives the record is Unevaluated with the reference's circular-answer
  message (and Partial when something survives).
- `numeric::evaluate` gained the three named constants (`pi`, `e`, `i`) so a complex candidate
  is numerically checkable (`sqrt(x) + 2 = 0` must be REJECTED, not kept as undecidable).

Probes (30 scripts; the table is representative, all 30 are byte-equal after the fix):

| Script | Oracle | Before | After |
|---|---|---|---|
| `solve_full(exp(x) == c, x)` | family `log(c) + 2*k*pi*i`, conditions `[c != 0]` | Unevaluated "No solver for this structure." | identical |
| `solve_full(exp(x) == 5, x)` | family `log(5) + 2*k*pi*i` | same display, canonical `(sym i)`, free symbols [i,k] | identical |
| `solve_full(exp(x) == c, x, real)` | Solved `log(c)`, conditions `[1 != 0]` | Unevaluated | identical |
| `solve_full(exp(x) == 5, x, real)` | Solved `log(5)` | complex family (real filter ignored) | identical |
| `solve_full(exp(x) == -5, x, real)` | NoSolutions "exp(u) = c has no real solution for c < 0." | Solved family `log(-5) + ...` | identical |
| `solve_full(exp(x) == 0, x, real)` | NoSolutions "exp(u) = 0 has no solution." | Unevaluated | identical |
| `solve_full(sin(x) == 1/2, x)` | `asin(1/2) + 2*k*pi`, `pi - asin(1/2) + 2*k*pi` | Unevaluated | identical |
| `solve_full(cos(x) == 1/2, x)` | `acos(1/2) + 2*k*pi`, `-acos(1/2) + 2*k*pi` | Unevaluated | identical |
| `solve_full(tan(x) == 1, x)` | `atan(1) + k*pi` | Unevaluated | identical |
| `solve_full(tan(x) == 0, x)` | `k*pi` (atan(0) folds to 0) | Unevaluated | identical |
| `solve_full(log(x) == c, x)` | Solved `exp(c)`, conditions `[1 != 0]` | Unevaluated | identical |
| `solve_full(log(x) == 2, x)` | Solved `exp(2)` | Unevaluated | identical |
| `solve_full(x*exp(x) == 1, x)` | Unevaluated (no solver for this structure) | **Solved, complete, ±1** | identical |
| `solve_full(exp(x) == x, x)` | Unevaluated, circular-answer message | Unevaluated, generic message | identical |
| `solve_full(cos(x) == x, x)` | Unevaluated "cos(x) = c with symbolic c is not supported in v1." | generic message | identical |
| `solve_full(sqrt(x) == x, x)` | Solved, `0` and `1` | Unevaluated | identical |
| `solve_full(sqrt(x) + 2 == 0, x)` | Unevaluated "(1 rejected)" | generic message | identical |
| `solve_full(x^2-4 == 0, x)`, `x^2+1 == 0, real`, `x^4-x^2-1 == 0` | Solved 2 / NoSolutions / Partial 2+2 | identical | unchanged (controls) |

The other 12 probes in the set cover `log(x)==x`, `x-log(x)`, `x-sin(x)`, `x-cos(x)`,
`x+exp(x)` (complex and real), `sin(x)==2` (both domains), `cos(x)==0`, `sin(x)==±1`,
`tan(x)==0, real`, `exp(x)==1`, `exp(2*x)==5`, `solve(...)` (non-full) and
`exp(x)==x, real`; all byte-equal.

Tests: `rust/lovelace-lang/src/circular_family_round31.rs` — 10 functions, the C# assertions
transcribed one for one (including the kernel-level check that every family member solves
`exp(x) = 3` at k = -3..4 and that k = 0 and k = 1 are distinct).

Remaining divergence in this area (recorded, not hidden): the additive DISPLAY order of sums that
mix a named constant with a number (or an infinity with a number) is still the pre-existing Rust
order — see section 2 — but no circular row asserts it.

## 2. Family 2 — indeterminate forms (11 rows)

Cause: the arithmetic identities `a - a = 0` and `0*a = 0` were applied unconditionally, so
`inf - inf` and `0*inf` folded to the definite value `0`. The reference's constructors consult
`HasUndefinedRisk` (infinity, an explicit quotient, or a power with a negative/unknown exponent)
before dropping a zero coefficient or a zero factor.

Fix (lovelace-symbolics `Expr::add` / `Expr::mul`): the zero-coefficient / zero-factor fold now
keeps the canonical indeterminate product when the remainder carries undefined risk, exactly as
the reference does. `0/x` therefore also stops folding to 0 (the reference prints `0/x`), which
is outside the 11 rows but is the same rule and was verified against the oracle.

| Script | Oracle | Before | After |
|---|---|---|---|
| `inf - inf` | `0*inf` | `0` | `0*inf` |
| `0*inf` | `0*inf` | `0` | `0*inf` |
| `(inf - inf) + 1` | `1 + 0*inf` | `1` | `0*inf + 1` (value right; additive order differs) |
| `inf/inf` | `inf/inf` | `inf/inf` | unchanged |
| `0/0` | `0/0` | `0/0` | unchanged |
| `inf + 1` | `1 + inf` | `inf + 1` | unchanged (pre-existing additive order) |
| `(inf + x) - (inf + x)` | `x + inf - (x + inf)` | `x - (x + inf) + inf` | unchanged (term placement differs; not a zero) |
| `x - x`, `0*x`, `pi - pi`, `0*pi`, `e - e` | `0` | `0` | `0` (controls) |
| `inf^0` | `1` | `1` | `1` (documented decision) |
| `0/x` (same rule) | `0/x` | `0` | `0/x` |

The additive display order (`1 + 0*inf` vs `0*inf + 1`, `1 + inf` vs `inf + 1`) is a
pre-existing term-ordering difference in the shared printer, not a value difference; none of the 11
C# assertions reads it (they assert not-definite-zero, infinity survival, and the exact string
`0*inf` for two rows, which all pass).

Tests: `rust/lovelace-symbolics/src/indeterminate_forms_round31.rs` — 11 functions, one per C#
fact, including the positive controls (symbols, Pi, E) and the `inf^0 = 1` decision.

## 3. Family 3 — real trigonometry (12 rows)

Cause: the Rust `Real::sin/cos` was a port of `ComplexMath.SinCosAtPrecision`, whose range
reduction narrows the turn count through an `i64` (`trunc_to_long`). For `10^20` / `10^100` the
quotient overflows, falls back to 0, and the series is evaluated at the unreduced argument, so the
result is wrong (recorded in the manifest: `sin(10^100)` at 30 places = 0.945509..., oracle
-0.372376...). There was also no special-angle table, so `sin(pi/2)` was 0.999... and
`cos(pi/6)` was one unit in the last place off.

Fix (lovelace-real, `transcendental.rs`): port of the reference's `Lovelace.Real/Real.cs`
kernel — the special-angle table is asked FIRST against the caller's own pi (16 exact sin/cos
pairs, the terminating/truncating candidate arithmetic of `SpecialAngleCandidate`), then a general
angle runs with the argument-aware width (`TrigIntegerDigits` + `TrigReductionPlaces` +
`TrigPiPlaces`), a pi fetched at that width, and a reduction whose quotient is an INTEGER division
(`WholeQuotient`) over arbitrary-precision integers, so no turn count can overflow. The series runs
at `digits + 10` guard places; the cosine uses the `sin(pi/2 - x)` shape within the guard of
pi/2. The previous `ComplexMath` port remains as `sin_cos_raw` / `complex_math_sin` /
`complex_math_cos`, and the numeric evaluator, complex trig and `Real::tan` now name that path
explicitly, so the wire's `evalf` values are unchanged (verified: `evalf(sin(1),30)`,
`evalf(cos(1),30)`, `evalf(tan(1),30)`, `evalf(sin(10),30)`, `evalf(sin(0.5),30)`,
`evalf(cos(2),50)`, `evalf(sin(100),30)` are byte-equal to the oracle).

Ground truth: the 30/50/100-place digits were measured from the pinned C# assembly itself
(a scratch harness calling `Real.Sin(new Real("10^20"), 30)` etc. under
`Real.WithPrecision(digits, digits)`, the exact calls the C# test makes); the C# test's own
mpmath goldens agree with them where the repository records them
(`sin(10^100)` 30 places = `372376123661276688262086695553`, the certified-prefix golden).

| Probe | Oracle (C# Real.Sin, measured) | Before | After |
|---|---|---|---|
| `sin(10^20)` at 30 / 50 / 100 | `645251285265780844205811711312` / `...68689` / 100 digits | overflowed turn count: outside [-1,1] | identical digits |
| `sin(10^100)` at 30 / 50 / 100 | `372376123661276688262086695553` / `...356743` / 100 digits | 0.945509... at 30 (recorded) | identical digits |
| `cos(10^100)` at 30 / 50 / 100 | `928081905074655343456194643776` / `...643905` / 100 digits | same overflow | identical digits |
| `sin(pi)`, `cos(pi)`, `sin(pi/2)`, `sin(pi/6)`, `cos(pi/6)` at 20/50/100 | exact `0`, `-1`, `1`, `1/2`, `sqrt(3)/2`, with the table value's exactness | 0.999..., one ulp off, no table | exact equality and exactness |

Tests: `rust/lovelace-real/src/round31_trig.rs` — 3 functions: the nine precision-oracle rows
(three arguments x three requested widths, against the measured digits), the three
pi-resolution rows (16 angles x 20/50/100 places, exact equality and exactness flag), and the
negative-angle symmetry control.

### 3.1 Scope note

The 12 rows assert `Lovelace.Real.Real.Sin/Cos`, so the fix has to live where that API lives
(`rust/lovelace-real/src`). The alternative — asserting the same facts through the language — is
not possible: the language leaves `sin(10^20)` symbolic and `evalf(sin(10^20))` makes the C#
ComplexMath path itself throw an Int64 overflow. This crate was outside the stated scope; the
parent should treat this as the one scope extension of the round, with the wire behaviour held
constant by the explicit two-kernel split and the seven `evalf` probes above.

## 4. Regressions found and resolved during the round (recorded)

- Four tests in `rust/lovelace-lang/src/round26_complex_math_parity.rs` pinned the `ComplexMath`
  kernel through `Real::sin/cos`. Once `Real::sin/cos` became the `Real.Sin/Cos` port, those call
  sites were silently testing the other kernel. The module now names the kernel under test through
  a small `ComplexMathTrig` extension trait (`cm_sin`/`cm_cos` over `sin_cos_raw`), so the four
  assertions (inexact zero reduction, inexact pi, inexact zero, `tan(pi/2)*cos(pi/2) = 1`) are
  unchanged in strength and the module cannot drift again. The C# assertions themselves are
  unchanged.
- The full-suite run after the production edits found exactly those four failures and no others
  (recorded, not hidden).
- One corpus run reported `lang/076` (plot file write) as a failure; it was an artefact of the
  ad-hoc invocation's missing TMP/TEMP, and the same binary reports 212/214 with the standard
  environment (below).

## 5. Gates

```
$ cd rust; cargo test --workspace --no-fail-fast --offline
array 58, desktop 67, int 125, json 7, lang 1,150, nat 226, real 2,419, run 346,
symbolics 72 (+1 integration) = 4,471 tests collected, 0 failed, every target ok, exit 0.
(this round's own additions: 11 indeterminate + 10 circular + 3 trig = 24 test functions;
 the nat 226 and the regenerated parity/ inventory are the concurrent digit-store work,
 not this round's)
```

```
$ rust/target/release/lovelace-corpus.exe        (from the repo root, standard TMP/TEMP)
FAIL sym/071    (pre-existing)
FAIL sym/076    (pre-existing; linsolve simplifies x/x to 1 before printing)
corpus: 212 passed, 2 failed, 214 total
```

```
$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-cli.mjs
CLI matrix: 16/16
PASS: every RUN-001..RUN-014 row behaves like the oracle
```

```
$ node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
generated cases (--cases 200): 200
generated corpus by category: GENERATED 200  157 match  43 mismatch  0 error
regressions: discovered=1845 replayed=1752 stillMismatching=1046 resolved=706 new=0
harness exit: 1 (standing mismatches)
```
Identical to the baseline (157/43 generated, 706/1,046 replayed, new=0): no differential delta.

Before/after oracle parity for the changed surfaces: circular 19 diffs -> 0 diffs over the 30
probes; indeterminate 7 diffs -> 4: three are additive DISPLAY ORDER only (values and structures equal;
none is asserted by the 11 rows) and one is a pre-existing error-position difference in a
malformed control script (undefined x); real trig 12 rows -> exact digits and exact
table equality; wire `evalf` trig 0 diffs over 7 probes both before and after.

## 6. Census ids expected to unblock (41 rows)

All ids are the deferred manifest ids whose assertion the named Rust test now reproduces. None is a
weakened assertion.

| Cluster | Rows | Manifest ids | Rust test |
|---|---:|---|---|
| Circular / inverse families | 18 | `Lovelace.Symbolics.Tests.InfiniteFamilyCompletenessTests`: `CircularCandidates_AreNotRepresentedAndAreNotACompleteSet#0..#9` (10), `Kernel_ExpSymbolic_Complex_EveryFamilyMemberSolves#0`, `RealDomain_ExpKeepsTheSingleSolution#0`, `RealDomain_ExpSymbolic_KeepsItsBehaviour_AndNeverCarriesTheComplexPeriod#0`, `SolveFull_ExpNumeric_Complex_IsNotThePrincipalValueAlone#0`, `SolveFull_ExpSymbolic_Complex_RepresentsTheWholeFamily#0`, `SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0`, `TheLegitimateInverseBranch_StillRepresentsItsValues#0`, `TheOtherInverses_KeepTheirOwnHonestAnswer#0` | `circular_family_round31` (10 functions) |
| Indeterminate forms | 11 | `Lovelace.Symbolics.Tests.IndeterminateFormTests`: `FiniteNamedConstantIdentities_StillFold#0`, `IndeterminateDifferenceEmbeddedInSum_DoesNotResurrectZero#0`, `InfinityMinusInfinity_IsNotDefiniteZero#0`, `InfinityPlusOne_StaysUnevaluated#0`, `NestedInfinityDifference_IsNotDefiniteZero#0`, `Sibling_InfinityOverInfinity_StaysUnevaluated#0`, `Sibling_InfinityToTheZero#0`, `Sibling_ZeroOverZero_IsNotDefiniteZero#0`, `SymbolMinusItself_StillFoldsToZero#0`, `ZeroTimesInfinity_IsNotDefiniteZero#0`, `ZeroTimesSymbol_StillFoldsToZero#0` | `indeterminate_forms_round31` (11 functions) |
| Real trig, large magnitudes | 9 | `Lovelace.Real.Tests.RealTrigPrecisionOracleTests.TheRequestedFractionalDigitsAreCorrect` `#5`, `#6`, `#7`, `#13`, `#14`, `#15`, `#21`, `#22`, `#23` | `round31_trig::requested_fractional_digits_are_correct_at_every_huge_magnitude` |
| Real trig, ambient-pi table | 3 | `Lovelace.Real.Tests.RealTrigPiResolutionPropertyTests.SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues` `#0`, `#1`, `#2` | `round31_trig::exact_multiples_of_the_ambient_pi_keep_their_exact_table_values` |
| **total** | **41** | | |

The SymPy oracle row is mapped to the shape it measured, not to a live SymPy call (the workspace
has no SymPy bridge); the two algebraic facts it proves — every family member solves, and k = 0 and
k = 1 are distinct — are asserted by `kernel_exp_symbolic_complex_every_family_member_solves`.
The parent should decide whether that is acceptable for the mapping or whether the row stays
deferred.

## 7. What remains, ranked

1. The additive DISPLAY order of sums mixing named constants (or infinity) with numbers is still
   the old Rust order (`inf + 1` vs `1 + inf`, `0*inf + 1` vs `1 + 0*inf`). It costs, as far as
   this round measured, only display parity on expressions no deferred row asserts; a change to the
   shared term order was not attempted because it would move every ported rendering at once.
2. The two whole-registry shape sweeps and the pretty/latex complex-constant rows remain as
   recorded by round 30.
3. The real-trig family's three sibling rows (`RealTrigCertifiedPrefixTests` 2,
   `RealTrigSymmetryTests` 1) are not part of the 12 and were left alone; the certified-prefix
   goldens they need are now reproducible with the same kernel, so they are the cheapest next trig
   rows.
4. `limit` at an indeterminate point (`limit(inf - inf, x, 0)`) was not probed in a well-formed
   script; the 11 rows are the arithmetic forms, not limit records.

## 8. Files changed

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | `has_undefined_risk`/`has_pole_risk`; zero-coefficient/zero-factor retention in `add`/`mul`; `poly_coeffs` refuses a variable-carrying coefficient; `divide_out` only strips the variable and its integer powers; NamedConstant ordinal in `canonical_order` |
| `rust/lovelace-symbolics/src/indeterminate_forms_round31.rs` | new: 11 kernel tests |
| `rust/lovelace-lang/src/builtins.rs` | new inverse-branch solver (elementary/inverse/periodic/exponential family, residual verification, circular-answer enforcement, family records); elementary branch wired after the polynomial/linear dispatch; `solve_linear_symbolic` coefficient guard; family helpers |
| `rust/lovelace-lang/src/circular_family_round31.rs` | new: 10 wire-level tests for the 18 rows |
| `rust/lovelace-lang/src/numeric.rs` | named constants `pi`/`e`/`i` evaluable; `sin`/`cos` still use the ComplexMath kernel (`sin_cos_raw`) |
| `rust/lovelace-lang/src/complex_math.rs` | real trig of the complex surface uses the ComplexMath kernel explicitly |
| `rust/lovelace-lang/src/round26_complex_math_parity.rs` | the four `Real::sin/cos` call sites now name the ComplexMath kernel under test (`cm_sin`/`cm_cos`) |
| `rust/lovelace-lang/src/lib.rs` | module registration |
| `rust/lovelace-real/src/transcendental.rs` | the `Real.Sin/Cos` port (special-angle table, integer-quotient reduction, argument-aware pi width); `complex_math_sin/cos` keep the previous kernel; `Real::tan` composes the wire kernel |
| `rust/lovelace-real/src/round31_trig.rs` | new: the 12 trig rows |
| `rust/lovelace-real/src/lib.rs` | module registration |
| `docs/goal-parity-contract/round-31/circular-trig-indeterminate.md` | this report |
