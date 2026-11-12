# RES-004 - per-field expand/collapse without data loss

Requirement (requirements.md:294): "Solve, inverse, compile, limit and transform records can be
expanded/collapsed by field without data loss."

Round 26 rendered the engine's structured projection recursively but left the expansion state in
egui's `CollapsingHeader` memory: every node defaulted open, no path-keyed state existed, the
tree could not be re-expanded reproducibly, and a UIA client could only read the header text - not
which node was collapsed. This round adds the interaction and the state model.

## 1. State model

The map lives beside the assembled frame view on the window, not inside the paint pass:

```text
committed result (display + structured JSON, from the one WorkspaceSnapshot)
        |
        | ResultIdentity::of(projection)          repr: display + compact JSON of `structured`
        v
ExpansionState { result: Option<ResultIdentity>, collapsed: BTreeMap<String, bool> }
        ^
        | bind(projection) once per painted RESULT section
        |   same identity  -> keep every path (a repaint, an edit, a quick eval)
        |   different      -> clear the map (a new result replaces the old one)
        |
paint_toggle(path, label) -- clicked() == pointer click OR Space/Enter --> toggle(path)
```

* `ExpansionState` (`results_tree.rs:116`) records only the user's decisions: a path present with
  `true` is collapsed, `false` is explicitly expanded, an absent path means the default
  **expanded**. The map never owns tree data, so hiding a node cannot discard its children.
* `ResultIdentity` (`results_tree.rs:87`) is the engine display plus the compact serialization of
  the structured projection. The buffer revision is deliberately **not** part of it: editing the
  buffer advances the revision and marks the result stale, and a quick eval adds logs/output but
  never publishes a snapshot. Neither changes the committed value, so neither clears the tree. A
  later commit that publishes a different value changes the identity and replaces the map.
* `bind` (`results_tree.rs:132`) returns true only when a previous result was replaced; a repaint
  returns false. The first bind (no previous result) does not count as a replacement.

### 1.1 Node paths

Paths are built from the projection's own names and indices
(`results_tree.rs:70,76`), so the same projection always yields the same key:

| Node | Path |
| --- | --- |
| the result itself | `result` |
| record field | `result.solutions` |
| 1-D array element | `result.solutions[0]` |
| nested record field | `result.solutions[0].value` |
| matrix row group | `result.inverse[0]` |
| matrix cell | `result.inverse[1][0]` |

### 1.2 What is collapsible

`paint_structured_node` / `paint_node_body` (`ui/app.rs:1286,1308`) give every node that has
children one header and one body. `paint_record_body` (`ui/app.rs:1492`) paints one toggle per
published field - including scalar fields, whose value row is the hidden child - and
`paint_array_body` (`ui/app.rs:1421`) paints one toggle per 1-D element and per 2-D row group.
A symbolic node is collapsible everywhere it occurs: as the result, as a field value, and as a
matrix cell (`paint_cell`, `ui/app.rs:1347`). Scalars and `Null` have no children, so they stay a
single row with no toggle.

### 1.3 Interaction and accessibility

One helper paints every header (`paint_toggle`, `ui/app.rs:1247`):

