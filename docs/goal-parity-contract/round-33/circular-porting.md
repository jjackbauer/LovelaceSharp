# Round 33 — the circular / trig / indeterminate rows, mapped

Scope of this round: **tests only**. The 41 census rows the round-31 production work claims to
unblock were re-read against their C# assertions, mapped to Rust tests that assert those exact
cases, and — where no test asserted the case — new tests were appended. The same fixes also unblock
24 adjacent rows (the two certified-prefix and one symmetry rows the round-26 sweep left, the 15
inverse-branch candidate-verification rows, and 6 family rows of `RelationsAndFamiliesTests`);
23 of those are mapped here and 1 is refused. The manifest was curated through
`parity/manifest-map.json` and regenerated with `parity/gen-manifest.mjs`.

No production code, no `.cs` file, no fixture and no corpus file was changed. The only files
written are four Rust test files (two existing ones appended to, two new modules plus their
`lib.rs` registration), `parity/manifest-map.json`, the manifest artifacts the gates regenerate
(`parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md`) and this report.

## 1. The 41 rows

37 of the 41 are mapped; 4 are refused with a measured reason (section 3).

### 1.1 Circular / inverse families — 18 rows, 17 mapped, 1 refused

All assertions of `Lovelace.Symbolics.Tests/InfiniteFamilyCompletenessTests.cs`. The Rust module is
`rust/lovelace-lang/src/circular_family_round31.rs` (`lovelace-lang::circular_family_round31::*`).

| Census id (`...InfiniteFamilyCompletenessTests/`) | Rust test |
|---|---|
| `CircularCandidates_AreNotRepresentedAndAreNotACompleteSet#0` (`exp(x) == x`, complex) | `circular_candidates_are_not_represented_and_are_not_a_complete_set` |
| `CircularCandidates_...#1` (`log(x) == x`, complex) | same |
| `CircularCandidates_...#2` (`x - log(x) == 0`) | same |
| `CircularCandidates_...#3` (`x - sin(x) == 0`) | same |
| `CircularCandidates_...#4` (`x - cos(x) == 0`) | same |
| `CircularCandidates_...#5` (`x*exp(x) == 1`) | same |
| `CircularCandidates_...#6` (`x + exp(x) == 0`, complex) | same |
| `CircularCandidates_...#7` (`exp(x) == x`, real) | same |
| `CircularCandidates_...#8` (`log(x) == x`, real) | same |
| `CircularCandidates_...#9` (`x + exp(x) == 0`, real) | same |
| `Kernel_ExpSymbolic_Complex_EveryFamilyMemberSolves#0` | **new** `kernel_exp_symbolic_complex_every_family_member_solves_in_a_complete_set` |
| `RealDomain_ExpKeepsTheSingleSolution#0` | **new** `real_domain_exp_keeps_the_single_solution_that_satisfies_the_equation` |
| `RealDomain_ExpSymbolic_KeepsItsBehaviour_AndNeverCarriesTheComplexPeriod#0` | `real_domain_exp_symbolic_keeps_its_behaviour_and_never_carries_the_complex_period` |
| `SolveFull_ExpNumeric_Complex_IsNotThePrincipalValueAlone#0` | `solve_full_exp_numeric_complex_is_not_the_principal_value_alone` |
| `SolveFull_ExpSymbolic_Complex_RepresentsTheWholeFamily#0` | `solve_full_exp_symbolic_complex_represents_the_whole_family` |
| `SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0` | **refused** (section 3.1) |
| `TheLegitimateInverseBranch_StillRepresentsItsValues#0` | `the_legitimate_inverse_branch_still_represents_its_values` |
| `TheOtherInverses_KeepTheirOwnHonestAnswer#0` | `the_other_inverses_keep_their_own_honest_answer` |

The ten `CircularCandidates` rows are one xUnit theory; the Rust test iterates the same ten
(equation, domain) pairs and asserts the C# row's four facts for each: not `Solved`, not
`NoSolutions`, `complete` false and `completeness` not `Complete`, and no published solution
value or family template whose canonical form contains `(sym x)`.

