// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Legacy Windows encoding conversion (UTF-8 ↔ CP1252/CP437).
//!
//! ```text
//! External I/O        Internal      External I/O
//! CP1252/CP437 --(decode)--> UTF-8 --(encode)--> UTF-8/CP1252
//! ```
//!
//! Uses `encoding_rs`. Invalid sequences → U+FFFD.

use encoding_rs::{IBM866, WINDOWS_1252};
use std::borrow::Cow;

/// Encoding types for process output and file content.
///
/// Maps to Windows code pages:
/// - `Utf8`: UTF-8 (65001)
/// - `Utf16`: UTF-16 LE (1200) - handled separately
/// - `Acp`: Active Code Page, typically Windows-1252 (1252)
/// - `Oem`: OEM Code Page, typically IBM437/866 (437/866)
/// - `Unknown`: Treat as ASCII/UTF-8 passthrough
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    /// Unknown encoding - treat as UTF-8 passthrough
    #[default]
    Unknown,
    /// UTF-8 (code page 65001)
    Utf8,
    /// UTF-16 Little Endian (code page 1200)
    Utf16Le,
    /// Active Code Page - typically Windows-1252
    Acp,
    /// OEM Code Page - typically IBM437 for US Windows
    Oem,
}

/// Converts bytes from the given encoding to UTF-8.
///
/// # Arguments
/// * `encoding` - The source encoding of the bytes
/// * `bytes` - The raw bytes to convert
///
/// # Returns
/// A UTF-8 string. Invalid sequences are replaced with U+FFFD (replacement character).
///
/// # Example
/// ```
/// use mob_rs::utility::encoding::{bytes_to_utf8, Encoding};
///
/// let cp1252_bytes = b"caf\xe9"; // "café" in Windows-1252
/// let utf8 = bytes_to_utf8(Encoding::Acp, cp1252_bytes);
/// assert_eq!(utf8, "café");
/// ```
#[must_use]
pub fn bytes_to_utf8(encoding: Encoding, bytes: &[u8]) -> Cow<'_, str> {
    match encoding {
        Encoding::Utf8 | Encoding::Unknown => {
            // For UTF-8 or unknown, use lossy conversion
            String::from_utf8_lossy(bytes)
        }
        Encoding::Utf16Le => {
            // UTF-16 LE: interpret bytes as u16 pairs
            utf16_le_to_utf8(bytes)
        }
        Encoding::Acp => {
            // Windows-1252 (Active Code Page)
            let (result, _had_errors) = WINDOWS_1252.decode_without_bom_handling(bytes);
            result
        }
        Encoding::Oem => {
            // IBM866 (OEM Code Page)
            let (result, _had_errors) = IBM866.decode_without_bom_handling(bytes);
            result
        }
    }
}

/// Converts UTF-16 LE bytes to UTF-8.
fn utf16_le_to_utf8(bytes: &[u8]) -> Cow<'static, str> {
    // Handle odd byte count by ignoring the last byte
    let len = bytes.len() & !1;
    if len == 0 {
        return Cow::Borrowed("");
    }

    // Convert bytes to u16 slice
    let u16_slice: Vec<u16> = bytes[..len]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    // Decode UTF-16
    Cow::Owned(String::from_utf16_lossy(&u16_slice))
}

/// A buffer for streaming process output with encoding conversion.
///
/// Collects raw bytes from a process and provides line-by-line iteration
/// with automatic UTF-8 conversion based on the specified encoding.
///
/// # Example
/// ```
/// use mob_rs::utility::encoding::{EncodedBuffer, Encoding};
///
/// let mut buffer = EncodedBuffer::new(Encoding::Acp);
/// buffer.add(b"line1\r\nline2\r\n");
///
/// let lines: Vec<String> = buffer.next_utf8_lines(true).collect();
/// assert_eq!(lines, vec!["line1", "line2"]);
/// ```
pub struct EncodedBuffer {
    /// Encoding of the buffer content
    encoding: Encoding,
    /// Raw byte buffer
    bytes: Vec<u8>,
    /// Byte offset of last processed position
    last_offset: usize,
}

impl EncodedBuffer {
    /// Creates a new buffer with the given encoding.
    #[must_use]
    pub const fn new(encoding: Encoding) -> Self {
        Self {
            encoding,
            bytes: Vec::new(),
            last_offset: 0,
        }
    }

