# LOG-001: print output captured exactly once, in order, byte-exactly

Requirement: print output is captured exactly once, in order, byte-exactly, across repeated
events, reruns and cancellation. Four output tests already existed; two acceptance clauses were
not covered: byte-sensitive line endings, and no duplication when a run is cancelled after some
prints (or when an event is delivered twice).

This round adds **five tests** and fixes **one real consumer defect** the repeated-delivery test
exposed. No product feature was added. No existing test was weakened, skipped or deleted.

## What existed before this round

| Test | Pinned |
| --- | --- |
| `engine::fake::tests::print_collects_output_and_returns_void` | the fake engine appends one entry per `print()` and `take_output` drains |
| `engine::tests::eval_full_snapshots_output_and_variables_in_one_step` | `eval_full` snapshots the drained output with the variables |
| `execution::tests::t8_output_events_are_exactly_the_printed_lines_and_do_not_accumulate_across_runs` | one `Output` event per printed line for a two-line engine, twice; the snapshot holds the same two lines each run |
| `workbench::tests::print_output_reaches_the_log_and_the_snapshot` | a fake-engine print reaches an `Output` log line and `snapshot.output` |

None of them asserted on bytes (they compared `String`s), none replayed an event, and none
cancelled a run that had already printed.

## The new tests

### 1. Byte-sensitive line endings

`execution::tests::t10_output_events_preserve_crlf_lf_and_bare_cr_bytes_in_order` (new)

The script embeds the control bytes **literally inside the string literals**:

```text
print("a<CR><LF>b")
print("c<LF>d")
print("e<CR>f")
```

(`<CR>` = `0x0D`, `<LF>` = `0x0A`; they are not escape sequences in the source.) The desktop
slicer splits only outside strings and `Statement::text()` trims only the statement ends, so the
bytes reach the engine and come back unchanged. The test:

1. evaluates the same script on a second `FakeEngine` and takes its `take_output()` as the
   expected engine bytes (the assertion is event bytes == engine bytes, not a hand-written
   rendering);
2. runs the script through `ExecutionHost` and collects the `Output` event texts;
3. compares, for every line, `event_text.as_bytes()` with `engine_line.as_bytes()`;
4. compares the concatenated `Vec<u8>` of all events with the concatenated engine bytes, and
   with the literal expected sequence.

Expected and observed concatenation:

```text
a \r \n b c \n d e \r f
61 0D 0A 62 63 0A 64 65 0D 66   (10 bytes)
```

No separator is inserted between print lines and no `\r` is dropped or rewritten: event 0 is
`a\r\nb` (4 bytes), event 1 is `c\nd` (3 bytes), event 2 is `e\rf` (3 bytes), and the terminal
event is `Finished`.

`workbench::tests::print_output_reaches_the_log_and_snapshot_byte_for_byte` (new)

The consumer side of the same script. It asserts that `Workbench` logs exactly three
`LogLevel::Output` lines whose `text.as_bytes()` are `b"a\r\nb"`, `b"c\nd"`, `b"e\rf"`, that
`snapshot.output` is the identical byte sequence in the same order, and that the concatenated log
bytes are `b"a\r\nbc\nde\rf"`. A desktop layer that trimmed, split on `\n` or re-joined the lines
would change the line count or the bytes and fail the test.

### 2. Repeated delivery (and re-draining)

`workbench::tests::replayed_events_do_not_duplicate_log_lines_or_snapshot_output` (new)

The test pulls the **real** `SequencedEvent`s from the workbench's `ExecutionHost` while a
two-statement run is in flight, applies each one through `Workbench::apply_event` (the same
consumer `poll` uses), and replays the first genuine `Output` event and the terminal event a
second time. It then calls `poll()` three more times after the run finished, waits for idle, and
asserts:

- the second delivery is refused (`apply_event` returns false);
- the log has exactly `["line1", "line2"]` as `Output` lines - each once, in order;
- `snapshot.output` is exactly `["line1", "line2"]`;
- `terminal_run_event_count() == 1`.

This test failed before the fix (see below).

### 3. Cancellation after prints

`workbench::tests::cancelling_after_the_first_print_keeps_each_printed_line_exactly_once` (new)

Script:

```text
print("one")
print("two")
print("three")
spin()
```

The test engine prints one line for each of the first three statements and then blocks inside a
single `spin()` kernel call that polls the live flag installed by `Evaluator::set_cancel_flag`
(the same seam `ExecutionHost::cancel` sets). The test waits until the first print has been
observed in the log, then until the run is provably inside the kernel, and only then cancels.
At that point all three print statements are committed, so the cancellation path
(`Ok(Err(..))` + `cancel_requested` -> `finish_cancelled`) must publish the committed prefix once.

Observed:

