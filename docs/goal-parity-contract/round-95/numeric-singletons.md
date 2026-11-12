# Round 95 — eight numeric singleton deferred rows

**Status: 8/8 resolved. Six production fixes, two rows already matched (no production change was
needed for them). 0 blocked.**

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`.
The eight ids below are exactly the `disposition = "deferred"` rows in `parity/manifest.toml`
whose id contains the eight class names. 37 rows were deferred before this round; 29 remain for
other lanes. `parity/` was not edited (out of scope), so this report also proposes the mapping
in §6 for the lane that owns the ledger.

## 1. The eight ids and their outcomes

| # | Census id (from parity/manifest.toml) | Class | Outcome |
|---:|---|---|---|
| 1 | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0` | IntegerConstructor | Fixed (From impls) |
| 2 | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0` | IntegerEquality | Already matched (PartialEq); test added |
| 3 | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0` | IntegerMultiply | Fixed (Mul impls) |
| 4 | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0` | IntegerSubtract | Fixed (Sub impls) |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | StructuredRealExactness | Fixed (terminating-quotient fast path) |
| 6 | `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive#0` | RealTrigFastPath | Fixed (Taylor loop cost) |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ArrayAllocationRefusalTests/TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | ArrayAllocationRefusal | Already matched (AllocationRefused / BudgetExceeded); test added |
| 8 | `Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MatMul_Batched#0` | NdArray | Fixed (batched matmul) |

## 2. Per-row before/after

### Row 1 — Constructor_GivenInt_DelegatesToLongOverload

Oracle row (`Lovelace.Integer.Tests/IntegerConstructorTests.cs:69`): `new Integer(-7)` and
`new Integer(-7L)` must format identically and agree on sign.

- **Before (deferred)**: Rust offered only `Int::from_i64`; there was no `From<i32>` conversion,
  so the "int delegates to long" contract had no Rust API to assert. Library-only row.
- **Fix**: `impl From<i32> for Int` delegates to `from_i64(i64::from(value))`;
  `impl From<i64> for Int` exposes the long overload itself.
- **After**: `Int::from(-7i32) == Int::from(-7i64)`, both `"-7"`, both negative.
- **Probe**: the language turns `-7` into `Integer -7` identically in both runners (exit 0).

### Row 2 — Equals_GivenReferenceToSelf_ReturnsTrue

Oracle row (`IntegerEqualityTests.cs:38`): `x.Equals(x)` is true.

- **Before (deferred)**: no assertion existed, but the capability already did — `Int` derives
  `PartialEq`/`Eq`, so `x.eq(&x)` is true. This row already matches; no production change was
  invented for it.
- **After**: regression test pins `reference.eq(reference)` for a real reference.

### Row 3 — OperatorStar_ProducesSameResultAsMultiplyMethod

Oracle row (`IntegerMultiplyTests.cs:141`): `a.Multiply(b)` and `a * b` are the same product
for `6 * -7`.

- **Before (deferred)**: only `Int::mul` existed; there was no `Mul` operator implementation.
- **Fix**: `impl Mul for Int` and `impl Mul<&Int> for &Int`, both delegating to `Int::mul`.
- **After**: `&a * &b == a.mul(&b) == -42`.
- **Probe**: `6 * -7` answers `Integer -42` (exact) in both runners.

### Row 4 — SubtractOperator_IsEquivalentToSubtractMethod

Oracle row (`IntegerSubtractTests.cs:118`): `a.Subtract(b)` and `a - b` are the same
difference for `100 - 37`.

- **Before (deferred)**: only `Int::sub` existed; there was no `Sub` operator implementation.
- **Fix**: `impl Sub for Int` and `impl Sub<&Int> for &Int`, both delegating to `Int::sub`.
- **After**: `&a - &b == a.sub(&b) == 63`.
- **Probe**: `100 - 37` answers `Natural 63` (exact) in both runners.

### Row 5 — TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2 (`2^-100000`)

Oracle row (`StructuredRealExactnessTests.cs:139`): `2^-100000` is certified exact with
numerator `1`, denominator `2^100000`, and a value starting `0.0000`.

