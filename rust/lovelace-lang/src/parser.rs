//! Recursive-descent parser for the Lovelace scripting language.
//!
//! Precedence, lowest to highest: assignment, comparison, additive, multiplicative,
//! power (with a leading sign), range, unary prefix, postfix (factorial/index/member),
//! primary. Ranges bind tighter than arithmetic, so `1..10 ^ 2` is `(1..10) ^ 2`.

use std::rc::Rc;

use crate::ast::{BinOp, Expr, FunctionDef, IndexSpec, InterpPart, Stmt, UnOp};
use crate::token::{tokenize, Token, TokenKind};
use crate::LangError;

/// Parses a token stream into a statement list.
pub fn parse_program(tokens: Vec<Token>) -> Result<Vec<Stmt>, LangError> {
    Parser { tokens, pos: 0 }.program()
}

/// Parses a token stream into a single expression.
pub fn parse_expression(tokens: Vec<Token>) -> Result<Expr, LangError> {
    let mut parser = Parser { tokens, pos: 0 };
    if parser.kind() == TokenKind::Eof {
        return Err(parser.error("Unexpected end of input: expected an expression."));
    }
    let expr = parser.parse_assignment()?;
    if parser.kind() != TokenKind::Eof {
        return Err(parser.error(format!(
            "Unexpected token '{}' at position {}.",
            parser.current().text,
            parser.current().position
        )));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn kind(&self) -> TokenKind {
        self.tokens[self.pos].kind
    }

    fn peek_kind(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map_or(TokenKind::Eof, |t| t.kind)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn error(&self, message: impl Into<String>) -> LangError {
        LangError::at(message.into(), self.current().position)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, LangError> {
        if self.kind() != kind {
            return Err(self.error(format!(
                "Expected '{kind}' but found '{}' at position {}.",
                self.current().text,
                self.current().position
            )));
        }
        Ok(self.advance())
    }

    fn match_ident(&self, text: &str) -> bool {
        self.kind() == TokenKind::Ident && self.current().text == text
    }

    fn program(mut self) -> Result<Vec<Stmt>, LangError> {
        let mut statements = Vec::new();
        while self.kind() != TokenKind::Eof {
            statements.push(self.parse_statement()?);
            if self.kind() == TokenKind::Semicolon {
                self.advance();
            } else if self.kind() != TokenKind::Eof && self.kind() != TokenKind::RBrace {
                return Err(self.error(format!(
                    "Expected ';' or end of input but found '{}' at position {}.",
                    self.current().text,
                    self.current().position
                )));
            }
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Stmt, LangError> {
        if self.kind() == TokenKind::LBrace {
            return self.parse_block();
        }
        if self.kind() == TokenKind::Ident {
            match self.current().text.as_str() {
                "func" => return self.parse_function(),
                "if" => return self.parse_if(),
                "while" => return self.parse_while(),
                "for" => return self.parse_for(),
                "return" => return self.parse_return(),
                "break" => {
                    self.advance();
                    return Ok(Stmt::Break);
                }
                "continue" => {
                    self.advance();
                    return Ok(Stmt::Continue);
                }
                _ => {}
            }
        }
        Ok(Stmt::Expr(self.parse_assignment()?))
    }

    fn parse_block(&mut self) -> Result<Stmt, LangError> {
        self.expect(TokenKind::LBrace)?;
        let mut statements = Vec::new();
        while self.kind() != TokenKind::RBrace && self.kind() != TokenKind::Eof {
            statements.push(self.parse_statement()?);
            if self.kind() == TokenKind::Semicolon {
                self.advance();
            } else if self.kind() != TokenKind::RBrace {
                return Err(self.error(format!(
                    "Expected ';' or '}}' but found '{}' at position {}.",
                    self.current().text,
                    self.current().position
                )));
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::Block(statements))
    }

    fn parse_function(&mut self) -> Result<Stmt, LangError> {
        self.advance(); // func
        if self.kind() != TokenKind::Ident {
            return Err(self.error(format!(
                "Expected function name but found '{}' at position {}.",
                self.current().text,
                self.current().position
            )));
        }
        let name = self.advance().text;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.kind() != TokenKind::RParen {
            params.push(self.expect(TokenKind::Ident)?.text);
            while self.kind() == TokenKind::Comma {
                self.advance();
                params.push(self.expect(TokenKind::Ident)?.text);
            }
        }
        self.expect(TokenKind::RParen)?;

        let body = if self.kind() == TokenKind::LBrace {
            match self.parse_block()? {
                Stmt::Block(statements) => statements,
                _ => unreachable!(),
            }
        } else if self.kind() == TokenKind::Equals {
            self.advance();
            vec![Stmt::Expr(self.parse_assignment()?)]
        } else {
            return Err(self.error(format!(
                "Expected '{{' or '=' after function signature but found '{}' at position {}.",
                self.current().text,
                self.current().position
            )));
        };

        Ok(Stmt::Func(Rc::new(FunctionDef { name, params, body })))
    }

    fn parse_if(&mut self) -> Result<Stmt, LangError> {
        self.advance(); // if
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_assignment()?;
        self.expect(TokenKind::RParen)?;
        let then_branch = Box::new(self.parse_statement()?);
        let else_branch = if self.match_ident("else") {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        Ok(Stmt::If { condition, then_branch, else_branch })
    }

    fn parse_while(&mut self) -> Result<Stmt, LangError> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_assignment()?;
        self.expect(TokenKind::RParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, LangError> {
        self.advance();
        let variable = self.expect(TokenKind::Ident)?.text;
        if !self.match_ident("in") {
            return Err(self.error(format!(
                "Expected 'in' after loop variable but found '{}' at position {}.",
                self.current().text,
                self.current().position
            )));
        }
        self.advance();
        let range = self.parse_assignment()?;
        let body = Box::new(self.parse_statement()?);
        Ok(Stmt::For { variable, range, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, LangError> {
        self.advance();
        let value = if matches!(
            self.kind(),
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
        ) {
            None
        } else {
            Some(self.parse_assignment()?)
        };
        Ok(Stmt::Return(value))
    }

    fn parse_assignment(&mut self) -> Result<Expr, LangError> {
        if self.kind() == TokenKind::Ident && self.peek_kind(1) == TokenKind::Equals {
            let name = self.advance().text;
            self.advance(); // =
            let value = self.parse_assignment()?;
            return Ok(Expr::Assign(name, Box::new(value)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, LangError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.kind() {
                TokenKind::DoubleEquals => BinOp::Equal,
                TokenKind::BangEquals => BinOp::NotEqual,
                TokenKind::Greater => BinOp::Greater,
                TokenKind::Less => BinOp::Less,
                TokenKind::GreaterEquals => BinOp::GreaterEqual,
                TokenKind::LessEquals => BinOp::LessEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, LangError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, LangError> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinOp::Multiply,
                TokenKind::Slash => BinOp::Divide,
                TokenKind::Percent => BinOp::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, LangError> {
        if matches!(self.kind(), TokenKind::Minus | TokenKind::Plus) {
            let op = if self.kind() == TokenKind::Minus { UnOp::Negate } else { UnOp::Plus };
            self.advance();
            let operand = self.parse_unary()?;
            let signed = if self.kind() == TokenKind::Caret {
                Expr::Unary(op, Box::new(self.parse_power_tail(operand)?))
            } else {
                Expr::Unary(op, Box::new(operand))
            };
            if self.kind() == TokenKind::DotDot {
                let ranged = self.parse_range_tail(signed)?;
                return self.parse_power_tail(ranged);
            }
            return Ok(signed);
        }
        let range = self.parse_range()?;
        self.parse_power_tail(range)
    }

    fn parse_power_tail(&mut self, left: Expr) -> Result<Expr, LangError> {
        if self.kind() != TokenKind::Caret {
            return Ok(left);
        }
        self.advance();
        let right = self.parse_power()?;
        Ok(Expr::Binary(BinOp::Power, Box::new(left), Box::new(right)))
    }

    fn parse_range(&mut self) -> Result<Expr, LangError> {
        let first = self.parse_unary()?;
        self.parse_range_tail(first)
    }

    fn parse_range_tail(&mut self, first: Expr) -> Result<Expr, LangError> {
        if self.kind() != TokenKind::DotDot {
            return Ok(first);
        }
        self.advance();
        let second = self.parse_unary()?;
        if self.kind() == TokenKind::DotDot {
            self.advance();
            let third = self.parse_unary()?;
            Ok(Expr::Range {
                start: Box::new(first),
                step: Some(Box::new(second)),
                end: Box::new(third),
            })
        } else {
            Ok(Expr::Range { start: Box::new(first), step: None, end: Box::new(second) })
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, LangError> {
        if self.kind() == TokenKind::Minus {
            self.advance();
            return Ok(Expr::Unary(UnOp::Negate, Box::new(self.parse_unary()?)));
        }
        if self.kind() == TokenKind::Plus {
            self.advance();
            return Ok(Expr::Unary(UnOp::Plus, Box::new(self.parse_unary()?)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, LangError> {
        let mut operand = self.parse_primary()?;
        loop {
            if self.kind() == TokenKind::Bang {
                self.advance();
                operand = Expr::Factorial(Box::new(operand));
                continue;
            }
            if self.kind() == TokenKind::LBracket {
                self.advance();
                let mut specs = Vec::new();
                if self.kind() == TokenKind::RBracket {
                    return Err(self.error("Empty index list."));
                }
                specs.push(self.parse_index_spec()?);
                while self.kind() == TokenKind::Comma {
                    self.advance();
                    specs.push(self.parse_index_spec()?);
                }
                self.expect(TokenKind::RBracket)?;
                operand = Expr::Index(Box::new(operand), specs);
                continue;
            }
            if self.kind() == TokenKind::Dot {
                self.advance();
                let member = self.expect(TokenKind::Ident)?.text;
                operand = Expr::Member(Box::new(operand), member);
                continue;
            }
            break;
        }
        Ok(operand)
    }

    fn parse_index_spec(&mut self) -> Result<IndexSpec, LangError> {
        let start = if self.kind() == TokenKind::Colon {
            None
        } else {
            Some(self.parse_assignment()?)
        };
        if self.kind() != TokenKind::Colon {
            return match start {
                Some(expr) => Ok(IndexSpec::Scalar(expr)),
                None => Err(self.error("Expected an index or slice.")),
            };
        }
        self.advance(); // :
        let stop = if matches!(
            self.kind(),
            TokenKind::Colon | TokenKind::Comma | TokenKind::RBracket
        ) {
            None
        } else {
            Some(self.parse_assignment()?)
        };
        let step = if self.kind() == TokenKind::Colon {
            self.advance();
            if matches!(self.kind(), TokenKind::Comma | TokenKind::RBracket) {
                None
            } else {
                Some(self.parse_assignment()?)
            }
        } else {
            None
        };
        Ok(IndexSpec::Slice { start, stop, step })
    }

    fn parse_primary(&mut self) -> Result<Expr, LangError> {
        match self.kind() {
            TokenKind::Number => Ok(Expr::Literal(self.advance().text)),
            TokenKind::StringLiteral => Ok(Expr::Text(self.advance().text)),
            TokenKind::InterpolatedString => {
                let raw = self.advance().text;
                self.parse_interpolated(&raw)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if self.kind() != TokenKind::RBracket {
                    elements.push(self.parse_assignment()?);
                    while self.kind() == TokenKind::Comma {
                        self.advance();
                        elements.push(self.parse_assignment()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::List(elements))
            }
            TokenKind::Ident => {
                let name = self.advance().text;
                if self.kind() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.kind() != TokenKind::RParen {
                        args.push(self.parse_assignment()?);
                        while self.kind() == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_assignment()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_assignment()?;
                self.expect(TokenKind::RParen)?;
                Ok(inner)
            }
            _ => Err(self.error(format!(
                "Unexpected token '{}' at position {}: expected a number, string, identifier, '[', or '('.",
                self.current().text,
                self.current().position
            ))),
        }
    }

    fn parse_interpolated(&mut self, raw: &str) -> Result<Expr, LangError> {
        let chars: Vec<char> = raw.chars().collect();
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
                text.push('{');
                i += 2;
                continue;
            }
            if c == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                text.push('}');
                i += 2;
                continue;
            }
            if c == '{' {
                let mut close = None;
                for (offset, ch) in chars.iter().enumerate().skip(i + 1) {
                    if *ch == '}' {
                        close = Some(offset);
                        break;
                    }
                }
                let Some(close) = close else {
                    return Err(self.error("Unterminated interpolation expression in string."));
                };
                if !text.is_empty() {
                    parts.push(InterpPart::Text(std::mem::take(&mut text)));
                }
                let expression: String = chars[i + 1..close].iter().collect();
                let tokens = tokenize(&expression).map_err(|message| self.error(message))?;
                let expr = parse_expression(tokens)?;
                parts.push(InterpPart::Expr(expr));
                i = close + 1;
                continue;
            }
            text.push(c);
            i += 1;
        }
        if !text.is_empty() {
            parts.push(InterpPart::Text(text));
        }
        Ok(Expr::Interpolated(parts))
    }
}
