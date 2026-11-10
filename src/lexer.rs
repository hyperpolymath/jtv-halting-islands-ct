// SPDX-License-Identifier: MPL-2.0
//! Shared tokenizer for JtV source text.
//!
//! Lexing is character-level infrastructure common to both sublanguages. It
//! is NOT where the diode lives. The diode lives in the two separate parsers
//! (`data.rs`, `control.rs`) and in the two separate AST types (`DataExpr`,
//! `ControlStmt`). The lexer only turns text into a flat token stream and
//! grants no grammatical power to either side.
//!
//! Ported from the retired Python PoC (`src/jtv/lexer.py`, removed in
//! `abe769d`). One deliberate difference: the Python version turned `N/M`
//! into an exact rational. The Rust implementation's DATA numbers are `i64`,
//! so `N/M` is lexed as [`TokenKind::Rational`] only so that the DATA parser
//! can reject it with a clear message instead of a confusing one.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Number,
    /// `N/M` literal. Recognised only to be rejected (see module docs).
    Rational,
    Ident,
    /// `plus`, `added_to`, `with`
    AddOpWord,
    /// `+`
    AddOpSym,
    CmpOp,
    And,
    Or,
    Not,
    True,
    False,
    Begin,
    End,
    If,
    Then,
    Else,
    While,
    Do,
    Print,
    Assign,
    LParen,
    RParen,
    Comma,
    Eof,
}

impl TokenKind {
    /// Tokens that belong only to the CONTROL grammar. The DATA parser has no
    /// production that accepts any of them.
    pub fn is_control_only(self) -> bool {
        matches!(
            self,
            TokenKind::Begin
                | TokenKind::End
                | TokenKind::If
                | TokenKind::Then
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::Do
                | TokenKind::Print
                | TokenKind::Assign
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    /// Byte offset of the token in the source.
    pub pos: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error: {} at position {}", self.message, self.pos)
    }
}

impl std::error::Error for LexError {}

fn keyword(word: &str) -> Option<TokenKind> {
    Some(match word {
        "plus" | "added_to" | "with" => TokenKind::AddOpWord,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "begin" => TokenKind::Begin,
        "end" => TokenKind::End,
        "if" => TokenKind::If,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "print" => TokenKind::Print,
        _ => return None,
    })
}

// Longest symbols first, so `<=` is not lexed as `<` followed by `=`.
const SYMBOLS: &[(&str, TokenKind)] = &[
    ("==", TokenKind::CmpOp),
    ("!=", TokenKind::CmpOp),
    ("<=", TokenKind::CmpOp),
    (">=", TokenKind::CmpOp),
    ("<", TokenKind::CmpOp),
    (">", TokenKind::CmpOp),
    ("+", TokenKind::AddOpSym),
    ("=", TokenKind::Assign),
    ("(", TokenKind::LParen),
    (")", TokenKind::RParen),
    (",", TokenKind::Comma),
];

/// Tokenize `source`. The returned vector always ends with exactly one
/// [`TokenKind::Eof`] token.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let bytes = source.as_bytes();
    let n = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < n {
        let c = bytes[i];

        if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
            i += 1;
            continue;
        }

        if c == b'#' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < n && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let mut kind = TokenKind::Number;
            if i + 1 < n && bytes[i] == b'/' && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < n && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                kind = TokenKind::Rational;
            }
            tokens.push(Token {
                kind,
                text: source[start..i].to_string(),
                pos: start,
            });
            continue;
        }

        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &source[start..i];
            let kind = keyword(word).unwrap_or(TokenKind::Ident);
            tokens.push(Token {
                kind,
                text: word.to_string(),
                pos: start,
            });
            continue;
        }

        if let Some(&(sym, kind)) = SYMBOLS.iter().find(|(sym, _)| source[i..].starts_with(sym)) {
            tokens.push(Token {
                kind,
                text: sym.to_string(),
                pos: i,
            });
            i += sym.len();
            continue;
        }

        let ch = source[i..].chars().next().unwrap_or('?');
        return Err(LexError {
            message: format!("unexpected character {ch:?}"),
            pos: i,
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        pos: n,
    });
    Ok(tokens)
}
