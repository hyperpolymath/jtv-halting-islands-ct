// SPDX-License-Identifier: MPL-2.0
//! The binary must read the file it is given. Before this test existed the
//! binary ignored its argument and ran a built-in copy of `oracle.jtv`.

mod common;

use std::process::{Command, Output};

fn jtv(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jtv-halting-islands-ct"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run binary")
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

#[test]
fn runs_the_fixture_file_it_is_given() {
    let o = jtv(&["tests/fixtures/count_to_four.jtv", "--mode", "control"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "1, 1\n2, 3\n3, 6\n4, 10\ntrue, 10\n");
}

#[test]
fn reads_an_edited_copy_of_a_file() {
    // Write a file at test time, so its content cannot be built in.
    let dir = std::env::temp_dir().join(format!("jtv-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fresh.jtv");
    std::fs::write(&path, "print(1234 plus 1)\n").unwrap();
    let o = jtv(&[path.to_str().unwrap(), "--mode=control"]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "1235\n");
}

#[test]
fn oracle_example_prints_ten() {
    let o = jtv(&["examples/oracle.jtv", "--mode", "control"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "10\n");
}

#[test]
fn data_mode_evaluates_pure_data_example() {
    let o = jtv(&["examples/pure_data.jtv", "--mode", "data"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "42\n");
}

#[test]
fn violation_example_is_rejected_by_the_diode() {
    let o = jtv(&["examples/violation.jtv", "--mode", "control"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stdout(&o).is_empty());
    let err = stderr(&o);
    assert!(
        err.contains("DATA parse error") && err.contains("diode"),
        "{err}"
    );
}

#[test]
fn missing_file_is_a_usage_level_error() {
    let o = jtv(&["tests/fixtures/does-not-exist.jtv", "--mode", "control"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("cannot read"));
}

#[test]
fn missing_mode_or_file_is_a_usage_error() {
    assert_eq!(jtv(&["examples/oracle.jtv"]).status.code(), Some(2));
    assert_eq!(jtv(&["--mode", "data"]).status.code(), Some(2));
    assert_eq!(
        jtv(&["examples/oracle.jtv", "--mode", "fast"])
            .status
            .code(),
        Some(2)
    );
}

/// Write `contents` to a fresh file under the temp dir and return its path.
fn temp_source(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("jtv-cli-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.jtv");
    std::fs::write(&path, contents).unwrap();
    path
}

fn run_source(name: &str, contents: &str, mode: &str) -> Output {
    let path = temp_source(name, contents);
    let o = jtv(&[path.to_str().unwrap(), "--mode", mode]);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
    o
}

#[test]
fn lexical_errors_exit_with_status_one() {
    for (name, mode, source) in [
        ("lex-data", "data", "1 $ 2"),
        ("lex-control", "control", "x = 1 $ 2"),
    ] {
        let o = run_source(name, source, mode);
        assert_eq!(o.status.code(), Some(1), "{mode}: {}", stderr(&o));
        assert!(stdout(&o).is_empty());
        assert!(stderr(&o).starts_with("error: "), "{}", stderr(&o));
    }
}

#[test]
fn evaluation_errors_exit_with_status_one() {
    for (name, mode, source) in [
        ("eval-data", "data", "9223372036854775807 plus 1"),
        ("eval-control", "control", "while 1 do x = 1"),
    ] {
        let o = run_source(name, source, mode);
        assert_eq!(o.status.code(), Some(1), "{mode}: {}", stderr(&o));
        assert!(stdout(&o).is_empty());
        assert!(stderr(&o).contains("evaluation error"), "{}", stderr(&o));
    }
}

#[test]
fn over_deep_input_is_a_parse_error_not_a_crash() {
    let chain = vec!["0"; 200_000].join(" + ");
    let o = run_source("deep-data", &chain, "data");
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("too deep"), "{}", stderr(&o));

    let nested = format!(
        "{}x = 1{}",
        "begin ".repeat(200_000),
        " end".repeat(200_000)
    );
    let o = run_source("deep-control", &nested, "control");
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("nested too deeply"), "{}", stderr(&o));
}

/// A write failure on stdout is reported with status 1, not a panic (101).
/// `/dev/full` fails every write with ENOSPC.
#[cfg(target_os = "linux")]
#[test]
fn stdout_write_failure_exits_with_status_one() {
    for (example, mode) in [
        ("examples/pure_data.jtv", "data"),
        ("examples/oracle.jtv", "control"),
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_jtv-halting-islands-ct"))
            .args([example, "--mode", mode])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(std::fs::File::create("/dev/full").expect("open /dev/full"))
            .output()
            .expect("run binary");
        assert_eq!(o.status.code(), Some(1), "{mode}: {}", stderr(&o));
        assert!(stderr(&o).contains("write error"), "{}", stderr(&o));
    }
}

/// A file name that is not valid UTF-8 is read, not a panic in argument
/// parsing. Linux file systems accept arbitrary non-NUL bytes in names.
#[cfg(target_os = "linux")]
#[test]
fn non_utf8_file_name_is_read() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = std::env::temp_dir().join(format!("jtv-cli-test-{}-non-utf8", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(OsStr::from_bytes(b"input-\xff.jtv"));
    std::fs::write(&path, "print(41 plus 1)\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_jtv-halting-islands-ct"))
        .arg(&path)
        .args(["--mode", "control"])
        .output()
        .expect("run binary");
    std::fs::remove_dir_all(&dir).ok();
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "42\n");
}
