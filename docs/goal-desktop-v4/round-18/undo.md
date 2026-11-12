# EDT-012: robust per-document undo/redo in the native editor

Requirement: burst typing is coalesced into sensible undo steps; undo/redo restores the text and
the caret/selection; every editing action (typing, delete, line join, paste, completion insert) is
undoable per document, without one document's history ever touching another's.

## What existed before this round

Two unrelated layers existed, and only the legacy one had undo/redo:

| Layer | State before |
| --- | --- |
| The shipped egui workbench (`rust/lovelace-desktop/src/ui/editor.rs`, the default window) | **No undo, no redo, no selection, no history.** `TextEditor` held `lines`, `caret_row`, `caret_col`, `edits`, the diagnostic decoration and the completion session. The key loop handled Text/Paste/Backspace/Delete/Enter/Tab/Escape/arrows/Home/End and nothing else. `git grep -E 'undo\|redo' HEAD -- rust/lovelace-desktop/src/ui` returns **no match**, and `git grep 'selection' HEAD -- rust/lovelace-desktop/src/ui/editor.rs` finds only completion-list navigation - there was no text selection at all. Ctrl+Z in the default window did nothing; typing, paste, delete, a Backspace line join and a completion acceptance could not be undone. |
| `--gui-selftest` | 14 checks (two tabs, run path, cancellation, plot, keyboard, run states). None of them touched editor history. |
| The pre-cutover headless core (`rust/lovelace-desktop/src/app.rs`; exercised by `--selftest`, and by the explicit `--legacy-win32` shell) | Per-document undo/redo already existed: `Document { buffer, cursor, undo, redo }`, snapshots of buffer+cursor, `UNDO_CAP = 128`, `Command::Undo`/`Redo`, and Ctrl+Z/Ctrl+Y in the legacy Win32 menu. This is **not** the shipped egui window; its `--selftest undo-redo` check still passes untouched. |

So the deliverable was not "fix the shipped undo" - the shipped editor had none. It was to add a
real per-document history to the window's editor, including the coalescing and selection behaviour
EDT-012 requires.

## What was added

### 1. A reusable history (`rust/lovelace-desktop/src/history.rs`, new)

- **`Snapshot`** - the complete state one undo or redo restores: the buffer's `lines`, the `Caret`
  (0-based row and character column, never byte offsets) and the selection `anchor` (the caret is
  the moving end). Storing the whole state per step makes multi-line edits (paste, selected-range
  delete, line join) undo correctly without a second edit-delta representation.
- **`History`** - two stacks with `UNDO_CAP = 128` (oldest evicted first), `begin`/`push`,
  `undo`/`redo`, `break_burst`, `reset`, and the step counts. A new edit clears the redo branch.
- **`EditKind::{Typing, Atomic}`**. A typing insert opens one step and following inserts **coalesce
  into it while they arrive within `TYPING_BURST_GAP = 600 ms`** and no other action intervenes.
  `History::begin` is asked *before* the editor clones its buffer, so a coalesced keystroke
  performs no full-buffer copy at all - one snapshot per burst, not per character (PERF-009).
  Every other edit is `Atomic`: its own step, and it ends an open burst.
- **`command_for_shortcut`** - the one key mapping: Ctrl+Z = undo, Ctrl+Y = redo,
  Ctrl+Shift+Z = redo. Without the command modifier it returns `None`, because a bare `z`/`y` is
  typed text. This is the same boolean-input shape as `keyboard::command_for_shortcut`, so the
  window and `--gui-selftest` share it instead of keeping two copies.

### 2. The editor's edit primitives record steps (`ui/editor.rs`)

Every mutating primitive now goes through one `begin_edit`/`insert_text` path:

| Action | Step |
| --- | --- |
| `Event::Text` (typing a/ b/ c) | one coalesced `Typing` step per burst |
| `insert_str` containing newlines (typed or pasted) | the split lines are part of that same step |
| Enter (`newline`) | one `Atomic` step |
| Backspace / Delete (one character) | one `Atomic` step |
| Backspace / Delete at a line start (line join) | one `Atomic` step; undo restores the newline and the caret on line 2 |
| Delete/Backspace with a selection | the whole selected range is one step; undo restores the deleted text **and the selection** |
| Paste (`Event::Paste`) | one `Atomic` step, even for multi-line text |
| Completion acceptance (Enter/Tab with the popup open) | one `Atomic` step; undo returns to the typed prefix, not to an empty line |
| Tab indentation | one `Atomic` step |

