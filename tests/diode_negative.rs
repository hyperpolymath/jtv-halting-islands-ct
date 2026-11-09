// SPDX-License-Identifier: MPL-2.0
//! THE HEART OF THE POC: the one-way bridge cannot be crossed backwards.
//!
//! Ported from the retired Python suite `tests/test_diode_negative.py`
//! (removed in `abe769d`).
//!
//! Layer 1 (here): source text that puts a control keyword in a DATA-grammar
//! position (a bare DATA expression, an assignment right-hand side, an
//! `if`/`while` condition, a `print` argument) must fail to *parse*, with the
//! error raised by the DATA parser, before any evaluation.
//!
//! Layer 2 (AST level): in Python this was a run-time validator. In Rust the
//! `DataExpr` type has no variant that can hold a `ControlStmt`, so the
//! equivalent negative tests are the paired `compile_fail` doctests in
//! `src/lib.rs`, which `cargo test` runs.

mod common;

use jtv_halting_islands_ct::{
    parse_control_program, parse_data_program, DataSourceError, ParseError,
};

#[test]
fn bare_data_expr_cannot_contain_control_keywords() {
    for source in [
        "if 1 < 2 then 3 else 4",
        "while 1 < 2 do 3",
        "begin x = 1 end",
        "print(1)",
        "if true then true else false",
        "x = 1",
    ] {
        match parse_data_program(source) {
            Err(DataSourceError::Parse(e)) => assert!(
                e.message.contains("diode"),
                "{source:?}: rejected, but not by the diode rule: {e}"
            ),
            other => panic!("{source:?} must be rejected by the DATA parser, got {other:?}"),
        }
    }
}

#[test]
fn control_program_cannot_smuggle_control_into_data_position() {
    for source in [
        "x = if 1 < 2 then 3 else 4", // control in assignment RHS
        "x = while 1 < 2 do 3",       // control in assignment RHS
        "if if true then true else false then x = 1", // control in if-condition
        "while begin x = 1 end do x = 1", // control in while-condition
        "print(if true then 1 else 2)", // control in print argument
        "print(1, begin end)",        // control in a later print argument
    ] {
        match parse_control_program(source) {
            Err(ParseError::Data(e)) => assert!(
                e.message.contains("diode"),
                "{source:?}: rejected in DATA position, but not by the diode rule: {e}"
            ),
            other => panic!("{source:?} must be rejected by the DATA parser, got {other:?}"),
        }
    }
}

#[test]
fn violation_example_file_is_rejected() {
    let err = parse_control_program(&common::example_source("violation.jtv"))
        .expect_err("examples/violation.jtv must be rejected");
    match err {
        ParseError::Data(e) => assert!(e.message.contains("diode"), "{e}"),
        other => panic!("expected a DATA parse error, got {other:?}"),
    }
}

#[test]
fn legal_bridge_uses_are_accepted() {
    // The positive control for the tests above: the same positions accept DATA.
    for source in [
        "x = 1 plus 2",
        "if 1 < 2 then x = 3 else x = 4",
        "while false do x = 1",
        "print(1, true)",
    ] {
        parse_control_program(source).unwrap_or_else(|e| panic!("{source:?} should parse: {e}"));
    }
}
