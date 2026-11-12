# Round 11 - DEF-001 Backspace-join crash and DEF-002 Up-arrow focus

Status: both defects reproduced, fixed in `rust/lovelace-desktop`, and re-proven against the
shipped binary with a real window. Full desktop suite is green. No gate or harness file was
modified.

Files changed (crate only):

| file | change |
| --- | --- |
| `rust/lovelace-desktop/src/ui/editor.rs` | recompute paint bounds after editing; lock arrow keys while the editor is focused |
| `rust/lovelace-desktop/src/ui/app.rs` | two regression tests (pure addition: `git diff --numstat` = 74/0 and 24/0, zero deletions) |

Evidence lives beside this file:

- `pre/` - two-line document, Backspace, crash (exit 101, panic, no final capture)
- `pre/regression-red.log` - the new tests run on the pre-fix code (red)
- `pre-controls/` - the same Backspace at line 1 start and inside line 1 on the pre-fix binary
- `post/` - the same Backspace gesture after the fix (window alive, buffer joined)
- `pre-arrow/`, `post-arrow/` - DEF-002 before/after
- `post-scenario/` - `tools/gui-harness.ps1 -Action scenario` after the fix
- `full-test.log`, `build.log` - final suite and build
- `gui-gesture.ps1` - the driver used for the gesture (same primitives as the harness)

## 1. Reproduction of DEF-001 (pre-fix)

Method. `tools/gui-harness.ps1 -Action scenario` does not accept a whole SendKeys sequence as
`-Keys`: it treats `-Keys` as the script text, escapes `{...}` so the braces are typed literally
(`ConvertTo-SendKeysLiteral`), and then appends its own Enter/run/cancel gesture. It therefore
cannot express `a{ENTER}b{HOME}{BACKSPACE}`. To replay the reported edit without touching the
harness, `docs/goal-desktop-v4/round-11/gui-gesture.ps1` reuses the harness primitives: a launch
with redirected stderr, restore/foreground, UI Automation lookup, real `SendKeys` with a
foreground assertion before every send, PNG capture and a UIA tree dump.

Sequence on the shipped `rust/target-desktop/debug/lovelace-desktop.exe`, empty `untitled.lv`
tab:

1. click the `untitled.lv` tab, click into the editor, Home;
2. type `a`, Enter, `b`, Home:
   `Script editor, 2 lines, caret line 2 column 1`, toolbar `2 lines / 3 bytes`;
3. press Backspace at the start of line 2.

Observed (`pre/gesture.log`, `pre/gesture-result.json`, `pre/stderr.log`):

```text
11:07:57.661 caret after home='Script editor, 2 lines, caret line 2 column 1'
11:07:58.044 before gesture editor='Script editor, 2 lines, caret line 2 column 1' size='2 lines / 3 bytes' uia_focus=True
11:07:58.066 send label=backspace-join keys='{BACKSPACE}'
11:07:59.537 after gesture alive=False exited=True exit=101 window_gone=True editor='' size='' uia_focus=False
11:07:59.568 result gesture=backspace-join alive=False window_gone=True exit=101 png=0 uia=0 stderr_chars=287
```

The process exited with code **101**, the window was gone, no final capture and no final UIA
dump were possible, and stderr contains:

