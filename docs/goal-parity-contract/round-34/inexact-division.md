# Round 34 — inexact-operand division: the reference's truncation at the computation cap

Status: implemented and measured. The three round-33 refused `RealTrigPiResolution` rows now evaluate
through the public arithmetic at the reference's own budget and reproduce the reference values; the
workspace, corpus, fixture, CLI and differential gates are green or unchanged (differential delta 0).

Scope of this round's writes:

| File | Change |
|---|---|
| `rust/lovelace-real/src/lib.rs` | the reference's `Divide` digit loop: budget-boundary stop, freed leading-zero accounting, denominator-only termination, `Normalize` of non-periodic results |
| `rust/lovelace-real/src/round34_inexact_division.rs` | **new**: 5 tests (ten probes, cap truncation, denominator-only termination, the wide-budget control, the three rows) |
| `rust/lovelace-real/src/real_parity_round7.rs` | one internal helper cross-check updated to the reference's measured cap behaviour (stronger, not weaker; §4) |
| `rust/lovelace-lang/src/builtins.rs` | `/` runs at the ambient `setprecision` cap instead of a 64-place floor |

No `.cs` file, fixture, corpus file or `parity/` artifact was changed. (Other files in the round-34
commit belong to the concurrent symbolics-tail lane.)

## 1. What the reference does — and where Rust differed

The reference is the pinned C# assembly (`parity/oracle.json`, commit `1cc28f2`), driven directly
through `Lovelace.Real` by a scratch probe under `rust/target/tmp/inexactprobe` (untracked), the
same method round 33 used. `Real.Divide`'s digit loop has four behaviours Rust did not reproduce:

1. **The budget stops the loop before the next remainder test.** The C# `while` condition is checked
   at the top of the iteration; a period that closes *exactly at* `MaxComputationDecimalPlaces` is
   therefore never discovered. The quotient is the budgeted truncation, inexact, with no period.
   Rust looked one iteration ahead at the boundary and published the exact period:
   `5*pi/3` at 20 places came back `5.2359877559829887307(6)` where the reference stores
   `5.23598775598298873076`; `pi/3` at 50 had the same one-place-early shape.
2. **Non-periodic results are normalised.** C# runs `Normalize` on every non-periodic quotient, so
   `2*pi/3` at 20 places is `2.0943951023931954923`, not the truncation's `...9230`.
3. **Termination is decided from the DENOMINATOR alone.** `TryTerminateExactly` does not cancel the
   numerator. `pi(100)`'s stored magnitude happens to be divisible by 3, so the exact value of
   `pi(100)/6` has 101 fractional digits; the reference still spends the 100-place budget in the
   digit loop and stores the 100-digit truncation `...446`. Rust gcd-reduced first, published the
   101st digit, and the 100-place pi-resolution angles consequently missed the special-angle table.
4. **A leading zero run is scale, not digits.** Once the opening run alone fills the budget the
   reference stops charging those places and bounds stored digits instead. This is what keeps
   `sin(1e-100)/cos(1e-100)` from being flushed to zero; fixing only (3) exposed that Rust relied on
   the gcd reduction here (see §4).

One engine-level divergence sat on top: `builtins::div` imposed `precision.max(64)`, so a script
under `setprecision(18)` divided at 64 places while the reference (and the class-Real side of
`LRealPeriodicFastPathTests`) divides at 18.

## 2. The ten probes (before/after vs. the oracle)

Budgets are the row's own `Real.WithPrecision(places, places)`; "before" is the pre-change Rust
public arithmetic at the same budget, "after" is this round. The oracle column is the pinned C#
assembly's `ToString`, exactness and period metadata. Every after value matches the oracle
byte-for-byte and in exactness; every after is non-periodic except the four exact controls.

