# Round 23 — symbolic-kernel panic sweep and display bound

Status: **fixed in tree; gates re-run**. Scope: `rust/lovelace-symbolics/src`,
`rust/lovelace-lang/src` (display/allocation guards), and tests. No `.cs`, fixture, or
`parity/` source/configuration was edited; the differential harness wrote only its run
directory. No documentation outside this report was edited.

## 1. Reproductions (before the fix)

Environment:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
```

### 1.1 Symbolic degree overflow

```powershell
.\rust\target\debug\lovelace-run.exe --eval 'x = symbol(\"x\"); solve_full((x^2 - 1)^9223372036854775807 == 0, x)' --omit-functions
```

Observed (before):

```text
exit 1
thread 'lovelace-run' (23808) panicked at lovelace-symbolics\src\lib.rs:497:21:
attempt to multiply with overflow
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
stdout: 0 bytes
```

The site is `Expr::degree_in`: `inner.degree_in(name) * n.numerator()...` multiplies
`2` (the degree of `x^2 - 1`) by `9223372036854775807`, which is outside `i64` and
panics in the debug profile. The same walk exists in `degree_in_total`.

### 1.2 Zero-width huge array display

```powershell
.\rust\target\debug\lovelace-run.exe --eval 'zeros(9223372036854775807, 0)' --omit-functions
```

Observed (before), with a 10 s `WaitForExit`:

```text
TIMEOUT
KILLED
stdout: 0 bytes
stderr: 0 bytes
```

The value is legitimate (`numel` is 0 and the shape is `[9223372036854775807, 0]`), but
`Value::display` walked the declared outer dimension and rendered one empty row per unit:
2^63-1 iterations, never returning. `--omit-variables` does not help; the result's own
display is the hang.

## 2. Oracle comparison

### 2.1 Symbolic case — the oracle succeeds

```powershell
dotnet exec Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval 'x = symbol(\"x\"); solve_full((x^2 - 1)^9223372036854775807 == 0, x)' --omit-functions
```

Observed: exit 0, 83.76 ms (second run 283.3 ms under load), `ok:true`, and

```text
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])
```

This is the exact string the Rust port now publishes (the regression test pins it verbatim).

### 2.2 Display case — the oracle does NOT return a structured refusal

```powershell
dotnet exec Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval 'zeros(9223372036854775807, 0)' --omit-functions
```

Observed: exit **-532462766** (0xE0434352), stdout **0 bytes**, 2.8 s, and stderr:

```text
Unhandled exception. System.ArgumentOutOfRangeException: Non-negative number required. (Parameter 'capacity')
   at System.Collections.Generic.List`1..ctor(Int32 capacity)
   at Lovelace.Suite.ValueFormatter.FormatLevel(ArrayValue array, Int64[] shape, Int32 dim, Int32 rank, Int64 offset, Boolean unicode) in ...\Lovelace.Suite\ValueFormatter.cs:line 75
   at Lovelace.Suite.ValueFormatter.FormatArrayCore(ArrayValue array, Boolean unicode) in ...\ValueFormatter.cs:line 59
   ...
```

**Premise correction.** The task brief says the oracle returns a structured result/refusal
for both inputs. That is true for the symbolic case and false for this one: the oracle process
aborts with no envelope because `List<string>((int)shape[dim])` narrows
`9223372036854775807` to `-1`. Round 22 §8.2 recorded the same oracle behaviour
(`exit -532462766`). There is therefore no oracle code/category/message to match. The Rust
fix keeps the oracle's *evaluation* result (the zero-element array is constructible; `numel`
answers 0) and makes the **display** a bounded computation, which is the only behaviour that
satisfies "never panic, never hang" on this input. A structured refusal was also rejected
because it would break the existing D5 contract and the round-22 test
`a_zero_dimension_keeps_the_shape_constructible` (which pins
`numel(zeros(9223372036854775807, 0)) == 0`).

## 3. Fixes

### 3.1 Saturating, exponent-safe degree

`rust/lovelace-symbolics/src/lib.rs:498` and `:771`: every degree operation now uses
`saturating_add`, `saturating_mul` and `saturating_sub`. The exponent is read with
`parse::<i64>()`; a positive value too wide for i64 saturates to `i64::MAX`, a negative one
to `i64::MIN`. A degree is an ordering/dispatch hint, never a value, so saturation is the
correct failure mode.

### 3.2 Materialization bound instead of resize-by-exponent

