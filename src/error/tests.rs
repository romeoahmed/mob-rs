// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{ConfigError, MobError, MobResult};

#[test]
fn test_config_error_display() {
    let err = ConfigError::MissingKey {
        section: "global".to_string(),
        key: "prefix".to_string(),
    };
    insta::assert_snapshot!(err.to_string());
}

#[test]
fn test_mob_error_size() {
    // MobError should be reasonably small
    // Box<str> variants (Bailed, Other) are 16 bytes (fat pointer: ptr + len)
    // With discriminant + alignment = 24 bytes
    let size = std::mem::size_of::<MobError>();
    assert!(size <= 24, "MobError is {size} bytes, expected <= 24");
}

#[test]
fn test_mob_result_size() {
    // Result<(), MobError> should be reasonably small
    let size = std::mem::size_of::<MobResult<()>>();
    assert!(size <= 24, "MobResult<()> is {size} bytes, expected <= 24");
}
