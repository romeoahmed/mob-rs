// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use super::TaskRegistry;
use crate::config::types::Aliases;

fn create_test_registry() -> TaskRegistry {
    let mut aliases: Aliases = BTreeMap::new();
    aliases.insert(
        "all".to_string(),
        vec!["usvfs".to_string(), "modorganizer".to_string()],
    );
    aliases.insert(
        "mo".to_string(),
        vec![
            "modorganizer".to_string(),
            "modorganizer-archive".to_string(),
        ],
    );
    aliases.insert(
        "nested".to_string(),
        vec!["all".to_string(), "cmake".to_string()],
    );

    let mut registry = TaskRegistry::new(aliases);
    registry.register_all([
        "usvfs",
        "modorganizer",
        "modorganizer-archive",
        "modorganizer-plugins",
        "cmake",
        "python",
    ]);
    registry
}

#[test]
fn test_registry_register() {
    let mut registry = TaskRegistry::new(BTreeMap::new());
    registry.register("test-task");

    assert!(registry.all_tasks().contains("test-task"));
}

#[test]
fn test_registry_register_all() {
    let mut registry = TaskRegistry::new(BTreeMap::new());
    registry.register_all(["task1", "task2", "task3"]);

    let tasks: Vec<_> = registry.all_tasks().iter().cloned().collect();
    insta::assert_yaml_snapshot!("registry_register_all", tasks);
}

#[test]
fn test_resolve_aliases_simple() {
    let registry = create_test_registry();

    let result = registry.resolve_aliases(&["mo".to_string()]);
    insta::assert_yaml_snapshot!("test_resolve_aliases_simple", result);
}

#[test]
fn test_resolve_aliases_nested() {
    let registry = create_test_registry();

    let result = registry.resolve_aliases(&["nested".to_string()]);
    // "nested" -> ["all", "cmake"] -> ["usvfs", "modorganizer", "cmake"]
    insta::assert_yaml_snapshot!("test_resolve_aliases_nested", result);
}

#[test]
fn test_resolve_aliases_non_alias() {
    let registry = create_test_registry();
    let result = registry.resolve_aliases(&["python".to_string()]);
    insta::assert_yaml_snapshot!("resolve_aliases_non_alias", result);
}

#[test]
fn test_match_pattern_exact() {
    let registry = create_test_registry();

    let result = registry.match_pattern("usvfs").unwrap();
    insta::assert_yaml_snapshot!("test_match_pattern_exact", result);
}

#[test]
fn test_match_pattern_glob() {
    let registry = create_test_registry();

    let result = registry.match_pattern("modorganizer*").unwrap();
    // BTreeSet iteration is sorted, so result should be sorted
    insta::assert_yaml_snapshot!("test_match_pattern_glob", result);
}

#[test]
fn test_match_pattern_all() {
    let registry = create_test_registry();
    let result = registry.match_pattern("*").unwrap();
    insta::assert_yaml_snapshot!("match_pattern_all", result);
}

#[test]
fn test_match_pattern_no_match() {
    let registry = create_test_registry();

    let result = registry.match_pattern("nonexistent").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_resolve_full() {
    let registry = create_test_registry();

    // Mix of alias and pattern
    let result = registry
        .resolve(&["mo".to_string(), "python".to_string()])
        .unwrap();
    insta::assert_yaml_snapshot!("test_resolve_full", result);
}

#[test]
fn test_resolve_deduplicates() {
    let registry = create_test_registry();

    // "all" includes "modorganizer", and we also specify it directly
    let result = registry
        .resolve(&["all".to_string(), "modorganizer".to_string()])
        .unwrap();
    // Should contain "modorganizer" only once (deduplicated)
    insta::assert_yaml_snapshot!("resolve_deduplicates", result);
}
