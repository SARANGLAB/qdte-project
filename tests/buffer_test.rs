use sarang_qdte::editor::buffer::Buffer;

#[test]
fn buffer_is_usable_through_public_api() {
    let mut b = Buffer::from_str("hello");
    b.insert_str(5, " world").unwrap();
    assert_eq!(b.as_string(), "hello world");
    assert_eq!(b.len_chars(), 11);
}

#[test]
fn buffer_handles_multiline_content_from_io() {
    // Simulasi: file CRLF sudah dinormalisasi jadi LF oleh reader.
    let content = "line one\nline two\nline three\n";
    let b = Buffer::from_str(content);
    assert_eq!(b.len_lines(), 4);
    assert_eq!(b.line(0).unwrap(), "line one");
    assert_eq!(b.line(2).unwrap(), "line three");
    assert_eq!(b.line(3).unwrap(), "");
}