The two new tests close the only gaps in that file: the C# kernel row also asserts `Solved` /
`Complete`, one integer family, an x-free template, that each of k = -3, -1, 0, 1, 4 solves
`exp(x) = 3`, and that k = 0 and k = 1 are numerically distinct at 40 digits; the C# real-domain
row also asserts that the single represented value satisfies `exp(x) - 5` at 40 digits. The new
Rust tests assert both, the last two through the kernel's own `builtins::residual_is_zero` at the
same 40 digits.

### 1.2 Indeterminate forms — 11 rows, all mapped

`rust/lovelace-symbolics/src/indeterminate_forms_round31.rs`. Each C# fact has one Rust test of the
same name (snake_case), asserting the same constructor expression, the same "not a definite zero"
predicate, the infinity-survival predicate, and the exact `0*inf` pretty form where the C# row
does:

| Census id (`...IndeterminateFormTests/`) | Rust test (same fact) |
|---|---|
| `FiniteNamedConstantIdentities_StillFold#0` | `finite_named_constant_identities_still_fold` |
| `IndeterminateDifferenceEmbeddedInSum_DoesNotResurrectZero#0` | `indeterminate_difference_embedded_in_sum_does_not_resurrect_zero` |
| `InfinityMinusInfinity_IsNotDefiniteZero#0` | `infinity_minus_infinity_is_not_definite_zero` |
| `InfinityPlusOne_StaysUnevaluated#0` | `infinity_plus_one_stays_unevaluated` |
| `NestedInfinityDifference_IsNotDefiniteZero#0` | `nested_infinity_difference_is_not_definite_zero` |
| `Sibling_InfinityOverInfinity_StaysUnevaluated#0` | `sibling_infinity_over_infinity_stays_unevaluated` |
| `Sibling_InfinityToTheZero#0` | `sibling_infinity_to_the_zero` |
| `Sibling_ZeroOverZero_IsNotDefiniteZero#0` | `sibling_zero_over_zero_is_not_definite_zero` |
| `SymbolMinusItself_StillFoldsToZero#0` | `symbol_minus_itself_still_folds_to_zero` |
| `ZeroTimesInfinity_IsNotDefiniteZero#0` | `zero_times_infinity_is_not_definite_zero` |
| `ZeroTimesSymbol_StillFoldsToZero#0` | `zero_times_symbol_still_folds_to_zero` |

### 1.3 Real trig, large magnitudes — 9 rows, all mapped

`Lovelace.Real.Tests/RealTrigPrecisionOracleTests.cs:82` theory rows
`TheRequestedFractionalDigitsAreCorrect#5, #6, #7, #13, #14, #15, #21, #22, #23` are the
`Sin_1e20`, `Sin_1e100` and `Cos_1e100` cases at 30, 50 and 100 places. All nine are mapped to
`lovelace-real::round31_trig::requested_fractional_digits_are_correct_at_every_huge_magnitude`
(`rust/lovelace-real/src/round31_trig.rs:53`), which asserts that each published expansion starts
with the measured 30/50/100-digit prefix.

The nine goldens were independently verified this round (section 4.2), so the mapping is against
the true expansion, not merely against the C# rendering.

### 1.4 Real trig, ambient-pi table — 3 rows, all refused

`SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues#0/#1/#2` are **refused**: the
C# row evaluates `pi * num / den` through the public arithmetic, and the Rust public arithmetic
does not reproduce that input. See section 3.2 for the measurement.

## 2. Adjacent rows the same fixes unblock — 24 rows, 23 mapped, 1 refused

### 2.1 Certified-prefix and symmetry siblings (3 rows, all mapped)

`rust/lovelace-real/src/round31_trig.rs`:

