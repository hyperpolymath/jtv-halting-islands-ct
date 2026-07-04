"""AST types for the CONTROL sublanguage (Turing-complete, imperative).

Every place CONTROL touches DATA — an assignment's right-hand side, an
if/while condition, a print argument — is validated through
`diode.assert_pure_data` in `__post_init__`. That call cannot be skipped:
it runs whenever one of these nodes is constructed, whether by the
control parser or by hand (e.g. in a test). This is the second diode
layer described in diode.py; the first is that the DATA parser's own
grammar can't produce a control node in the first place.

Note the asymmetry: CONTROL nodes are allowed to *hold* DataNode values
(that is the bridge). DataNode types (data_ast.py) have no symmetrical
field that could hold a ControlStmt — the bridge is one-way by
construction, not by convention.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .data_ast import DataNode
from .diode import assert_pure_data


class ControlStmt:
    """Marker base for every CONTROL-language statement node."""


@dataclass(frozen=True)
class Assignment(ControlStmt):
    var: str
    expr: DataNode  # THE BRIDGE: a halted DATA value binds to a control variable

    def __post_init__(self) -> None:
        assert_pure_data(self.expr, where=f"assignment to {self.var!r}")


@dataclass(frozen=True)
class If(ControlStmt):
    cond: DataNode
    then_branch: ControlStmt
    else_branch: ControlStmt | None = None

    def __post_init__(self) -> None:
        assert_pure_data(self.cond, where="if-condition")


@dataclass(frozen=True)
class While(ControlStmt):
    cond: DataNode
    body: ControlStmt

    def __post_init__(self) -> None:
        assert_pure_data(self.cond, where="while-condition")
        # NOTE: `body` is CONTROL and may run forever — that is expected.
        # The diode only guarantees the *condition expression* halts on
        # each check; it says nothing about how many times the loop
        # iterates. See README LIMITATIONS: totality-oracle, not
        # whole-program-halting.


@dataclass(frozen=True)
class Print(ControlStmt):
    args: tuple[DataNode, ...] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        for i, a in enumerate(self.args):
            assert_pure_data(a, where=f"print argument {i}")


@dataclass(frozen=True)
class Block(ControlStmt):
    stmts: tuple[ControlStmt, ...] = field(default_factory=tuple)
