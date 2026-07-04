"""Parse + execute tests for the Turing-complete CONTROL language."""

from fractions import Fraction

import pytest

from jtv.control_eval import run_control_program
from jtv.control_parser import ControlParseError, parse_control_program
from jtv.data_parser import DataParseError


def run(source: str, capsys):
    env = run_control_program(parse_control_program(source))
    out = capsys.readouterr().out
    return env, out


def test_assignment_binds_data_result(capsys):
    env, _ = run("x = 1 plus 2", capsys)
    assert env["x"] == Fraction(3)


def test_if_then_else(capsys):
    env, _ = run(
        """
        begin
          x = 5
          if x < 10 then
            y = 1
          else
            y = 0
        end
        """,
        capsys,
    )
    assert env["y"] == Fraction(1)


def test_while_loop_oracle(capsys):
    env, out = run(
        """
        begin
          n = 5
          sum = 0
          i = 0
          while i < n do
            begin
              sum = sum plus i
              i = i plus 1
            end
          print(sum)
        end
        """,
        capsys,
    )
    assert env["sum"] == Fraction(10)
    assert out.strip() == "10"


def test_print_multiple_args(capsys):
    _, out = run("print(1 plus 1, true, 1/3 plus 1/3 added_to 1/3)", capsys)
    assert out.strip() == "2, true, 1"


def test_example_oracle_file(example_source, capsys):
    env, out = run(example_source("oracle.jtv"), capsys)
    assert env["sum"] == Fraction(10)
    assert out.strip() == "10"


def test_example_violation_file_is_rejected(example_source):
    with pytest.raises(DataParseError):
        parse_control_program(example_source("violation.jtv"))


@pytest.mark.parametrize(
    "source",
    [
        "begin x = 1",       # missing 'end'
        "if true then",      # missing branch statement
        "while true",        # missing 'do'
        "print(1",           # missing ')'
    ],
)
def test_malformed_control_programs_are_rejected(source):
    with pytest.raises((ControlParseError, DataParseError, IndexError)):
        parse_control_program(source)
