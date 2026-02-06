// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Utility modules.
//!
//! ```text
//! encoding
//!   bytes_to_utf8()  CP1252/CP437/UTF-16 --> UTF-8
//!   EncodedBuffer    streaming line iterator
//! fs
//!   walk:  parallel_walk(), find_files(), WalkOptions
//!   copy:  copy_dir_contents_async(), copy_files_async()
//! ```

pub mod encoding;
pub mod fs;
