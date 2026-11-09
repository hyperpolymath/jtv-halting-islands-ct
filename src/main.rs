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
//! diode violation), 2 on a usage error or an unreadable file.

use std::collections::HashMap;
use std::io::Write;
use std::process::ExitCode;

use jtv_halting_islands_ct::{eval_control, eval_data, parse_control_program, parse_data_program};

const USAGE: &str = "usage: jtv-halting-islands-ct <file> --mode data|control";

#[derive(Clone, Copy)]
enum Mode {
    Data,
    Control,
}

fn parse_args(args: &[String]) -> Result<(String, Mode), String> {
    let mut file = None;
    let mut mode = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let value = if arg == "--mode" {
            i += 1;
            Some(args.get(i).ok_or("--mode needs a value")?.as_str())
        } else {
            arg.strip_prefix("--mode=")
        };
        match value {
            Some("data") => mode = Some(Mode::Data),
            Some("control") => mode = Some(Mode::Control),
            Some(other) => return Err(format!("unknown mode {other:?}")),
            None if arg.starts_with("--") => return Err(format!("unknown option {arg:?}")),
            None if file.is_none() => file = Some(arg.to_string()),
            None => return Err(format!("unexpected extra argument {arg:?}")),
        }
        i += 1;
    }
    Ok((file.ok_or("missing <file>")?, mode.ok_or("missing --mode")?))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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

    let result = match mode {
        Mode::Data => parse_data_program(&source)
            .map_err(|e| e.to_string())
            .and_then(|expr| {
                eval_data(&expr, &HashMap::new()).map_err(|e| format!("evaluation error: {e}"))
            })
            .map(|v| println!("{v}")),
        Mode::Control => parse_control_program(&source)
            .map_err(|e| e.to_string())
            .and_then(|prog| {
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                let r = eval_control(&prog, &mut HashMap::new(), &mut out)
                    .map_err(|e| format!("evaluation error: {e}"));
                let _ = out.flush();
                r
            }),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
