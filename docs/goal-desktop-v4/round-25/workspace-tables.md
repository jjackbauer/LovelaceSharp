# WKS-001 / WKS-002 — the Workspace variables and functions tables

Requirements (`docs/goal-desktop-v4/requirements.md:275-276`):

| Id | Requirement |
| --- | --- |
| **WKS-001** | Show name, kind and display value for all globals including `_`, sorted by ordinal name by default; sortable. |
| **WKS-002** | Show function name, parameters, builtin/user/plugin origin and summary when available; counts agree with the current engine registry. |

## 1. What was missing

The Workspace pane painted two lists, not tables:

1. **Variables were `name = typed display`.** The kind was fused into the displayed string because
   `DocumentView::variables` carries the engine's legacy `(name, typed display)` pairs
   (`a = 1 (Natural)`). There was no kind column and no separate display value; `_` was only
   visible as that fused string, not as a row with its own name / kind / value cells.
2. **Nothing was clickable.** No column headers existed, no sort state existed, and the pane had
   no way to sort either list in either direction. The order was whatever the engine happened to
   publish.
3. **The function pane showed the legacy user-function list.** `DocumentView::functions` came from
   `Evaluator::functions()`; the real engine's implementation returns only the script's `func`
   definitions, so the registry's builtins and plugin builtins never appeared in the pane even
   though completion offered them. There was no origin column and no summary.
4. **The displayed function count could not agree with the registry.** With the real engine the
   pane header counted the user definitions while the engine's registry held all builtins and
   plugin functions. A count that claimed to describe the engine was therefore wrong.
5. **No truncation/tooltip contract.** Long displays were painted in full and simply overflowed or
   widened the pane; there was no cell cut and no way to read the whole value.

The regression tests pinned these facts: the real-engine snapshot test already asserted that
`_` is captured (`execution.rs`), but nothing exercised the pane's variable kinds, function
origins, header sorting or count agreement.

## 2. What changed

### 2.1 The engine capture now carries origin and summary

`rust/lovelace-desktop/src/engine.rs`:

- `FunctionOrigin { Builtin, Plugin, User }` and `FunctionOrigin::label()` (`engine.rs:119`).
- `EngineFunction` gained `plugin: Option<String>` and `summary: Option<String>`
  (`engine.rs:146`) plus `EngineFunction::origin()` (`engine.rs:165`): a script `func` is `user`,
  a builtin with a plugin name is `plugin`, every other builtin is `builtin`. `is_builtin` is still
  read for completion, so the completion behaviour is unchanged.
- `EngineState::from_legacy` leaves both new fields `None`; a minimal evaluator cannot claim a
  registry summary or a plugin it never published.

`rust/lovelace-desktop/src/engine/real.rs:104` maps the engine's own `StateFunction` field for
field. `StateFunction` carries the plugin but not the registry summary, so the adapter reads the
summary from the same engine registry the capture is built from
(`lovelace_lang::registry::find(name).summary`). No summary is transcribed into the desktop and a
user definition gets `None`.

### 2.2 The snapshot's function list is the registry capture

`rust/lovelace-desktop/src/execution.rs:732`: at every commit the state capture is taken and the
legacy `functions: Vec<(String, Vec<String>)>` is rebuilt from it. The snapshot's function count,
the pane's rows and the completion registry are therefore the same set: builtins, plugin builtins
and user definitions. A minimal evaluator with no state capture keeps its legacy pair list.

### 2.3 The table models

New module `rust/lovelace-desktop/src/workspace_table.rs`:

- `VariableRow { name, kind, display, ordinal }` and `FunctionRow { name, parameters, origin,
  plugin, summary, ordinal }`; rows are built from the snapshot's `EngineState`
  (`variable_rows` / `function_rows`, lines 258/273).
- `SortState<C>` (`workspace_table.rs:52`) with the required click contract: the default is the
  capture's ordinal (name) order; the first click on a column sorts ascending, the second
  descending, a third ascending; selecting a different column starts ascending again. The first
  click on the already-sorted default Name column stays ascending (the default row order is the
  "ordinal name" order the requirement names), so it takes two clicks to reach descending.
- `ordered_variables` / `ordered_functions` (lines 298/318): every sort compares the column key
  and then the capture `ordinal`. The ordinal tie-break makes equal keys keep capture order in
  both directions and makes every sort a permutation of exactly the same rows — no row can be
  lost and a descending sort cannot reverse tied rows.
- `cell_text` (`workspace_table.rs:31`) cuts a value at `MAX_CELL_CHARS` (40) characters on a
  character boundary and appends `…`; the model keeps the full string for the hover text.

### 2.4 The pane

`rust/lovelace-desktop/src/ui/app.rs`:

