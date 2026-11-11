//! Round 46: the C# `TokenKind` ordinal contract and the reference lexer's classification.
//!
//! The twenty `token_kind_given_all_values_has_expected_ordinal_*` tests are the census rows
//! `Lovelace.Suite.Tests/TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#0..#19`: each
//! asserts the exact `(int)kind` value the C# theory row asserts. The remaining cases mirror
//! `TokenizerNumberTests`, `TokenizerIdentifierTests` and `TokenizerOperatorTests` against the
//! new public `lexer` surface, so every one of the thirty kinds has a classification case.

use crate::lexer::{Token, TokenKind, Tokenizer, tokenize};

/// Scans a source the reference lexer accepts.
fn lex(input: &str) -> Vec<Token> {
    tokenize(input).expect("the reference lexer accepts the source")
}

/// Asserts one token's complete reference shape.
fn assert_token(actual: &Token, kind: TokenKind, text: &str, position: usize) {
    assert_eq!(
        actual,
        &Token {
            kind,
            text: String::from(text),
            position,
        },
        "expected {kind:?} {text:?} at {position}, got {actual:?}"
    );
}

// ---------------------------------------------------------------------------
// TokenKind_GivenAllValues_HasExpectedOrdinal, rows #0..#19
// ---------------------------------------------------------------------------

