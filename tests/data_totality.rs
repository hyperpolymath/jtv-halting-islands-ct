// SPDX-License-Identifier: MPL-2.0
//! Bounded exhaustive check of the claim that DATA evaluation always
//! returns: `eval_data` must give `Ok` or `Err`, never panic, on every
//! `DataExpr` up to depth 2 built from a fixed set of atoms, including the
//! `i64` extremes (which made the previous unchecked `+` panic in debug
//! builds).
//!
//! This is a test over a finite space, not a proof. It does not cover
//! deeper trees, and it says nothing about stack usage on very deep inputs.

use jtv_halting_islands_ct::{eval_data, BoolOp, CompOp, DataExpr, DataValue, Env};

fn atoms() -> Vec<DataExpr> {
    vec![
        DataExpr::Number(0),
        DataExpr::Number(1),
        DataExpr::Number(-1),
        DataExpr::Number(i64::MAX),
        DataExpr::Number(i64::MIN),
        DataExpr::Boolean(true),
        DataExpr::Boolean(false),
        DataExpr::VarRef("x".into()),
        DataExpr::VarRef("unbound".into()),
    ]
}

fn binary(l: &DataExpr, r: &DataExpr) -> Vec<DataExpr> {
    let (l, r) = (Box::new(l.clone()), Box::new(r.clone()));
    let mut v = vec![DataExpr::Add(l.clone(), r.clone())];
    for op in [
        CompOp::Eq,
        CompOp::Neq,
        CompOp::Lt,
        CompOp::Gt,
        CompOp::Lte,
        CompOp::Gte,
    ] {
        v.push(DataExpr::Comparison(op, l.clone(), r.clone()));
    }
    for op in [BoolOp::And, BoolOp::Or] {
        v.push(DataExpr::BoolOp(op, l.clone(), r.clone()));
    }
    v
}

fn grow(level: &[DataExpr]) -> Vec<DataExpr> {
    let mut next: Vec<DataExpr> = atoms();
    next.extend(level.iter().map(|e| DataExpr::Not(Box::new(e.clone()))));
    for l in level {
        for r in level {
            next.extend(binary(l, r));
        }
    }
    next
}

#[test]
fn eval_data_returns_on_every_tree_up_to_depth_two() {
    let mut env = Env::new();
    env.insert("x".into(), DataValue::Number(i64::MAX));

    let depth1 = grow(&atoms());
    let mut checked = 0usize;
    let mut ok = 0usize;
    for l in &depth1 {
        for r in &depth1 {
            for e in binary(l, r) {
                if eval_data(&e, &env).is_ok() {
                    ok += 1;
                }
                checked += 1;
            }
        }
        if eval_data(&DataExpr::Not(Box::new(l.clone())), &env).is_ok() {
            ok += 1;
        }
        checked += 1;
    }
    // Sanity: the space is non-trivial and contains both outcomes.
    assert!(checked > 1_000_000, "checked only {checked} trees");
    assert!(ok > 0 && ok < checked, "ok={ok} checked={checked}");
}
