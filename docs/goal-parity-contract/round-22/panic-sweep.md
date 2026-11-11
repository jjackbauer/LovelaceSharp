# Round 22 — builtin panic sweep

Scope: production changes only in `rust/lovelace-lang/src/builtins.rs` (plus the test-module
registration in `lib.rs` and the new test file), tests, and this report. No `.cs`, fixture,
corpus source, or `parity/` source file was edited; the differential gate wrote its usual run
directory under `parity/diff/runs/`, and no other document was changed.

## 1. Result

- The two null-fate boundary cells are fixed: `eye(9223372036854775808)` and
  `1..9223372036854775808` now return the oracle's exact `InvalidArgument`/`TypeMismatch`
  refusal instead of a panic (cell #31) or a multi-second `DomainError` (cell #316).
- The sweep found and fixed **seven additional reachable panic sites** in the same class
  (`zeros`/`ones` allocation-shape overflow, `eye`'s `n * n`, two slice arithmetic
  overflows, and `len` on a rank-0 shape) and **two silent numeric-cast defects** (`pow`'s
  Real exponent and `series`' order) that produced wrong values instead of the oracle's
  typed refusal. The same fix also corrects `squeeze`'s rank-1 result to the oracle's
  Vector kind.
- Added **16 regression tests** in `rust/lovelace-lang/src/panic_sweep_round22.rs`.
- Gates: workspace **4,059 passed / 0 failed** (was 4,043; +16), corpus **212/214**
  (unchanged known `sym/071`, `sym/076`), fixtures **20/20 oracle + 20/20 rust**, CLI
  **16/16**, differential generated delta **0** (154 match / 46 mismatch / 0 error, identical
  to the previous run; regression replay identical, `new=0`).

## 2. Reproduction (before the fix)

All probes were run from the repository root with the workspace toolchain and
`--omit-functions --omit-variables` where the envelope was compared.

### 2.1 Cell #31 — `eye(rows = 2^63)`

Debug runner (`rust/target/debug/lovelace-run.exe --eval "eye(9223372036854775808)"`):

```text
thread 'lovelace-run' (18444) panicked at lovelace-lang\src\builtins.rs:2309:47:
attempt to multiply with overflow
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
exit code: 101
```

The release runner does not panic on the multiplication; the allocation path aborts the
process instead:

```text
memory allocation of 55834574848 bytes failed
exit code: -1073740791 (0xC0000409)
```

Both are process failures, not diagnostics. The C# cell forbids an internal failure.

### 2.2 Cell #316 — `1..9223372036854775808`

The round-21 report attributed this cell to the same `builtins.rs:2309` panic. That is **not
reproducible in the round-22 tree**: the bound crossed as a `Real` and the range loop ran to
its one-million-element cap. Observed before the fix:

```json
{"ok":false,"code":"InvalidOperation","category":"DomainError",
 "message":"Range is too large."}
exit code: 1, elapsed 3,415.19 ms (release) / 13,804.14 ms (debug)
```

The oracle refuses immediately with `InvalidArgument`/`TypeMismatch`; the cell is still a
defect (wrong code/category, wrong message, and a multi-second wait), just not a panic. It is
fixed to the oracle record below.

### 2.3 Additional panic sites the sweep reproduced

| Script | Before: process outcome | stderr site |
|---|---|---|
| `zeros(9223372036854775808)` | exit 101 | `alloc/raw_vec/mod.rs:28:5: capacity overflow` |
| `zeros(9223372036854775807)` | exit 101 | `alloc/raw_vec/mod.rs:28:5: capacity overflow` |
| `zeros(2, 9223372036854775808)` | exit 101 | `core/iter/traits/accum.rs:206:1: attempt to multiply with overflow` |
| `zeros(4294967296, 4294967296)` | exit 101 | `core/iter/traits/accum.rs:206:1: attempt to multiply with overflow` |
| `ones(18446744073709551615)` | exit 101 | `alloc/raw_vec/mod.rs:28:5: capacity overflow` |
| `eye(4294967296)` | exit 101 | `builtins.rs:2309:47: attempt to multiply with overflow` |
| `eye(9223372036854775807)` | exit 101 | `builtins.rs:2309:47: attempt to multiply with overflow` |
| `[1, 2, 3][9223372036854775806:9223372036854775807:2]` | exit 101 | `builtins.rs:889:17: attempt to add with overflow` |
| `[1, 2, 3][-9223372036854775808:1]` | exit 101 | `builtins.rs:848:41: attempt to negate with overflow` |
| `[1, 2, 3][0:-9223372036854775808]` | exit 101 | `builtins.rs:848:41: attempt to negate with overflow` |
| `len(squeeze([[[1]]]))` | exit 101 | `builtins.rs:2247:68` (index out of bounds: shape[0] of a rank-0 array) |

Several inputs were also silent wrong answers rather than panics: `zeros(-1)` built `[0]`,
`eye(-1)` built `[[1]]`, `eye(0)` built `[]`, and `series(sin(x), x, 0, 2^63)` printed
`O(x^-9223372036854775808)` because the order wrapped at the `as i64` cast.

## 3. Oracle contract and fix

The oracle messages below are the pinned runner's (`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`).
The fixes reproduce them byte for byte.

| Cell / input | Oracle code / category | Oracle message |
|---|---|---|
| `eye(2^63)` | `InvalidArgument` / `TypeMismatch` | `eye(): argument 1 (rows) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 9223372036854775808.` |
| `eye(4294967296)` | `AllocationRefused` / `BudgetExceeded` | `eye() requested one array of shape [4294967296, 4294967296], which holds more than 9223372036854775807 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces.` |
| `eye(16385)` | `AllocationRefused` / `BudgetExceeded` | same shape message with `[16385, 16385]` and `268468225 element(s)` |
| `zeros(2^63)` | `InvalidArgument` / `TypeMismatch` | `zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got 9223372036854775808.` |
| `zeros(2, 2^63)` | `InvalidArgument` / `TypeMismatch` | `zeros(): argument 2 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got 9223372036854775808.` |
| `zeros(9223372036854775807)` | `AllocationRefused` / `BudgetExceeded` | `zeros() requested one array of shape [9223372036854775807], which holds 9223372036854775807 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces.` |
| `zeros(4294967296, 4294967296)` | `AllocationRefused` / `BudgetExceeded` | same shape message with `[4294967296, 4294967296]` and `more than 9223372036854775807 element(s)` |
| `zeros(-1)` | `InvalidArgument` / `TypeMismatch` | `zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.` |
| `zeros(2.5)` | `InvalidArgument` / `TypeMismatch` | `zeros(): argument 1 (a dimension) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.` |
| `zeros(2.0)` | `InvalidArgument` / `TypeMismatch` | `zeros(): argument 1 (a dimension) must be a Natural or Integer; got the Real 2, which is integral — write it as 2.` |
| `1..9223372036854775808` | `InvalidArgument` / `TypeMismatch` | `range(): argument 'bound' must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 9223372036854775808.` |
| `1..9223372036854775808..3` | `InvalidArgument` / `TypeMismatch` | `range(): argument 'step' must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 9223372036854775808.` |
| `[1,2,3][0:9223372036854775808]` | `InvalidArgument` / `TypeMismatch` | the same `range(): argument 'bound' ...` message |
| `[1,2,3][0:1:9223372036854775808]` | `InvalidArgument` / `TypeMismatch` | the same `range(): argument 'step' ...` message |
| `2.0^9223372036854775808` | `ArithmeticError` / `DomainError` | `Value was either too large or too small for an Int64.` |
| `series(sin(x), x, 0, 9223372036854775808)` | `InvalidArgument` / `TypeMismatch` | `series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Natural 9223372036854775808.` |
| `series(sin(x), x, 0, 2147483648)` | `InvalidArgument` / `TypeMismatch` | `series(): argument 4 (order) must be a Natural or Integer between 1 and 2147483647; got 2147483648.` |

### 3.1 Implementation

`rust/lovelace-lang/src/builtins.rs`:

- `exact_integer_argument` (line 2784) implements the reference's `ExactIntegerArgument`
  contract for array dimensions and `eye` rows: Natural/Integer inside the declared bound;
  a Real that is integral gets the write-it-as-an-integer message, a fractional Real or other
  kind gets the denotes-an-exact-integer message, and a Natural/Integer wider than `i64` gets
  the named bound refusal. `CountBounds` supplies the reference's `Int64Range.Describe()`
  prose verbatim.
- `ensure_allocation` (line 2870) and `allocation_refusal` (line 2847) implement
  `ArrayAllocationBudget`: `MAX_SINGLE_ALLOCATION_ELEMENTS = 1 << 28`; the shape product is
  measured in `u128` with saturating multiplication (never overflows), saturated to
  `i64::MAX` when the true product is wider, and refused before any buffer exists with
  `AllocationRefused`/`BudgetExceeded` and the reference's sentence. Zero dimensions keep
  the product zero (the D5 contract).
- `eye` (line 2372) reads rows through the positive-count contract and measures `[n, n]`
  through the budget before `Vec::with_capacity`; the old `n * n` capacity expression is
  gone.
- `dims_from` (line 2935) now takes the builtin name and returns contract-checked `u64`
  dimensions; `zeros`/`ones` (line 2367) and `reshape` use it, so a negative dimension is
  refused instead of being read as its magnitude, and a fractional Real is refused instead of
  reaching the allocator. `array_of` (line 2945) budgets the shape before filling it.
- `make_range` (line 788) refuses an exact-integer bound/step wider than `i64` by name before
  the progression starts, using `range_component_refusal` (line 764).
- `slice_integer` (line 899) parses a slice bound/step through the same `i64` contract;
  `slice_indices` (line 906) clips both bounds to `[0, len]` with saturating adds and advances
  with `checked_add`, so `i64::MIN` bounds and a step that would overflow end the slice instead
  of panicking.
- `pow` (line 472) converts the exponent with `i64::try_from`, publishing the reference's
  `ArithmeticError`/`DomainError` record instead of wrapping to `i64::MIN`.
- `series_order_argument` (line 2892) reproduces the Symbolics plugin's own order contract
  (`1..=int.MaxValue`, wider integers are the plugin's denotes-an-exact-integer refusal).
- `power_branch_outcome` (line 5540) rejects non-positive copy counts and uses
  `checked_mul` plus the allocation budget for its result vector (defensive: the public call
  site already restricts copies to 2..=16).
- `squeeze` (line 2430) wraps a squeezed rank <= 1 as a `Value::Vector`, matching the
  reference's "collapse to rank 1, never rank 0" rule; `len` (line 2247) reads `shape[0]`
  through `first()` with the scalar fallback 1 instead of indexing an empty shape.

No error-type change was needed: `LangError::coded` already carries the `AllocationRefused`
and `ArithmeticError` codes the runner publishes.

## 4. Before/after probes

| Script | Rust before | Rust after (and oracle) |
|---|---|---|
| `eye(9223372036854775808)` | exit 101, `attempt to multiply with overflow` | exit 1, `InvalidArgument`/`TypeMismatch`, exact message above |
| `eye(4294967296)` | exit 101, `attempt to multiply with overflow` | exit 1, `AllocationRefused`/`BudgetExceeded`, exact message |
| `eye(0)` | exit 0, `[]` | exit 1, positive-count refusal (oracle) |
| `eye(-1)` | exit 0, `[[1]]` | exit 1, positive-count refusal (oracle) |
| `zeros(9223372036854775808)` | exit 101, `capacity overflow` | exit 1, dimension refusal |
| `zeros(9223372036854775807)` | exit 101, `capacity overflow` | exit 1, `AllocationRefused` |
| `zeros(2, 9223372036854775808)` | exit 101, `attempt to multiply with overflow` | exit 1, dimension refusal |
| `zeros(4294967296, 4294967296)` | exit 101, `attempt to multiply with overflow` | exit 1, `AllocationRefused` |
| `ones(18446744073709551615)` | exit 101, `capacity overflow` | exit 1, dimension refusal |
| `1..9223372036854775808` | exit 1, `DomainError`, `Range is too large.` (3.4 s) | exit 1, `InvalidArgument`, exact message (0.06 ms) |
| `[1,2,3][-9223372036854775808:1]` | exit 101, `attempt to negate with overflow` | exit 0, `[1]` (oracle: `[1]`) |
| `[1,2,3][0:-9223372036854775808]` | exit 101, `attempt to negate with overflow` | exit 0, `[]` (oracle: `[]`) |
| `[1,2,3][9223372036854775806:9223372036854775807:2]` | exit 101, `attempt to add with overflow` | exit 0, `[]` (oracle: `[]`) |
| `[1,2,3][0:1:9223372036854775807]` | exit 0, `[1]` | exit 0, `[1]` (oracle: `[1]`) |
| `2.0^9223372036854775808` | code inspection: `e_u64 as i64` wraps the exponent (not probed before the fix) | exit 1, `ArithmeticError`, exact message |
| `series(sin(x), x, 0, 9223372036854775808)` | exit 0, `O(x^-9223372036854775808)` | exit 1, `InvalidArgument`, exact message |
| `len(squeeze([[[1]]]))` | exit 101, shape index out of bounds | exit 0, `1` (oracle: `1`) |
| `squeeze([[[1, 2]]])` | exit 0, `[1, 2] (Array)` | exit 0, `[1, 2] (Vector)` (oracle: `Vector`) |
| `zeros(2, 3)` | exit 0 | exit 0, `[[0, 0, 0], [0, 0, 0]]` unchanged |
| `eye(3)` | exit 0 | exit 0, identity matrix unchanged |
| `1..5` | exit 0 | exit 0, `[1, 2, 3, 4, 5]` unchanged |

`1..10^30` is parsed as `(1..10)^30` by this grammar, so it is a vector power and is not the
wide-bound case; the wide-bound probes use the decimal literal exactly as the matrix does.

## 5. Audited sites

48 sites were audited across the four panic classes. `fixed` means a production change in this
round; `safe` means the invariant that prevents the panic was checked and is documented here.

### 5.1 Direct indexing / non-empty assumptions

| Site (function, current line) | Verdict |
|---|---|
| `index_int` / `slice_indices` `items[idx]`, `slice_indices_or_single` (861, 906) | fixed by `slice_integer` + clipping; `index_int` already bounds-checks |
| `index` multi-axis rebuild `per_axis[axis][idx]` (~1010) | safe: each axis was validated by `slice_indices_or_single` first |
| `reduce` all-elements `flat[0]` (~2968) | safe: empty array is handled before the index |
| `reduce` axis `shape[0]`/`shape[1]` (3036) | hardened: the rank check now precedes both indexes (the old order read `shape[1]` before asking the rank); rank-1 values are `Value::Vector` |
| `det` `m[0][0]`, `m[1][1]` (~3070) | safe: `n == 1` / `n == 2` arms precede them |
| `linsolve` `rhs[r]` (~1936) | safe: `rhs.len() == n` is checked |
| `matmul` `a_mat[r][k]`, `b_mat[k][c]` (~2844) | safe: both matrices are built from their validated shapes |
| `conv` `out[i + j]` (~2321) | safe: both input vectors are non-empty and the output has `a.len()+b.len()-1` slots |
| `dft` `roots[(k * t) % n]` (~4354) | safe: `n > 0`, roots has `n` entries, and `n` is an allocated vector length |
| `plot` `points[0]` (~1427) | safe: guarded by `points.len() == 1` |
| `DspRandom` `seed_array[...]` (~3912) | safe: the array is `[i32; 56]` and every index is 1..=55 |
| `dsp_delay` `values[index]` (~3734) | safe: index is range-checked before use |
| `len` `a.shape()[0]` (2247) | fixed: `first().copied().unwrap_or(1)`; `squeeze` no longer produces rank 0 |
| `squeeze` rank collapse (2430) | fixed: rank <= 1 is wrapped as `Value::Vector`, matching the oracle (the deferred 1x1x2 `squeeze` divergence is resolved) |

### 5.2 unwrap/expect on user-reachable Option/Result

| Site | Verdict |
|---|---|
| `div` / `rem` `div_rem_signed(...).expect("nonzero divisor")` (259, 338) | safe: the zero divisor is refused first |
| `e_constant` `div_rem(&divisor).expect(...)` (2639, 2652) | safe: divisors are `k >= 1` and `10^12` |
| `evalf` `as_int(...).expect(...)` (~2281) | safe by the enclosing Natural/Integer match; the conversion is total for those kinds |
| `digit_count_bounded` / `precision_digit_count` / `evalf_digit_count` expects (~2535, 2557, 2607) | same invariant: only Natural/Integer reach the expect |
| `dsp_to_real` / `dsp_to_complex` `as_real(...).expect(...)` (~3424) | safe by the numeric-kind match |
| `Int::from_decimal(&nat.to_decimal()).expect(...)` (~3980, 4115, 4122) | safe: every Nat decimal is a valid Int |
| `divide_var` / `quotient_parts` `pop().unwrap()` (~4853, ~6078) | safe: both are reached only after `len() == 1` |
| `Real::from_str("0.5").expect(...)` (~4144, ~4424) | safe: a literal constant |
| `Real::from_str(&assembled).unwrap_or_else(Real::zero)` (~2605) | safe: fallible but not panicking |

### 5.3 Division by zero

| Site | Verdict |
|---|---|
| `div` / `rem` (~246, ~350) | safe: zero divisor returns `divide_by_zero()` |
| `inv`, `divrem`, `evalir` coefficient division (~481, ~1139, ~1469) | safe: `None` maps to `divide_by_zero()`; denominators are checked |
| `mean` via `numeric_total` (~2740) | safe: empty array refused before division; count >= 1 |
| `dsp_movingavg` divisor (~3488) | safe: `w` is a positive count |
| `roots_of_unity` `gcd` / `(n / g)` (~4394) | safe: `g >= 1` and `n > 0` |
| `gcd_usize` loop `%` (~4414) | safe: loop condition is `b != 0` |
| plot `(x - xmin) / xr` (~3224) | safe: `xr`/`yr` are replaced by 1.0 when the span is zero |

### 5.4 Numeric casts and unchecked arithmetic

| Site | Verdict |
|---|---|
| `eye` `n * n` (old 2309) | fixed: shape measured in `u128` before allocation |
| `array_of` `dims.iter().product::<usize>()` (old 2749) | fixed: `ensure_allocation` with saturating `u128` product |
| `filled` `Vec::with_capacity(total)` (old 2702) | fixed: `total` is budgeted to <= 2^28 before this call |
| `slice_indices` `-n`, `start += len`, `i += step` (old 848/877/889) | fixed: no negate; saturating adds; `checked_add` |
| `pow` `e_u64 as i64` (old 469) | fixed: `i64::try_from` with the oracle's `ArithmeticError` |
| `series` order `as i64` (old 1580) | fixed: `series_order_argument` |
| `power_branch_outcome` `copies as usize`, `roots.len() * copies` (old 5506) | fixed defensively: non-positive copies rejected, `checked_mul`, budget check |
| `index_int` `n as usize` | safe: `n < len` is checked before the cast |
| `dsp_sample_count` `n as usize` | safe: `n` is refused above `int.MaxValue` and is non-negative |
| digit counts `magnitude as usize` | safe: bounded by `1..=2^31-1` before the cast |
| `reduce` axis `as usize` | safe: compared with `shape.len()` before use |
| `dims_from` dimension `as u64` / `as usize` | safe: contract-bounded to `0..=i64::MAX` |
| `roots_of_unity` `(twice / g) as i64`, `(n / g) as i64` | safe: `n` is an allocated vector length |
| `DspRandom::next_digit` `(sample * 10.0) as u8` | safe: Rust float-to-int casts saturate |
| DSP loop indices `k as i64` | safe: `n <= int.MaxValue` |
| `horner_form` `lowest as i64` | safe: `lowest` is a coefficient index |
| plot constants `width as i64` etc. | safe: compile-time f64 constants |
| multiplicity/count `as i64` | safe: multiplicities are positive and bounded by the polynomial degree |

## 6. Tests

`rust/lovelace-lang/src/panic_sweep_round22.rs`, 16 tests, one for each fixed site group:

1. `eye_rows_beyond_i64_is_the_oracles_argument_refusal`
2. `eye_shape_over_the_allocation_budget_is_refused_before_the_allocator`
3. `eye_zero_and_negative_rows_are_positive_count_refusals_not_magnitudes`
4. `array_dimensions_beyond_i64_are_argument_refusals`
5. `array_shapes_over_the_allocation_budget_are_refused_before_the_allocator`
6. `a_negative_dimension_is_refused_instead_of_read_as_its_magnitude`
7. `a_zero_dimension_keeps_the_shape_constructible`
8. `a_range_bound_beyond_i64_is_refused_by_name`
9. `a_range_step_beyond_i64_is_refused_by_name`
10. `slice_bounds_at_i64_min_are_clipped_instead_of_overflowing`
11. `a_slice_step_that_would_overflow_ends_the_slice_at_the_last_index`
12. `slice_bounds_beyond_i64_are_refused_by_name`
13. `reshape_dimensions_use_the_same_exact_integer_contract`
14. `a_series_order_beyond_i64_is_the_oracles_argument_refusal`
15. `a_real_exponent_beyond_i64_is_the_oracles_arithmetic_refusal`
16. `squeeze_of_singletons_is_a_rank_one_vector_and_len_does_not_panic`

Every refusal test asserts the oracle's exact `code`, `category` and `message`; the value
tests assert the oracle's exact display. The module is registered in `lib.rs` next to the
other round modules.

## 7. Gates

| Gate | Observed |
|---|---|
| `cargo test --workspace` | **4,059 passed, 0 failed** (4,043 + 16; log `rust/target/tmp/round22-cargo-test-final3.log`) |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** — only the known `sym/071` and `sym/076` |
| `node parity/tools/gate-fixtures.mjs` | oracle **20/20**, rust **20/20**, PASS |
| `node parity/tools/gate-cli.mjs` | CLI matrix **16/16**, PASS |
| `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated **154 match / 46 mismatch / 0 error** (delta **0** against the previous run `20261111T002135Z-seed1`); replay **600 match / 1152 mismatch / 0 error**, `new=0`; harness exit 1 because the baseline is red, not because of a new mismatch |

The differential run directory is `parity/diff/runs/20261111T002503Z-seed1`; the prior run
used for the delta is `parity/diff/runs/20261111T002135Z-seed1`, and both manifests report
identical counts.

## 8. What remains

1. **Panic outside the authorized production scope.** The sweep found one more reachable panic:
   `solve_full((x^2 - 1)^9223372036854775807 == 0, x)` panics at
   `rust/lovelace-symbolics/src/lib.rs:497` (`inner.degree_in(name) * exponent`, attempt to
   multiply with overflow). It is a one-line `saturating_mul` fix in the symbolics crate, which
   this round's scope excludes; the oracle answers the same input with a solved record.
2. **Zero-width huge shape display.** `zeros(9223372036854775807, 0)` constructs (zero
   elements; `numel` answers 0) but did not return within 8 s when the array itself was the
   result, so the display path is the suspected cost. The oracle process fails on the same
   input without an envelope (exit -532462766), so this is not a parity target yet; a defensive
   display/shape guard is a separate change.
3. **Manifest bookkeeping.** Cells #31 and #316 remain `deferred` in
   `parity/manifest-map.json`; `parity/` is out of scope for this round, so the manifest owner
   must re-run `node parity/gen-manifest.mjs` (and the gate) after accepting this evidence.
4. **Pre-existing range divergences not addressed.** A Real range bound (`1..2.5`) is still
   accepted by Rust and refused by the oracle, and `eye(2, 3)` still refuses the second
   dimension. Both predate this round and are not panic defects.

## 9. Reproduction commands

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
cd $repo\rust
cargo test --workspace
cargo build --release -p lovelace-run -p lovelace-corpus
cargo run --release -p lovelace-corpus
cd $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
# the before/after probes:
.\rust\target\debug\lovelace-run.exe --eval 'eye(9223372036854775808)' --json --omit-functions --omit-variables
.\rust\target\debug\lovelace-run.exe --eval '1..9223372036854775808' --json --omit-functions --omit-variables
```