- **Before**: the pre-fix release runner ran for **825,354 ms and then aborted with exit `-1`
  and no envelope** (measured this round; the row's round-45 note said "exit -1"). Cause: the
  denominator's termination was decided by dividing out 2 one factor at a time, and the digit
  loop then charged 100,000 leading places to the budget.
- **Fix**: `fraction_exact` now calls the reference's `TryTerminateExactly` decomposition before
  the digit loop (`terminate_exactly`, with the reference's `2^62/2^31/...` and
  `5^27/5^13/...` factor ladders) and returns `magnitude * 10^-scale` normalized. This makes the
  quotient exact at any budget without walking 100,000 digits or 100,000 factors.
- **After (Rust)**: **725 ms** on the first post-fix run and **358 ms** on the final binary, exit 0, `ok:true`, `kind:"Real"`, `exact:true`,
  numerator `1` (1 char), denominator 30,103 digits, value 100,002 chars starting `0.0000`.
- **After (oracle)**: 1,002 ms, exit 0, the same `exact:true`, numerator `1`, the same 30,103-digit
  denominator (prefix `999002093014`, suffix `389883109376`), the same 100,002-char value.
  The compared fields are identical: `ok`, `kind`, `exact`, numerator, the full 30,103-digit
  denominator, the full 100,002-char value and the 102-char display all match exactly.

### Row 6 — Cos_AtDefaultPrecision_StaysInteractive

Oracle row (`RealTrigFastPathTests.cs:197`): at `WithPrecision(1000, 40)`, one `Real.Cos(0.5)`
after a warmup must take less than **10,000 ms** and must not be periodic.

- **Before**: `evalf(cos(0.5), 1000)` took **32,275 ms**; an instrumented release unit probe found
  204 Taylor iterations costing 6.6 s in `div_truncate`, 6.7 s in the termination test and 0.37 s
  in additions. Root causes, all three present:
  1. `div_truncate` kept the quotient's trailing zeros although the reference's
     `DivideNonPeriodic` normalizes; each term stored `frac_digits` more zeros, so the term
     magnitude grew to ~200,000 digits while its exponent fell.
  2. `below_decimal_guard` compared against `10^-(guard+1)` through the exact rational path
     (`Real::cmp`), rendering/parsing 200,000-digit magnitudes every iteration.
  3. `Real::normalized` stripped trailing zeros one division by ten at a time.
- **Fix**: `div_truncate` normalizes like the reference; the Taylor loops use the already-ported
  N23 `DecimalGuard` integer test (carried power of ten, no decimal rendering); `normalized` uses
  the reference's galloping `StripTrailingDecimalZeros`; `Int::from_nat` avoids a
  decimal render/parse round trip in every magnitude-to-integer conversion.
- **After**: unit probe `Real::cos(0.5)` at 1000 places **0.81 s**; `evalf(cos(0.5), 1000)` CLI
  **4,158 ms** on the first post-fix run and **3,654 ms** on the final binary (oracle CLI:
  15,769 ms). The published 1000-place value is byte-identical to the oracle's (1002 chars,
  `exact:false`), and the 10 s assertion passes in both release and debug test builds. No
  assertion was relaxed.

### Row 7 — TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies

Oracle row (`ArrayAllocationRefusalTests.cs:45`): the suite declares a refusal the runner
classifies; the C# test only asserts the type exists.

- **Before**: already matched. The Rust runner declares the refusal as the published
  `AllocationRefused` / `BudgetExceeded` error pair and classifies it before allocating; this is
  the Rust surface for the reference's exception type. No production change was invented.
- **Probe**: `zeros(1000000000)` on both runners: exit 1, `ok:false`,
  `code:"AllocationRefused"`, `category:"BudgetExceeded"`, `recoverable:true`, identical
  message (1,000,000,000 > 268,435,456). 
- **After**: dedicated declaration/classification regression test added.

### Row 8 — MatMul_Batched

