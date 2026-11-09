// SPDX-License-Identifier: MPL-2.0
//! The DATA sublanguage: its own AST, its own parser, its own evaluator.
//!
//! DATA is addition-only: no recursion, no loops, no conditionals. Numbers
//! are `i64`; addition is checked, so overflow is an evaluation error, not a
//! panic or a silent wrap.
//!
//! **The parser half of the diode.** The parser below has no production that
//! accepts `begin`, `end`, `if`, `then`, `else`, `while`, `do`, `print` or
//! `=`. When one of those tokens appears where this parser is trying to read
//! a DATA expression, it falls through to [`DataParseError`]. There is no
//! code path that could turn a control keyword into part of a [`DataExpr`].
//!
//! **The type half of the diode.** [`DataExpr`] has no variant or field that
//! can hold a `ControlStmt`. In the Python PoC this was a run-time validator
//! (`diode.py`, `assert_pure_data`); in Rust it is the type definition
//! itself, so a hand-built AST that puts control inside data does not
//! compile. `src/lib.rs` carries `compile_fail` doctests that check exactly
//! this.
//!
//! Grammar implemented (see `GRAMMAR.ebnf`):
//!
//! ```text
//! data_expr       = boolean_expr
//! boolean_expr    = boolean_term { ("and" | "or") boolean_term }
//! boolean_term    = ["not"] boolean_atom
//! boolean_atom    = boolean_literal | "(" boolean_expr ")" | additive_expr [ comp_op additive_expr ]
//! additive_expr   = term { add_op term }
//! term            = number_literal | data_variable
//! ```

use std::collections::HashMap;
use std::fmt;

use crate::lexer::{tokenize, LexError, Token, TokenKind};

#[derive(Clone, Debug, PartialEq)]
pub enum DataExpr {
    Number(i64),
    Boolean(bool),
    VarRef(String),
    Add(Box<DataExpr>, Box<DataExpr>),
    Comparison(CompOp, Box<DataExpr>, Box<DataExpr>),
    Not(Box<DataExpr>),
    BoolOp(BoolOp, Box<DataExpr>, Box<DataExpr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataValue {
    Number(i64),
    Boolean(bool),
}

impl fmt::Display for DataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataValue::Number(n) => write!(f, "{n}"),
            DataValue::Boolean(b) => write!(f, "{b}"),
        }
    }
}

pub type Env = HashMap<String, DataValue>;

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for DataParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DATA parse error: {} at position {}",
            self.message, self.pos
        )
    }
}

impl std::error::Error for DataParseError {}

fn unexpected(tok: &Token) -> DataParseError {
    let message = match tok.kind {
        k if k.is_control_only() => format!(
            "unexpected token {:?} ({:?}): control-flow keywords are not part of the DATA \
             grammar; the diode blocks control from being smuggled into a data expression",
            k, tok.text
        ),
        TokenKind::Rational => format!(
            "rational literal {:?} is not supported: DATA numbers are i64 in this \
             implementation (see README, 'DATA numbers')",
            tok.text
        ),
        TokenKind::Eof => "unexpected end of input".to_string(),
        k => format!("unexpected token {:?} ({:?})", k, tok.text),
    };
    DataParseError {
        message,
        pos: tok.pos,
    }
}

struct DataParser<'t> {
    tokens: &'t [Token],
    pos: usize,
}

