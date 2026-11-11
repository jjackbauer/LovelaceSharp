# Round 18 — in-kernel cancellation and the wire rows that depend on it

Scope: `rust/lovelace-lang/src`, `rust/lovelace-symbolics/src`, `rust/lovelace-run/src`. No `.cs`,
fixture, corpus or `parity/` file was touched. The report itself is the only file outside those
three crates.

Before this round the `--cancel-after` deadline was observed only BETWEEN top-level statements. One
long statement therefore ran to completion and the runner either refused for an unrelated reason
first (`sum(1..10000000)` answered `InvalidOperation`/"Range is too large." after 3.5 s) or reported
cancellation *post hoc* with a ledger whose `elapsedMs` was the whole run (`matmul(eye(400),
eye(400))`: 16.9 s under a 100 ms budget). The oracle stops inside the kernel and reports the
cancellation while it is happening.

## 1. What changed

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/cancel.rs` (new) | The deadline token every kernel polls. Thread-local `Option<Instant>` + `EXPIRED` flag; `arm()` returns a scope that restores the previous state, `poll(iteration)` reads the clock once per 64 iterations, `check()` is the unstrided boundary form. With no deadline armed the hot path is one `Option` read, so an un-budgeted run is unchanged. |
| `rust/lovelace-symbolics/src/lib.rs` | `pub mod cancel`; polls in `expand`/`expand_inner`/`distributive_mul`, `series`/`series_add`/`series_mul`/`series_inv`, `substitute`, `collect`, `diff`, `apart`, `eval_at`, `poly_div`, `cancel`, `poly_coeffs`, `real_root_count`, `solve_polynomial_named` (round loop, rational-root search, divisor scan), `cubic_root_expressions`, `solve_biquadratic`. |
| `rust/lovelace-lang/src/lib.rs` | `Engine::set_cancel_after`/`cancel_after`/`was_cancelled`/`last_completed_position`; `eval` arms the deadline for exactly one evaluation (RAII scope) and converts a kernel-observed stop into the structured `Cancelled` failure; statement-boundary check, `while`/`for` iteration checks, call-boundary checks (pre-call and post-kernel); `cancelled_error()` + `CANCELLED_MESSAGE` publish the reference's `Cancelled`/`BudgetExceeded` pair. |
| `rust/lovelace-lang/src/builtins.rs` | Polls in `make_range`, `reduce` (axis and flat), `matmul` (triple loop), `dot`, `norm`, `mean`'s total, every `elementwise`/`elementwise_scalar`/broadcast map, `zeros`/`ones` (chunked fill), `eye`, `det`, `inv_matrix`, `factor_expr`, `simplify`/`simplify_traced`, `integrate_basic`/`integrate_simple_fraction`, `solve_linear_symbolic`, `solve_system_impl`, and a checkpoint at the entry of the `expand`/`simplify`/`factor`/`solve`/`solve_system`/`integrate`/`integrate_full` branches. |
| `rust/lovelace-run/src/main.rs` | `execute()` (arms the deadline, measures elapsed) and `envelope_for()` are now the single flow `main` and the in-process wire tests share; `cancelled_envelope` takes the engine's `Cancelled` error (message + position), reports `stopped:true`, publishes `output` and `partialOutput` from the same committed lines and `partialVariables` from the committed prefix, and adds the overrun diagnostic only when the stop landed past the budget (the engine's own diagnostics are cleared, as the reference clears them). The success path keeps `stopped:false` and the "completed anyway" diagnostic: a completed evaluation is never turned into a cancellation. `--help` now says the budget cancels. |
| `rust/lovelace-run/src/parity_batch.rs`, `error_taxonomy_batch.rs` | Drive the shared `execute`/`envelope_for` pair; the cancelled-envelope test supplies the `Cancelled` error. |
| `rust/lovelace-lang/src/cancellation_round18.rs` (new, 15 tests) | Engine-level ports of the C# kernel-cancellation rows + the kernel polls + position/partial-state checks. |
| `rust/lovelace-run/src/cancellation_batch.rs` (new, 10 tests) | Wire ports of `CancellationBudgetTests`, `PrintOutputOnFailureTests.TheCancelledPath`, the envelope key set, the RUN-010 position, and the completed-but-late arm. |

## 2. The token, and why it is cheap

```text
poll(i):  if EXPIRED                       -> true          (one thread-local read)
          if DEADLINE is None              -> false         (one thread-local read)
          if (i & 63) == 0                 -> Instant::now() >= deadline
