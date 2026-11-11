# Round 48 — the Real fast-path internals surface (`fast_path`), exposed additively

**Status: implemented and verified; 9 of the 10 census rows ported, the tenth measured and left
deferred.** The ten rows are case #0 of `Lovelace.Real.Tests.RealTrigFastPathTests` — C# tests that
reach the reference `Real`'s *private* small-value fast paths by reflection instead of going
through the public arithmetic. The Rust kernel ports the same decisions
(`rust/lovelace-real/src/transcendental.rs`) but keeps every helper private, so no Rust surface
could assert the contracts. This round adds that surface:
`rust/lovelace-real/src/fast_path.rs` (production, additive), with the ten mirrored cases in
`rust/lovelace-real/src/fast_path_tests.rs`. No `.cs` file, fixture, `parity/` file or other
documentation was edited; the only changes to existing files are two module declarations in
`lib.rs` and ten `fn` → `pub(crate) fn` visibility widenings in `transcendental.rs` (no function
body was touched).

## 1. Result

| Item | Value |
|---|---|
| Missing API closed | the observable form of the reference's private fast-path layer: the path decision, the constants it uses, the values it produces |
| Census rows unblocked | **9 of 10** (`RealTrigFastPathTests`, one mirrored test each) |
| Census row measured and deferred | `Cos_AtDefaultPrecision_StaysInteractive#0` — a wall-clock contract the Rust kernel does not meet (30.2 s release vs its 10 s bound); see §6 |
| Production file | `rust/lovelace-real/src/fast_path.rs` (449 lines) |
| Test file | `rust/lovelace-real/src/fast_path_tests.rs` (622 lines, 9 tests) |
| Values asserted | 309 concrete input/expected pairs plus the 200-place series' term-by-term guards |
| Shared-file edits | `lib.rs`: `pub mod fast_path;` + `#[cfg(test)] mod fast_path_tests;`; `transcendental.rs`: 10 × `fn` → `pub(crate) fn` |

The surface is observational only: no production path calls it, so it cannot change a result. Where
the kernel already implements the reference's decision, the accessor calls the kernel's function
(the contract is the live one); where the kernel spells the same contract a simpler way, the
accessor re-expresses the reference's algorithm and the tests pin it against the kernel's own
expression **and** against the public arithmetic it replaces.

## 2. The C# surface, member by member (the internals map)

`Lovelace.Real/Real.cs`. "Line" is the member's declaration line. Every member below is `private`
in the reference (the test file uses `BindingFlags.NonPublic`).

