# Round 36 — RUN rows, evidence and verdicts

Status: **10 PASS, 2 NOT DONE** (RUN-005, RUN-007). Desktop suite on the final tree:
`test result: ok. 395 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.93s`.

Scope of this round's changes: `rust/lovelace-desktop/**` (workbench, run-state, execution,
engine, ui). Baseline for "what existed before" is commit **ef17854** (the commit before this
round's changes). During the round a concurrent session committed the whole tree as **e74dc64**;
the working tree was not reverted or edited outside this scope, and the final verification below
was run on that tree.

All commands use the fixed environment:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

---

## 1. The starting failure, reproduced before anything was changed

### 1.1 Full suite

Command (as above). Observed:

```text
failures:
    workbench::tests::editing_during_a_run_marks_the_result_stale_and_never_presents_it_as_current

test result: FAILED. 394 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.67s
error: test failed, to rerun pass `-p lovelace-desktop --bin lovelace-desktop`
```

Log: `docs/goal-desktop-v4/round-36/cargo-test-repro-full.log` (71,456 bytes, UTF-16 from the
Windows PowerShell 5.1 host; readable with `Get-Content`).

### 1.2 The failing test alone

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml" `
  workbench::tests::editing_during_a_run_marks_the_result_stale_and_never_presents_it_as_current -- --exact
```

Observed assertion failure (verbatim):

```text
thread 'workbench::tests::editing_during_a_run_marks_the_result_stale_and_never_presents_it_as_current' (35432) panicked at lovelace-desktop\src\workbench.rs:5890:9:
assertion `left == right` failed: exactly one terminal run event for the discarded run
  left: 2
 right: 1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 394 filtered out; finished in 0.00s
```

Log: `docs/goal-desktop-v4/round-36/cargo-test-run012-repro.log` (1,453 bytes).

### 1.3 What the state machine actually contained

Before touching the assertion I printed the document's transition log from inside the test (a
temporary `eprintln!`, removed in the same edit that fixed the assertion) and re-ran it with
`--nocapture`. Observed:

```text
DIAGNOSTIC transitions: [(Some(RunId(1)), Idle, Queued), (Some(RunId(1)), Queued, Running),
(Some(RunId(1)), Running, Finished), (Some(RunId(2)), Finished, Queued),
(Some(RunId(2)), Queued, Cancelling), (Some(RunId(2)), Cancelling, Cancelled),
(Some(RunId(2)), Cancelled, Stale)]
```

Run 1 has exactly one terminal event (`Finished`); the discarded run 2 has exactly one terminal
event (`Cancelled`) immediately followed by `Stale`. The product behaviour was correct.

### 1.4 Which side was wrong

`terminal_run_event_count()` is documented as the number of terminal run events **in the whole
document log** (`rust/lovelace-desktop/src/run_state.rs:211-218`), and the two runs in this test
were already 2 runs in one document. The test message said "for the discarded run" while the
assertion measured the document. The product did not fail: the discarded run ends with its one
`Cancelled` event and the display moves to `Stale`.

The specified behaviour is the RUN-012 row (`docs/goal-desktop-v4/requirements.md:245`):

> | **RUN-012** | P0 | Q | Protect against stale results after source edits | Result event includes snapshot revision; editing while run active never falsely presents stale result as current; user can inspect old result with label. |

That row says nothing about a per-document terminal-event total. The invariant it rests on is
the one in `run_state.rs`'s module contract: "Exactly one terminal run event (`Finished`,
`Cancelled` or `Failed`) is accepted per run". **The test's expectation was wrong-scoped, not the
product.** I corrected the test and did not weaken an assertion:

- from `terminal_run_event_count() == 1` (document-wide, ambiguous), to:
  - exactly **1** terminal event whose `run` id is the discarded run's (stronger: it names the run),
  - **and** exactly **2** terminal events in the whole log (one per run; also catches a duplicated
    or lost terminal event).
- Every other assertion in the test is unchanged. No assertion was deleted, skipped or relaxed.

Corrected assertions (excerpt), `rust/lovelace-desktop/src/workbench.rs:5891-5918`:

```rust
let discarded_run = document.transitions().get(cancelled)
    .and_then(|transition| transition.run)
    .expect("the discarded terminal transition names its run");
let discarded_terminals = document.transitions().iter()
    .filter(|transition| transition.run == Some(discarded_run)
        && transition.to.is_terminal_run_event())
    .count();
assert_eq!(discarded_terminals, 1, "exactly one terminal run event for the discarded run ...");
assert_eq!(document.terminal_run_event_count(), 2,
    "the log keeps run 1's Finished and the discarded run's Cancelled, and nothing else: ...");
```

Observed after the correction:

```text
test workbench::tests::editing_during_a_run_marks_the_result_stale_and_never_presents_it_as_current ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 394 filtered out; finished in 0.00s
```

Log: `docs/goal-desktop-v4/round-36/cargo-test-run012-after.log` (859 bytes).

**No product change was needed for this failure.** The RUN-012 product work (discarded result
leaves `Stale`, label kept for inspection) was already present in the tree from the earlier
part of this round and passed every other assertion of the test. One small build-warning fix was
made while verifying the window build: `set_active_precision` is now marked
`#[allow(dead_code)]` (numeric setter kept for hosts; the toolbar uses
`set_active_precision_text`), so `cargo build` is warning-free.

