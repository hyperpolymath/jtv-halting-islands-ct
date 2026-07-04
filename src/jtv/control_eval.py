"""Evaluator for the CONTROL sublanguage.

Unlike data_eval.eval_data, this function is NOT guaranteed to terminate:
While.body is executed by a plain Python `while` loop driven by a data
condition that is re-evaluated (and re-checked by the diode's totality
argument) on every pass, but nothing stops a CONTROL program from writing
a condition that never goes false. That openness is intentional — CONTROL
is openly Turing-complete. See README LIMITATIONS: this file gives you a
totality oracle for each individual condition/assignment check, not a
halting proof for the program built out of them.
"""

from __future__ import annotations

from .control_ast import Assignment, Block, ControlStmt, If, Print, While
from .data_eval import Env, eval_data, format_value


def exec_stmt(stmt: ControlStmt, env: Env) -> None:
    if isinstance(stmt, Assignment):
        env[stmt.var] = eval_data(stmt.expr, env)
        return
    if isinstance(stmt, If):
        cond = eval_data(stmt.cond, env)
        if cond:
            exec_stmt(stmt.then_branch, env)
        elif stmt.else_branch is not None:
            exec_stmt(stmt.else_branch, env)
        return
    if isinstance(stmt, While):
        while eval_data(stmt.cond, env):
            exec_stmt(stmt.body, env)
        return
    if isinstance(stmt, Print):
        values = [eval_data(a, env) for a in stmt.args]
        print(", ".join(format_value(v) for v in values))
        return
    if isinstance(stmt, Block):
        for s in stmt.stmts:
            exec_stmt(s, env)
        return
    raise TypeError(f"unhandled CONTROL node type {type(stmt).__name__}")  # unreachable: sealed type set


def run_control_program(program: Block, env: Env | None = None) -> Env:
    env = {} if env is None else env
    exec_stmt(program, env)
    return env
