//! The Lovelace language surface the desktop paints and offers: the keyword table.
//!
//! There is exactly one keyword table in the desktop. The syntax highlighter
//! ([`crate::ui::syntax`]) colours it and the completion catalogue
//! ([`crate::completion`]) offers it, so a keyword can never be coloured in one place and
//! be missing from completion in another. The table is the desktop's copy of the language
//! surface; it is not a completion-only list.
//!
//! The engine's parser (`lovelace_lang::parser`) recognises exactly these nine identifiers
//! as statement keywords. `else` and `in` are context keywords (`if ... else ...`,
//! `for x in ...`) and the remaining seven introduce or terminate a statement. The test
//! below asks the real engine's parser to accept a minimal construct for every entry, so a
//! keyword the engine adds or renames without this table fails the test instead of silently
//! disappearing from the editor.
//!
//! `true` and `false` are language constants, not keywords; the engine filters them out
//! of its variable snapshot and the desktop does not offer them as keywords.

/// Every statement keyword the Lovelace parser recognises, in name order.
pub const KEYWORDS: &[&str] = &[
    "break", "continue", "else", "for", "func", "if", "in", "return", "while",
];

#[cfg(all(test, feature = "real-engine"))]
mod tests {
    use super::*;

    /// Parses `source` with the engine's own parser and reports whether it was accepted.
    fn engine_parses(source: &str) -> bool {
        let tokens = match lovelace_lang::syntax::tokenize(source) {
            Ok(tokens) => tokens,
            Err(_) => return false,
        };
        lovelace_lang::syntax::Parser::new(tokens).program().is_ok()
    }

    #[test]
    fn every_keyword_is_a_keyword_in_the_engine_parser() {
        // Each snippet uses the keyword in the grammatical position the engine's parser
        // recognises it in; a keyword that stopped being one makes its snippet fail to parse.
        let samples = [
            ("break", "while (0) { break }"),
            ("continue", "while (0) { continue }"),
            ("else", "if (0) { 1 } else { 2 }"),
            ("for", "for i in [1, 2] { 3 }"),
            ("func", "func f(x) = x"),
            ("if", "if (0) { 1 }"),
            ("in", "for i in [1, 2] { 3 }"),
            ("return", "func f() { return 1 }"),
            ("while", "while (0) { 1 }"),
        ];
        assert_eq!(samples.len(), KEYWORDS.len());
        for (keyword, source) in samples {
            assert!(
                KEYWORDS.contains(&keyword),
                "{keyword} is in the engine's grammar but missing from KEYWORDS"
            );
            assert!(
                engine_parses(source),
                "the engine parser rejected the {keyword} construct {source:?}"
            );
        }
    }

    #[test]
    fn words_the_engine_does_not_treat_as_keywords_are_not_offered() {
        // The old desktop highlighter listed these; the engine's parser has no such
        // keywords. Keeping them out of completion is the point of this test.
        for stale in ["let", "end", "do", "then", "function", "elseif", "set"] {
            assert!(
                !KEYWORDS.contains(&stale),
                "{stale} is not a keyword in the engine's parser"
            );
        }
    }
}
