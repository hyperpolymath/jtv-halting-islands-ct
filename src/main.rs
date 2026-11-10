//! JtV command-line runner.
//!
//! ```text
//! jtv-halting-islands-ct <file> --mode data|control
//! ```
//!
//! * `--mode data`: parse the whole file as one DATA expression, print its value.
//! * `--mode control`: parse the file as a CONTROL program and run it.
//!
//! Exit status: 0 on success, 1 on a lex/parse/evaluation error (including a
//! diode violation) or an error writing to stdout, 2 on a usage error or an
//! unreadable file.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use jtv_halting_islands_ct::{eval_control, eval_data, parse_control_program, parse_data_program};

const USAGE: &str = "usage: jtv-halting-islands-ct <file> --mode data|control";

#[derive(Clone, Copy)]
enum Mode {
    Data,
    Control,
}

/// Parse the arguments as `OsString`s, so a file name that is not valid
/// UTF-8 reaches `read_to_string` intact instead of panicking in
/// `std::env::args`. Only option names and the mode value must be UTF-8.
fn parse_args(args: &[OsString]) -> Result<(PathBuf, Mode), String> {
    let mut file = None;
    let mut mode = None;
    let mut i = 0;
    while i < args.len() {
        let raw = &args[i];
        let arg = raw.to_str();
        let value = match arg {
            Some("--mode") => {
                i += 1;
                let v = args.get(i).ok_or("--mode needs a value")?;
                Some(v.to_str().ok_or_else(|| format!("unknown mode {v:?}"))?)
            }
            Some(a) => a.strip_prefix("--mode="),
            None => None,
        };
        match value {
            Some("data") => mode = Some(Mode::Data),
            Some("control") => mode = Some(Mode::Control),
            Some(other) => return Err(format!("unknown mode {other:?}")),
            None if arg.is_some_and(|a| a.starts_with("--")) => {
                return Err(format!("unknown option {raw:?}"))
            }
            None if file.is_none() => file = Some(PathBuf::from(raw)),
            None => return Err(format!("unexpected extra argument {raw:?}")),
        }
        i += 1;
    }
    Ok((file.ok_or("missing <file>")?, mode.ok_or("missing --mode")?))
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (path, mode) = match parse_args(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {path:?}: {e}");
            return ExitCode::from(2);
        }
    };

    // Write through a locked handle and propagate write and flush errors
    // (e.g. a closed pipe), instead of `println!`, which panics on them.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let result = match mode {
        Mode::Data => parse_data_program(&source)
            .map_err(|e| e.to_string())
            .and_then(|expr| {
                eval_data(&expr, &HashMap::new()).map_err(|e| format!("evaluation error: {e}"))
            })
            .and_then(|v| writeln!(out, "{v}").map_err(|e| format!("write error: {e}"))),
        Mode::Control => parse_control_program(&source)
            .map_err(|e| e.to_string())
            .and_then(|prog| {
                eval_control(&prog, &mut HashMap::new(), &mut out)
                    .map_err(|e| format!("evaluation error: {e}"))
            }),
    };
    // Flush on every path, so output printed before an error is not lost.
    let flushed = out.flush().map_err(|e| format!("write error: {e}"));
    let result = result.and(flushed);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
