// SPDX-License-Identifier: MPL-2.0
//! Parse + evaluate tests for the DATA language. Ported from the retired
//! Python `tests/test_data_language.py`; rational-number cases are replaced
//! because DATA numbers are now i64 (see README, "DATA numbers").

mod common;

use jtv_halting_islands_ct::{eval_data, parse_data_program, DataSourceError, DataValue, Env};

fn ev(source: &str) -> DataValue {
    let expr = parse_data_program(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    eval_data(&expr, &Env::new()).unwrap_or_else(|e| panic!("{source:?}: {e}"))
}

fn num(n: i64) -> DataValue {
    DataValue::Number(n)
}

fn b(v: bool) -> DataValue {
    DataValue::Boolean(v)
}

#[test]
fn all_add_op_spellings() {
    assert_eq!(ev("1 + 2"), num(3));
    assert_eq!(ev("1 plus 2"), num(3));
    assert_eq!(ev("1 added_to 2"), num(3));
    assert_eq!(ev("1 with 2"), num(3));
}

#[test]
fn integer_literal() {
    assert_eq!(ev("42"), num(42));
}

#[test]
fn rational_literal_is_rejected_with_a_clear_message() {
    match parse_data_program("1/3 plus 1/3") {
        Err(DataSourceError::Parse(e)) => assert!(e.message.contains("rational literal"), "{e}"),
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn literal_too_large_for_i64_is_rejected() {
    assert!(parse_data_program("9223372036854775808").is_err());
    assert_eq!(ev("9223372036854775807"), num(i64::MAX));
}

#[test]
fn overflow_is_an_error_not_a_panic_or_wrap() {
    let expr = parse_data_program("9223372036854775807 plus 1").unwrap();
    let err = eval_data(&expr, &Env::new()).unwrap_err();
    assert!(err.contains("overflow"), "{err}");
}

#[test]
fn variable_lookup() {
    let expr = parse_data_program("x plus 1").unwrap();
    let mut env = Env::new();
    env.insert("x".into(), num(4));
    assert_eq!(eval_data(&expr, &env), Ok(num(5)));
}

#[test]
fn undefined_variable_is_an_error() {
    let expr = parse_data_program("y").unwrap();
    assert!(eval_data(&expr, &Env::new()).is_err());
}

#[test]
fn comparison_ops() {
    assert_eq!(ev("1 < 2"), b(true));
    assert_eq!(ev("2 < 1"), b(false));
    assert_eq!(ev("2 <= 2"), b(true));
    assert_eq!(ev("2 >= 3"), b(false));
    assert_eq!(ev("2 == 2"), b(true));
    assert_eq!(ev("2 != 2"), b(false));
    assert_eq!(ev("3 > 2"), b(true));
}

#[test]
fn boolean_literals_and_not() {
    assert_eq!(ev("true"), b(true));
    assert_eq!(ev("false"), b(false));
    assert_eq!(ev("not true"), b(false));
    assert_eq!(ev("not false"), b(true));
}

#[test]
fn and_or() {
    assert_eq!(ev("true and false"), b(false));
    assert_eq!(ev("true or false"), b(true));
    assert_eq!(ev("1 < 2 and 3 > 2"), b(true));
    assert_eq!(ev("1 > 2 or 3 > 2"), b(true));
}

#[test]
fn parenthesized_boolean_grouping() {
    assert_eq!(ev("not (3 < 1)"), b(true));
    assert_eq!(ev("(1 < 2) and (2 < 3)"), b(true));
}

#[test]
fn type_errors_are_errors() {
    // The grammar only allows numbers/variables as `+` and comparison
    // operands, so a boolean *literal* there is already a parse error.
    assert!(parse_data_program("true < 1").is_err());
    assert!(parse_data_program("true plus 1").is_err());
    // A variable bound to a boolean gets past the parser and is caught by
    // the evaluator instead.
    let mut env = Env::new();
    env.insert("t".into(), b(true));
    for source in ["t plus 1", "t < 1", "not 1", "1 and true", "1 or 2"] {
        let expr = parse_data_program(source).unwrap();
        assert!(
            eval_data(&expr, &env).is_err(),
            "{source:?} should not evaluate"
        );
    }
}

#[test]
fn value_formatting() {
    assert_eq!(num(-7).to_string(), "-7");
    assert_eq!(b(true).to_string(), "true");
    assert_eq!(b(false).to_string(), "false");
}

#[test]
fn example_pure_data_file() {
    let expr = parse_data_program(&common::example_source("pure_data.jtv")).unwrap();
    assert_eq!(eval_data(&expr, &Env::new()), Ok(num(42)));
}

#[test]
fn malformed_data_expressions_are_rejected() {
    for source in ["1 +", "+ 1", "1 < ", "(1 < 2", "1 2", "", "1 $ 2"] {
        assert!(
            parse_data_program(source).is_err(),
            "{source:?} should be rejected"
        );
    }
}