Oracle row (`NdArrayTests.cs:296`): batched rank-3 `ArrayMath.MatMul` of the same array gives
shape `[2,2,2]` and data `7, 10, 15, 22, 67, 78, 91, 106`.

- **Before**: `matmul` refused rank-3 operands: Rust exit 1, `DomainError/InvalidOperation`,
  "Expected a rank-2 matrix."; oracle exit 0 with the batched array.
- **Fix**: `matmul` dispatches rank >= 2 operands with equal leading (batch) dimensions to
  `matmul_batched`, which multiplies each matching 2-D slice with the existing exact kernel and
  rebuilds the `batch + [m, n]` shape. Rank-2 and vector behaviour is untouched.
- **After**: Rust exit 0, `display: "[[[7, 10], [15, 22]], [[67, 78], [91, 106]]]"`,
  `kind:"Array"`, `shape:[2,2,2]`, elements Natural exact `7,10,15,22,67,78,91,106` — identical
  to the oracle envelope. 261 µs.

## 3. Production changes (by file)

| File | Change |
|---|---|
| `rust/lovelace-int/src/lib.rs` | `Int::from_nat` (lossless magnitude constructor); `From<i32>` (delegates to the long overload), `From<i64>`; `Mul`, `Mul<&Int>`, `Sub`, `Sub<&Int>` delegating to `mul`/`sub`. |
| `rust/lovelace-real/src/lib.rs` | `int_from_nat` is now lossless (`Int::from_nat`); `normalized` uses the galloping trailing-zero strip; `terminate_exactly` implements the reference's `TryTerminateExactly` with factor ladders; `from_fraction` answers a terminating quotient exactly before the digit loop. |
| `rust/lovelace-real/src/transcendental.rs` | `div_truncate` normalizes its quotient (reference `DivideNonPeriodic`); `sin_taylor_guarded`/`cos_taylor_guarded` use `fast_path::DecimalGuard`; `try_terminate_exactly` delegates to the shared helper; `int_from_mag` is lossless. |
| `rust/lovelace-lang/src/builtins.rs` | `matmul` gained the rank->=2 batched branch and `matmul_batched`. This row's runner surface lives in lovelace-lang; the alternative was to leave the measured exit-code divergence in place. |
| `rust/lovelace-array/src` | **No production change.** `NdArray` has no `ArrayMath` surface; the batched kernel lives at the language boundary. |

## 4. Tests added (one per row, eight total)

| Row | Test |
|---|---|
| 1 | `rust/lovelace-int/src/round95_singletons.rs:9 constructor_given_int_delegates_to_long_overload` |
| 2 | `rust/lovelace-int/src/round95_singletons.rs:19 equals_given_reference_to_self_returns_true` |
| 3 | `rust/lovelace-int/src/round95_singletons.rs:29 operator_star_produces_same_result_as_multiply_method` |
| 4 | `rust/lovelace-int/src/round95_singletons.rs:41 subtract_operator_is_equivalent_to_subtract_method` |
| 5 | `rust/lovelace-real/src/round95_singletons.rs:12 terminating_quotient_beyond_the_digit_budget_is_certified_exact` |
| 6 | `rust/lovelace-real/src/round95_singletons.rs:32 cos_at_default_precision_stays_interactive` |
| 7 | `rust/lovelace-lang/src/round95_singletons.rs:30 suite_declares_the_allocation_refusal_the_runner_classifies` |
| 8 | `rust/lovelace-lang/src/round95_singletons.rs:17 matmul_batched_multiplies_each_three_dimensional_slice` |

The row 6 test asserts the row's own 10 s bound; the row 5 test asserts exactness, numerator,
denominator and the `0.0000` prefix; the row 8 test asserts the oracle's published display. No
expectation was changed to match the implementation.

## 5. Gate outputs

