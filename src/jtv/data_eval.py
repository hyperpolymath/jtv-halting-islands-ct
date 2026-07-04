"""Evaluator for the DATA sublanguage.

Every case here is a plain structural recursion over a finite AST with no
loop and no recursion into anything but strictly smaller subtrees — there
is no way for this function to fail to terminate. That is why "it parsed
as a DataNode" is already a termination certificate: this evaluator can't
diverge, whatever DataNode you hand it, because the DATA grammar admits no
node shape that could make it try.
"""

from __future__ import annotations

from fractions import Fraction

from .data_ast import Add, BoolOp, BooleanLiteral, Comparison, DataNode, Not, NumberLiteral, VarRef

Value = Fraction | bool
Env = dict[str, Value]


class DataEvalError(Exception):
    pass


_CMP_OPS = {
    "==": lambda a, b: a == b,
    "!=": lambda a, b: a != b,
    "<": lambda a, b: a < b,
    ">": lambda a, b: a > b,
    "<=": lambda a, b: a <= b,
    ">=": lambda a, b: a >= b,
}


def eval_data(node: DataNode, env: Env) -> Value:
    if isinstance(node, NumberLiteral):
        return node.value
    if isinstance(node, VarRef):
        if node.name not in env:
            raise DataEvalError(f"undefined data variable {node.name!r}")
        return env[node.name]
    if isinstance(node, Add):
        left = eval_data(node.left, env)
        right = eval_data(node.right, env)
        if not isinstance(left, Fraction) or not isinstance(right, Fraction):
            raise DataEvalError("'+' requires numeric operands")
        return left + right
    if isinstance(node, Comparison):
        left = eval_data(node.left, env)
        right = eval_data(node.right, env)
        return _CMP_OPS[node.op](left, right)
    if isinstance(node, BooleanLiteral):
        return node.value
    if isinstance(node, Not):
        operand = eval_data(node.operand, env)
        if not isinstance(operand, bool):
            raise DataEvalError("'not' requires a boolean operand")
        return not operand
    if isinstance(node, BoolOp):
        left = eval_data(node.left, env)
        right = eval_data(node.right, env)
        if not isinstance(left, bool) or not isinstance(right, bool):
            raise DataEvalError(f"'{node.op}' requires boolean operands")
        return (left and right) if node.op == "and" else (left or right)
    raise DataEvalError(f"unhandled DATA node type {type(node).__name__}")  # unreachable: sealed type set


def format_value(value: Value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, Fraction):
        return str(value.numerator) if value.denominator == 1 else f"{value.numerator}/{value.denominator}"
    raise DataEvalError(f"unhandled value type {type(value).__name__}")