A returned `TextEditor::command(HistoryCommand)` is the single entry point the key loop calls;
`undo()`/`redo()` are thin wrappers. Undo/redo set `edits`, so the window commits the restored text
to `Workbench::set_source` exactly like a typed edit: the document source and its revision follow
the undo, and the diagnostics decoration is dropped on the same pass (the existing edit-detection
path is reused, no second document model).

### 3. Keys, per document

- Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z are read from the same frame's key events in the editor's event
  loop and mapped through `history::command_for_shortcut` (above).
- The history lives in the `TextEditor`, and the window owns **one editor per document tab**
  (`WorkbenchApp::editors: Vec<TextEditor>`). It is therefore per document with no process-wide
  state: `WorkbenchApp` never has a global stack, and an undo of tab A can only pop tab A's editor.
  Switching tabs keeps each editor's history alive; an external `load` (file load / replacement)
  resets that editor's history because the old snapshots no longer describe the buffer.
- Typing coalescing is real-time: an insertion more than 600 ms after the previous one opens a new
  step; so do caret moves (arrows/Home/End), a click, a drag, Escape, the history keys themselves,
  and every atomic edit.

### 4. Selection, because undo must restore it

The editor had no selection, so a minimal one was added with the history (EDT-011's multi-cursor is
still out of scope):

- mouse drag selects from the press origin; a plain click places the caret and clears it;
- Shift+arrow/Home/End and Shift+click extend it; `select_to` is the programmatic form;
- the selection is painted behind the text (including across lines);
- typing, paste, Enter, Tab, Backspace and Delete replace/delete the selected range as one step;
- the selection anchor is part of every `Snapshot`, so undo restores the selection as it was.

### 5. `--gui-selftest`

One new check, `undo-redo`, joins the list (15 checks on Windows). It performs two typed edits
(`ab`, then `cd` after an Enter, so two separate bursts) and a Ctrl+Z / Ctrl+Y round trip plus a
Ctrl+Z / Ctrl+Shift+Z round trip, all through `command_for_shortcut` + `TextEditor::command` - the
same mapping and command entry point the window's key loop uses. Every step prints the observed
buffer and 0-based caret. It also proves a second editor keeps its own history.

### 6. Help text

`main.rs`'s `--help` shortcut list now names Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z.

## Tests and observed output

20 new tests; all existing tests were kept and still pass. Concrete input -> expected state:

| Test | Pins |
| --- | --- |
| `history::tests::ctrl_z_undoes_ctrl_y_and_ctrl_shift_z_redo` | the mapping: `Z`+command = Undo, `Y`+command = Redo, `Z`+shift+command = Redo, no command modifier = `None` |
| `history::tests::a_typing_burst_coalesces_until_a_pause_or_a_non_typing_edit` | 3 insertions < 600 ms = 1 step; a pause > gap = a new step; an atomic edit = its own step and ends the burst |
| `history::tests::undo_and_redo_round_trip_and_a_new_edit_discards_the_redo_branch` | undo -> redo restores the snapshot; an edit after an undo leaves 0 redo steps |
| `history::tests::an_empty_history_refuses_to_undo_or_redo` | no history = `None`, buffer untouched |
| `history::tests::the_oldest_step_is_evicted_at_the_cap` | 133 pushes keep the newest 128; the oldest kept state is #5 |
| `ui::editor::tests::typing_burst_is_one_step_and_undo_redo_restore_the_caret` | `""` + a,b,c -> `"abc"` caret (0,3); undo -> `""` caret (0,0); redo -> `"abc"` caret (0,3); 1 undo step |
| `ui::editor::tests::a_pause_starts_a_new_typing_step` | `ab` at t, `cd` at t+300 ms = 1 step; `ef` after t+300 ms+gap+1 ms = 2 steps; undo -> `"abcd"` caret (0,4) |
| `ui::editor::tests::a_non_typing_action_starts_a_new_step` | `ab`, Enter, `cd` = 3 steps; undo -> `"ab\n"` (1,0); undo -> `"ab"` (0,2); undo -> `""` |
| `ui::editor::tests::a_caret_move_between_typing_runs_starts_a_new_step` | `ab`, Left, `X` -> `"aXb"` = 2 steps; undo -> `"ab"` caret (0,1) |
| `ui::editor::tests::backspace_line_join_undo_restores_the_caret_on_line_two` | `"a\nb"` caret (1,0), Backspace -> `"ab"` (0,1); undo -> `"a\nb"` caret (1,0) / 1-based (2,1) |
| `ui::editor::tests::deleting_a_selection_undoes_to_the_text_and_the_selection` | `"abcdef"` select (0,1)..(0,4) = `bcd`, Delete -> `"aef"` (0,1); undo -> `"abcdef"`, selection ((0,1),(0,4)), selected `bcd`, caret (0,4) |
| `ui::editor::tests::a_multiline_paste_is_one_undo_step` | `"x"` caret end, paste `"a\nb"` -> `"xa\nb"` caret (1,1), 1 step; undo -> `"x"` caret (0,1) |
| `ui::editor::tests::a_completion_insertion_is_its_own_undo_step` | `"sq"`, accept `sqrt` -> `"sqrt"` (0,4); undo -> `"sq"` (0,2); undo -> `""` |
| `ui::editor::tests::each_editor_has_its_own_history` | undo in one editor leaves the other's buffer and steps untouched |
| `ui::app::tests::ctrl_z_undoes_a_typing_burst_and_ctrl_y_redoes_it` | 3 Text frames `a`,`b`,`c` -> `"abc"` Ln1 Col4; Ctrl+Z -> `""` Ln1 Col1 and document source `""`; Ctrl+Y -> `"abc"` Ln1 Col4; Ctrl+Z then Ctrl+Shift+Z -> `"abc"` |
| `ui::app::tests::a_letter_key_event_does_not_split_a_typing_burst_but_an_arrow_does` | a real Key+Text pair for `a` and `b` = 1 undo step; ArrowLeft then `X` = 2 steps; undo -> `"ab"` Ln1 Col2, undo -> `""` |
| `ui::app::tests::ctrl_z_restores_a_deleted_selection` | `"abcdef"` selection `bcd`, Delete -> `"aef"`; Ctrl+Z -> `"abcdef"` selection ((0,1),(0,4)) and document source restored |
| `ui::app::tests::ctrl_z_restores_a_backspace_line_join` | `"a\nb"` Ln2 Col1, Backspace -> `"ab"`; Ctrl+Z -> `"a\nb"` Ln2 Col1 and document source restored |
| `ui::app::tests::undo_in_one_document_never_changes_the_other` | A types `abc`, B types `xy`; undo in B -> B empty, A still `abc`; switch back and undo A -> A empty, B empty |
| `ui::app::tests::a_completion_insertion_is_its_own_undo_step_in_the_window` | type `sq`, Enter -> `sqrt` Ln1 Col5; Ctrl+Z -> `sq` Ln1 Col3 (popup closed); Ctrl+Z -> `""` |

