//! Ports of `Lovelace.Suite.Tests` lexer, token and parser cases (census rows that this
//! crate's public `syntax` module reproduces exactly).
//!
//! The C# assertion is the contract: every test below uses the same concrete source text and the
//! same structural expectation the C# test asserts (token payload/position, AST node shape, or a
//! diagnostic position). Cases whose Rust shape differs are refused in the round report with the
//! observable difference.

use crate::syntax::{Expr, Parser, Stmt, Tok, Token, tokenize};

/// The C# `Token` is a record with structural equality over its three fields. Rust's `Token` has
/// the same two stored fields (the text lives in the `Tok` payload), so the test module supplies
/// the same structural equality rather than weakening the C# equality assertions.
impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.tok == other.tok && self.pos == other.pos
    }
}

/// A stable structural rendering of one AST node. The C# parser tests assert node types and field
/// contents; this descriptor makes exactly those facts comparable without weakening them.
fn sexpr(e: &Expr) -> String {
    match e {
        Expr::Number(text) => format!("num({text})"),
        Expr::Repeating(text) => format!("repr({text})"),
        Expr::Str(text) => format!("str({text})"),
        Expr::Interp(parts) => format!("interp({} parts)", parts.len()),
        Expr::Var(name, _) => format!("var({name})"),
        Expr::Call(name, args, _) => {
            let args: Vec<String> = args.iter().map(sexpr).collect();
            format!("call({name}, [{}])", args.join(", "))
        }
        Expr::Binary(op, left, right) => format!("{op:?}({}, {})", sexpr(left), sexpr(right)),
        Expr::Neg(inner) => format!("neg({})", sexpr(inner)),
        Expr::Plus(inner) => format!("plus({})", sexpr(inner)),
        Expr::Not(inner) => format!("not({})", sexpr(inner)),
        Expr::Fact(inner) => format!("fact({})", sexpr(inner)),
        Expr::List(items) => {
            let items: Vec<String> = items.iter().map(sexpr).collect();
            format!("list([{}])", items.join(", "))
        }
        Expr::Range(start, step, end) => match step {
            Some(step) => format!("range({}, {}, {})", sexpr(start), sexpr(step), sexpr(end)),
            None => format!("range({}, {}, {})", sexpr(start), "none", sexpr(end)),
        },
        Expr::Index(base, args) => format!("index({}, {} args)", sexpr(base), args.len()),
        Expr::Assign(name, value) => format!("assign({name}, {})", sexpr(value)),
        Expr::Member(base, name) => format!("member({}, {name})", sexpr(base)),
    }
}

fn parse(src: &str) -> Expr {
    let toks = tokenize(src).expect("source tokenises");
    Parser::new(toks)
        .single_expression()
        .expect("source parses")
}

fn parse_shape(src: &str) -> String {
    sexpr(&parse(src))
}

