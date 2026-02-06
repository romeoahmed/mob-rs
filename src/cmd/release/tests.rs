// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::version::default_rc_path;
use super::{
    DevbuildArgs, OfficialArgs, archive_name, ensure_output_dir, ensure_output_file,
    modorganizer_super_dir, resolve_official_output_dir, resolve_output_dir,
};
use crate::cli::release::{
    BinaryOutputArgs, OfficialInstallerArgs, OfficialOutputArgs, PdbOutputArgs,
};
use crate::config::Config;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_archive_name_cases() {
    // Consolidate all archive_name test cases into a single snapshot
    let cases = vec![
        ("basic", archive_name("2.5.0", None, None)),
        ("with_suffix", archive_name("2.5.0", Some("rc1"), None)),
        ("with_what", archive_name("2.5.0", None, Some("pdbs"))),
        (
            "suffix_and_what",
            archive_name("2.5.0", Some("beta"), Some("src")),
        ),
        (
            "empty_suffix_ignored",
            archive_name("2.5.0", Some(""), Some("pdbs")),
        ),
        (
            "empty_what_ignored",
            archive_name("2.5.0", Some("rc1"), Some("")),
        ),
    ];
    insta::assert_yaml_snapshot!("archive_name_cases", cases);
}

#[test]
fn test_resolve_output_dir_from_args() {
    let args = DevbuildArgs {
        output_dir: Some(PathBuf::from("/custom/output")),
        ..Default::default()
    };
    let config = Config::default();
    let result = resolve_output_dir(&args, &config).unwrap();
    insta::assert_yaml_snapshot!(
        "resolve_output_dir_from_args",
        result.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn test_resolve_output_dir_from_config() {
    let args = DevbuildArgs::default();
    let config = Config {
        paths: crate::config::paths::PathsConfig {
            prefix: Some(PathBuf::from("/mo2")),
            ..Default::default()
        },
        ..Default::default()
    };
    let result = resolve_output_dir(&args, &config).unwrap();
    insta::assert_yaml_snapshot!(
        "resolve_output_dir_from_config",
        result.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn test_resolve_output_dir_error_no_prefix() {
    let args = DevbuildArgs::default();
    let config = Config::default();
    let result = resolve_output_dir(&args, &config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("prefix"),
        "expected error about prefix, got: {err_msg}"
    );
}

#[test]
fn test_resolve_official_output_dir_from_args() {
    let args = OfficialArgs {
        output_dir: Some(PathBuf::from("/release/output")),
        branch: "v2.5.0".to_string(),
        outputs: OfficialOutputArgs {
            bin: BinaryOutputArgs {
                bin: true,
                no_bin: false,
            },
            pdbs: PdbOutputArgs {
                pdbs: true,
                no_pdbs: false,
            },
            installer: OfficialInstallerArgs {
                no_installer: false,
            },
        },
        force: false,
    };
    let config = Config::default();
    let result = resolve_official_output_dir(&args, &config).unwrap();
    insta::assert_yaml_snapshot!(
        "resolve_official_output_dir_from_args",
        result.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn test_ensure_output_file_force() {
    let temp = temp_dir();
    let file = temp.path().join("existing.7z");
    std::fs::write(&file, "test").expect("failed to create file");

    // Without force - should fail
    let result = ensure_output_file(&file, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--force"));

    // With force - should succeed
    let result = ensure_output_file(&file, true);
    assert!(result.is_ok());
}

#[test]
fn test_ensure_output_file_not_exists() {
    let temp = temp_dir();
    let file = temp.path().join("new.7z");

    let result = ensure_output_file(&file, false);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ensure_output_dir_creates_dir() {
    let temp = temp_dir();
    let new_dir = temp.path().join("releases");

    assert!(!new_dir.exists());
    let result = ensure_output_dir(&new_dir, false).await;
    assert!(result.is_ok());
    assert!(new_dir.exists());
}

#[tokio::test]
async fn test_ensure_output_dir_dry_run() {
    let temp = temp_dir();
    let new_dir = temp.path().join("releases");

    assert!(!new_dir.exists());
    let result = ensure_output_dir(&new_dir, true).await;
    assert!(result.is_ok());
    // Should NOT create directory in dry run
    assert!(!new_dir.exists());
}

#[tokio::test]
async fn test_ensure_output_dir_existing() {
    let temp = temp_dir();
    let existing_dir = temp.path().join("releases");
    fs::create_dir(&existing_dir)
        .await
        .expect("failed to create dir");

    let result = ensure_output_dir(&existing_dir, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ensure_output_dir_file_conflict() {
    let temp = temp_dir();
    let file = temp.path().join("releases");
    fs::write(&file, "test")
        .await
        .expect("failed to create file");

    let result = ensure_output_dir(&file, false).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not a directory"));
}

#[test]
fn test_default_rc_path() {
    let config = Config {
        paths: crate::config::paths::PathsConfig {
            build: Some(PathBuf::from("/mo2/build")),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = default_rc_path(&config).unwrap();
    insta::assert_yaml_snapshot!(
        "default_rc_path",
        result.to_string_lossy().replace('\\', "/")
    );
}

#[test]
fn test_modorganizer_super_dir() {
    let config = Config {
        paths: crate::config::paths::PathsConfig {
            build: Some(PathBuf::from("/mo2/build")),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = modorganizer_super_dir(&config).unwrap();
    insta::assert_yaml_snapshot!(
        "modorganizer_super_dir",
        result.to_string_lossy().replace('\\', "/")
    );
}
