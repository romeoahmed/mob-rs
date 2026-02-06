// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::core::vs::VsInstallation;
use std::path::PathBuf;

#[test]
fn test_msbuild_path_construction() {
    // Test the path construction logic without requiring actual VS installation
    let vs = VsInstallation {
        instance_id: "test".to_string(),
        installation_path: PathBuf::from(
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        ),
        installation_version: "17.0.0.0".to_string(),
        display_name: "Test".to_string(),
        is_complete: true,
        is_prerelease: false,
    };

    let msbuild = vs.msbuild_path();
    let expected = PathBuf::from(
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\MSBuild.exe",
    );

    assert_eq!(msbuild, expected);
}

#[test]
fn test_devenv_path_construction() {
    // Test the path construction logic without requiring actual VS installation
    let vs = VsInstallation {
        instance_id: "test".to_string(),
        installation_path: PathBuf::from(
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        ),
        installation_version: "17.0.0.0".to_string(),
        display_name: "Test".to_string(),
        is_complete: true,
        is_prerelease: false,
    };

    let devenv = vs.devenv_path();
    let expected = PathBuf::from(
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe",
    );

    assert_eq!(devenv, expected);
}