fn stmt_shape(src: &str) -> String {
    let toks = tokenize(src).expect("source tokenises");
    let stmts = Parser::new(toks).program().expect("source parses");
    match stmts.into_iter().next().expect("one statement") {
        Stmt::Expr(e, _) => sexpr(&e),
        other => format!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TokenizerNumberTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_integer_literal_returns_single_number_token() {
    let toks = tokenize("42").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Num("42".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_decimal_literal_returns_single_number_token() {
    let toks = tokenize("3.14").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Num("3.14".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_periodic_literal_returns_single_number_token() {
    let toks = tokenize("0.(3)").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Repr("0.(3)".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_leading_dot_returns_single_number_token() {
    let toks = tokenize(".5").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Num(".5".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_periodic_with_multiple_digits_returns_single_number_token() {
    let toks = tokenize("1.2(345)").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Repr("1.2(345)".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_empty_string_returns_eof_only() {
    let toks = tokenize("").unwrap();
    assert_eq!(toks.len(), 1);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Eof,
            pos: 0
        }
    );
}

#[test]
fn tokenize_given_number_after_whitespace_reports_correct_position() {
    let toks = tokenize("  42").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Num("42".into()),
            pos: 2
        }
    );
}

// ---------------------------------------------------------------------------
// TokenizerIdentifierTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_identifier_returns_identifier_token() {
    let toks = tokenize("abc").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Ident("abc".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_identifier_with_underscore_returns_identifier_token() {
    let toks = tokenize("my_var").unwrap();
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Ident("my_var".into()),
            pos: 0
        }
    );
}

#[test]
fn tokenize_given_leading_underscore_identifier_returns_identifier_token() {
    let toks = tokenize("_result").unwrap();
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Ident("_result".into()),
            pos: 0
        }
    );
}

#[test]
fn tokenize_given_identifier_with_digit_suffix_returns_identifier_token() {
    let toks = tokenize("x1").unwrap();
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Ident("x1".into()),
            pos: 0
        }
    );
}

// ---------------------------------------------------------------------------
// TokenizerOperatorTests
// ---------------------------------------------------------------------------

#[test]
fn tokenize_given_single_char_operators_returns_correct_token_kinds() {
    let toks = tokenize("+-*/%^=(,)!").unwrap();
    assert_eq!(toks.len(), 12);
    let expected = [
        ("+", 0),
        ("-", 1),
        ("*", 2),
        ("/", 3),
        ("%", 4),
        ("^", 5),
        ("=", 6),
        ("(", 7),
        (",", 8),
        (")", 9),
        ("!", 10),
    ];
    for (i, (text, pos)) in expected.iter().enumerate() {
        assert_eq!(
            toks[i],
            Token {
                tok: Tok::Punct((*text).into()),
                pos: *pos
            },
            "token {i}"
        );
    }
    assert_eq!(toks[11].tok, Tok::Eof);
}

#[test]
fn tokenize_given_greater_and_less_returns_correct_token_kinds() {
    let toks = tokenize("> <").unwrap();
    assert_eq!(toks.len(), 3);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Punct(">".into()),
            pos: 0
        }
    );
    assert_eq!(
        toks[1],
        Token {
            tok: Tok::Punct("<".into()),
            pos: 2
        }
    );
    assert_eq!(toks[2].tok, Tok::Eof);
}

#[test]
fn tokenize_given_two_char_operators_returns_correct_token_kinds() {
    let toks = tokenize("== != >= <=").unwrap();
    assert_eq!(toks.len(), 5);
    let expected = [("==", 0), ("!=", 3), (">=", 6), ("<=", 9)];
    for (i, (text, pos)) in expected.iter().enumerate() {
        assert_eq!(
            toks[i],
            Token {
                tok: Tok::Punct((*text).into()),
                pos: *pos
            },
            "token {i}"
        );
    }
    assert_eq!(toks[4].tok, Tok::Eof);
}

#[test]
fn tokenize_given_double_equals_produces_one_double_equals_token() {
    let toks = tokenize("==").unwrap();
    assert_eq!(toks.len(), 2);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Punct("==".into()),
            pos: 0
        }
    );
    assert_eq!(toks[1].tok, Tok::Eof);
}

#[test]
fn tokenize_given_whitespace_skips_whitespace_preserves_positions() {
    let toks = tokenize("1 + 2").unwrap();
    assert_eq!(toks.len(), 4);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Num("1".into()),
            pos: 0
        }
    );
    assert_eq!(
        toks[1],
        Token {
            tok: Tok::Punct("+".into()),
            pos: 2
        }
    );
    assert_eq!(
        toks[2],
        Token {
            tok: Tok::Num("2".into()),
            pos: 4
        }
    );
    assert_eq!(toks[3].tok, Tok::Eof);
}

#[test]
fn tokenize_given_unknown_char_throws_descriptive_error() {
    let err = tokenize("@").expect_err("an unknown character is refused");
    // The C# assertion is that the message names the character and that position 0 is reported;
    // Rust carries the position as a typed field instead of repeating it in the message.
    assert!(err.message.contains('@'), "message: {}", err.message);
    assert_eq!(err.position, 0);
}