impl<'t> DataParser<'t> {
    fn peek(&self) -> &'t Token {
        // `tokenize` always ends the stream with Eof, and `advance` never
        // moves past it, so this index is always in bounds.
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &'t Token {
        let tok = &self.tokens[self.pos];
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn boolean_expr(&mut self) -> Result<DataExpr, DataParseError> {
        let mut left = self.boolean_term()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::And => BoolOp::And,
                TokenKind::Or => BoolOp::Or,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.boolean_term()?;
            left = DataExpr::BoolOp(op, Box::new(left), Box::new(right));
        }
    }

    fn boolean_term(&mut self) -> Result<DataExpr, DataParseError> {
        if self.peek().kind == TokenKind::Not {
            self.advance();
            let atom = self.boolean_atom()?;
            return Ok(DataExpr::Not(Box::new(atom)));
        }
        self.boolean_atom()
    }

    fn boolean_atom(&mut self) -> Result<DataExpr, DataParseError> {
        match self.peek().kind {
            TokenKind::True => {
                self.advance();
                Ok(DataExpr::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(DataExpr::Boolean(false))
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.boolean_expr()?;
                let close = self.peek();
                if close.kind != TokenKind::RParen {
                    return Err(unexpected(close));
                }
                self.advance();
                Ok(inner)
            }
            _ => {
                let left = self.additive_expr()?;
                if self.peek().kind != TokenKind::CmpOp {
                    return Ok(left);
                }
                let op = match self.advance().text.as_str() {
                    "==" => CompOp::Eq,
                    "!=" => CompOp::Neq,
                    "<" => CompOp::Lt,
                    ">" => CompOp::Gt,
                    "<=" => CompOp::Lte,
                    ">=" => CompOp::Gte,
                    other => unreachable!("lexer produced unknown CmpOp {other:?}"),
                };
                let right = self.additive_expr()?;
                Ok(DataExpr::Comparison(op, Box::new(left), Box::new(right)))
            }
        }
    }

    fn additive_expr(&mut self) -> Result<DataExpr, DataParseError> {
        let mut left = self.term()?;
        while matches!(self.peek().kind, TokenKind::AddOpSym | TokenKind::AddOpWord) {
            self.advance();
            let right = self.term()?;
            left = DataExpr::Add(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<DataExpr, DataParseError> {
        let tok = self.peek();
        match tok.kind {
            TokenKind::Number => {
                let n = tok.text.parse::<i64>().map_err(|_| DataParseError {
                    message: format!("number literal {:?} does not fit in i64", tok.text),
                    pos: tok.pos,
                })?;
                self.advance();
                Ok(DataExpr::Number(n))
            }
            TokenKind::Ident => {
                self.advance();
                Ok(DataExpr::VarRef(tok.text.clone()))
            }
            _ => Err(unexpected(tok)),
        }
    }
}

/// Parse one `data_expr` from `tokens`, starting at `start`. Returns the
/// expression and the index of the first unconsumed token.
///
/// This is the single entry point the CONTROL parser uses for every DATA
/// position (assignment right-hand side, `if`/`while` condition, `print`
/// argument). CONTROL never parses DATA with its own grammar.
pub fn parse_data_tokens(
    tokens: &[Token],
    start: usize,
) -> Result<(DataExpr, usize), DataParseError> {
    let mut p = DataParser { tokens, pos: start };
    let expr = p.boolean_expr()?;
    Ok((expr, p.pos))
}

/// Errors from parsing a whole DATA source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataSourceError {
    Lex(LexError),
    Parse(DataParseError),
}

impl fmt::Display for DataSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSourceError::Lex(e) => e.fmt(f),
            DataSourceError::Parse(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for DataSourceError {}

/// Parse an entire source string as a single DATA expression.
pub fn parse_data_program(source: &str) -> Result<DataExpr, DataSourceError> {
    let tokens = tokenize(source).map_err(DataSourceError::Lex)?;
    let (expr, pos) = parse_data_tokens(&tokens, 0).map_err(DataSourceError::Parse)?;
    if tokens[pos].kind != TokenKind::Eof {
        return Err(DataSourceError::Parse(unexpected(&tokens[pos])));
    }
    Ok(expr)
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// Evaluate a DATA expression.
///
/// Why this halts (informal argument, not a machine-checked proof): every
/// recursive call is on a strictly smaller, owned (`Box`) subtree of a finite
/// tree, and there is no loop. Addition uses `checked_add`, so `i64`
/// overflow returns `Err` instead of panicking. Type mismatches and unbound
/// variables also return `Err`. The bounded exhaustive test in
/// `tests/data_totality.rs` checks that this function returns (never
/// panics) on every AST up to a fixed depth over a fixed atom set.
///
/// Caveat: halting is not resource-boundedness. The recursion depth equals
/// the AST depth, so an extremely deep expression can exhaust the native
/// stack.
pub fn eval_data(expr: &DataExpr, env: &Env) -> Result<DataValue, String> {
    match expr {
        DataExpr::Number(n) => Ok(DataValue::Number(*n)),
        DataExpr::Boolean(b) => Ok(DataValue::Boolean(*b)),
        DataExpr::VarRef(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined data variable {name:?}")),
        DataExpr::Add(left, right) => match (eval_data(left, env)?, eval_data(right, env)?) {
            (DataValue::Number(a), DataValue::Number(b)) => a
                .checked_add(b)
                .map(DataValue::Number)
                .ok_or_else(|| format!("integer overflow: {a} + {b} does not fit in i64")),
            _ => Err("'+' requires numeric operands".to_string()),
        },
        DataExpr::Comparison(op, left, right) => {
            match (eval_data(left, env)?, eval_data(right, env)?) {
                (DataValue::Number(a), DataValue::Number(b)) => Ok(DataValue::Boolean(match op {
                    CompOp::Eq => a == b,
                    CompOp::Neq => a != b,
                    CompOp::Lt => a < b,
                    CompOp::Gt => a > b,
                    CompOp::Lte => a <= b,
                    CompOp::Gte => a >= b,
                })),
                _ => Err("comparison requires numeric operands".to_string()),
            }
        }
        DataExpr::Not(operand) => match eval_data(operand, env)? {
            DataValue::Boolean(b) => Ok(DataValue::Boolean(!b)),
            _ => Err("'not' requires a boolean operand".to_string()),
        },
        DataExpr::BoolOp(op, left, right) => {
            match (eval_data(left, env)?, eval_data(right, env)?) {
                (DataValue::Boolean(a), DataValue::Boolean(b)) => {
                    Ok(DataValue::Boolean(match op {
                        BoolOp::And => a && b,
                        BoolOp::Or => a || b,
                    }))
                }
                _ => Err("'and'/'or' require boolean operands".to_string()),
            }
        }
    }
}
