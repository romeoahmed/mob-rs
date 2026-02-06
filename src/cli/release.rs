// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Release command arguments.
//!
//! # Output Options
//!
//! ```text
//! devbuild:
//! --bin/--no-bin, --pdbs/--no-pdbs, --src/--no-src
//! --version X.Y.Z, --suffix "-beta"
//! official:
//! --bin/--pdbs/--src, --inst
//! ```

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for the `release` command.
#[derive(Debug, Clone, Args)]
pub struct ReleaseArgs {
    /// Release mode (devbuild or official).
    #[command(subcommand)]
    pub mode: ReleaseMode,
}

/// Release mode variants.
#[derive(Debug, Clone, Subcommand)]
pub enum ReleaseMode {
    /// Create a development build release.
    Devbuild(DevbuildArgs),

    /// Create an official release.
    Official(OfficialArgs),
}

/// Arguments for devbuild release.
#[derive(Debug, Clone, Default, Args)]
pub struct DevbuildArgs {
    /// Output selection toggles.
    #[command(flatten)]
    pub outputs: ReleaseOutputArgs,

    /// Version source selection.
    #[command(flatten)]
    pub version_source: VersionSourceArgs,

    /// Overrides the path to version.rc.
    #[arg(long = "rc", value_name = "PATH")]
    pub rc_path: Option<PathBuf>,

    /// Overrides the version string.
    #[arg(long = "version", value_name = "VERSION")]
    pub version: Option<String>,

    /// Sets the output directory to use instead of `$prefix/releases`.
    #[arg(long = "output-dir", value_name = "PATH")]
    pub output_dir: Option<PathBuf>,

    /// Optional suffix to add to the archive filenames.
    #[arg(long = "suffix", value_name = "SUFFIX")]
    pub suffix: Option<String>,

    /// Ignores file size warnings and existing release directories.
    #[arg(long)]
    pub force: bool,
}

impl DevbuildArgs {
    /// Returns effective bin setting.
    #[must_use]
    pub const fn create_bin(&self) -> bool {
        !self.outputs.bin.no_bin && self.outputs.bin.bin
    }

    /// Returns effective pdbs setting.
    #[must_use]
    pub const fn create_pdbs(&self) -> bool {
        !self.outputs.pdbs.no_pdbs && self.outputs.pdbs.pdbs
    }

    /// Returns effective src setting.
    #[must_use]
    pub const fn create_src(&self) -> bool {
        !self.outputs.src.no_src && self.outputs.src.src
    }

    /// Returns effective installer setting.
    #[must_use]
    pub const fn copy_installer(&self) -> bool {
        self.outputs.installer.installer && !self.outputs.installer.no_installer
    }
}

/// Arguments for official release.
#[derive(Debug, Clone, Args)]
pub struct OfficialArgs {
    /// Use this branch in the super repos.
    #[arg(value_name = "BRANCH")]
    pub branch: String,

    /// Sets the output directory to use instead of `$prefix/releases`.
    #[arg(long = "output-dir", value_name = "PATH")]
    pub output_dir: Option<PathBuf>,

    /// Output selection toggles.
    #[command(flatten)]
    pub outputs: OfficialOutputArgs,

    /// Ignores file size warnings and existing release directories.
    #[arg(long)]
    pub force: bool,
}

impl OfficialArgs {
    /// Returns effective bin setting.
    #[must_use]
    pub const fn create_bin(&self) -> bool {
        !self.outputs.bin.no_bin && self.outputs.bin.bin
    }

    /// Returns effective pdbs setting.
    #[must_use]
    pub const fn create_pdbs(&self) -> bool {
        !self.outputs.pdbs.no_pdbs && self.outputs.pdbs.pdbs
    }

    /// Returns whether to build and copy the installer.
    #[must_use]
    pub const fn build_installer(&self) -> bool {
        !self.outputs.installer.no_installer
    }
}

/// Release output toggles for devbuild.
#[derive(Debug, Clone, Default, Args)]
pub struct ReleaseOutputArgs {
    /// Binary archive output toggle.
    #[command(flatten)]
    pub bin: BinaryOutputArgs,

    /// PDB archive output toggle.
    #[command(flatten)]
    pub pdbs: PdbOutputArgs,

    /// Source archive output toggle.
    #[command(flatten)]
    pub src: SrcOutputArgs,

    /// Installer output toggle.
    #[command(flatten)]
    pub installer: InstallerOutputArgs,
}

/// Release output toggles for official builds.
#[derive(Debug, Clone, Default, Args)]
pub struct OfficialOutputArgs {
    /// Binary archive output toggle.
    #[command(flatten)]
    pub bin: BinaryOutputArgs,

    /// PDB archive output toggle.
    #[command(flatten)]
    pub pdbs: PdbOutputArgs,

    /// Installer output toggle.
    #[command(flatten)]
    pub installer: OfficialInstallerArgs,
}

/// Binary output toggle.
#[derive(Debug, Clone, Default, Args)]
pub struct BinaryOutputArgs {
    /// Create the binary archive.
    #[arg(long = "bin", conflicts_with = "no_bin", default_value_t = true)]
    pub bin: bool,

    /// Don't create the binary archive.
    #[arg(long = "no-bin")]
    pub no_bin: bool,
}

/// PDB output toggle.
#[derive(Debug, Clone, Default, Args)]
pub struct PdbOutputArgs {
    /// Create the PDBs archive.
    #[arg(long = "pdbs", conflicts_with = "no_pdbs", default_value_t = true)]
    pub pdbs: bool,

    /// Don't create the PDBs archive.
    #[arg(long = "no-pdbs")]
    pub no_pdbs: bool,
}

/// Source output toggle.
#[derive(Debug, Clone, Default, Args)]
pub struct SrcOutputArgs {
    /// Create the source archive.
    #[arg(long = "src", conflicts_with = "no_src", default_value_t = true)]
    pub src: bool,

    /// Don't create the source archive.
    #[arg(long = "no-src")]
    pub no_src: bool,
}

/// Installer output toggle for devbuild.
#[derive(Debug, Clone, Default, Args)]
pub struct InstallerOutputArgs {
    /// Copy the installer.
    #[arg(long = "inst", conflicts_with = "no_installer")]
    pub installer: bool,

    /// Don't copy the installer.
    #[arg(long = "no-inst", conflicts_with = "installer")]
    pub no_installer: bool,
}

/// Installer output toggle for official builds.
#[derive(Debug, Clone, Default, Args)]
pub struct OfficialInstallerArgs {
    /// Skip building the installer task.
    #[arg(long = "no-installer")]
    pub no_installer: bool,
}

/// Version source selection arguments.
#[derive(Debug, Clone, Default, Args)]
pub struct VersionSourceArgs {
    /// Retrieves version information from ModOrganizer.exe.
    #[arg(long = "version-from-exe", conflicts_with = "version_from_rc")]
    pub version_from_exe: bool,

    /// Retrieves version information from modorganizer/src/version.rc.
    #[arg(long = "version-from-rc", conflicts_with = "version_from_exe")]
    pub version_from_rc: bool,
}