```text
lovelace-desktop: plot directory C:\Users\ricar\AppData\Roaming\Lovelace\plots

thread 'main' (31152) panicked at lovelace-desktop\src\ui\editor.rs:372:35:
index out of bounds: the len is 1 but the index is 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

This is a real process termination, not a hidden window or a hung UI: `alive_after_gesture` is
false, `window_gone` is true and `exit_code` is 101.

`pre/gesture-before.png` is the captured two-line `a` / `b` document immediately before the
Backspace; it is 1296x839 and shows `2 lines / 3 bytes`, `Ln 2, Col 1`.

Narrowing controls. The same driver was run against a rebuilt pre-fix binary (the `editor.rs`
fix temporarily reverted to `HEAD`, `cargo build` exit 0) on the same `a\nb` document:

- Backspace at a **non-boundary position inside line 1** (caret line 1, column 2), which
  deletes one character but keeps both lines (`pre-controls/in-line1/gesture.log`):

  ```text
  11:13:22.698 send label=backspace-in-line1 keys='{BACKSPACE}'
  11:13:24.191 after gesture alive=True exited=False exit= window_gone=False editor='Script editor, 2 lines, caret line 1 column 1' size='2 lines / 2 bytes' uia_focus=True
  ```

- Backspace at the **start of line 1** (caret line 1, column 1), where there is no previous line
  to join (`pre-controls/line1-start/gesture.log`):

  ```text
  11:14:19.282 send label=backspace-line1-start keys='{BACKSPACE}'
  11:14:20.769 after gesture alive=True exited=False exit= window_gone=False editor='Script editor, 2 lines, caret line 1 column 1' size='2 lines / 3 bytes' uia_focus=True
  ```

Both controls stayed alive with the expected buffer, so Backspace itself is not the trigger. The
join at the start of line 2 is the ordinary edit that **removes** a line; the crash needs that
line-count change. After the controls, the fixed `editor.rs` was restored, the crate was
clean-rebuilt (`cargo clean -p lovelace-desktop` then `cargo build`, exit 0), and the full suite
was rerun on that final binary (section 7).

## 2. Failing source line

Pre-fix `rust/lovelace-desktop/src/ui/editor.rs:372:35` (commit `dad97ae`):

```rust
let line = &self.lines[row];
```

It is inside the paint loop:

```rust
for row in first_row..last_row {
    ...
    let line = &self.lines[row];   // editor.rs:372
```

`TextEditor::ui` sampled `let total = self.line_count();` at the top of the pass
(`editor.rs:279`), before keyboard events were handled. `backspace()` then removed the second
line, but `last_row = (first_row + visible_rows).min(total)` still used the stale `total == 2`,
so the loop asked for `lines[1]` of the now one-line buffer: `index out of bounds: the len is 1
but the index is 1`.

## 3. Fix

Two small hunks in `TextEditor::ui`:

```rust
        // Editing shrinks or grows the buffer (Backspace at a line start removes a line). The
        // loop above may have changed `lines`, so recompute the bounds this pass paints with;
        // the values captured at the top of the pass describe the pre-edit buffer and would let
        // the paint loop index a line that no longer exists (DEF-001).
        let total = self.line_count();
        let max_scroll = (total as f32 * row_h - rect.height()).max(0.0);
```

`total` and `max_scroll` now describe the buffer after this pass's edits, so the label, scroll
clamp and paint loop all use the live line count. The second hunk (DEF-002) is described in
section 6. No `catch_unwind`, no test was weakened, no index guard was papered over: the stale
bound itself is gone.

## 4. Regression tests and observed red -> green

Both tests are in `rust/lovelace-desktop/src/ui/app.rs`, `mod tests`:

- `ui::app::tests::backspace_that_joins_two_lines_keeps_the_paint_pass_in_bounds`
  - input: source `"a\nb"`, caret `(row 1, col 0)`, frame with Backspace;
  - expected: editor source `"ab"`, caret `(1, 2)`, workbench document source `"ab"`.
- `ui::app::tests::arrow_up_moves_the_caret_without_surrendering_editor_focus`
  - input: source `"a\nb"`, caret `(row 1, col 0)`, ArrowUp, then text `"c"`;
  - expected: caret `(1, 1)`, `ctx.memory().focused() == Some(surface_id())`, source `"ca\nb"`.

Red, on the pre-fix code (`pre/regression-red.log`): Backspace panics at the same line as the
GUI; ArrowUp moves focus to another widget.

```text
thread 'ui::app::tests::backspace_that_joins_two_lines_keeps_the_paint_pass_in_bounds' (26892)
panicked at lovelace-desktop\src\ui\editor.rs:372:35:
index out of bounds: the len is 1 but the index is 1
test ui::app::tests::backspace_that_joins_two_lines_keeps_the_paint_pass_in_bounds ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 107 filtered out

thread 'ui::app::tests::arrow_up_moves_the_caret_without_surrendering_editor_focus' (26436)
panicked at lovelace-desktop\src\ui\app.rs:956:9:
assertion `left == right` failed: Up must not move keyboard focus out of the editor
  left: Some(Id::unique(6303579221173924000))
 right: Some(Id::unique("editor_surface"))
test ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 107 filtered out
```

The red run was made with the unmodified code before either fix was applied, so reverting the
fix re-creates both failures (the panic and the focus assertion).

Green, after the fix (`post-test.log`, `post-arrow-test.log`):

```text
test ui::app::tests::backspace_that_joins_two_lines_keeps_the_paint_pass_in_bounds ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.15s

test ui::app::tests::arrow_up_moves_the_caret_without_surrendering_editor_focus ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.14s
```

## 5. Post-fix proof of the exact gesture

Same driver, same sequence, final fixed binary (`cargo clean -p lovelace-desktop` then
`cargo build -p lovelace-desktop`, both exit 0; `lovelace-desktop.exe` timestamp
2026-11-12T11:14:43-03:00). `post/gesture.log`:

```text
11:16:16.398 before gesture editor='Script editor, 2 lines, caret line 2 column 1' size='2 lines / 3 bytes' uia_focus=True
11:16:16.421 send label=backspace-join keys='{BACKSPACE}'
11:16:17.917 after gesture alive=True exited=False exit= window_gone=False editor='Script editor, 1 lines, caret line 1 column 2' size='1 lines / 2 bytes' uia_focus=True
11:16:18.210 capture label=gesture-final rect=1296x839 file=...\post\gesture-final.png bytes=123711
11:16:18.270 send label=end-after-gesture keys='{END}'
11:16:18.943 after end editor='Script editor, 1 lines, caret line 1 column 3'
```

The window is alive, still has keyboard focus (`uia_focus=True`), the buffer is the expected
joined one line / two bytes (`ab`), and the caret sits at column 2 (between `a` and `b`). End
then moves it to column 3, proving the editor keeps accepting keys. `post/gesture-final-uia.txt`
repeats the same facts through UIA:

```text
Text | 1 lines / 2 bytes
Text | untitled.lv  |  rev 4  |  precision 1000  |  Ln 1, Col 2  |  run Idle
Edit | Script editor, 1 lines, caret line 1 column 2
```

`post/gesture-final.png` shows the rendered `a|b` line, the `1 lines / 2 bytes` labels and the
status strip.

Harness scenario still works after the fix (`post-scenario/`, run through the same
`tools/gui-harness.ps1` file):

```text
scenario caret after typing statement='Script editor, 1 lines, caret line 1 column 27'
scenario caret after enter='Script editor, 2 lines, caret line 2 column 1'
scenario transition phase=ctrl+enter elapsed=109ms state='... run Running'
scenario transition phase=escape elapsed=58.9ms state='... run Cancelled'
scenario close pid=26400 graceful=True
SCENARIO_OK RUNNING_MS=109 CANCELLED_MS=58.9 ...
```

(The summary header it writes still says "Round 10 GUI scenario evidence" because that string is
hardcoded in the harness; the harness was not modified.)

## 6. DEF-002 - Up arrow loses editor focus

Reproduced pre-fix with the same driver (`pre-arrow/gesture.log`):

```text
11:08:07.475 before gesture editor='Script editor, 2 lines, caret line 2 column 1' size='2 lines / 3 bytes' uia_focus=True
11:08:07.510 send label=arrow-up keys='{UP}'
11:08:07.785 after up editor='Script editor, 2 lines, caret line 1 column 1' uia_focus=False
11:08:07.808 send label=type-after-up keys='c'
11:08:08.712 after gesture ... editor='Script editor, 2 lines, caret line 1 column 1' size='2 lines / 3 bytes' uia_focus=False
```

Up moves the caret to line 1 but UIA `HasKeyboardFocus` flips to false; the following `c` and
`End` are ignored, which is exactly the reported "refocus click is needed". The window stays
alive (this is not the crash).

Cause is in the same editor/focus code path, via egui 0.37's cardinal focus navigation
(`egui-0.37.0/src/memory/mod.rs:589-593`): a bare `ArrowUp`/`ArrowDown`/`ArrowLeft`/`ArrowRight`
sets a `FocusDirection`, and `Focus::end_pass` (`memory/mod.rs:636-640`) then moves
`focused_widget` to the closest focusable widget in that direction. This happens before the
editor's event loop, so the editor handles the caret move but ends the frame without focus.

Fix in `editor.rs`, while the editor has focus:

```rust
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
```

This is the same mechanism egui's own `TextEdit` uses (`set_focus_lock_filter`). Tab and Escape
are deliberately left unfiltered: Tab keeps its focus-traversal behaviour and Escape must still
reach the window-global Cancel shortcut. Like `TextEdit`, the lock is stored for the following
pass, so the very first frame that gains focus is not covered until the next frame; every normal
keyboard interaction (including the GUI repro, many frames after focus) is covered.

Post-fix (`post-arrow/gesture.log`):

```text
11:16:31.000 send label=arrow-up keys='{UP}'
11:16:31.275 after up editor='Script editor, 2 lines, caret line 1 column 1' uia_focus=True
11:16:31.297 send label=type-after-up keys='c'
11:16:32.194 after gesture ... editor='Script editor, 2 lines, caret line 1 column 2' size='2 lines / 4 bytes' uia_focus=True
11:16:33.229 after end editor='Script editor, 2 lines, caret line 1 column 3'
```

`c` now lands in the editor (`ca\nb`, 4 bytes), focus stays in the editor, and End still
behaves. `post-arrow/gesture-final-uia.txt` confirms `Edit | Script editor, 2 lines, caret line
1 column 2`.

One related observation, not a defect: a bare **Escape** intentionally clears egui keyboard
focus (`memory/mod.rs:598-601`), which is why the harness scenario still clicks the editor to
refocus after its Esc-cancel step. That is egui's designed Escape behaviour and is separate from
DEF-002 (Up).

## 7. Full desktop suite

Command (as specified in the round brief):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Result (`full-test.log`):

```text
test result: ok. 108 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
```

108 = the 106 pre-existing tests plus the two new regression tests. No pre-existing test was
edited or removed: `git diff --numstat -- rust/lovelace-desktop` is `74 0
src/ui/app.rs` and `24 0 src/ui/editor.rs`, i.e. additions only, and all 106 original test
names still appear as `... ok` in the log. The final suite above was run after the pre-fix
control experiment was undone and the fixed crate was clean-rebuilt from scratch
(`cargo clean -p lovelace-desktop` then `cargo build -p lovelace-desktop`, exit 0), so the
shipped binary and the tested binary are the fixed source. `rustfmt --check` on the two changed
files exits 0 (the crate-wide `cargo fmt --check` still flags pre-existing formatting in
`engine.rs`/`execution.rs`, which this round did not touch).

## 8. DEF-001 status

Fixed and verified end-to-end. Recommended gate update: DEF-001 `status: closed`, evidence
`round-11/crash-fix.md` plus `round-11/post/`. DEF-002 is also fixed and verified; recommend
closing it with `round-11/crash-fix.md` plus `round-11/post-arrow/`. The gate file itself was
not modified in this round.
