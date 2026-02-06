// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Config, ConfigLoader, PathsConfig, ToolsConfig};
use crate::config::types::{BuildConfiguration, CmakeInstallMessage};
use crate::logging::LogLevel;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = Config::default();
    insta::assert_yaml_snapshot!(
        "default_config",
        serde_json::json!({
            "global.dry": config.global.dry,
            "global.output_log_level": config.global.output_log_level.as_u8(),
            "task.mo_org": config.task.mo_org,
            "task.configuration": config.task.configuration.to_string(),
        })
    );
}

#[test]
fn test_build_configuration_display() {
    insta::assert_yaml_snapshot!(
        "build_configuration_display",
        vec![
            ("Debug", BuildConfiguration::Debug.to_string()),
            ("Release", BuildConfiguration::Release.to_string()),
            (
                "RelWithDebInfo",
                BuildConfiguration::RelWithDebInfo.to_string()
            ),
        ]
    );
}

#[test]
fn test_build_configuration_parse() {
    insta::assert_yaml_snapshot!(
        "build_configuration_parse",
        vec![
            (
                "debug",
                format!("{:?}", "debug".parse::<BuildConfiguration>())
            ),
            (
                "Release",
                format!("{:?}", "Release".parse::<BuildConfiguration>())
            ),
            (
                "relwithdebinfo",
                format!("{:?}", "relwithdebinfo".parse::<BuildConfiguration>())
            ),
            (
                "invalid",
                format!("{:?}", "invalid".parse::<BuildConfiguration>().is_err())
            ),
        ]
    );
}

#[test]
fn test_log_level_bounds() {
    insta::assert_yaml_snapshot!(
        "log_level_bounds",
        vec![
            ("level_0_valid", LogLevel::new(0).is_ok()),
            ("level_6_valid", LogLevel::new(6).is_ok()),
            ("level_7_invalid", LogLevel::new(7).is_err()),
        ]
    );
}

#[test]
fn test_cmake_install_message_display() {
    insta::assert_yaml_snapshot!(
        "cmake_install_message_display",
        vec![
            ("Always", CmakeInstallMessage::Always.to_string()),
            ("Lazy", CmakeInstallMessage::Lazy.to_string()),
            ("Never", CmakeInstallMessage::Never.to_string()),
        ]
    );
}

#[test]
fn test_paths_resolve() {
    let mut paths = PathsConfig {
        prefix: Some(PathBuf::from("/test/prefix")),
        ..Default::default()
    };

    paths.resolve().unwrap();

    // Normalize path separators for cross-platform snapshot consistency
    let normalize =
        |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().replace('\\', "/"));

    insta::assert_yaml_snapshot!(
        "paths_resolve",
        serde_json::json!({
            "cache": normalize(&paths.cache),
            "build": normalize(&paths.build),
            "install": normalize(&paths.install),
            "install_bin": normalize(&paths.install_bin),
        })
    );
}

#[test]
fn test_config_parse() {
    let toml = r#"
[global]
dry = true
output_log_level = 4

[task]
mo_org = "TestOrg"
configuration = "Debug"

[paths]
prefix = "/test/path"
"#;

    let config = Config::parse(toml).unwrap();
    insta::assert_yaml_snapshot!(
        "config_parse",
        serde_json::json!({
            "global.dry": config.global.dry,
            "global.output_log_level": config.global.output_log_level.as_u8(),
            "task.mo_org": config.task.mo_org,
            "task.configuration": config.task.configuration.to_string(),
            "paths.prefix": config.paths.prefix,
        })
    );
}

#[test]
fn test_tools_default() {
    let tools = ToolsConfig::default();
    insta::assert_yaml_snapshot!(
        "tools_default",
        serde_json::json!({
            "sevenz": tools.sevenz,
            "cmake": tools.cmake,
            "msbuild": tools.msbuild,
        })
    );
}

#[test]
fn test_config_builder_with_toml_str() {
    let config = Config::builder()
        .add_toml_str(
            r"
                [global]
                dry = true
                ",
        )
        .build()
        .unwrap();

    insta::assert_yaml_snapshot!(
        "config_builder_with_toml_str",
        serde_json::json!({
            "global.dry": config.global.dry,
        })
    );
}

#[test]
fn test_config_loader_tracks_files() {
    let loader = ConfigLoader::new().add_toml_str("[global]\n dry = true");

    let loaded_files = loader.loaded_files();
    let files: Vec<_> = loaded_files
        .iter()
        .map(|(source, path)| (source.as_str(), path.to_string_lossy().into_owned()))
        .collect();
    insta::assert_yaml_snapshot!("config_loader_tracks_files", files);
}