* visible text is `<indicator> <label>`, e.g. `▾ solutions: Vector 2 - 2 elements` or
  `▸ status:`; the indicator is `▾` U+25BE expanded and `▸` U+25B8 collapsed (Hack-Regular, the
  monospace font egui selects for the header, covers both - checked against the font's cmap).
* the control is an egui button, so a pointer click and Space/Enter on the focused header arrive
  as the same `clicked()` and call the same `ExpansionState::toggle`. A click moves keyboard focus
  onto the header it changed, so the keyboard can continue from the node the user touched.
* the accessible name is `<label> [<path>] <expanded|collapsed>` (`ExpansionState::accessible_name`,
  `results_tree.rs:207`), e.g.
  `solutions: Vector 2 - 2 elements [result.solutions] collapsed`. UIA therefore reads the path and
  the state, not just the label.
* the hover text repeats the path, the state and the action.

The state is threaded from the window (`WorkbenchApp::expansion`, `ui/app.rs:176`) through
`workspace_pane` into `paint_result_section` (`ui/app.rs:1180,806`). `result_toggle_rects`
(`ui/app.rs:179`) records every painted header's path and rectangle - the same test seam as
`variable_header_rects` - and is used by the end-to-end pointer test.

## 2. Tests

Full log: `cargo-test.log`; the suite is **248 passed, 0 failed, 0 ignored** (236 before this
round + 12 new). `no-default-features-test.log` compiles and runs the engine-free crate:
**232 passed, 0 failed**.

| Test | What it proves | Observed |
| --- | --- | --- |
| `results_tree::tests::collapsing_a_nested_record_field_keeps_the_children_in_the_model` | A hand-built `SolveResult` projection: toggling `result.solutions` records one path; the model still has its two `Solution` records, their fields and the `Symbolic -2` value. | `ok` |
| `results_tree::tests::re_expanding_a_collapsed_field_restores_identical_model_content` | A content digest of every headline and published value is byte-identical before the collapse and after the re-expand. | `ok` |
| `results_tree::tests::the_expansion_map_survives_a_repaint_and_a_new_result_clears_it` | Bind, collapse two paths, bind the same projection again: both decisions stay and `len` is unchanged. Bind a projection with different solution values: the map is empty and every path is expanded. | `ok` |
| `results_tree::tests::the_accessible_name_carries_the_node_path_and_the_state` | Header text starts `▾`/`▸`; the accessible name ends `[result.solutions] expanded` / `... collapsed`; a sibling keeps its own state. | `ok` |
| `results_tree::tests::paths_name_record_fields_and_array_rows` | The path grammar: fields, 1-D elements, matrix cells and a nested field. | `ok` |
| `results_tree::tests::a_real_solve_record_publishes_the_nested_fields_the_tree_collapses` | Real engine `x = symbol("x"); solve_full(x^2 - 4 == 0, x)`: `SolveResult`, the published `solutions` vector, two nested `Solution` records with `value`/`multiplicity`; collapsing `result.solutions[0]` leaves `[1]` expanded. | `ok` |
| `ui::app::tests::the_result_tree_collapses_a_nested_field_and_restores_identical_content` | Paint layer: the expanded frame paints `solutions: Vector 2 - 2 elements`, `[0] Solution - 2 fields`, `pretty     -2`. After collapsing, the header keeps the `▸` indicator but no nested row or rendering is painted; the model still has 2 elements. Re-expanding paints a byte-identical text vector. | `ok` |
| `ui::app::tests::a_click_and_space_on_a_result_header_change_the_same_state` | One pointer press/release on the painted header, then Space, then Enter. The resulting `ExpansionState` equals a model state toggled the same number of times (click = 1, click+Space = 2, click+Space+Enter = 3). | `ok` |
| `ui::app::tests::a_result_collapse_survives_edits_and_a_quick_eval_and_a_new_result_clears_it` | A repaint with revision 99, new output/log/run state and the same committed projection keeps the collapse. A different solution set clears the map and paints expanded with its own values. | `ok` |
| `ui::app::tests::the_result_tree_collapses_a_symbolic_result_without_losing_it` | A `Symbolic` result: the header carries `▾ Symbolic  x^2 - 1`; collapsing `result` hides both `pretty` and `canonical`, the model still has them, and re-expanding is byte-identical. | `ok` |
| `ui::app::tests::an_array_row_collapses_independently_of_its_sibling` | The `solutions` vector: collapsing `result.solutions[0]` hides row 0's symbolic value and leaves row 1's value untouched; both rows stay in the model; re-expanding is byte-identical. | `ok` |
| `ui::app::tests::a_window_click_collapses_a_field_and_an_edit_keeps_it` | Real engine through the whole window at 2500x1380: a pointer click on the rect the window painted for `result.solutions` collapses it; editing the buffer afterwards keeps it collapsed. | `ok` |

Observed lines (from `cargo-test.log`):

```
test results_tree::tests::collapsing_a_nested_record_field_keeps_the_children_in_the_model ... ok
test results_tree::tests::re_expanding_a_collapsed_field_restores_identical_model_content ... ok
test results_tree::tests::the_expansion_map_survives_a_repaint_and_a_new_result_clears_it ... ok
test results_tree::tests::the_accessible_name_carries_the_node_path_and_the_state ... ok
test results_tree::tests::paths_name_record_fields_and_array_rows ... ok
test results_tree::tests::a_real_solve_record_publishes_the_nested_fields_the_tree_collapses ... ok
test ui::app::tests::the_result_tree_collapses_a_nested_field_and_restores_identical_content ... ok
test ui::app::tests::a_click_and_space_on_a_result_header_change_the_same_state ... ok
test ui::app::tests::a_result_collapse_survives_edits_and_a_quick_eval_and_a_new_result_clears_it ... ok
test ui::app::tests::the_result_tree_collapses_a_symbolic_result_without_losing_it ... ok
test ui::app::tests::an_array_row_collapses_independently_of_its_sibling ... ok
test ui::app::tests::a_window_click_collapses_a_field_and_an_edit_keeps_it ... ok
```

The round-26 RES-001 negative test still runs and still proves the display is never parsed; only
its header-text expectation changed from `Array 2 x 2 - 4 elements` to the new glyph-prefixed
`▾ Array 2 x 2 - 4 elements`, and its "no field label" assertion now checks that no painted text
ends in `quotient:` (the display caption itself contains that word, so an exact-text check would
have silently stopped testing the field label).

## 3. Window proof

**Method.** The console session was unlocked (interactive session 1), so the normal harness
(`tools/gui-harness.ps1`) drove the shipped `lovelace-desktop.exe` for launch, UIA invocation,
UIA dumps and the Space key; the two frames are `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`
after a `RedrawWindow` and a 2500x1380 `SetWindowPos`, the established full-frame capture for this
desktop. `tools/` was not modified. Script: `capture-expand-collapse.ps1`; driver log:
`work/driver.log`; script log: `capture-run.log`.

Document (`work/solve-record.lv`, 46 bytes):

```
x = symbol("x")
solve_full(x^2 - 4 == 0, x)
```

Steps recorded in `capture-run.log`:

1. Launched the exe on that document (`launch pid=25044`), waited for the window
   (`hwnd=28773686`), restored it to 2500x1380.
2. Invoked **Run** through UIA `InvokePattern`; polled the live status until
   `solve-record.lv | rev 1 | precision 1000 | Ln 1, Col 1 | run Finished`.
3. Found the 12 direct field headers by their accessible path (the root is `[result]`, nested rows
   add brackets, so the 12 are exactly `result.<name>`), and invoked each one; after the loop all
   12 names read `collapsed`.
4. Captured frame 1: `expand-collapse-collapsed.png`.
5. Dumped UIA (`expand-collapse-collapsed-uia.txt`) and asserted all 12 fields read `collapsed`,
   and that `Solution - `, `[result.solutions[0]]`, `pretty     ` and `canonical  ` are absent.
6. Expanded exactly one field, `solutions`, **by keyboard**: UIA `SetFocus` on its header, then the
   harness sent `Space` to the foreground window (`keys pid=25044 keys= `).
   `expanded result.solutions method=keyboard Space on the UIA-focused header` - no fallback was
   needed.
7. Captured frame 2: `expand-collapse-expanded.png`.
8. Dumped UIA (`expand-collapse-expanded-uia.txt`) and asserted
   `[result.solutions] expanded`, `[result.solutions[0]] expanded`,
   `[result.solutions[1]] expanded`, `[result.solutions[0].value] expanded`, `pretty`,
   `canonical`, and - proving only the named field moved - `[result.status] collapsed`.
9. Probed both frames for the palette (`4EC9B0` accent, `D8DEE9` text, `8B98A9` dim) and required
   the two SHA-256 hashes to differ. UIA node counts: 573 collapsed, 607 expanded.

Artifacts:

| File | Bytes | Dimensions | SHA-256 | What |
| --- | --- | --- | --- | --- |
| `expand-collapse-collapsed.png` | **422,290** | 2500x1380 | `68AEC26804FD88A55772387F3B12439D0FE81264A1FD5B606F29F638A90B5767` | **Proof image 1**: PrintWindow frame, all 12 fields collapsed. The RESULT tree reads `▾ SolveResult - 12 fields`, then `▸ status:`, `▸ variable: Symbolic x`, `▸ domain:`, `▸ complete:`, `▸ completeness:`, `▸ solutions: Vector 2 - 2 elements`, `▸ families: Vector 0 - 0 elements`, `▸ common_conditions: ...`, `▸ represented_count:`, `▸ unrepresented_count:`, `▸ unrepresented_reason:`, `▸ diagnostics: Vector 0 - 0 elements`. No field value, row or symbolic rendering is painted. VARIABLES shows `_ Record SolveResult( status...` and `x Symbolic x`; status strip `run Finished`. |
| `expand-collapse-expanded.png` | **420,715** | 2500x1380 | `08D392DEBB4403EB9516DCAAA860E5F9DB28CE49F8E3F3E0161FC8BEBACFB43B` | **Proof image 2**: PrintWindow frame of the same result with `solutions` expanded. `▾ solutions: Vector 2 - 2 elements` now owns `▾ [0] Solution - 4 fields` (`▾ value: Symbolic -2` with `pretty -2`, `canonical (rat -2 1)`, `domain rational`, `free x`, `nodes 1`; `▾ conditions: Vector 0 - 0 elements` -> `empty`; `▾ multiplicity:` -> `Integer 1 exact=true`; `▾ exactness:` -> `Enum Exact type=SolutionExactness`) and `▾ [1] Solution - 4 fields` with `pretty 2` / `canonical (rat 2 1)`. The other 10 fields stay `▸` collapsed, so the frame shows both states side by side. |
| `expand-collapse-collapsed-uia.txt` | 39,129 | - | - | Live UIA tree after step 5 (573 nodes). |
| `expand-collapse-expanded-uia.txt` | 41,747 | - | - | Live UIA tree after step 8 (607 nodes). |
| `capture-expand-collapse.ps1` | 16,604 | - | - | Reproducible driver (PS 5.1, ASCII). |
| `capture-run.log` | 1,629 | - | - | Script output: ids, status, 12 paths, capture sizes, method, palette, hashes. |
| `work/driver.log` | 1,143 | - | - | Harness launch / wait / click / keys / UIA / close timings. |
| `work/solve-record.lv` | 46 | - | - | The 2-statement script. |
| `work/uia-tree.txt` | 41,747 | - | - | Last harvester tree (expanded). |

Both PNGs were opened and inspected after the final capture run; the indicators are the glyphs
`▸` and `▾`, not egui's own painted arrow (the old `CollapsingHeader` is gone from the tree).
Frame 1's accent = 445, text = 43, dim = 391 sampled pixels; frame 2's accent = 542, text = 58,
dim = 365 (the collapsed frame hides most TEXT-coloured values on purpose, so its exact-TEXT count
is lower).

### 3.1 UIA excerpts

Collapsed frame (every field is listed with its path and state; nested content is gone):

```
Button | SolveResult - 12 fields [result] expanded
Button | status: [result.status] collapsed
Button | variable: Symbolic  x [result.variable] collapsed
Button | domain: [result.domain] collapsed
Button | complete: [result.complete] collapsed
Button | completeness: [result.completeness] collapsed
Button | solutions: Vector 2 - 2 elements [result.solutions] collapsed
...
Button | diagnostics: Vector 0 - 0 elements [result.diagnostics] collapsed
```

Expanded frame (one field moved; its nested nodes and renderings are back):

```
Button | solutions: Vector 2 - 2 elements [result.solutions] expanded
Button | [0] Solution - 4 fields [result.solutions[0]] expanded
Button | value: Symbolic  -2 [result.solutions[0].value] expanded
Button | conditions: Vector 0 - 0 elements [result.solutions[0].conditions] expanded
Button | multiplicity: [result.solutions[0].multiplicity] expanded
Button | exactness: [result.solutions[0].exactness] expanded
Text | pretty     -2
Text | canonical  (rat -2 1)
Text | domain     rational
Text | nodes      1
Button | [1] Solution - 4 fields [result.solutions[1]] expanded
Text | pretty     2
Text | canonical  (rat 2 1)
...
Button | status: [result.status] collapsed
```

## 4. Suite summary

```
$ cargo test -p lovelace-desktop --manifest-path rust/Cargo.toml
test result: ok. 248 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s
```

```
$ cargo test -p lovelace-desktop --no-default-features --manifest-path rust/Cargo.toml
test result: ok. 232 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s
```

- Before the round: 236 passed. New: 12 tests (§2).
- This round's changes are limited to `rust/lovelace-desktop/src/results_tree.rs` and
  `rust/lovelace-desktop/src/ui/app.rs`, plus this directory's artifacts; no engine source was
  edited as part of RES-004.
- `cargo build -p lovelace-desktop` completes without warnings.

## 5. Code points

| Piece | Location |
| --- | --- |
| Indicators (`▾`/`▸`) | `results_tree.rs:27,30` |
| Path grammar | `results_tree.rs:70,76` |
| `ResultIdentity` | `results_tree.rs:87` |
| `ExpansionState` + `bind`/`toggle`/state word | `results_tree.rs:116-210` |
| Window state field | `ui/app.rs:176` |
| Painted header rectangles | `ui/app.rs:179,806,1014` |
| RESULT section binding + paint | `ui/app.rs:1180` |
| One toggle header (click / Space / Enter) | `ui/app.rs:1247` |
| Node / body painters | `ui/app.rs:1286,1308` |
| Matrix cell toggle | `ui/app.rs:1347` |
| Array row/groups | `ui/app.rs:1421` |
| Record fields | `ui/app.rs:1492` |
| Model tests | `results_tree.rs:869-1113` |
| Paint + window tests | `ui/app.rs:3313-3685` |