---

## 2. Verdict summary

| Row | Verdict | One-line basis |
| --- | --- | --- |
| RUN-004 | **PASS** | Mid-run status/log carry `2/3`, the statement label and `reused 2` (real engine). |
| RUN-005 | **NOT DONE** | The engine publishes no kernel sub-progress; the probe pins the gap. |
| RUN-007 | **NOT DONE** | Failure prefix is compared with `REF-error.json`; no web Studio cancellation capture exists, so the cancellation half is unproved. |
| RUN-010 | **PASS** | Second Run refused with a visible advisory; run untouched, no extra Queued, one terminal event. |
| RUN-011 | **PASS** | Two documents in flight at once; cancelling one leaves the other finishing 5/5. |
| RUN-012 | **PASS** | Discarded result ends `Cancelled` then `Stale`, old snapshot stays labelled; test corrected, window capture added. |
| RUN-013 | **PASS** | Failure keeps the 2-statement prefix, no half write, no duplicate print; retry completes. |
| RUN-014 | **PASS** | Code, category, message, line, column and suggestions reach diagnostic, log and snapshot; jump has a round-34 window proof. |
| RUN-016 | **PASS** | 10,005 events drained in 341 frames, max frame 10.44 ms, log order intact. |
| RUN-017 | **PASS** | Panicking engine ends Failed, releases the lock, keeps the buffer and runs again. |
| RUN-019 | **PASS** | Invalid precision rejected locally with range guidance; valid value applied; control visible in the window. |
| RUN-020 | **PASS** | `setprecision(20)` moves the document/status/snapshot precision and invalidates the cache; reuse resumes. |

---

## 3. Per-row accounting

The "before" column is the ef17854 blob of the named file (marker counts from
`git show ef17854:<file>`); "changed" is the round-36 diff (also saved as
`docs/goal-desktop-v4/round-36/workbench-round36.diff`, 59,750 bytes).