| Script | places | Rust before | Rust after | C# oracle | verdict |
|---|---:|---|---|---|---|
| `pi(20)*5/3` | 20 | `5.2359877559829887307(6)` exact=false | `5.23598775598298873076` exact=false | `5.23598775598298873076` exact=false, periodic=false, exp=-20 | MATCH |
| `pi(50)/3` | 50 | `1.0471975511965977461542144610931676280657231331250(3)` exact=false | `1.04719755119659774615421446109316762806572313312503` exact=false | `1.04719755119659774615421446109316762806572313312503` exact=false, periodic=false, exp=-50 | MATCH |
| `pi(30)/7` | 30 | `0.448798950512827605494663340468` exact=false | `0.448798950512827605494663340468` exact=false | `0.448798950512827605494663340468` exact=false, periodic=false, exp=-30 | MATCH |
| `pi(20)*2/3` | 20 | `2.09439510239319549230` exact=false | `2.0943951023931954923` exact=false | `2.0943951023931954923` exact=false, periodic=false, exp=-19 | MATCH |
| `e(20)/3` | 20 | `0.90609394281968174512` exact=false | `0.90609394281968174512` exact=false | `0.90609394281968174512` exact=false, periodic=false, exp=-20 | MATCH |
| `1/3` | 20 | `0.(3)` exact=true | `0.(3)` exact=true | `0.(3)` exact=true, periodic=true, 0/1 | MATCH |
| `1/7` | 20 | `0.(142857)` exact=true | `0.(142857)` exact=true | `0.(142857)` exact=true, periodic=true, 0/6 | MATCH |
| `10/3` | 20 | `3.(3)` exact=true | `3.(3)` exact=true | `3.(3)` exact=true, periodic=true, 0/1 | MATCH |
| `1/4` | 20 | `0.25` exact=true | `0.25` exact=true | `0.25` exact=true, periodic=false, exp=-2 | MATCH |
| `pi(50)*5/3` | 50 | `5.2359877559829887307710723054658381403286156656251(6)` exact=false | `5.23598775598298873077107230546583814032861566562516` exact=false | `5.23598775598298873077107230546583814032861566562516` exact=false, periodic=false, exp=-50 | MATCH |

`pi(30)/7` and `e(20)/3` were already correct before (controls: the fix must not overreach);
`pi(20)*2/3` additionally needed rule (2). The exact controls keep their exact periodic/terminating
representations and denominators.

`an_inexact_quotient_is_still_periodic_when_its_period_closes_before_the_cap` pins the other side of
the rule, measured at `WithPrecision(1000,1000)`: `pi(20)*5/3` = `5.2359877559829887307(6)`,
exact=false, PeriodStart 19, PeriodLength 1. The rule is the BUDGET, not the operand's provenance.

## 3. The three refused rows (round 33 §3.2)

The rows build `angle = pi * num / den` with the public arithmetic under
`Real.WithPrecision(places, places)` and assert `Real.Sin(angle)`/`Real.Cos(angle)` equal the exact
table value for the 16 special angles at 20/50/100 places.

- **Before**: at 20 places `5*pi/3` was `5.2359877559829887307(6)`; at 50 `pi/3` and
  `5*pi/3` were periodic; at 100 the den = 6 angles carried the 101st digit. In each case the
  angle differed from the reference's candidate, the table lookup missed, and Sin/Cos fell through
  to the Taylor series: non-exact values, and at 100 places multi-kilobyte non-table values
  (measured in the before probe).
- **After**: all 48 public-arithmetic angles equal `transcendental::special_angle_candidate` — the
  reference's own candidate construction — and Sin/Cos equal the table values with the table's
  exactness flags. Pinned by
  `round34_inexact_division::exact_multiples_of_the_ambient_pi_match_the_reference_rows`
  (16 angles x 20/50/100).
