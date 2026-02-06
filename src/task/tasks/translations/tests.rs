// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{TranslationsTask, parse_project_name};

#[test]
fn test_translations_task_naming() {
    let cases: Vec<_> = [TranslationsTask::new(), TranslationsTask::default()]
        .into_iter()
        .map(|task| task.name().to_string())
        .collect();
    insta::assert_yaml_snapshot!("translations_task_naming", cases);
}

#[test]
fn test_parse_project_name() {
    insta::assert_debug_snapshot!(
        "parse_project_name_bsa_extractor",
        parse_project_name("mod-organizer-2.bsa_extractor")
    );
    insta::assert_debug_snapshot!(
        "parse_project_name_organizer",
        parse_project_name("mod-organizer-2.organizer")
    );
    insta::assert_debug_snapshot!(
        "parse_project_name_invalid",
        parse_project_name("invalid-name")
    );
    insta::assert_debug_snapshot!(
        "parse_project_name_trailing_dot",
        parse_project_name("prefix.")
    );
    insta::assert_debug_snapshot!(
        "parse_project_name_leading_dot",
        parse_project_name(".suffix")
    );
}
