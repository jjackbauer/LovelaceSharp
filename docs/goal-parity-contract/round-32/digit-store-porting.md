# Round 32 — mapping the digit-store and complex-closed-form tests into parity

Scope: `parity/manifest-map.json` plus this report, and the parity artifacts the gates
regenerate (`parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md`). No Rust
production file, no `.cs` file, no fixture, no corpus file and no other document was changed by
this port.

Baseline: 5,115/6,209 ported (82.38 %), 118 excluded, 976 deferred, suite 4,458 green.

Result: **5,233/6,209 ported (84.28 %), 858 deferred, 118 excluded — 118 new verified mappings**
and no tests appended, because every mapped id already had a covering Rust test:

| Family | Rows | Rust test(s) |
|---|---:|---|
| `Lovelace.Representation.Tests` digit store | 90 | `lovelace-nat::digit_store::tests::*` (85 mirror functions; theories share one) |
| `Lovelace.Symbolics.Tests.ComplexClosedFormTests` | 11 | 9 `lovelace-symbolics::complex_closed_forms_round30::*` + 2 `lovelace-run::round30_run_batch::*` quadratic solution-set tests |
| `Lovelace.Symbolics.Tests.HelpMetadataCompletenessTests` (adjacent) | 17 | 5 `lovelace-lang::help::tests::*` functions (the 13 alias rows share one theory) |
| **total** | **118** | |

## 1. Method

1. The ids were read from `parity/census.json` by project, class, method and case index; the 90
   Representation ids are every case in `Lovelace.Representation.Tests` except the empty
   `UnitTest1/Test1#0` placeholder, and the 11 closed-form ids are every case in
   `ComplexClosedFormTests`.
2. Every C# assertion was re-read from the C# source before mapping. A row was counted only when
   the Rust test asserts the same input and the same expected value or state, not merely a test
   with a similar name.
3. The Representation mirror relationship is exact by method name: all 90 census methods resolve to
   a function of the same name in `rust/lovelace-nat/src/digit_store.rs`. The three
   `SnapshotDigits_GivenTenDigitSequence...` theory cases share one function, and the four
   `SnapshotDigits_LengthEqualsDigitCount` cases share one function, which accounts for the 90 ids
   and 85 mirror functions.
4. The 9 closed-form rows in `rust/lovelace-symbolics/src/complex_closed_forms_round30.rs` are the
   one-to-one snake_case mirrors of the C# method names; the two quadratic rows are the two tests in
   `rust/lovelace-run/src/round30_run_batch.rs` whose comments and assertions name
   `QuadraticWithNegativeDiscriminant_YieldsTheExactComplexSolutionSet` and
   `...CoversANonZeroRealPart`. They assert the exact canonical solution set, the completeness and
   unrepresented count, and that every root substitutes back to exactly zero.
5. The 17 adjacent help ids were not in the 140-row round-30 port. `rust/lovelace-lang/src/help.rs`
   already had the five tests that pin them: complete metadata, optional-parameter signatures, the
   13 category aliases, the `funcs` name-search fallback and the `funcs(None)` category listing.
6. `parity/manifest-map.json` was re-read immediately before writing. The new file adds exactly
   118 keys and changes or removes none: `git diff` is 590 inserted lines (118 entries at five
   lines each) and 0 deleted lines, against HEAD.
7. Each note records the C# file and line that states the assertion and the Rust file, line and test
   id that re-asserts it, so the mapping is auditable without re-deriving it from the test name.

## 2. Representation rows (90)

All ids are `Lovelace.Representation.Tests`; the Rust crate is `lovelace-nat` and every test is in
`rust/lovelace-nat/src/digit_store.rs` under `mod tests`, so the Rust id is
`lovelace-nat::digit_store::tests::<C# method name>`.

| C# test class | Cases | Rust functions |
|---|---:|---|
| `DigitStoreTests` | 44 | 44 (one per method) |
| `DigitStoreSnapshotDigitsTests` | 13 | 9 (two theories share one function each) |
| `DigitStoreResetTests` | 7 | 7 |
| `DigitStoreCopyDigitsFromTests` | 6 | 6 |
| `DigitStoreMutatorTests` | 5 | 5 |
| `DigitStoreTrimLeadingZerosTests` | 6 | 6 |
| `DigitStoreToStringParallelTests` | 5 | 5 |
| `DigitStoreThreadSafetyTests` | 4 | 4 |
| **total** | **90** | **85** |

The mapped assertions include the constructors and copy constructor, digit/nibble access, the
snapshot copy and its independence, `GrowDigits`/`ShrinkDigits` sentinels (0x0C/0x0F), `Reset`,
`CopyDigitsFrom` including the zero-source no-op and the cross-copy lock-order test, the mutator
metadata, leading-zero trimming, both `ToString` overloads, and the four concurrency scenarios.

Deliberately not mapped: `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0`.
The C# body is empty; there is no input, assertion or expected output to mirror. It stays deferred.

## 3. ComplexClosedForm rows (11)

