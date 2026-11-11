# Round 7 — exact-rational preperiod fix and the Real periodic rows it unblocked

Oracle: `1cc28f255eca6029849f880949fc7be1d40ab8af`. This round fixes the confirmed public wire
defect in `rust/lovelace-real` (a periodic decimal with a non-empty preperiod converted to the
wrong rational) and then ports the Lovelace.Real.Tests rows that round 6 refused because of it.

## 1. The defect and the fix

`Real::to_fraction` (`rust/lovelace-real/src/lib.rs:349`) converted

```text
0.pre(per) = int_part + pre/10^s + per/(10^s * (10^k - 1))
```

but over the common denominator `10^s * (10^k - 1)` it added the repeating block as
`per * 10^s` instead of `per`:

```text
before: num = int_part*den + pre*(10^k - 1) + per*10^s
after:  num = int_part*den + pre*(10^k - 1) + per
```

Only the numerator term changed; the denominator, the storage, and every rendering path are
untouched. `to_rational` needed no change because it reduces the result of `to_fraction`; every
caller (`add`, `sub`, `mul`, `div`, `cmp`, `sqrt`, `pow_int`) therefore inherits the correct value.

Exact value of the second probe row, from the definition:

```text
0.27(02) = 27/100 + 2/(100*99) = 2675/9900 = 107/396   (not 1/4)
```

## 2. Wire probe — nine scripts (C# oracle vs Rust)

Both runners were invoked as `--eval '<script>' --omit-functions`; the structured
`numerator`/`denominator` fields and the display string are quoted. The display is identical in
every before/after case.