- log `Output` lines are exactly `["one", "two", "three"]`, each string occurring once, in order;
- `snapshot.output` is exactly `["one", "two", "three"]`, `committed_steps == 3`;
- the run state is `RunState::Cancelled`, `terminal_run_event_count() == 1`;
- the log carries `Run cancelled after ...`;
- no prefix line is duplicated.

### 4. Rerun

`workbench::tests::rerunning_the_same_printing_script_logs_exactly_one_line_per_run` (new)

The same script (`print("tick")`) is run twice on the same document. After each run the snapshot
must contain exactly `["tick"]` (one line, not accumulated). At the end the log must contain
**exactly two** `Output` lines with text `tick` - one per run, never three or four.

## Defect found and fixed

The repeated-delivery test failed against the current code. Observed pre-fix failure
(`pre-fix-failure.log`, captured with the watermark temporarily disabled and then restored; the
same test passes in `new-tests.log` and in the full suite below):

```text
running 1 test
test workbench::tests::replayed_events_do_not_duplicate_log_lines_or_snapshot_output ... FAILED
thread 'workbench::tests::replayed_events_do_not_duplicate_log_lines_or_snapshot_output' panicked at
lovelace-desktop\src\workbench.rs:1462:17:
a repeated Output delivery must be ignored, not logged twice
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 173 filtered out
```

Failing assertion: `assert!(!workbench.apply_event(&event), "a repeated Output delivery must be
ignored, not logged twice")`. The second application of the same `SequencedEvent` returned
`true` and appended the print text to the log a second time.

**Fix** (`workbench.rs`): `WorkbenchDocument` now keeps a `last_sequence: u64` delivery
watermark, initialised to 0. `Workbench::apply_event` ignores (returns false) any event whose
`SequencedEvent::sequence` is at or below the document's watermark, and records the sequence of
an accepted event. `ExecutionHost::next_event` numbers every event it hands out with a strictly
increasing sequence, so this drops only a repeated delivery - a replayed `Output` cannot append
a second log line and a replayed `Finished` cannot publish or log twice. No production path
reuses a sequence number; the existing
`workbench::tests::a_duplicate_terminal_event_cannot_produce_a_second_terminal_transition` still
passes.

## Observed test output

`new-tests.log` (each test run individually with `--nocapture`):

```text
test execution::tests::t10_output_events_preserve_crlf_lf_and_bare_cr_bytes_in_order ... ok
test workbench::tests::print_output_reaches_the_log_and_snapshot_byte_for_byte ... ok
test workbench::tests::replayed_events_do_not_duplicate_log_lines_or_snapshot_output ... ok
test workbench::tests::cancelling_after_the_first_print_keeps_each_printed_line_exactly_once ... ok
test workbench::tests::rerunning_the_same_printing_script_logs_exactly_one_line_per_run ... ok
```

## Full suite summary

Command and environment from the round brief:

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

| Run | Before this round | After |
| --- | --- | --- |
| default features | 169 passed; 0 failed (measured in `baseline-cargo-test.log`) | **174 passed; 0 failed; 0 ignored** (`cargo-test.log`) |
| `--no-default-features` (fake engine) | 164 recorded in round 18 for the same configuration | **169 passed; 0 failed; 0 ignored** (`cargo-test-fake-engine.log`) |
| `cargo check -p lovelace-desktop --tests` | - | exit 0, no warnings from this crate (`cargo-check.log`) |

The five new tests are the only test-count change (+5 in both configurations); all pre-existing
tests still pass unchanged.

## Limits kept explicit

- The byte-for-byte assertions cover the desktop's tested output path: the typed `Output`
  events, the workbench log lines and the snapshot's `output`. The egui window renders
  `LogLine.text` without a prefix (`LogLevel::Output`), but the pixel/UI text layer is not
  byte-diffed by these headless tests.
- The cancellation test uses a deterministic test engine whose kernel polls the live cancel
  flag; it proves the desktop's once-only publishing/cancellation path, while the shipped real
  engine's kernel seam is covered separately by
  `execution::tests::t9_cancel_inside_one_kernel_is_terminal_within_two_seconds`.
- The literal CR/LF/CR script is evaluated by `FakeEngine`, the shipped fallback and the
  deterministic engine used by these tests. The desktop does not inspect or rewrite the text in
  between, so the byte guarantee is a property of the desktop boundary, not of any engine's
  string-escape syntax.

## Files

- `rust/lovelace-desktop/src/execution.rs` - new `t10_output_events_preserve_crlf_lf_and_bare_cr_bytes_in_order`.
- `rust/lovelace-desktop/src/workbench.rs` - four new tests, two test engines, and the
  per-document delivery watermark fix in `WorkbenchDocument` / `apply_event`.
- `docs/goal-desktop-v4/round-19/` - this document, `cargo-test.log`,
  `cargo-test-fake-engine.log`, `cargo-check.log`, `new-tests.log`, `pre-fix-failure.log`, and
  the baseline `baseline-cargo-test.log`.
