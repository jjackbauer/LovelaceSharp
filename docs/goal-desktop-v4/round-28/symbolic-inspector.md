# SYM-001 — symbolic inspector for the last committed result

Requirement (requirements.md:307): "Display pretty, canonical, expression tree, current
assumptions, simplification trace and MathIR, where engine supplies them."

Rounds 26–27 rendered a symbolic value's pretty and canonical text inside the RESULT tree. This
round adds a dedicated **Inspector** view to the Workspace pane, bound to the last **committed**
result, and determines from the engine read APIs what each of the six sections can show. It does
not modify any engine crate.

## 1. The six sections and the engine source

The engine adapter captures one inspection payload at the same moment it captures the result's
display and structured projection (`engine/real.rs:73`). The payload is carried with the
committed snapshot; the window only renders it.

| Section | Engine source (read-only) | Supplied? | What the Inspector shows |
| --- | --- | --- | --- |
| **pretty** | `lovelace_lang::structured::project_value(subject, None).pretty` — the same projection RES-001 renders | Yes for a symbolic subject: a `Symbolic` value, or a `TransformResult`'s `expression` field | Heading `Pretty`, then the engine's rendering in monospace, selectable text (`x^2 …`) |
| **canonical** | `structured::project_value(subject, None).canonical` | Yes for the same subjects | Heading `Canonical`, then the engine's canonical S-expression (`(mul (rat 3 1) …)`) |
| **expression tree** | the engine's own `lovelace_symbolics::Expr` carried by `Value::Symbolic`; `engine/inspection.rs:251` walks its variants (`Num`, `Sym`, `Add`, `Mul`, `Pow`, `Fun`, `Div`, `Rel`, `Real`) and uses `Expr::to_string()` for each node's label | Yes for a symbolic subject. **Not present in `structured::project_value`** | Heading `Expression tree · N nodes`; each node is `kind [detail] · engine rendering`, an expandable header (▾/▸) with the engine's child order; leaves are indent-only rows |
| **assumptions** | `lovelace_lang::builtins::call(engine, "inspect", [subject], 0)` → the `Inspection.assumptions` vector; plus a `TransformResult`'s `conditions` field | Yes for a symbolic subject. The engine supplies the session atoms that constrain the subject's free symbols (possibly zero); a `TransformResult` adds its result conditions as a second source | Heading `Assumptions · N rows`; each row is typed: `[session|result] relation > x > 0` with canonical `(gt (sym x) (rat 0 1))`, or `[result] FiniteCondition …`. Records stay records; a sentence is never split |
| **simplification trace** | a `TransformResult`'s `steps` field (the record `simplify_full` publishes), including `rule_id`, `classification`, `before`, `after`, `required_conditions` | **Only when the result is a `TransformResult`** — i.e. the script called `simplify_full`. A plain `diff`/`simplify`/`Symbolic` result has no trace and says so | Heading `Simplification trace · N steps`; per step: `1. logexp.exp-log`, `classification  Conditional`, `before  exp(log(x))`, `after   x`, canonical lines, and typed `requires` conditions |
| **MathIR** | `lovelace_lang::mathir::compile_ordered(subject, subject.free_symbols())` — the same compile path `compile`/`compile_full` use (`builtins.rs:1834`) | Yes for a symbolic subject with at least one free symbol that lowers and validates. A refusal is the engine's own `LangError.message`; zero free symbols is the engine's "at least one parameter" rule | Heading `MathIR · compile · parameters [x] · result <type>`, then the serialized IR in monospaced, selectable text inside a bounded `ScrollArea::both` |

### 1.1 What the engine does *not* supply

* `structured::project_value` carries only the symbolic pretty/canonical/domain/exact/nodeCount/
  freeSymbols fields. It has no tree, no assumptions, no trace and no IR. The tree therefore
  comes from the engine's expression value itself (the same value the projection was made from),
  not from the projection and never from pretty text. No symbolic logic is reimplemented: the
  adapter reads the engine's enum variants, `Expr::to_string()`, `Expr::node_count()`,
  `Expr::free_symbols()` and the public `inspect` and `mathir::compile_ordered` entry points.
* There is no `diff_full` in the engine registry. `diff` and `simplify` return a `Symbolic`
  value, so they supply five of six sections and the trace section shows the reason. `simplify_full`
  returns a `TransformResult`, the only carrier of rewrite steps.
