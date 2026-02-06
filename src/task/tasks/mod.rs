// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Concrete task implementations.
//!
//! ```text
//! task::tasks
//! Build: ModOrganizerTask, UsvfsTask
//! Download: StylesheetsTask, ExplorerPPTask
//! Copy/Package: LicensesTask, InstallerTask
//! TranslationsTask: Transifex → lrelease → .qm
//! ```
//!
//! This module contains the actual task implementations that build MO2 components:
//! - `ModOrganizerTask` - The central `ModOrganizer` project
//! - `UsvfsTask` - Multi-arch USVFS build
//! - `StylesheetsTask` - Download stylesheets from GitHub
//! - `ExplorerPPTask` - Download Explorer++ binaries
//! - `LicensesTask` - Copy license files
//! - `TranslationsTask` - Transifex translations
//! - `InstallerTask` - Inno Setup installer

pub mod explorerpp;
pub mod installer;
pub mod licenses;
pub mod modorganizer;
pub mod stylesheets;
pub mod translations;
pub mod usvfs;
