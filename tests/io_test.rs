use std::fs;

use sarang_qdte::io::{reader, writer, LineEnding};

#[test]
fn reads_existing_lf_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, "hello\nworld\n").unwrap();

    let loaded = reader::read(&path).unwrap();
    assert!(loaded.existed);
    assert_eq!(loaded.content, "hello\nworld\n");
    assert_eq!(loaded.line_ending, LineEnding::Lf);
    assert!(!loaded.had_bom);
}

#[test]
fn reads_crlf_file_normalizes_to_lf_in_memory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, "hello\r\nworld\r\n").unwrap();

    let loaded = reader::read(&path).unwrap();
    assert!(loaded.existed);
    assert_eq!(loaded.content, "hello\nworld\n");
    assert_eq!(loaded.line_ending, LineEnding::Crlf);
}

#[test]
fn read_nonexistent_returns_empty_buffer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.txt");

    let loaded = reader::read(&path).unwrap();
    assert!(!loaded.existed);
    assert_eq!(loaded.content, "");
    assert!(!loaded.had_bom);
}

#[test]
fn write_creates_file_with_lf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.txt");

    writer::write(&path, "hello\nworld\n", LineEnding::Lf, false).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello\nworld\n");
}

#[test]
fn write_converts_lf_to_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.txt");

    writer::write(&path, "hello\nworld\n", LineEnding::Crlf, false).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello\r\nworld\r\n");
}

#[test]
fn round_trip_preserves_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, "line1\r\nline2\r\n").unwrap();

    let loaded = reader::read(&path).unwrap();
    writer::write(&loaded.path, &loaded.content, loaded.line_ending, loaded.had_bom).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "line1\r\nline2\r\n");
}

#[test]
fn reads_utf8_bom_file_strips_bom() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bom.txt");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"hello\n");
    fs::write(&path, &bytes).unwrap();

    let loaded = reader::read(&path).unwrap();
    assert!(loaded.existed);
    assert!(loaded.had_bom);
    assert_eq!(loaded.content, "hello\n");
}

#[test]
fn write_restores_bom() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bom-out.txt");

    writer::write(&path, "hello\n", LineEnding::Lf, true).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert_eq!(&bytes[3..], b"hello\n");
}

#[test]
fn round_trip_preserves_bom_and_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bom-crlf.txt");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"line1\r\nline2\r\n");
    fs::write(&path, &bytes).unwrap();

    let loaded = reader::read(&path).unwrap();
    writer::write(&loaded.path, &loaded.content, loaded.line_ending, loaded.had_bom).unwrap();

    assert_eq!(fs::read(&path).unwrap(), bytes);
}