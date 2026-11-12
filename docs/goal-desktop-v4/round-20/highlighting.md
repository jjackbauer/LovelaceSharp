# EDT-002: Lovelace grammar highlighting, proven by token class

Requirement: *Numbers, exact rationals, strings/interpolation, keywords, operators, function
definitions, builtins, delimiters, brackets and symbolic tokens highlight contextually.*

This round replaces the old word-plus-regex highlighter with a grammar classifier that follows
the engine parser's tokeniser, sources keywords from the one shared table, sources builtins from
the engine's own registry snapshot, and discovers symbolic names from the document's
`symbol("name")` calls. The classifier is unit-tested by exact byte range and class, and the
shipped window is captured with a five-line craft script that exercises every class.

## What the real grammar is

The authority is `lovelace_lang::syntax::tokenize` (the parser tokeniser used by
`crate::language`'s engine-drift test), cross-checked against `lovelace_lang::lexer`:

- **No comment syntax.** The parser tokeniser has no comment branch and rejects `#` with
  `Unexpected character '#' at position N`. The old highlighter painted `#` to end-of-line as a
  comment; that class was wrong and is gone. `#` is now plain text and the rest of the line is
  tokenised normally.
- **No slash-rational literal.** `1/3` is three tokens: `Num("1")`, `Punct("/")`,
  `Num("3")`. The engine folds the division to an exact rational when it evaluates, but lexically
  it is not a literal. The grammar's only exact-rational literal form is the repeating decimal:
  `0.(3)`, `0.1(6)`, `0(3)`, `12(34)` (`Tok::Repr`). The highlighter paints that form
  `Rational`; the numbers and `/` in `1/3` stay `Number` / `Operator`.
- **Numbers** are integer or decimal, including a leading-dot decimal (`.5`); `_` is not a digit
  separator in this grammar and is no longer swallowed into a number.
- **Strings** are `"..."`; **interpolated strings** are `$"..."` with `{expression}` parts and
  `{{` / `}}` escapes. The engine tokenises the whole interpolated literal as one token and
  splits the braces in the parser; the highlighter mirrors that split so the embedded expression
  gets real token classes.
- **Keywords** are exactly `language::KEYWORDS`
  (`break continue else for func if in return while`), the same table completion uses.
- **Builtins** are whatever `EngineState.functions` marks `is_builtin` after the active
  document's committed snapshot is overlaid. The old desktop list was deleted: it named
  `round`, `matrix`, `product` and `eval`, which the engine registry does not publish, and it
  missed engine builtins such as `squeeze`. The real-engine test walks every `is_builtin`
  function in `Engine::capture_state` and requires a `Builtin` span for each.
- **Function definitions** are contextual: the identifier immediately after `func` is the
  declared name. `func sqrt(x)` paints `sqrt` as `Function` even though `sqrt` is also a builtin;
  the next identifier is not a definition name.
- **Symbolic tokens** are lexical: every `symbol("name")` (or `symbol("name", domain)`) call in
  the document declares `name`, and every identifier with that spelling in an expression is
  painted `Symbol`. The string argument stays a string, and a name used only as a string is not
  changed. This is intentionally scope-insensitive (a local parameter with the same spelling is
  also symbol-coloured); the highlighter reports lexical intent, not resolved bindings.
- **Operators / delimiters** are the parser's punctuator set: operators
  `+ - * / % ^ ! = == != < <= > >= ..`; delimiters `( ) [ ] { } , ; : .`. `#`, `$` outside a
  string and any other character are plain (the parser rejects them).

## Class table

Colours are `syntax::DARK`. The capture's five lines and the per-test ranges are below.

| Class | Rule / source of truth | Highlighter action | Evidence | Colour |
| --- | --- | --- | --- | --- |
| `Number` | parser `Num`: digits, one decimal point, or leading-dot decimal | one span, integers and decimals share the class | `syntax.rs:408`; `integer_and_decimal_literals_are_one_number_class` | `#b5ce8b` green |
| `Rational` | parser `Repr`: `0.(3)`, `0.1(6)`, `0(3)`, `12(34)`; **not** `1/3` | one span for the repeating literal | `syntax.rs:408`; `repeating_decimals_are_the_grammars_only_exact_rational_literal` | `#e5c07b` gold |
| `String` | `"..."` with backslash escapes; `$"..."` literal segments | quotes and escaped text are one string run; interpolated literal segments stay string | `syntax.rs:480`; `plain_strings_include_quotes_and_escaped_quotes`, `interpolation_braces_...` | `#ce9178` salmon |
| `Interpolation` | `{` / `}` around an embedded expression; `{{` / `}}` are text | braces get their own class; the expression is classified recursively | `syntax.rs:495`; `interpolation_braces_...` | `#f0a8d0` pink |
| `Keyword` | `language::KEYWORDS`, the parser's nine statement keywords | word table lookup | `syntax.rs:372`; `every_shared_keyword_is_painted_as_a_keyword` iterates the table | `#c586c0` purple |
| `Operator` | parser punctuator set | longest match first (`==`, `!=`, `<=`, `>=`, `..`, then one-char) | `syntax.rs:563`; `operators_and_delimiters_use_the_parser_punctuator_set` | `#56b6c2` cyan |
| `Function` | identifier token immediately after `func` | contextual, not a name list | `syntax.rs:318`, `:372`; `the_declared_function_name_is_a_function_not_a_builtin` | `#4ec9b0` teal |
| `Builtin` | `EngineState.functions[].is_builtin` from the engine snapshot | exact name lookup in the per-pass set; no desktop list | `syntax.rs:155`, `editor.rs:696`; `every_real_engine_registry_builtin_is_painted_as_builtin`, `builtins_come_from_the_engine_snapshot_not_a_desktop_list` | `#4fc1ff` blue |
| `Delimiter` | `()[]{}` plus `, ; : .` | one span per punctuator | `syntax.rs:584`; operator/delimiter test | `#8b98a9` grey-blue |
| `Symbol` | `discover_symbols`: first text argument of a `symbol(...)` call, then any matching identifier | document-scoped lexical set, refreshed when the buffer changes | `syntax.rs:206`; `symbol_calls_declare_the_symbolic_tokens_used_in_expressions` | `#c8b6ff` lavender |
| `Plain` | whitespace, ordinary identifiers, unknown characters | fallback colour | `syntax.rs:318`; every `assert_exact` test requires the spans to tile the line | `#d4d9e0` |

**Comments are not in the table because the grammar has none.** The completion popup's separate
caret-context guard still suppresses completion after `#`; that is a UI guard, not a highlighter
class, and is unchanged here.

Known limits, stated rather than painted over:

- The editor lays out one row at a time (that is its virtualisation contract), so a string literal
  that physically spans rows is not carried across the row boundary. Every class in this round's
  craft script is single-line, and the grammar's ordinary scripts put strings on one line.
- Symbol discovery is lexical and scope-insensitive: a parameter or local with the same spelling
  as a discovered symbol is also symbol-coloured, and a `symbol("x")` call written inside an
  interpolation expression is skipped because `discover_symbols` treats the whole `$"..."` literal
  as text. The capture uses a top-level `symbol("t")` call.
- Builtins describe the **last committed** engine snapshot, matching completion. A script that
  redefines a builtin name is still painted as the builtin until its definition commits, then the
  snapshot's `is_builtin: false` entry wins.

## Code changes

- `rust/lovelace-desktop/src/ui/syntax.rs`
  - `TokenClass`, `Span`, `classify`, and `job` (the layout builder) are one classifier: the
    unit tests assert the same spans the editor paints.
  - `SyntaxContext` holds the engine builtin names and the document symbol names;
    `discover_symbols` scans the buffer for `symbol("...")` calls. `scan_number`,
    `scan_string`, `scan_interpolated`, `scan_operator`, and `is_delimiter` follow the parser.
  - The old `pub const BUILTINS` hardcoded list is gone.
  - 12 new tests (below).
- `rust/lovelace-desktop/src/ui/editor.rs`
  - `TextEditor` caches a `SyntaxContext`, refreshes builtins from the engine snapshot every paint
    pass (cheap comparison) and re-discovers symbols when the buffer's edit counter moves.
  - `job` is called with the context. The editor test drives the refresh directly.
- `rust/lovelace-desktop/src/ui/app.rs`
  - One `Workbench::completion_state()` capture now builds both the completion catalogue and the
    editor's highlighter context, so the highlighter sees the raw builtin flags (not the
    catalogue's shadowing dedup: a variable named `sqrt` must not hide the builtin colour).
- `rust/lovelace-desktop/src/workbench.rs`
  - The old convenience `completion_catalogue()` remains the host/test entry point and carries a
    documented `allow(dead_code)` now that the window builds from the shared capture.

## Unit tests and observed output

The 13 relevant tests are in `syntax.rs` and `editor.rs`; `syntax-tests.log` is the extracted
observed output. Each `assert_exact` checks that spans tile the whole line (start at 0, end at
`line.len()`, no gaps) and then compares the exact `(byte range, class)` vector of every
non-plain span.

| Test | What it pins |
| --- | --- |
| `integer_and_decimal_literals_are_one_number_class` | `12` and `3.25` Number; `.5` is one Number, not `.` + `5` |
| `repeating_decimals_are_the_grammars_only_exact_rational_literal` | `0.(3)` `0.1(6)` `0(3)` `12(34)` Rational; `1/3` is Number, Operator, Number |
| `plain_strings_include_quotes_and_escaped_quotes` | `"a\"b"` one String span; unterminated string; `#` inside a string stays String |
| `interpolation_braces_and_their_expression_are_classified_separately` | `$"x={x + 1}"` splits String / Interp / Symbol / Operator / Number; `{{x}}` is all String; `sqrt` inside interpolation is Builtin |
| `every_shared_keyword_is_painted_as_a_keyword` | iterates `KEYWORDS` (no second list), every keyword plus `func` alone |
| `the_declared_function_name_is_a_function_not_a_builtin` | `func sqrt(x)` makes `sqrt` Function, not Builtin; only the immediate identifier is declared |
| `builtins_come_from_the_engine_snapshot_not_a_desktop_list` | supplied `sqrt`/`squeeze` are Builtin; old-list `round`/`matrix`/`product`/`eval` are not |
| `user_functions_are_not_builtins` | an `is_builtin: false` engine function does not enter the builtin set |
| `operators_and_delimiters_use_the_parser_punctuator_set` | exact ranges for all operators and delimiters in one line |
| `symbol_calls_declare_the_symbolic_tokens_used_in_expressions` | `t`/`u` declared by `symbol(...)`; `"t"` stays String; `v` stays plain |
| `layout_job_sections_are_the_classified_spans_in_order` | every `LayoutJob` section's byte range, font and colour matches the classifier |
| `every_real_engine_registry_builtin_is_painted_as_builtin` | walks the real `Engine::capture_state` registry (>100 builtins); `squeeze` yes, `round` no |
| `syntax_context_refreshes_builtins_from_the_engine_and_symbols_from_the_buffer` | editor wiring: engine builtins, buffer symbols, cache invalidation on load and on a changed snapshot |

Observed output (`syntax-tests.log`):

```text
test ui::editor::tests::syntax_context_refreshes_builtins_from_the_engine_and_symbols_from_the_buffer ... ok
test ui::syntax::tests::builtins_come_from_the_engine_snapshot_not_a_desktop_list ... ok
test ui::syntax::tests::every_shared_keyword_is_painted_as_a_keyword ... ok
test ui::syntax::tests::integer_and_decimal_literals_are_one_number_class ... ok
test ui::syntax::tests::interpolation_braces_and_their_expression_are_classified_separately ... ok
test ui::syntax::tests::layout_job_sections_are_the_classified_spans_in_order ... ok
test ui::syntax::tests::operators_and_delimiters_use_the_parser_punctuator_set ... ok
test ui::syntax::tests::plain_strings_include_quotes_and_escaped_quotes ... ok
test ui::syntax::tests::every_real_engine_registry_builtin_is_painted_as_builtin ... ok
test ui::syntax::tests::repeating_decimals_are_the_grammars_only_exact_rational_literal ... ok
test ui::syntax::tests::symbol_calls_declare_the_symbolic_tokens_used_in_expressions ... ok
test ui::syntax::tests::the_declared_function_name_is_a_function_not_a_builtin ... ok
test ui::syntax::tests::user_functions_are_not_builtins ... ok
```

## Window proof

### Environment and method

The console session is still locked: the capture log records
`foreground=66290 (foreground is the lock screen in this session)`, never the product window.
`tools/gui-harness.ps1 -Action scenario` cannot pass its foreground assertion here, so this round
uses the same two-part method as round 16:

1. **Posted input.** After launch, `WM_SETFOCUS` is posted. Each character is delivered as
   `WM_KEYDOWN` (with `VK_PROCESSKEY`, `0xE5`) / `WM_CHAR` / `WM_KEYUP`; each newline is
   `VK_RETURN` down/up. The messages are queued while the window thread is suspended, because
   winit peeks the next keyboard message inside its `WM_KEYDOWN` handler and only then accepts
   the `WM_CHAR` as the text of that key event.
   - `VK_PROCESSKEY` matters: with the real character VK, `TranslateMessage` injects the
     **unshifted** character first (`4` for `$`, `9` for `(`, `'` for `"`). `VK_PROCESSKEY`
     leaves the posted `WM_CHAR` as the text.
   - The window thread suspend/resume is only around the three-message batch; it does not alter
     the editor's state machine or the painted result.
2. **Capture with `PrintWindow(PW_RENDERFULLCONTENT)`.** `CopyFromScreen` returns black on the
   locked desktop; `PrintWindow` asks the compositor for the window's own surface. One attempt
   succeeded; the script retries up to six times.

The compositor DPI changed during the round (96 dpi on the first runs, 288 dpi later). At 288 dpi
a physical 1296x839 window is a 432x280 logical pane and clips every line, and resizing it to a
1296-point logical window made wgpu reject a 10764x6017 surface (maximum 8192). The script
therefore launches the product with the standard `__COMPAT_LAYER=DPIUNAWARE` compatibility layer,
so the app's own surface is the normal 96-dpi one. This changes no product code and no rendering
path; it is the same 1x window surface the earlier rounds captured when the session reported
96 dpi. The script fails loudly if the layer does not yield 96 dpi.

The harness's unmodified `-Action uia` path captures the live accessibility tree (it needs no
foreground) and reports:

