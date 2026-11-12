# Round 9 — Kernel cancellation from the UI (desktop v4)

**Status:** implemented and observed. Cancel now reaches work INSIDE one kernel call, not only
between statements. A single opaque high-precision primitive (`sqrt(2)` at precision 200000) is
declared a known limitation in `DEV-008`; the flag cannot reach it. All pre-existing tests pass.

## Goal and the seam that was hooked

`ExecutionHost` used to check only a statement-boundary token, so one long kernel ran to
completion before `Cancelled` was published. The fix adds a live shared `Arc<AtomicBool>` and polls
it at the SAME seams the deadline already used:

| Layer | Change |
|---|---|
| `lovelace-symbolics::cancel` | New thread-local `FLAG: RefCell<Option<Arc<AtomicBool>>>` beside `DEADLINE`. `arm_with_flag(deadline, flag)` replaces both; `arm(deadline)` delegates to `arm_with_flag(deadline, None)`. `Scope::drop` restores the previous deadline, latch AND flag. `expired()` checks `FLAG` BEFORE the clock, so a set flag costs one atomic load; a set flag latches `EXPIRED`. `poll(iteration)` now treats a flag-only arm as armed and calls `expired()` at the existing `POLL_INTERVAL` (64) stride; `check()` is unchanged as the unstrided seam. |
| `lovelace-lang::Engine` | New `cancel_flag: Option<Arc<AtomicBool>>` field and `pub fn set_cancel_flag(&mut self, flag: Option<Arc<AtomicBool>>)`. `Engine::eval` arms `cancel::arm_with_flag(deadline, self.cancel_flag.clone())` at the exact place the deadline was armed. A kernel that observes the flag produces the SAME `Cancelled` / `BudgetExceeded` `LangError` a spent deadline produces: the existing post-evaluation `cancel::observed()` mapping is unchanged. |
| `lovelace-desktop::execution` | `submit` creates ONE `Arc<AtomicBool>` per run. `RunWorker` clones it into the engine through `Evaluator::set_cancel_flag` before `RunStarted`; `RunHandle` keeps the other clone. `ExecutionHost::cancel` still sets the statement token (`RunControl::cancelled`) AND stores the flag. When an engine `eval` returns an error while either seam is set, `RunWorker` publishes the terminal `Cancelled` (committed snapshot first) instead of `Failed`. |
| `lovelace-desktop::engine` (approved extension) | The worker owns `Box<dyn Evaluator>`, not `lovelace_lang::Engine`, so the flag needs one trait method: default no-op `Evaluator::set_cancel_flag` (fake/other test engines fall back to statement-boundary cancellation) and the `RealEngine` override that forwards it to `Engine::set_cancel_flag`. |

Cancellation remains cooperative and latched: a kernel that sees the flag returns its identity/partial
value, the evaluation discards it, and the host never exposes the partial statement.

## Commands

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"

# acceptance: three crates
& "$repo\.toolchain\cargo\bin\cargo.exe" test --manifest-path "$repo\rust\Cargo.toml" `
  -p lovelace-symbolics -p lovelace-lang -p lovelace-desktop

# the new desktop single-kernel proof, with its printed latency
& "$repo\.toolchain\cargo\bin\cargo.exe" test --manifest-path "$repo\rust\Cargo.toml" `
  -p lovelace-desktop -- t9 --nocapture

# the new engine-level live-flag proof, with its printed latency
& "$repo\.toolchain\cargo\bin\cargo.exe" test --manifest-path "$repo\rust\Cargo.toml" `
  -p lovelace-lang --lib -- kernel_cancel --nocapture

# the shipped window's headless acceptance path
& "$repo\.toolchain\cargo\bin\cargo.exe" run --manifest-path "$repo\rust\Cargo.toml" `
  -p lovelace-desktop -- --gui-selftest
```

## Observed outputs (pasted)

Three-crate acceptance, exit code 0:

```
     Running unittests src\main.rs (rust\target-desktop\debug\deps\lovelace_desktop-dd274d469d4bf684.exe)
