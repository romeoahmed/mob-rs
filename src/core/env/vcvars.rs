// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Visual Studio environment variable capture (Windows-only).
//!
//! ```text
//! vs::find_latest()
//!   --> capture_vcvars(arch)
//!       PowerShell: Import-Module DevShell.dll
//!       Enter-VsDevShell -arch={x86|x64}
//!       Get-ChildItem Env: -> KEY=VALUE -> Env
//!
//! X86 --> -arch=x86 -host_arch=x64
//! X64 --> -arch=x64 -host_arch=x64
//! ```

use super::container::Env;
use super::types::Arch;
use crate::core::vs;
use crate::error::Result;
use anyhow::Context;
use std::process::{Command, Stdio};
use tracing::{Level, debug, enabled, trace};

/// Captures Visual Studio environment variables using `PowerShell`'s Enter-VsDevShell.
///
/// This function:
/// 1. Locates the VS installation using vswhere (via `core::vs`)
/// 2. Runs `PowerShell` with `Enter-VsDevShell` and captures environment variables
/// 3. Parses the output to extract environment variables
///
/// # Arguments
/// * `arch` - Target architecture (x86 or x64)
///
/// # Returns
/// An `Env` containing the Visual Studio environment variables.
///
/// # Errors
/// Returns an error if VS cannot be found or the `DevShell` fails to initialize.
pub fn capture_vcvars(arch: Arch) -> Result<Env> {
    debug!(arch = %arch, "Capturing VS environment via Enter-VsDevShell");

    let vs_info = vs::find_latest(None).context("Failed to find Visual Studio installation")?;

    let devshell_dll = vs_info.devshell_dll();
    if !devshell_dll.exists() {
        anyhow::bail!(
            "Microsoft.VisualStudio.DevShell.dll not found at: {}",
            devshell_dll.display()
        );
    }

    trace!(
        devshell = %devshell_dll.display(),
        instance = %vs_info.instance_id,
        "Found Visual Studio installation"
    );

    // Build PowerShell command to enter VS DevShell
    let (target_arch, host_arch) = match arch {
        Arch::X86 => ("x86", "x64"),
        Arch::X64 => ("x64", "x64"),
    };

    let ps_script = format!(
        r#"
Import-Module "{dll}"
Enter-VsDevShell {instance} -SkipAutomaticLocation -DevCmdArguments "-arch={target_arch} -host_arch={host_arch}" | Out-Null
Get-ChildItem Env: | ForEach-Object {{ "$($_.Name)=$($_.Value)" }}
"#,
        dll = devshell_dll.display(),
        instance = vs_info.instance_id,
        target_arch = target_arch,
        host_arch = host_arch,
    );

    if enabled!(Level::TRACE) {
        trace!(script = %ps_script, "running PowerShell script for VS DevShell");
    }

    let output = Command::new("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run pwsh for VS DevShell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Enter-VsDevShell failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut env = Env::new();
    env.copy_for_write();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = &line[..eq_pos];
            let value = &line[eq_pos + 1..];

            if !key.is_empty() {
                if enabled!(Level::TRACE) {
                    trace!(key = key, value = value, "captured env var");
                }
                env.set(key, value);
            }
        }
    }

    debug!(count = env.len(), "Captured environment variables");
    Ok(env)
}
