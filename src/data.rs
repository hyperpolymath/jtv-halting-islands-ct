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

/// Deepest DATA AST the parser will build. Parsing and evaluation both
/// recurse on the AST, so source text that would produce a taller tree (for
/// example a very long `0 + 0 + ...` chain, which builds a left-deep tree)
/// is rejected with a [`DataParseError`] instead of risking a native stack
/// overflow.
pub const MAX_DATA_DEPTH: usize = 1024;

/// Deepest parenthesis nesting the DATA parser accepts. Parentheses add no
/// AST nodes, so [`MAX_DATA_DEPTH`] alone would not bound the parser's own
/// recursion on input such as `((((...))))`.
pub const MAX_DATA_NESTING: usize = 128;

struct DataParser<'t> {
    tokens: &'t [Token],
    pos: usize,
    /// Current parenthesis nesting.
    nesting: usize,
}

/// A parsed subexpression together with its AST height.
type Parsed = (DataExpr, usize);

/// Height of a node whose tallest child has height `child`, or an error at
/// `at` if that exceeds [`MAX_DATA_DEPTH`].
fn checked_height(child: usize, at: &Token) -> Result<usize, DataParseError> {
    let height = child + 1;
    if height > MAX_DATA_DEPTH {
        return Err(DataParseError {
            message: format!(
                "expression too deep: the DATA parser builds ASTs of height at most \
                 {MAX_DATA_DEPTH}"
            ),
            pos: at.pos,
        });
    }
    Ok(height)
}

impl<'t> DataParser<'t> {
    fn peek(&self) -> &'t Token {
        // `parse_data_tokens` checks that the slice ends with Eof and that
        // `start` is in range; `advance` never moves past an Eof, so this
        // index is always in bounds.
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &'t Token {
        let tok = &self.tokens[self.pos];
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn boolean_expr(&mut self) -> Result<Parsed, DataParseError> {
        let (mut left, mut height) = self.boolean_term()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::And => BoolOp::And,
                TokenKind::Or => BoolOp::Or,
                _ => return Ok((left, height)),
            };
            let op_tok = self.advance();
            let (right, right_height) = self.boolean_term()?;
            height = checked_height(height.max(right_height), op_tok)?;
            left = DataExpr::BoolOp(op, Box::new(left), Box::new(right));
        }
    }

    fn boolean_term(&mut self) -> Result<Parsed, DataParseError> {
        if self.peek().kind == TokenKind::Not {
            let not_tok = self.advance();
            let (atom, height) = self.boolean_atom()?;
            return Ok((
                DataExpr::Not(Box::new(atom)),
                checked_height(height, not_tok)?,
            ));
        }
        self.boolean_atom()
    }

    fn boolean_atom(&mut self) -> Result<Parsed, DataParseError> {
        match self.peek().kind {
            TokenKind::True => {
                self.advance();
                Ok((DataExpr::Boolean(true), 1))
            }
            TokenKind::False => {
                self.advance();
                Ok((DataExpr::Boolean(false), 1))
            }
            TokenKind::LParen => {
                let open = self.advance();
                if self.nesting >= MAX_DATA_NESTING {
                    return Err(DataParseError {
                        message: format!(
                            "parentheses nested too deeply: the DATA parser accepts at most \
                             {MAX_DATA_NESTING} levels"
                        ),
                        pos: open.pos,
                    });
                }
                self.nesting += 1;
                let inner = self.boolean_expr()?;
                self.nesting -= 1;
                let close = self.peek();
                if close.kind != TokenKind::RParen {
                    return Err(unexpected(close));
                }
                self.advance();
                Ok(inner)
            }
            _ => {
                let (left, left_height) = self.additive_expr()?;
                if self.peek().kind != TokenKind::CmpOp {
                    return Ok((left, left_height));
                }
                let op_tok = self.advance();
                let op = match op_tok.text.as_str() {
                    "==" => CompOp::Eq,
                    "!=" => CompOp::Neq,
                    "<" => CompOp::Lt,
                    ">" => CompOp::Gt,
                    "<=" => CompOp::Lte,
                    ">=" => CompOp::Gte,
                    // `tokenize` never produces any other CmpOp text, but
                    // `Token` is public, so a hand-built stream can.
                    _ => return Err(unexpected(op_tok)),
                };
                let (right, right_height) = self.additive_expr()?;
                let height = checked_height(left_height.max(right_height), op_tok)?;
                Ok((
                    DataExpr::Comparison(op, Box::new(left), Box::new(right)),
                    height,
                ))
            }
        }
    }

    fn additive_expr(&mut self) -> Result<Parsed, DataParseError> {
        let (mut left, mut height) = self.term()?;
        while matches!(self.peek().kind, TokenKind::AddOpSym | TokenKind::AddOpWord) {
            let op_tok = self.advance();
            let (right, right_height) = self.term()?;
            height = checked_height(height.max(right_height), op_tok)?;
            left = DataExpr::Add(Box::new(left), Box::new(right));
        }
        Ok((left, height))
    }

    fn term(&mut self) -> Result<Parsed, DataParseError> {
        let tok = self.peek();
        match tok.kind {
            TokenKind::Number => {
                let n = tok.text.parse::<i64>().map_err(|_| DataParseError {
                    message: format!("number literal {:?} does not fit in i64", tok.text),
                    pos: tok.pos,
                })?;
                self.advance();
                Ok((DataExpr::Number(n), 1))
            }
            TokenKind::Ident => {
                self.advance();
                Ok((DataExpr::VarRef(tok.text.clone()), 1))
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
///
/// `tokens` must end with a [`TokenKind::Eof`] token (as [`tokenize`]'s
/// output does) and `start` must index into it; otherwise this returns a
/// [`DataParseError`]. Expressions taller than [`MAX_DATA_DEPTH`] or with
/// parentheses nested deeper than [`MAX_DATA_NESTING`] are also rejected.
pub fn parse_data_tokens(
    tokens: &[Token],
    start: usize,
) -> Result<(DataExpr, usize), DataParseError> {
    match tokens.last() {
        Some(last) if last.kind == TokenKind::Eof && start < tokens.len() => {}
        last => {
            return Err(DataParseError {
                message: "token stream must end with Eof and the start index must be \
                          inside it"
                    .to_string(),
                pos: last.map_or(0, |t| t.pos),
            })
        }
    }
    let mut p = DataParser {
        tokens,
        pos: start,
        nesting: 0,
    };
    let (expr, _height) = p.boolean_expr()?;
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
/// stack. The parser never builds an AST taller than [`MAX_DATA_DEPTH`], so
/// this only applies to a `DataExpr` built by hand.
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