Observed output (from `cargo-test-verbose.log`):

```text
test history::tests::an_empty_history_refuses_to_undo_or_redo ... ok
test history::tests::a_typing_burst_coalesces_until_a_pause_or_a_non_typing_edit ... ok
test history::tests::ctrl_z_undoes_ctrl_y_and_ctrl_shift_z_redo ... ok
test history::tests::undo_and_redo_round_trip_and_a_new_edit_discards_the_redo_branch ... ok
test history::tests::the_oldest_step_is_evicted_at_the_cap ... ok
test ui::editor::tests::a_caret_move_between_typing_runs_starts_a_new_step ... ok
test ui::editor::tests::a_completion_insertion_is_its_own_undo_step ... ok
test ui::editor::tests::a_multiline_paste_is_one_undo_step ... ok
test ui::editor::tests::a_non_typing_action_starts_a_new_step ... ok
test ui::editor::tests::a_pause_starts_a_new_typing_step ... ok
test ui::editor::tests::backspace_line_join_undo_restores_the_caret_on_line_two ... ok
test ui::editor::tests::deleting_a_selection_undoes_to_the_text_and_the_selection ... ok
test ui::editor::tests::each_editor_has_its_own_history ... ok
test ui::editor::tests::typing_burst_is_one_step_and_undo_redo_restore_the_caret ... ok
test ui::app::tests::ctrl_z_restores_a_backspace_line_join ... ok
test ui::app::tests::a_letter_key_event_does_not_split_a_typing_burst_but_an_arrow_does ... ok
test ui::app::tests::ctrl_z_restores_a_deleted_selection ... ok
test ui::app::tests::undo_in_one_document_never_changes_the_other ... ok
test ui::app::tests::ctrl_z_undoes_a_typing_burst_and_ctrl_y_redoes_it ... ok
test ui::app::tests::a_completion_insertion_is_its_own_undo_step_in_the_window ... ok
```

## `--gui-selftest`

The check prints its observed buffer/0-based caret after every step (from
`gui-selftest.log`):

