//! Tokens, the AST, and the recursive-descent parser.
//!
//! The grammar mirrors the documented language: a range binds tighter than arithmetic, a power
//! binds tighter than a range but is right-associative, unary minus binds tighter than a range,
//! and postfix call/index/factorial bind tightest.

use crate::LangError;

/// One lexical token.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Integer or decimal literal text.
    Num(String),
    /// A repeating literal such as 0.(3), stored as the full source text.
    Repr(String),
    /// A quoted string (already unescaped for the plain form).
    Str(String),
    /// An interpolated string, kept as its raw body for the parser to split.
    Interp(String),
    /// Identifier or keyword.
    Ident(String),
    /// A punctuator.
    Punct(String),
    /// Statement separator (newline or semicolon).
    Semi,
    /// End of input.
    Eof,
}

/// A token with its source position (UTF-16 code units).
#[derive(Clone, Debug)]
pub struct Token {
    /// The token.
    pub tok: Tok,
    /// Offset of the first code unit.
    pub pos: usize,
}

/// The published input-nesting budget (the reference's InputDepth.Max).
pub const INPUT_DEPTH_MAX: usize = 256;

/// A token's own source text, as the reference's Token.Text publishes it.
fn token_text(token: &Tok) -> &str {
    match token {
        Tok::Num(t)
        | Tok::Repr(t)
        | Tok::Str(t)
        | Tok::Interp(t)
        | Tok::Ident(t)
        | Tok::Punct(t) => t,
        Tok::Semi => ";",
        Tok::Eof => "",
    }
}

/// Binary operators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// %
    Rem,
    /// ^
    Pow,
    /// ==
    Eq,
    /// !=
    Ne,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
}

/// An index specification inside brackets.
#[derive(Clone, Debug)]
pub enum IndexArg {
    /// A single index expression.
    Single(Expr),
    /// A slice a:b or a:b:c, with colons as None bounds.
    Slice(Option<Expr>, Option<Expr>, Option<Expr>),
    /// The bare colon, meaning the whole axis.
    Full,
}

/// An expression node.
#[derive(Clone, Debug)]
pub enum Expr {
    /// A numeric literal (integer or decimal).
    Number(String),
    /// A repeating literal, e.g. 0.(3).
    Repeating(String),
    /// A string literal.
    Str(String),
    /// An interpolated string, split into literal and expression parts.
    Interp(Vec<InterpPart>),
    /// A variable reference.
    Var(String, usize),
    /// A function call.
    Call(String, Vec<Expr>, usize),
    /// Binary application.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Unary minus.
    Neg(Box<Expr>),
    /// Logical not.
    Not(Box<Expr>),
    /// Factorial.
    Fact(Box<Expr>),
    /// A list literal.
    List(Vec<Expr>),
    /// A range a..b or a..step..b.
    Range(Box<Expr>, Option<Box<Expr>>, Box<Expr>),
    /// Indexing with one or more arguments.
    Index(Box<Expr>, Vec<IndexArg>),
    /// Assignment (yields the assigned value).
    Assign(String, Box<Expr>),
    /// Field access on a record.
    Member(Box<Expr>, String),
}

/// One piece of an interpolated string.
#[derive(Clone, Debug)]
pub enum InterpPart {
    /// Literal text.
    Text(String),
    /// An embedded expression.
    Expr(Expr),
}

/// A statement node.
#[derive(Clone, Debug)]
pub enum Stmt {
    /// An expression evaluated for its value.
    Expr(Expr, usize),
    /// if (cond) then [else other]
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    /// while (cond) body
    While(Expr, Box<Stmt>),
    /// for NAME in range body
    For(String, Expr, Box<Stmt>),
    /// { ... } with its own value = the last statement's value
    Block(Vec<Stmt>),
    /// return [expr]
    Return(Option<Expr>, usize),
    /// break
    Break,
    /// continue
    Continue,
    /// func name(params) = expr | { body }
    Func(String, Vec<String>, Box<Stmt>),
    /// An empty statement.
    Empty,
}