- `sync()` rebuilds both tables from the frame's one committed capture on every pass
  (`app.rs:244`), keeping the user's sort across runs and tab switches.
- `workspace_pane` (`app.rs:725`) now paints a three-column header for each table. The headers are
  real egui `Button`s (`Name`, `Kind`, `Value`; `Name`, `Parameters`, `Origin`) with a `▲`/`▼`
  direction arrow on the sorted column and a hover explanation; a click calls `SortState::clicked`.
  The first click sorts ascending, the second descending.
- Every row is painted with aligned fixed-width cells (`table_cell`, `app.rs:1154`). A cell's
  visible text is the character-truncated text and the full string is attached as
  `.on_hover_text(...)`, including a variable's whole display value and a function's whole summary.
- Function rows show name, comma-joined parameters (`a, b`) and the origin wording, coloured by
  origin (builtin dim, plugin warn, user accent); a registry summary is a second, small, truncated
  line under its row (`app.rs:931-955`).
- Section labels state the counts the rows are built from: `VARIABLES · N` and `FUNCTIONS · N`;
  the pane header states the compact `N vars · M funcs`. The N/M are the table row counts, which
  are the snapshot's counts.
- The filter still narrows the painted rows by name; the counts deliberately remain the totals
  they describe.

## 3. Tests

Ten new tests; the suite went from 215 to **225 passed, 0 failed** (full log
`cargo-test.log`).

| Test | What it proves | Observed |
| --- | --- | --- |
| `workspace_table::tests::variables_default_to_name_ascending_and_include_the_underscore_global` | Default order is name/ordinal ascending; `_` is a row with `Natural` and its display; the same set sorts both directions by name (`_` first ascending, last descending) and by kind keeps tied rows in capture order. | `ok` |
| `workspace_table::tests::a_header_click_sorts_ascending_then_descending_and_a_new_column_restarts_ascending` | First click ascending, second descending, third ascending; a new column restarts ascending; default column starts ascending. | `ok` |
| `workspace_table::tests::sorting_is_stable_for_tied_keys_and_never_loses_a_row` | Every sort keeps all four rows exactly once; descending kind leaves the three tied `Natural` rows in capture order (`[3,0,1,2]`). | `ok` |
| `workspace_table::tests::functions_show_parameters_origin_and_summary` | `aaa` is `user`, `abs` is `builtin`, `acos` is `plugin` (with `Lovelace.Symbolics`), comma-joined parameters, summary carried; origin counts are 1/1/1. | `ok` |
| `workspace_table::tests::the_function_count_is_the_snapshot_registry_count` | A snapshot whose legacy list has only the user function still displays all 3 registry rows: the table count equals `snapshot.state.functions.len()`, not the legacy list. | `ok` |
| `workspace_table::tests::a_large_value_is_truncated_for_the_cell_and_kept_for_the_tooltip` | A 100-character value becomes exactly 40 characters ending in `…`, while the model keeps all 100 for the tooltip. | `ok` |
| `workspace_table::tests::truncation_counts_characters_not_bytes` | Truncation counts characters, not UTF-8 bytes, and never splits a multi-byte character. | `ok` |
| `engine::tests::the_real_registry_classifies_builtin_plugin_and_user_functions` | With the shipped engine: `aaa` → `user`, `abs` → `builtin` (registry summary present), `acos` → `plugin` with `Lovelace.Symbolics` and a summary. | `ok` |
| `workbench::tests::the_real_snapshot_publishes_underscore_and_the_registry_count_for_the_tables` | A real run of `func aaa(x) = x * 2`, `a = 21`, `aaa(a)` publishes `_` (`Natural`, `42`) and `a` (`Natural`, `21`); the table has `fresh_registry.functions.len() + 1` rows and equals both `view.state.functions.len()` and `view.functions.len()`. | `ok` |
| `ui::app::tests::workspace_header_clicks_sort_both_tables_and_the_counts_are_the_snapshot_counts` | A real pointer click on the painted header rectangles: first click stays/enters ascending, second reverses the rows; switching columns starts ascending; both tables' row counts equal the committed snapshot's counts. | `ok` |

Observed suite lines (from `cargo-test.log`):