```text
Edit | Script editor, 5 lines, caret line 5 column 35 | id= | focusable=True | enabled=True
```

So the five lines and the caret after the final `"` really landed in the editor buffer.

### Craft script in the window

```text
func area(x) = x^2 + 1/3
for i in [1, 2] { if (i == 1) { continue } else { break } }
while (area(2) < 5) { return 0.(3) + pi(2) }
t = symbol("t")
z = $"t={t + 1.5}, area={area(2)}"
```

Line-by-line coverage:

- line 1: `func` Keyword, declared `area` Function, delimiters, `=`/`^`/`+`/`/` Operators,
  integer numbers; `1/3` is Number / Operator / Number, not a Rational literal.
- line 2: `for in if continue else break` Keywords, `[ ] { } ( ) ,` Delimiters, `==` Operator,
  numbers.
- line 3: `while`/`return` Keywords, `0.(3)` Rational (gold), `pi` Builtin (blue), numbers,
  operators and delimiters.
- line 4: `t` Symbol, `symbol` Builtin, `"t"` String. The `t` inside the string is not recoloured.
- line 5: `$"..."` String segments, `{`/`}` Interpolation (pink), `t` Symbol (lavender),
  `1.5` Number, `area` inside the call plain (only the declaration is `Function`), `z` plain.