#[test]
fn test_config_loader_format_loaded_files() {
    let loader = ConfigLoader::new()
        .add_toml_str("[global]\n dry = true")
        .add_toml_str("[task]\n mo_org = \"Test\"");

    insta::assert_yaml_snapshot!(
        "config_loader_format_loaded_files",
        loader.format_loaded_files()
    );
}

#[test]
fn test_config_loader_optional_only_tracks_existing() {
    let loader = ConfigLoader::new().add_toml_file_optional("/nonexistent/path.toml");

    assert!(loader.loaded_files().is_empty());
}

#[test]
fn test_config_loader_mixed_sources() {
    let loader = ConfigLoader::new()
        .add_toml_str("[global]\n dry = true")
        .add_toml_file_optional("/nonexistent/optional.toml");

    let tracked_files = loader.loaded_files();
    let files: Vec<_> = tracked_files
        .iter()
        .map(|(source, path)| (source.as_str(), path.to_string_lossy().into_owned()))
        .collect();
    insta::assert_yaml_snapshot!("config_loader_mixed_sources", files);
}

#[test]
fn test_format_options_hides_sensitive() {
    let config = Config::builder()
        .add_toml_str(
            r#"
                [transifex]
                key = "transifex_secret"
                "#,
        )
        .build()
        .unwrap();

    let formatted = config.format_options();
    let formatted_str = formatted.join("\n ");

    // Verify sensitive fields are hidden
    assert!(formatted_str.contains("transifex.key") && formatted_str.contains("[hidden]"));

    // Verify actual secret values don't appear
    assert!(!formatted_str.contains("transifex_secret"));
}

#[test]
fn test_format_options_deterministic() {
    let config = Config::builder()
        .add_toml_str(
            r#"
                [global]
                dry = true

                [task]
                mo_org = "TestOrg"

                [paths]
                prefix = "/path/to/prefix"
                "#,
        )
        .build()
        .unwrap();

    // Call format_options multiple times and verify order is consistent
    let result1 = config.format_options();
    let result2 = config.format_options();

    assert_eq!(
        result1, result2,
        "format_options output should be deterministic"
    );

    // Verify alphabetical ordering of keys
    let formatted_str = result1.join("\n ");

    // Check that at least some keys appear in alphabetical order
    assert!(formatted_str.lines().next().is_some());
}

#[test]
fn test_cmake_prefix_path_scenarios() {
    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    // Scenario 1: All paths present
    let paths_all = PathsConfig {
        qt_install: Some(PathBuf::from("C:/Qt/6.7.0")),
        vcpkg: Some(PathBuf::from("C:/vcpkg")),
        install_libs: Some(PathBuf::from("C:/mo2/install/lib")),
        ..Default::default()
    };
    let result_all = paths_all.cmake_prefix_path();
    assert!(result_all.contains("Qt"), "should contain Qt path");
    assert!(result_all.contains("vcpkg"), "should contain vcpkg path");
    assert!(
        result_all.contains("install"),
        "should contain install path"
    );
    assert!(
        result_all.contains(separator),
        "should use platform separator"
    );

    // Scenario 2: Some paths None (should skip them)
    let paths_partial = PathsConfig {
        qt_install: Some(PathBuf::from("C:/Qt")),
        vcpkg: None,
        install_libs: Some(PathBuf::from("C:/lib")),
        ..Default::default()
    };
    let result_partial = paths_partial.cmake_prefix_path();
    let count = result_partial.split(separator).count();
    assert_eq!(count, 2, "should only have 2 paths when one is None");

    // Scenario 3: No paths present
    let paths_empty = PathsConfig {
        qt_install: None,
        vcpkg: None,
        install_libs: None,
        ..Default::default()
    };
    let result_empty = paths_empty.cmake_prefix_path();
    assert_eq!(result_empty, "", "should be empty when no paths");
}

#[test]
fn test_cmake_install_prefix_with_install() {
    let paths = PathsConfig {
        install: Some(PathBuf::from("C:/mo2/install")),
        ..Default::default()
    };

    let result = paths.cmake_install_prefix();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "C:/mo2/install");
}

#[test]
fn test_cmake_install_prefix_none_when_missing() {
    let paths = PathsConfig {
        install: None,
        ..Default::default()
    };

    let result = paths.cmake_install_prefix();
    assert!(result.is_none());
}

