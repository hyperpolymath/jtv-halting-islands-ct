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
