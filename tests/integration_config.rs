// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for configuration loading.
//!
//! Tests the Config module with realistic TOML configurations.

use mob_rs::config::Config;

// =============================================================================
// Loading from TOML strings
// =============================================================================

#[test]
fn config_parse_minimal() {
    let toml = r#"
[paths]
prefix = "/build"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_parse_global_section() {
    let toml = r"
[global]
dry = true
redownload = true
output_log_level = 5
";
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_parse_task_section() {
    let toml = r#"
[task]
mo_org = "MyOrg"
mo_branch = "develop"
configuration = "Debug"
git_shallow = false
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_parse_tools_section() {
    let toml = r#"
[tools]
cmake = "/opt/cmake/bin/cmake"
msbuild = "/usr/bin/msbuild"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_parse_versions_section() {
    let toml = r#"
[versions]
vs_toolset = "14.4"
sdk = "10.0.22621.0"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

// =============================================================================
// Task-Specific Overrides
// =============================================================================

#[test]
fn config_task_specific_override() {
    let toml = r#"
[task]
git_shallow = true
configuration = "RelWithDebInfo"

[tasks.usvfs]
git_shallow = false
configuration = "Release"

[tasks.cmake_common]
configuration = "Debug"
"#;
    let config = Config::parse(toml).unwrap();

    insta::assert_yaml_snapshot!(serde_json::json!({
        "base_config": config,
        "usvfs_config": config.task_config("usvfs"),
        "cmake_config": config.task_config("cmake_common"),
        "other_task_config": config.task_config("other_task"),
    }));
}

// =============================================================================
// Path Resolution
// =============================================================================

#[test]
fn config_paths_resolve() {
    let toml = r#"
[paths]
prefix = "/test/prefix"
"#;
    let mut config = Config::parse(toml).unwrap();
    config.paths.resolve().unwrap();

    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_paths_resolve_with_custom_paths() {
    let toml = r#"
[paths]
prefix = "/test/prefix"
cache = "/custom/cache"
install = "/custom/install"
"#;
    let mut config = Config::parse(toml).unwrap();
    config.paths.resolve().unwrap();

    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_paths_missing_prefix_error() {
    let toml = r"
[global]
dry = true
";
    let mut config = Config::parse(toml).unwrap();
    let result = config.paths.resolve();
    assert!(result.is_err());
}

// =============================================================================
// Builder Pattern
// =============================================================================

#[test]
fn config_builder_layered() {
    // Base layer
    let config = Config::builder()
        .add_toml_str(
            r#"
[global]
dry = false
output_log_level = 3

[task]
mo_org = "BaseOrg"
"#,
        )
        // Override layer
        .add_toml_str(
            r#"
[global]
dry = true

[task]
mo_branch = "feature"
"#,
        )
        .build()
        .unwrap();

    insta::assert_yaml_snapshot!(config);
}

#[test]
fn config_builder_set_override() {
    let config = Config::builder()
        .add_toml_str(
            r"
[global]
dry = false
",
        )
        .set("global.dry", true)
        .unwrap()
        .build()
        .unwrap();

    insta::assert_yaml_snapshot!(config);
}

// =============================================================================
// Aliases
// =============================================================================

#[test]
fn config_aliases() {
    let toml = r#"
[aliases]
super = ["modorganizer*", "usvfs"]
plugins = ["plugin_*"]
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

// =============================================================================
// Default Values
// =============================================================================

#[test]
fn config_default_values() {
    let config = Config::default();

    insta::assert_yaml_snapshot!(config);
}

// =============================================================================
// Transifex Configuration
// =============================================================================

#[test]
fn config_transifex() {
    let toml = r#"
[transifex]
enabled = true
team = "my-team"
project = "my-project"
minimum = 80
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config);
}

// =============================================================================
// CMake Configuration
// =============================================================================

#[test]
fn config_cmake_defaults() {
    let toml = r#"
[paths]
prefix = "/build"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config.cmake);
}

#[test]
fn config_cmake_install_message_variants() {
    for variant in ["always", "lazy", "never"] {
        let toml = format!(
            r#"
[cmake]
install_message = "{variant}"
"#
        );
        let config = Config::parse(&toml).unwrap();
        insta::assert_yaml_snapshot!(
            format!("config_cmake_install_message_{variant}"),
            config.cmake
        );
    }
}

#[test]
fn config_cmake_host() {
    let toml = r#"
[cmake]
host = "x64"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config.cmake);
}

#[test]
fn config_cmake_full() {
    let toml = r#"
[cmake]
install_message = "lazy"
host = "x64"
"#;
    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(config.cmake);
}
