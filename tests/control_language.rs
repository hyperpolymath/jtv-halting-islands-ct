// SPDX-License-Identifier: MPL-2.0
//! Parse + execute tests for the Turing-complete CONTROL language. Ported
//! from the retired Python `tests/test_control_language.py`.

mod common;

use jtv_halting_islands_ct::{
    eval_control, parse_control_program, DataValue, Env, MAX_CONTROL_NESTING,
};

fn run(source: &str) -> (Env, String) {
    let prog = parse_control_program(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    let mut env = Env::new();
    let mut out = Vec::new();
    eval_control(&prog, &mut env, &mut out).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    (env, String::from_utf8(out).unwrap())
}

#[test]
fn assignment_binds_data_result() {
    let (env, _) = run("x = 1 plus 2");
    assert_eq!(env["x"], DataValue::Number(3));
}

#[test]
fn if_then_else() {
    let (env, _) = run("begin\n  x = 5\n  if x < 10 then\n    y = 1\n  else\n    y = 0\nend");
    assert_eq!(env["y"], DataValue::Number(1));
    // False condition: the else branch must run.
    let (env, _) = run("begin\n  x = 15\n  if x < 10 then\n    y = 1\n  else\n    y = 0\nend");
    assert_eq!(env["y"], DataValue::Number(0));
    // False condition and no else branch: nothing runs.
    let (env, _) = run("if false then y = 1");
    assert!(!env.contains_key("y"));
}

#[test]
fn while_loop_oracle() {
    let (env, out) = run(
        "begin n = 5 sum = 0 i = 0 while i < n do begin sum = sum plus i i = i plus 1 end print(sum) end",
    );
    assert_eq!(env["sum"], DataValue::Number(10));
    assert_eq!(out, "10\n");
}

#[test]
fn print_multiple_args() {
    let (_, out) = run("print(1 plus 1, true, 40 plus 2)");
    assert_eq!(out, "2, true, 42\n");
}

#[test]
fn example_oracle_file() {
    let (env, out) = run(&common::example_source("oracle.jtv"));
    assert_eq!(env["sum"], DataValue::Number(10));
    assert_eq!(out, "10\n");
}

#[test]
fn non_boolean_condition_is_an_evaluation_error() {
    let prog = parse_control_program("while 1 do x = 1").unwrap();
    assert!(eval_control(&prog, &mut Env::new(), &mut Vec::new()).is_err());
}

#[test]
fn malformed_control_programs_are_rejected() {
    for source in [
        "begin x = 1",
        "if true then",
        "while true",
        "print(1",
        "x",
        "end",
        "x = 1 )",
    ] {
        assert!(
            parse_control_program(source).is_err(),
            "{source:?} should be rejected"
        );
    }
}

#[test]
fn nesting_within_the_limit_runs() {
    let n = 200;
    let source = format!("{}x = 1{}", "begin ".repeat(n), " end".repeat(n));
    let (env, _) = run(&source);
    assert_eq!(env["x"], DataValue::Number(1));
}

#[test]
fn nesting_past_the_limit_is_a_parse_error() {
    for n in [MAX_CONTROL_NESTING, 200_000] {
        let source = format!("{}x = 1{}", "begin ".repeat(n), " end".repeat(n));
        let err = parse_control_program(&source).unwrap_err();
        assert!(err.to_string().contains("nested too deeply"), "{err}");
        let source = format!("{}x = 1", "if true then ".repeat(n));
        let err = parse_control_program(&source).unwrap_err();
        assert!(err.to_string().contains("nested too deeply"), "{err}");
    }
}
