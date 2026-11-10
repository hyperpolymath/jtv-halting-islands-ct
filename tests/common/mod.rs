// SPDX-License-Identifier: MPL-2.0
//! Shared helpers for the integration tests (included with `mod common;`).
#![allow(dead_code)]

use std::path::PathBuf;

pub fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

pub fn example_source(name: &str) -> String {
    std::fs::read_to_string(repo_path(&format!("examples/{name}"))).expect("read example")
}
