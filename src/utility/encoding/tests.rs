// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{EncodedBuffer, Encoding, bytes_to_utf8};

#[test]
fn test_utf8_passthrough() {
    let input = "Hello, 世界!";
    let result = bytes_to_utf8(Encoding::Utf8, input.as_bytes());
    insta::assert_snapshot!(result);
}

#[test]
fn test_windows_1252_conversion() {
    // "café" in Windows-1252: 0x63 0x61 0x66 0xe9
    let input = b"caf\xe9";
    let result = bytes_to_utf8(Encoding::Acp, input);
    insta::assert_snapshot!(result);
}

#[test]
fn test_utf16_le_conversion() {
    // "Hi" in UTF-16 LE: 0x48 0x00 0x69 0x00
    let input = b"H\x00i\x00";
    let result = bytes_to_utf8(Encoding::Utf16Le, input);
    insta::assert_snapshot!(result);
}

#[test]
fn test_encoded_buffer_lines() {
    let mut buffer = EncodedBuffer::new(Encoding::Utf8);
    buffer.add(b"line1\r\nline2\nline3");

    let lines: Vec<String> = buffer.next_utf8_lines(true).collect();
    insta::assert_yaml_snapshot!(lines);
}

#[test]
fn test_encoded_buffer_incremental() {
    let mut buffer = EncodedBuffer::new(Encoding::Utf8);

    buffer.add(b"line1\n");
    let lines1: Vec<String> = buffer.next_utf8_lines(false).collect();

    buffer.add(b"line2\npartial");
    let lines2: Vec<String> = buffer.next_utf8_lines(false).collect();

    // Finish - get the partial line
    let lines3: Vec<String> = buffer.next_utf8_lines(true).collect();

    insta::assert_yaml_snapshot!(
        "incremental_phases",
        vec![
            ("phase1_after_line1", lines1),
            ("phase2_after_line2_partial", lines2),
            ("phase3_finish", lines3),
        ]
    );
}

#[test]
fn test_encoded_buffer_empty_lines_skipped() {
    let mut buffer = EncodedBuffer::new(Encoding::Utf8);
    buffer.add(b"line1\n\n\nline2\n");

    let lines: Vec<String> = buffer.next_utf8_lines(true).collect();
    insta::assert_yaml_snapshot!(lines);
}