| Census id | Rust test |
|---|---|
| `RealTrigCertifiedPrefixTests/Sin_And_Cos_OfTenToTheHundred_PublishTheDigitsTheyClaim#0` | **new** `sin_and_cos_of_ten_to_the_hundred_publish_the_digits_they_claim` |
| `RealTrigCertifiedPrefixTests/SpecialAngles_AreStillExactAtEveryPrecision#0` | **new** `special_angles_are_still_exact_at_every_precision` |
| `RealTrigSymmetryTests/Sin_GivenNegativeSpecialAngles_KeepsTheirExactValues#0` | **new** `sin_of_negative_special_angles_keeps_their_exact_values` |

The C# certified row runs under `WithPrecision(30, 30)` and compares sin/cos(10^100) with the two
mpmath prefixes its comments record; the Rust test asserts the same two prefixes and the published
width. The C# special-angles row runs under `WithPrecision(60, 40)`, whose COMPUTATION width is 60
(`Real.Sin` reads `MaxComputationDecimalPlaces`), so the Rust tests use 60.

### 2.2 Inverse-branch candidate verification (15 rows, 14 mapped, 1 refused)

`rust/lovelace-lang/src/inverse_branch_round33.rs` (`inverse_branch_round33::*`). These rows are
the reason the round-31 inverse gate exists, and all but one are now reproduced.

| Census id (`...InverseBranchCandidateVerificationTests/`) | Rust test |
|---|---|
| `SolveFull_SqrtPlusTwo_DoesNotClaimACompleteSolutionSet#0` | `solve_full_sqrt_plus_two_does_not_claim_a_complete_solution_set` |
| `Kernel_SqrtPlusTwo_IsNotSolvedAndRepresentsNothing#0` | `kernel_sqrt_plus_two_is_not_solved_and_represents_nothing` |
| `TheExtraneousCandidateFour_IsNotRepresented#0` | `the_extraneous_candidate_four_is_not_represented` |
| `SqrtMinusTwo_KeepsItsGenuineRoot#0` | `sqrt_minus_two_keeps_its_genuine_root` |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#0` (`sqrt(x) - 2`) | `every_represented_solution_satisfies_the_original_equation` |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#1` (`x^(1/3) - 3`) | same |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#2` (`exp(x) - 3`, family) | same |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#3` (`log(x) - 2`) | same |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` (`sqrt(x^2 + 1) - 2`) | **refused** (section 3.3) |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#5` (`x^2 - 4`) | same |
| `EveryRepresentedSolutionSatisfiesTheOriginalEquation#6` (`x^3 - 8`) | same |
| `UnityRootBranches_AreKeptWhenTheyAreGenuine#0` | `unity_root_branches_are_kept_when_they_are_genuine` |
| `UnityRootBranchWithOnlyExtraneousCandidates_IsNotACompleteClaim#0` | `unity_root_branch_with_only_extraneous_candidates_is_not_a_complete_claim` |
| `CandidateWithAnUndecidableResidual_IsKept#0` | `candidate_with_an_undecidable_residual_is_kept` |
| `PolynomialSolve_StaysSolvedAndCompleteWithTwoSolutions#0` | `polynomial_solve_stays_solved_and_complete_with_two_solutions` |

The C# theory rows call `Solvers.Solve` at the kernel boundary; the Rust tests assert the same
facts on the language's `solve_full` record (status, completeness, represented_count, solutions,
families, diagnostics) and verify the represented values with the kernel's own 40-digit residual
predicate. `CandidateWithAnUndecidableResidual_IsKept` also pins the principal member `log(y)`
of the `log(y) + 2*pi*k*i` family and the k = 2, y = 7 member solving `exp(x) = 7`.

### 2.3 Relations and families (6 rows, all mapped)

`rust/lovelace-lang/src/relations_families_round33.rs`:

| Census id (`...RelationsAndFamiliesTests/`) | Rust test |
|---|---|
| `SinFamily_MembersSatisfyEquation_Numerically#0` | `sin_family_members_satisfy_the_equation_numerically` |
| `SinFamily_MembersSatisfyEquation_Numerically#1` | same |
| `SinFamily_MembersSatisfyEquation_Numerically#2` | same |
| `Solve_CosHalf_ReturnsTwoFamilies#0` | `solve_cos_half_returns_two_families` |
| `Solve_ExpZero_Empty#0` | `solve_exp_zero_is_empty` |
| `Solve_SinBeyondRange_EmptyWithNote#0` | `solve_sin_beyond_range_is_empty_over_the_reals_and_unevaluated_over_the_complexes` |

