// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{VsInstallation, parse_vswhere_json};
use std::path::PathBuf;

#[test]
fn test_parse_vswhere_json_empty() {
    let json = "[]";
    let result = parse_vswhere_json(json).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_parse_vswhere_json_single() {
    let json = r#"[
        {
            "instanceId": "abc123",
            "installationPath": "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community",
            "displayName": "Visual Studio Community 2022",
            "installationVersion": "17.9.34728.123",
            "isComplete": true,
            "isPrerelease": false
        }
    ]"#;

    let result = parse_vswhere_json(json).unwrap();
    insta::assert_debug_snapshot!("core_parse_vswhere_json_single", result);
}

#[test]
fn test_parse_vswhere_json_multiple_sorted() {
    let json = r#"[
        {
            "instanceId": "old",
            "installationPath": "C:\\VS2019",
            "displayName": "VS 2019",
            "installationVersion": "16.5.0.0",
            "isComplete": true,
            "isPrerelease": false
        },
        {
            "instanceId": "new",
            "installationPath": "C:\\VS2022",
            "displayName": "VS 2022",
            "installationVersion": "17.9.0.0",
            "isComplete": true,
            "isPrerelease": false
        }
    ]"#;

    let mut result = parse_vswhere_json(json).unwrap();
    result.sort_by_key(|vs| std::cmp::Reverse(vs.version_tuple()));
    insta::assert_debug_snapshot!("core_parse_vswhere_json_multiple_sorted", result);
}

#[test]
fn test_parse_vswhere_json_version_sorting() {
    let json = r#"[
        {
            "instanceId": "a",
            "installationPath": "C:\\A",
            "displayName": "A",
            "installationVersion": "17.9.0.0",
            "isComplete": true,
            "isPrerelease": false
        },
        {
            "instanceId": "b",
            "installationPath": "C:\\B",
            "displayName": "B",
            "installationVersion": "17.14.0.0",
            "isComplete": true,
            "isPrerelease": false
        }
    ]"#;

    let mut result = parse_vswhere_json(json).unwrap();
    result.sort_by_key(|vs| std::cmp::Reverse(vs.version_tuple()));

    let versions: Vec<_> = result
        .iter()
        .map(|vs| vs.installation_version.as_str())
        .collect();
    insta::assert_yaml_snapshot!("parse_vswhere_json_version_sorting", versions);
}

#[test]
fn test_parse_vswhere_json_invalid() {
    let json = "not json";
    let result = parse_vswhere_json(json);
    assert!(result.is_err());
}

#[test]
fn test_parse_vswhere_json_real_world() {
    let json = r#"[
        {
            "instanceId": "76a9c886",
            "installDate": "2026-01-31T15:07:32Z",
            "installationName": "VisualStudio/17.14.25",
            "installationPath": "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools",
            "installationVersion": "17.14.36915.13",
            "productId": "Microsoft.VisualStudio.Product.BuildTools",
            "productPath": "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\Common7\\Tools\\LaunchDevCmd.bat",
            "state": 4294967295,
            "isComplete": true,
            "isLaunchable": true,
            "isPrerelease": false,
            "displayName": "Visual Studio Build Tools 2022",
            "description": "Visual Studio build tools description",
            "channelId": "VisualStudio.17.Release"
        }
    ]"#;

    let result = parse_vswhere_json(json).unwrap();
    insta::assert_debug_snapshot!("parse_vswhere_json_real_world", result);
}

#[test]
fn test_derived_paths() {
    let vs = VsInstallation {
        instance_id: "test".to_string(),
        installation_path: PathBuf::from(
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        ),
        installation_version: "17.9.0.0".to_string(),
        display_name: "Test".to_string(),
        is_complete: true,
        is_prerelease: false,
    };

    // Normalize path separators for cross-platform snapshot consistency
    let normalize = |p: PathBuf| p.to_string_lossy().replace('\\', "/");

    insta::assert_yaml_snapshot!(
        "vs_derived_paths",
        serde_json::json!({
            "devshell_dll": normalize(vs.devshell_dll()),
            "msbuild_path": normalize(vs.msbuild_path()),
            "devenv_path": normalize(vs.devenv_path()),
        })
    );
}

#[test]
fn test_version_tuple_parsing() {
    let vs = VsInstallation {
        instance_id: "test".to_string(),
        installation_path: PathBuf::from("C:\\test"),
        installation_version: "17.14.36915.13".to_string(),
        display_name: "Test".to_string(),
        is_complete: true,
        is_prerelease: false,
    };

    insta::assert_yaml_snapshot!("version_tuple_parsing", vs.version_tuple());
}