running 106 tests
test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
     Running unittests src\lib.rs (rust\target-desktop\debug\deps\lovelace_lang-b4a09a4b7eebf57e.exe)
running 1584 tests
test result: ok. 1584 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 85.88s
     Running unittests src\lib.rs (rust\target-desktop\debug\deps\lovelace_symbolics-dbbc81b88919e02b.exe)
running 186 tests
test result: ok. 186 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s
```

Desktop worker proof (ONE kernel statement, cancel 200 ms after `StepStarted`, timeout 2 s):

```
running 1 test
T9 single-kernel cancel-to-terminal took 126.344 ms
test execution::tests::t9_cancel_inside_one_kernel_is_terminal_within_two_seconds ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 105 filtered out; finished in 0.33s
```

Engine-level live-flag proof (helper thread sets the flag 200 ms in):

```
running 2 tests
test kernel_cancel_round9::a_cleared_flag_does_not_change_a_short_run ... ok
kernel-cancel: matmul(eye(400), eye(400)) stopped 71.2 ms after the flag was set (271.7 ms after eval began)
test kernel_cancel_round9::a_live_flag_set_from_another_thread_stops_one_long_kernel ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1582 filtered out; finished in 0.27s
```

`--gui-selftest` long-run line (the cancel case is now ONE long kernel statement):

```
GUI_SELFTEST ok long-run: submit returned in 43 us; 300 ms sample: 198 frames, 200 next-event calls, 2 applied, max 3.900 us; cancel terminal in 27.57 ms after 0 of 1 statements
```

Related unchanged checks from the same run:

```
GUI_SELFTEST ok cancel: cancelled after 2 of 50 statements in 1.55 ms (first committed 1)
GUI_SELFTEST ok keyboard: Ctrl+Enter -> Run (status "run Running", Run disabled, Cancel visible); Esc -> run Cancelled in 3.11 ms; toolbar restored
GUI_SELFTEST summary checks=10 failed=2
```

The two failures are the pre-existing plot checks (`plot`, `plot-dir`): `%APPDATA%\Lovelace\plots`
is not writable in this environment (`Access is denied. (os error 5)`), the same 8/10 and the same
two failures recorded in round 7 (EVD-057). They are unrelated to cancellation.

## Before / after

| Case | Before | After |
|---|---|---|
| Desktop Cancel while the worker is inside ONE kernel | statement-boundary token only; the kernel ran to completion, then `Cancelled` | **126.344 ms** cancel→terminal (single `matmul(eye(400), eye(400))`, `t9`) |
| Engine live flag, one polling kernel | no live flag existed | **71.2 ms** flag-set→`Err(Cancelled)` (engine test) |
| `--gui-selftest` `long-run` cancel case | 13 statements (`setprecision` + 12×`sqrt(2)` at 20000); round-7 measurement 76.49 ms, cancellable only at boundaries | ONE statement `matmul(eye(400), eye(400))`; **27.57 ms** cancel→terminal after **0 of 1** statements |
| `--gui-selftest` `keyboard` Esc | round-7 measurement 388.03 ms | **3.11 ms** |
| `--gui-selftest` `cancel` (50 short statements) | 1.63 ms (round 5) | **1.55 ms**, unchanged semantics |
| Single `sqrt(2)` at precision 200000 | 29,721 ms measured through the engine (round 7), no sub-2 s cancel | **still uninterruptible**: 15.2–22.1 s clean (63.48 s observed end-to-end under load) with the flag set at 200 ms; see below |

## Remaining uninterruptible kernel: `sqrt(2)` at precision 200000

The live flag cannot reach this kernel because there is no poll seam to reach. `sqrt` on a number
calls `lovelace_real::Real::sqrt`, which is one closed big-integer computation: no
`lovelace_symbolics::cancel` call exists anywhere in `lovelace-real`, `lovelace-nat` or
`lovelace-int`. The engine's only checks are at the call boundary before and after it.

Measured profile (throwaway probe replicating `Real::sqrt` with the same public `Nat`/`Real` API,
debug/test profile, 200000 digits, idle machine after the first run proved to be contending with
the other session's test runs):

| Phase | Time |
|---|---|
| `10^400020` + multiply | 119.9 ms |
| `Nat::to_decimal(big)` (400021 digits) | **1515.0 ms** |
| initial guess `10^200005` | 54.3 ms |
| `isqrt`: 22 Newton iterations of `Nat::div_rem` | **575.0–667.2 ms each**, ≈13.2 s total |
| strip guard digits + `to_decimal` | 336.3 ms |
| `Real::from_str` | 86.1 ms |
| `result.mul(&result)` | 39.0 ms |
| `Int::cmp` for the exact/inexact decision | **1708.6 ms** |

The replica totals ≈17.1 s. A real `Engine::eval("setprecision(200000)\nsqrt(2)")` measured
**15.2 s** and **22.1 s** in two clean runs on this machine (round 7 measured 29,721 ms through the
engine). Rendering the result is not the cost: `Engine::format` of the 200000-digit value took
0.2 ms.

Two separate consequences:

1. The flag cannot stop the work at all: no seam exists in that layer.
2. Adding a seam only at phase boundaries would leave a worst-case wait of one whole phase: the
   final `Int::cmp` alone measured 1.7 s and `Nat::to_decimal` 1.5 s. That is marginal against a
   2 s bound even on an idle machine and is lost under load, so a reliable bound needs the seam
   INSIDE the big-integer primitives (chunked operations), which is the layering question in
   OQ-003.

Observed with the real engine before the sqrt test was replaced (live flag set at 200 ms):

```
test kernel_cancel_round9::a_live_flag_set_from_another_thread_stops_one_long_kernel ... FAILED
... it returned <200000-digit value> after 63.483 s
```

That 63.48 s was the test's wall time while the machine was contending with the other session's
test runs; the clean re-measurements above put the same evaluation at 15.2–22.1 s. Either way the
run returned `Ok`, `was_cancelled()` stayed false, and no terminal `Cancelled` was published
inside 2 s. The gap is recorded as `DEV-008` (provisional) with the layering question `OQ-003`,
and the sqrt case is deliberately NOT marked as cancelled or as passing anywhere.

Other known-uninterruptible work is unchanged from round 18: loops that live in the low-level
crates (for example `lovelace-nat` powers/factorials) are still only observable at the statement
boundary. Any primitive that performs one opaque big-integer operation is in the same class as
`sqrt`.

## Can the GUI tell the user the phase is uninterruptible?

**No.** `RunPhase` has only `Idle`, `Running` and `Cancelling`; the status strip shows
`run Running`, then `run Cancelling` while the flag is set, and the user sees that state until the
soft-uncancellable evaluation returns. There is no "uninterruptible" phase, no per-kernel
classification, and no notice that Cancel will not take effect promptly for this particular
workload. Adding one would need the engine to publish which kernel is running and whether it polls;
that is out of this round's scope and is not faked here.

## Test integrity

* No pre-existing test was deleted or weakened. `cancellation_round18` (15 deadline tests), the
  round-3 `t4` statement-boundary cancellation test, and every other pre-existing test still pass
  in the 1584/106/186 summaries above.
* New tests: 5 in `cancel.rs` (set flag expires; cleared flag does not; a flag set after arming is
  observed at the poll stride; arming without a flag keeps the deadline behaviour and disarms an
  installed flag; a scope restores the previous flag), 2 in `kernel_cancel_round9.rs`, 1 in
  `execution.rs` (`t9`).
* One test written during this round (`sqrt(2)` cancel) was removed before delivery after the
  measurement proved the premise false; it was never a pre-existing test and nothing asserts the
  sqrt case passes.
* The only "new" desktop plumbing files beyond the original scope are
  `src/engine.rs` (default no-op trait method) and `src/engine/real.rs` (override), approved by
  the orchestrator because the worker only owns `Box<dyn Evaluator>`.
* Concurrent session: `rust/lovelace-lang/src/lib.rs` and `rust/lovelace-lang/src/numeric.rs` were
  already modified by another session when this round started. Its `structural_equality` module and
  round-78 test declarations are preserved in the diff; its `numeric.rs` changes were not touched.