* MathIR is absent for a relation (`Node kind Relation cannot be lowered to MathIR.`), for a
  symbolic subject with no free symbols (the compile path's parameter refusal), and for any
  lowering/validation refusal; the engine's exact sentence is what the pane shows.
* Session assumptions are the **relevant** atoms `inspect` publishes for the subject's free
  symbols, not the engine's whole assumption set. The result conditions of a `TransformResult`
  are a separate, explicitly labelled source.

## 2. Bound to the last committed result, never the buffer

```text
RealEngine::eval(src)
    -> Value
    -> structured::project_value(engine, value)          (display structure, RES-001)
    -> engine::inspection::capture(engine, value)        (SYM-001 payload)
    -> Evaluator::last_structured / last_inspection
    -> RunOutput.result_structured / result_inspection
    -> execution::Progress -> WorkspaceSnapshot
    -> workbench::DocumentView
    -> ui::app::paint_inspector
```

* The payload is produced before `eval` returns, so it describes exactly the value the same
  commit publishes. No script is evaluated again for the inspector.
* `WorkspaceSnapshot.result_inspection` (`execution.rs:71`) is copied into `DocumentView`
  (`workbench.rs:224,1343`). The editor buffer has no path into either.
* Editing the buffer advances the document revision and marks the result stale, but publishes no
  snapshot: the Inspector payload, tree and rendered text do not move. A quick eval also never
  publishes a snapshot. A new committed result publishes a new payload in the same
  `StepCommitted`/`Finished` snapshot.
* The Workspace pane has two views over that one snapshot: `Result` (existing RESULT tree,
  VARIABLES and FUNCTIONS) and `Inspector` (this payload). `Inspector` is a real button
  (`ui/app.rs:826`). Its tree expansion map (`InspectorExpansion`, `inspector.rs:415`) is keyed by
  the payload's compact serialization, so a buffer edit keeps every collapse decision and a new
  committed result clears it.

## 3. What the UI does for each missing state

| State | Painted |
| --- | --- |
| No result yet | `No run has published a result yet.` |
| Engine publishes no inspection payload (e.g. the deterministic fake) | One explicit sentence naming all six sections as unsupplied by that engine |
| Result is not symbolic (`Natural`, `SolveResult`, `Text`, …) | Six section headings, each with its own specific reason, e.g. "The engine publishes pretty for a Symbolic value or a TransformResult's expression. The last committed result is Natural." |
| `Symbolic` result (diff/simplify) | pretty, canonical, tree, assumptions, MathIR; trace heading with "only a TransformResult (simplify_full) has steps" |
| `TransformResult` | the six sections with the subject `result.expression`; trace steps; session assumptions and result conditions both labelled |
| MathIR refusal | the engine's own sentence under `MathIR`; pretty/canonical/tree/assumptions/trace stay rendered |
| Empty assumptions / empty trace supplied by the engine | `No active assumptions constrain the free symbols of this result.` / `recorded 0 rewrite steps` — an engine answer, not an empty box |

Tree headers carry the indicator, node kind, engine rendering, node path and state in their
accessible name, e.g. `Mul · 3*x^2 [inspect.tree] expanded`; a click or Space/Enter toggles the
same path. MathIR is selectable (`Label::selectable(true)`), monospaced and scrollable inside the
pane. The payload headings are painted amber when a section is unsupplied.

## 4. Tests

`cargo test -p lovelace-desktop`: **261 passed, 0 failed, 0 ignored** (248 before this round + 13
new). Full log: `cargo-test.log`. `--no-default-features`: **238 passed, 0 failed, 0 ignored**;
log: `no-default-features-test.log`.

| Test | What it proves | Observed |
| --- | --- | --- |
| `inspector::tests::a_full_payload_reads_all_six_sections_from_the_engine_data` | The typed model reads all six sections, the tree count/child order, the typed relation fields, the trace step and the MathIR parameters from the payload. | `ok` |
| `inspector::tests::a_missing_mathir_keeps_its_reason_and_leaves_the_other_sections` | A `{reason}` MathIR section is `Missing` with exactly the engine sentence; pretty/canonical/tree remain `Supplied`; an omitted trace is a specific reason, never `None`. | `ok` |
| `inspector::tests::the_expansion_map_survives_the_same_payload_and_clears_on_a_new_one` | Binding the same payload twice keeps the collapsed path; a different payload clears the map. | `ok` |
| `inspector::tests::tree_paths_name_child_order` | `inspect.tree.0` / `inspect.tree.0.1` path grammar. | `ok` |
| `engine::inspection::tests::a_symbolic_expression_projects_its_tree_from_the_engine_ast` | Real engine `diff(x^3, x)`: pretty `3*x^2`, canonical `(mul …)`, tree root `Mul` with 2 children, MathIR starts `#!mathir 2` and contains `PowInt`, trace reason names `simplify_full`. | `ok` |
| `engine::inspection::tests::inspect_publishes_the_session_assumptions_as_typed_relations` | After `assume(x > 0)`, the `inspect` path publishes one `session`/`Relation` row with op `gt`, operands `x` and `0`, canonical `(gt …)`. | `ok` |
| `engine::inspection::tests::a_transform_result_publishes_the_steps_and_its_conditions` | Real `simplify_full(exp(log(x)))`: subject `TransformResult` at `result.expression`; one `logexp.exp-log`/`Conditional` step, before `exp(log(x))`, after `x`, required `FiniteCondition`; result conditions and session assumptions both published; MathIR has `param x`. | `ok` |
| `engine::inspection::tests::a_relation_that_cannot_lower_reports_the_engine_reason_and_keeps_the_tree` | Real `x < 0`: MathIR reason is exactly `Node kind Relation cannot be lowered to MathIR.`; tree root is `Rel`; pretty `x < 0`. | `ok` |
| `engine::inspection::tests::a_non_symbolic_result_reports_a_specific_reason_in_every_section` | Real `1 + 1`: subject kind `Natural`; each of the six sections carries a reason naming `Natural`. | `ok` |
| `ui::app::tests::the_inspector_paints_all_six_sections_from_the_engine_payload` | Paint layer: all six headings and values (pretty, canonical, `Mul · 3*x^2`, `relation >`, `before  exp(log(x))`, `#!mathir 2`) are painted; the result `display:` caption is not. | `ok` |
| `ui::app::tests::a_missing_mathir_shows_the_reason_and_leaves_the_other_sections_usable` | The engine refusal sentence is painted while pretty/canonical/tree/assumptions are still painted. | `ok` |
| `ui::app::tests::the_inspector_is_bound_to_the_committed_result_not_the_buffer` | Real engine through the window: committing `diff(x^3, x)` paints `3*x^2` and `x > 0`; editing the buffer to `1 + 1` changes neither the payload nor one painted byte; committing `diff(x^4, x)` changes the payload and paints `4*x^3` with `3*x^2` gone. | `ok` |
| `ui::app::tests::the_workspace_inspector_tab_swaps_the_view_and_the_result_tab_restores_it` | A real pointer click on the painted Workspace `Inspector` tab swaps the view (INSPECTOR painted, `RESULT · structured projection` gone); a click on `Result` restores it. | `ok` |

Observed lines (`cargo-test.log`):

```text
test engine::inspection::tests::a_non_symbolic_result_reports_a_specific_reason_in_every_section ... ok
test engine::inspection::tests::a_relation_that_cannot_lower_reports_the_engine_reason_and_keeps_the_tree ... ok
test engine::inspection::tests::a_symbolic_expression_projects_its_tree_from_the_engine_ast ... ok
test engine::inspection::tests::a_transform_result_publishes_the_steps_and_its_conditions ... ok
test engine::inspection::tests::inspect_publishes_the_session_assumptions_as_typed_relations ... ok
test inspector::tests::a_full_payload_reads_all_six_sections_from_the_engine_data ... ok
test inspector::tests::a_missing_mathir_keeps_its_reason_and_leaves_the_other_sections ... ok
test inspector::tests::the_expansion_map_survives_the_same_payload_and_clears_on_a_new_one ... ok
test inspector::tests::tree_paths_name_child_order ... ok
test ui::app::tests::a_missing_mathir_shows_the_reason_and_leaves_the_other_sections_usable ... ok
test ui::app::tests::the_inspector_is_bound_to_the_committed_result_not_the_buffer ... ok
test ui::app::tests::the_inspector_paints_all_six_sections_from_the_engine_payload ... ok
test ui::app::tests::the_workspace_inspector_tab_swaps_the_view_and_the_result_tab_restores_it ... ok
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s
```

## 5. Window proof

**Method.** The console session was unlocked (interactive session 1), so the normal harness
(`tools/gui-harness.ps1`) drove the shipped `lovelace-desktop.exe`: launch on the document, UIA
`InvokePattern` on **Run**, status poll to `run Finished`, UIA `SetFocus` on the Workspace
**Inspector** tab and a Space key to select it (egui's selectable tab exposes `Button` but not
`InvokePattern`; the round-27 keyboard route is reused; a raw pointer click at the published
bounding rectangle is the recorded fallback and was not needed). Frames are
`PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` after `BringWindowToTop`, `SetForegroundWindow`,
`RedrawWindow` and a 2500x1380 `SetWindowPos`; the capture function retries a blank DWM surface.
`tools/` was not modified. Script: `capture-inspector.ps1`; driver log: `work/driver.log`;
script log: `capture-run.log`.

Scenario 1 (`work/symbolic-diff.lv`, 46 bytes) — a **Symbolic** result:

```text
x = symbol("x")
assume(x > 0)
diff(x^3, x)
```

Scenario 2 (`work/simplify-trace.lv`, 60 bytes) — a **TransformResult** with a rewrite step:

```text
x = symbol("x")
assume(x > 0)
simplify_full(exp(log(x)))
```

Steps recorded in `capture-run.log`: launch (pid/hwnd), resize to 2500x1380, invoke Run, status
`run Finished`, select Inspector by keyboard, PrintWindow, UIA dump, 20/16 required strings
asserted, palette probe, SHA-256; then close and repeat for the second document. UIA node counts:
79 for the diff frame, 88 for the trace frame (selecting Inspector replaces the Result view, so
the harvested tree is small). The two frames have different SHA-256 hashes, proving the inspector
moved with the result.

Artifacts:

| File | Bytes | Dimensions | SHA-256 | What |
| --- | --- | --- | --- | --- |
| `symbolic-diff.png` | **358,025** | 2500x1380 | `273FA80285B371316BF3698B238157F7D816425950882C1F67686CB0CF7B32C3` | **Proof image 1**: the Inspector tab selected for `diff(x^3, x)`. The pane reads `INSPECTOR · last committed result`, `subject Symbolic · result`, Pretty `3*x^2`, Canonical `(mul (rat 3 1) (pow (sym x) (rat 2 1)))`, `Expression tree · 5 nodes` with `▾ Mul · 3*x^2` → `Num · 3`, `▾ Pow · x^2` → `Sym · x`, `Num · 2`; `Assumptions · 1 row` with `[session] relation > x > 0` and its canonical form; `Simplification trace` with the explicit "only a TransformResult (simplify_full)" reason; `MathIR · compile · parameters [x] · result Integer` with the full 9-line IR ending `Mul 0 0,3`. |
| `simplify-trace.png` | **377,769** | 2500x1380 | `1EB8826D4F18820E200E407169A05DE14318C0D200EC3DCC054D7151E5418AFB` | **Proof image 2**: the Inspector for `simplify_full(exp(log(x)))`. `subject TransformResult · result.expression · Satisfied`; pretty `x`, canonical `(sym x)`, tree `Sym · x`; `Assumptions · 2 rows` (`[session] relation > x > 0`, `[result] FiniteCondition …`); `Simplification trace · 1 step` = `1. logexp.exp-log`, `classification Conditional`, `before exp(log(x))` with canonical, `after x` with canonical, `requires FiniteCondition(…log(x))`; `MathIR · compile · parameters [x]` with `#!mathir 2 / param x / Parameter 0 0`. |
| `symbolic-diff-uia.txt` | 6,056 | - | - | Live UIA tree for proof image 1; 20 required engine strings asserted. |
| `simplify-trace-uia.txt` | 6,798 | - | - | Live UIA tree for proof image 2; 16 required engine strings asserted. |
| `capture-inspector.ps1` | 15,379 | - | - | Reproducible driver (PS 5.1, ASCII). |
| `capture-run.log` | 1,743 | - | - | Script output: ids, status, method, sizes, assertions, palette, hashes. |
| `work/driver.log` | 1,367 | - | - | Harness launch/wait/click/keys/UIA/close log. |
| `work/symbolic-diff.lv`, `work/simplify-trace.lv` | 46, 60 | - | - | The two scripts. |
| `work/uia-tree.txt` | 6,798 | - | - | Last harvester tree (trace scenario). |
| `cargo-test.log` | 46,308 | - | - | Full default-feature suite. |
| `no-default-features-test.log` | 41,710 | - | - | Engine-free suite. |
| `cargo-build.log` | 1,040 | - | - | `cargo build -p lovelace-desktop`, no warnings. |

Both PNGs were opened and inspected after the final capture run. The palette probe on
`symbolic-diff.png` found accent `4EC9B0` 89 px, text `D8DEE9` 122 px, dim `8B98A9` 318 px;
on `simplify-trace.png` 303 / 76 / 257. (The inspector pane is narrow by the shipped 0.20
Workspace fraction, so text wraps; every section is still visible and readable at 2500x1380.
The accent count varies with the caret/focus state of the frame; the threshold only requires
the pane palette to be present.)

### 5.1 UIA excerpts

`symbolic-diff-uia.txt` (the tree replaces the Result view; the six headings and the engine values
are real UIA Text/Button nodes):

```text
Button | Inspector | id= | focusable=True | enabled=True
Text | INSPECTOR · last committed result | ...
Text | subject  Symbolic · result | ...
Text | Pretty | ...
Text | 3*x^2 | ...
Text | Canonical | ...
Text | (mul (rat 3 1) (pow (sym x) (rat 2 1))) | ...
Text | Expression tree | ...
Text | 5 nodes | ...
Button | Mul · 3*x^2 [inspect.tree] expanded | ...
Text | Num · 3 | ...
Button | Pow · x^2 [inspect.tree.1] expanded | ...
Text | Assumptions | ...
Text | relation > | ...
Text | x > 0 | ...
Text | Simplification trace | ...
Text | The engine publishes rewrite steps only in a TransformResult (simplify_full). ...
Text | MathIR | ...
Text | #!mathir 2
param x
...
PowInt 0 1,2
Mul 0 0,3 | ...
```

`simplify-trace-uia.txt`:

```text
Text | subject  TransformResult · result.expression · Satisfied | ...
Text | Pretty | ...
Text | x | ...
Text | Canonical | ...
Text | (sym x) | ...
Text | Expression tree | ...
Text | Assumptions | ...
Text | [session] | ...
Text | relation > | ...
Text | [result] | ...
Text | FiniteCondition | ...
Text | Simplification trace | ...
Text | 1. logexp.exp-log | ...
Text | classification  Conditional | ...
Text | before  exp(log(x)) | ...
Text | after   x | ...
Text | FiniteCondition | ...
Text | MathIR | ...
Text | #!mathir 2
param x
... | ...
```

## 6. Honest limits of this round

* The expression tree is the engine's `Expr`, not a field of the structured projection. Node
  kinds are the engine's enum variants, child order is the engine's, and labels are the engine's
  `Display`; no display text is parsed into structure. This is recorded in the table above rather
  than claimed as projection data.
* The trace is only the engine's `TransformResult.steps`. A `diff` result has no trace, and the
  UI says exactly that instead of showing an empty box. There is no engine `diff_full`.
* Assumptions are the engine's relevant session atoms from `inspect`; the adapter does not read
  the full assumption set (the engine has no public whole-set accessor) and does not invent one.
* MathIR is compiled at commit time from the captured expression and its free symbols through the
  engine's compile path; it is not stored by the engine with the value. A compilation refusal is
  shown verbatim.
* This round implements expand/collapse and accessible paths for the tree, but not node selection,
  copy-node-canonical, keyboard tree traversal beyond button focus, MathIR copy/export, or bounded
  virtualisation of very large trees (SYM-003/007/008/010/011 are later requirements).
* The inspector is Windows/egui UI only. Without the `real-engine` feature the deterministic fake
  publishes no inspection, and the pane shows its explicit "no payload" state; the 238-test
  engine-free suite covers that path.

## 7. Code points

| Piece | Location |
| --- | --- |
| Inspection payload model (`Section`, `TreeNode`, `Assumption`, `TraceStep`, `MathIr`, `Inspection`) | `inspector.rs:159` |
| Payload parser | `inspector.rs:182` |
| Tree collapse state + paths | `inspector.rs:415,490,493` |
| Engine capture entry point | `engine/inspection.rs:26` |
| Expression-tree projection from the engine AST | `engine/inspection.rs:251` |
| Assumptions via `inspect` + result conditions | `engine/inspection.rs:307` |
| Trace from `TransformResult.steps` | `engine/inspection.rs:392` |
| MathIR via `compile_ordered` | `engine/inspection.rs:449` |
| Non-symbolic six-reason payload | `engine/inspection.rs:175` |
| Real adapter capture point | `engine/real.rs:73` |
| `Evaluator::last_inspection` | `engine.rs:391` |
| Snapshot field / commit capture | `execution.rs:71,750,878` |
| `DocumentView` field | `workbench.rs:224,1343` |
| Workspace Result/Inspector tabs | `ui/app.rs:784,821,834` |
| Inspector painter | `ui/app.rs:1229` |
| Tree node painter | `ui/app.rs:1499` |
| Payload-independent counts | `ui/app.rs:1437` |

This round's own edits are limited to `rust/lovelace-desktop/**` and this directory; the
workspace `rust/Cargo.lock` records the desktop's new path dependency on `lovelace-symbolics`.
No engine crate source, `tools/` or gate file was modified.