| Census method | Rust test |
|---|---|
| `SqrtOfNegativeOne_IsTheExactImaginaryUnit` | `lovelace-symbolics::complex_closed_forms_round30::sqrt_of_negative_one_is_the_exact_imaginary_unit` |
| `SqrtOfNegativeFour_IsExactlyTwoI` | `lovelace-symbolics::complex_closed_forms_round30::sqrt_of_negative_four_is_exactly_two_i` |
| `SqrtOfNegativePerfectSquares_AreExactImaginaryConstants` | `lovelace-symbolics::complex_closed_forms_round30::sqrt_of_negative_perfect_squares_are_exact_imaginary_constants` |
| `SqrtOfNegativeNonSquare_IsTheExactImaginaryMultipleOfTheRadical` | `lovelace-symbolics::complex_closed_forms_round30::sqrt_of_negative_non_square_is_the_exact_imaginary_multiple_of_the_radical` |
| `NegativeBaseRationalPowers_MatchTheSympyPrincipalBranch` | `lovelace-symbolics::complex_closed_forms_round30::negative_base_rational_powers_match_the_sympy_principal_branch` |
| `ImaginaryUnitIntegerPowers_FoldExactly` | `lovelace-symbolics::complex_closed_forms_round30::imaginary_unit_integer_powers_fold_exactly` |
| `PositiveBaseNonIntegerPower_IsStillLeftUnevaluated` | `lovelace-symbolics::complex_closed_forms_round30::positive_base_non_integer_power_is_still_left_unevaluated` |
| `NegativeBaseThirdRoot_IsOutsideTheBoundedSet` | `lovelace-symbolics::complex_closed_forms_round30::negative_base_third_root_is_outside_the_bounded_set` |
| `SqrtOfANonNegativeValue_IsUnchanged` | `lovelace-symbolics::complex_closed_forms_round30::sqrt_of_a_non_negative_value_is_unchanged` |
| `QuadraticWithNegativeDiscriminant_YieldsTheExactComplexSolutionSet` | `lovelace-run::round30_run_batch::complex_quadratic_with_negative_discriminant_yields_the_exact_complex_solution_set` |
| `QuadraticWithNegativeDiscriminant_CoversANonZeroRealPart` | `lovelace-run::round30_run_batch::complex_quadratic_covers_a_non_zero_real_part` |

The two quadratic tests are the only closed-form rows whose Rust coverage is at the language/record
boundary rather than the symbolics kernel; they assert the same canonical roots, the zero
`unrepresented_count`, and zero after expansion of each substituted root.

## 4. Adjacent help rows (17)

`HelpMetadataCompletenessTests` was fixed by the round-30 help renderer but its rows were never
mapped. `rust/lovelace-lang/src/help.rs` already carries the five matching tests:

| Census methods | Cases | Rust test |
|---|---:|---|
| `EveryBuiltin_HasCompleteMetadata` | 1 | `help::tests::every_builtin_has_complete_metadata` |
| `Signature_DistinguishesOptionalParameters` | 1 | `help::tests::signature_distinguishes_optional_parameters` |
| `CategoryAliases_Resolve` #0-#12 | 13 | `help::tests::category_aliases_resolve` |
| `Funcs_FallsBackToNameSearch` | 1 | `help::tests::funcs_falls_back_to_name_search` |
| `Funcs_ListsEveryBuiltinUnderACategory` | 1 | `help::tests::funcs_lists_every_builtin_under_a_category` |

Each alias row is mapped to the theory that loops over the same 13 aliases in the same order, so
every case has a concrete resolved alias rather than a generic "aliases work" claim.

## 5. Verification observed

Source stability: the newest `.rs` file was `2026-11-11 13:14:54 UTC`; the final inventory, gate
and report were produced at `13:20 UTC`, more than five minutes later, and the gate's
`sourcesSha256` matched the measured tree. The concurrent production lane's 13 new tests are
counted in the measured 4,471; this port changed no Rust source.

```text
$ powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=4471 suiteExit=0 passed=4471 failed=0

$ node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5233,
    "deferred": 858
  },
  "coveragePercent": 84.28,
  "rustTests": 4471
}

$ node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5233, "consolidated": 0, "excluded": 118, "deferred": 858 },
  "coveragePercent": 84.28,
  "claimedRustTests": 5233,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5233 claim Rust coverage (84.28%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 5233/6209 ported (84.28%)

$ rust\target\release\lovelace-corpus.exe   (TMP=rust/target/tmp)
corpus: 212 passed, 2 failed, 214 total

$ node parity/tools/gate-fixtures.mjs
PASS: oracle 20/20, rust 20/20
```

The two corpus failures are the pre-existing `sym/071` and `sym/076` recorded in the round-23,
round-30 and round-31 reports; they are untouched by this mapping.

## 6. Refusals and limits

- `UnitTest1/Test1#0` is refused: the C# body is empty, so a Rust test would be vacuous. The row
  remains deferred.
- The C# xUnit suite cannot be executed in this environment (the VSTest testhost limitation recorded
  in `parity/README.md`); the C# source is the assertion contract and every mapped assertion was
  transcribed from it.
- The two quadratic rows do not separately assert the record's `domain` field, which the C# row also
  checks; their exact complex canonical roots, completeness and zero-substitution checks establish
  the same concrete solution set. No test was appended: the scope permits appends only for a listed
  id with no covering test, and both already have one.
- The differential runner was not run; it does not generate these surfaces.

## 7. Files changed

| File | Change |
|---|---|
| `parity/manifest-map.json` | 118 new `ported` entries (90 Representation + 11 closed form + 17 help); no existing entry changed or removed |
| `parity/manifest.toml` | regenerated by `parity/gen-manifest.mjs` |
| `parity/rust-tests.json` | regenerated by `parity/gen-rust-tests.ps1` (4,471 tests, suite exit 0) |
| `parity/coverage.md` | regenerated by `parity/tools/coverage-report.mjs` |
| `docs/goal-parity-contract/round-32/digit-store-porting.md` | this report |

Not changed by this port: `rust/lovelace-nat/src/digit_store.rs` and its round-31 report,
`rust/lovelace-symbolics/src/complex_closed_forms_round30.rs`,
`rust/lovelace-lang/src/help.rs`, and the concurrent lane's production files (`builtins.rs`,
`numeric.rs`, `complex_math.rs`, the round-31 trig/circular modules).