| Row | Requirement (requirements.md) | What existed before (ef17854) | What changed in round 36 | Test or artifact path | Observed result | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| RUN-004 | Expose step-level progress: UI shows completed/total statements, current label and reused count, with stable progress and cancellable status. | Status strip was `doc · rev · precision · Ln,Col · run state` only; no current label, no progress segment, no reused mark on log lines (`current_label`, `progress_text` 0 matches). | `WorkbenchDocument::current_label` from `StepStarted`; `progress_text()` = `completed/total · label · reused N`; status-strip progress segment; `mark_pending_reused` appends `(reused)` to the buffered statement line; `DocumentView::progress`. | `rust/lovelace-desktop/src/workbench.rs:5597` (real engine); window status in `docs/goal-desktop-v4/round-36/window/scenario-final-uia.txt:29`. | `test ... run_progress_shows_completed_total_label_and_reused_count_mid_run ... ok`. Mid-run: progress starts `2/3`, label contains `matmul`, progress contains `reused 2`, status contains `run Running` + the segment; after: `3/3 · reused 2`, current label cleared. Window capture shows the progress segment live (`1/1 · reused 0` at Finished and at Stale). | **PASS** |
| RUN-005 | Expose numeric subprogress: long sqrt/pi/factorial or comparable operations publish human-readable phase and fraction; progress stays meaningful and monotonic where defined. | No sub-progress seam at all: the engine emits only the committed/total fraction after a statement; `SubProgress` carried no phase. | Nothing in the product (row not implemented); a probe test was added that pins the gap. | `rust/lovelace-desktop/src/execution.rs:2305`; metrics `docs/goal-desktop-v4/round-36/cargo-test-round36-metrics.log`. | `RUN-005 probe: sqrt(2) at 50000 digits took 1227 ms with 0 event(s) between StepStarted and StepCommitted; the only SubProgress was label="1/1" fraction=1; REF-digits/REF-precision publish subProgress=1 subLabel=sqrt`. The test's own doc says "RUN-005 evidence, not a pass". | **NOT DONE** — the engine has no per-kernel phase/fraction callback; the desktop reports statement granularity only. Web Studio publishes `subProgress=1, subLabel=sqrt`; matching it needs an engine seam, which is out of this tree's scope. |
| RUN-007 | Specify cancellation state semantics: contract defines which completed statements remain committed and what happens to in-flight statement; tests compare with web Studio contract. | `RunStateMachine` already defined Queued/Running → Cancelling → Cancelled and the committed prefix on cancellation (module docs and cancel tests); no comparison with the web Studio oracle at all (`REF-error` 0 matches in ef17854). | New test that parses `reference/REF-error.json` and compares the failure's committed prefix, missing write, diagnostic count and line with the desktop. | `rust/lovelace-desktop/src/workbench.rs:6435`; oracle `docs/goal-desktop-v4/reference/REF-error.json`; existing cancellation tests `workbench.rs:3481`, `workbench.rs:4630`, `run_state.rs:478`. | `test ... the_failed_run_matches_the_oracle_ref_error_prefix ... ok`: oracle and desktop both keep `y`, both omit `z`, one diagnostic at line 2. | **NOT DONE** — the comparison covers the **failure** path (`REF-error` is status `error`), not cancellation. Every `REF-*.json` in the tree reports `final.status` as `finished` or `error` (checked all 13, including the top-level `final` of `REF-side-effects.json`), so there is no web Studio capture of a cancelled run to compare against; cancellation semantics are proven only by the desktop's own tests. The row's second half is therefore unproved. |
| RUN-010 | Reject duplicate active runs per tab: pressing Run during an active run cannot corrupt engine; explicit queue/reject behavior is observable. | Refusal only logged the line `A run is already in progress.`; no document advisory, no status-strip visibility (`run_refusal` 0 matches). | `WorkbenchDocument::run_refusal`, the sentence `Run refused: a run is already in progress (<state>). The running job was not disturbed.`, status-strip segment, log line; cleared when a run is accepted or ends; the host-busy refusal uses the same channel. | `rust/lovelace-desktop/src/workbench.rs:5678`. | `test ... run_during_a_run_is_refused_with_a_visible_advisory_and_does_not_disturb_the_run ... ok`: second Run returns false, run id unchanged, advisory text present in status strip and log, zero new `Queued` transitions, exactly one terminal event, result `ok` with 4 committed statements, advisory cleared at close. | **PASS** |
| RUN-011 | Allow multi-tab concurrency: Tab A long calculation does not prevent tab B execution or cancellation. | Multi-document workbench existed, but tab labels were title only (`tab_label` 0 matches) and there was no concurrent-run test. | `WorkbenchDocument::tab_label()` appends the run-state label; test drives two documents at once. | `rust/lovelace-desktop/src/workbench.rs:5754`; `tab_label` at `workbench.rs:331`. | `test ... two_documents_run_concurrently_and_one_cancels_while_the_other_finishes ... ok`: after `poll()` both documents are in flight; cancelling document 2 gives `Cancelling` then `Cancelled`; document 1 finishes `Finished` with `(5, "ok")`; tab labels contain `Finished` and `Cancelled`; `is_running()` false. | **PASS** |
| RUN-012 | Protect against stale results after source edits: result event includes snapshot revision; editing while run active never falsely presents stale result as current; user can inspect old result with label. | An edit during a run discarded the terminal result and left the display in `Cancelled`; `Finished → Stale` existed only for an edit after a finished run. No `abandon()` edge, `DocumentView` had no `buffer_revision`, and the result pane printed `rev N · committed C/T` with no stale label (`STALE` 0 matches in the ef17854 ui). | `RunStateMachine::abandon()` (Queued/Running/Cancelling/Cancelled/Failed → Stale); `discard_terminal` applies the one terminal `Cancelled`, then `abandon`, logs `Run result discarded` and the stale notice; `DocumentView::buffer_revision`; result pane label `· STALE: not the current source`; tab label names `Stale`. Test assertion corrected as described in section 1 (test only; product unchanged). | `rust/lovelace-desktop/src/workbench.rs:5829` (corrected), `:5949`, `:3881` (updated expectation); `run_state.rs:294`; `ui/app.rs:2126-2145`; window `docs/goal-desktop-v4/round-36/window/scenario-final-uia.txt:26,29,52,575,577` + `scenario-final.png` (171,782 bytes). | All three tests `... ok`. Transitions: run 1 `Idle→Queued→Running→Finished`; run 2 `Finished→Queued→Cancelling→Cancelled→Stale`; state `Stale`; 1 terminal event for the discarded run, 2 in the whole log; old snapshot still `ok`; displayed revision 1 < buffer revision 2; status contains `run Stale`; log contains `Run result discarded` and `stale`; exactly one `[result] = ok` line. First-run case: `Stale`, no snapshot, one Stale transition. Window: tab `untitled.lv * · Stale`, status `run Stale`, result pane `rev 5 · committed 1/1 · STALE: not the current source`, log `[result] = 1` and `The displayed result is stale`. | **PASS** |
| RUN-013 | Preserve partial results safely: on error or cancellation, completed-statement workspace and outputs remain consistent; no half-written variable or duplicate print line. | The execution core already published the committed prefix on a failed run, and cancellation tests existed, but there was no test that pinned the failure's variables, outputs and retry in one place. | New evidence test; the failed snapshot now renders the full diagnostic (`render_fully`, from RUN-014). | `rust/lovelace-desktop/src/workbench.rs:5976`; cancellation side `workbench.rs:3481`, `:4630`. | `test ... a_failure_keeps_the_committed_prefix_and_never_half_writes ... ok`: `Failed`; snapshot committed 2; `y` present, `b` absent; output exactly `["kept"]`; `[stdout] kept` exactly once; log `Run failed after 2 committed statements`; no `statement 4/4`; edit clears the decoration; corrected run `Finished` with 4 committed statements and `b = 99`. | **PASS** |
| RUN-014 | Present structured errors and recovery: diagnostics display code, category, message, line, column and suggestions; user can jump to source and retry after correction. | `Diagnostic` carried message, line and column only; no engine code/category/suggestions seam (`last_error` absent); log line was `Diagnostic: {render()}`. | `EngineError` + `Evaluator::last_error()` seam, captured by `RealEngine`; `Diagnostic::{code, category, suggestions, classify, render_fully}`; log and failed snapshot use `render_fully`; editor header shows `code (category) · Ln · Col · message`. | `rust/lovelace-desktop/src/workbench.rs:6060`; `execution.rs:361`; `engine.rs:341`; `ui/app.rs:3397`; jump: `docs/goal-desktop-v4/journeys.md` J-12 **PASS (round 34)** with `J-12.png` / `J-12-jump.png`. | `test ... a_diagnostic_presents_code_category_message_line_column_and_suggestions ... ok`: `code = UnknownFunction`, `category = UnsupportedOperation`, message contains `Unknown function 'sqrtt'` and `Did you mean 'sqrt'?`, `(line, column) = (2, 1)`, `suggestions = ["sqrt"]`, `render_fully` names every field, the log carries them, and the failed snapshot text contains code + category. Retry after correction is proven by RUN-013's corrected run; jump is the round-34 J-12 window capture (F8 → Ln 2, Col 11). | **PASS** |
| RUN-016 | Prevent UI starvation from output floods: 10,000 emitted log events do not freeze editor; visible log updates batch without changing log order or final integrity. | The nonblocking poll and its `PollReport` (calls, applied, next_event_max) and the log cap already existed at ef17854; there was no flood test. | Nothing in the product; the test measures the existing batching path. | `rust/lovelace-desktop/src/workbench.rs:6118`; metrics `docs/goal-desktop-v4/round-36/cargo-test-round36-metrics.log` (1,884 bytes). | `test ... ten_thousand_log_lines_do_not_freeze_the_event_loop ... ok`. Printed metrics: `RUN-016 flood: submit 171 us, 10005 events applied in 341 frame(s), max poll pass 0.14 ms, max frame 10.44 ms, run 11.60 ms, retained 500 lines, last 9999`; flood lines are contiguous through 9,999, no duplicates or gaps; retained length is `LOG_CAP` (500). | **PASS** (headless workbench/view-level: it assembles the view and renders every log line each frame; no window freeze capture). |
| RUN-017 | Handle engine panics and worker failures gracefully: a simulated worker failure produces recoverable error state, releases run lock and preserves buffer/recovery files; process remains usable. | `panic::catch_unwind` around engine construction, eval and every accessor already existed at ef17854; there was no opt-in fault switch and no test proving the workbench path. | New `rust/lovelace-desktop/src/engine/fault.rs` fault evaluator, selected by `LOVELACE_ENGINE_FAULT=panic-once` in both default-factory builds; test. | `rust/lovelace-desktop/src/workbench.rs:6232`; `rust/lovelace-desktop/src/engine/fault.rs`; factory switch `engine.rs:579-600`. | `test ... a_panicking_engine_is_recovered_and_leaves_the_buffer_usable ... ok`: state `Failed`; `!is_running()` (lock released); source unchanged; diagnostic contains `simulated engine panic`; snapshot committed 0; log `Run failed after 0 committed statements`; a second run is accepted and finishes `ok` with 2 committed statements. Recovery-journal file behaviour is not re-driven by this test; it is covered by the existing `recovery::tests` suite, and the panic path leaves the buffer and its dirty state intact. | **PASS** |
| RUN-019 | Expose precision controls on the execution bar: one visibly labeled input sets computation/display precision for the active session; invalid values fail locally with clear guidance. | Toolbar used an `egui::DragValue` spinner clamped to `MIN..=MAX`, so invalid input was silently clamped; no validation message and no text entry (`set_active_precision_text` 0 matches). | `parse_precision()` (empty / non-numeric / out-of-range / too-large guidance), `Workbench::set_precision_text` / `set_active_precision_text`, toolbar `TextEdit` + `Apply` + error label, `DocumentView::precision`. | `rust/lovelace-desktop/src/workbench.rs:6307`, `:2459`, `:1439`; toolbar `ui/app.rs:1623-1665`; window `window/scenario-final-uia.txt:18-19`. | `test ... the_precision_input_rejects_invalid_values_with_guidance ... ok`: `""`, `abc`, `-5`, `1.5`, `0`, `100001`, and a 26-digit value are each rejected with `from 1 to 100000` guidance and leave the precision unchanged (7 guidance log lines); `" 250 "` returns `Ok(250)`, document precision 250, status contains `precision 250`. Window: the `Precision` label and the `Display precision for the active document: a whole number of digits from 1 to 100000` edit field are present in the UIA tree. | **PASS** (validation is headless-proven; the control's presence is window-proven). |
| RUN-020 | Synchronize script-set precision: a `setprecision(n)` statement updates toolbar, engine, cache invalidation and affected value presentation consistently. | Precision was submitted to the engine but the snapshot carried no precision; a script `setprecision` changed the engine only, so the document/toolbar stayed on the submitted value and the next run reused the stale precision. | `WorkspaceSnapshot::precision`, `Progress::precision` captured at every commit, `adopt_run_precision` + log line, `DocumentView::precision`, toolbar sync via `precision_seen`. | `rust/lovelace-desktop/src/workbench.rs:6362`; `execution.rs:105`, `:926`, `:1074`; `workbench.rs:2138`; `ui/app.rs:610-619`. | `test ... a_script_setprecision_updates_the_toolbar_precision_and_invalidates_the_cache ... ok`: document precision becomes 20; status contains `precision 20`; snapshot precision 20; `sqrt(2)` renders 19 fractional digits; log contains `Script set precision`; run 2 reuses nothing (`[false, false]`, result unchanged); run 3 resumes cache use (`[false, true]`, reused 1). | **PASS** |

---

## 4. Commands and raw outputs

### 4.1 Final full suite (required command)

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed final line (log `docs/goal-desktop-v4/round-36/cargo-test-green-final.log`, 35,163 bytes):

```text
test result: ok. 395 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.93s
```

`real-engine` is a default feature (`rust/lovelace-desktop/Cargo.toml:28-31`), so the
`#[cfg(feature = "real-engine")]` tests above all ran in this suite.

Round-36 tests observed in that run (all `ok`):

```text
test workbench::tests::run_progress_shows_completed_total_label_and_reused_count_mid_run ... ok
test workbench::tests::run_during_a_run_is_refused_with_a_visible_advisory_and_does_not_disturb_the_run ... ok
test workbench::tests::two_documents_run_concurrently_and_one_cancels_while_the_other_finishes ... ok
test workbench::tests::editing_during_a_run_marks_the_result_stale_and_never_presents_it_as_current ... ok
test workbench::tests::an_abandoned_first_run_leaves_stale_and_publishes_nothing ... ok
test workbench::tests::stale_events_from_an_abandoned_run_are_discarded ... ok
test workbench::tests::a_failure_keeps_the_committed_prefix_and_never_half_writes ... ok
test workbench::tests::a_diagnostic_presents_code_category_message_line_column_and_suggestions ... ok
test workbench::tests::ten_thousand_log_lines_do_not_freeze_the_event_loop ... ok
test workbench::tests::a_panicking_engine_is_recovered_and_leaves_the_buffer_usable ... ok
test workbench::tests::the_precision_input_rejects_invalid_values_with_guidance ... ok
test workbench::tests::a_script_setprecision_updates_the_toolbar_precision_and_invalidates_the_cache ... ok
test workbench::tests::the_failed_run_matches_the_oracle_ref_error_prefix ... ok
test execution::tests::t10_a_long_sqrt_kernel_publishes_no_kernel_sub_progress ... ok
```

The test count went 382 (ef17854) to 395: **13 new tests** (12 in `workbench.rs`, 1 in
`execution.rs`), and one pre-existing test (`stale_events_from_an_abandoned_run_are_discarded`,
`workbench.rs:3881`) was updated from `Cancelled` to `Stale` to match RUN-012.

### 4.2 Metric runs (printed values cited in the table)

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml" -- --nocapture ten_thousand_log_lines
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml" -- --nocapture t10_a_long_sqrt_kernel
```

Both print `test result: ok. 1 passed; 0 failed` and the metric lines quoted in rows RUN-016 and
RUN-005; full output in `docs/goal-desktop-v4/round-36/cargo-test-round36-metrics.log`.

### 4.3 Artifact inventory

| Path | Bytes | What it is |
| --- | --- | --- |
| `docs/goal-desktop-v4/round-36/cargo-test-repro-full.log` | 71,456 | Full-suite reproduction: 394 passed, 1 failed. |
| `docs/goal-desktop-v4/round-36/cargo-test-run012-repro.log` | 1,453 | The failing test alone: `left: 2 right: 1`. |
| `docs/goal-desktop-v4/round-36/cargo-test-run012-after.log` | 859 | Corrected test: 1 passed. |
| `docs/goal-desktop-v4/round-36/cargo-test-green.log` | 35,067 | Intermediate green run (395 passed) before the `allow(dead_code)` cleanup. |
| `docs/goal-desktop-v4/round-36/cargo-test-green-final.log` | 35,163 | Final green run (395 passed, 0 failed). |
| `docs/goal-desktop-v4/round-36/cargo-test-round36-metrics.log` | 1,884 | RUN-005 and RUN-016 printed metrics. |
| `docs/goal-desktop-v4/round-36/workbench-round36.diff` | 59,750 | `git diff` of the round-36 workbench changes. |

---

## 5. Window capture (step 5)

**The app can take the foreground in this session.** The existing, out-of-scope harness
`tools/gui-harness.ps1` was run (read-only use; it is not edited here) with a fresh `APPDATA`
sandbox. It found the real window (`Lovelace Desktop`, 1296x839) and a populated UIA tree
(73 descendants when first ready, 76 in the scenario summary) and drove the stale gesture.

```powershell
$env:APPDATA="$repo\docs\goal-desktop-v4\round-36\appdata-stale"
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$repo\tools\gui-harness.ps1" `
  -Action scenario -Flow stale -FastStatus -Keys 'a = 1' `
  -Exe "$repo\rust\target-desktop\debug\lovelace-desktop.exe" `
  -OutDir "$repo\docs\goal-desktop-v4\round-36\window" `
  -Log "$repo\docs\goal-desktop-v4\round-36\window\driver.log"
```

Observed (driver log, 6,899 bytes):

```text
scenario window mainhandle=4723292 hwnd=66783486 title='Lovelace Desktop - untitled.lv *' rect=1296x839 ...
scenario uia ready hwnd=66783486 nodes=73 after=140ms
scenario transition phase=ctrl+enter elapsed=67.3ms state='... run Queued'
scenario transition phase=ctrl+enter elapsed=212.8ms state='... run Finished  |  1/1 · reused 0'
scenario observed Finished elapsed=212.8ms state='... run Finished  |  1/1 · reused 0'
scenario transition phase=stale-edit elapsed=154.6ms state='... run Stale  |  1/1 · reused 0'
scenario flow=stale finished_ms=212.8 stale_ms=154.6 stale=True edit_send_ms=29.1
SCENARIO_OK FLOW=stale ATTEMPTS=1 CAPTURED=0 REQUESTED=0
```

Window artifacts, from `docs/goal-desktop-v4/round-36/window/`:

| File | Size | Evidence |
| --- | --- | --- |
| `scenario-final.png` | 171,782 bytes, valid PNG (`89 50 4E 47`) | The painted window after the edit. |
| `scenario-final-uia.txt` | 38,358 bytes, 579 UIA nodes | L26 tab `untitled.lv * · Stale`; L29 status `… run Stale … 1/1 · reused 0`; L52 result pane `rev 5 · committed 1/1 · STALE: not the current source`; L575 `[result] = 1`; L577 `[info] The displayed result is stale: the source changed after it finished.` |
| `scenario-summary.md` | 2,837 bytes | Sampler and transition table (Finished 212.8 ms, Stale 154.6 ms). |

What this capture proves, honestly:

- **RUN-012**: the stale result is labelled in the window (`STALE: not the current source`), the
  status strip says `run Stale`, the tab says `Stale`, and the earlier result stays inspectable.
- **RUN-004**: the progress segment (`1/1 · reused 0`) is visible in the window's status strip.
  The mid-run `2/3` + current label + `reused 2` state is proven by the workbench test only.
- **RUN-019**: the window shows the labelled `Precision` input and its full guidance hover text.
  The rejection/apply behaviour is proven by the workbench test only.
- **RUN-014**: the code/category/suggestions fields added this round are headless-proven; the
  jump-to-source half was window-proven in round 34 (J-12, `J-12.png` / `J-12-jump.png`), which
  predates the new fields.

The other rows (RUN-005, RUN-007, RUN-010, RUN-011, RUN-013, RUN-016, RUN-017, RUN-020) have
**no window artifact** in this round; they are headless-proven by the tests cited above. RUN-011
needs two live documents driven simultaneously and RUN-005 is a known engine gap, so neither can
be usefully captured with the current single-document scenario.

---

## 6. NOT DONE — the honest gaps

### RUN-005 — numeric sub-progress is not implemented

The probe (`execution.rs:2294-2396`) is deliberately named "evidence, not a pass". A 50,000-digit
`sqrt(2)` runs 1,227 ms and publishes **zero** events between `StepStarted` and `StepCommitted`;
the only `SubProgress` is the committed fraction `1/1`. The web Studio reference records
`subProgress=1, subLabel=sqrt` for the same class of kernel (`REF-digits`, `REF-precision`).
Closing this row needs an engine-side phase/fraction callback; the desktop cannot invent one.

### RUN-007 — the cancellation comparison with web Studio is missing

The new oracle test proves the **failure** prefix against `REF-error.json` (keeps `y`, does not
write `z`, one diagnostic at line 2). It does not prove the **cancellation** half: there is no
`REF-*` capture of a cancelled run in the tree (all 13 captures report `final.status` `finished` or `error`), so
the desktop's cancellation semantics are compared only with its own state-machine contract and
tests (`cancellation_keeps_the_committed_prefix`, `cancelling_after_statement_two_keeps_the_same_coherent_prefix`).
The row says "tests compare with web Studio contract"; until a web Studio cancellation capture
exists, that half stays unproved.

---

## 7. Caveats and provenance

- Every number and output in this document was observed in this session; nothing is inferred or
  copied from a prior round except the explicitly labelled round-34 J-12 capture.
- The two NOT DONE rows are marked NOT DONE even though the tests that touch them pass: a passing
  gap-pin (RUN-005) and a passing failure-oracle (RUN-007) are not proof of the row.
- The round-36 logs `cargo-test-1.log` and `cargo-test-2.log` are the previous implementer's
  UTF-8 transcripts and `cargo-test-baseline.log` is UTF-16; all three are left untouched. The
  logs cited above were written by this session (UTF-8 except `cargo-test-repro-full.log`, which
  the Windows PowerShell host wrote as UTF-16; `Get-Content` reads it).
- HEAD moved from ef17854 to e74dc64 during the round: a concurrent session committed the whole
  tree (including this round's desktop changes and this docs folder's diff). No file outside
  `rust/lovelace-desktop/**` and `docs/goal-desktop-v4/round-36/**` was modified by this work.