- **Byte-level cross-check against the pinned assembly**: a probe printed each row's angle, sin and
  cos display and each exactness flag on both sides (48 rows x 6 fields = 288 comparisons); after
  applying only the reference's own non-periodic display truncation at `places` to the Rust strings,
  **288/288 agree**. The comparison program and both probe outputs are under
  `rust/target/tmp/`: `inexactprobe/Program.rows.cs` (the C# row body, rebuilt from the test file),
  `compare-round34-rows.mjs` (the comparison; output `round34-row-compare.txt` = "comparisons=288
  mismatches=0"), `oracle-rows.txt` and `divprobe-after.txt`.

The language runner is not a valid oracle for these expressions: under `setprecision(20)`, the C#
runner itself prints `5.(2)` for `(pi(20)*5)/3` and `1.(0)` for `pi(20)/3` (its numeric-operator
path, not `Real.Divide`), which is why round 31 recorded the real-trig rows as not wire-reachable.
The verification is therefore through the pinned `Lovelace.Real` assembly, as in round 33.

## 4. Cross-crate regressions (production fixed; the existing assertions were kept)

The first full workspace run after the division fix was red in `lovelace-lang`; root causes and
production repairs:

1. `round26_complex_math_parity::complex_math_tiny_values_are_not_flushed_to_zero` —
   `tan(1e-100)` became `0`. Cause: rule (3) alone removed the gcd shortcut this quotient had used;
   the true quotient is `10^-100` but its 100-place leading-zero run exhausted the 30-place budget.
   Fix: rule (4), the reference's freed leading-zero accounting.
2. `suite_parity_batch::r15_periodic_expression_at_limited_precision_matches_class_real` —
   `(1/17)+(1/19)` at 18 places. Cause: the engine divided at 64 places while the test's class-Real
   side (like the C# test) divides at 18, where `1/19`'s 18-digit period closes exactly at the cap
   and truncates. Fix: removed the `precision.max(64)` floor; `setprecision(n)` is the ambient
   computation cap, as `Real.WithPrecision(n, n)` is in C#.
3. `dsp_round21::cos_given_pi_over_6_is_sqrt3_over_2` — same engine-budget cause; with the floor
   gone the DSP angle is the reference's cap value and the table hit is restored.

One pre-existing Rust test encoded the removed peek: `real_parity_round7`'s
`check_third_minus_n_threes` compared the difference to `fraction_real(1, 3*10^n, n+1)`, which the
reference returns as a *truncation*. The assertion now (a) still checks the exact period, at budget
`n+2`, and (b) additionally pins the n+1 truncation (inexact, `n` zeros then `3`). That is stricter
than the single old assertion; no assertion was removed from the tests the parent named.

## 5. Gates (all measured after the final source state)

```
$ cd rust; cargo test --workspace --no-fail-fast
array 58, desktop 67, int 125, json 7, lang 1231, nat 226, real 2427,
run 346, symbolics 75, run integration 1  =  4,563 tests, 0 failed, exit 0
```
(`rust/target/tmp/round34-workspace-final2.log`; the new tests are 5 of the real crate's 2427.)

```
$ rust/target/release/lovelace-corpus.exe        (from the repo root, standard TMP/TEMP)
FAIL sym/071
FAIL sym/076
corpus: 212 passed, 2 failed, 214 total          (the same two pre-existing examples)
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

Differential, measured both ways with the same oracle and the same harness:

```
$ node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
before (runner sha256 818e28b3...):  generated 200 = 157 match / 43 mismatch / 0 error
                                    regressions 1752 = 706 match / 1046 mismatch
                                    new=0, resolved=706, stillMismatching=1046, exit 1
after  (runner sha256 76e3e8f0...):  generated 200 = 157 match / 43 mismatch / 0 error
                                    regressions 1752 = 706 match / 1046 mismatch
                                    new=0, resolved=706, stillMismatching=1046, exit 1
```

**Differential delta: 0** — no generated case and no replayed regression changed status. The "before"
binary is the pre-change release build preserved at `rust/target/tmp/round34-baseline/`; the after
binary is the round-34 build of `rust/target/release/lovelace-run.exe`.

## 6. Tests added

`rust/lovelace-real/src/round34_inexact_division.rs` (5 tests):

| Test | Pins |
|---|---|
| `inexact_operand_quotients_are_the_reference_truncations_at_the_budget` | the ten probe values/exactness of §2 |
| `the_reference_truncates_a_period_that_closes_exactly_at_the_cap` | 1/7 at 5/6 places truncated inexact, at 7 places `0.(142857)` exact (exact operands too) |
| `termination_is_decided_from_the_denominator_like_the_reference` | `pi(100)/6` truncates to 100 digits (`...446`), inexact |
| `an_inexact_quotient_is_still_periodic_when_its_period_closes_before_the_cap` | budget rule, not operand provenance: `5.2359877559829887307(6)`, PeriodStart 19 |
| `exact_multiples_of_the_ambient_pi_match_the_reference_rows` | all 16 angles x 20/50/100: public arithmetic == reference candidate, Sin/Cos == reference table + exactness flags |

## 7. What remains

- **DisplayDecimalPlaces is still not modelled by `Real::to_string`.** The reference truncates a
  non-periodic `ToString` at `DisplayDecimalPlaces`; Rust prints every stored digit. The values are
  equal (e.g. `pi(50)/4` stores 51 digits in both and the reference prints 50), and the rows above are
  value assertions, but a *printed* value can still differ in trailing digits. This is a display rule
  outside this round and deliberately not changed ("do not change display rules for exact values").
- The three manifest rows #0/#1/#2 are still `deferred` in `parity/manifest.toml`; remapping them is a
  `parity/` edit (outside this round's scope). The evidence for the mapping is the test above plus
  §3's 288/288 comparison.
- The language runner's Real/Integer operator path (`5.(2)` for `(pi(20)*5)/3` on the C# side) remains
  a separate, pre-existing divergence; the rows are library-level and were never wire-reachable.