/// Tokenises a source string.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LangError> {
    let chars: Vec<char> = src.chars().collect();
    let mut positions: Vec<usize> = Vec::with_capacity(chars.len());
    let mut unit = 0usize;
    for c in &chars {
        positions.push(unit);
        unit += c.len_utf16();
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let pos = positions[i];
        if c == '\n' {
            out.push(Token {
                tok: Tok::Semi,
                pos,
            });
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // The reference language has NO comment syntax: the tokenizer has no comment branch and
        // '#' is simply not a token, so a comment attempt is a ParseError naming the character
        // (Unexpected character '#' at position N). A permissive comment branch here would accept
        // a script the oracle refuses and answer a different exit code, which is a wire difference.
        if c == '"' {
            let (text, next) = scan_string(&chars, i + 1, pos)?;
            out.push(Token {
                tok: Tok::Str(text),
                pos,
            });
            i = next;
            continue;
        }
        if c == '$' && i + 1 < chars.len() && chars[i + 1] == '"' {
            let (text, next) = scan_string(&chars, i + 2, pos)?;
            out.push(Token {
                tok: Tok::Interp(text),
                pos,
            });
            i = next;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            let mut text = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                text.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1] == '(' {
                // Repeating literal: 0.(3) or 0.1(6)
                let mut j = i + 2;
                let mut period = String::new();
                while j < chars.len() && chars[j].is_ascii_digit() {
                    period.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == ')' && !period.is_empty() {
                    text.push('.');
                    text.push('(');
                    text.push_str(&period);
                    text.push(')');
                    i = j + 1;
                    out.push(Token {
                        tok: Tok::Repr(text),
                        pos,
                    });
                    continue;
                }
            }
            if i < chars.len()
                && chars[i] == '.'
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit()
            {
                text.push('.');
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    text.push(chars[i]);
                    i += 1;
                }
            }
            // Repeating form: 0.(3) or 0.1(6)
            if i < chars.len()
                && chars[i] == '('
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit()
            {
                let mut period = String::new();
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    period.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == ')' {
                    text.push('(');
                    text.push_str(&period);
                    text.push(')');
                    i = j + 1;
                    out.push(Token {
                        tok: Tok::Repr(text),
                        pos,
                    });
                    continue;
                }
            }
            let _ = start;
            out.push(Token {
                tok: Tok::Num(text),
                pos,
            });
            continue;
        }
        // Identifiers are ASCII: the reference tokenizer accepts [A-Za-z_] and then
        // [A-Za-z0-9_], so a non-ASCII letter is an Unexpected-character ParseError (position in
        // UTF-16 code units, which is what the diagnostic carries). Unicode is still allowed
        // inside string literals; only the identifier alphabet is ASCII.
        if c.is_ascii_alphabetic() || c == '_' {
            let mut text = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                text.push(chars[i]);
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(text),
                pos,
            });
            continue;
        }
        // Punctuators, longest first.
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if two == ".." || two == "==" || two == "!=" || two == "<=" || two == ">=" {
            out.push(Token {
                tok: Tok::Punct(two),
                pos,
            });
            i += 2;
            continue;
        }
        if c == ';' {
            out.push(Token {
                tok: Tok::Semi,
                pos,
            });
            i += 1;
            continue;
        }
        let one = c.to_string();
        if "+-*/%^!=<>()[]{},:.".contains(c) {
            out.push(Token {
                tok: Tok::Punct(one),
                pos,
            });
            i += 1;
            continue;
        }
        return Err(
            LangError::plain(format!("Unexpected character '{}' at position {}.", c, pos))
                .with_position(pos),
        );
    }
    out.push(Token {
        tok: Tok::Eof,
        pos: unit,
    });
    Ok(out)
}