`lib.rs:2049-2074` introduces `MAX_MATERIALIZED_POLYNOMIAL_DEGREE = 1 << 16`,
`degree_within_materialization` and `degree_index`. `poly_coeffs` (`:2078`) refuses a
degree above the bound (returns `None`, which the solver reports as its structured
`Unevaluated` outcome) and `collect` (`:1947`) returns the expanded expression unchanged
instead of synthesizing `x^<saturated>`. This is what makes
`collect((x^2 - 1)^9223372036854775807, x)` print the oracle's unchanged expression.

### 3.3 Structural inverse branch for every positive integer exponent

`rust/lovelace-lang/src/builtins.rs:1743-1784`: the existing 2..=16 branch is preserved;
exponents 17..=i64::MAX, and positive integers wider than i64, take the oracle's principal
inverse branch (`power_branch_outcome(..., 1)`). For `u^n = 0` this solves `u = 0` once,
exactly as the oracle does when its 1..=16 count parse fails. No exponent is cast to a count or
used to size a vector. Exponent 0 and 1 keep the polynomial path.

### 3.4 Monomials above the materialization bound

`rust/lovelace-lang/src/builtins.rs` (immediately before the polynomial dispatch): a pure
`Pow(variable, n) = 0` is answered directly. For `1 <= n <= i32::MAX` it returns the one
root 0 with multiplicity `n` (matching the oracle's `x^1000000` record); above
`i32::MAX`, where the reference's own int conversion makes the polynomial a nonzero constant,
it returns the oracle's `NoSolutions` record with the `solve.no-solutions` diagnostic. This
keeps `x^1000000` from reaching the 65 536-coefficient materialization guard (and from the
pre-fix `capacity overflow`) while matching the oracle on both sides of the boundary.

### 3.5 Overflow-safe rational-root search

`lib.rs:2422`: `find_rational_root` no longer calls `i64::MIN.abs()`
(`unsigned_abs` is used), scales with checked `i128` arithmetic instead of `as i64` /
unchecked multiply, refuses coefficients that do not fit the enumeration, and skips divisors
above `i64::MAX`. `divisors` (`:2469`) takes `u64` and is capped at 100 000; `lcm`
(`:2493`) returns `Option<u64>` from `checked_mul`. `Rat::pow_i64` (`:100`) uses
`checked_neg`, so `i64::MIN` is a refusal (`None`) rather than a negate panic.

### 3.6 Bounded display

`rust/lovelace-lang/src/lib.rs:295-363`: `MAX_DISPLAY_NODES = 100_000` and a
`DisplayBudget` guard both `Value::Vector` display (`:224`) and the recursive
`render_array` walk (`:323`). `render_array` spends one node per declared-dimension
iteration before descending and appends `...` when the budget is exhausted, so a zero-width
shape with a huge outer dimension terminates. Values at or below the budget render byte for
byte as before (the 20 000-element wide-value depth test still ends with `19999]`; `zeros(2,
0)` still prints `[[], []]`).

Note: `rust/lovelace-lang/src/format.rs` is **not in the compiled module graph** (the rustc
dep-info lists only `lib.rs`, `builtins.rs`, `registry.rs`, `structured.rs`, `syntax.rs`
and the test modules). Its parallel array formatter is therefore not on the CLI path; the
guard is placed in the compiling `lib.rs` display functions.

### 3.7 After probes

```powershell
.\rust\target\debug\lovelace-run.exe --eval 'x = symbol(\"x\"); solve_full((x^2 - 1)^9223372036854775807 == 0, x)' --omit-functions
```

```text
exit 0, 230.6 ms (debug)
SolveResult(status: Solved, ... solutions: [Solution(value: -1, ...), Solution(value: 1, ...)], ...)
stderr: 0 bytes
```

```powershell
.\rust\target\debug\lovelace-run.exe --eval 'zeros(9223372036854775807, 0)' --omit-functions
```

```text
exit 0, 103.4 ms (debug); release: exit 0, display 400 005 bytes
display head: [[], [], [], [], [], [], [], [], [], [], [], [], [], [], [],
display tail: [], [], [], [], ...]
```

`collect((x^2 - 1)^9223372036854775807, x)` now prints the oracle's
`(x^2 - 1)^9223372036854775807`.

## 4. Audited sites

Every site below was read in the current tree. "Fixed" means a regression test exists in
§5; "safe by guard" means the indexing/arithmetic is dominated by an explicit length/range
check; "remains" is a known bounded-or-unbounded cost that this round did not change.

| Site | Class | Disposition |
|---|---|---|
| `lib.rs:498` `degree_in` (Pow `*`, Mul `+`, Div `-`, exponent parse) | overflow / user exponent | **fixed**, saturating; tests §5 |
| `lib.rs:771` `degree_in_total` (same arithmetic) | overflow / user exponent | **fixed**, saturating; same test |
| `lib.rs:1947` `collect` (degree used as coefficient key) | allocation by user degree | **fixed**, returns expanded expression above the bound; tests §5 |
| `lib.rs:2078` `poly_coeffs` (`degree as usize`, `resize(index + 1)`) | unchecked cast / allocation | **fixed**, `degree_index` bound; tests §5 |
| `lib.rs:2422` `find_rational_root` (`abs()`, `unwrap_or(0)`, `n * scale as i64`) | overflow / lossy cast | **fixed**, unsigned_abs + checked i128; tests §5 |
| `lib.rs:2469` `divisors` (loops/casts) | count loop | changed to `u64`, capped at 100 000 |
| `lib.rs:2493` `lcm` (unchecked multiply) | overflow | **fixed**, checked `Option` |
| `lib.rs:100` `Rat::pow_i64` (`-exp`, positive `0..exp` loop) | overflow / user-sized loop | **fixed** for `i64::MIN`; positive loop remains (see below) |
| `lib.rs:1876`, `:1905` `expand_inner` Pow loop (`0..k` from a `u32` exponent, distributive growth) | user-sized loop | remains; polls `cancel`, so a deadline stops it, but no deadline means the cost is the input exponent (round-22 already measured `(x+1)^200` ≈ 5 s). |
| `lib.rs:1530`, `:1490`, `:1523`, `:1579`, `:1589` `series` loops over `order` | user-sized loop | remains; the language reader caps order at 2 147 483 647, the library API accepts any `i64`. |
| `lib.rs:2757` `perfect_square` (`r * r`) | overflow | safe: `r` is capped at 200 000 before the multiply can approach `u64::MAX` |
| `lib.rs:559-573` `to_string` Fun arm (`args[0]` for `not`) | unchecked indexing | safe: the empty-args case returns at `:559`; `rootof` checks `len() == 2` |
| `lib.rs:550`, `:614`, `:1174` `rest[0]` / `factors[1..]` | unchecked indexing | safe by `first()`/`len()` branches immediately above each use |
| `lib.rs:1210-1225` latex Add (`terms[0]`, `terms[1]`, `terms[2..]`) | unchecked indexing | safe: empty check, then `len() == 1` / `len() >= 2` branches |
| `lib.rs:1309` latex `bottoms[0]` | unchecked indexing | safe: only in the `!denominators.is_empty()` branch |
| `lib.rs:2941-2979` `canonical`, `:2985` `node_count`, `:3053` `domain`, `:3250` `debug_print` (`args[0]`) | unchecked indexing | safe: every arm tests `(name, args.len())` or `args.len() == 1/2` first |
| `lib.rs:2284` `solve_polynomial_named` (empty coeffs) | unchecked indexing | safe: explicit `is_empty` return at `:2291` |
| `lib.rs:2525`, `:2564` quadratic (`coeffs[2]`), `:2598` cubic (`coeffs[3]`), `:2703` biquadratic (`coeffs[4]`) | unchecked indexing | safe: callers gate on `working.len() == 3/4` and `solve_biquadratic` checks `len() != 5` |
| `lib.rs:2174-2196` `poly_remainder`, `:1734-1766` `poly_div`, `:2497` `divide_by_root` | unchecked indexing | safe by loop bounds (`r.len() > db + 1`, `rem.len() >= den.len()`, `coeffs.len() >= 2`) |
| `lib.rs:2201` `real_root_count` (`sequence[n-2]`, 256 sequence cap) | indexing / count | safe for n>=2; the sequence cap bounds the Sturm chain (the coefficient degree is now materialization-bounded) |
| `lib.rs:3418` `CanonicalParser::parse_node` and the recursive expression walks (`to_string`, `canonical`, `node_count`, `expand_inner`, `diff`, `substitute`, `free_symbols`, `is_exact`, `domain`, `debug_print`, `latex_render`) | recursion with user-controlled depth | library API has no depth guard; the language surface bounds symbolic depth at 256 (`ensure_symbolic_budget`, measured iteratively) and parsed/value depth at 512/1024. Remains for direct library callers (e.g. a hand-built `Expr` or `canonical_parse` of a deeply nested text). |
| `rust/lovelace-lang/src/lib.rs:224`, `:323` display of Vector/Array | display loop by user size | **fixed**, node budget; tests §5 (timeout test) |
| `rust/lovelace-lang/src/lib.rs:1447` `value_depth` | recursion depth | safe: iterative stack |
| `rust/lovelace-lang/src/builtins.rs:1743-1784` solve inverse branch | user exponent -> allocation/count | **fixed**, principal branch; tests §5 |
| `rust/lovelace-lang/src/builtins.rs:1687-1735` monomial branch (before the polynomial dispatch) | user exponent -> materialized degree | **fixed**, direct root/multiplicity for 1..=i32::MAX and the oracle's NoSolutions above it; tests §5 |

Remaining cost bounds, stated plainly: `expand` and `series` still do work proportional to a
user exponent/order when no deadline is armed, and `Rat::pow_i64` still has a positive
`0..exp` loop for library callers. They are reachable long computations, not panics; this
round's reported non-termination (the display walk) is closed.

## 5. Regression tests

`rust/lovelace-symbolics/src/panic_sweep_round23.rs` (registered in `lib.rs`; 5 tests):

1. `huge_exponent_degree_saturates_instead_of_multiplying_with_overflow`
2. `collect_leaves_the_huge_power_alone`
3. `poly_coeffs_refuses_a_degree_it_cannot_materialize`
4. `a_polynomial_with_the_i64_min_coefficient_does_not_panic_in_root_search`
5. `a_negative_i64_min_exponent_is_refused_not_negated`

`rust/lovelace-lang/src/symbolics_round23.rs` (registered in `lib.rs`; 8 tests):

1. `solve_full_of_a_huge_power_of_a_sum_matches_the_oracle_record` — exact oracle display
2. `a_monomial_above_the_materialization_bound_matches_the_oracle` — exact `x^1000000` record
3. `a_monomial_above_i32_is_the_oracles_nosolutions_record` — exact `x^9223372036854775807` record
4. `collect_of_a_huge_power_of_a_sum_returns_the_unchanged_expression`
5. `term_ordering_with_a_huge_degree_does_not_overflow`
6. `solve_full_with_a_coefficient_at_i64_min_returns_instead_of_panicking`
7. `a_zero_width_huge_dimension_displays_within_a_timeout_and_a_node_budget` — the display
   runs on a worker thread; the main thread does `recv_timeout(10 s)` and the test **fails if
   it did not return**, then asserts `numel == 0`, the visible `...`, the 100 000-node length
   bound and the `(Array)` typed suffix. This is a timeout-style test, not a compile test.
8. `small_zero_width_arrays_render_exactly_as_before` — `zeros(2, 0)`, `zeros(3, 0)`,
   `zeros(2, 3)` unchanged.

The suite count rose from 4 059 to **4 072** (the 13 tests above); the gate table below was
re-measured after the monomial branch was added.

## 6. Gates

| Gate | Observed |
|---|---|
| `cargo test --workspace` | **4 072 passed, 0 failed**, 19 suites (log `rust/target/tmp/round23-cargo-test.log`) |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** — only `sym/071`, `sym/076` (log `rust/target/tmp/round23-corpus.log`) |
| `node parity/tools/gate-fixtures.mjs` | oracle **20/20**, rust **20/20**, PASS |
| `node parity/tools/gate-cli.mjs` | CLI matrix **16/16**, PASS |
| `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated **154 match / 46 mismatch / 0 error**; replayed **600 match / 1 152 mismatch / 0 error**; `new=0`; **delta 0** against the round-22 row (same counts). Run dir `parity/diff/runs/20261111T005216Z-seed1`; harness exit 1 because the baseline is red, not because of a new mismatch |

## 7. What remains

1. The two corpus failures `sym/071` and `sym/076` are pre-existing and outside this
   round's fixes.
2. Unbounded-but-not-panicking costs listed in §4: `expand`/`series` work proportional to a
   user exponent/order without a deadline, and `Rat::pow_i64`'s positive loop for direct
   library callers. A future round can cap or budget these; capping them would change
   expansion/series semantics and oracle parity, so it was not done here.
3. The dotnet oracle's own zero-width display abort (`ValueFormatter.cs:75`) is a C# defect
   outside this round's scope; the Rust port now terminates where the oracle does not return.
4. `expand((x+1)^4000)` and similar remain deadline-bounded only; a script with no deadline
   can still spend a long time there.
