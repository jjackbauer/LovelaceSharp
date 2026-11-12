# Round 3 — the execution core actually runs

**Status:** green. `cargo test -p lovelace-desktop` passes 81 of 81 tests with exit code 0.
**Scope touched:** `rust/lovelace-desktop/src/execution.rs` (implementation and one corrected
test), `docs/goal-desktop-v4/round-3/execution.md` (this report). `statements.rs` was **not**
changed by this session — it was already repaired before my first run (see §2). `main.rs` was not
changed.

**Command used (Windows PowerShell, cargo not on PATH):**

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

---

## 1. Test-first failing output I observed

The first run of the command above (09:13:15 local) recompiled the crate and reported:

```text
running 81 tests

---- execution::tests::diagnostic_position_is_translated_back_to_the_full_source stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:1079:9:
assertion `left == right` failed
  left: 21
 right: 27

---- execution::tests::real_engine_executes_sliced_statements_and_snapshots_variables stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:582:25:
next_event returned None before a terminal event; collected []

---- execution::tests::t6_panicking_engine_fails_one_run_and_the_host_stays_usable stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:582:25:
next_event returned None before a terminal event; collected []

---- execution::tests::t2_submit_does_not_block_and_run_started_arrives_quickly stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:631:32:
RunStarted was not available within 100 ms of submit

---- execution::tests::t8_output_events_are_exactly_the_printed_lines_and_do_not_accumulate_across_runs stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:582:25:
next_event returned None before a terminal event; collected []

---- execution::tests::t3_events_are_sequenced_and_ordered_with_run_identity stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:582:25:
next_event returned None before a terminal event; collected []

---- execution::tests::t5_duplicate_submit_is_busy_and_other_documents_run_concurrently stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:873:25:
document 2 did not finish within 5 s

---- execution::tests::t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:755:25:
no StepCommitted event within 5 s; collected []

---- execution::tests::t7_stale_finished_events_are_ignored_by_the_sink stdout ----
thread ... panicked at lovelace-desktop\src\execution.rs:984:9:
the first Finished must be applied

failures:
    execution::tests::diagnostic_position_is_translated_back_to_the_full_source
    execution::tests::real_engine_executes_sliced_statements_and_snapshots_variables
    execution::tests::t2_submit_does_not_block_and_run_started_arrives_quickly
    execution::tests::t3_events_are_sequenced_and_ordered_with_run_identity
    execution::tests::t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot
    execution::tests::t5_duplicate_submit_is_busy_and_other_documents_run_concurrently
    execution::tests::t6_panicking_engine_fails_one_run_and_the_host_stays_usable
    execution::tests::t7_stale_finished_events_are_ignored_by_the_sink
    execution::tests::t8_output_events_are_exactly_the_printed_lines_and_do_not_accumulate_across_runs

test result: FAILED. 72 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

This is **not** the 67 passed / 14 failed run the orchestrator reported. The difference is exactly
the five `statements::tests::t1_*` tests: they were already green at my first run. The orchestrator's
report was taken against an earlier revision of `statements.rs`; that revision was replaced at
09:13:02, eight seconds before my session started (09:13:10). The old and new files and the
abandoned implementer session are documented in §2. I did not re-run the old revision; its failure
text is quoted from the orchestrator's message and from the recovered session transcript.

---

## 2. Root cause of the empty slicer — exact lines

The 7 297-byte version of `rust/lovelace-desktop/src/statements.rs` (202 lines) did not implement
the slicer at all. Recovered byte-for-byte from the previous implementer's DSH session transcript
(`~/.dsh/sessions/--C-Users-ricar-dev-LovelaceSharp--/d415102c-.../session.v4.jsonl.zstd`, written
by its `write` tool call; 7 295 characters / 7 297 UTF-8 bytes — the two extra bytes are the `…`
in `Statement::label`), the relevant lines are:

```rust
 59 /// Splits a source buffer into its top-level statements.
 60 ///
 61 /// The function is total: every input, including empty, comment-only and unbalanced input,
 62 /// yields a vector and never panics.
 63 pub fn slice(source: &str) -> Vec<Statement> {
 64     let _ = source;
 65     Vec::new()
 66 }
 67
 68 /// Returns the 1-based line and column of a byte offset in `source`.
 69 ///
 70 /// Columns count Unicode scalar values, matching the engine's user-facing editor positions.
 71 pub fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
 72     let _ = (source, byte_offset);
 73     (1, 1)
 74 }
```

`slice()` therefore **always returned an empty vector**, regardless of input. That is the exact
cause of `assertion left == right failed: statements: []` at `statements.rs:102` (the first test's
`assert_eq!(statements.len(), 3)`) and of the follow-on `index out of bounds: the len is 0` at
`statements.rs:198`. `line_column()` was also a `(1, 1)` stub, which is why the diagnostic test
would still have been wrong after the slicer was fixed.

Evidence that the current file is a later repair, and that **the tests were not changed**:

* Recovered old file: 7 297 bytes, 202 lines. Current file: 10 823 bytes, 316 lines.
* The test module (`#[cfg(test)]` to EOF) is **byte-identical** between the recovered old file and
  the current file: 4 562 characters each, exact `-ceq` comparison true. Only the implementation
  changed.
