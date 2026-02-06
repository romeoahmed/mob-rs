// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Windows Job Object wrapper for child process management.
//!
//! ```text
//! JobObject (Windows-only)
//!   new()            --> KILL_ON_JOB_CLOSE
//!   assign_process() --> add child
//!   drop()           --> terminate all children
//! ```
//!
//! Thread-safe: implements `Send + Sync`.
//!
//! # Example
//! ```ignore
//! use mob_rs::core::job::JobObject;
//!
//! let job = JobObject::new()?;
//!
//! // After spawning a child process, assign it to the job
//! job.assign_process(child_handle)?;
//!
//! // When job is dropped, all assigned processes are terminated
//! ```

use crate::error::JobError;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// Converts a Windows API error to a `std::io::Error`.
fn windows_error_to_io(err: &windows::core::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(err.code().0)
}

/// A Windows Job Object configured to kill all assigned processes when closed.
///
/// When this struct is dropped or when the owning process exits (even abnormally),
/// Windows will automatically terminate all processes that have been assigned to it.
///
/// # Thread Safety
/// This type implements `Send` and `Sync` because:
/// - `HANDLE` is just a pointer-sized value
/// - Windows Job Objects support multi-threaded access
pub struct JobObject(HANDLE);

// SAFETY: HANDLE is just a pointer-sized value, and Windows Job Objects
// can be safely accessed from multiple threads.
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    /// Creates a new Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` set.
    ///
    /// This ensures all assigned processes are killed when:
    /// - The `JobObject` is dropped
    /// - The parent process exits (even abnormally)
    ///
    /// # Errors
    /// Returns an error if the Job Object could not be created or configured.
    pub fn new() -> Result<Self, JobError> {
        // SAFETY: CreateJobObjectW is safe with None arguments.
        // We check the result and handle errors appropriately.
        unsafe {
            let job = CreateJobObjectW(None, None)
                .map_err(|e| JobError::CreateFailed(windows_error_to_io(&e)))?;

            // Configure the job to kill all processes when closed
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let result = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw const info).cast(),
                u32::try_from(std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                    .unwrap_or(u32::MAX),
            );

            if let Err(e) = result {
                // Clean up the job handle on failure
                let _ = CloseHandle(job);
                return Err(JobError::ConfigureFailed(windows_error_to_io(&e)));
            }

            Ok(Self(job))
        }
    }

    /// Returns the raw Windows HANDLE for this Job Object.
    ///
    /// # Safety
    /// The returned handle is valid only as long as this `JobObject` exists.
    /// Do not close the handle manually.
    #[cfg(test)]
    pub(crate) const fn as_raw_handle(&self) -> HANDLE {
        self.0
    }

    /// Assigns a process to this Job Object using its HANDLE.
    ///
    /// # Arguments
    /// * `process_handle` - A valid HANDLE to the process to assign.
    ///
    /// # Errors
    /// Returns an error if the process could not be assigned.
    ///
    /// # Safety
    /// The `process_handle` must be a valid process handle.
    pub fn assign_process(&self, process_handle: HANDLE) -> Result<(), JobError> {
        // SAFETY: We trust the caller to provide a valid process handle.
        // AssignProcessToJobObject will fail gracefully if the handle is invalid.
        unsafe {
            AssignProcessToJobObject(self.0, process_handle)
                .map_err(|e| JobError::AssignHandleFailed(windows_error_to_io(&e)))
        }
    }

    /// Assigns a process to this Job Object using its PID.
    ///
    /// Opens a handle to the process with the minimum required permissions,
    /// assigns it to the job, then closes the handle.
    ///
    /// # Arguments
    /// * `pid` - The process ID to assign.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The process could not be opened (e.g., insufficient permissions, invalid PID)
    /// - The process could not be assigned to the job
    pub fn assign_pid(&self, pid: u32) -> Result<(), JobError> {
        // SAFETY: OpenProcess is safe if given a valid PID.
        // We request minimal permissions needed for job assignment.
        unsafe {
            let process =
                OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid).map_err(|e| {
                    JobError::OpenProcessFailed {
                        pid,
                        source: windows_error_to_io(&e),
                    }
                })?;

            // Assign to job, then close handle regardless of result
            let result = AssignProcessToJobObject(self.0, process);
            let _ = CloseHandle(process);

            result.map_err(|e| JobError::AssignFailed {
                pid,
                source: windows_error_to_io(&e),
            })
        }
    }

    /// Terminates all processes in this Job Object.
    ///
    /// # Arguments
    /// * `exit_code` - The exit code for terminated processes.
    ///
    /// # Errors
    /// Returns an error if termination failed.
    pub fn terminate(&self, exit_code: u32) -> Result<(), JobError> {
        // SAFETY: TerminateJobObject is safe with a valid job handle.
        unsafe {
            TerminateJobObject(self.0, exit_code)
                .map_err(|e| JobError::TerminateFailed(windows_error_to_io(&e)))
        }
    }

    /// Returns information about the processes in this Job Object.
    ///
    /// # Returns
    /// A tuple of (`active_processes`, `total_processes_spawned`)
    ///
    /// # Errors
    ///
    /// Returns a `JobError::QueryFailed` if querying job information fails.
    pub fn process_count(&self) -> Result<(u32, u32), JobError> {
        // SAFETY: QueryInformationJobObject is safe with a valid job handle.
        unsafe {
            let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();

            QueryInformationJobObject(
                Some(self.0),
                JobObjectBasicAccountingInformation,
                (&raw mut info).cast(),
                u32::try_from(std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>())
                    .unwrap_or(u32::MAX),
                None,
            )
            .map_err(|e| JobError::QueryFailed(windows_error_to_io(&e)))?;

            Ok((info.ActiveProcesses, info.TotalProcesses))
        }
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        // SAFETY: We own this handle and it's valid.
        // Closing the handle will automatically kill all assigned processes
        // due to JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests;