### Artifact

- `docs/goal-desktop-v4/round-20/highlighting.png` - **1296x760**, **142,289 bytes**,
  SHA-256 `AF62874B5F849B8F9EE8D46E8F3FE25B2FC8521D6F723A1C9D4AD08020959FEB`.
  It is the live product window: five lines, caret after the final quote, status strip
  `Ln 5, Col 35 | run Idle`.
- `highlighting-detail.png` - 520x170 crop, 24,222 bytes, SHA-256
  `BF2650FDEADF450612867B3D7512FD5E71D6A1EE3DBED6288DA44C129BEFE813`.
- `highlighting-detail-2x.png` - 1040x340 nearest-neighbour enlargement, 35,520 bytes, SHA-256
  `5287C577B9D28F6481C548CEB21B3D33EB26DA61E0595E8FC2758983EBFF53BC`.
- `highlighting-capture.log` - the final launch/input/capture/palette log; it starts at
  `13:50:22` and records `dpi=96`, `rect=1296x760`, one successful `PrintWindow`, the UIA line
  and the palette counts.
- `uia-tree.txt` / `uia-driver.log` - the unmodified harness accessibility capture.
- `capture-highlighting.ps1` - the reproducible script.

### Palette probe

The probe counts exact `#RRGGBB` pixels in the editor text body only
(logical `x=68..500, y=121..251`), which excludes the gutter, the pane border and the
accent-coloured header line. Counts from the final run:

