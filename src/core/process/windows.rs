// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Windows-specific process utilities.
//!
//! ```text
//! send_ctrl_break(pid) --> CTRL_BREAK_EVENT
//! setup_job_object(child) --> JobObject(KILL_ON_JOB_CLOSE)
//! cancellation: ctrl_break -> 500ms -> kill -> wait
//! ```

use crate::error::Result;
use anyhow::Context;
use tokio::process::Child;

use crate::core::job::JobObject;

/// Sends CTRL+BREAK to a process on Windows.
///
/// # Errors
///
/// Returns an error if `GenerateConsoleCtrlEvent` fails.
pub(super) fn send_ctrl_break(pid: u32) -> Result<()> {
    use windows::Win32::System::Console::CTRL_BREAK_EVENT;
    use windows::Win32::System::Console::GenerateConsoleCtrlEvent;

    // SAFETY: GenerateConsoleCtrlEvent is safe to call with a valid process group ID
    unsafe {
        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid).map_err(|e: windows::core::Error| {
            anyhow::anyhow!("Failed to send CTRL_BREAK: {}", e.message())
        })?;
    }
    Ok(())
}

pub(super) fn setup_job_object(child: &Child) -> Result<Option<JobObject>> {
    if let Some(pid) = child.id() {
        let job = JobObject::new().context("Failed to create job object")?;
        job.assign_pid(pid)
            .context("Failed to assign process to job")?;
        Ok(Some(job))
    } else {
        Ok(None)
    }
}