#[test]
fn tokenize_given_unknown_char_mid_input_throws_with_correct_position() {
    let err = tokenize("1 + @").expect_err("an unknown character is refused");
    assert!(err.message.contains('@'), "message: {}", err.message);
    assert_eq!(err.position, 4);
}

#[test]
fn tokenize_given_empty_string_returns_eof_only_for_operators() {
    let toks = tokenize("").unwrap();
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].tok, Tok::Eof);
}

#[test]
fn tokenize_given_complex_expression_returns_correct_sequence() {
    let toks = tokenize("a = 3.14 * b").unwrap();
    assert_eq!(toks.len(), 6);
    assert_eq!(
        toks[0],
        Token {
            tok: Tok::Ident("a".into()),
            pos: 0
        }
    );
    assert_eq!(
        toks[1],
        Token {
            tok: Tok::Punct("=".into()),
            pos: 2
        }
    );
    assert_eq!(
        toks[2],
        Token {
            tok: Tok::Num("3.14".into()),
            pos: 4
        }
    );
    assert_eq!(
        toks[3],
        Token {
            tok: Tok::Punct("*".into()),
            pos: 9
        }
    );
    assert_eq!(
        toks[4],
        Token {
            tok: Tok::Ident("b".into()),
            pos: 11
        }
    );
    assert_eq!(toks[5].tok, Tok::Eof);
}

// ---------------------------------------------------------------------------
// UnterminatedStringTests (the two whose Rust diagnostic matches)
// ---------------------------------------------------------------------------

#[test]
fn unterminated_interpolated_string_is_a_parse_error() {
    assert!(tokenize("$\"a{x}").is_err());
}

#[test]
fn well_formed_strings_still_tokenize() {
    let plain = &tokenize("\"abc\"").unwrap()[0];
    assert_eq!(plain.tok, Tok::Str("abc".into()));
    let interpolated = &tokenize("$\"a{x}\"").unwrap()[0];
    assert!(matches!(interpolated.tok, Tok::Interp(_)));
}

// ---------------------------------------------------------------------------
// TokenTests
// ---------------------------------------------------------------------------

#[test]
fn token_given_number_literal_kind_returns_correct_kind() {
    let token = Token {
        tok: Tok::Num("42".into()),
        pos: 0,
    };
    assert!(matches!(token.tok, Tok::Num(_)));
}

#[test]
fn token_given_text_returns_correct_text() {
    let token = Token {
        tok: Tok::Ident("myVar".into()),
        pos: 3,
    };
    match token.tok {
        Tok::Ident(text) => assert_eq!(text, "myVar"),
        other => panic!("wrong token: {other:?}"),
    }
}

#[test]
fn token_given_position_returns_correct_position() {
    let token = Token {
        tok: Tok::Punct("+".into()),
        pos: 7,
    };
    assert_eq!(token.pos, 7);
}

#[test]
fn token_given_equal_tokens_are_structurally_equal() {
    let t1 = Token {
        tok: Tok::Num("42".into()),
        pos: 0,
    };
    let t2 = Token {
        tok: Tok::Num("42".into()),
        pos: 0,
    };
    assert_eq!(t1, t2);
}

#[test]
fn token_given_different_kinds_are_not_equal() {
    let t1 = Token {
        tok: Tok::Num("42".into()),
        pos: 0,
    };
    let t2 = Token {
        tok: Tok::Ident("42".into()),
        pos: 0,
    };
    assert_ne!(t1, t2);
}

#[test]
fn token_given_different_texts_are_not_equal() {
    let t1 = Token {
        tok: Tok::Ident("a".into()),
        pos: 0,
    };
    let t2 = Token {
        tok: Tok::Ident("b".into()),
        pos: 0,
    };
    assert_ne!(t1, t2);
}

#[test]
fn token_given_different_positions_are_not_equal() {
    let t1 = Token {
        tok: Tok::Punct("+".into()),
        pos: 0,
    };
    let t2 = Token {
        tok: Tok::Punct("+".into()),
        pos: 5,
    };
    assert_ne!(t1, t2);
}

