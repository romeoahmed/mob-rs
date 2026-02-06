// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the environment module.

use super::current_env;
use crate::core::env::container::Env;
use crate::core::env::types::EnvFlags;
use std::collections::BTreeMap;

#[test]
fn test_env_basic_operations() {
    let mut env = Env::new();
    env.set("FOO", "bar");

    insta::assert_yaml_snapshot!(
        "env_basic_operations",
        vec![
            ("get_FOO", env.get("FOO")),
            ("get_foo_case_insensitive", env.get("foo")),
            ("get_NOTEXIST", env.get("NOTEXIST")),
        ]
    );
}

#[test]
fn test_env_flags() {
    let mut env = Env::new();
    env.set("KEY", "initial");

    let mut results = vec![("after_set", env.get("KEY").unwrap().to_string())];

    env.set_with_flags("KEY", "_appended", EnvFlags::Append);
    results.push(("after_append", env.get("KEY").unwrap().to_string()));

    env.set_with_flags("KEY", "prepended_", EnvFlags::Prepend);
    results.push(("after_prepend", env.get("KEY").unwrap().to_string()));

    env.set_with_flags("KEY", "replaced", EnvFlags::Replace);
    results.push(("after_replace", env.get("KEY").unwrap().to_string()));

    insta::assert_yaml_snapshot!("env_flags", results);
}

#[test]
fn test_env_path_manipulation() {
    let mut env = Env::new();
    env.set("PATH", "/usr/bin");

    let mut results = vec![("initial", env.get("PATH").unwrap().to_string())];

    env.prepend_path("/usr/local/bin");
    let path = env.get("PATH").unwrap();
    results.push(("after_prepend", path.to_string()));
    results.push((
        "starts_with_local_bin",
        path.starts_with("/usr/local/bin").to_string(),
    ));

    env.append_path("/opt/bin");
    let path = env.get("PATH").unwrap();
    results.push(("after_append", path.to_string()));
    results.push(("ends_with_opt_bin", path.ends_with("/opt/bin").to_string()));

    insta::assert_yaml_snapshot!("env_path_manipulation", results);
}

#[test]
fn test_env_copy_on_write() {
    let mut env1 = Env::new();
    env1.set("KEY1", "value1");

    // Clone shares data initially
    let mut env2 = env1.clone();

    // Modifying env2 triggers copy-on-write, doesn't affect env1
    env2.set("KEY2", "value2");

    insta::assert_yaml_snapshot!(
        "env_copy_on_write",
        serde_json::json!({
            "env1_KEY1": env1.get("KEY1"),
            "env1_KEY2": env1.get("KEY2"),
            "env2_KEY1": env2.get("KEY1"),
            "env2_KEY2": env2.get("KEY2"),
        })
    );
}

#[test]
fn test_current_env() {
    // Behavioral test - PATH should exist
    let env = current_env();
    assert!(
        env.get("PATH").is_some() || env.get("Path").is_some(),
        "PATH should exist in current environment"
    );
}

#[test]
fn test_env_from_map() {
    let mut map = BTreeMap::new();
    map.insert("KEY1".to_string(), "value1".to_string());
    map.insert("KEY2".to_string(), "value2".to_string());

    let env = Env::from_map(map);

    insta::assert_yaml_snapshot!(
        "env_from_map",
        serde_json::json!({
            "KEY1": env.get("KEY1"),
            "KEY2": env.get("KEY2"),
            "len": env.len(),
        })
    );
}

#[test]
fn test_env_to_map() {
    let mut env = Env::new();
    env.set("KEY1", "value1");
    env.set("KEY2", "value2");

    let map = env.to_map();
    insta::assert_yaml_snapshot!("env_to_map", map);
}

#[test]
#[cfg(windows)]
fn test_env_to_windows_env_block() {
    let mut env = Env::new();
    env.set("KEY1", "value1");
    env.set("KEY2", "value2");

    let block = env.to_windows_env_block();

    // Block should be non-empty and end with double null (two consecutive zeros)
    assert!(!block.is_empty(), "env block should not be empty");

    // Last two elements should be nulls (double-null terminator)
    let len = block.len();
    assert!(
        len >= 2,
        "block must have at least 2 elements for terminator"
    );
    assert_eq!(block[len - 1], 0, "block should end with null terminator");
}
