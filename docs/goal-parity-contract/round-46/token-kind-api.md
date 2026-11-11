# Round 46 — the compiled lexer `TokenKind` surface

**Status: complete.** `rust/lovelace-lang` now exposes an additive public lexer vocabulary
(`lovelace_lang::lexer`) that is the Rust equivalent of the C# `Lovelace.Suite.TokenKind` /
`Token` / `Tokenizer` surface. The twenty round-45 UNRUNNABLE rows
`TokenTests.TokenKind_GivenAllValues_HasExpectedOrdinal#0..#19` have exact ordinal tests; all
thirty variants have a classification test against the reference lexer's own cases. No parser
behaviour changed, and no `.cs` file, fixture, corpus example, parity mapping or existing test was
edited.

## 1. What round 45 recorded, and what was read

Round 45 grouped 20 rows under one missing API:

> compiled lexer TokenKind enum (`syntax::Tok` has 8 variants; token.rs is dead)

The contract is the C# library surface:

| Read | Fact used |
|---|---|
| `Lovelace.Suite/Token.cs` (lines 13–58) | `enum TokenKind` has **30 members**, not 20. Members 0–19 are the original REPL tokens (`NumberLiteral` through `Eof`); members 20–29 are the v1 additions appended after `Eof` so the 0–19 ordinals stay stable. |
| `Lovelace.Suite/Token.cs` (lines 75) | `Token` is a record with three fields: `Kind`, `Text`, `Position`, and structural equality. |
| `Lovelace.Suite/Tokenizer.cs` (199 lines) | `Tokenize` skips whitespace (newlines included), scans `$"…"` before `"…"`, numbers with an optional decimal and optional periodic `(digits)` suffix, ASCII identifiers, two-char operators before single-char, then punctuation; always appends one `Eof` token. |
| `Lovelace.Suite.Tests/TokenTests.cs` (lines 79–103) | The 20 rows are a `[Theory]` over 20 `[InlineData]` pairs; every body is `Assert.Equal(expectedOrdinal, (int)kind)`. |
| `TokenizerNumberTests.cs`, `TokenizerIdentifierTests.cs`, `TokenizerOperatorTests.cs` | The classification corpus mirrored by the new tests (same inputs, same texts and positions). |

The 20 rows assert only the enum ordinals, but they cannot compile without the enum type itself.
The Rust parser's `syntax::Tok` is a different vocabulary (8 shapes, newline → statement separator,
periodic literal → `Repr`), so it is not a substitute and was left untouched.

## 2. The additive surface

New file `rust/lovelace-lang/src/lexer.rs`, registered in `lib.rs` as `pub mod lexer;` (plus
`#[cfg(test)] mod round46_token_kind;` for the tests). Nothing existing was removed or reordered.

| Item | C# counterpart | Contract |
|---|---|---|
| `pub enum TokenKind` (`#[repr(i32)]`, explicit `= 0` … `= 29`) | `TokenKind` | Member names and ordinals are the reference ones; the 20 row-bearing members and the 10 appended v1 members are all present. |
| `TokenKind::ALL`, `COUNT` | (none) | Declaration order and count are inspectable/testing aids. |
| `TokenKind::ordinal()` | `(int)kind` | Returns the C# ordinal. |
| `TokenKind::as_str()` + `Display` | `kind.ToString()` | The enum member name. |
| `TokenKind::from_ordinal(i32)` | (none) | Inverse of the cast; `None` outside 0–29. |
| `pub struct Token { kind, text, position }` | `record Token(Kind, Text, Position)` | Derived `PartialEq`/`Eq` gives the reference record's structural equality over all three fields. `position` is a UTF-16 code-unit offset, as C# string indexing is. |
| `pub struct LexError { message, position }` | `InvalidOperationException` / `FormatException` | One typed refusal carrying the reference sentence verbatim and the offset as a field. |
| `pub struct Tokenizer` + `Tokenizer::tokenize(&self, input)` | `new Tokenizer().Tokenize(input)` | Facade mirroring the C# call shape. |
| `pub fn tokenize(input)` | `Tokenizer.Tokenize` | The scan itself; always ends with exactly one `TokenKind::Eof`. |

Classification is the **reference lexer's**, not the script normaliser's. In particular:

- a newline is whitespace and produces no token — the parser's own `syntax::tokenize` is what
  turns newlines into statement separators, and that behaviour is unchanged;