#[test]
fn token_kind_given_all_values_has_expected_ordinal_00_number_literal() {
    assert_eq!(TokenKind::NumberLiteral as i32, 0);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_01_identifier() {
    assert_eq!(TokenKind::Identifier as i32, 1);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_02_plus() {
    assert_eq!(TokenKind::Plus as i32, 2);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_03_minus() {
    assert_eq!(TokenKind::Minus as i32, 3);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_04_star() {
    assert_eq!(TokenKind::Star as i32, 4);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_05_slash() {
    assert_eq!(TokenKind::Slash as i32, 5);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_06_percent() {
    assert_eq!(TokenKind::Percent as i32, 6);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_07_caret() {
    assert_eq!(TokenKind::Caret as i32, 7);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_08_bang() {
    assert_eq!(TokenKind::Bang as i32, 8);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_09_equals() {
    assert_eq!(TokenKind::Equals as i32, 9);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_10_double_equals() {
    assert_eq!(TokenKind::DoubleEquals as i32, 10);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_11_bang_equals() {
    assert_eq!(TokenKind::BangEquals as i32, 11);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_12_greater() {
    assert_eq!(TokenKind::Greater as i32, 12);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_13_less() {
    assert_eq!(TokenKind::Less as i32, 13);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_14_greater_equals() {
    assert_eq!(TokenKind::GreaterEquals as i32, 14);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_15_less_equals() {
    assert_eq!(TokenKind::LessEquals as i32, 15);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_16_l_paren() {
    assert_eq!(TokenKind::LParen as i32, 16);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_17_r_paren() {
    assert_eq!(TokenKind::RParen as i32, 17);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_18_comma() {
    assert_eq!(TokenKind::Comma as i32, 18);
}

#[test]
fn token_kind_given_all_values_has_expected_ordinal_19_eof() {
    assert_eq!(TokenKind::Eof as i32, 19);
}

// ---------------------------------------------------------------------------
// The appended v1 members keep the 0-19 ordinals and extend them to 29
// ---------------------------------------------------------------------------

#[test]
fn token_kind_v1_additions_are_appended_after_eof() {
    let expected: [(TokenKind, i32, &str); 10] = [
        (TokenKind::DotDot, 20, "DotDot"),
        (TokenKind::LBrace, 21, "LBrace"),
        (TokenKind::RBrace, 22, "RBrace"),
        (TokenKind::LBracket, 23, "LBracket"),
        (TokenKind::RBracket, 24, "RBracket"),
        (TokenKind::StringLiteral, 25, "StringLiteral"),
        (TokenKind::InterpolatedString, 26, "InterpolatedString"),
        (TokenKind::Semicolon, 27, "Semicolon"),
        (TokenKind::Colon, 28, "Colon"),
        (TokenKind::Dot, 29, "Dot"),
    ];
    for (kind, ordinal, name) in expected {
        assert_eq!(kind as i32, ordinal, "{name}");
        assert_eq!(kind.ordinal(), ordinal, "{name}");
        assert_eq!(kind.as_str(), name, "{name}");
        assert_eq!(kind.to_string(), name, "{name}");
        assert_eq!(TokenKind::from_ordinal(ordinal), Some(kind), "{name}");
    }
}

#[test]
fn token_kind_all_lists_every_variant_in_discriminant_order() {
    assert_eq!(TokenKind::ALL.len(), TokenKind::COUNT);
    for (index, kind) in TokenKind::ALL.iter().enumerate() {
        let ordinal = index as i32;
        assert_eq!(kind.ordinal(), ordinal, "{kind:?}");
        assert_eq!(TokenKind::from_ordinal(ordinal), Some(*kind), "{kind:?}");
        assert!(!kind.as_str().is_empty(), "{kind:?}");
    }
    assert_eq!(TokenKind::from_ordinal(-1), None);
    assert_eq!(TokenKind::from_ordinal(TokenKind::COUNT as i32), None);
}

// ---------------------------------------------------------------------------
// TokenizerOperatorTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_single_char_operators_returns_correct_token_kinds() {
    let tokens = lex("+-*/%^=(,)!");
    assert_eq!(tokens.len(), 12);
    let expected: [(TokenKind, &str, usize); 11] = [
        (TokenKind::Plus, "+", 0),
        (TokenKind::Minus, "-", 1),
        (TokenKind::Star, "*", 2),
        (TokenKind::Slash, "/", 3),
        (TokenKind::Percent, "%", 4),
        (TokenKind::Caret, "^", 5),
        (TokenKind::Equals, "=", 6),
        (TokenKind::LParen, "(", 7),
        (TokenKind::Comma, ",", 8),
        (TokenKind::RParen, ")", 9),
        (TokenKind::Bang, "!", 10),
    ];
    for (i, (kind, text, position)) in expected.iter().enumerate() {
        assert_token(&tokens[i], *kind, text, *position);
    }
    assert_eq!(tokens[11].kind, TokenKind::Eof);
    assert_eq!(tokens[11].position, 11);
}

#[test]
fn tokenize_given_greater_and_less_returns_correct_token_kinds() {
    let tokens = lex("> <");
    assert_eq!(tokens.len(), 3);
    assert_token(&tokens[0], TokenKind::Greater, ">", 0);
    assert_token(&tokens[1], TokenKind::Less, "<", 2);
    assert_eq!(tokens[2].kind, TokenKind::Eof);
    assert_eq!(tokens[2].position, 3);
}

#[test]
fn tokenize_given_two_char_operators_returns_correct_token_kinds() {
    let tokens = lex("== != >= <=");
    assert_eq!(tokens.len(), 5);
    let expected: [(TokenKind, &str, usize); 4] = [
        (TokenKind::DoubleEquals, "==", 0),
        (TokenKind::BangEquals, "!=", 3),
        (TokenKind::GreaterEquals, ">=", 6),
        (TokenKind::LessEquals, "<=", 9),
    ];
    for (i, (kind, text, position)) in expected.iter().enumerate() {
        assert_token(&tokens[i], *kind, text, *position);
    }
    assert_eq!(tokens[4].kind, TokenKind::Eof);
    assert_eq!(tokens[4].position, 11);
}

#[test]
fn tokenize_given_double_equals_produces_one_double_equals_token() {
    let tokens = lex("==");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::DoubleEquals, "==", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_whitespace_skips_whitespace_preserves_positions() {
    let tokens = lex("1 + 2");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "1", 0);
    assert_token(&tokens[1], TokenKind::Plus, "+", 2);
    assert_token(&tokens[2], TokenKind::NumberLiteral, "2", 4);
    assert_eq!(tokens[3].kind, TokenKind::Eof);
    assert_eq!(tokens[3].position, 5);
}

#[test]
fn tokenize_given_unknown_char_throws_descriptive_error() {
    let err = tokenize("@").expect_err("an unknown character is refused");
    assert_eq!(err.message, "Unexpected character '@' at position 0.");
    assert_eq!(err.position, 0);
    assert_eq!(err.to_string(), err.message);
}

#[test]
fn tokenize_given_unknown_char_mid_input_throws_with_correct_position() {
    let err = tokenize("1 + @").expect_err("an unknown character is refused");
    assert_eq!(err.message, "Unexpected character '@' at position 4.");
    assert_eq!(err.position, 4);
}

#[test]
fn tokenize_given_empty_string_returns_eof_only() {
    let tokens = lex("");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], TokenKind::Eof, "", 0);
}

#[test]
fn tokenize_given_complex_expression_returns_correct_sequence() {
    let tokens = lex("a = 3.14 * b");
    assert_eq!(tokens.len(), 6);
    assert_token(&tokens[0], TokenKind::Identifier, "a", 0);
    assert_token(&tokens[1], TokenKind::Equals, "=", 2);
    assert_token(&tokens[2], TokenKind::NumberLiteral, "3.14", 4);
    assert_token(&tokens[3], TokenKind::Star, "*", 9);
    assert_token(&tokens[4], TokenKind::Identifier, "b", 11);
    assert_eq!(tokens[5].kind, TokenKind::Eof);
    assert_eq!(tokens[5].position, 12);
}

// ---------------------------------------------------------------------------
// TokenizerNumberTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_integer_literal_returns_single_number_token() {
    let tokens = lex("42");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "42", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_decimal_literal_returns_single_number_token() {
    let tokens = lex("3.14");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "3.14", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_periodic_literal_returns_single_number_token() {
    let tokens = lex("0.(3)");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "0.(3)", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_leading_dot_returns_single_number_token() {
    let tokens = lex(".5");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, ".5", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_periodic_with_multiple_digits_returns_single_number_token() {
    let tokens = lex("1.2(345)");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "1.2(345)", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_number_after_whitespace_reports_correct_position() {
    let tokens = lex("  42");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "42", 2);
    assert_eq!(tokens[1].position, 4);
}

#[test]
fn tokenize_given_range_is_not_consumed_by_the_number() {
    let tokens = lex("1..2");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "1", 0);
    assert_token(&tokens[1], TokenKind::DotDot, "..", 1);
    assert_token(&tokens[2], TokenKind::NumberLiteral, "2", 3);
    assert_eq!(tokens[3].kind, TokenKind::Eof);
}

// ---------------------------------------------------------------------------
// TokenizerIdentifierTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_identifier_returns_identifier_token() {
    let tokens = lex("abc");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::Identifier, "abc", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_identifier_with_underscore_returns_identifier_token() {
    let tokens = lex("my_var");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::Identifier, "my_var", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_leading_underscore_identifier_returns_identifier_token() {
    let tokens = lex("_result");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::Identifier, "_result", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn tokenize_given_identifier_with_digit_suffix_returns_identifier_token() {
    let tokens = lex("x1");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::Identifier, "x1", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

// ---------------------------------------------------------------------------
// Strings and the appended v1 punctuation
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_string_literal_returns_the_raw_content() {
    let tokens = lex("\"abc\"");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::StringLiteral, "abc", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
    assert_eq!(tokens[1].position, 5);
}

#[test]
fn tokenize_given_interpolated_string_returns_the_raw_content() {
    let tokens = lex("$\"a{x}\"");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::InterpolatedString, "a{x}", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
    assert_eq!(tokens[1].position, 7);
}

#[test]
fn tokenize_given_empty_string_literal_returns_an_empty_token() {
    let tokens = lex("\"\"");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::StringLiteral, "", 0);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
    assert_eq!(tokens[1].position, 2);
}

#[test]
fn tokenize_given_v1_punctuation_returns_the_appended_kinds() {
    let tokens = lex(".. { } [ ] ; : .");
    assert_eq!(tokens.len(), 9);
    let expected: [(TokenKind, &str, usize); 8] = [
        (TokenKind::DotDot, "..", 0),
        (TokenKind::LBrace, "{", 3),
        (TokenKind::RBrace, "}", 5),
        (TokenKind::LBracket, "[", 7),
        (TokenKind::RBracket, "]", 9),
        (TokenKind::Semicolon, ";", 11),
        (TokenKind::Colon, ":", 13),
        (TokenKind::Dot, ".", 15),
    ];
    for (i, (kind, text, position)) in expected.iter().enumerate() {
        assert_token(&tokens[i], *kind, text, *position);
    }
    assert_eq!(tokens[8].kind, TokenKind::Eof);
    assert_eq!(tokens[8].position, 16);
}

#[test]
fn tokenize_given_newline_skips_it_as_whitespace() {
    // The reference lexer treats a newline as whitespace; the parser's own normaliser is what
    // turns it into a statement separator. This surface must classify exactly as the reference.
    let tokens = lex("1\n+\n2");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "1", 0);
    assert_token(&tokens[1], TokenKind::Plus, "+", 2);
    assert_token(&tokens[2], TokenKind::NumberLiteral, "2", 4);
    assert_eq!(tokens[3].kind, TokenKind::Eof);
    assert_eq!(tokens[3].position, 5);
}

#[test]
fn tokenize_given_trailing_whitespace_places_eof_at_input_end() {
    let tokens = lex("42  ");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], TokenKind::NumberLiteral, "42", 0);
    assert_token(&tokens[1], TokenKind::Eof, "", 4);
}

#[test]
fn tokenize_given_unterminated_string_reports_the_reference_message() {
    let err = tokenize("\"abc").expect_err("an unterminated string is refused");
    assert_eq!(err.message, "Unterminated string literal at position 0.");
    assert_eq!(err.position, 0);
}

#[test]
fn tokenize_given_unterminated_interpolated_string_reports_the_reference_message() {
    let err = tokenize("$\"abc").expect_err("an unterminated string is refused");
    assert_eq!(err.message, "Unterminated string literal at position 0.");
    assert_eq!(err.position, 0);
}

#[test]
fn tokenizer_type_exposes_the_reference_facade() {
    let via_type = Tokenizer
        .tokenize("1+2")
        .expect("the reference tokenizer accepts the source");
    assert_eq!(via_type, lex("1+2"));
    assert_eq!(TokenKind::NumberLiteral.to_string(), "NumberLiteral");
}