fn scan_string(
    chars: &[char],
    mut i: usize,
    start_pos: usize,
) -> Result<(String, usize), LangError> {
    let mut text = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            return Ok((text, i + 1));
        }
        if c == '\\' && i + 1 < chars.len() {
            text.push(chars[i]);
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        text.push(c);
        i += 1;
    }
    Err(LangError::coded(
        format!("Unterminated string literal at position {start_pos}."),
        start_pos,
        "InvalidInput",
        "ParseError",
    ))
}

/// The reference's token-kind name for a punctuator, as its Expect publishes it.
fn punctuation_kind_name(p: &str) -> &'static str {
    match p {
        "(" => "LParen",
        ")" => "RParen",
        "[" => "LBracket",
        "]" => "RBracket",
        "{" => "LBrace",
        "}" => "RBrace",
        "," => "Comma",
        ":" => "Colon",
        "." => "Dot",
        "=" => "Equals",
        ";" => "Semicolon",
        _ => "Token",
    }
}

/// The parser.
pub struct Parser {
    toks: Vec<Token>,
    i: usize,
    /// Live descent depth, in the reference's units (one per statement, expression, power or
    /// consumed prefix sign). Checked on the way DOWN, so the refusal happens before the recursion
    /// that would exhaust the process stack.
    depth: usize,
}

impl Parser {
    /// Builds a parser over a token stream.
    pub fn new(toks: Vec<Token>) -> Parser {
        Parser {
            toks,
            i: 0,
            depth: 0,
        }
    }

    /// Opens one nesting level, refusing past the published input budget.
    fn enter(&mut self, stage: &str) -> Result<(), LangError> {
        self.depth += 1;
        if self.depth > INPUT_DEPTH_MAX {
            let depth = self.depth;
            self.depth -= 1;
            return Err(LangError::depth(stage, depth, INPUT_DEPTH_MAX, "nesting")
                .with_position(self.pos()));
        }
        Ok(())
    }

