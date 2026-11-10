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
            out.push(Token { tok: Tok::Semi, pos });
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            let (text, next) = scan_string(&chars, i + 1)?;
            out.push(Token { tok: Tok::Str(text), pos });
            i = next;
            continue;
        }
        if c == '$' && i + 1 < chars.len() && chars[i + 1] == '"' {
            let (text, next) = scan_string(&chars, i + 2)?;
            out.push(Token { tok: Tok::Interp(text), pos });
            i = next;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
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
                    out.push(Token { tok: Tok::Repr(text), pos });
                    continue;
                }
            }
            if i < chars.len() && chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                text.push('.');
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    text.push(chars[i]);
                    i += 1;
                }
            }
            // Repeating form: 0.(3) or 0.1(6)
            if i < chars.len() && chars[i] == '(' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
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
                    out.push(Token { tok: Tok::Repr(text), pos });
                    continue;
                }
            }
            let _ = start;
            out.push(Token { tok: Tok::Num(text), pos });
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut text = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                text.push(chars[i]);
                i += 1;
            }
            out.push(Token { tok: Tok::Ident(text), pos });
            continue;
        }
        // Punctuators, longest first.
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if two == ".." || two == "==" || two == "!=" || two == "<=" || two == ">=" {
            out.push(Token { tok: Tok::Punct(two), pos });
            i += 2;
            continue;
        }
        if c == ';' {
            out.push(Token { tok: Tok::Semi, pos });
            i += 1;
            continue;
        }
        let one = c.to_string();
        if "+-*/%^!=<>()[]{},:.".contains(c) {
            out.push(Token { tok: Tok::Punct(one), pos });
            i += 1;
            continue;
        }
        return Err(LangError { message: format!("Unexpected character '{}'.", c), position: pos });
    }
    out.push(Token { tok: Tok::Eof, pos: unit });
    Ok(out)
}

fn scan_string(chars: &[char], mut i: usize) -> Result<(String, usize), LangError> {
    let mut text = String::new();
    let mut start_pos = i;
    let mut unit = 0usize;
    for c in &chars[..i] {
        unit += c.len_utf16();
    }
    start_pos = unit;
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
    Err(LangError { message: "Unterminated string literal.".to_string(), position: start_pos })
}

/// The parser.
pub struct Parser {
    toks: Vec<Token>,
    i: usize,
}

impl Parser {
    /// Builds a parser over a token stream.
    pub fn new(toks: Vec<Token>) -> Parser {
        Parser { toks, i: 0 }
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
            Err(self.err(&format!("Expected '{}'.", p)))
        }
    }

    fn err(&self, msg: &str) -> LangError {
        LangError { message: msg.to_string(), position: self.pos() }
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
            self.eat_semi();
        }
        Ok(out)
    }

    /// Parses a single expression (used for interpolation).
    pub fn single_expression(&mut self) -> Result<Expr, LangError> {
        let e = self.expression()?;
        Ok(e)
    }

    fn statement(&mut self) -> Result<Stmt, LangError> {
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
                    self.eat_semi();
                    let mut other = None;
                    if let Tok::Ident(k) = self.peek().clone() {
                        if k == "else" {
                            self.bump();
                            self.eat_semi();
                            other = Some(Box::new(self.statement()?));
                        }
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
                    let var = match self.bump() {
                        Tok::Ident(v) => v,
                        _ => return Err(self.err("Expected a loop variable name.")),
                    };
                    match self.bump() {
                        Tok::Ident(k) if k == "in" => {}
                        _ => return Err(self.err("Expected 'in' after the loop variable.")),
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
                    let fname = match self.bump() {
                        Tok::Ident(n) => n,
                        _ => return Err(self.err("Expected a function name.")),
                    };
                    self.expect_punct("(")?;
                    let mut params = Vec::new();
                    if !self.eat_punct(")") {
                        loop {
                            match self.bump() {
                                Tok::Ident(p) => params.push(p),
                                _ => return Err(self.err("Expected a parameter name.")),
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
                    self.eat_semi();
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
                self.eat_semi();
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
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, LangError> {
        let mut left = self.additive()?;
        loop {
            let op = match self.peek() {
                Tok::Punct(p) => match p.as_str() {
                    "==" => BinOp::Eq,
                    "!=" => BinOp::Ne,
                    "<" => BinOp::Lt,
                    "<=" => BinOp::Le,
                    ">" => BinOp::Gt,
                    ">=" => BinOp::Ge,
                    _ => break,
                },
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
        loop {
            let op = match self.peek() {
                Tok::Punct(p) => match p.as_str() {
                    "+" => BinOp::Add,
                    "-" => BinOp::Sub,
                    _ => break,
                },
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
        loop {
            let op = match self.peek() {
                Tok::Punct(p) => match p.as_str() {
                    "*" => BinOp::Mul,
                    "/" => BinOp::Div,
                    "%" => BinOp::Rem,
                    _ => break,
                },
                _ => break,
            };
            self.bump();
            let right = self.power()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn power(&mut self) -> Result<Expr, LangError> {
        let left = self.range()?;
        if self.eat_punct("^") {
            let right = self.power()?;
            return Ok(Expr::Binary(BinOp::Pow, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn range(&mut self) -> Result<Expr, LangError> {
        let start = self.unary()?;
        if !self.eat_punct("..") {
            return Ok(start);
        }
        let second = self.unary()?;
        if self.eat_punct("..") {
            let end = self.unary()?;
            return Ok(Expr::Range(Box::new(start), Some(Box::new(second)), Box::new(end)));
        }
        Ok(Expr::Range(Box::new(start), None, Box::new(second)))
    }

    fn unary(&mut self) -> Result<Expr, LangError> {
        if self.eat_punct("-") {
            let inner = self.unary()?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        if let Tok::Ident(k) = self.peek().clone() {
            if k == "not" {
                self.bump();
                let inner = self.unary()?;
                return Ok(Expr::Not(Box::new(inner)));
            }
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, LangError> {
        let mut e = self.primary()?;
        loop {
            if self.eat_punct("(") {
                let name = match &e {
                    Expr::Var(n, _) => n.clone(),
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
                let pos = self.pos();
                e = Expr::Call(name, args, pos);
                continue;
            }
            if self.eat_punct("[") {
                let mut args = Vec::new();
                loop {
                    if self.eat_punct(":") {
                        // A leading colon is the whole axis unless a bound follows.
                        if matches!(self.peek(), Tok::Punct(p) if p == "," || p == "]") {
                            args.push(IndexArg::Full);
                        } else {
                            let b = self.expression()?;
                            args.push(IndexArg::Slice(None, Some(b), None));
                        }
                    } else {
                        let first = self.expression()?;
                        if self.eat_punct(":") {
                            if matches!(self.peek(), Tok::Punct(p) if p == "," || p == "]") {
                                args.push(IndexArg::Slice(Some(first), None, None));
                            } else {
                                let b = self.expression()?;
                                args.push(IndexArg::Slice(Some(first), Some(b), None));
                            }
                        } else {
                            args.push(IndexArg::Single(first));
                        }
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
                match self.bump() {
                    Tok::Ident(field) => {
                        e = Expr::Member(Box::new(e), field);
                        continue;
                    }
                    _ => return Err(self.err("Expected a field name after '.'.")),
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
            other => Err(LangError {
                message: format!("Unexpected token {:?}.", other),
                position: pos,
            }),
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