check():  Instant::now() >= deadline (only for a recursion boundary / outer sweep)
```

The state is thread-local, not global: two engines on two threads cannot cancel each other, and a
test that arms a deadline cannot leak it into the rest of the process. `Engine::eval` arms the
deadline at entry and restores the previous state on every exit path (including unwind), so the
next run starts un-budgeted unless the caller arms it again.

A kernel that observes the deadline returns its identity/partial value (`expand` returns its input,
`solve_polynomial` returns `Unevaluated`, `series` returns `None`, `integrate_basic` returns
`None`, the loops return the structured error). The partial value is never observable: the
evaluation discards it and publishes `Cancelled`, exactly as the reference discards the partial its
cancelled kernel was building.

## 3. The cancelled envelope (oracle, measured)

Measured from the frozen oracle (`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`) with
`--eval "sum(1..10000000)" --cancel-after 100 --omit-functions --omit-variables`:

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.",
 "recoverable":true,
 "diagnostics":[{"message":"cancellation deadline exceeded: --cancel-after 100 ms, elapsed 102.52 ms (2.52 ms over budget); the deadline was observed, but only after the excess had already been spent.","position":0,"line":1,"column":1}],
 "elapsed":"102.52 ms","elapsedTime":{"value":102.52,"unit":"ms"},
 "timings":[{"position":0,"elapsed":{"value":...,"unit":"ms"},"resultKind":"Void","hasOutput":false}],
 "output":[],"partialOutput":[],"partialVariables":[],
 "cancellation":{"budgetMs":100,"elapsedMs":102.52,"stopped":true,"exceeded":true,"excessMs":2.52},
 "artifacts":[]}
```

The Rust envelope carries the same 17 keys with the same values:

| Field | Oracle | Rust (same argv) |
|---|---|---|
| exit / ok / code / category | 1 / false / `Cancelled` / `BudgetExceeded` | same |
| message | `the evaluation was cancelled by the caller.` | same |
| diagnostics | one overrun entry, position 0, line 1, column 1 | same |
| cancellation | `{budgetMs:100, stopped:true, exceeded:true}`, `elapsedMs`/`excessMs` measured | same verdicts, same measured numbers |
| output / partialOutput | `[]` / `[]` (the committed lines when the script printed) | same |
| partialVariables | committed prefix only | same |
| timings | the stopped statement keeps a `Void` entry | same |
| artifacts | `[]` | same |

The ledger is self-consistent by construction: `exceeded == (excessMs > 0)`, `excessMs >= 0`, and the
overrun diagnostic is present exactly when `exceeded` is true. On a cancellation the engine's own
diagnostics are cleared (the reference clears them too), so a cancelled run's `diagnostics` array is
the runner's overrun entry only.

## 4. Before/after probes

