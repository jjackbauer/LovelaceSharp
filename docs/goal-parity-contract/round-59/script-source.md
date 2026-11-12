# Round 59 — the ScriptSource block: normaliser, source identity, and the position map

**Status: the seven census rows blocked on `ScriptSource` are now decided by an additive Rust
surface in `rust/lovelace-run`, with 15 new tests (7 exact mirrors + 8 covering the additive
identity/offset API). No existing runner behaviour, wire shape, evaluation path, `.cs` file,
fixture, `parity/` file or other document was changed; the only document written is this report.**

Measured 2026-11-11 (America/Sao_Paulo) on the working tree inherited at `e7f1bda` plus this
lane's changes. Another lane was editing `rust/lovelace-symbolics` (one new module, +10 tests)
throughout; it is not this lane's work and is called out in the gate table.

## 1. What the reference actually exposes

The requested "source identity plus position mapping" is **not** on `ScriptSource`. Reading the
whole type: `Lovelace.Suite/ScriptSource.cs` is a static class with exactly **one** member,
`ToSemicolonStatements(string)` (line 25). It has no name, path, origin, offset or
position-to-source member. The position map the diagnostics use lives in a different, internal
type: `Lovelace.Run/ScriptPositions.cs` (constructor line 36; `Source`/`EngineSource` 44/47;
`ToSourceOffset` 53; `LineColumn` 65). The C# host knows which file it read, so identity is
implicit there; the Rust runner has three source surfaces (`--eval`, `--stdin`, `--file`),
so this lane records the identity explicitly as an additive type.

### Member map