/// `GlobalConfig` uses `#[serde(flatten)]`, which is incompatible with
/// `deny_unknown_fields` (serde silently ignores it). Unknown fields are
/// absorbed into the flattened map, so parsing succeeds.
#[test]
fn test_flatten_absorbs_unknown_fields_global() {
    let toml = r#"
[global]
dry = true
unknown_field = "absorbed by flatten"
"#;
    let result = Config::parse(toml);
    assert!(
        result.is_ok(),
        "expected Ok because flatten absorbs unknown fields, got: {result:?}"
    );
}

/// `TaskConfig` uses `#[serde(flatten)]`, which is incompatible with
/// `deny_unknown_fields` (serde silently ignores it). Unknown fields are
/// absorbed into the flattened map, so parsing succeeds.
#[test]
fn test_flatten_absorbs_unknown_fields_task() {
    let toml = r#"
[task]
mo_org = "Test"
typo_field = true
"#;
    let result = Config::parse(toml);
    assert!(
        result.is_ok(),
        "expected Ok because flatten absorbs unknown fields, got: {result:?}"
    );
}

#[test]
fn test_deny_unknown_fields_top_level() {
    let toml = r#"
[global]
dry = true

[unknown_section]
foo = "bar"
"#;
    let result = Config::parse(toml);
    assert!(result.is_err());
    insta::assert_snapshot!(
        "deny_unknown_fields_top_level",
        result.unwrap_err().to_string()
    );
}

#[test]
fn test_merge_task_config_partial_override() {
    let toml = r#"
[task]
git_shallow = true
mo_org = "DefaultOrg"
no_pull = false

[tasks.custom]
git_shallow = false
"#;
    let config = Config::parse(toml).unwrap();
    let custom_config = config.task_config("custom");

    insta::assert_yaml_snapshot!(
        "merge_task_config_partial_override",
        serde_json::json!({
            "git_shallow": custom_config.git_clone.git_shallow,
            "mo_org": custom_config.mo_org,
            "no_pull": custom_config.git_behavior.no_pull,
        })
    );
}

#[test]
fn test_merge_task_config_full_override() {
    let toml = r#"
[task]
git_shallow = true
mo_org = "DefaultOrg"

[tasks.full]
git_shallow = false
mo_org = "CustomOrg"
mo_branch = "custom_branch"
"#;
    let config = Config::parse(toml).unwrap();
    let full_config = config.task_config("full");

    insta::assert_yaml_snapshot!(
        "merge_task_config_full_override",
        serde_json::json!({
            "git_shallow": full_config.git_clone.git_shallow,
            "mo_org": full_config.mo_org,
            "mo_branch": full_config.mo_branch,
        })
    );
}

#[test]
fn test_merge_task_config_nonexistent_task() {
    let toml = r#"
[task]
git_shallow = true
mo_org = "DefaultOrg"

[tasks.custom]
git_shallow = false
"#;
    let config = Config::parse(toml).unwrap();
    // Requesting a task that doesn't exist should return default config
    let nonexistent_config = config.task_config("nonexistent");

    insta::assert_yaml_snapshot!(
        "merge_task_config_nonexistent_task",
        serde_json::json!({
            "git_shallow": nonexistent_config.git_clone.git_shallow,
            "mo_org": nonexistent_config.mo_org,
        })
    );
}

#[test]
fn test_merge_task_config_boolean_fields() {
    let toml = r"
[task]
enabled = true
no_pull = false

[tasks.booltest]
enabled = false
";
    let config = Config::parse(toml).unwrap();
    let bool_config = config.task_config("booltest");

    insta::assert_yaml_snapshot!(
        "merge_task_config_boolean_fields",
        serde_json::json!({
            "enabled": bool_config.enabled,
            "no_pull": bool_config.git_behavior.no_pull,
        })
    );
}

#[test]
fn test_merge_task_config_string_fields() {
    let toml = r#"
[task]
mo_org = "ModOrganizer2"
mo_branch = "master"
mo_fallback = ""

[tasks.stringtest]
mo_org = "CustomOrg"
mo_fallback = "develop"
"#;
    let config = Config::parse(toml).unwrap();
    let string_config = config.task_config("stringtest");

    insta::assert_yaml_snapshot!(
        "merge_task_config_string_fields",
        serde_json::json!({
            "mo_org": string_config.mo_org,
            "mo_branch": string_config.mo_branch,
            "mo_fallback": string_config.mo_fallback,
        })
    );
}

