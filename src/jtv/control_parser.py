"""Recursive-descent parser for the CONTROL sublanguage.

Grammar (see GRAMMAR.ebnf):

    control_program = { control_stmt }
    control_stmt    = assignment | if_stmt | while_stmt | print_stmt | block
    block           = "begin" { control_stmt } "end"
    assignment      = variable "=" data_expr        <- THE BRIDGE
    if_stmt         = "if" boolean_expr "then" control_stmt [ "else" control_stmt ]
    while_stmt      = "while" boolean_expr "do" control_stmt
    print_stmt      = "print" "(" data_expr { "," data_expr } ")"

Every data_expr / boolean_expr embedded above is parsed by calling into
`jtv.data_parser` (a wholly separate parser producing wholly separate
DataNode types — see data_parser.py), not by this parser's own grammar.
This module has no rule that produces a DataNode directly; it only ever
receives one back from data_parser and stores it inside a Assignment/If/
While/Print node, each of which re-validates it via the diode
(control_ast.py's __post_init__). Two independent parsers, two
independent AST hierarchies, one explicit, structurally-checked hand-off.
"""

from __future__ import annotations

from .control_ast import Assignment, Block, ControlStmt, If, Print, While
from .data_parser import parse_data_tokens
from .lexer import Token, tokenize


class ControlParseError(Exception):
    pass


class _ControlParser:
    def __init__(self, tokens: list[Token]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Token:
        return self.tokens[self.pos]

    def advance(self) -> Token:
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def _expect(self, kind: str) -> Token:
        tok = self.peek()
        if tok.kind != kind:
            raise ControlParseError(
                f"expected {kind} but found {tok.kind} ({tok.text!r}) at position {tok.pos}"
            )
        return self.advance()

    def _parse_data_subexpr(self):
        node, next_pos = parse_data_tokens(self.tokens, self.pos)
        self.pos = next_pos
        return node

    def parse_program(self) -> Block:
        stmts = []
        while self.peek().kind != "EOF":
            stmts.append(self.parse_stmt())
        return Block(stmts=tuple(stmts))

    def parse_stmt(self) -> ControlStmt:
        tok = self.peek()
        if tok.kind == "BEGIN":
            return self.parse_block()
        if tok.kind == "IF":
            return self.parse_if()
        if tok.kind == "WHILE":
            return self.parse_while()
        if tok.kind == "PRINT":
            return self.parse_print()
        if tok.kind == "IDENT":
            return self.parse_assignment()
        raise ControlParseError(f"unexpected token {tok.kind} ({tok.text!r}) at position {tok.pos}")

    def parse_block(self) -> Block:
        self._expect("BEGIN")
        stmts = []
        while self.peek().kind != "END":
            if self.peek().kind == "EOF":
                raise ControlParseError("unterminated block: expected 'end', found EOF")
            stmts.append(self.parse_stmt())
        self._expect("END")
        return Block(stmts=tuple(stmts))

    def parse_assignment(self) -> Assignment:
        var_tok = self._expect("IDENT")
        self._expect("ASSIGN")
        expr = self._parse_data_subexpr()
        return Assignment(var=var_tok.text, expr=expr)

    def parse_if(self) -> If:
        self._expect("IF")
        cond = self._parse_data_subexpr()
        self._expect("THEN")
        then_branch = self.parse_stmt()
        else_branch = None
        if self.peek().kind == "ELSE":
            self.advance()
            else_branch = self.parse_stmt()
        return If(cond=cond, then_branch=then_branch, else_branch=else_branch)

    def parse_while(self) -> While:
        self._expect("WHILE")
        cond = self._parse_data_subexpr()
        self._expect("DO")
        body = self.parse_stmt()
        return While(cond=cond, body=body)

    def parse_print(self) -> Print:
        self._expect("PRINT")
        self._expect("LPAREN")
        args = [self._parse_data_subexpr()]
        while self.peek().kind == "COMMA":
            self.advance()
            args.append(self._parse_data_subexpr())
        self._expect("RPAREN")
        return Print(args=tuple(args))


def parse_control_program(source: str) -> Block:
    tokens = tokenize(source)
    return _ControlParser(tokens).parse_program()