| C# member | Line | Observable the rows assert | Rust accessor |
|---|---:|---|---|
| `Real.Exponent` (get/set) | 153 | stored decimal scale, compared digit for digit | `fast_path::exponent` |
| `Real.ToNatural()` (inherited from `Integer`) | `Integer.cs:152` | stored magnitude, compared as a digit string | `fast_path::magnitude` |
| `Real.IsPeriodic` | 172 | whether the stored digits carry a repeating block | `fast_path::is_periodic` |
| `Int.IsNegative` | `Integer.cs` | the stored sign (not a numeric comparison) | `fast_path::is_negative` |
| `new Real(Nat, bool, long)` (internal ctor) | 396-420 | a value at an explicit scale, trailing zeros intact | `fast_path::raw` |
| `Sqrt2Half`, `Sqrt3Half` (per-precision caches) | 1760, 1763 | the table's sqrt constants at the active precision | `fast_path::sqrt2_half`, `sqrt3_half` |
| `TrySpecialAngle` | 2150 | hit + the exact sin/cos of the 16 table angles | `fast_path::try_special_angle` |
| `SpecialAngleCandidate` | 2129 | the angle `pi*num/den` denotes at pi's scale | `fast_path::special_angle_candidate` |
| table rows | 2162-2180 | the 16 `(num, den, sin, cos)` tuples, in order | `fast_path::special_angle_table` |
| `ReduceToTwoPi(value, pi, twoPi)` | 2062 | the value folded into [0, 2π) | `fast_path::reduce_to_two_pi` (keeps the unused `pi`) |
| `WholeQuotient(left, right)` | 2084 | integer turn count, equal to `Truncate(Divide)` | `fast_path::whole_quotient` |
| `Truncate(r)` | 2753 | integer part at scale 0 (digit-string split) | `fast_path::truncate` |
| `HalfOf(value)` | 2110 | exact half of a pi-shaped value / the general division | `fast_path::half_of`, `half_path` |
| `Normalize(r)` | 2919 | canonical trailing-zero-free form | `fast_path::normalize` |
| `StripTrailingDecimalZeros` | 2945 | the galloping binary strip, with its constants | `fast_path::strip_trailing_decimal_zeros`, `INITIAL_STRIP_STEP`, `MAX_STRIP_STEP` |
| `ToInteger(zeros)` | 2836 | magnitude shifted by `zeros` places, with sign | `fast_path::to_integer` |
| `IsBelowDecimalGuard(term, guard, ref tenPower, ref tenPowerExponent)` | 2257 | `\|term\| < 10^-(guard+1)`, plus the carried power | `fast_path::is_below_decimal_guard`, `fast_path::DecimalGuard` |
| `CosTaylor`, `SinTaylor` | 2223, 2202 | the series sum at a given guard | `fast_path::cos_taylor`, `sin_taylor` |
| `Sin`/`Cos` path order | 1901-2009 | table first (caller's π), then reduction + series | `fast_path::trig_path`, `TrigPath` |
| `DivideNonPeriodic` | 1113 | the fixed-point quotient the Taylor loops divide with | used by `fast_path::cos_taylor` (kernel `transcendental::div_truncate`) |

The precision scope is `Real.WithPrecision(p, p)` (`Real.cs:94`), i.e. `MaxComputationDecimalPlaces = p`
(`Real.cs:71`). The Rust kernel has no global precision state: it takes `places`/`digits` as
arguments, so every accessor takes `places` explicitly. That is the only signature-level
translation in the file.

## 3. What the ten rows observe, and the values they expect

The expected values below are read from the C# case bodies, not from the test names. "p" is the
scope's computation precision (20, 80 and 200 in the sweeps).

1. **`SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly` (line 50).** For `(num, den)` in
   `(0,1), (1,6), (1,4), (1,3), (1,2), (2,3), (3,4), (5,6), (1,1), (7,6), (5,4), (4,3), (3,2), (5,3), (7,4), (11,6)`,
   the angle built as `pi * Real(num) / Real(den)` (public multiply and divide at p) must be
   recognised by the table with the exact values: `(0, 1)` for 0, `(1/2, sqrt3/2)` for π/6,
   `(sqrt2/2, sqrt2/2)` for π/4, `(sqrt3/2, 1/2)` for π/3, `(1, 0)` for π/2,
   `(sqrt3/2, -1/2)` for 2π/3, `(sqrt2/2, -sqrt2/2)` for 3π/4, `(1/2, -sqrt3/2)` for 5π/6,
   `(0, -1)` for π, `(-1/2, -sqrt3/2)` for 7π/6, `(-sqrt2/2, -sqrt2/2)` for 5π/4,
   `(-sqrt3/2, -1/2)` for 4π/3, `(-1, 0)` for 3π/2, `(-sqrt3/2, 1/2)` for 5π/3,
   `(-sqrt2/2, sqrt2/2)` for 7π/4 and `(-1/2, sqrt3/2)` for 11π/6. The table's
   `Sqrt2Half`/`Sqrt3Half` are built with the same public `Sqrt(x)/2` expression the case's
   expected values use.
2. **`NonSpecialAngles_AreNotCapturedByTheTable` (line 100).** `0.5, 1, 2, 7, 1/3` and the 101-digit
   literal
   `0.52359877559829887307710723054658381403286156656251763682915743205130273438103483310467247089035284466`
   (π/6 written out, not a special angle) must miss; and for every table row with `num != 0`,
   `pi*num/den - 10^-p` (one unit in the last place below the angle) must miss. The reference
   rejects a non-periodic candidate early when its own last stored place is above the angle's
   (`x.Exponent > angle.Exponent`, Real.cs:2188) and otherwise compares exactly.
3. **`ArgumentReduction_IntegerQuotient_MatchesTheDividedForm` (line 135).** For
   `0.5, 1, 2, 6, 6.28318530717958647692528676655900576839433879875021, 12, 100, 1000, 12345.6789, 0.000001`
   at each p: `WholeQuotient(value, twoPi)` must equal `Truncate(value / twoPi)` in **Exponent,
   digit string and value**, and `ReduceToTwoPi(value, pi, twoPi)` must equal `value - twoPi * q`
   (plus `twoPi` when that is negative), where `q` is that truncated quotient. The point of
   `WholeQuotient` is one integer division instead of the remainder-tracking decimal loop, so the
   equality is the contract.
4. **`SeriesTermination_IntegerTest_MatchesTheThresholdComparison` (line 171).** At `guard = 60`,
   `threshold = "0." + 60 zeros + "1" = 10^-61`; terms `0, 0.1, 1, 10^-61` (exactly on the
   threshold), `0.9e-61` (just under), `1.1e-61` (just over), `10^-60`, `9e-61`, `9e-60`, `12.5`,
   `0.999...` (60 nines); both signs. The guard's verdict must be exactly
   `Real.Abs(candidate) < threshold`.
5. **`SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms` (line 212).** At
   `guard = 200`, `x = 0.5` (200/200 scope): the cosine series' loop
   `term = DivideNonPeriodic(-term * x^2, (2k-1)(2k), guard)`, `sum += term` must agree with
   `|term| < 10^-201` **term by term while carrying the power of ten across iterations** (the case
   passes the previous `tenPower`/`tenPowerExponent` back in), run for more than 20 terms, and end
   on exactly the value `CosTaylor(0.5, 200)` produces.
6. **`Cos_AtDefaultPrecision_StaysInteractive` (line 270).** `WithPrecision(1000, 40)`,
   `Real.Cos(0.5)`: the result must not be periodic and the call must take under 10 000 ms. **Not
   met by Rust — see §6.**
7. **`Normalize_BinaryStrip_MatchesTheDigitStringStrip` (line 292).** Fourteen literals (`8.000`,
   `1`, `0.500`, `1.2300`, `100`, `0.000`, `-1.250`, `123.456`, `0.0`, `-0`, `0.000...10000`, `10`,
   `1.000...0`, `7.7000...0`) built with the internal magnitude/exponent constructor so the
   trailing zeros are still there; the binary strip must equal the *digit-string* strip
   (`ReferenceNormalize`, transcribed from the C# test) in exponent, digits and value.
8. **`ToInteger_BinaryShift_MatchesTheDigitPadding` (line 321).** Ten values
   (`1`, `-1`, `8.000`, `0.5`, `123.456`, `100`, `0`, `0.000`, `-0.25`, `3.14159...510`) × `zeros` in
   `{0, 1, 17, 18, 19, 40}`: the binary magnitude shift must equal `new Int(Nat.Parse(digits + zeros), Int.IsNegative(value))` in
   magnitude and sign.
9. **`HalfOf_MatchesTheDividedForm_ForPiShapedValues` (line 350).** For `p in {20, 80, 200, 1000}`:
   `pi.Exponent == -p`, `HalfOf(pi) == pi / 2` in exponent, digits and value, and `HalfOf(3)`,
   `HalfOf(0.25)`, `HalfOf(1.5)` (not stored at the active precision, so the general path) must
   equal the public division too.
10. **`Normalize_BinaryStrip_MatchesTheDigitStringStrip_ForLongZeroRuns` (line 379).** For
    `zeros in {0, 1, 17, 18, 19, 37, 200, 2000}` and `cap in {0, 1, 18, 19, zeros, zeros+1, zeros+500}`:
    `new Real(10^zeros, false, -cap)` stripped must equal `ReferenceNormalize` in exponent, digits
    and value — the long-run and cap-larger-than-digits shapes the galloping step exists for.

## 4. The Rust surface

### 4.1 Accessors that call the kernel's own function

These are the live contract, not a re-statement: `truncate`
(`transcendental::truncate_toward_zero`), `whole_quotient` (`transcendental::whole_quotient`, the
reduction's own quotient), `reduce_to_two_pi` (`transcendental::reduce_to_two_pi_whole`),
`try_special_angle`/`special_angle_table`/`special_angle_candidate` (the same functions
`Real::sin`/`Real::cos` call first), `is_below_decimal_guard` (the predicate the live Taylor loops
break on), `cos_taylor`/`sin_taylor`, and `trig_path`, which re-runs the kernel's decision sequence —
reduce against the caller's π, ask the table, then reduce against the argument-aware π and ask
again — so it cannot disagree with the kernel about a table hit. It deliberately does not replicate
the quadrant fold; the `Taylor` arm reports the reduced argument the series starts from.

### 4.2 Reference fast paths the kernel spells differently

The Rust kernel does not have all of these as separate functions, so the module carries the
reference's algorithm and the tests pin it three ways: against the reference's replaced expression
(the digit-string strip / the public divide / `Abs(term) < threshold`), against the kernel's own
expression where one exists (`is_below_decimal_guard`, `Real::normalized`), and against
`cos_taylor`.

| Accessor | Reference algorithm | Pinned by |
|---|---|---|
| `normalize`, `strip_trailing_decimal_zeros` | galloping decimal division: step 18, double on success, halve on failure, cap `2^40`; no digit rendering | rows 7 and 10, plus `Real::normalized` |
| `half_of`, `half_path` | `magnitude * 5` one place down when `Exponent == -places`; else the public division | row 9 |
| `to_integer` | `Nat.ShiftLeftDecimal(zeros)` on the magnitude, then the stored sign | row 8 |
| `DecimalGuard` | the integer guard with the carried `10^k` and its exponent | rows 4 and 5 |
| `raw`, `exponent`, `magnitude`, `is_negative`, `is_periodic` | the internal constructor and the four read-only observables | every row |

The constants the rows depend on are exposed as `INITIAL_STRIP_STEP` (18), `MAX_STRIP_STEP`
(`1 << 40`), `SERIES_GUARD_DIGITS` (10), plus `sqrt2_half`/`sqrt3_half`, which return exactly the
values the table's π/4 and π/6 rows hand back (the reference caches them per precision; the Rust
table rebuilds them per lookup).

### 4.3 Conventions

* No global precision state: the C# `using var scope = Real.WithPrecision(p, p)` becomes an
  explicit `places = p` argument, and `Real.Pi` becomes `Real::pi(p)`.
* `reduce_to_two_pi(value, pi, two_pi)` keeps the `pi` parameter for signature parity and documents
  it as unused: the reference's `ReduceToTwoPi` never reads it either.
* Every accessor is documented in the module (the crate inherits `missing_docs = warn`), and the
  module's table is the internals map above, one row per reference member.

## 5. The ten census ids

Manifest keys as they appear in `parity/manifest-map.json` (all `#0`, all
`Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/…`). **I did not edit `parity/`** —
the manifest flip for the nine is the orchestrator's.

| # | Census id (method name) | Rust test | Status |
|---:|---|---|---|
| 1 | `SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly#0` | `special_angles_built_the_original_way_still_hit_the_table_exactly` | ported |
| 2 | `NonSpecialAngles_AreNotCapturedByTheTable#0` | `non_special_angles_are_not_captured_by_the_table` | ported |
| 3 | `ArgumentReduction_IntegerQuotient_MatchesTheDividedForm#0` | `argument_reduction_integer_quotient_matches_the_divided_form` | ported |
| 4 | `SeriesTermination_IntegerTest_MatchesTheThresholdComparison#0` | `series_termination_integer_test_matches_the_threshold_comparison` | ported |
| 5 | `SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms#0` | `series_termination_integer_test_tracks_the_carried_power_across_many_terms` | ported |
| 6 | `Normalize_BinaryStrip_MatchesTheDigitStringStrip#0` | `normalize_binary_strip_matches_the_digit_string_strip` | ported |
| 7 | `ToInteger_BinaryShift_MatchesTheDigitPadding#0` | `to_integer_binary_shift_matches_the_digit_padding` | ported |
| 8 | `HalfOf_MatchesTheDividedForm_ForPiShapedValues#0` | `half_of_matches_the_divided_form_for_pi_shaped_values` | ported |
| 9 | `Normalize_BinaryStrip_MatchesTheDigitStringStrip_ForLongZeroRuns#0` | `normalize_binary_strip_matches_the_digit_string_strip_for_long_zero_runs` | ported |
| 10 | `Cos_AtDefaultPrecision_StaysInteractive#0` | — | **blocked** (measured, §6) |

## 6. The blocked row: `Cos_AtDefaultPrecision_StaysInteractive#0`

This one is not an API gap any more: the surface can now express the row, and the measurement says
the row's contract does not hold.

| Measurement (this machine, same tree) | Value |
|---|---:|
| `Real::cos(0.5)` at 1000 places, debug test build | **96.3 s** |
| `evalf(cos(0.5), 1000)`, release `lovelace-run.exe` end to end | **30.2 s** |
| C# row's bound (`elapsed < 10_000 ms`) | 10 s |
| Stored magnitude of the returned value | **204,270 digits** |
| `Real::pi(1000)` alone | 4 ms |

The result is correct — an approximation, not periodic — but the cost is the per-term scale growth
the reference's accounting produces: every Taylor term is a `DivideNonPeriodic` quotient whose
exponent falls by the guard places (`Real.cs:1126-1129`; the loop at `2223-2241`), so the stored
magnitude of term k is about k × guard digits long. The .NET bignum absorbs that — the C# row's
own comment records its measured cost as about 1 s and calls the 10 s bound "an order of magnitude
above" it — while `lovelace-nat`'s division does not. Normalising the quotient does not help (its
last digit is generally non-zero), so the fix is a production decision: stop the scale growth
(truncate each term to the value's own scale, as `TruncateFrac` does) or speed up large `Nat`
division. Both are outside an additive-surface round, so the row keeps its deferred disposition
with the numbers above instead of a mirrored test that would have to be weakened.

I could not run the C# case itself to compare on this machine: the .NET testhost is refused by the
sandbox (`Win32Exception (5): Access is denied` when it opens its own process handle), so the
reference number cited is the C# row's own comment, not a measurement of mine.

## 7. Verification (observed this round)

| Check | Command | Observed |
|---|---|---|
| Rust suite | `cargo test --workspace` | `crates=11 tests=4961 suiteExit=0 passed=4961 failed=0` (was 4,952; +9) |
| New tests | `cargo test -p lovelace-real fast_path_tests::` | `9 passed; 0 failed; … finished in 0.85s` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| Formatting | `cargo fmt --check` | exit 0 |
| Documentation corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Protocol fixtures | `powershell -File parity/gate-fixtures.ps1` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `powershell -File parity/gate-cli.ps1` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| Release build | `cargo build --release -p lovelace-run -p lovelace-corpus` | `Finished release profile … in 21.76s` |

## 8. Notes and limits

* **The expected values are constructions, not decimal literals.** The C# rows assert equality with
  `Real.Sqrt(2)/2`, `Real.Sqrt(3)/2`, `Real.One`, `Real.Zero` and the public division; the Rust
  tests assert the same constructions (via `fast_path::sqrt2_half`/`sqrt3_half` and the public
  `mul`/`div`), at each precision, so a table drift in either language fails the test.
* **Row 1 was probed first.** `docs/goal-parity-contract/round-33/circular-porting.md` records a
  Rust divergence in the public `pi*num/den` construction ("one place earlier" period detection).
  Before writing the test I probed all 16 angles at 20, 80 and 200 places: **all 48 hits** are table
  hits, and the public angle equals `SpecialAngleCandidate` in every case. The test asserts that
  equality explicitly, so if the divergence ever appears the row fails loudly rather than silently
  falling into the series.
* **No arithmetic changed.** `git diff` for the crate is 3 added lines in `lib.rs` and ten
  `fn` → `pub(crate) fn` lines in `transcendental.rs`; no function body, constant or call order
  moved. The surface is `pub` but has no production caller.
* **The two normalize tests also pin the kernel's own strip**:
  `value.clone().normalized()` must equal the galloping result, so `fast_path::normalize` cannot
  drift from `Real::normalized` while agreeing with the digit-string reference.
* **What remains**: flip the nine ids in `parity/manifest-map.json` (coverage moves from 5,788/6,209
  to 5,797/6,209, 93.36 %), and take the row-6 cost decision in a production round. The nine tests
  are the row contracts; nothing else is waiting on this surface.
