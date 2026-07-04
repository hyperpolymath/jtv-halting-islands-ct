"""Recursive-descent parser for the DATA sublanguage ONLY.

This parser's grammar (encoded directly in the structure of the methods
below) has no production that mentions BEGIN, IF, THEN, ELSE, WHILE, DO,
PRINT, or ASSIGN. Those tokens simply never appear on the right-hand side
of any `parse_*` method here. If the token stream contains one of them at
a position this parser is trying to consume, `_expect`/`_atom` fall through
to the "unexpected token" error path below — there is no code path that
would let a control keyword become part of a DataNode. That absence is
the diode's first layer (see diode.py for the second).

Grammar implemented (see GRAMMAR.ebnf for the full spec):

    data_expr       = boolean_expr
    boolean_expr    = boolean_term { ("and" | "or") boolean_term }
    boolean_term    = ["not"] boolean_atom
    boolean_atom    = comparison_expr | boolean_literal | "(" boolean_expr ")" | additive_expr
    comparison_expr = additive_expr comp_op additive_expr
    additive_expr   = term { add_op term }
    term            = number_literal | data_variable
"""

from __future__ import annotations

from .data_ast import Add, BoolOp, BooleanLiteral, Comparison, DataNode, Not, NumberLiteral, VarRef
from .lexer import Token, parse_number_literal, tokenize

_CONTROL_KEYWORDS = {"BEGIN", "END", "IF", "THEN", "ELSE", "WHILE", "DO", "PRINT", "ASSIGN"}


class DataParseError(Exception):
    pass


class _DataParser:
    def __init__(self, tokens: list[Token]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Token:
        return self.tokens[self.pos]

    def advance(self) -> Token:
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def _unexpected(self, tok: Token) -> DataParseError:
        if tok.kind in _CONTROL_KEYWORDS:
            return DataParseError(
                f"unexpected token {tok.kind} ({tok.text!r}) at position {tok.pos}: "
                "control-flow keywords are not part of the DATA grammar — "
                "the diode blocks control from being smuggled into a data expression"
            )
        return DataParseError(f"unexpected token {tok.kind} ({tok.text!r}) at position {tok.pos}")

    # data_expr = boolean_expr
    def parse_data_expr(self) -> DataNode:
        return self.parse_boolean_expr()

    def parse_boolean_expr(self) -> DataNode:
        left = self.parse_boolean_term()
        while self.peek().kind in ("AND", "OR"):
            op_tok = self.advance()
            right = self.parse_boolean_term()
            left = BoolOp(op="and" if op_tok.kind == "AND" else "or", left=left, right=right)
        return left

    def parse_boolean_term(self) -> DataNode:
        negate = False
        if self.peek().kind == "NOT":
            self.advance()
            negate = True
        atom = self.parse_boolean_atom()
        return Not(operand=atom) if negate else atom

    def parse_boolean_atom(self) -> DataNode:
        tok = self.peek()
        if tok.kind == "TRUE":
            self.advance()
            return BooleanLiteral(True)
        if tok.kind == "FALSE":
            self.advance()
            return BooleanLiteral(False)
        if tok.kind == "LPAREN":
            self.advance()
            inner = self.parse_boolean_expr()
            self._expect("RPAREN")
            return inner
        # Otherwise: additive_expr, optionally followed by a comparison op.
        left = self.parse_additive_expr()
        if self.peek().kind == "CMPOP":
            op_tok = self.advance()
            right = self.parse_additive_expr()
            return Comparison(op=op_tok.text, left=left, right=right)
        return left

    def parse_additive_expr(self) -> DataNode:
        left = self.parse_term()
        while self.peek().kind in ("ADDOP_SYM", "ADDOP_WORD"):
            self.advance()
            right = self.parse_term()
            left = Add(left=left, right=right)
        return left

    def parse_term(self) -> DataNode:
        tok = self.peek()
        if tok.kind == "NUMBER":
            self.advance()
            return NumberLiteral(parse_number_literal(tok.text))
        if tok.kind == "IDENT":
            self.advance()
            return VarRef(tok.text)
        raise self._unexpected(tok)

    def _expect(self, kind: str) -> Token:
        tok = self.peek()
        if tok.kind != kind:
            raise self._unexpected(tok)
        return self.advance()


def parse_data_tokens(tokens: list[Token], start: int = 0) -> tuple[DataNode, int]:
    """Parse a data_expr starting at `start`; returns (node, next_pos).

    Used both as the standalone entry point and by the CONTROL parser to
    consume an embedded data sub-expression from a shared token stream.
    """
    parser = _DataParser(tokens)
    parser.pos = start
    node = parser.parse_data_expr()
    return node, parser.pos


def parse_data_program(source: str) -> DataNode:
    """Parse an entire source string as a single DATA-language expression."""
    tokens = tokenize(source)
    node, pos = parse_data_tokens(tokens, 0)
    trailing = tokens[pos]
    if trailing.kind != "EOF":
        raise _DataParser(tokens)._unexpected(trailing)
    return node