The C# `SinFamily` theory substitutes k = 0, 1, 2 into the `k*pi` family and requires
`|sin(member)| < 10^-19` at 50 digits; the Rust test substitutes the same members and asserts the
same residual through the 40-digit / 10^-24 kernel predicate (a stricter budget). The
`SinBeyondRange` row's kernel `Note` is the wire's reason diagnostic, which the Rust test
requires to be non-empty for both the real and the complex answer.

## 3. Refusals (5, with measurements)

### 3.1 `SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0`

The C# row requires a live SymPy: `solveset(exp(x) - c, x, S.Complexes)` must print an ImageSet
over the Integers with the `2*_n*pi` period, `simplify(exp(log(c) + 2*pi*I*k) - c)` must be
`"0"`, and the concrete list for k in -3..3 must be seven zeros. The workspace has no SymPy bridge
and this machine has no `python3` (the Store alias stub prints "Python was not found"), so a Rust
test can only assert the shape the oracle measured (one integer-parameter family, period
`2*pi*i`, template `log(c) + 2*pi*i*k`, and `{log(5)}` over the reals). That is a weaker
assertion than the oracle call, so the row stays deferred. The shape is pinned by
`exp_over_complexes_is_an_imageset_not_one_value` but not claimed for the census row.

### 3.2 `SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues#0/#1/#2`

The C# row computes the angle with the public arithmetic under `WithPrecision(places, places)`:
`angle = pi * num / den`, then asserts `Real.Sin(angle)`/`Real.Cos(angle)` equal the exact table
value at 20, 50 and 100 places. The Rust public arithmetic does not produce that input:

- the reference `Real.Divide` stops the digit loop at `MaxComputationDecimalPlaces` PLACES and
  therefore returns the truncated decimal for these angles;
- the Rust `Real::div` additionally checks, after generating the last budgeted digit, whether the
  NEW remainder has already been seen. When the period starts one place before the cap, it detects
  the period and returns the exact rational.

Measured with the pinned C# assembly (a scratch probe under `rust/target/tmp/cstrigprobe`,
untracked, driving `Lovelace.Real`) and with the Rust `Real` kernel:

| places | angle | C# (reference) | Rust (public `pi.mul(num).div(den, places)`) |
|---:|---|---|---|
| 20 | 5*pi/3 | `5.23598775598298873076` (not periodic) | `5.2359877559829887307(6)` (periodic, exact) |
| 20 | `sin(5*pi/3)` | `-0.86602540378443864676` (table value) | non-exact Taylor value `-0.8660254037844386467659...` |
| 50 | pi/3 | `1.04719755119659774615421446109316762806572313312503` | `1.0471975511965977461542144610931676280657231331250(3)` |
| 50 | 5*pi/3 | `...562516` | `...56251(6)` (periodic) |

The table itself is correct for the value the reference's Divide produces: the existing
`exact_multiples_of_the_ambient_pi_keep_their_exact_table_values` test
(`rust/lovelace-real/src/round31_trig.rs:132`) asserts all 16 angles through
`transcendental::special_angle_candidate` and passes. But that is a different input from the one the
C# row evaluates: through the public operators the row's assertion is false in Rust, so the three
rows stay deferred. Fixing it needs a production change to `Real::div`'s period detection at the
budget boundary, which is outside this test-only round. (This also means the round-31 report's claim
that these three rows are covered overstates them.)

### 3.3 `EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` (`sqrt(x^2 + 1) - 2`)

The C# row expects `Solved` with two solutions. The Rust solver returns `Unevaluated` for that
shape: the nested inverse `sqrt(u) = 2 -> u = 4 -> x^2 = 3` is not reached. The other six rows of
the theory are mapped; this one cannot be reproduced without a production change.