// ---------------------------------------------------------------------------
// ParserTests
// ---------------------------------------------------------------------------

#[test]
fn parse_given_number_literal_returns_literal_expr() {
    assert_eq!(parse_shape("42"), "num(42)");
}

#[test]
fn parse_given_identifier_returns_variable_expr() {
    assert_eq!(parse_shape("x"), "var(x)");
}

#[test]
fn parse_given_function_call_single_arg_returns_call_expr() {
    assert_eq!(parse_shape("abs(x)"), "call(abs, [var(x)])");
}

#[test]
fn parse_given_function_call_multiple_args_returns_call_expr() {
    assert_eq!(
        parse_shape("divrem(a, b)"),
        "call(divrem, [var(a), var(b)])"
    );
}

#[test]
fn parse_given_parenthesized_expression_returns_inner_expr() {
    assert_eq!(parse_shape("(1 + 2)"), "Add(num(1), num(2))");
}

#[test]
fn parse_given_unary_minus_returns_unary_expr_negate() {
    assert_eq!(parse_shape("-5"), "neg(num(5))");
}

#[test]
fn parse_given_postfix_factorial_returns_postfix_expr() {
    assert_eq!(parse_shape("5!"), "fact(num(5))");
}

#[test]
fn parse_given_chained_postfix_and_unary_correct_precedence() {
    assert_eq!(parse_shape("-5!"), "neg(fact(num(5)))");
}

#[test]
fn parse_given_addition_returns_binary_expr_add() {
    assert_eq!(parse_shape("1 + 2"), "Add(num(1), num(2))");
}

#[test]
fn parse_given_multiply_before_add_correct_precedence() {
    assert_eq!(parse_shape("1 + 2 * 3"), "Add(num(1), Mul(num(2), num(3)))");
}

#[test]
fn parse_given_power_right_associative_correct_tree() {
    assert_eq!(parse_shape("2 ^ 3 ^ 4"), "Pow(num(2), Pow(num(3), num(4)))");
}

#[test]
fn parse_given_parentheses_override_precedence_correct_tree() {
    assert_eq!(
        parse_shape("(1 + 2) * 3"),
        "Mul(Add(num(1), num(2)), num(3))"
    );
}

#[test]
fn parse_given_comparison_returns_binary_expr_with_op() {
    assert_eq!(parse_shape("a == b"), "Eq(var(a), var(b))");
}

#[test]
fn parse_given_comparison_lower_than_arithmetic_correct_precedence() {
    assert_eq!(parse_shape("a + 1 > b"), "Gt(Add(var(a), num(1)), var(b))");
}

#[test]
fn parse_given_range_before_power_range_binds_tighter() {
    assert_eq!(
        parse_shape("1..10 ^ 2"),
        "Pow(range(num(1), none, num(10)), num(2))"
    );
}

#[test]
fn parse_given_scalar_times_range_range_binds_tighter() {
    assert_eq!(
        parse_shape("2 * 1..5"),
        "Mul(num(2), range(num(1), none, num(5)))"
    );
}

#[test]
fn parse_given_range_plus_scalar_range_binds_tighter() {
    assert_eq!(
        parse_shape("1..5 + 1"),
        "Add(range(num(1), none, num(5)), num(1))"
    );
}

#[test]
fn parse_given_assignment_returns_assign_expr() {
    assert_eq!(stmt_shape("x = 5"), "assign(x, num(5))");
}

#[test]
fn parse_given_empty_input_throws_error() {
    let toks = tokenize("").unwrap();
    let err = Parser::new(toks)
        .single_expression()
        .expect_err("empty source is refused");
    assert!(!err.message.trim().is_empty());
}

#[test]
fn parse_given_unexpected_token_throws_descriptive_error() {
    let toks = tokenize("+ +").unwrap();
    let err = Parser::new(toks)
        .single_expression()
        .expect_err("unexpected token is refused");
    assert!(!err.message.trim().is_empty());
}
