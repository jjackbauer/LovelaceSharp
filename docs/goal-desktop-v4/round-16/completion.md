# EDT-003: live, context-aware completion in the desktop editor

Requirement: the native editor gets a completion popup sourced at run time from the language,
the active engine's registry and the document's last committed workspace - never from a
completion-local list. This round adds the catalogue, the caret-context rule, the popup, the
keyboard path and the insertion, and ships the window evidence.

## Catalogue sources

The catalogue is rebuilt every pass by `Workbench::completion_catalogue`
(`rust/lovelace-desktop/src/workbench.rs:377`) from four live sources:

| Source | Where it comes from | Detail column |
| --- | --- | --- |
| Keywords | `crate::language::KEYWORDS` (`rust/lovelace-desktop/src/language.rs`) | `keyword` |
| Engine registry functions | `Evaluator::state()` (`rust/lovelace-desktop/src/engine.rs:116`); the real adapter copies `lovelace_lang::Engine::capture_state` field for field (`engine/real.rs`) | `sqrt(x)` |
| User-defined functions | the same engine capture, `is_builtin == false` (script `func`, host/plugin registrations) | `myfunc(a, b)` |
| Document variables | `WorkspaceSnapshot.state.variables` from the active document's last **committed** run, with the engine's own `kind` | `a (Natural)` |

Details of the wiring:

- **The registry is the engine's, not a desktop copy.** `Evaluator::state()` returns an
  `EngineState` (`engine.rs:141`). `RealEngine::state()` calls
  `Engine::capture_state()`, which publishes every enabled builtin from
  `lovelace_lang::registry::BUILTINS` with its declared parameters and plugin, every host
  registration, and the script's own `func` definitions (`is_builtin: false`). The desktop
  maps names, parameters and the builtin flag verbatim; it has no builtin table anywhere in
  the completion code. The deterministic fake engine reports its own value kinds and its own
  `builtin_functions()` registry the same way.
- **The registry is available before the first run.** `Workbench::new` and
  `Workbench::with_factory` capture the engine instance they already construct for the
  engine name, so the popup offers `sqrt(x)` on a fresh document. After a run,
  `Workbench::completion_state` overlays the last committed snapshot's variables and
  functions on top of that registry (a symbol the run published replaces the registry entry
  of the same name).
- **The snapshot is the last committed one.** `ExecutionHost`'s worker takes
  `Evaluator::state()` at the same commit point as `variables()`/`functions()`
  (`execution.rs`, `Progress::state`) and publishes it in `WorkspaceSnapshot.state`; the
  catalogue therefore describes exactly the committed prefix the workspace pane displays.
- **Keywords have one table.** `KEYWORDS` is the same table the syntax highlighter paints
  (`ui/syntax.rs` imports it), so colouring and completion cannot disagree. The list is the
  engine parser's keyword set (`break continue else for func if in return while`); the old
  highlighter list (`let`, `end`, `function`, `set`, ...) was wrong and is gone. Two tests in
  `language.rs` ask the real engine's parser to accept one minimal construct per keyword and
  reject the stale words, so an engine change fails the test instead of silently dropping a
  keyword from completion.

`CompletionItem` (label, detail, insert text, kind) is deduplicated by label with a fixed
precedence - variable > user function > builtin > keyword - and sorted case-insensitively by
name, so a variable named `sqrt` hides the builtin of that name and appears exactly once.
`completion::filter` (case-insensitive prefix) puts an exact label match first, then the
remaining prefix matches alphabetically. `insert` is always the label, so acceptance cannot
expand a prefix into text the user did not choose.

## UI behaviour

All of this lives in `rust/lovelace-desktop/src/ui/editor.rs` and
`rust/lovelace-desktop/src/completion.rs`.

- **Ctrl+Space** opens the popup for the identifier prefix before the caret; with no prefix
  it opens the whole catalogue. The same keystroke in a string or comment opens nothing.
- **Typing** one identifier character (`[A-Za-z_]` start, `[A-Za-z0-9_]` continue) refreshes
  or opens the popup automatically; typing anything else (space, punctuation, a digit that
  starts a number, a quote, `#`) closes it. `1sq` is a number, not a prefix.
- **Filtering** is case-insensitive over the label; the list keeps the highlight when the
  narrower filter still contains it.
- **Up/Down** move the highlight (clamped, no wrap); **Enter/Tab** accept; **Esc** dismisses
  without touching the buffer or the caret; **Ctrl+Enter** remains Run and never accepts;
  arrows/Home/End/click dismiss and then act normally.