## 4. Verification

### 4.1 Gates (run after the tests and the manifest were final)

```
$ powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=4489 suiteExit=0 passed=4489 failed=0
```

```
$ node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 5293, "deferred": 798 },
  "coveragePercent": 85.25,
  "rustTests": 4489
}

$ node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5293, "consolidated": 0, "excluded": 118, "deferred": 798 },
  "coveragePercent": 85.25,
  "claimedRustTests": 5293,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5293 claim Rust coverage (85.25%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 5293/6209 ported (85.25%)
```

Baseline at the start of this round: 5,233/6,209 (84.28 %), 118 exclusions, 858 deferred. After:
**5,293/6,209 (85.25 %), 118 exclusions, 798 deferred** — +60 rows, +0.97 points. By project:
Lovelace.Real.Tests 2,461 -> 2,473 (97.00 -> 97.48 %); Lovelace.Symbolics.Tests 754 -> 802
(61.50 -> 65.42 %).

```
$ rust/target/release/lovelace-corpus.exe
FAIL sym/071    (pre-existing)
FAIL sym/076    (pre-existing; linsolve simplifies x/x to 1 before printing)
corpus: 212 passed, 2 failed, 214 total

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20
```

The workspace suite grew from 4,471 to 4,489 tests (+18: 3 trig, 2 circular, 9 inverse-branch,
4 relations-and-families), all green, exit 0.

### 4.2 Independent verification of the nine trig goldens

No mpmath or SymPy interpreter is installed here, so the C# oracle tests skip. To keep the nine
precision rows honest the 30/50/100-digit constants were re-derived with an independent fixed-point
implementation (Node BigInt: Machin's formula for pi at 400 places, integer-remainder reduction of
10^20 and 10^100 modulo 2*pi, Taylor series at 400 places). All nine constants in
`rust/lovelace-real/src/round31_trig.rs` were reproduced digit for digit, including the 100-digit
ones. The two 30-digit certified prefixes also match the mpmath values the C# test's comments
record. This is recorded as evidence, not as a committed test (no fixtures were added).

### 4.3 Single-writer discipline

`parity/manifest-map.json` was re-read immediately before each write, and the patch programs
aborted if the bytes changed between load and write (they did not). The 65 curated rows were
checked against `parity/census.json` first; none had a prior non-deferred disposition.

## 5. Files changed

| File | Change |
|---|---|
| `rust/lovelace-real/src/round31_trig.rs` | 3 appended tests (certified-prefix x2, symmetry x1); module doc updated with the pi-row refusal |
| `rust/lovelace-lang/src/circular_family_round31.rs` | 2 appended tests (kernel-complete-set, real-domain residual) |
| `rust/lovelace-lang/src/inverse_branch_round33.rs` | new: 9 tests for the 14 mapped inverse-branch rows |
| `rust/lovelace-lang/src/relations_families_round33.rs` | new: 4 tests for the 6 mapped family rows |
| `rust/lovelace-lang/src/lib.rs` | test-module registration for the two new modules |
| `parity/manifest-map.json` | 60 ported rows + 5 refused rows (65 curated entries) |
| `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` | regenerated by the gates |
| `docs/goal-parity-contract/round-33/circular-porting.md` | this report |

## 6. What was not verified

- The C# suite itself was not run: the pinned test host aborts under this harness with a
  `Win32Exception (5) Access is denied` while setting up process-exit watching. The C# rows were
  read from source and, for the pi-row measurement, probed through the pinned assembly. The
  round-31 claim of 4,471 C#-green tests is taken as given by the orchestrator.
- The remaining `RealTrigFastPathTests` (10) and `RelationsAndFamiliesTests` non-family rows were
  not attempted; they need kernel internals (normalisation/termination helpers) or are unrelated to
  this round's fixes.
- The 100-digit trig goldens were verified by the independent implementation above, but not by
  mpmath itself (no interpreter). mpmath agrees with the 30-digit certified prefixes the repository
  records.