```text
GUI_SELFTEST undo-redo type "ab": buffer "ab", caret (0, 2)
GUI_SELFTEST undo-redo Enter: buffer "ab\n", caret (1, 0)
GUI_SELFTEST undo-redo type "cd": buffer "ab\ncd", caret (1, 2)
GUI_SELFTEST undo-redo Ctrl+Z: buffer "ab\n", caret (1, 0)
GUI_SELFTEST undo-redo Ctrl+Y: buffer "ab\ncd", caret (1, 2)
GUI_SELFTEST undo-redo Ctrl+Z (again): buffer "ab\n", caret (1, 0)
GUI_SELFTEST undo-redo Ctrl+Shift+Z: buffer "ab\ncd", caret (1, 2)
GUI_SELFTEST ok undo-redo: two typed bursts around Enter, 3 steps; Ctrl+Z -> "ab\n" caret (1,0); Ctrl+Y -> "ab\ncd" caret (1,2); Ctrl+Z/Ctrl+Shift+Z round trip; per-editor histories independent
```

Run (default environment):

```text
rust\target-desktop\debug\lovelace-desktop.exe --gui-selftest
GUI_SELFTEST summary checks=15 failed=2
```

The two failures are the pre-existing plot checks (`plot`, `plot-dir`), not the undo check: this
agent's process sandbox denies writes under `%APPDATA%\Lovelace\plots`
(`Cannot write in the plot directory ...: Access is denied. (os error 5)`), which is exactly the
round-7 plot-environment blocker. A three-line Rust probe compiled and run the same way reproduces
`PermissionDenied` writing `.write-probe` there, while PowerShell and `cmd` can write; the
product's probe reports the truth. To obtain a green run for the same binary and checks, APPDATA
was relocated under the workspace (which the sandbox permits) and the working directory was kept
outside it:

```text
$env:APPDATA = 'C:\Users\ricar\dev\LovelaceSharp\.toolchain\selftest-appdata'
# from rust\target-desktop:
lovelace-desktop.exe --gui-selftest
GUI_SELFTEST ok plot: ...
GUI_SELFTEST ok plot-dir: resolved ...\.toolchain\selftest-appdata\Lovelace\plots ...
GUI_SELFTEST ok undo-redo: ...
GUI_SELFTEST summary checks=15 failed=0
```

That run exits 0 (`SELFTEST_EXIT=0`) and is saved as
`gui-selftest-workspace-appdata.log`. No check was changed or skipped. For completeness the legacy
`--selftest` also passes its untouched `undo-redo` check: 14/15 in the default environment (the one
failure is `engine-plot`, the same `%APPDATA%` write denial), 15/15 with the relocated APPDATA
(`legacy-selftest.log`, `legacy-selftest-workspace-appdata.log`).

## Full suite summary

Command and environment from the round brief:

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

| Run | Result |
| --- | --- |
| default features | **169 passed; 0 failed; 0 ignored** (was 149; +20 new tests) |
| `--no-default-features` (fake engine) | **164 passed; 0 failed; 0 ignored** (was 144; +20) |
| `cargo check -p lovelace-desktop --tests` | exit 0, **no warnings** |
| legacy `--selftest` | 15/15 with the relocated APPDATA; `undo-redo` ok in both environments |

Full logs in this directory: `cargo-test.log`, `cargo-test-verbose.log`,
`cargo-test-fake-engine.log`, `cargo-check.log`, `cargo-build.log`, `gui-selftest.log`,
`gui-selftest-workspace-appdata.log`, `legacy-selftest.log`,
`legacy-selftest-workspace-appdata.log`.

## Limits kept explicit

- The history is owned by the `TextEditor`; today that is exactly one editor per document tab, so
  it is per-document by construction. A future second view over the same document would have to
  move/share the history with the document model rather than with a second editor.
- The selection added here is a single anchor+caret range (enough for EDT-012's restore rule);
  multi-cursor and column selection (EDT-011) and cut/copy are not claimed.
- IME composition (EDT-017) is not claimed; the pre-edit/non-pre-edit Text events this grammar
  emits are coalesced like any other typing.
- A typing pause is a 600 ms wall-clock gap; a burst longer than that splits, by design.

## Files

- `rust/lovelace-desktop/src/history.rs` - new: snapshot, per-document stacks, coalescing, shortcut
  mapping and their unit tests.
- `rust/lovelace-desktop/src/ui/editor.rs` - history and selection fields, edit-step recording,
  undo/redo/command, selection painting and drag/shift selection, keyboard path, unit tests.
- `rust/lovelace-desktop/src/ui/app.rs` - window-path undo tests (burst, delete+selection, line
  join, two-document isolation, completion step).
- `rust/lovelace-desktop/src/gui_selftest.rs` - the `undo-redo` check and its per-step output.
- `rust/lovelace-desktop/src/main.rs` - `--help` shortcut list.
- `rust/lovelace-desktop/src/ui/mod.rs` - exposes the editor module to the in-crate selftest check.
- `docs/goal-desktop-v4/round-18/` - this document and the logs named above.
