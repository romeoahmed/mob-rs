// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::builder::{ProcessBuilder, ProcessFlags};
use crate::core::env::container::Env;

#[tokio::test]
async fn test_process_echo() {
    // Use Write-Output in PowerShell, echo in Unix shell
    #[cfg(windows)]
    let output = ProcessBuilder::raw("Write-Output 'hello'")
        .capture_output()
        .run()
        .await
        .expect("echo should succeed");

    #[cfg(not(windows))]
    let output = ProcessBuilder::new("echo")
        .arg("hello")
        .capture_output()
        .run()
        .await
        .expect("echo should succeed");

    assert!(output.success());
    insta::assert_snapshot!(output.stdout().trim());
}

#[tokio::test]
async fn test_process_exit_code() {
    let output = ProcessBuilder::raw("exit 42")
        .flag(ProcessFlags::ALLOW_FAILURE)
        .run()
        .await
        .expect("process should complete");

    insta::assert_snapshot!(output.exit_code().to_string());
}

#[tokio::test]
async fn test_process_env() {
    let mut env = Env::new();
    env.set("TEST_VAR", "test_value");

    // PowerShell uses $env:VAR syntax, Unix uses $VAR
    #[cfg(windows)]
    let output = ProcessBuilder::raw("Write-Output $env:TEST_VAR")
        .env(env)
        .capture_stdout()
        .run()
        .await
        .expect("process should succeed");

    #[cfg(not(windows))]
    let output = ProcessBuilder::raw("echo $TEST_VAR")
        .env(env)
        .capture_stdout()
        .run()
        .await
        .expect("process should succeed");

    insta::assert_snapshot!(output.stdout().trim());
}

#[test]
fn test_executable_lookup_found() {
    // cargo should always be available since we're running tests with cargo
    // Test which() - returns Result<ProcessBuilder>
    let which_result = ProcessBuilder::which("cargo");
    assert!(which_result.is_ok(), "which: cargo should be found in PATH");
    let builder = which_result.unwrap();
    assert!(
        builder.program().exists(),
        "which: returned program path should exist"
    );
    assert!(
        builder
            .program()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("cargo"),
        "which: should find cargo executable"
    );

    // Test exists() - returns bool
    assert!(
        ProcessBuilder::exists("cargo"),
        "exists: cargo should exist in PATH"
    );

    // Test find() - returns Option<PathBuf>
    let find_result = ProcessBuilder::find("cargo");
    assert!(find_result.is_some(), "find: cargo should be found");
    let path = find_result.unwrap();
    assert!(path.exists(), "find: returned path should exist");

    // Test find_all() - returns iterator
    let find_all_results: Vec<_> = ProcessBuilder::find_all("cargo").collect();
    assert!(
        !find_all_results.is_empty(),
        "find_all: should find at least one cargo"
    );
    for path in &find_all_results {
        assert!(path.exists(), "find_all: all returned paths should exist");
    }
}

#[test]
fn test_executable_lookup_not_found() {
    let program = "nonexistent_program_12345";

    // Test which() - returns error
    let which_result = ProcessBuilder::which(program);
    assert!(
        which_result.is_err(),
        "which: nonexistent program should not be found"
    );
    let err_msg = format!("{}", which_result.unwrap_err());
    assert!(
        err_msg.contains("not found") || err_msg.contains(program),
        "which: error should mention the program: {err_msg}"
    );

    // Test exists() - returns false
    assert!(
        !ProcessBuilder::exists(program),
        "exists: nonexistent program should not exist"
    );

    // Test find() - returns None
    let find_result = ProcessBuilder::find(program);
    assert!(
        find_result.is_none(),
        "find: nonexistent program should return None"
    );

    // Test find_all() - returns empty iterator
    assert!(
        ProcessBuilder::find_all(program).next().is_none(),
        "find_all: should find no matches"
    );
}
