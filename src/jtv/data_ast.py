"""AST types for the DATA sublanguage.

This is one half of the structural diode: every node type defined here
holds only `Fraction`/`bool`/`str` primitives or other `DataNode`
instances. None of them has a field capable of holding a control-language
node (If/While/Block/... from control_ast.py). That is not a convention —
it is a fact about the shape of these classes. A DATA-language AST literally
cannot contain a control node, the same way a tuple of ints cannot contain
a string unless you construct it to.

DATA is addition-only and has no recursion, no loops, and no conditionals
that branch on unevaluated control. Every node here evaluates by plain
structural recursion over a finite tree, so evaluation always terminates.
That is the load-bearing property of the whole POC: membership in this
grammar (i.e. "this text parses into one of these node types") already
certifies termination, so checking totality collapses to parsing.
"""

from __future__ import annotations

from dataclasses import dataclass
from fractions import Fraction


class DataNode:
    """Marker base for every DATA-language AST node. Sealed set below."""


@dataclass(frozen=True)
class NumberLiteral(DataNode):
    value: Fraction


@dataclass(frozen=True)
class VarRef(DataNode):
    name: str


@dataclass(frozen=True)
class Add(DataNode):
    left: DataNode
    right: DataNode


@dataclass(frozen=True)
class Comparison(DataNode):
    op: str  # one of == != < > <= >=
    left: DataNode
    right: DataNode


@dataclass(frozen=True)
class BooleanLiteral(DataNode):
    value: bool


@dataclass(frozen=True)
class Not(DataNode):
    operand: DataNode


@dataclass(frozen=True)
class BoolOp(DataNode):
    op: str  # "and" | "or"
    left: DataNode
    right: DataNode


# The sealed whitelist used by the diode validator (see diode.py). Any
# object that is not an instance of one of these classes is, by
# definition, not a DATA-language node.
DATA_NODE_TYPES: tuple[type, ...] = (
    NumberLiteral,
    VarRef,
    Add,
    Comparison,
    BooleanLiteral,
    Not,
    BoolOp,
)