- **Acceptance** replaces exactly the characters between the prefix start and the caret with
  `item.insert` (the label) and leaves the caret after it. A caret mid-word replaces only the
  typed prefix (`sqx` with the caret after `sq` becomes `sqrtx`). A stale popup whose prefix
  no longer matches the buffer refuses the edit and closes instead of rewriting text.
- **Popup geometry.** The popup is a painter-only overlay: no widget, no interaction, no
  focus request, so it cannot block typing or swallow a click. It is anchored one row
  **below** the caret row (or above it when that side has less room) and the number of rows
  is capped by the free space, so it does not cover the caret line (unless the pane is
  shorter than one row, in which case nothing is painted). At most 9 rows are shown; the
  visible window scrolls with the highlight.
- **User functions vs builtins** are marked by the kind bar and label colour (builtin keeps
  the highlighter's blue, user/host functions use the accent teal, variables plain text,
  keywords the keyword colour) and the pane header states the selected entry's origin.
- **Caret context** is the cheap line-local scan in `completion::caret_context`: a `#`
  outside a string starts a comment, a `"` opens/closes a string with backslash escapes, and
  neither offers completion. The language's strings and comments are line-local
  (`syntax::job` scans them the same way), so no lexer is needed and the rule is exact for
  this grammar. A multi-line string does not exist in the grammar; if one were ever added,
  this rule would need the lexer.

The selected entry is also published in the editor pane header as a normal egui label, e.g.
`Completion: sq -> sqrt(x) [builtin] 1/2`. That is deliberate: the popup itself is painted
(a widget overlay that captured pointer/focus would violate "never blocks typing"), so the
header label is what makes the selected signature part of the window's accessibility tree and
of a UIA capture.

## Unit tests and observed output

`rust/lovelace-desktop/src/completion.rs` covers the catalogue, the caret context, the prefix
and the insertion; `language.rs`, `engine.rs`, `workbench.rs` and `ui/app.rs` cover the
sources and the window path.

| Test | What it pins |
| --- | --- |
| `completion::tests::catalogue_combines_keywords_functions_and_variables_without_duplicates` | keywords + builtins + user functions + variables, dedup, name order, `sqrt (Real)`, `norm(a, b)`, user `sin` overriding builtin `sin` |
| `completion::tests::every_catalogue_entry_comes_from_one_of_the_live_sources` | no invented entry: every label is one of the arguments |
| `completion::tests::filter_is_case_insensitive_and_puts_the_exact_label_first` | `sq` -> `sq, sqrt`; `SQ` identical; `s` alphabetical; `zz` empty |
| `completion::tests::identifier_prefix_scans_only_the_identifier_run_before_the_caret` | `x = sq` -> `sq` at col 4; `_a1`; no run; `1sq` -> none; dot; mid-word `sqx` |
| `completion::tests::caret_context_suppresses_strings_and_comments` | code/string/comment, closing quote, `#` inside string, escaped quote |
| `completion::tests::accepting_replaces_only_the_prefix_and_moves_the_caret` | `x = sq` -> `x = sqrt` caret col 8; `sqx` mid-word -> `sqrtx`; highlighted item inserted |
| `completion::tests::escape_dismisses_without_changing_the_buffer` | dismissal leaves the text and refuses a later accept |
| `completion::tests::arrow_navigation_is_clamped_and_a_stale_prefix_is_refused` | Up/Down clamp; a changed buffer refuses the edit |
| `language::tests::every_keyword_is_a_keyword_in_the_engine_parser` | each of the nine keywords parses in the real engine |
| `language::tests::words_the_engine_does_not_treat_as_keywords_are_not_offered` | stale highlighter words are not offered |
| `engine::tests::the_real_engine_publishes_its_registry_and_marks_user_functions` | `sqrt` is a builtin with parameter `x`; `myfunc` is `is_builtin: false`; variables carry `Natural` |
| `workbench::tests::completion_catalogue_merges_the_engine_registry_with_the_last_committed_snapshot` | registry `sqrt(x)` + snapshot `a (Natural)` + keywords + fake registry, deduplicated |
| `workbench::tests::completion_catalogue_offers_the_real_registry_and_a_script_defined_function` | before a run the real registry reaches completion; after running `func myfunc(a,b)` it is offered as `Function`, not `Builtin` |
| `ui::app::tests::typing_sq_opens_completion_with_the_engine_function_and_enter_replaces_only_the_prefix` | type `sq` -> popup label `sqrt`, detail `sqrt(x)`, header `Completion: sq -> sqrt(x) [builtin] 1/2`; Enter -> `sqrt`, caret (1,5), document synced |
| `ui::app::tests::arrow_keys_move_the_completion_selection_and_tab_accepts_it` | exact label first, Down/Up, Tab accepts, caret after text |
| `ui::app::tests::escape_dismisses_completion_without_changing_the_buffer_or_accepting` | Esc leaves `sq` and (1,3); the next Enter is a newline |
| `ui::app::tests::ctrl_space_opens_the_whole_catalogue_and_strings_and_comments_suppress_it` | empty-prefix catalogue; no space inserted; string and comment suppress |

