"""THE HEART OF THE POC.

Every test in this file proves the one-way bridge cannot be crossed
backwards: control flow can never end up inside a data expression.

Two independent layers are exercised:

  1. Text-level: feeding source text where a control keyword appears in a
     DATA-grammar position (an assignment RHS, an if/while condition, a
     print argument, or a bare data expression) must fail to *parse* —
     DataParseError, raised by the DATA parser's own grammar, before any
     evaluation is attempted.

  2. AST-level: constructing a CONTROL node (Assignment/If/While/Print)
     directly in Python with a non-DataNode object in a data-position
     field must fail at construction time with DiodeViolation, proving
     the seal is a structural property of the types and not merely a
     parser-level convention that a future refactor could bypass.
"""

from fractions import Fraction

import pytest

from jtv.control_ast import Assignment, If, Print, While
from jtv.control_parser import parse_control_program
from jtv.data_ast import Add, NumberLiteral
from jtv.data_parser import DataParseError, parse_data_program
from jtv.diode import DiodeViolation, assert_pure_data


# --- Layer 1: control keywords rejected by the DATA parser's own grammar ---

@pytest.mark.parametrize(
    "source",
    [
        "if 1 < 2 then 3 else 4",
        "while 1 < 2 do 3",
        "begin x = 1 end",
        "print(1)",
        "if true then true else false",
    ],
)
def test_bare_data_expr_cannot_contain_control_keywords(source):
    with pytest.raises(DataParseError):
        parse_data_program(source)


@pytest.mark.parametrize(
    "source",
    [
        "x = if 1 < 2 then 3 else 4",              # control in assignment RHS
        "x = while 1 < 2 do 3",                     # control in assignment RHS
        "if if true then true else false then x = 1", # control in if-condition
        "while begin x = 1 end do x = 1",           # control in while-condition
        "print(if true then 1 else 2)",             # control in print argument
    ],
)
def test_control_program_cannot_smuggle_control_into_data_position(source):
    with pytest.raises(DataParseError):
        parse_control_program(source)


def test_violation_example_file_is_rejected(example_source):
    with pytest.raises(DataParseError) as exc_info:
        parse_control_program(example_source("violation.jtv"))
    assert "diode" in str(exc_info.value).lower() or "control-flow keywords" in str(exc_info.value)


# --- Layer 2: the AST-level validator rejects hand-built violations too ---

class _FakeControlNode:
    """Stands in for a control_ast node, to prove the check isn't `isinstance(..., Block)` special-casing."""


def test_assert_pure_data_rejects_non_data_node():
    with pytest.raises(DiodeViolation):
        assert_pure_data(_FakeControlNode())


def test_assignment_constructor_rejects_non_data_expr():
    with pytest.raises(DiodeViolation):
        Assignment(var="x", expr=_FakeControlNode())


def test_if_constructor_rejects_non_data_cond():
    with pytest.raises(DiodeViolation):
        If(cond=_FakeControlNode(), then_branch=Assignment(var="x", expr=NumberLiteral(Fraction(1))))


def test_while_constructor_rejects_non_data_cond():
    with pytest.raises(DiodeViolation):
        While(cond=_FakeControlNode(), body=Assignment(var="x", expr=NumberLiteral(Fraction(1))))


def test_print_constructor_rejects_non_data_arg():
    with pytest.raises(DiodeViolation):
        Print(args=(NumberLiteral(Fraction(1)), _FakeControlNode()))


def test_assert_pure_data_rejects_control_node_nested_inside_data_node():
    # Build a DataNode whose child field holds something that is not a
    # legal DataNode, bypassing the parser entirely (constructing the
    # tree directly, as a future refactor or a buggy tool might).
    poisoned = Add(left=NumberLiteral(Fraction(1)), right=_FakeControlNode())
    with pytest.raises(DiodeViolation):
        assert_pure_data(poisoned)


def test_assert_pure_data_accepts_legal_data_tree():
    tree = Add(left=NumberLiteral(Fraction(1)), right=NumberLiteral(Fraction(2)))
    assert assert_pure_data(tree) is tree
