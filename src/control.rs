// SPDX-License-Identifier: MPL-2.0
//! The CONTROL sublanguage: Turing-complete and imperative.
//!
//! Grammar (see `GRAMMAR.ebnf`):
//!
//! ```text
//! control_program = { control_stmt }
//! control_stmt    = assignment | if_stmt | while_stmt | print_stmt | block
//! block           = "begin" { control_stmt } "end"
//! assignment      = variable "=" data_expr        <- THE BRIDGE
//! if_stmt         = "if" boolean_expr "then" control_stmt [ "else" control_stmt ]
//! while_stmt      = "while" boolean_expr "do" control_stmt
//! print_stmt      = "print" "(" data_expr { "," data_expr } ")"
//! ```
//!
//! Every DATA position above is parsed by [`crate::data::parse_data_tokens`],
//! a separate parser producing the separate [`DataExpr`] type. This module has
//! no rule that builds a `DataExpr` itself; it only stores the one DATA hands
//! back. CONTROL nodes may *hold* `DataExpr` values (that is the bridge);
//! `DataExpr` has no field that could hold a [`ControlStmt`].
//!
//! [`eval_control`] is NOT guaranteed to terminate: `while` is unrestricted.
//! What JtV offers is that each DATA condition, assignment and print argument
//! evaluated along the way returns. See README, Limitations.

use std::fmt;
use std::io::Write;

use crate::data::{eval_data, parse_data_tokens, DataExpr, DataParseError, DataValue, Env};
use crate::lexer::{tokenize, LexError, Token, TokenKind};

#[derive(Clone, Debug, PartialEq)]
pub enum ControlStmt {
    Assignment(String, DataExpr),
    If(DataExpr, Box<ControlStmt>, Option<Box<ControlStmt>>),
    While(DataExpr, Box<ControlStmt>),
    Print(Vec<DataExpr>),
    Block(Vec<ControlStmt>),
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for ControlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CONTROL parse error: {} at position {}",
            self.message, self.pos
        )
    }
}

impl std::error::Error for ControlParseError {}

/// Every way parsing a CONTROL program can fail. `Data` means the failure
/// happened inside a DATA position; that is where diode violations land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Lex(LexError),
    Data(DataParseError),
    Control(ControlParseError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Lex(e) => e.fmt(f),
            ParseError::Data(e) => e.fmt(f),
            ParseError::Control(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<DataParseError> for ParseError {
    fn from(e: DataParseError) -> Self {
        ParseError::Data(e)
    }
}

impl From<ControlParseError> for ParseError {
    fn from(e: ControlParseError) -> Self {
        ParseError::Control(e)
    }
}

struct ControlParser<'t> {
    tokens: &'t [Token],
    pos: usize,
}

impl<'t> ControlParser<'t> {
    fn peek(&self) -> &'t Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &'t Token {
        let tok = &self.tokens[self.pos];
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&'t Token, ControlParseError> {
        let tok = self.peek();
        if tok.kind != kind {
            return Err(ControlParseError {
                message: format!(
                    "expected {kind:?} but found {:?} ({:?})",
                    tok.kind, tok.text
                ),
                pos: tok.pos,
            });
        }
        Ok(self.advance())
    }

    /// Hand the token stream to the DATA parser. The only way CONTROL
    /// obtains a `DataExpr`.
    fn data(&mut self) -> Result<DataExpr, DataParseError> {
        let (expr, next) = parse_data_tokens(self.tokens, self.pos)?;
        self.pos = next;
        Ok(expr)
    }

    fn program(&mut self) -> Result<ControlStmt, ParseError> {
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            stmts.push(self.stmt()?);
        }
        Ok(ControlStmt::Block(stmts))
    }

    fn stmt(&mut self) -> Result<ControlStmt, ParseError> {
        let tok = self.peek();
        match tok.kind {
            TokenKind::Begin => self.block(),
            TokenKind::If => {
                self.advance();
                let cond = self.data()?;
                self.expect(TokenKind::Then)?;
                let then_branch = self.stmt()?;
                let else_branch = if self.peek().kind == TokenKind::Else {
                    self.advance();
                    Some(Box::new(self.stmt()?))
                } else {
                    None
                };
                Ok(ControlStmt::If(cond, Box::new(then_branch), else_branch))
            }
            TokenKind::While => {
                self.advance();
                let cond = self.data()?;
                self.expect(TokenKind::Do)?;
                let body = self.stmt()?;
                Ok(ControlStmt::While(cond, Box::new(body)))
            }
            TokenKind::Print => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let mut args = vec![self.data()?];
                while self.peek().kind == TokenKind::Comma {
                    self.advance();
                    args.push(self.data()?);
                }
                self.expect(TokenKind::RParen)?;
                Ok(ControlStmt::Print(args))
            }
            TokenKind::Ident => {
                let var = self.advance().text.clone();
                self.expect(TokenKind::Assign)?;
                let expr = self.data()?;
                Ok(ControlStmt::Assignment(var, expr))
            }
            TokenKind::Eof => Err(ControlParseError {
                message: "expected a statement but found end of input".to_string(),
                pos: tok.pos,
            }
            .into()),
            k => Err(ControlParseError {
                message: format!("unexpected token {k:?} ({:?})", tok.text),
                pos: tok.pos,
            }
            .into()),
        }
    }

    fn block(&mut self) -> Result<ControlStmt, ParseError> {
        self.expect(TokenKind::Begin)?;
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::End {
            if self.peek().kind == TokenKind::Eof {
                return Err(ControlParseError {
                    message: "unterminated block: expected 'end', found end of input".to_string(),
                    pos: self.peek().pos,
                }
                .into());
            }
            stmts.push(self.stmt()?);
        }
        self.expect(TokenKind::End)?;
        Ok(ControlStmt::Block(stmts))
    }
}

/// Parse a whole CONTROL program. The result is always a `Block`.
pub fn parse_control_program(source: &str) -> Result<ControlStmt, ParseError> {
    let tokens = tokenize(source).map_err(ParseError::Lex)?;
    ControlParser {
        tokens: &tokens,
        pos: 0,
    }
    .program()
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

fn eval_cond(cond: &DataExpr, env: &Env, what: &str) -> Result<bool, String> {
    match eval_data(cond, env)? {
        DataValue::Boolean(b) => Ok(b),
        DataValue::Number(_) => Err(format!("{what} condition must be boolean")),
    }
}

/// Run a CONTROL statement, writing `print` output to `out`. May not
/// terminate (unrestricted `while`).
pub fn eval_control(stmt: &ControlStmt, env: &mut Env, out: &mut dyn Write) -> Result<(), String> {
    match stmt {
        ControlStmt::Assignment(var, expr) => {
            let val = eval_data(expr, env)?;
            env.insert(var.clone(), val);
        }
        ControlStmt::If(cond, then_branch, else_branch) => {
            if eval_cond(cond, env, "if")? {
                eval_control(then_branch, env, out)?;
            } else if let Some(elb) = else_branch {
                eval_control(elb, env, out)?;
            }
        }
        ControlStmt::While(cond, body) => {
            while eval_cond(cond, env, "while")? {
                eval_control(body, env, out)?;
            }
        }
        ControlStmt::Print(args) => {
            let vals = args
                .iter()
                .map(|a| eval_data(a, env).map(|v| v.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            writeln!(out, "{}", vals.join(", ")).map_err(|e| format!("write error: {e}"))?;
        }
        ControlStmt::Block(stmts) => {
            for s in stmts {
                eval_control(s, env, out)?;
            }
        }
    }
    Ok(())
}