Observed output (full log in `cargo-test.log`; the excerpt is `completion-tests.log`):

```text
test completion::tests::caret_context_suppresses_strings_and_comments ... ok
test completion::tests::arrow_navigation_is_clamped_and_a_stale_prefix_is_refused ... ok
test completion::tests::accepting_replaces_only_the_prefix_and_moves_the_caret ... ok
test completion::tests::catalogue_combines_keywords_functions_and_variables_without_duplicates ... ok
test completion::tests::escape_dismisses_without_changing_the_buffer ... ok
test completion::tests::every_catalogue_entry_comes_from_one_of_the_live_sources ... ok
test completion::tests::filter_is_case_insensitive_and_puts_the_exact_label_first ... ok
test completion::tests::identifier_prefix_scans_only_the_identifier_run_before_the_caret ... ok
test engine::tests::the_real_engine_publishes_its_registry_and_marks_user_functions ... ok
test language::tests::words_the_engine_does_not_treat_as_keywords_are_not_offered ... ok
test language::tests::every_keyword_is_a_keyword_in_the_engine_parser ... ok
test workbench::tests::completion_catalogue_merges_the_engine_registry_with_the_last_committed_snapshot ... ok
test workbench::tests::completion_catalogue_offers_the_real_registry_and_a_script_defined_function ... ok
test ui::app::tests::escape_dismisses_completion_without_changing_the_buffer_or_accepting ... ok
test ui::app::tests::typing_sq_opens_completion_with_the_engine_function_and_enter_replaces_only_the_prefix ... ok
test ui::app::tests::arrow_keys_move_the_completion_selection_and_tab_accepts_it ... ok
test ui::app::tests::ctrl_space_opens_the_whole_catalogue_and_strings_and_comments_suppress_it ... ok
```

## Window proof

### The required scenario, and the environment blocker

The required driver was run with the harness untouched (twice; both runs stopped at the same
step-3 focus assertion, the first from the round-16 root and the second into
`scenario-blocked/`, which is the one recorded below):

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\gui-harness.ps1 -Action scenario ^
  -Flow stale -FastStatus -CaptureStates "run Stale" -Keys "a = 1;" -StaleChar "sq" ^
  -MaxAttempts 1 -Exe rust\target-desktop\debug\lovelace-desktop.exe ^
  -OutDir docs\goal-desktop-v4\round-16\scenario-blocked ^
  -Log docs\goal-desktop-v4\round-16\scenario-blocked\driver.log