The "before" figures are from the pre-change release binary on this host (two independent baseline
builds, `rust/target/tmp/r16-baseline-target` and `rust/target/tmp/before-target`, produce the same
verdict and envelope shape; their measured durations differ by a few milliseconds, as any two
builds' do, and the build predates every edit in this round). The "after" figures are
`rust/target/release/lovelace-run.exe` built from this round's tree. Wall time is the spawned
process; `elapsedMs` is the ledger's engine measurement, which is what the C# fence reads.

### P1 — huge sum: `sum(1..10000000)`, `--cancel-after 100`

| | Before | After |
|---|---|---|
| wall | **3,479 ms** | **121 ms** |
| envelope | exit 1, `ok:false`, code `InvalidOperation`, message `Range is too large.` | exit 1, `ok:false`, code `Cancelled`, category `BudgetExceeded` |
| ledger | `stopped:false`, `exceeded:true`, `elapsedMs:3458.076` | `stopped:true`, `exceeded:true`, `elapsedMs:100.665` |
| diagnostics | the domain refusal **plus** a post-hoc overrun entry | one overrun entry, position 0/1/1 |

The before run never reached the cancellation path: the range build refused with an unrelated error
after 3.5 s. After: the range/reduction loop observed the deadline at 100.7 ms and the envelope is
the oracle's cancelled envelope.

### P2 — matmul: `matmul(eye(400), eye(400))`, `--cancel-after 100`

| | Before | After |
|---|---|---|
| wall | **17,091 ms** | **131 ms** |
| envelope | exit 1, `Cancelled` — but only after the product finished | exit 1, `Cancelled` |
| ledger | `stopped:true`, `elapsedMs:16936.936` | `stopped:true`, `elapsedMs:110.411` |

The triple loop now polls the innermost index; 64 million boxed multiply-adds stop 110 ms into a
100 ms budget instead of running for 17 s first.

### P3 — simplify/expand: `simplify(expand((symbol("x")+1)^120))`, `--cancel-after 50`

| | Before | After |
|---|---|---|
| wall | **634 ms** | **70 ms** |
| envelope | exit 1, `Cancelled` post hoc | exit 1, `Cancelled` |
| ledger | `stopped:true`, `elapsedMs:611.768` | `stopped:true`, `elapsedMs:50.666` |

The expansion that builds the argument is the long pole here (the Rust simplifier itself is a
single bottom-up walk that finishes in microseconds on 121 terms); its distribution loop polls, and
the simplifier's own walk polls too (`simplify_cancels_inside_the_walk` builds a 200,000-term sum
without a deadline and stops inside the walk at 20 ms).

### P4 — solve: `solve(expand((symbol("x")+1)^100) == 0, symbol("x"))`, `--cancel-after 10`

| | Before | After |
|---|---|---|
| wall | **484 ms** | **29 ms** |
| envelope | exit 1, `Cancelled` post hoc | exit 1, `Cancelled` |
| ledger | `stopped:true`, `elapsedMs:464.832` | `stopped:true`, `elapsedMs:10.507` |

The Rust solver answers the CLI-scale workloads in about a millisecond, so the CLI probe stops in
the expansion that builds its argument. The solver's own token is proven directly: with an armed
expired deadline `solve_polynomial` abandons the search (`Unevaluated`) and the token records the
observation, and every `solve`/`solve_system` call site refuses under an expired deadline
(`solve_consults_the_deadline`, `symbolic_builtins_refuse_at_the_call_seam_when_the_deadline_is_spent`).

### P5 — negative control: `sum(1..1000)`, `--cancel-after 60000`

Before and after are identical: exit 0, `ok:true`, `result.display = "500500"`, ledger
`{budgetMs:60000, stopped:false, exceeded:false, excessMs:0}`, no diagnostics. An armed deadline
that is never reached changes nothing; with no deadline at all the envelope carries no
`cancellation` and no `diagnostics` key, byte-for-byte as before.

## 5. Probe-by-probe agreement with the oracle

Same argv on both runners, compared on the fields the wire contract names (measured on this host):