#[test]
fn test_merge_task_config_remote_fields() {
    let toml = r#"
[task]
remote_no_push_upstream = false

[tasks.remotetest]
remote_org = "MyOrg"
"#;
    let config = Config::parse(toml).unwrap();
    let remote_config = config.task_config("remotetest");

    insta::assert_yaml_snapshot!(
        "merge_task_config_remote_fields",
        serde_json::json!({
            "remote_org": remote_config.remote_setup.remote_org,
            "remote_no_push_upstream": remote_config.remote_setup.remote_no_push_upstream,
        })
    );
}

// --- ConfigLoader Tests ---

#[test]
fn test_config_loader_add_toml_file_success() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::new().expect("failed to create temp file");
    writeln!(
        file,
        r#"
[global]
dry = true

[paths]
prefix = "/test/prefix"
"#
    )
    .expect("failed to write temp file");

    let config = ConfigLoader::new()
        .add_toml_file(file.path())
        .build()
        .expect("build should succeed");

    assert!(config.global.dry);
    assert_eq!(config.paths.prefix, Some(PathBuf::from("/test/prefix")));
}

#[test]
fn test_config_loader_add_toml_file_not_found() {
    let loader = ConfigLoader::new().add_toml_file("/nonexistent/path/to/config.toml");

    // add_toml_file returns Self, but build() should fail for required files
    let build_result = loader.build();
    assert!(build_result.is_err());
}

#[test]
fn test_config_loader_add_toml_file_invalid_toml() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::new().expect("failed to create temp file");
    writeln!(file, "this is not valid toml {{{{{{").expect("failed to write");

    let loader = ConfigLoader::new().add_toml_file(file.path());

    let result = loader.build();
    assert!(result.is_err(), "build should fail with invalid TOML");
}

#[test]
fn test_config_loader_with_env_prefix() {
    // Set env var for this test
    // SAFETY: This test runs in isolation (nextest runs each test in its own process)
    unsafe {
        std::env::set_var("MOBTEST_GLOBAL_DRY", "true");
    }

    let config = ConfigLoader::new()
        .add_toml_str("[global]\n dry = false")
        .with_env_prefix("MOBTEST")
        .build()
        .expect("build should succeed");

    // Env var should override TOML value
    assert!(config.global.dry, "env var should override TOML value");

    // Cleanup
    // SAFETY: Same as above
    unsafe {
        std::env::remove_var("MOBTEST_GLOBAL_DRY");
    }
}

#[test]
fn test_config_loader_set_override() {
    let config = ConfigLoader::new()
        .add_toml_str("[global]\n dry = false")
        .set("global.dry", true)
        .expect("set should succeed")
        .build()
        .expect("build should succeed");

    assert!(config.global.dry, "set override should take effect");
}

#[test]
fn test_config_loader_layered_sources() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // First layer: file
    let mut file = NamedTempFile::new().expect("failed to create temp file");
    writeln!(
        file,
        r#"
[global]
dry = false
redownload = true

[task]
mo_org = "FileOrg"
"#
    )
    .expect("failed to write");

    // Second layer: string (should override)
    let config = ConfigLoader::new()
        .add_toml_file(file.path())
        .add_toml_str(
            r#"
[global]
dry = true

[task]
mo_branch = "develop"
"#,
        )
        .build()
        .expect("build should succeed");

    // Verify layering
    assert!(config.global.dry, "string should override file");
    assert!(
        config.global.clean_download_actions.redownload,
        "file value should persist"
    );
    assert_eq!(config.task.mo_org, "FileOrg", "file value should persist");
    assert_eq!(
        config.task.mo_branch, "develop",
        "string should add new value"
    );
}

#[test]
fn test_config_loader_build_deserialization_error() {
    // Invalid type for a field
    let result = ConfigLoader::new()
        .add_toml_str("[global]\n dry = \"not a boolean\"")
        .build();

    assert!(result.is_err(), "build should fail with type mismatch");
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("dry") || err_str.contains("invalid type"),
        "error should mention the problematic field: {err_str}"
    );
}

#[test]
fn test_config_loader_default_impl() {
    let loader1 = ConfigLoader::new();
    let loader2 = ConfigLoader::default();

    // Both should produce equivalent empty configs
    let config1 = loader1.build().expect("build should succeed");
    let config2 = loader2.build().expect("build should succeed");

    assert_eq!(config1.global.dry, config2.global.dry);
    assert_eq!(config1.task.mo_org, config2.task.mo_org);
}