* `statements.rs` mtime is 09:13:02; my first test run was 09:13:15. The previous implementer
  session `d415102c` has its final `turn/end` with `"reason":{"kind":"aborted","reason":{"kind":"parent"}}`
  at 09:13:02; the fixed file was already in place when it was aborted. My session was started
  afterwards and inherited the fixed slicer. I did not modify `statements.rs`.
* The orchestrator's 67/14 run was against the stub revision. The repaired implementation is a
  byte scanner that tracks strings, `#` comments, `()[]{}` depth, `;`/newline separators, trims the
  statement span while keeping absolute byte offsets and 1-based lines; it passes all five slicer
  tests with exact reconstruction, including CRLF and comment-only inputs.

---

## 3. Fixes made

### 3.1 Slicer

Already fixed before my first run; I verified rather than rewrote it. No assertions or expected
values were touched (test module proved identical to the 7 297-byte version). I audited the scanner
for the acceptance cases: strings with newlines and semicolons, escaped quotes, comments containing
semicolons, brace blocks, CRLF trimming, empty/comment-only sources, byte-exact gap reconstruction,
1-based UTF-8 columns.

### 3.2 `execution.rs` — implemented the execution core

The file was a compilable skeleton with the public types and all tests, but the behaviour was
stubbed out: `submit()` inserted a document and returned a handle without spawning anything,
`cancel()` returned `false`, and `EventSink::apply()` returned `false`. The following was
implemented:

| Area | Fix |
|---|---|
| Host plumbing | `ExecutionHost` now owns the channel sender, an `Arc<Mutex<HostState>>` and an `Arc<Mutex<HashMap<DocumentId, WorkspaceSnapshot>>>`, so workers can publish snapshots and retire themselves. |
| `submit` | Checks duplicate runs per document (`Busy`), allocates the run id, registers the active run, slices the source, and spawns a named worker (`lovelace-run-<doc>-<run>`) with a fresh engine factory. It never evaluates source on the caller thread. On `spawn` failure it removes the active entry, marks the handle finished and returns `SubmitError::Spawn`. |
| Worker | Creates one engine on the worker via the factory, sets precision, emits `RunStarted{total}`, then for each statement: checks the cancellation token, emits `StepStarted`, evaluates under `catch_unwind`, drains `take_output` into one `Output` event per line, captures variables/functions/plot, increments committed, emits `StepCommitted` and one `SubProgress`; terminal exits emit exactly one `Finished`, `Cancelled` or `Failed`. |
| Cancellation | `RunHandle`'s token is a state-checked `AtomicBool`; `cancel()` accepts only while the run is still in the host's active map and not finished/cancelled, and is checked at every statement boundary. |
| Snapshot | A coherent `WorkspaceSnapshot` is published **before** the terminal event for `Finished`, `Cancelled` and `Failed`, and `last_snapshot()` returns it. Cancelled/failed snapshots carry only the committed prefix. |
| Panic isolation | `catch_unwind` around the factory, precision, evaluation, output drain and accessors, plus an outer catch in `RunWorker::run`. A panic becomes one `Failed` event carrying the panic text, retires the run so the document is immediately usable, and never poisons the host. |
| `EventSink::apply` | Ignores events whose revision is older than the applied revision; applies `Finished` snapshots (advancing to the snapshot revision) and accepts non-stale non-terminal events without changing state. |
| Diagnostics | `Diagnostic::for_statement` parses the engine's `"position N"` (zero-based offset within the statement), clamps it to the statement text, translates it to an absolute byte offset with `line_column`, and falls back to the statement start when the message has no position. |
| Cleanup | Runs are removed from the active map and the handle marked finished **before** the terminal event is sent, so a `submit` immediately after a terminal event cannot see a stale `Busy`. |
| Warnings | The intentionally host-facing `RunHandle::revision()`/field are marked `#[allow(dead_code)]`; the final build is warning-free. |

### 3.3 One test was wrong and was corrected (with justification)

`execution::tests::real_engine_executes_sliced_statements_and_snapshots_variables` expected

```rust
assert_eq!(snapshot.variables.len(), 2);
... ("a", "2"), ("b", "3")
```

for the **real** engine. That is inconsistent with the workspace contract and with the engine's own
reference behaviour:

* `rust/CONTRACT.md:94`: `pub fn variables(&self) -> Vec<(String, String)>; // name, typed display (sorted)`.
  The real engine correctly returns typed displays such as `"2 (Natural)"`.
