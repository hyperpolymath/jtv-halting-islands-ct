"""The diode: structural enforcement of the one-way DATA -> CONTROL bridge.

Two independent layers do the enforcing:

  (a) Two separate parsers with two separate AST hierarchies (data_ast.py
      / control_ast.py). The DATA parser's grammar has no production that
      accepts BEGIN/IF/THEN/ELSE/WHILE/DO/PRINT tokens, so a fragment that
      tries to embed control flow inside a data expression fails to parse
      — full stop, at the syntax level, before any evaluation happens.

  (b) This module: `assert_pure_data`, a validator that walks a DataNode
      tree and rejects anything that is not built exclusively from the
      sealed DATA_NODE_TYPES whitelist. Every CONTROL-side AST node that
      embeds a data expression (Assignment.expr, If.cond, While.cond,
      Print.args) calls this validator at construction time, so even a
      hand-built AST (bypassing the parser entirely, e.g. from a test or
      a future refactor) cannot smuggle a control node into a data
      position and have it silently accepted.

The two layers are redundant by design: (a) means illegal *text* never
parses; (b) means illegal *AST shapes* never construct, even if someone
builds one directly in Python. The one-way property is a structural fact
of the types, not a runtime check you could forget to call.
"""

from __future__ import annotations

import dataclasses
from fractions import Fraction

from .data_ast import DataNode, DATA_NODE_TYPES

# The only non-DataNode value types any legal DataNode field may hold.
_PRIMITIVE_FIELD_TYPES = (Fraction, bool, str)


class DiodeViolation(Exception):
    """Raised when a non-DATA node is found where a DATA node is required."""


def assert_pure_data(node: object, where: str = "<data position>") -> DataNode:
    if not isinstance(node, DataNode) or not isinstance(node, DATA_NODE_TYPES):
        raise DiodeViolation(
            f"diode violation at {where}: expected a DATA-language node, "
            f"got {type(node).__name__!r} — control cannot be smuggled into data"
        )
    for field in dataclasses.fields(node):
        _check_field_value(getattr(node, field.name), where=f"{where} -> {field.name}")
    return node  # type: ignore[return-value]


def _check_field_value(value: object, where: str) -> None:
    if isinstance(value, DataNode):
        assert_pure_data(value, where=where)
        return
    if isinstance(value, _PRIMITIVE_FIELD_TYPES):
        return
    if isinstance(value, tuple):
        for item in value:
            _check_field_value(item, where=where)
        return
    # Anything else — including a control_ast node, or any other object —
    # is not a legal thing to find inside a DATA-language AST field.
    raise DiodeViolation(
        f"diode violation at {where}: illegal embedded value of type "
        f"{type(value).__name__!r} inside a DATA node — only DATA nodes and "
        "primitive values (Fraction/bool/str) are allowed there"
    )
