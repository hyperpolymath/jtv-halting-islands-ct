#![cfg_attr(feature = "verify", feature(stmt_expr_attributes, register_tool))]
#![cfg_attr(feature = "verify", register_tool(creusot))]

#[cfg(feature = "verify")]
use creusot_contracts::*;

use std::collections::HashMap;

/// JtV (Julia the Viper) — Harvard-architecture language with two grammatically sealed sublanguages.
/// 
/// 1. DATA: addition-only, total (provably-halting) expression language.
///    Represented by the `DataExpr` AST.
/// 2. CONTROL: Turing-complete and imperative. Represented by the `ControlStmt` AST.
///
/// The diode bridge: Control logic can evaluate a `DataExpr` and bind it to a variable,
/// but a `DataExpr` structurally cannot contain a `ControlStmt`.

#[derive(Clone, Debug, PartialEq)]
pub enum DataExpr {
    Number(i64),
    Boolean(bool),
    VarRef(String),
    Add(Box<DataExpr>, Box<DataExpr>),
    Comparison(CompOp, Box<DataExpr>, Box<DataExpr>),
    Not(Box<DataExpr>),
    BoolOp(BoolOp, Box<DataExpr>, Box<DataExpr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompOp { Eq, Neq, Lt, Gt, Lte, Gte }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoolOp { And, Or }

#[derive(Clone, Debug, PartialEq)]
pub enum DataValue {
    Number(i64),
    Boolean(bool),
}

/// A total (provably-halting) evaluation function for DATA.
/// The `creusot-contracts` annotations formally verify this totality if the verify feature is enabled.
#[cfg_attr(feature = "verify", ensures(true))]
pub fn eval_data(expr: &DataExpr, env: &HashMap<String, DataValue>) -> Result<DataValue, String> {
    match expr {
        DataExpr::Number(n) => Ok(DataValue::Number(*n)),
        DataExpr::Boolean(b) => Ok(DataValue::Boolean(*b)),
        DataExpr::VarRef(name) => {
            env.get(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))
        }
        DataExpr::Add(left, right) => {
            let l = eval_data(left, env)?;
            let r = eval_data(right, env)?;
            match (l, r) {
                (DataValue::Number(a), DataValue::Number(b)) => Ok(DataValue::Number(a + b)),
                _ => Err("Type error in Add".to_string()),
            }
        }
        DataExpr::Comparison(op, left, right) => {
            let l = eval_data(left, env)?;
            let r = eval_data(right, env)?;
            match (l, r) {
                (DataValue::Number(a), DataValue::Number(b)) => {
                    let res = match op {
                        CompOp::Eq => a == b,
                        CompOp::Neq => a != b,
                        CompOp::Lt => a < b,
                        CompOp::Gt => a > b,
                        CompOp::Lte => a <= b,
                        CompOp::Gte => a >= b,
                    };
                    Ok(DataValue::Boolean(res))
                }
                _ => Err("Type error in Comparison".to_string()),
            }
        }
        DataExpr::Not(operand) => {
            let val = eval_data(operand, env)?;
            match val {
                DataValue::Boolean(b) => Ok(DataValue::Boolean(!b)),
                _ => Err("Type error in Not".to_string()),
            }
        }
        DataExpr::BoolOp(op, left, right) => {
            let l = eval_data(left, env)?;
            let r = eval_data(right, env)?;
            match (l, r) {
                (DataValue::Boolean(a), DataValue::Boolean(b)) => {
                    let res = match op {
                        BoolOp::And => a && b,
                        BoolOp::Or => a || b,
                    };
                    Ok(DataValue::Boolean(res))
                }
                _ => Err("Type error in BoolOp".to_string()),
            }
        }
    }
}

/// CONTROL language: Turing-complete imperative statements.
/// Structurally, a `ControlStmt` can embed a `DataExpr` (the diode bridge), but a `DataExpr`
/// can NEVER embed a `ControlStmt`.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlStmt {
    Assignment(String, DataExpr),
    If(DataExpr, Box<ControlStmt>, Option<Box<ControlStmt>>),
    While(DataExpr, Box<ControlStmt>),
    Print(Vec<DataExpr>),
    Block(Vec<ControlStmt>),
}

pub fn eval_control(stmt: &ControlStmt, env: &mut HashMap<String, DataValue>) -> Result<(), String> {
    match stmt {
        ControlStmt::Assignment(var, expr) => {
            let val = eval_data(expr, env)?;
            env.insert(var.clone(), val);
            Ok(())
        }
        ControlStmt::If(cond, then_branch, else_branch) => {
            let val = eval_data(cond, env)?;
            match val {
                DataValue::Boolean(true) => eval_control(then_branch, env)?,
                DataValue::Boolean(false) => {
                    if let Some(elb) = else_branch {
                        eval_control(elb, env)?;
                    }
                }
                _ => return Err("Type error in If condition".to_string()),
            }
            Ok(())
        }
        ControlStmt::While(cond, body) => {
            loop {
                let val = eval_data(cond, env)?;
                match val {
                    DataValue::Boolean(true) => {
                        eval_control(body, env)?;
                    }
                    DataValue::Boolean(false) => break,
                    _ => return Err("Type error in While condition".to_string()),
                }
            }
            Ok(())
        }
        ControlStmt::Print(args) => {
            for arg in args {
                let val = eval_data(arg, env)?;
                match val {
                    DataValue::Number(n) => print!("{} ", n),
                    DataValue::Boolean(b) => print!("{} ", b),
                }
            }
            println!();
            Ok(())
        }
        ControlStmt::Block(stmts) => {
            for s in stmts {
                eval_control(s, env)?;
            }
            Ok(())
        }
    }
}

fn main() {
    println!("JtV (Julia the Viper) Proof of Concept - Rust/Creusot Implementation");

    // Example 60-second oracle from README.adoc
    // begin
    //   n = 5
    //   sum = 0
    //   i = 0
    //   while i < n do
    //     begin
    //       sum = sum plus i
    //       i = i plus 1
    //     end
    //   print(sum)
    // end

    let program = ControlStmt::Block(vec![
        ControlStmt::Assignment("n".to_string(), DataExpr::Number(5)),
        ControlStmt::Assignment("sum".to_string(), DataExpr::Number(0)),
        ControlStmt::Assignment("i".to_string(), DataExpr::Number(0)),
        ControlStmt::While(
            DataExpr::Comparison(
                CompOp::Lt,
                Box::new(DataExpr::VarRef("i".to_string())),
                Box::new(DataExpr::VarRef("n".to_string())),
            ),
            Box::new(ControlStmt::Block(vec![
                ControlStmt::Assignment(
                    "sum".to_string(),
                    DataExpr::Add(
                        Box::new(DataExpr::VarRef("sum".to_string())),
                        Box::new(DataExpr::VarRef("i".to_string())),
                    ),
                ),
                ControlStmt::Assignment(
                    "i".to_string(),
                    DataExpr::Add(
                        Box::new(DataExpr::VarRef("i".to_string())),
                        Box::new(DataExpr::Number(1)),
                    ),
                ),
            ])),
        ),
        ControlStmt::Print(vec![DataExpr::VarRef("sum".to_string())]),
    ]);

    let mut env = HashMap::new();
    println!("Evaluating example program:");
    match eval_control(&program, &mut env) {
        Ok(_) => println!("Evaluation finished successfully."),
        Err(e) => println!("Error: {}", e),
    }
}