```
test workspace_table::tests::variables_default_to_name_ascending_and_include_the_underscore_global ... ok
test workspace_table::tests::a_header_click_sorts_ascending_then_descending_and_a_new_column_restarts_ascending ... ok
test workspace_table::tests::sorting_is_stable_for_tied_keys_and_never_loses_a_row ... ok
test workspace_table::tests::functions_show_parameters_origin_and_summary ... ok
test workspace_table::tests::the_function_count_is_the_snapshot_registry_count ... ok
test workspace_table::tests::a_large_value_is_truncated_for_the_cell_and_kept_for_the_tooltip ... ok
test workspace_table::tests::truncation_counts_characters_not_bytes ... ok
test engine::tests::the_real_registry_classifies_builtin_plugin_and_user_functions ... ok
test workbench::tests::the_real_snapshot_publishes_underscore_and_the_registry_count_for_the_tables ... ok
test ui::app::tests::workspace_header_clicks_sort_both_tables_and_the_counts_are_the_snapshot_counts ... ok

test result: ok. 225 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
```

`--no-default-features` also compiles the test binary (the real-engine tests are `cfg`-gated and
the fake engine reports origin `builtin` with no plugin/summary).

## 4. Window proof

The session was **unlocked** (no `LogonUI`, interactive session 1), so this used the normal
harness `tools/gui-harness.ps1` for launch, UIA invocation and capture (read-only; `tools/` was
not modified). The script is `capture-workspace-tables.ps1`; the driver log is
`work/driver.log`.

Method:

1. `work/workspace-tables.lv` contains `func aaa(x) = x * 2`, `a = 21`, `aaa(a)`.
2. Launched `rust/target-desktop/debug/lovelace-desktop.exe work/workspace-tables.lv` through
   `-Action launch`; the window was restored and sized to 1500x1320 with `SetWindowPos`.
3. Invoked **Run** through UI Automation `InvokePattern` (`-Action click -Name Run`). No typing.
4. Polled the live status until `workspace-tables.lv | rev 1 | precision 1000 | Ln 1, Col 1 | run Finished`.
5. Dumped the accessibility tree (`workspace-tables-uia.txt`, 37,383 bytes).
6. Took the harness `-Action capture` PNG and a
   `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` PNG; both were opened and inspected.
7. Probed the PrintWindow PNG's exact colours (`accent=167`, `plugin-warn=79`, `dim=466`,
   sampled every 2 px).

Artifacts:

| File | Size (bytes) | Dimensions | What |
| --- | --- | --- | --- |
| `workspace-tables-printwindow.png` | **298,467** | 1500x1320 | **The proof image**: `PrintWindow` frame. Workspace shows `2 vars · 124 funcs`, `VARIABLES · 2` header row with `Name ▲ / Kind / Value`, the rows `_ Natural 42` and `a Natural 21`; `FUNCTIONS · 124` with `Name ▲ / Parameters / Origin`, the rows `aaa x user`, `abs x builtin` (summary "Absolute value (magnitude for complex v…"), `acos x plugin` (summary "The principal arccosine."), and further plugin/builtin rows. Log tail ends `[result] = 42`, `[timing] Run finished: 3 statements committed in 7 ms.` Status strip `run Finished`. |
| `workspace-tables.png` | 302,633 | 1500x1320 | Harness `-Action capture` frame of the same window and run. |
| `workspace-tables-uia.txt` | 37,383 | — | Live UIA tree (same as `work/uia-tree.txt`). |
| `capture-run.log` | 2,254 | — | Script output: pid/hwnd, resize, status, counts, capture sizes, probe. |
| `work/driver.log` | 2,070 | — | harness launch / wait / click / status / UIA / capture / PrintWindow / close timings. |
| `work/workspace-tables.lv` | 37 | — | the 3-statement script. |
| `capture-workspace-tables.ps1` | 11,423 | — | reproducible driver. |

### UIA excerpt (the count-agreement proof)

```
Text | 2 vars · 124 funcs
Text | rev 1 · committed 3/3
Text | VARIABLES · 2
Button | Name ▲ | focusable=True
Button | Kind | focusable=True
Button | Value | focusable=True
Text | _
Text | Natural
Text | 42
Text | a
Text | Natural
Text | 21
Text | FUNCTIONS · 124
Button | Name ▲ | focusable=True
Button | Parameters | focusable=True
Button | Origin | focusable=True
Text | aaa
Text | x
Text | user
Text | abs
Text | x
Text | builtin
Text | Absolute value (magnitude for complex v…
Text | acos
Text | x
Text | plugin
Text | The principal arccosine.
...
```

The tree exposed every painted function row, so the origin labels can be counted: **43 builtin +
80 plugin + 1 user = 124**, exactly the `FUNCTIONS · 124` and `2 vars · 124 funcs` counts. That is
the registry count, not a synthesized number; the script fails if the three origin rows do not sum
to the header's count.

## 5. Suite summary

```
$ cargo test -p lovelace-desktop --manifest-path rust/Cargo.toml
test result: ok. 225 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
```

- Baseline before this round: 215 passed (`baseline-test.log`).
- New: 10 tests (§3).
- `--no-default-features` test binary compiles.