| script | display (C# = Rust) | C# oracle | Rust before | Rust after |
|---|---|---|---:|---:|
| `1/6` | `0.1(6)` | 1/6 | 23/30 | **1/6** |
| `1/7` | `0.(142857)` | 1/7 | 1/7 | 1/7 |
| `1/3` | `0.(3)` | 1/3 | 1/3 | 1/3 |
| `1/12` | `0.08(3)` | 1/12 | 31/75 | **1/12** |
| `0.1(6)` | `0.1(6)` | 1/6 | 23/30 | **1/6** |
| `0.27(02)` | `0.27(02)` | 107/396 | 2873/9900 | **107/396** |
| `2.5` | `2.5` | 5/2 | 5/2 | 5/2 |
| `-1/6` | `-0.1(6)` | -1/6 | -23/30 | **-1/6** |
| `10/3` | `3.(3)` | 10/3 | 10/3 | 10/3 |

The confirmed repro now agrees:

```text
Lovelace.Run ... --eval '1/6' --omit-functions -> numerator 1, denominator 6, display 0.1(6)
lovelace-run   --eval '1/6' --omit-functions -> numerator 1, denominator 6, display 0.1(6)
```

## 3. The 1,104 re-run rows: mappings added

Round 6 refused 1,104 Real rows under `PREC` (501 add corpus + 501 subtract corpus + 90 multiply
corpus + 12 fact/divide rows). Every one was re-run against the fixed conversion. 1,085 now
reproduce the C# assertion and are mapped; 19 still differ for a *different* reason
(digit budget, not conversion) and stay refused.

| Re-run set | Cases | Ported | Refused |
|---|---:|---:|---:|
| `RealAddPeriodicExactTests.Add_ExactRationalPair_Property_DenotesTheExactRationalSum` | 501 | 492 | 9 |
| `RealAddPeriodicExactTests.Subtract_ExactRationalPair_Property_DenotesTheExactRationalDifference` | 501 | 492 | 9 |
| `RealMultiplyPeriodicExactTests.Multiply_PeriodicQuotientTimesItsDivisor_Property_IsExactlyTheDividend` | 90 | 90 | 0 |
| The 12 fact/divide rows below | 12 | 11 | 1 |
| **total** | **1,104** | **1,085** | **19** |

The 11 ported fact rows:

- `RealAddTests.Add_GivenPeriodic1Over6Plus1Over6_ReturnsOneThird`
- `RealAddPeriodicExactTests.Subtract_OneThirdMinusNThrees_IsExactlyOneOverThreeTimesTenToTheN` (998, 999, 1000)
- `RealAddPeriodicExactTests.Subtract_PeriodicOperandMinusItsOwnTruncation_IsTheTruncationRemainder`
- `RealDivideExactnessPropertyTests.DividingPeriodicOperands_KeepsRepresentableQuotientsExact`
- `RealDivideExactnessPropertyTests.QuotientsThatCannotBeRepresented_StayInexact`
- `RealMultiplyPeriodicExactTests.Multiply_DividendOverDivisorTimesDivisor_AuditShapes_AreExactlyTheDividend` #2
- `RealMultiplyPeriodicExactTests.Multiply_PeriodicTimesNonPeriodic_IsExactWhenTheProductIsRational`
- `RealMultiplyPeriodicExactTests.Multiply_PeriodicTimesTruncatedIrrational_IsExactForTheStoredRationalsOnly`
- `RealPeriodicIntegerValuePropertyTests.PeriodicTailAfterNine_IsTheNextInteger`

All 1,085 tests live in the test-only module `rust/lovelace-real/src/real_parity_round7.rs`
(declared as `#[cfg(test)] mod real_parity_round7;`). It contains 1,074 corpus tests + 11 fact
tests + 4 direct conversion pins. Every mapping is a `ported` entry in
`parity/manifest-map.json` pointing at `lovelace-real::real_parity_round7::<test>`.

Coverage moved from 1,457/6,209 (23.47%) to **2,542/6,209 (40.94%)**; Lovelace.Real.Tests moved
from 1,201/2,537 (47.34%) to **2,286/2,537 (90.11%)**.

## 4. Remaining refusals from this re-run (19, all `DIGIT_BUDGET`)

The conversion is correct for these rows too, but the arithmetic caller's digit budget is not the
C# 1000-place budget, so the stored result is a truncation and the C# exact-rational assertion
still does not reproduce. Per the standard, they are refused rather than weakened. Changing that
budget would move displays and is outside this task's "conversion only" scope.

1. **18 corpus rows** — `Add` and `Subtract` #582-#584, #606-#608, #630-#632, operands
   `(p,q,r,s)` in `(1,19,1,17), (1,19,-1,17), (1,19,5,17), (2,19,1,17), (2,19,-1,17),
   (2,19,5,17), (-5,19,1,17), (-5,19,-1,17), (-5,19,5,17)`. `1/19 + 1/17 = 36/323` has a
   144-digit period; `fraction_real` is called with `max(input fractional digits)+40 = 58`, so
   the sum is stored truncated. (C# at 1000 places keeps the period.)
2. **1 fact row** — `RealDivideExactnessPropertyTests.DivisionAndItsMultiplicationTwin_AgreeOnValueAndExactness`.
   The 10x10 family fails for `(1/7)/(23/1) = 1/161`: `mul` allows 62 places for the twin
   `1/7 * 1/23` while the period is 66 digits, so the twin is truncated. C# at 1000 places keeps it.

Both reasons are persisted on `deferred` entries in `parity/manifest-map.json` (search
`DIGIT_BUDGET`), so `gen-manifest` no longer resets them to the generic reason.

## 5. Observed verification

```text
$ cargo test --manifest-path rust/Cargo.toml -p lovelace-real
running 2308 tests
test result: ok. 2308 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

$ cargo test --manifest-path rust/Cargo.toml --workspace
58 + 67 + 125 + 7 + 8 + 134 + 2308 + 27 = 2734 passed; 0 failed
(all crate results "ok", bins/doc-tests 0)

$ cargo run --release -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total
(the two failures are the pre-existing `sym/076` and its companion; unchanged from 212/214)

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust: 20/20
PASS: oracle 20/20, rust 20/20

$ powershell -File parity/gen-rust-tests.ps1
crates=11 tests=2734 suiteExit=0 passed=2734 failed=0

$ node parity/gen-manifest.mjs
{ "rows": 6209, "dispositions": { "deferred": 3667, "ported": 2542 }, "coveragePercent": 40.94, "rustTests": 2734 }

$ node parity/tools/gate-manifest.mjs
{ "censusCases": 6209, "manifestRows": 6209, "dispositions": { "ported": 2542, "consolidated": 0,
  "excluded": 0, "deferred": 3667 }, "coveragePercent": 40.94, "claimedRustTests": 2542,
  "projectsInCensus": 15, "failures": [] }
PASS (structural): 6209 test cases dispositioned; 2542 claim Rust coverage (40.94%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 2542/6209 ported (40.94%)
| Lovelace.Real.Tests | 2537 | 2286 | 251 | 0 | 90.11 % | port |
```

No display moved: all nine probe displays are byte-identical before and after, the corpus is still
212/214, and the 20 golden fixtures remain 20/20 for both oracle and Rust.

## 6. Files changed

- `rust/lovelace-real/src/lib.rs` — the three-line numerator fix in `to_fraction` (production) and
  the `#[cfg(test)] mod real_parity_round7;` declaration (tests only).
- `rust/lovelace-real/src/real_parity_round7.rs` — new test-only module (1,089 tests).
- `parity/manifest-map.json` — 1,085 new `ported` entries and 19 `deferred` entries with the
  `DIGIT_BUDGET` reason.
- `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated by the
  documented generators.
- `docs/goal-parity-contract/round-7/real-fraction-fix.md` — this report.

## 7. What was not verified

- The 19 refusals above: their C# exact-rational assertions still differ because of the arithmetic
  digit budget, so they remain `deferred` and are not claimed as coverage.
- The full differential envelope gate was not run in this task; the required gates (workspace
  tests, corpus, golden fixtures, manifest gate, coverage) were run and are quoted above.
- Only the nine requested scripts were probed on the wire; the 1,085 corpus/fact mappings are
  verified through the Rust assertions, not by individual `dotnet` invocations.
