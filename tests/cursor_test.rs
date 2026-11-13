use qdte::editor::buffer::Buffer;
use qdte::editor::cursor::Cursor;

#[test]
fn navigate_through_multiline_buffer() {
    let b = Buffer::from_str("line one\nline two\nline three");
    let mut c = Cursor::new();

    c.move_line_end(&b);
    assert_eq!(c.char_idx(), 8); // end of "line one"

    c.move_down(&b);
    assert_eq!(c.line(&b), 1);
    assert_eq!(c.grapheme_col(&b), 8);

    c.move_buffer_end(&b);
    assert_eq!(c.char_idx(), b.len_chars());

    c.move_buffer_start(&b);
    assert_eq!(c.char_idx(), 0);
}

#[test]
fn horizontal_movement_respects_graphemes() {
    // "🇮🇩" is 2 regional indicators = 2 chars, 1 grapheme
    let b = Buffer::from_str("a🇮🇩b");
    let mut c = Cursor::new();

    c.move_right(&b); // a
    assert_eq!(c.char_idx(), 1);
    c.move_right(&b); // whole flag
    assert_eq!(c.char_idx(), 3);
    c.move_right(&b); // b
    assert_eq!(c.char_idx(), 4);
}
