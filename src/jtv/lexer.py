"""Shared tokenizer for JtV source text.

Lexing is character-level infrastructure common to both sublanguages — it
is NOT where the diode lives. The diode lives in the *grammars* (the two
separate parsers below) and in the *AST types* (data_ast.py vs
control_ast.py), which is where control could otherwise be smuggled into
data. The lexer just turns text into a flat token stream; it grants no
grammatical power to either side.
"""

from __future__ import annotations

from dataclasses import dataclass
from fractions import Fraction

# Reserved words. Anything not in this table lexes as IDENT (a variable
# name usable by either sublanguage's `variable` / `data_variable` rule).
_KEYWORDS = {
    "plus": "ADDOP_WORD",
    "added_to": "ADDOP_WORD",
    "with": "ADDOP_WORD",
    "and": "AND",
    "or": "OR",
    "not": "NOT",
    "true": "TRUE",
    "false": "FALSE",
    "begin": "BEGIN",
    "end": "END",
    "if": "IF",
    "then": "THEN",
    "else": "ELSE",
    "while": "WHILE",
    "do": "DO",
    "print": "PRINT",
}

_SYMBOLS = [
    ("==", "CMPOP"),
    ("!=", "CMPOP"),
    ("<=", "CMPOP"),
    (">=", "CMPOP"),
    ("<", "CMPOP"),
    (">", "CMPOP"),
    ("+", "ADDOP_SYM"),
    ("=", "ASSIGN"),
    ("(", "LPAREN"),
    (")", "RPAREN"),
    (",", "COMMA"),
]


class LexError(Exception):
    pass


@dataclass(frozen=True)
class Token:
    kind: str
    text: str
    pos: int

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"Token({self.kind!r}, {self.text!r}@{self.pos})"


def tokenize(source: str) -> list[Token]:
    tokens: list[Token] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]

        if c in " \t\r\n":
            i += 1
            continue

        if c == "#":
            while i < n and source[i] != "\n":
                i += 1
            continue

        if c.isdigit():
            start = i
            while i < n and source[i].isdigit():
                i += 1
            if i < n and source[i] == "/" and i + 1 < n and source[i + 1].isdigit():
                i += 1
                while i < n and source[i].isdigit():
                    i += 1
            tokens.append(Token("NUMBER", source[start:i], start))
            continue

        if c.isalpha() or c == "_":
            start = i
            while i < n and (source[i].isalnum() or source[i] == "_"):
                i += 1
            word = source[start:i]
            kind = _KEYWORDS.get(word, "IDENT")
            tokens.append(Token(kind, word, start))
            continue

        matched = False
        for sym, kind in _SYMBOLS:
            if source.startswith(sym, i):
                tokens.append(Token(kind, sym, i))
                i += len(sym)
                matched = True
                break
        if matched:
            continue

        raise LexError(f"unexpected character {c!r} at position {i}")

    tokens.append(Token("EOF", "", n))
    return tokens


def parse_number_literal(text: str) -> Fraction:
    if "/" in text:
        num, den = text.split("/", 1)
        return Fraction(int(num), int(den))
    return Fraction(int(text))