| Probe | exit / code | ledger verdicts | diagnostic (position/line/column) | output / partialOutput |
|---|---|---|---|---|
| P1 sum 100 ms | 1 / `Cancelled` both | `{100, stopped:true, exceeded:true}` both | 0/1/1 both, same normalised text | `[]` / `[]` both |
| P2 matmul 100 ms | 1 / `Cancelled` both | `{100, true, true}` both | 0/1/1 both | `[]` / `[]` both |
| P3 simplify/expand 50 ms | 1 / `Cancelled` both | `{50, true, true}` both | 0/1/1 both | `[]` / `[]` both |
| P4 solve 10 ms | 1 / `Cancelled` both | `{10, true, true}` both | 0/1/1 both | `[]` / `[]` both |
| P5 in-budget 60 s | 0 / `ok:true` both | `{60000, false, false}` both | absent both | `[]` both |

Both runners publish 17 keys on the cancelled envelope and 14 on the in-budget success envelope for
the same argv.

## 6. C# cases this covers

The census contains 26 ids matching `cancel`/`Cancel`; 15 are cancellation-behaviour cases and 11
are the unrelated symbolic `cancel`-named rows. (The brief's "~322 Lovelace.Run.Tests cases" figure
is not traceable in this tree's census or manifest: the cancellation-specific pool there is 15.)

| C# case | Where | Status |
|---|---|---|
| `ArrayKernelCancellationTests.SumOverRange_GivenShortBudget_CancelsInsideTheRangeAndReduction` | `lovelace-lang::cancellation_round18` | ported |
| `...ProductOverRange_GivenShortBudget_CancelsInsideTheGrowingProduct` | same | ported (`prod(1..20000)`, 50 ms; the C# row's 200,000-iteration product is bounded so a non-observing failure returns in 0.3 s instead of 30 s; the assertion and the budget/work ratio are unchanged) |
| `...MatrixProduct_GivenShortBudget_CancelsInsideTheTripleLoop` | same | ported (`matmul(eye(400), eye(400))`, 100 ms) |
| `...WhileLoop_GivenShortBudget_CancelsBetweenIterations` | same | ported (1M iterations instead of 100M, for failure-time bounding; the loop is still 15x the budget) |
| `...ElementwiseArrayArithmetic_GivenShortBudget_CancelsInsideTheMap` | same | ported (`zeros(1000000); a + a; sum`, 100 ms; C# uses 5M elements) |
| `...RunInsideBudget_GivenGenerousBudget_StillReturnsTheResult` | same | ported |
| `CancellationBudgetTests.Run_GivenBudgetFarBelowTheSum_StopsInsideTheKernelAndSaysSo` | `lovelace-run::cancellation_batch` | ported |
| `...Run_GivenBudgetFarBelowTheMatrixProduct_StopsInsideTheKernelAndSaysSo` | same | ported |
| `...Run_GivenWorkInsideTheBudget_SucceedsNormallyAndReportsNoExcess` | same | ported |
| `...Run_GivenNoBudget_CarriesNoCancellationBlockAtAll` | same | ported |
| `...Run_GivenExceededDeadline_CarriesAMachineReadableDiagnostic` | same | ported |
| `...Run_GivenADeadline_ReportsItsVerdictConsistently` | same | ported (five attempts, as the C# row) |
| `PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput` | same | ported |
| `CancellationObservationTests.Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly` | — | **refused**: the power loop is in `lovelace-nat`, outside this round's scope; the call is only stoppable at the statement boundary (measured: the engine still finishes `2^10000000` in 159 ms, then renders) |
| `CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly` | — | **refused**: same reason (`Nat::factorial`); the completed-but-late arm it exercises is covered by `a_run_that_completes_past_its_deadline_is_a_success_with_a_truthful_ledger` |

The two refused rows stay refused rather than weakened; they need `rust/lovelace-nat` changes,
which are out of scope.

## 7. Verification

```text
cargo test --workspace --no-fail-fast
  lovelace-array 58; lovelace-desktop 67; lovelace-int 125; lovelace-json 7;
  lovelace-lang 833 (818 + 15 new); lovelace-nat 134; lovelace-real 2,345;
  lovelace-run 172 (162 + 10 new) + 1 integration; lovelace-symbolics 47
  = 3,789 tests, 0 failed, exit 0            (before this round: 3,764)

cargo run --release -p lovelace-corpus
  corpus: 212 passed, 2 failed, 214 total    (sym/071, sym/076 — unchanged)

node parity/tools/gate-fixtures.mjs
  oracle: 20/20, rust: 20/20                 (20 fixtures green)

node parity/tools/gate-cli.mjs
  CLI matrix: 16/16                          (RUN-010, the cancellation row, included)

node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
  generated: 154 match, 46 mismatch, 0 error   (before: 154 / 46 / 0)
  regressions: 1,152 still mismatch, 600 resolved, 0 new   (before: same)
```

RUN-010 (`x = symbol("x"); expand((x+1)^40)`, `--cancel-after 5`) now compares equal on every field
the gate compares: `partialVariables` is `["x"]` on both runners (the old OQ-013 `_` difference is
gone: the statement the deadline stopped no longer commits), `output`/`partialOutput` are empty on
both, and the overrun diagnostic is at position 0 / line 1 / column 1 on both. The oracle cancels
BETWEEN statements there (its first statement's JIT already spends the 5 ms budget) and publishes
position 0; this runner cancels inside the expansion and publishes the completed prefix (position
0) while `timings[]` records the stopped statement at 17 (that array is volatile-normalised by the
gate).

## 8. Remaining (measured, not hidden)

1. **The Natural kernels are not polled.** `2^1000000000` and `2000000!` can only be stopped at the
   statement boundary until `rust/lovelace-nat` grows a poll hook; the two `CancellationObservationTests`
   rows stay refused. `200000!` (52.7 ms engine) is the measured completed-but-late arm: exit 0,
   `ok:true`, ledger `stopped:false/exceeded:true`, diagnostics "the evaluation completed anyway".
2. **Diagnostic position on a mid-statement stop.** The reference positions the overrun diagnostic
   at the last entry of `timings[]`. When it observes the deadline inside a statement that entry is
   the STOPPED statement (measured: `...; expand((x+1)^120)` → 17; `print("hello"); sum(...)` → 33).
   This runner always stops inside a statement, so it publishes the last COMPLETED statement — the
   same value the reference publishes on RUN-010, and the only position difference left. On a
   multi-statement stop both runners publish the same `timings[]` (measured: `0:Symbolic, 17:Void,
   33:Void` for the print case); on RUN-010 itself the oracle has one entry fewer because IT stopped
   between statements (that array is volatile-normalised by the CLI gate).
3. **Text mode.** `--text` prints `Error [Cancelled/BudgetExceeded]: the evaluation was cancelled by
   the caller.` and exits 1; the oracle's text surface for a cancellation was not compared (no row
   exercises it).
4. **Not every kernel is polled.** Canonical `Sym::add`/`Sym::mul` construction, the tokenizer/parser
   and envelope rendering remain unpolled; a single statement dominated by those can still overrun.

## 9. Reproduction

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
Set-Location $repo

# AFTER: the round-18 binary
rust\target\release\lovelace-run.exe --eval "sum(1..10000000)" --cancel-after 100 `
  --json --omit-functions --omit-variables

# BEFORE: the pre-round binary (built before any edit in this round)
rust\target\tmp\r16-baseline-target\release\lovelace-run.exe --eval "sum(1..10000000)" `
  --cancel-after 100 --json --omit-functions --omit-variables

# every cancellation test
cargo test --manifest-path rust/Cargo.toml -p lovelace-lang cancellation_round18
cargo test --manifest-path rust/Cargo.toml -p lovelace-run cancellation_batch

# the wire gates
node parity/tools/gate-cli.mjs
node parity/tools/gate-fixtures.mjs
cargo run --release --manifest-path rust/Cargo.toml -p lovelace-corpus

# the differential (the generated cases never set --cancel-after)
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
```

The differential's `--out` directory records the `rust` binary's sha256 with the run, so the
before/after pair is attributable to the two binaries rather than to a moving tree.
