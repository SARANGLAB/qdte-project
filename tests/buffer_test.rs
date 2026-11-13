use qdte::editor::buffer::Buffer;

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

#[test]
fn char_to_line_at_len_with_trailing_newline() {
    let b = Buffer::from_str("a\tb\n");
    // char_idx 4 is past the trailing '\n' — first position of the
    // empty last line. Must return 1, not 0.
    assert_eq!(b.char_to_line(4), 1);
}

#[test]
fn char_to_line_at_len_no_trailing_newline() {
    let b = Buffer::from_str("abc");
    // No trailing newline, so past-end stays on line 0.
    assert_eq!(b.char_to_line(3), 0);
}