```text
plain #D4D9E0 pixels=22
number #B5CE8B pixels=78
rational #E5C07B pixels=15
string #CE9178 pixels=32
interpolation #F0A8D0 pixels=14
keyword #C586C0 pixels=83
operator #56B6C2 pixels=18
function #4EC9B0 pixels=11
builtin #4FC1FF pixels=8
delimiter #8B98A9 pixels=43
symbol #C8B6FF pixels=3
```

All eleven classes the palette defines were painted into the text region. This is a
presence check, not a claim that every pixel is fully opaque: small single glyphs can be entirely
antialiased (the `symbol` count is three exact pixels), which is why the unit tests assert
ranges and classes, not renderer output.

### Classes that are visually close, and one colour collision with chrome

No two **token classes** share a colour. The honest caveats:

- `Function` teal `#4ec9b0` is the same RGB as the window's accent colour (pane headers, active
  tab underline, pane border). In the text body the only teal run is the declared `area`; the
  probe crop deliberately excludes the header and border. If a future theme changes the accent,
  the function token and the chrome must be kept distinct or this collision grows.
- The closest token pair is `Keyword` `#c586c0` vs `Symbol` `#c8b6ff`: both are purple-family,
  distinguished by lightness/saturation. `Plain` `#d4d9e0` vs `Symbol` `#c8b6ff` is the next
  closest (both light), so a symbol inside plain text is the hardest distinction to see at
  13 px; the capture shows `t` in `{t + 1.5}` as lavender and the plain `area` call as light
  grey.
- `Rational` gold `#e5c07b` vs `Number` green `#b5ce8b` is clearly separable, but both are
  numeric-looking; the test table, not the colour, is the proof that `0.(3)` is the rational
  literal.

No class is asserted to be distinguishable by colour alone where that is not true; the exact
classes are proven by the unit-test spans, and the capture is corroborating visual evidence.

## Full suite summary

Environment from the round brief:

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

Observed (`cargo-test.log`):

```text
test result: ok. 187 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

Also observed on the fake-engine fallback (`cargo-test-fake-engine.log`):

```text
test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

The count grew from 174 to 187: 12 highlighter tests plus the editor's context-refresh test.

## Files

- `rust/lovelace-desktop/src/ui/syntax.rs` - classifier, palette, context, 12 tests.
- `rust/lovelace-desktop/src/ui/editor.rs` - syntax-context cache and refresh; job call site;
  editor wiring test.
- `rust/lovelace-desktop/src/ui/app.rs` - one engine snapshot feeds catalogue and highlighter.
- `rust/lovelace-desktop/src/workbench.rs` - note on the retained convenience entry point.
- `docs/goal-desktop-v4/round-20/` - this document, capture script, PNGs, logs, UIA tree.

No engine crate, `tools/`, or gate file was changed by this round.