    /// Creates a new buffer with initial bytes.
    #[must_use]
    pub const fn with_bytes(encoding: Encoding, bytes: Vec<u8>) -> Self {
        Self {
            encoding,
            bytes,
            last_offset: 0,
        }
    }

    /// Appends bytes to the buffer.
    pub fn add(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Returns the entire buffer content as UTF-8.
    #[must_use]
    pub fn utf8_string(&self) -> String {
        bytes_to_utf8(self.encoding, &self.bytes).into_owned()
    }

    /// Returns an iterator over UTF-8 lines that haven't been processed yet.
    ///
    /// # Arguments
    /// * `finished` - If `true`, treats remaining bytes after the last newline
    ///   as a complete line. If `false`, those bytes are held for later.
    ///
    /// # Notes
    /// - Empty lines are skipped
    /// - Handles both LF and CRLF line endings
    /// - Updates internal offset to avoid reprocessing lines
    pub fn next_utf8_lines(&mut self, finished: bool) -> impl Iterator<Item = String> + '_ {
        EncodedLineIterator {
            buffer: self,
            finished,
        }
    }

    /// Resets the buffer, clearing all content and resetting the offset.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.last_offset = 0;
    }
}

/// Iterator over lines in an `EncodedBuffer`.
struct EncodedLineIterator<'a> {
    buffer: &'a mut EncodedBuffer,
    finished: bool,
}

impl Iterator for EncodedLineIterator<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.encoding {
            Encoding::Utf16Le => self.next_utf16_line(),
            _ => self.next_byte_line(),
        }
    }
}

impl EncodedLineIterator<'_> {
    /// Extracts the next line for byte-based encodings (UTF-8, ACP, OEM).
    fn next_byte_line(&mut self) -> Option<String> {
        let bytes = &self.buffer.bytes;
        let mut offset = self.buffer.last_offset;

        if offset >= bytes.len() {
            return None;
        }

        loop {
            let start = offset;

            // Find end of line (LF or CR)
            while offset < bytes.len() {
                if bytes[offset] == b'\n' || bytes[offset] == b'\r' {
                    break;
                }
                offset += 1;
            }

            if offset < bytes.len() {
                // Found a newline
                let line_bytes = &bytes[start..offset];

                // Skip past newline characters
                while offset < bytes.len() && (bytes[offset] == b'\n' || bytes[offset] == b'\r') {
                    offset += 1;
                }

                self.buffer.last_offset = offset;

                // Skip empty lines
                if line_bytes.is_empty() {
                    continue;
                }

                // Convert to UTF-8
                return Some(bytes_to_utf8(self.buffer.encoding, line_bytes).into_owned());
            } else if self.finished && start < bytes.len() {
                // No newline found but finished - return remaining as final line
                self.buffer.last_offset = bytes.len();
                let line_bytes = &bytes[start..];

                if line_bytes.is_empty() {
                    return None;
                }

                return Some(bytes_to_utf8(self.buffer.encoding, line_bytes).into_owned());
            }
            // No complete line available yet
            return None;
        }
    }

    /// Extracts the next line for UTF-16 LE encoding.
    fn next_utf16_line(&mut self) -> Option<String> {
        let bytes = &self.buffer.bytes;
        // Align to 2-byte boundary
        let size = bytes.len() & !1;
        let mut offset = self.buffer.last_offset & !1;

        if offset >= size {
            return None;
        }

        loop {
            let start = offset;

            // Find end of line (LF or CR as u16)
            while offset + 1 < size {
                let ch = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                if ch == '\n' as u16 || ch == '\r' as u16 {
                    break;
                }
                offset += 2;
            }

            if offset + 1 < size {
                // Found a newline
                let line_bytes = &bytes[start..offset];

                // Skip past newline characters
                while offset + 1 < size {
                    let ch = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                    if ch != '\n' as u16 && ch != '\r' as u16 {
                        break;
                    }
                    offset += 2;
                }

                self.buffer.last_offset = offset;

                // Skip empty lines
                if line_bytes.is_empty() {
                    continue;
                }

                // Convert to UTF-8
                return Some(utf16_le_to_utf8(line_bytes).into_owned());
            } else if self.finished && start < size {
                // No newline found but finished - return remaining as final line
                self.buffer.last_offset = size;
                let line_bytes = &bytes[start..size];

                if line_bytes.is_empty() {
                    return None;
                }

                return Some(utf16_le_to_utf8(line_bytes).into_owned());
            }
            // No complete line available yet
            return None;
        }
    }
}

#[cfg(test)]
mod tests;