    /// Closes one nesting level.
    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.i.min(self.toks.len() - 1)].tok
    }

    fn pos(&self) -> usize {
        self.toks[self.i.min(self.toks.len() - 1)].pos
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.i.min(self.toks.len() - 1)].tok.clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn eat_punct(&mut self, p: &str) -> bool {
        if let Tok::Punct(q) = self.peek() {
            if q == p {
                self.bump();
                return true;
            }
        }
        false
    }

    fn expect_punct(&mut self, p: &str) -> Result<(), LangError> {
        if self.eat_punct(p) {
            Ok(())
        } else {
            // The reference's Expect publishes the TOKEN KIND it wanted (RParen, LBrace, Comma,
            // …) and the token it found, not the punctuation glyph. A consumer that compares
            // envelopes reads the same sentence from both runners.
            Err(self.err(&format!(
                "Expected '{}' but found '{}' at position {}.",
                punctuation_kind_name(p),
                token_text(self.peek()),
                self.pos()
            )))
        }
    }

    /// The separator between two statements is mandatory: a whitespace-only gap is a parse
    /// error, exactly as it is in the reference. A juxtaposition such as `2x` is therefore a
    /// refusal, never an implicit product.
    fn separator_error(&self, terminator: &str) -> LangError {
        self.err(&format!(
            "Expected ';' or {} but found '{}' at position {}.",
            terminator,
            token_text(self.peek()),
            self.pos()
        ))
    }

    /// The reference's primary-expression refusal for a token that cannot start one.
    fn unexpected_primary(&self) -> LangError {
        self.err(&format!(
            "Unexpected token '{}' at position {}: expected a number, string, identifier, '[', or '('.",
            token_text(self.peek()),
            self.pos()
        ))
    }

    fn err(&self, msg: &str) -> LangError {
        LangError::plain(msg).with_position(self.pos())
    }

    fn eat_semi(&mut self) {
        while matches!(self.peek(), Tok::Semi) {
            self.bump();
        }
    }

    /// Parses a whole program.
    pub fn program(&mut self) -> Result<Vec<Stmt>, LangError> {
        let mut out = Vec::new();
        self.eat_semi();
        while !matches!(self.peek(), Tok::Eof) {
            out.push(self.statement()?);
            if matches!(self.peek(), Tok::Semi) {
                self.eat_semi();
            } else if matches!(self.peek(), Tok::Eof) {
                break;
            } else if matches!(self.peek(), Tok::Punct(p) if p == "}") {
                // The reference lets a stray '}' past the separator check and refuses it as a
                // primary token; the sentence and position are the same either way.
                return Err(self.unexpected_primary());
            } else {
                return Err(self.separator_error("end of input"));
            }
        }
        Ok(out)
    }

    /// Parses a single expression (used for interpolation).
    pub fn single_expression(&mut self) -> Result<Expr, LangError> {
        let e = self.expression()?;
        Ok(e)
    }

    fn statement(&mut self) -> Result<Stmt, LangError> {
        self.enter("statement nesting")?;
        let result = self.statement_inner();
        self.leave();
        result
    }

    fn statement_inner(&mut self) -> Result<Stmt, LangError> {
        let pos = self.pos();
        if let Tok::Ident(name) = self.peek().clone() {
            match name.as_str() {
                "if" => {
                    self.bump();
                    self.expect_punct("(")?;
                    let cond = self.expression()?;
                    self.expect_punct(")")?;
                    self.eat_semi();
                    let then = self.statement()?;
                    // The separator after the 'then' branch belongs to the ENCLOSING statement
                    // list unless an 'else' actually follows it. Consuming it unconditionally
                    // made `if (c) { … }; next` lose the ';' and refuse `next` at the block
                    // boundary.
                    let checkpoint = self.i;
                    self.eat_semi();
                    let mut other = None;
                    if let Tok::Ident(k) = self.peek().clone() {
                        if k == "else" {
                            self.bump();
                            self.eat_semi();
                            other = Some(Box::new(self.statement()?));
                        }
                    }
                    if other.is_none() {
                        self.i = checkpoint;
                    }
                    return Ok(Stmt::If(cond, Box::new(then), other));
                }
                "while" => {
                    self.bump();
                    self.expect_punct("(")?;
                    let cond = self.expression()?;
                    self.expect_punct(")")?;
                    self.eat_semi();
                    let body = self.statement()?;
                    return Ok(Stmt::While(cond, Box::new(body)));
                }
                "for" => {
                    self.bump();
                    let var = match self.peek().clone() {
                        Tok::Ident(v) => {
                            self.bump();
                            v
                        }
                        _ => {
                            return Err(self.err(&format!(
                                "Expected 'Ident' but found '{}' at position {}.",
                                token_text(self.peek()),
                                self.pos()
                            )));
                        }
                    };
                    match self.peek().clone() {
                        Tok::Ident(k) if k == "in" => {
                            self.bump();
                        }
                        _ => {
                            return Err(self.err(&format!(
                                "Expected 'in' after loop variable but found '{}' at position {}.",
                                token_text(self.peek()),
                                self.pos()
                            )));
                        }
                    }
                    let range = self.expression()?;
                    self.eat_semi();
                    let body = self.statement()?;
                    return Ok(Stmt::For(var, range, Box::new(body)));
                }
                "return" => {
                    self.bump();
                    if matches!(self.peek(), Tok::Semi | Tok::Eof) || self.is_block_end() {
                        return Ok(Stmt::Return(None, pos));
                    }
                    let e = self.expression()?;
                    return Ok(Stmt::Return(Some(e), pos));
                }
                "break" => {
                    self.bump();
                    return Ok(Stmt::Break);
                }
                "continue" => {
                    self.bump();
                    return Ok(Stmt::Continue);
                }
                "func" => {
                    self.bump();
                    let fname = match self.peek().clone() {
                        Tok::Ident(n) => {
                            self.bump();
                            n
                        }
                        _ => {
                            return Err(self.err(&format!(
                                "Expected function name but found '{}' at position {}.",
                                token_text(self.peek()),
                                self.pos()
                            )));
                        }
                    };
                    self.expect_punct("(")?;
                    let mut params = Vec::new();
                    if !self.eat_punct(")") {
                        loop {
                            match self.peek().clone() {
                                Tok::Ident(p) => {
                                    self.bump();
                                    params.push(p);
                                }
                                _ => {
                                    return Err(self.err(&format!(
                                        "Expected 'Ident' but found '{}' at position {}.",
                                        token_text(self.peek()),
                                        self.pos()
                                    )));
                                }
                            }
                            if self.eat_punct(",") {
                                continue;
                            }
                            self.expect_punct(")")?;
                            break;
                        }
                    }
                    if self.eat_punct("=") {
                        let body = self.expression()?;
                        return Ok(Stmt::Func(fname, params, Box::new(Stmt::Expr(body, pos))));
                    }
                    if !matches!(self.peek(), Tok::Punct(p) if p == "{") {
                        return Err(self.err(&format!(
                            "Expected '{{' or '=' after function signature but found '{}' at position {}.",
                            token_text(self.peek()),
                            self.pos()
                        )));
                    }
                    let body = self.statement()?;
                    return Ok(Stmt::Func(fname, params, Box::new(body)));
                }
                _ => {}
            }
            // Assignment?
            if let Tok::Punct(p) = &self.toks[(self.i + 1).min(self.toks.len() - 1)].tok {
                if p == "=" && name != "=" {
                    self.bump();
                    self.bump();
                    let value = self.expression()?;
                    return Ok(Stmt::Expr(Expr::Assign(name, Box::new(value)), pos));
                }
            }
        }
        if self.eat_punct("{") {
            let mut stmts = Vec::new();
            self.eat_semi();
            while !self.is_block_end() && !matches!(self.peek(), Tok::Eof) {
                stmts.push(self.statement()?);
                if matches!(self.peek(), Tok::Semi) {
                    self.eat_semi();
                } else if !self.is_block_end() {
                    // A block that runs to end of input is a separator refusal naming the
                    // missing '}', exactly as the reference's ParseBlock does.
                    return Err(self.separator_error("'}'"));
                }
            }
            self.expect_punct("}")?;
            return Ok(Stmt::Block(stmts));
        }
        if self.is_block_end() {
            return Ok(Stmt::Empty);
        }
        let e = self.expression()?;
        Ok(Stmt::Expr(e, pos))
    }

    fn is_block_end(&self) -> bool {
        matches!(self.peek(), Tok::Punct(p) if p == "}")
    }

    /// Parses an expression.
    pub fn expression(&mut self) -> Result<Expr, LangError> {
        self.enter("expression nesting")?;
        let result = self.comparison();
        self.leave();
        result
    }

    fn comparison(&mut self) -> Result<Expr, LangError> {
        let mut left = self.additive()?;
        while let Tok::Punct(p) = self.peek() {
            let op = match p.as_str() {
                "==" => BinOp::Eq,
                "!=" => BinOp::Ne,
                "<" => BinOp::Lt,
                "<=" => BinOp::Le,
                ">" => BinOp::Gt,
                ">=" => BinOp::Ge,
                _ => break,
            };
            self.bump();
            let right = self.additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, LangError> {
        let mut left = self.multiplicative()?;
        while let Tok::Punct(p) = self.peek() {
            let op = match p.as_str() {
                "+" => BinOp::Add,
                "-" => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn multiplicative(&mut self) -> Result<Expr, LangError> {
        let mut left = self.power()?;
        while let Tok::Punct(p) = self.peek() {
            let op = match p.as_str() {
                "*" => BinOp::Mul,
                "/" => BinOp::Div,
                "%" => BinOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.power()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    // Power (right-associative), with a leading sign handled at THIS level so the sign binds
    // looser than '^' (the mathematical convention) but tighter than '..':
    //     -x^2   == -(x^2)       (-2^2 is -4, not 4)
    //     x^-2   == x^(-2)       (the exponent may carry its own sign)
    //     -1..2  == (-1)..2      (a signed range start is not a negation of the range)
    // This is the reference grammar's ParsePower shape: the sign's operand is a ParseUnary, so a
    // sign never reaches past the power unless the power is what follows it.
    fn power(&mut self) -> Result<Expr, LangError> {
        if matches!(self.peek(), Tok::Punct(p) if p == "-" || p == "+") {
            let negative = matches!(self.peek(), Tok::Punct(p) if p == "-");
            self.bump();
            let operand = self.unary()?;
            // "-x^2" negates the whole power; "-1..2" signs only the first range element.
            // A unary plus is the identity: it is consumed at the same grammar position so
            // "+2^2" still binds the power first and "+1..2" still signs only the start.
            let signed = if matches!(self.peek(), Tok::Punct(p) if p == "^") {
                let inner = self.power_tail(operand)?;
                if negative {
                    Expr::Neg(Box::new(inner))
                } else {
                    inner
                }
            } else if negative {
                Expr::Neg(Box::new(operand))
            } else {
                operand
            };
            if matches!(self.peek(), Tok::Punct(p) if p == "..") {
                let ranged = self.range_tail(signed)?;
                return self.power_tail(ranged);
            }
            return Ok(signed);
        }
        let left = self.range()?;
        self.power_tail(left)
    }

    fn power_tail(&mut self, left: Expr) -> Result<Expr, LangError> {
        if !self.eat_punct("^") {
            return Ok(left);
        }
        // Right-associative; the exponent may itself be signed or a power: x^-2, 2^3^2.
        self.enter("power nesting")?;
        let right = self.power();
        self.leave();
        Ok(Expr::Binary(BinOp::Pow, Box::new(left), Box::new(right?)))
    }

    fn range(&mut self) -> Result<Expr, LangError> {
        let start = self.unary()?;
        self.range_tail(start)
    }

    fn range_tail(&mut self, start: Expr) -> Result<Expr, LangError> {
        if !self.eat_punct("..") {
            return Ok(start);
        }
        let second = self.unary()?;
        if self.eat_punct("..") {
            let end = self.unary()?;
            return Ok(Expr::Range(
                Box::new(start),
                Some(Box::new(second)),
                Box::new(end),
            ));
        }
        Ok(Expr::Range(Box::new(start), None, Box::new(second)))
    }

    fn unary(&mut self) -> Result<Expr, LangError> {
        if self.eat_punct("-") {
            self.enter("expression nesting")?;
            let inner = self.unary();
            self.leave();
            return Ok(Expr::Neg(Box::new(inner?)));
        }
        if self.eat_punct("+") {
            // Unary plus is the identity; it is still a consumed prefix operator (one nesting
            // level), exactly as the reference's ParseUnary charges it.
            self.enter("expression nesting")?;
            let inner = self.unary();
            self.leave();
            return inner;
        }
        if let Tok::Ident(k) = self.peek().clone() {
            if k == "not" {
                self.bump();
                self.enter("expression nesting")?;
                let inner = self.unary();
                self.leave();
                return Ok(Expr::Not(Box::new(inner?)));
            }
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, LangError> {
        let mut e = self.primary()?;
        loop {
            if self.eat_punct("(") {
                // The CALL's position is the callee's, not the closing paren's: a diagnostic
                // about a call points where the call starts.
                let (name, pos) = match &e {
                    Expr::Var(n, p) => (n.clone(), *p),
                    _ => return Err(self.err("Only a named function can be called.")),
                };
                let mut args = Vec::new();
                if !self.eat_punct(")") {
                    loop {
                        args.push(self.expression()?);
                        if self.eat_punct(",") {
                            continue;
                        }
                        self.expect_punct(")")?;
                        break;
                    }
                }
                e = Expr::Call(name, args, pos);
                continue;
            }
            if self.eat_punct("[") {
                let mut args = Vec::new();
                loop {
                    // One axis: a scalar, or a slice start:stop:step with every part optional.
                    let mut start = None;
                    if !self.eat_punct(":") {
                        start = Some(self.expression()?);
                        if !self.eat_punct(":") {
                            args.push(IndexArg::Single(start.take().unwrap()));
                            if self.eat_punct(",") {
                                continue;
                            }
                            self.expect_punct("]")?;
                            break;
                        }
                    }
                    let stop = if matches!(self.peek(), Tok::Punct(p) if p == ":" || p == "," || p == "]")
                    {
                        None
                    } else {
                        Some(self.expression()?)
                    };
                    let step = if self.eat_punct(":") {
                        if matches!(self.peek(), Tok::Punct(p) if p == "," || p == "]") {
                            None
                        } else {
                            Some(self.expression()?)
                        }
                    } else {
                        None
                    };
                    // The bare colon(s) mean the whole axis; any bound or step is a slice.
                    if start.is_none() && stop.is_none() && step.is_none() {
                        args.push(IndexArg::Full);
                    } else {
                        args.push(IndexArg::Slice(start, stop, step));
                    }
                    if self.eat_punct(",") {
                        continue;
                    }
                    self.expect_punct("]")?;
                    break;
                }
                e = Expr::Index(Box::new(e), args);
                continue;
            }
            if self.eat_punct("!") {
                e = Expr::Fact(Box::new(e));
                continue;
            }
            if self.eat_punct(".") {
                match self.peek().clone() {
                    Tok::Ident(field) => {
                        self.bump();
                        e = Expr::Member(Box::new(e), field);
                        continue;
                    }
                    _ => {
                        return Err(self.err(&format!(
                            "Expected 'Ident' but found '{}' at position {}.",
                            token_text(self.peek()),
                            self.pos()
                        )));
                    }
                }
            }
            break;
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, LangError> {
        let pos = self.pos();
        match self.bump() {
            Tok::Num(t) => Ok(Expr::Number(t)),
            Tok::Repr(t) => Ok(Expr::Repeating(t)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Interp(raw) => self.interpolated(&raw, pos),
            Tok::Ident(name) => match name.as_str() {
                "true" => Ok(Expr::Number("1".to_string())),
                "false" => Ok(Expr::Number("0".to_string())),
                _ => Ok(Expr::Var(name, pos)),
            },
            Tok::Punct(p) if p == "(" => {
                let e = self.expression()?;
                self.expect_punct(")")?;
                Ok(e)
            }
            Tok::Punct(p) if p == "[" => {
                let mut items = Vec::new();
                if !self.eat_punct("]") {
                    loop {
                        items.push(self.expression()?);
                        if self.eat_punct(",") {
                            if self.eat_punct("]") {
                                break;
                            }
                            continue;
                        }
                        self.expect_punct("]")?;
                        break;
                    }
                }
                Ok(Expr::List(items))
            }
            other => Err(LangError::plain(format!(
                "Unexpected token '{}' at position {}: expected a number, string, identifier, '[', or '('.",
                token_text(&other),
                pos
            ))
            .with_position(pos)),
        }
    }

    fn interpolated(&mut self, raw: &str, pos: usize) -> Result<Expr, LangError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    text.push('{');
                    continue;
                }
                if !text.is_empty() {
                    parts.push(InterpPart::Text(std::mem::take(&mut text)));
                }
                let mut inner = String::new();
                let mut depth = 1;
                for c2 in chars.by_ref() {
                    if c2 == '{' {
                        depth += 1;
                    }
                    if c2 == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    inner.push(c2);
                }
                let toks = tokenize(&inner)?;
                let mut p = Parser::new(toks);
                let e = p.single_expression()?;
                parts.push(InterpPart::Expr(e));
                continue;
            }
            if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                text.push('}');
                continue;
            }
            text.push(c);
        }
        if !text.is_empty() {
            parts.push(InterpPart::Text(text));
        }
        let _ = pos;
        Ok(Expr::Interp(parts))
    }
}