* `rust/lovelace-lang/src/lib.rs:787-796`: `variables()` maps each value through `v.typed()`.
* `rust/lovelace-lang/src/suite_parity_batch.rs:4486-4496` (reference parity
  `Evaluate_GivenResult_StoresInUnderscoreVariable`) asserts that evaluating a top-level expression
  stores its display under `_` with `"2 (Natural)"`. The script `a = 2\nb = a + 1\nb\n` therefore
  leaves exactly `_`, `a`, `b`.

I changed only that test's real-engine branch to assert the contract exactly: length 3 and the
three `(name, typed display)` pairs `_`/`3 (Natural)`, `a`/`2 (Natural)`, `b`/`3 (Natural)`. This is
**stronger** (it checks the `_` entry and the typed format explicitly), not a relaxation. The
fake-engine branch is preserved unchanged under `#[cfg(not(feature = "real-engine"))]`; the
filtered `--no-default-features` run passes (`1 passed; 80 filtered out`). No other test assertion,
tolerance or expected value was changed.

### 3.4 Observability only

Two `eprintln!` lines were added where the tests already measured wall-clock time (`T2 submit
returned in …`, `T2 RunStarted received …`, `T4 cancel-to-terminal took …`). They do not assert
anything and do not affect pass/fail; they exist so the report can quote the tests' own numbers.

---

## 4. Final observed result

Final run of the command at the top of this report (default features, no `--nocapture` needed for
the summary):

```text
running 81 tests
...
test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
CARGO_EXIT=0
```

No warnings. No tests were skipped, deleted, ignored or filtered in the acceptance run. There are no
`todo!()`/`unimplemented!()` left in the module.

### T2 / T4 wall-clock numbers measured by the tests

Final `--nocapture` run (thresholds are the tests' own assertions):

| Test | Measurement | Observed | Bound |
|---|---|---|---|
| T2 `t2_submit_does_not_block_and_run_started_arrives_quickly` | `submit()` return | **154 µs** (an earlier run: 70.1 µs) | < 100 ms |
| T2 | `RunStarted` received after submit | **276.1 µs** (earlier: 299.9 µs) | < 100 ms |
| T4 `t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot` | cancel accepted → `Cancelled` terminal | **20.21 ms** (earlier: 20.55 ms) | < 2 s |

T4 is a 200-statement script with a 20 ms sleep per evaluation (4 s of work if uncancelled); the run
stopped after the first committed statement, within about one statement boundary. T5
(`duplicate submit Busy while another document finishes`) takes about 1.5 s of its 5 s bound.

---

## 5. Limits / could not verify

* **Unit-test level only.** The Win32 shell/GUI was not launched or screen-checked. `ExecutionHost`
  is currently referenced only by `execution.rs` tests; `app.rs`/`win32/*` were out of scope, so the
  end-to-end wiring that guarantees the UI thread never evaluates is **not** demonstrated here —
  only the core that makes such wiring possible is tested.
* **Cancellation is cooperative at statement boundaries.** The `Evaluator` API has no cancellation
  hook, so a single long-running `eval` cannot be interrupted; T4 observes cancellation between
  statements. A script with one 10-minute statement would still exceed 2 s.
* **Timing is one debug build on this machine.** T2/T4 numbers vary run to run (submit was observed
  between 70 µs and 154 µs); they are far below the asserted bounds but are not a hard real-time
  guarantee. No release-mode benchmark was run.
* **No stress/fuzz/loom testing.** T5 covers two documents and T3/T8 cover two sequential runs; no
  test creates dozens of documents or hammers `submit`/`cancel`/`next_event` concurrently.
* **Panic hook output.** The default Rust panic hook still prints the panic to stderr when an engine
  panics; the host survives and the test passes, but the process output is noisy. Suppressing it
  would require a global hook and was not done.
* **Real-engine variables contract.** The real-engine test now asserts the documented typed display
  and the `_` entry. The fake engine intentionally returns plain display; the two engines' variable
  strings differ, which is existing behaviour outside this round's scope.
* **`--no-default-features`** was verified only with the single real-engine test (`1 passed`), not
  with the full suite; the acceptance command uses default features.
* **Not verified:** external corpus parity, release build, GUI thread-affinity checks, memory/thread
  leakage beyond test teardown.

---

## 6. Evidence index

* Failing first run: §1 (72 passed / 9 failed; nine execution failures, all traced to the
  `submit/cancel/apply` stubs and the diagnostic stub).
* Recovered 7 297-byte slicer: §2 (stub at lines 63–66; tests byte-identical to current).
* Test correction justification: §3.3 (`CONTRACT.md:94`; `lovelace-lang/src/lib.rs:787-796`;
  `suite_parity_batch.rs:4486-4496`).
* Final run, summary line and T2/T4 numbers: §4.
* Files changed: `rust/lovelace-desktop/src/execution.rs`, this report. `statements.rs` verified,
  not modified by this session; `main.rs` not modified.