- `0.(3)` is one `NumberLiteral` with text `0.(3)` (the C# lexer has no `Repr` kind);
- `1..2` is `NumberLiteral(1)`, `DotDot`, `NumberLiteral(2)`, because the decimal scanner refuses
  `.` when the next character is also `.`;
- string text is the raw content between the quotes (no unescaping), and an unterminated string is
  refused with `Unterminated string literal at position N.`;
- an unknown character is refused with `Unexpected character 'c' at position N.`;
- the `Eof` token has empty text and position = input length in UTF-16 units.

## 3. Variant map — all 30 members of the C# enum

| Ordinal | C# `TokenKind` member | Rust | Lexeme / role | Census rows |
|---:|---|---|---|---|
| 0 | `NumberLiteral` | `TokenKind::NumberLiteral` | integer, decimal or periodic literal | #0 |
| 1 | `Identifier` | `TokenKind::Identifier` | `[a-zA-Z_][a-zA-Z0-9_]*` | #1 |
| 2 | `Plus` | `TokenKind::Plus` | `+` | #2 |
| 3 | `Minus` | `TokenKind::Minus` | `-` | #3 |
| 4 | `Star` | `TokenKind::Star` | `*` | #4 |
| 5 | `Slash` | `TokenKind::Slash` | `/` | #5 |
| 6 | `Percent` | `TokenKind::Percent` | `%` | #6 |
| 7 | `Caret` | `TokenKind::Caret` | `^` | #7 |
| 8 | `Bang` | `TokenKind::Bang` | `!` | #8 |
| 9 | `Equals` | `TokenKind::Equals` | `=` | #9 |
| 10 | `DoubleEquals` | `TokenKind::DoubleEquals` | `==` | #10 |
| 11 | `BangEquals` | `TokenKind::BangEquals` | `!=` | #11 |
| 12 | `Greater` | `TokenKind::Greater` | `>` | #12 |
| 13 | `Less` | `TokenKind::Less` | `<` | #13 |
| 14 | `GreaterEquals` | `TokenKind::GreaterEquals` | `>=` | #14 |
| 15 | `LessEquals` | `TokenKind::LessEquals` | `<=` | #15 |
| 16 | `LParen` | `TokenKind::LParen` | `(` | #16 |
| 17 | `RParen` | `TokenKind::RParen` | `)` | #17 |
| 18 | `Comma` | `TokenKind::Comma` | `,` | #18 |
| 19 | `Eof` | `TokenKind::Eof` | end of input | #19 |
| 20 | `DotDot` | `TokenKind::DotDot` | `..` range operator | appended |
| 21 | `LBrace` | `TokenKind::LBrace` | `{` | appended |
| 22 | `RBrace` | `TokenKind::RBrace` | `}` | appended |
| 23 | `LBracket` | `TokenKind::LBracket` | `[` | appended |
| 24 | `RBracket` | `TokenKind::RBracket` | `]` | appended |
| 25 | `StringLiteral` | `TokenKind::StringLiteral` | `"…"`, text = raw content | appended |
| 26 | `InterpolatedString` | `TokenKind::InterpolatedString` | `$"…"`, text = raw content | appended |
| 27 | `Semicolon` | `TokenKind::Semicolon` | `;` | appended |
| 28 | `Colon` | `TokenKind::Colon` | `:` | appended |
| 29 | `Dot` | `TokenKind::Dot` | `.` record member access | appended |

## 4. The 20 census ids this round unblocks

Each row is one `[InlineData]` of the C# theory; each has its own Rust test asserting the same
`(int)kind` value. The test names below are the crate-qualified names the manifest records.

| Census id | C# pair | Rust test |
|---|---|---|
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#0` | `(NumberLiteral, 0)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_00_number_literal` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#1` | `(Identifier, 1)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_01_identifier` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#2` | `(Plus, 2)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_02_plus` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#3` | `(Minus, 3)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_03_minus` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#4` | `(Star, 4)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_04_star` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#5` | `(Slash, 5)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_05_slash` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#6` | `(Percent, 6)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_06_percent` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#7` | `(Caret, 7)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_07_caret` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#8` | `(Bang, 8)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_08_bang` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#9` | `(Equals, 9)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_09_equals` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#10` | `(DoubleEquals, 10)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_10_double_equals` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#11` | `(BangEquals, 11)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_11_bang_equals` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#12` | `(Greater, 12)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_12_greater` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#13` | `(Less, 13)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_13_less` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#14` | `(GreaterEquals, 14)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_14_greater_equals` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#15` | `(LessEquals, 15)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_15_less_equals` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#16` | `(LParen, 16)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_16_l_paren` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#17` | `(RParen, 17)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_17_r_paren` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#18` | `(Comma, 18)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_18_comma` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#19` | `(Eof, 19)` | `lovelace-lang::round46_token_kind::token_kind_given_all_values_has_expected_ordinal_19_eof` |

All 20 ids are shown in full so they can be copied directly into the manifest.

## 5. Tests added — 51

All in `rust/lovelace-lang/src/round46_token_kind.rs`:

| Group | Count | What is asserted |
|---|---:|---|
| Ordinal rows | 20 | One test per census row: `TokenKind::X as i32 == expected`. |
| v1 ordinal append | 1 | Every 20–29 member keeps its appended ordinal, name and inverse lookup. |
| Inventory invariants | 1 | `ALL` is in discriminant order, `COUNT == 30`, out-of-range `from_ordinal` is `None`. |
| Operator / error cases | 8 | Single-char sequence, greater/less, two-char sequence, `==` disambiguation, whitespace positions, the two unknown-character refusals, and the complex-expression sequence — the C# `TokenizerOperatorTests` inputs. |
| Number cases | 7 | Integer, decimal, periodic, leading dot, multi-digit periodic, offset position, and `1..2` not swallowed by the number. |
| Identifier cases | 4 | Plain, underscore, leading underscore, digit suffix. |
| Strings | 3 | Plain content, interpolated content, empty literal. |
| v1 punctuation | 1 | `.. { } [ ] ; : .` produce the appended kinds with positions. |
| Whitespace / Eof | 2 | Newline is whitespace; trailing whitespace places `Eof` at input end. |
| Refusals | 2 | Both unterminated-string forms produce the reference sentence. |
| Facade / empty source | 2 | `Tokenizer` delegates to `tokenize`; empty input yields one `Eof` token. |

The groups total 51, and every one of the 30 kinds is reachable from a classification case.

Suite delta: **4,843 → 4,894 tests** (+51).

## 6. Gate outputs

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Rust suite | `cargo test --workspace` | `19` result lines, **4,894 passed, 0 failed** | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | no warnings; `Finished` dev profile | 0 |
| Formatting | `cargo fmt --all -- --check` | no output | 0 |

These are the outputs observed at 18:44 local, when this lane's 51 tests and the falsification
lane's empty placeholder module were compiled. The falsification lane then wrote its implementation
and ten `#[test]` functions at 18:45, after this run; those tests are not in the 4,894 count. The
corpus, fixture, CLI, clippy and fmt results do not depend on the new lexer surface (nothing existing
consumes it), but the workspace suite must be re-run once both lanes have landed; its count will be
4,894 plus whatever tests the falsification lane contributes.

## 7. What remains

- **Parity bookkeeping is intentionally outside this lane's scope.** `parity/manifest.toml`,
  `parity/manifest-map.json` and `parity/rust-tests.json` were not edited. The 20 ids in §4 should
  be moved from `deferred` to `ported` with the `lovelace-lang::round46_token_kind::…` test
  paths, and `parity/gen-rust-tests.ps1` must be re-run after this source edit so the manifest gate
  sees the new tests (it records `suiteExit=0`; the count changes from 4,843 to 4,894).
- **`rust/lovelace-lang/src/token.rs` is still dead code** (not declared in `lib.rs`; its enum
  order and normaliser differ from the reference). It was deliberately not revived or edited: the
  new surface is additive, and deleting a file is a separate cleanup decision.
- **No parser or runner change.** `syntax::Tok`, `syntax::tokenize`, the normaliser, the engine and
  the wire envelopes are byte-for-byte unchanged, which is why the corpus, fixture and CLI gates
  should stay at their prior values.
- **Concurrent lane note:** the parent reported another lane adding a `falsification_round46`
  module line to `lib.rs` while this round was in flight. This lane's `lib.rs` additions are only
  `pub mod lexer;` (between `pub mod help;` and `pub mod numeric;`) and
  `#[cfg(test)] mod round46_token_kind;` (after `mod round44_parity_classes;`); both must be
  reconciled, not overwritten. The falsification lane appended its own module line after ours and
  then landed its implementation and ten tests at 18:45, after the gate run above; its tests are a
  separate lane's count, and the workspace suite must be re-run at integration.