| C# member | Observed semantics | Rust equivalent |
|---|---|---|
| `ScriptSource.ToSemicolonStatements(string)` (`ScriptSource.cs:25`) | CRLF and lone CR become LF (line 27); one leading U+FEFF is stripped (31-32); a newline at bracket depth 0 becomes `;`, or a space at the start / after an existing `;` (78-90); newlines inside `{}`, `[]`, `()` and inside a double-quoted string are preserved (39-61, 86-89); the rewrite is length-preserving (26-27); a quote toggles string mode and a backslash has no special meaning (43-60). | `script_source::to_semicolon_statements(&str) -> String` |
| `ScriptPositions..ctor(string)` (`ScriptPositions.cs:36`) | Builds `EngineSource = ScriptSource.ToSemicolonStatements(source)` and the engine-to-source index (74-86). | `ScriptPositions::new(&str)` |
| `ScriptPositions.Source` / `EngineSource` (44/47) | The caller's text and the engine's normalised text. | `ScriptPositions::source()` / `engine_source()` |
| `ScriptPositions.ToSourceOffset(int)` (53-60) | Engine UTF-16 offset to caller UTF-16 offset; negative clamps to 0, offset at/past the end clamps to the caller's end. | `ScriptPositions::to_source_offset(isize) -> usize` |
| `ScriptPositions.LineColumn(int)` (65-66, 91-117) | 1-based line/column read off the caller's text; CRLF, CR and LF each end a line once; out-of-range is `(1, offset + 1)`. | `ScriptPositions::line_column(usize) -> (usize, usize)` |
| (no C# member) | Host-side source identity: display name, optional path, origin surface. | `SourceIdentity` + `SourceOrigin::{Eval, File, Stdin}` |
| (no C# member) | A diagnostic paired with the source it names, still serialising the four wire fields only. | `Diagnostic` (`message/position/line/column` + `source()`) |
| (no C# member) | One bundle of identity + caller text + normalised text + map. | `ScriptSource` |

The offset scan is over **UTF-16 code units**, matching the published protocol (a non-BMP
character occupies two units); a test pins that a char-based scan would lose one.

## 2. The seven census ids

All seven are `Lovelace.Suite.Tests.ScriptSourceTests` methods; each mirror asserts the C#
`Assert.Equal` byte-for-byte on the same input literal.

| # | Census id | C# source | Rust test (inventory id) |
|---:|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenNewlineSeparatedStatements_JoinsWithSemicolons#0` | `ScriptSourceTests.cs:8` | `lovelace-run::script_source::tests::to_semicolon_statements_given_newline_separated_statements_joins_with_semicolons` |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenCrlf_NormalizesLineEndings#0` | `ScriptSourceTests.cs:16` | `lovelace-run::script_source::tests::to_semicolon_statements_given_crlf_normalizes_line_endings` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenBlock_PreservesNewlinesInsideBraces#0` | `ScriptSourceTests.cs:24` | `lovelace-run::script_source::tests::to_semicolon_statements_given_block_preserves_newlines_inside_braces` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenNewlineAfterBlock_EmitsSemicolon#0` | `ScriptSourceTests.cs:32` | `lovelace-run::script_source::tests::to_semicolon_statements_given_newline_after_block_emits_semicolon` |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenStringWithNewline_PreservesIt#0` | `ScriptSourceTests.cs:40` | `lovelace-run::script_source::tests::to_semicolon_statements_given_string_with_newline_preserves_it` |
| 6 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenTrailingSemicolon_SuppressesBlankSeparator#0` | `ScriptSourceTests.cs:48` | `lovelace-run::script_source::tests::to_semicolon_statements_given_trailing_semicolon_suppresses_blank_separator` |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenLeadingBom_StripsIt#0` | `ScriptSourceTests.cs:56` | `lovelace-run::script_source::tests::to_semicolon_statements_given_leading_bom_strips_it` |

The eighth C# method in the class, `EvaluateAsync_GivenMultiLineScript_PlotsOneOverXSquared`
(line 64), was already `ported` (to `lovelace-lang::suite_engine_batch`) and is untouched.

## 3. Additive surface and wiring

New file `rust/lovelace-run/src/script_source.rs`:

* `to_semicolon_statements` — the exact port of the C# normaliser (including the "suppress a
  separator after `;`" and "at the start" rules, and `char::is_whitespace` for the last
  non-whitespace tracker);
* `SourceOrigin` / `SourceIdentity` — `eval` has name `<eval>`, `stdin` has `<stdin>`,
  `file` carries the path as both name and `path()`;
* `ScriptPositions` — the `ScriptPositions.cs` port, including the CRLF skip in the index
  build, negative/overrun clamping in `to_source_offset`, and the CRLF/CR/LF line rule;
* `Diagnostic` — holds `message`, `position`, `line`, `column` and the `SourceIdentity`;
  `to_json()` emits **only** the four fields the wire always had (pinned by a test);
* `ScriptSource` — `new(identity, text)`, `text()`, `normalized()`, `positions()`,
  `diagnostic(message, source_offset)` and `diagnostic_at_engine_offset(message, engine_offset)`.

Wiring in `main.rs` (behaviour-preserving):

1. `Options` gains an internal `origin` (`SourceOrigin`), set by `--eval` / `--stdin` /
   `--file` / a bare path; the source is wrapped once in `ScriptSource::new(identity, text)`.
2. `envelope_for`, `success_envelope`, `cancelled_envelope` and `failure_envelope` take
   `&ScriptSource` instead of `&str`; the old `line_col` helper is replaced by
   `ScriptPositions::line_column` through `ScriptSource::diagnostic`.
3. Failure and overrun diagnostics are built by `script.diagnostic(...).to_json()`, so every
   diagnostic value is paired with the source identity while the JSON document, key set, field
   order, exit codes and evaluation path (the caller's text) are unchanged.
4. The three in-process harness call sites (`parity_batch`, `round35_run_batch`,
   `error_taxonomy_batch`) construct the same `ScriptSource`; no assertion changed.

The normalised text and `to_source_offset` are host-facing API today: the live tokenizer already
treats a top-level newline as a separator, so the runner keeps evaluating the caller's text and no
offset translation is needed on its path. They are `#[allow(dead_code)]` at the module
declaration for exactly that reason (same pattern as `mod schemas`), not because they are unused
by tests.

## 4. Tests

15 new tests in `script_source::tests`, all green:

* the 7 mirrors in section 2;
* `source_identity_distinguishes_eval_stdin_and_file`;
* `script_source_reports_the_caller_text_and_its_normalised_form`;
* `positions_map_crlf_engine_offsets_back_to_the_callers_text` (engine offset 5 = LF half of
  CRLF -> source 6; offset 6 -> 7; negative/overrun clamps; line 2 column 1);
* `positions_map_a_leading_bom_back_to_the_callers_text` (engine 0 -> source 1, line 1 col 2);
* `positions_are_utf16_code_unit_offsets` (emoji surrogate pair: column 8 at UTF-16 offset 7,
  14-unit engine text);
* `diagnostic_names_its_source_and_keeps_the_four_field_wire_shape`;
* `diagnostic_at_engine_offset_translates_before_it_names_the_line`;
* `empty_source_has_no_offset_a_script_can_name`.

## 5. Gate outputs

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Workspace suite | `cargo test --workspace` | **5,054 passed, 0 failed** across 19 result lines; `lovelace-run` **395 passed** (was 380, +15). The 5,054 includes +10 tests landed concurrently in `lovelace-symbolics` by the other lane (its diff is one `mod` line + new `rewrite_registry.rs`); this lane's contribution is exactly +15. | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`; `rust: 20/20`; `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | no diagnostics | 0 |
| Format | `cargo fmt --all -- --check` | no diff | 0 |

The release runner used by the fixture/CLI gates was rebuilt from this tree
(`cargo build --release -p lovelace-run`) before those gates ran.

## 6. What remains (outside this lane's scope)

1. **Manifest mapping.** `parity/manifest-map.json` was **not** edited (explicit scope ban).
   The seven rows are still `deferred` with the round-45 reason; they should be changed to
   `ported` with the `rust` ids in section 2 (4 space-indented lines each, as round 58 did),
   then `parity/gen-rust-tests.ps1` and `node parity/gen-manifest.mjs` run to regenerate the
   inventory/manifest. The manifest gate will otherwise report the Rust inventory stale, which is
   expected after adding 15 tests.
2. **No wire promotion of the identity.** The envelope intentionally keeps the four-field
   diagnostic; `Diagnostic::source()` is available to an in-process host, not to a JSON consumer.
   Adding a `source` field would be a wire-shape change and was not done.
3. **Normaliser on the evaluation path.** The runner does not call
   `to_semicolon_statements` before `Engine::eval` (it does not need to). If a future surface
   wants the C# preprocessing semantics exactly, `ScriptSource::diagnostic_at_engine_offset` is
   the ready path; today it is host API plus tests.