| Gate | Result |
|---|---|
| all rows below | re-run on the final release binary after the zero-canonicalisation fix |
| `cargo test --workspace` | **BLOCKED by lovelace-desktop** (concurrent desktop lane; 5 compile errors in `src/ui/app.rs`/`src/win32/ffi.rs`, files already dirty before this round). Not caused by these crates. |
| `cargo test --workspace --exclude lovelace-desktop` | exit 0: **5,293 passed, 0 failed, 1 ignored** across the non-desktop crates (includes this round's 8 new tests and the concurrent engine-singletons lane's additions) |
| `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` (exit 0) |
| `parity/gate-fixtures.ps1` | `PASS: oracle 20/20, rust 20/20` |
| `parity/gate-cli.ps1` | `CLI matrix: 16/16` — PASS |
| `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200/200** match, replayed **1758/1758** match, 0 mismatches, 0 errors, 116 duplicates skipped (exit 0; final-binary run `20261112T195911Z-seed1`) |
| `node parity/diff/run.mjs --regressions` | **1873/1873** replay match, 0 mismatches, 0 errors (exit 0; final-binary run `20261112T200017Z-seed1`) |
| `cargo clippy -p lovelace-int -p lovelace-real -p lovelace-array --all-targets -- -D warnings` | clean (exit 0) |
| `cargo clippy -p lovelace-lang --all-targets -- -D warnings` | blocked by a concurrent lane's hunk, `builtins.rs:830` `manual_checked_ops` (broadcast work); no finding is in this round's hunks. |
| `rustfmt --check` on every file this round changed or added | clean (`lovelace-int/src/lib.rs`, both `round95_singletons.rs` test files per crate, `lovelace-real/src/{lib,transcendental}.rs`, and the two new lines in `lovelace-lang/src/lib.rs`; `builtins.rs` outside this round's hunks is left to its owner) |
| `cargo fmt --all -- --check` | blocked by the desktop lane and the concurrent lang hunks; this round's files are rustfmt-clean. |

## 6. Proposed mapping (parity/ was not edited)

| Census id | Rust test |
|---|---|
| `.../Constructor_GivenInt_DelegatesToLongOverload#0` | `lovelace-int::round95_singletons::constructor_given_int_delegates_to_long_overload` |
| `.../Equals_GivenReferenceToSelf_ReturnsTrue#0` | `lovelace-int::round95_singletons::equals_given_reference_to_self_returns_true` |
| `.../OperatorStar_ProducesSameResultAsMultiplyMethod#0` | `lovelace-int::round95_singletons::operator_star_produces_same_result_as_multiply_method` |
| `.../SubtractOperator_IsEquivalentToSubtractMethod#0` | `lovelace-int::round95_singletons::subtract_operator_is_equivalent_to_subtract_method` |
| `.../TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | `lovelace-real::round95_singletons::terminating_quotient_beyond_the_digit_budget_is_certified_exact` |
| `.../Cos_AtDefaultPrecision_StaysInteractive#0` | `lovelace-real::round95_singletons::cos_at_default_precision_stays_interactive` |
| `.../TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | `lovelace-lang::round95_singletons::suite_declares_the_allocation_refusal_the_runner_classifies` |
| `.../MatMul_Batched#0` | `lovelace-lang::round95_singletons::matmul_batched_multiplies_each_three_dimensional_slice` |

## 7. Notes and honest limits

- The row-6 fix is a performance fix, not an assertion change: the 10 s bound is unchanged and
  passes with the value still non-periodic.
- The row-5 before measurement (825 s then exit -1) is from the pre-fix release binary; the row's
  own round-45 reason had recorded the abort without the duration.
- Two of the eight rows needed no production change (2 and 7); they are mapped, not re-implemented.
- The round caught one regression of its own before the final gates: canonicalising a zero's
  exponent in `normalized` was needed so `evalf(1/(3*10^1000), 30)` and `0.0 % 5.0` render `0`
  (the reference's rendering), not `0.000…`; the fast-path `Normalize` shim keeps the reference's
  zero-unchanged rule because `normalize_binary_strip_matches_the_digit_string_strip` pins it.
- `parity/manifest.toml` is untouched (prohibited by this lane's scope), so the ledger still says
  `deferred` for all eight until the mapping lane applies §6.
