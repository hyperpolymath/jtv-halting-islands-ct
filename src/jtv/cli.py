"""Minimal CLI: run a .jtv file as either a DATA expression or a CONTROL program."""

from __future__ import annotations

import argparse
import sys

from .control_eval import run_control_program
from .control_parser import parse_control_program
from .data_eval import eval_data, format_value
from .data_parser import parse_data_program


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run a JtV source file.")
    parser.add_argument("file", help="path to a .jtv source file")
    parser.add_argument(
        "--mode", choices=["data", "control"], required=True,
        help="parse+evaluate as a standalone DATA expression, or parse+run as a CONTROL program",
    )
    args = parser.parse_args(argv)

    with open(args.file, encoding="utf-8") as f:
        source = f.read()

    if args.mode == "data":
        node = parse_data_program(source)
        print(format_value(eval_data(node, {})))
    else:
        program = parse_control_program(source)
        run_control_program(program)
    return 0


if __name__ == "__main__":
    sys.exit(main())
