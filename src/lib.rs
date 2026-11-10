// SPDX-License-Identifier: MPL-2.0
//! JtV (Julia the Viper): a Harvard-architecture language with two
//! grammatically sealed sublanguages.
//!
//! * DATA ([`data`]): addition-only expressions over `i64` and booleans. No
//!   recursion, no loops, no conditionals.
//! * CONTROL ([`control`]): Turing-complete and imperative.
//!
//! The diode: CONTROL can evaluate a [`DataExpr`] and bind its value, but a
//! [`DataExpr`] cannot contain a [`ControlStmt`]. Two layers enforce this.
//!
//! 1. **Grammar.** The DATA parser has no production for any control token,
//!    so control text in a DATA position fails to parse. Tested in
//!    `tests/diode_negative.rs`.
//! 2. **Types.** `DataExpr` has no variant that can hold a `ControlStmt`, so a
//!    hand-built AST that tries to does not compile. The doctests below check
//!    this. Each `compile_fail` example is paired with an identical example
//!    that compiles once the `ControlStmt` is replaced by a `DataExpr`, so the
//!    failure is attributable to the type mismatch and not to some unrelated
//!    mistake in the snippet. (Stable rustdoc does not check the error code
//!    of a `compile_fail` test, which is why the pairing matters.)
//!
//! Control inside `Add` does not compile:
//!
//! ```compile_fail
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let _ = DataExpr::Add(
//!     Box::new(DataExpr::Number(1)),
//!     Box::new(ControlStmt::Print(vec![])),
//! );
//! ```
//!
//! The same shape with a DATA operand compiles:
//!
//! ```
//! use jtv_halting_islands_ct::DataExpr;
//! let _ = DataExpr::Add(
//!     Box::new(DataExpr::Number(1)),
//!     Box::new(DataExpr::Number(2)),
//! );
//! ```
//!
//! Control inside `Not` does not compile:
//!
//! ```compile_fail
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let _ = DataExpr::Not(Box::new(ControlStmt::Block(vec![])));
//! ```
//!
//! ```
//! use jtv_halting_islands_ct::DataExpr;
//! let _ = DataExpr::Not(Box::new(DataExpr::Boolean(true)));
//! ```
//!
//! A control statement in an assignment's DATA position does not compile:
//!
//! ```compile_fail
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let _ = ControlStmt::Assignment("x".to_string(), ControlStmt::Block(vec![]));
//! ```
//!
//! ```
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let _ = ControlStmt::Assignment("x".to_string(), DataExpr::Number(1));
//! ```
//!
//! A control statement as a `while` condition does not compile:
//!
//! ```compile_fail
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let body = ControlStmt::Block(vec![]);
//! let _ = ControlStmt::While(ControlStmt::Block(vec![]), Box::new(body));
//! ```
//!
//! ```
//! use jtv_halting_islands_ct::{ControlStmt, DataExpr};
//! let body = ControlStmt::Block(vec![]);
//! let _ = ControlStmt::While(DataExpr::Boolean(false), Box::new(body));
//! ```
//!
//! **What is not claimed.** Nothing in this crate is machine-verified. The
//! argument that DATA evaluation halts is informal (see
//! [`data::eval_data`]) and is backed by tests, not by a proof tool.

pub mod control;
pub mod data;
pub mod lexer;

pub use control::{
    eval_control, parse_control_program, ControlParseError, ControlStmt, ParseError,
    MAX_CONTROL_NESTING,
};
pub use data::{
    eval_data, parse_data_program, BoolOp, CompOp, DataExpr, DataParseError, DataSourceError,
    DataValue, Env, MAX_DATA_DEPTH, MAX_DATA_NESTING,
};
pub use lexer::LexError;
