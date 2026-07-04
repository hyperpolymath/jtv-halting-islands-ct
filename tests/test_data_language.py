"""Parse + evaluate tests for the total DATA language."""

from fractions import Fraction

import pytest

from jtv.data_eval import DataEvalError, eval_data, format_value
from jtv.data_parser import DataParseError, parse_data_program


def ev(source: str):
    return eval_data(parse_data_program(source), {})


def test_exact_rational_addition():
    assert ev("1/3 plus 1/3 added_to 1/3") == Fraction(1, 1)


def test_all_add_op_spellings():
    assert ev("1 + 2") == Fraction(3)
    assert ev("1 plus 2") == Fraction(3)
    assert ev("1 added_to 2") == Fraction(3)
    assert ev("1 with 2") == Fraction(3)


def test_integer_literal():
    assert ev("42") == Fraction(42)


def test_fraction_literal():
    assert ev("3/4") == Fraction(3, 4)


def test_variable_lookup():
    node = parse_data_program("x plus 1")
    assert eval_data(node, {"x": Fraction(4)}) == Fraction(5)


def test_undefined_variable_raises():
    node = parse_data_program("y")
    with pytest.raises(DataEvalError):
        eval_data(node, {})


def test_comparison_ops():
    assert ev("1 < 2") is True
    assert ev("2 < 1") is False
    assert ev("2 <= 2") is True
    assert ev("2 >= 3") is False
    assert ev("2 == 2") is True
    assert ev("2 != 2") is False
    assert ev("3 > 2") is True


def test_boolean_literals_and_not():
    assert ev("true") is True
    assert ev("false") is False
    assert ev("not true") is False
    assert ev("not false") is True


def test_and_or():
    assert ev("true and false") is False
    assert ev("true or false") is True
    assert ev("1 < 2 and 3 > 2") is True
    assert ev("1 > 2 or 3 > 2") is True


def test_parenthesized_boolean_grouping():
    assert ev("not (3 < 1)") is True
    assert ev("(1 < 2) and (2 < 3)") is True


def test_format_value():
    assert format_value(Fraction(1)) == "1"
    assert format_value(Fraction(1, 3)) == "1/3"
    assert format_value(True) == "true"
    assert format_value(False) == "false"


def test_example_pure_data_file(example_source):
    node = parse_data_program(example_source("pure_data.jtv"))
    assert eval_data(node, {}) == Fraction(1)


@pytest.mark.parametrize(
    "source",
    [
        "1 +",          # dangling operator
        "+ 1",          # leading operator
        "1 < ",         # dangling comparison
        "(1 < 2",       # unterminated parens
        "1 2",          # two terms with no operator
    ],
)
def test_malformed_data_expressions_are_rejected(source):
    with pytest.raises(DataParseError):
        parse_data_program(source)