```

It aborts at its step-3 focus assertion. The console session is locked in this environment:
`GetForegroundWindow()` is `Windows Default Lock Screen`, `SetForegroundWindow` is refused,
and `AttachThreadInput` to that thread fails, so the harness cannot make the app foreground
to type into it. Verbatim tail of `scenario-blocked\scenario-blocked.log`:

```text
12:52:08.392 scenario window mainhandle=856122 hwnd=856122 title='Lovelace Desktop' rect=1296x839 at 234,234 after=1118ms restored=False stable=True
12:52:08.460 scenario uia ready hwnd=856122 nodes=42 after=66ms
powershell.exe : scenario: cannot focus the window before selecting the empty tab
At C:\Users\ricar\dev\LovelaceSharp\tools\gui-harness.ps1:857 char:47
+ ... ndow $h)) { throw 'scenario: cannot focus the window before selecting ...
scenario: cannot focus the window before selecting the empty tab
```

The failed attempt's own driver log is kept as `scenario-blocked\driver-first-attempt.log`.
No harness file was modified; the failure is a property of the locked console, not of the
harness or the product.

### What was captured instead

`docs\goal-desktop-v4\round-16\capture-popup.ps1` takes the same evidence from the same live
window without touching the lock screen or the foreground policy:

1. launches `rust\target-desktop\debug\lovelace-desktop.exe` on an empty `completion.lv`
   document;
2. posts real key messages to its window (`WM_SETFOCUS`, then
   `WM_KEYDOWN`/`WM_CHAR`/`WM_KEYUP` for `s` and `q`) - winit consumes them exactly as it
   consumes the messages SendKeys generates, so the editor really types `sq`;
3. calls the **unmodified** harness with `-Action uia` (it needs no foreground) to capture
   the live accessibility tree: `uia-driver.log`, `uia-tree.txt`;
4. captures the window with `PrintWindow(PW_RENDERFULLCONTENT)`, which asks the compositor
   for the window's own surface. `CopyFromScreen` returns black on a locked desktop, so it
   cannot be used here.

**Artifact:** `docs\goal-desktop-v4\round-16\completion-sq-sqrt.png` - **139,137 bytes**,
1296x839 PNG, SHA-256
`3AD0C8FFB70898E09BFAF46F544D14358B7642D8194CAB5AF120335DF7CC15BB`.
It is the shipped window of the shipped binary, opened and inspected: the editor holds `sq`
with the caret after `q`; the popup is painted one row below the caret line and shows

```text
sqrt      sqrt(x)      <- highlighted
squeeze   squeeze(a)
```

and the pane header reads `Completion: sq -> sqrt(x) [builtin] 1/2`; the status strip reads
`Ln 1, Col 3 | run Idle`. A crop of the same PNG is
`completion-sq-sqrt-detail.png` (18,530 bytes, SHA-256
`530CB78AE2721431D5ADBA33B601C26410C79BF46C20A77E92A3EEEFFEBACE5C`). The pre-typing frame
`completion-before.png` (133,958 bytes) is the same window with the empty document and no
popup.

### Scenario excerpt (`sq` -> `sqrt` with signature)

From `completion-uia-excerpt.txt`, extracted from the harness's `uia-tree.txt` while the
popup was open:

```text
Text | Completion: sq -> sqrt(x) [builtin] 1/2 | id= | focusable=False | enabled=True
Edit | Script editor, 1 lines, caret line 1 column 3 | id= | focusable=True | enabled=True
```

So the typed prefix is exactly `sq`, the highlighted catalogue entry is the engine's `sqrt`
with its declared parameter `x`, the header states the `builtin` origin, and the caret is at
column 3 - the popup did not insert anything on its own. The `1/2` is the `sqrt`/`squeeze`
pair the registry has for the `sq` prefix.

Reproduce the capture with:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File docs\goal-desktop-v4\round-16\capture-popup.ps1
```

## Full test summary

Command (Windows PowerShell 5.1 environment from the round brief):

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

Observed tail (full output in `cargo-test.log`, 25,762 bytes):

```text
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

Also observed, on the fallback engine (the real-engine-only tests are feature-gated off):

```text
cargo test -p lovelace-desktop --no-default-features --manifest-path rust\Cargo.toml
test result: ok. 144 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
```

Both runs compiled with no warnings from `lovelace-desktop` after the final build.

## Files

- `rust/lovelace-desktop/src/completion.rs` - new: catalogue, filter, prefix, caret context,
  session/insertion and their unit tests.
- `rust/lovelace-desktop/src/language.rs` - new: the single keyword table + engine-parser
  drift tests.
- `rust/lovelace-desktop/src/engine.rs` - `EngineState`/`EngineFunction`/`EngineVariable`,
  `Evaluator::state()`.
- `rust/lovelace-desktop/src/engine/real.rs` - maps `Engine::capture_state()`.
- `rust/lovelace-desktop/src/engine/fake.rs` - kind-aware fake capture.
- `rust/lovelace-desktop/src/execution.rs` - `WorkspaceSnapshot.state`, captured at commit.
- `rust/lovelace-desktop/src/workbench.rs` - registry capture, snapshot overlay,
  `completion_catalogue`, wiring tests.
- `rust/lovelace-desktop/src/ui/editor.rs` - popup state, keys, insertion, painter.
- `rust/lovelace-desktop/src/ui/app.rs` - catalogue handed to the editor, header indicator,
  Esc gating, UI tests.
- `rust/lovelace-desktop/src/ui/syntax.rs` - consumes the shared keyword table.
- `docs/goal-desktop-v4/round-16/` - this document, `capture-popup.ps1`, PNGs, logs.
