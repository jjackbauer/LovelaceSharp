# Census: desktop SVG reader and Rust engine result surface

Read-only census. No quality judgement, no proposed changes. Every fact below was read in the cited file; nothing is inferred from a name alone.

## Read set and scope application

- `rust/lovelace-desktop/src/svg.rs` — read in full (664 lines).
- `rust/lovelace-lang/src/lib.rs` — read in full (2115 lines, including the `#[cfg(test)]` test modules' names and the public re-exports at the end).
- `rust/lovelace-lang/src/structured.rs` — read in full (293 lines). It is reached one hop from `lib.rs`: `pub mod structured;` at `lib.rs:24`, and its module doc says it is the public projection defined "so the desktop host can publish the same structure" (`structured.rs:7-13`).
- `rust/lovelace-lang/src/registry.rs` — read lines 1-31 only: `BuiltinInfo` is the two-hop input type of `structured::descriptor_json` (`lib.rs` -> `structured` -> `registry`).
- `rust/lovelace-lang/src/syntax.rs` — read lines 1-166 only: `BinOp`, `Tok`, `Token` are the public re-exports at `lib.rs:1881` and `lib.rs:1883`; `Expr` is the return type of the public `parse_expression` at `lib.rs:1869`.

Not read: `win32/`, `app.rs`, `selftest.rs`, `builtins.rs`, `modus.rs`, `context.rs`, `mathir.rs`, the remaining `syntax.rs`, any other crate (including `lovelace-symbolics`, `lovelace-json`, `lovelace-nat`, `lovelace-int`, `lovelace-real`, `lovelace-array`), and any other file in the repository. No cargo command was run (build lock held by another process). Only this deliverable was written.

Confidence values: `High` = the behaviour is the body of the cited line/arm. `High (enumeration)` = an absence established by enumerating every relevant arm or call site in the cited range. `Medium` = the statement combines several branches.

## 1. OBS table

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-001 | `svg.rs:1-2` | The module describes itself as `A deliberately tiny SVG reader, plus the generator that produces the plot documents it consumes.` | High |
| OBS-002 | `svg.rs:5-7` | The engine publishes a plot as an SVG string `produced by the workspace's own code, not by a third-party renderer`, using `a small, fixed subset of SVG: a root svg element with a size, an optional title element, one or more polyline elements with a points attribute and a handful of text labels.` | High |
| OBS-003 | `svg.rs:8-9` | `The plot pane does not need a general renderer, so this module parses exactly that subset and ignores everything else.` | High |
| OBS-004 | `svg.rs:9-11` | `Nothing here uses floating point`; coordinates are `kept in thousandths of an SVG user unit (MILLI) so the GDI layer can scale them with integer arithmetic.` | High |
| OBS-005 | `svg.rs:13` | `pub const MILLI: i64 = 1000;` | High |
| OBS-006 | `svg.rs:15-22` | `Point { x: i64, y: i64 }` is documented as `A point in thousandths of an SVG user unit.` | High |
| OBS-007 | `svg.rs:24-31` | `Polyline { points: Vec<Point>, stroke: Option<String> }`; `points` are `Vertices, in document order.`; `stroke` is `The stroke attribute, if present.` | High |
| OBS-008 | `svg.rs:33-43` | `TextAnchor` variants are `Start` (marked `#[default]`), `Middle`, `End`, documented as left-aligned at x, centred on x, right-aligned at x. | High |
| OBS-009 | `svg.rs:45-60` | `TextItem` has `x: i64`, `y: i64`, `text: String`, `size: i64`, `anchor: TextAnchor`, `bold: bool`; docs say baseline origin, `text content, with entity references decoded`, font size in thousandths, `Text anchor`, whether `font-weight asked for bold.` | High |
| OBS-010 | `svg.rs:62-77` | `Plot` has `width: i64`, `height: i64`, `view_box: Option<[i64; 4]>`, `title: Option<String>`, `polylines: Vec<Polyline>`, `texts: Vec<TextItem>`; width/height are in thousandths; polylines and texts are `in document order.` | High |
| OBS-011 | `svg.rs:79-90` | `Plot::default()` sets `width: 640 * MILLI`, `height: 400 * MILLI`, and `None`/empty for view_box, title, polylines, texts. | High |
| OBS-012 | `svg.rs:92-97` | `Plot::is_empty` is `self.polylines.is_empty() && self.texts.is_empty() && self.title.is_none()`. | High |
| OBS-013 | `svg.rs:108-110` | `parse` doc: `Unknown elements are ignored, and malformed input yields whatever was parsed before the problem rather than an error`; the function returns `Plot`, not a `Result`. | High |
| OBS-014 | `svg.rs:111-116` | `parse` starts `let mut plot = Plot::default();`, advances with `next_tag`, and matches on `tag.name`. | High |
| OBS-015 | `svg.rs:117-125` | The `"svg" if !tag.close` arm reads `width` and `height` via `parse_milli`, filters them with `.filter(\|v\| *v > 0)`, and reads `viewBox` via `attr(&attrs, "viewBox").and_then(parse_view_box)`. | High |
| OBS-016 | `svg.rs:126-138` | Size match: `(Some(w), Some(h), vb) =>` sets width, height and `plot.view_box = vb`; `(w, h, Some(vb)) =>` fills a missing width/height from `vb[2]`/`vb[3]`; `_ => {}` leaves the `Plot::default()` size. | High |
| OBS-017 | `svg.rs:140-147` | `title` is parsed only for `!tag.close && !tag.self_closing`; content is read to `</title>`, `text.trim()`-ed, and stored only `if !text.is_empty()`. | High |
| OBS-018 | `svg.rs:149-156` | `text` is parsed only for `!tag.close && !tag.self_closing`; `x` and `y` use `parse_milli(...).unwrap_or(0)`; `font-size` is filtered `> 0` and defaults to `12 * MILLI`. | High |
| OBS-019 | `svg.rs:157-161` | `text-anchor` maps only `Some("middle")` to `TextAnchor::Middle` and `Some("end")` to `TextAnchor::End`; everything else is `TextAnchor::Start`. | High |
| OBS-020 | `svg.rs:162-164` | `font-weight` sets `bold` for `w == "bold" \|\| w == "600" \|\| w == "700" \|\| w == "800" \|\| w == "900"`, otherwise `false`. | High |
| OBS-021 | `svg.rs:165-177` | `text` content is read to `</text>`, trimmed, and a `TextItem` is pushed only when non-empty. | High |
| OBS-022 | `svg.rs:180-185` | The `"polyline" if !tag.close` arm parses `points` with `map(parse_points).unwrap_or_default()` and `stroke` with `map(\|s\| s.to_string())`, then always pushes a `Polyline`. | High |
| OBS-023 | `svg.rs:186` | The wildcard arm `_ => {}` ignores every element name not matched by the `svg`, `title`, `text`, `polyline` arms. | High |
| OBS-024 | `svg.rs:192-208` | `next_tag` skips comments: on `src[i..].starts_with("<!--")` it finds `"-->"` and continues scanning. | High |
| OBS-025 | `svg.rs:210-213` | `next_tag` skips processing instructions and declarations: `src[i..].starts_with("<?") \|\| src[i..].starts_with("<!")` advances to the first `>`. | High |
| OBS-026 | `svg.rs:215-232` | Tag-name scanning stops at ASCII whitespace, `>` or `/`; `let close = bytes[j] == b'/';`; the resulting `name` is compared against lowercase literals in `parse`, so matching is exact/case-sensitive. | High (enumeration) |
| OBS-027 | `svg.rs:235-262` | Attribute scanning is quote-aware (`'` and `"` suspend `>` detection); a `/` immediately before `>` sets `self_closing = true` and is excluded from the attribute slice. | High |
| OBS-028 | `svg.rs:274-279` | `text_until` uses a literal `src[from..].find(close_tag)`, then `decode_entities(&strip_tags(raw))`. | High |
| OBS-029 | `svg.rs:282-294` | `strip_tags` increments `depth` on `<`, saturating-decrements on `>`, and keeps only characters seen at `depth == 0`. | High |
| OBS-030 | `svg.rs:297-335` | `decode_entities` decodes `amp`, `lt`, `gt`, `quot`, `apos`, and decimal or `x`/`X` hexadecimal numeric references; an undecoded `&` is pushed as ordinary text on the fall-through path. | High |
| OBS-031 | `svg.rs:338-392` | `attributes` accepts quoted values with either quote character or unquoted values; a name with no `=` is kept with an empty value; empty names are skipped. | High |
| OBS-032 | `svg.rs:394-400` | `attr` returns the first `(k, _)` whose key equals `name` exactly. | High |
| OBS-033 | `svg.rs:402-421` | `parse_points` splits on `,` or ASCII whitespace; any part that fails `parse_milli` makes the whole result `Vec::new()`; `chunks_exact(2)` discards an odd trailing number. | High |
| OBS-034 | `svg.rs:423-469` | `parse_milli` trims, accepts an optional `-` or `+`, accumulates an integer and at most three fractional digits (further digits are consumed but not accumulated), returns `None` if any input character remains, and uses `saturating_mul`/`saturating_add`. | High |
| OBS-035 | `svg.rs:471-483` | `parse_view_box` splits on `,`/whitespace, drops parts that `parse_milli` rejects via `filter_map`, and returns `Some` only when exactly 4 numbers survive. | High |
| OBS-036 | `svg.rs:485-499` | `escape_xml` escapes `&`, `<`, `>`, `"`, `'` to the five predefined XML entities. | High |
| OBS-037 | `svg.rs:501-509` | `map_range` maps with `i128` arithmetic only and returns `(out_lo + out_hi) / 2` when `hi == lo`. | High |
| OBS-038 | `svg.rs:511-513` | `make_plot_svg` doc: `Produces the plot document that this workspace's engine carries for a plot call`; `This is the generator the fake engine uses, and it is the writer that the reader in parse is tested against.` | High |
| OBS-039 | `svg.rs:514-534` | The generator fixes `width = 640`, `height = 400`, `margin = 48`, spaces points evenly from `margin` to `width - margin`, and maps ys into `height - margin .. margin`. | High |
| OBS-040 | `svg.rs:536-541` | The generated document begins with an XML declaration and `<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">`, then `<title>`. | High |
| OBS-041 | `svg.rs:542-554` | The generator emits one `<rect x="0" y="0" width="{width}" height="{height}" fill="#ffffff" stroke="#c8c8c8"/>` and two `<line ... stroke="#d0d0d0"/>` axis elements. | High |
| OBS-042 | `svg.rs:555-559` | The generator emits `<polyline fill="none" stroke="#1f4e79" stroke-width="2" points="{points}"/>` only when `!points.is_empty()`. | High |
| OBS-043 | `svg.rs:560-574` | The generator emits three `<text>` elements: the title (`font-size="16" font-weight="bold" text-anchor="middle"`), `max {hi}` and `min {lo}` (`font-size="11"`). | High |
| OBS-044 | `svg.rs:583-593` | `parse_milli` tests assert `"12" -> 12000`, `"12.5" -> 12500`, `"-3.125" -> -3125`, `"0.1" -> 100`, `".5" -> 500`, and `None` for `"1e3"`, `"px"`, `""`. | High |
| OBS-045 | `svg.rs:595-609` | The round-trip test asserts title, `640 * MILLI` width, `400 * MILLI` height, `Some([0, 0, 640 * MILLI, 400 * MILLI])` view_box, one polyline with `ys.len()` points, stroke `#1f4e79`, and texts containing the title and `max 9`. | High |
| OBS-046 | `svg.rs:611-634` | The handwritten-tag test asserts `Café & co` title decoding, point `10_500` from `10.5`, `TextAnchor::Middle`, `14_000` font size, bold, and text `A < B`. | High |
| OBS-047 | `svg.rs:652-656` | The test `parse_ignores_nested_tspan_markup` asserts the text of `a<tspan>b</tspan>c` is `abc`. | High |
| OBS-048 | `lib.rs:1` | Crate doc: `The Lovelace scripting language: values, the tree-walking interpreter and the engine facade.` | High |
| OBS-049 | `lib.rs:3-4` | Crate doc: `every value renders with its documented display form and reports a kind, and the engine reports errors as a message plus a UTF-16 position into the source.` | High |
| OBS-050 | `lib.rs:13-27` | Public modules: `builtins`, `complex`, `complex_math`, `context`, `fixed_dsp`, `help`, `lexer`, `mathir`, `modus`, `numeric`, `registry`, `structured`, `syntax`, `value_widen`. | High |
| OBS-051 | `lib.rs:24` | `pub mod structured;` | High |
| OBS-052 | `lib.rs:28-32` | `pub mod cancel` re-exports `lovelace_symbolics::cancel::*`. | High |
| OBS-053 | `lib.rs:45-71` | `ErrorSignature` has `function: String`, `parameters: Option<Vec<String>>`, `expected_min: Option<usize>`, `expected_max: Option<usize>`, `variadic: Option<bool>`, `actual: Option<usize>`, `position: Option<usize>`, `parameter: Option<String>`, `expected_shape: Option<String>`, `arrived_kind: Option<String>`. | High |
| OBS-054 | `lib.rs:73-92` | `LangError` has `message: String`, `position: usize`, `code: &'static str`, `category: &'static str`, `signature: Option<ErrorSignature>`, `suggestions: Vec<String>`, `hints: Vec<String>`, `registry_size: Option<usize>`; `position` is documented as `UTF-16 code-unit offset into the source.` | High |
| OBS-055 | `lib.rs:94-155` | `LangError::at` uses code `"InvalidOperation"` and category `"DomainError"`; `plain` calls `at(..., 0)`; `coded` takes code and category; `depth` uses `"DepthExceeded"`/`"BudgetExceeded"`; `with_position` and `with_signature` are builders. | High |
| OBS-056 | `lib.rs:166-169` | `CANCELLED_MESSAGE` is `"the evaluation was cancelled by the caller."` | High |
| OBS-057 | `lib.rs:171-176` | `cancelled_error()` returns `LangError::coded(CANCELLED_MESSAGE, 0, "Cancelled", "BudgetExceeded")`. | High |
| OBS-058 | `lib.rs:178-215` | `Value` is a public enum with variants `Natural(Nat)`, `Integer(Int)`, `Real(Real)`, `RealClamped(Real, &'static str, u64)`, `Boolean(bool)`, `Text(String)`, `Vector(Vec<Value>)`, `Array(NdArray<Value>)`, `Function(String)`, `Void`, `Record(String, Vec<(String, Value)>)`, `Complex(Real, Real)`, `Symbolic(Sym)`, `Domain(String)`, `Enum(String, String)`. | High |
| OBS-059 | `lib.rs:217-237` | `Value::kind_name` returns one of `Natural`, `Integer`, `Real`, `Boolean`, `Text`, `Vector`, `Array`, `Function`, `Void`, `Complex`, `Symbolic`, `Domain`, `Enum`, `Record`; `RealClamped` reports `Real`. | High |
| OBS-060 | `lib.rs:239-299` | `Value::display` renders each variant; `Void` renders as the empty string; `Text` as its text; `Record` as `name(field: value, ...)`; `Vector` and `Array` use `MAX_DISPLAY_NODES` and can emit `...`. | High |
| OBS-061 | `lib.rs:325-332` | `MAX_DISPLAY_NODES` is `100_000` and bounds display nodes only; the doc states the array itself remains constructible and `shape`/`numel` still answer it. | High |
| OBS-062 | `lib.rs:301-314` | `Value::typed` renders `Void` as `(void)`, `Text` as display, `Enum` as the bare member name, `Record` as `display (TypeName)`, everything else as `display (kind_name)`. | High |
| OBS-063 | `lib.rs:316-322` | `Value::is_numeric` is true only for `Natural`, `Integer`, `Real`, `RealClamped`. | High |
| OBS-064 | `lib.rs:234` | Comment: `A record's kind is Record; its own type name is published in the structured form.` | High |
| OBS-065 | `lib.rs:303` | Doc: `A record publishes its OWN type name (SolveResult, Inspection, …)`; `an enum publishes its bare member name`. | High |
| OBS-066 | `lib.rs:1283` | The member-access error text says: `Symbolic expressions have no members; call the *_full builtin (e.g. solve_full) for structured results.` | High |
| OBS-067 | `lib.rs:467-476` | `OpTiming` has `position: usize`, `result_kind: String`, `has_output: bool`; there is no duration field. | High |
| OBS-068 | `lib.rs:478-487` | `PlotCapture` has `path: String`, `title: String`, `svg: String`, documented as the captured plot of the last plot call. | High |
| OBS-069 | `lib.rs:494-544` | `Engine` is a public struct; its documented purpose is `variables, functions, precision and captured output`; the listed fields include private `output: Vec<String>`, `plot: Option<PlotCapture>`, `plot_dir`, `plot_file`, `revision`, `timings: Vec<OpTiming>`. | High |
| OBS-070 | `lib.rs:552-593` | `Engine::new()` builds the all-builtins engine with a temp `lovelace-plots` directory and `plot_file: "plot.svg"`; `Engine::new_bare()` sets `enabled_plugins` to `Some(HashSet::new())`, hiding plugin-advertised builtins until loaded. | High |
| OBS-071 | `lib.rs:595-670` | Public registration methods: `load_plugin`, `register_builtin`, `register_builtin_descriptor`, `register_array_builtin`, `register_kernel`, `try_dispatch_kernel`; their parameter types include `modus::` types not read in this census. | High |
| OBS-072 | `lib.rs:672-702` | `plugin_name_of` returns `Option<&str>`; `registered_descriptor` returns `Option<&modus::BuiltinDescriptor>`; `has_function` returns `bool` and checks `funcs`, `modus`, and `registry::find`. | High |
| OBS-073 | `lib.rs:704-707` | `register_assumption_context` takes `std::sync::Arc<context::Context>` and delegates to `self.modus.register_assumption_context(context)`. | High |
| OBS-074 | `lib.rs:709-718` | `set_precision(decimals)` stores `decimals.max(1)` and sets `precision_explicit = true`; `precision()` returns the stored value. | High |
| OBS-075 | `lib.rs:720-729` | `display_precision()` returns `self.precision` when `precision_explicit`, otherwise `100`. | High |
| OBS-076 | `lib.rs:731-742` | `set_plot_dir` stores the string and calls `create_dir_all`; `set_cancel_after` stores `Option<Duration>`. | High |
| OBS-077 | `lib.rs:744-762` | `cancel_after()` returns `Option<Duration>`; `was_cancelled()` returns `bool`; `last_completed_position()` returns `usize`; `revision()` returns `u64`. | High |
| OBS-078 | `lib.rs:764-772` | `last_plot()` returns `self.plot.clone()`, i.e. `Option<PlotCapture>`; `take_output()` returns `std::mem::take(&mut self.output)`, i.e. `Vec<String>`. | High |
| OBS-079 | `lib.rs:774-784` | `format(&Value)` returns `String` via `value.display()` under `with_display_decimal_places(self.display_precision(), ...)`; `format_typed` does the same with `value.typed()`. | High |
| OBS-080 | `lib.rs:786-796` | `variables()` returns `Vec<(String, String)>` of name and `v.typed()`, excluding `"true"` and `"false"`, sorted by name. | High |
| OBS-081 | `lib.rs:798-808` | `variable_values()` returns `Vec<(String, Value)>` cloned, excluding `"true"` and `"false"`, sorted by name. | High |
| OBS-082 | `lib.rs:810-813` | `operation_timings()` returns `&[OpTiming]`, documented as one entry per top-level statement that ran, in statement order. | High |
| OBS-083 | `lib.rs:815-824` | `functions()` returns `Vec<(String, Vec<String>)>` of name and parameter names, sorted by name; function bodies are not returned. | High |
| OBS-084 | `lib.rs:826-867` | `eval(&mut self, source: &str) -> Result<Value, LangError>` arms a deadline for exactly that evaluation, installs a plugin assumption scope when present, runs `eval_inner`, restores assumptions, and replaces a result with a `Cancelled` error when `cancel::observed()`. | High |
| OBS-085 | `lib.rs:895-898` | `eval_text` resets `last_completed_position = 0`, `self.plot = None`, and `self.timings.clear()` at the start of each evaluation. | High |
| OBS-086 | `lib.rs:914-963` | The statement loop pushes an `OpTiming` for each statement returning `Flow::Value` or `Flow::Return` and, on an `Err`, pushes one with `result_kind: "Void"` for the failing statement before returning the error. | High |
| OBS-087 | `lib.rs:999-1002` | After a successful evaluation, `self.revision += 1`; if the final value is not `Value::Void`, it is stored under the variable name `"_"`. | High |
| OBS-088 | `lib.rs:1424-1432` | `plot_file` has a crate-private getter; `set_plot_file(name)` stores the name. | High |
| OBS-089 | `lib.rs:1438-1441` | `Engine::assumptions()` is `pub(crate)`, not public. | High |
| OBS-090 | `lib.rs:1474-1603` | `assumption_atoms_on`, `domain_assumptions_on`, `is_positive`, `is_nonzero` are `pub(crate)`; `record_assumption*`, `add_bound`, `clear_assumptions` are also `pub(crate)`. | High |
| OBS-091 | `lib.rs:1868-1873` | `pub fn parse_expression(source: &str) -> Result<Expr, LangError>` tokenizes and returns `Parser::single_expression()`. | High |
| OBS-092 | `lib.rs:1875-1878` | `pub fn token_count(source: &str) -> Result<usize, LangError>` returns `syntax::tokenize(source)?.len()`. | High |
| OBS-093 | `lib.rs:1880-1883` | Public re-exports: `pub use syntax::BinOp as Operator;` and `pub use syntax::Tok as Token;`. | High |
| OBS-094 | `structured.rs:1-13` | Module doc: `The structured projection of runtime values onto the agent-facing wire.`; `One projection, total over the value kinds`; `The public interface is consumed by lovelace-run`; `It is defined here (and not in the runner) so the desktop host can publish the same structure.` | High |
| OBS-095 | `structured.rs:15-16` | Imports `lovelace_json::Json`; no local `Json` type is defined. | High |
| OBS-096 | `structured.rs:24-26` | `project_value(_engine: &Engine, value: &Value, budget: Option<usize>) -> Json` always starts the object with `("kind", Json::Str(value.kind_name().to_string()))`. | High |
| OBS-097 | `structured.rs:28-35` | Natural and Integer project `("value", Json::Str(...))` and `("exact", Json::Bool(true))`. | High |
| OBS-098 | `structured.rs:36-54` | Real/RealClamped project `value`, `exact`, `numerator`/`denominator` when exact, and for `RealClamped` also `truncated`, `truncationReason`, `budget`. | High |
| OBS-099 | `structured.rs:55-63` | Boolean projects `value` as `"true"`/`"false"`; Text projects `value` as the text. | High |
| OBS-100 | `structured.rs:64-77` | Vector projects `kind: "Array"`, `type: "Vector"`, `shape: [len]`, and recursive `elements`. | High |
| OBS-101 | `structured.rs:78-92` | Array projects `type: "Array"`, `shape` from `a.shape()`, and recursive `elements` from `a.iter()`. | High |
| OBS-102 | `structured.rs:93-99` | Function projects `value` as the function name; Void rewrites `kind` to `"Null"`. | High |
| OBS-103 | `structured.rs:100-116` | Record projects `type` as the record type name and `fields` as objects `{name, value}`. | High |
| OBS-104 | `structured.rs:117-121` | Complex projects `re`, `im`, and `exact` (`re.is_exact() && im.is_exact()`). | High |
| OBS-105 | `structured.rs:122-144` | Symbolic projects `pretty`, `canonical`, `domain`, `exact`, `nodeCount`, `freeSymbols`, plus `truncated`/`truncationReason`/optional `budget` when shortened. | High |
| OBS-106 | `structured.rs:145-151` | Domain projects `domain` lowercased; Enum projects `type` and `value` (variant). | High |
| OBS-107 | `structured.rs:153` | `let _ = budget;` — after the Symbolic branch the `budget` parameter is explicitly unused for every other kind. | High |
| OBS-108 | `structured.rs:157-188` | `truncate` returns the full string when the budget is `None` or `node_count <= max_nodes`; otherwise, when the text is longer than `max(6 * budget, 48)` characters it cuts to that character allowance, and otherwise to a proportional character count, at a token boundary, appending ` …`; a zero cut yields `…`. | High |
| OBS-109 | `structured.rs:217-249` | `descriptor_json` emits `name`, `parameters`, `minArity`, `parameterCount`, `variadic`, `builtin: true`, `category`, plus optional `plugin`, `summary`, `returnKind`, `related`, `examples`. | High |
| OBS-110 | `structured.rs:251-267` | `listing_json` emits `name`, `parameters`, `minArity`, `parameterCount`, `variadic`, `category`. | High |
| OBS-111 | `structured.rs:269-272` | `project_function` returns `crate::registry::find(name).map(descriptor_json)`, i.e. `Option<Json>`. | High |
| OBS-112 | `structured.rs:274-293` | `project_functions` returns an object with `pattern`, `functions`, `total`, `returned`, `truncated`; `limit` defaults to `50`; `registry::matching(pattern)` supplies matches. | High |
| OBS-113 | `registry.rs:7-31` | `BuiltinInfo` has `name`, `parameters`, `min_arity`, `parameter_count`, `variadic`, `plugin`, `category`, `summary`, `return_kind`, `related`, `examples`; all fields are `pub`. | High |
| OBS-114 | `syntax.rs:9-28` | `Tok` variants: `Num(String)`, `Repr(String)`, `Str(String)`, `Interp(String)`, `Ident(String)`, `Punct(String)`, `Semi`, `Eof`. | High |
| OBS-115 | `syntax.rs:30-37` | `Token` is `pub struct Token { pub tok: Tok, pub pos: usize }`, position documented as `UTF-16 code units`. | High |
| OBS-116 | `syntax.rs:56-83` | `BinOp` variants: `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Pow`, `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`. | High |
| OBS-117 | `syntax.rs:96-129` | `Expr` variants: `Number(String)`, `Repeating(String)`, `Str(String)`, `Interp(Vec<InterpPart>)`, `Var(String, usize)`, `Call(String, Vec<Expr>, usize)`, `Binary(BinOp, Box<Expr>, Box<Expr>)`, `Neg(Box<Expr>)`, `Not(Box<Expr>)`, `Fact(Box<Expr>)`, `List(Vec<Expr>)`, `Range(Box<Expr>, Option<Box<Expr>>, Box<Expr>)`, `Index(Box<Expr>, Vec<IndexArg>)`, `Assign(String, Box<Expr>)`, `Member(Box<Expr>, String)`. | High |

## 2. SVG support table

`svg.rs` contains no painting function. "Rendered how?" below therefore records only the `Plot` field the parser fills; the module comment assigns coordinate scaling to `the GDI layer` (`svg.rs:9-11`), whose drawing code is outside SCOPE (see section 4).

| SVG feature (element/attribute/transform) | Parsed? | Rendered how? | Citation |
|---|---|---|---|
| `<svg>` start tag | Yes; applies size/viewBox when its attributes allow, otherwise the default remains | Fills `Plot.width`, `Plot.height`, `Plot.view_box`; no painter in this file | `svg.rs:117-138` |
| `<svg/>` self-closing | Yes; the arm checks only `!tag.close` | Same as `<svg>` | `svg.rs:117` |
| `</svg>` close tag | No; all element arms require `!tag.close` | Nothing stored | `svg.rs:117,140,149,180` |
| `svg@width` | Conditionally; must parse to a value `> 0`, and is committed when height also parses or a valid viewBox exists | `Plot.width` in thousandths of a unit | `svg.rs:119-137` |
| `svg@height` | Conditionally; same rules as width | `Plot.height` in thousandths of a unit | `svg.rs:122-137` |
| `svg@viewBox` | Yes when exactly four numbers survive `parse_milli`; stored even if width/height are present | `Plot.view_box: Option<[i64;4]>`; no coordinate transform is applied in `svg.rs` | `svg.rs:125-135,471-483` |
| `svg@xmlns` | No; no lookup | Dropped | `svg.rs:119-125` |
| `<title>` | Yes for open, non-self-closing tags; text trimmed, empty ignored | `Plot.title: Option<String>` | `svg.rs:140-147` |
| `<title/>` self-closing | No; guarded by `!tag.self_closing` | Dropped | `svg.rs:140` |
| `<text>` | Yes for open, non-self-closing tags; body to `</text>`, trimmed, empty ignored | `Plot.texts` entry | `svg.rs:149-177` |
| `text@x`, `text@y` | Yes via `parse_milli`, default `0` | `TextItem.x`, `TextItem.y` (baseline origin, milli) | `svg.rs:151-152` |
| `text@font-size` | Yes; value must be `> 0`, else `12 * MILLI` | `TextItem.size` (milli) | `svg.rs:153-156` |
| `text@text-anchor` | Yes; only exact `middle` and `end`, otherwise `Start` | `TextItem.anchor` | `svg.rs:157-161` |
| `text@font-weight` | Yes; bold for `bold`, `600`, `700`, `800`, `900` | `TextItem.bold` | `svg.rs:162-164` |
| `<polyline>` | Yes for open and self-closing tags (only close is excluded) | `Plot.polylines` entry | `svg.rs:180-185` |
| `polyline@points` | Yes; comma/whitespace separated; any bad part empties the list; odd trailing number dropped | `Polyline.points: Vec<Point>` (milli) | `svg.rs:182,402-421` |
| `polyline@stroke` | Yes; raw string; absent gives `None`, empty gives `Some("")` | `Polyline.stroke: Option<String>` | `svg.rs:183` |
| `polyline@fill` | No; not looked up (generator writes `fill="none"`) | Dropped | `svg.rs:180-185,555-559` |
| `polyline@stroke-width` | No; not looked up (generator writes `stroke-width="2"`) | Dropped; no width is stored | `svg.rs:183,555-559` |
| `transform` attribute (any element) | No; `transform` is not among the attribute names read by `parse` | Dropped; `viewBox` is stored but no transform is applied | `svg.rs:111-190` (all `attr` call sites) |
| `<rect>` | No match arm; wildcard ignores it (generator emits one background rect) | No `Plot` representation | `svg.rs:186,542-544` |
| `<line>` | No match arm; wildcard ignores it (generator emits two axis lines) | No `Plot` representation | `svg.rs:186,545-554` |
| `<path>`, `<circle>`, `<g>`, `<defs>`, `<style>`, other elements | No match arm | Dropped | `svg.rs:186` |
| `<tspan>` inside `<text>`/`<title>` | Not an element arm, but `strip_tags` removes the markup and keeps depth-0 text | Inner text concatenated into `TextItem.text` / `Plot.title` | `svg.rs:165,274-294` |
| XML comments `<!-- ... -->` | Yes; skipped by `next_tag` | Not content | `svg.rs:205-208` |
| Processing instructions `<?...?>` and declarations `<!...>` | Yes; skipped to the first `>` | Not content | `svg.rs:210-213` |
| Any closing tag | No; no arm matches when `tag.close` is true | Nothing stored | `svg.rs:117,140,149,180` |
| `&amp; &lt; &gt; &quot; &apos;` | Yes; decoded in title/text bodies | Characters in stored `String` | `svg.rs:296-335` |
| Numeric references `&#233;` / `&#xE9;` | Yes; decimal and `x`/`X` hexadecimal | Characters in stored `String` | `svg.rs:312-322` |
| Unrecognised entity text | No; the `&` is emitted as ordinary text and scanning continues | Kept literally | `svg.rs:324-333` |
| Decimal coordinate syntax (`12`, `12.5`, `.5`, signs) | Yes; optional sign, optional fraction, max three fraction digits; exponents/units rejected; excess fraction digits truncated, not rounded | `i64` thousandths | `svg.rs:425-469` |
| `viewBox` separators and arity | Comma and/or whitespace; unparsable parts filtered; exactly 4 numbers required | `[i64; 4]` in thousandths | `svg.rs:471-483` |
| Multiple `<polyline>` / `<text>` elements | Yes; each push appends | `Plot.polylines` / `Plot.texts` in document order | `svg.rs:73-76,165-177,180-185` |
| Truncated or malformed document | Parse returns whatever was accumulated; no error type | Partial `Plot` | `svg.rs:108-111,189` |
| `style`, `class`, CSS, `font-family`, `dominant-baseline`, `opacity` | No; none is looked up | Dropped | `svg.rs:111-190` |

## 3. Engine surface table

Legend: **Structured** = typed Rust data that a caller can match or read. **String-only** = a preformatted `String` or list of strings. **JSON** = a `lovelace_json::Json` built by `structured.rs`. **Scalar** = a plain number/bool/`Duration`. **Not exposed** = no public accessor found in `lib.rs`.

| Type / method | What it exposes | Structured or string-only? | Citation |
|---|---|---|---|
| `Value` (enum) | 15 variants: `Natural(Nat)`, `Integer(Int)`, `Real(Real)`, `RealClamped(Real, &'static str, u64)`, `Boolean(bool)`, `Text(String)`, `Vector(Vec<Value>)`, `Array(NdArray<Value>)`, `Function(String)`, `Void`, `Record(String, Vec<(String, Value)>)`, `Complex(Real, Real)`, `Symbolic(Sym)`, `Domain(String)`, `Enum(String, String)` | Structured | `lib.rs:179-215` |
| `Value::kind_name` | Kind label; `RealClamped` reports `Real`, `Record` reports `Record` | String-only | `lib.rs:219-237` |
| `Value::display` | Display form; `Void` empty; `Vector`/`Array` abbreviated with `...` after `MAX_DISPLAY_NODES` | String-only | `lib.rs:240-299,325-332` |
| `Value::typed` | Display plus kind; `Record` appends its own record type name, `Enum` is bare, `Text` has no suffix, `Void` is `(void)` | String-only | `lib.rs:301-314` |
| `Value::is_numeric` | True only for `Natural`, `Integer`, `Real`, `RealClamped` | Boolean predicate | `lib.rs:316-322` |
| `Value::Record(String, Vec<(String, Value)>)` | Record type name and ordered named fields; docs name `SolveResult` and `Inspection` as examples of record type names | Structured | `lib.rs:206,234,303` |
| `Value::Symbolic(Sym)` | Carries `Sym`, the private alias of `lovelace_symbolics::Expr`; `lib.rs` does not re-export `Sym` | Structured payload; `Sym` API not read | `lib.rs:41,210` |
| `Value::RealClamped(Real, &'static str, u64)` | Value, truncation reason (`"digit-cap"`), and the budget that stopped it | Structured | `lib.rs:187-192` |
| `ErrorSignature` | The call-site signature fields listed in OBS-053 | Structured | `lib.rs:49-71` |
| `LangError` | Message, UTF-16 position, stable code, category, optional signature, suggestions, hints, registry size | Structured | `lib.rs:74-92` |
| `LangError::at` / `plain` / `coded` / `depth` | Constructors; defaults `InvalidOperation`/`DomainError`; depth uses `DepthExceeded`/`BudgetExceeded` | Structured error builders | `lib.rs:96-143` |
| `LangError::with_position` / `with_signature` | Builder methods returning `LangError` | Structured | `lib.rs:146-155` |
| `CANCELLED_MESSAGE` / `cancelled_error()` | Fixed cancellation sentence; a `LangError` with code `Cancelled`, category `BudgetExceeded` | Structured error (message is a string) | `lib.rs:169-176` |
| `OpTiming` | Statement `position`, `result_kind`, `has_output`; no duration | Structured metadata | `lib.rs:467-476` |
| `PlotCapture` | `path`, `title`, `svg` — the plot is exposed only as the raw SVG document string | Structured wrapper; payload is a string | `lib.rs:478-487` |
| `Engine::new` / `Engine::new_bare` | Constructors; `new_bare` exposes only core builtins until a plugin is loaded | Not a result | `lib.rs:552-593` |
| `Engine::eval` | `Result<Value, LangError>` for a whole script; final value; cancellation mapped to `Cancelled` error | Structured result or structured error | `lib.rs:826-867` |
| `Engine::last_plot` | `Option<PlotCapture>` cloned from the captured plot | Structured wrapper; SVG payload is a string | `lib.rs:764-767` |
| `Engine::take_output` | `Vec<String>` of captured print lines, drained | String-only | `lib.rs:769-772` |
| `Engine::format` | `String` display form at the engine's display precision | String-only | `lib.rs:774-779` |
| `Engine::format_typed` | `String` display form with kind at display precision | String-only | `lib.rs:781-784` |
| `Engine::variables` | `Vec<(String, String)>` of name and typed rendering, sorted, excluding `true`/`false` | String-only | `lib.rs:786-796` |
| `Engine::variable_values` | `Vec<(String, Value)>` of name and value, sorted, excluding `true`/`false` | Structured | `lib.rs:798-808` |
| `Engine::operation_timings` | `&[OpTiming]`, one per top-level statement that ran | Structured | `lib.rs:810-813` |
| `Engine::functions` | `Vec<(String, Vec<String>)>` of user-function name and parameter names; no public method in `lib.rs` returns the body | Structured metadata (strings) | `lib.rs:815-824` |
| `Engine::revision` / `last_completed_position` / `was_cancelled` / `cancel_after` | `u64`, `usize`, `bool`, `Option<Duration>` | Scalar | `lib.rs:744-762` |
| `Engine::precision` / `display_precision` | `usize`; display precision is `100` until `set_precision` is called | Scalar | `lib.rs:709-729` |
| `Engine::set_plot_dir` / `set_plot_file` | Configure the plot output directory and file name | Configuration | `lib.rs:731-735,1424-1432` |
| `Engine::load_plugin` / `register_builtin` / `register_builtin_descriptor` / `register_array_builtin` / `register_kernel` / `try_dispatch_kernel` | Registration surface; signatures use `modus::` types (definitions not read) | Registration API | `lib.rs:595-670` |
| `Engine::plugin_name_of` / `registered_descriptor` / `has_function` | `Option<&str>`, `Option<&modus::BuiltinDescriptor>`, `bool`; descriptor fields not read | Descriptor reference / scalar | `lib.rs:672-702` |
| `Engine::register_assumption_context` | Takes `Arc<context::Context>`; `context::Context` definition not read | Configuration | `lib.rs:704-707` |
| `structured::project_value` | Always `kind`; per-kind fields listed below; `budget` only truncates Symbolic | JSON | `structured.rs:24-155` |
| `project_value` Natural/Integer | `value` string, `exact: true` | JSON | `structured.rs:28-35` |
| `project_value` Real/RealClamped | `value`, `exact`, exact rational `numerator`/`denominator`; clamped adds `truncated`, `truncationReason`, `budget` | JSON | `structured.rs:36-54` |
| `project_value` Boolean/Text | `value` | JSON | `structured.rs:55-63` |
| `project_value` Vector/Array | `kind: "Array"`, `type`, `shape`, recursive `elements` (`Vector` also sets `type: "Vector"`) | JSON | `structured.rs:64-92` |
| `project_value` Function/Void | Function `value`; Void `kind: "Null"` | JSON | `structured.rs:93-99` |
| `project_value` Record | `type` plus `fields: [{name, value}]` | JSON | `structured.rs:100-116` |
| `project_value` Complex | `re`, `im`, `exact` | JSON | `structured.rs:117-121` |
| `project_value` Symbolic | `pretty`, `canonical`, `domain`, `exact`, `nodeCount`, `freeSymbols`, plus truncation flags; no child-expression tree | JSON (strings/counts only) | `structured.rs:122-144` |
| `project_value` Domain/Enum | Domain `domain` lowercased; Enum `type` and `value` | JSON | `structured.rs:145-151` |
| `structured::descriptor_json` | `name`, `parameters`, `minArity`, `parameterCount`, `variadic`, `builtin`, `category`, plus optional `plugin`, `summary`, `returnKind`, `related`, `examples` | JSON | `structured.rs:217-249` |
| `structured::listing_json` | `name`, `parameters`, `minArity`, `parameterCount`, `variadic`, `category` | JSON | `structured.rs:251-267` |
| `structured::project_function` | `Option<Json>` from `registry::find(name)` | JSON | `structured.rs:269-272` |
| `structured::project_functions` | `pattern`, `functions`, `total`, `returned`, `truncated`; default limit 50 | JSON | `structured.rs:274-293` |
| `Json` (return type) | `lovelace_json::Json`; the type's definition and serialization are not read | JSON (external type) | `structured.rs:16` |
| `registry::BuiltinInfo` | 11 public fields (name, parameters, min/max arity, variadic, plugin, category, summary, return kind, related, examples) | Structured descriptor | `registry.rs:7-31` |
| `parse_expression` | `Result<syntax::Expr, LangError>` for one expression | Structured AST | `lib.rs:1868-1873` |
| `syntax::Expr` | 15 source-AST variants including `Var(String, usize)` and `Call(String, Vec<Expr>, usize)` positions; `Interp(Vec<InterpPart>)` | Structured AST | `syntax.rs:96-129` |
| `token_count` | `Result<usize, LangError>` | Scalar | `lib.rs:1875-1878` |
| `Operator` = `syntax::BinOp` | 12 binary-operator variants | Enum | `lib.rs:1881`, `syntax.rs:56-83` |
| `Token` = `syntax::Tok` | 8 token variants; `syntax::Token` adds `pos: usize` | Enum/struct | `lib.rs:1883`, `syntax.rs:9-37` |
| `Engine` session assumptions/bounds | No public accessor: `assumptions()`, `assumption_atoms_on`, `domain_assumptions_on`, `is_positive`, `is_nonzero`, `add_bound`, `clear_assumptions` are `pub(crate)` | Not exposed | `lib.rs:1438-1441,1474-1603` |
| `Engine` user-function bodies | `funcs` is private; `functions()` returns only parameter names | Not exposed | `lib.rs:497,815-824` |

Type count: 12 distinct named types — `Value`, `ErrorSignature`, `LangError`, `OpTiming`, `PlotCapture`, `Engine` (6 core), `Json`, `registry::BuiltinInfo` (2 projection/descriptor), `syntax::Expr`, `syntax::BinOp`, `syntax::Tok`, `syntax::Token` (4 syntax). The public `Flow` enum (`lib.rs:1611-1621`) is returned only by the private `Engine::exec`, so it is not a front-end result type and is not counted. `Sym`/`Nat`/`Int`/`Real`/`NdArray`/`modus::*`/`context::Context` are named in signatures but their definitions were not read, so they are not counted as surveyed types.

## 4. What I could NOT determine within SCOPE, and why

- **How a parsed `Plot` is painted.** `svg.rs` has no drawing code. Its comment says the GDI layer scales the integer coordinates (`svg.rs:9-11`), but that layer is not in `svg.rs`, and `win32/` and `app.rs` are explicitly OUT OF SCOPE. Colour use, stroke widths, text metrics, clipping and `viewBox` mapping are therefore undetermined.
- **Whether `app.rs` calls `parse` at all, and what string it passes.** The desktop wiring is outside SCOPE; only the module itself was read.
- **What SVG the real Rust engine emits, and how `PlotCapture` is produced.** `make_plot_svg` is documented as `the generator the fake engine uses` (`svg.rs:511-513`); the real plot builtin, its SVG writer, and the call sites of the crate-private `capture_plot` (`lib.rs:1416-1418`) are not in the read set. Whether real engine plots contain elements this parser ignores (for example a `rect`/`line` background and axes) is therefore not determined.
- **The field schemas of record results such as `SolveResult` and `Inspection`.** `lib.rs` names them only as examples (`lib.rs:303`) and points to `*_full` builtins (`lib.rs:1283`); no construction site is in the read set: `lib.rs:1390` dispatches to `builtins::call`, and `builtins.rs` was not read. `Value::Record` and `structured::project_value` are generic, so record values can be projected without knowing the names, but the field list itself is not enumerable from the read set.
- **Node-level inspection of a symbolic value.** `Value::Symbolic` holds `Sym = lovelace_symbolics::Expr` (`lib.rs:41,210`), but `lib.rs` does not re-export that type; the only re-export from `lovelace_symbolics` is its `cancel` module (`lib.rs:28-32`). `lovelace-symbolics` is another crate and outside the permitted re-export scope, so `Expr`'s fields/methods/child accessors were not read. `structured::project_value` exposes only pretty/canonical text, domain, exactness, node count and free symbols (`structured.rs:122-144`); whether a child-expression tree can be built from the public API is undetermined.
- **The `Json` type and its serialization.** `structured.rs` imports `lovelace_json::Json` (`structured.rs:16`) and builds it with `Json::Str`, `Json::Bool`, `Json::Num`, `Json::Arr`, `Json::obj`, `Json::num_array`, `Json::str_array`; `lovelace-json` is another crate not re-exported by `lib.rs`, so the enum's variants, ordering and wire serialization were not read.
- **Registry lookup semantics.** `project_function` calls `registry::find(name)` and `project_functions` calls `registry::matching(pattern)` (`structured.rs:271,277`); only `BuiltinInfo`'s field declarations were read, not those functions' bodies, so exact-vs-pattern matching and case sensitivity are undetermined.
- **`modus` descriptor/plugin payload types.** Methods at `lib.rs:595-707` name `modus::PluginArgs`, `Payload`, `PayloadArray`, `ArrayOp`, `KernelScalar`, `FieldKernel`, `ModusPlugin`, `BuiltinDescriptor`, and `context::Context`; the `modus` and `context` modules were not read. The registration surface is therefore known only by its public signatures.
- **Direct numeric access to `Nat`, `Int`, `Real`, `NdArray`.** These payload types appear in `Value` variants (`lib.rs:182-200`) but are not re-exported by `lib.rs`; their accessor APIs were not read. Structured output reaches them only through `structured.rs` calls such as `to_decimal`, `to_string`, `is_exact`, `to_rational`, `shape`, and `iter`, so non-string numeric access is undetermined.
