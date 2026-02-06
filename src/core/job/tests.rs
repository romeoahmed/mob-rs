// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::JobObject;

#[test]
fn test_job_object_creation() {
    let job = JobObject::new();
    assert!(job.is_ok(), "JobObject creation should succeed");
}

#[test]
fn test_job_object_as_raw_handle() {
    let job = JobObject::new().expect("JobObject creation should succeed");
    let handle = job.as_raw_handle();
    // Handle should be valid (non-null, not INVALID_HANDLE_VALUE)
    assert!(!handle.is_invalid(), "JobObject handle should be valid");
}

#[test]
fn test_job_object_process_count() {
    let job = JobObject::new().expect("JobObject creation should succeed");
    let (active, total) = job.process_count().expect("process_count should succeed");
    // Initially, no processes are assigned
    insta::assert_yaml_snapshot!(
        "job_object_process_count",
        serde_json::json!({
            "active": active,
            "total": total,
        })
    );
}
