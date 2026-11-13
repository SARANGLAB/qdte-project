use qdte::editor::{Action, Editor};

#[test]
fn type_a_sentence() {
    let mut e = Editor::new();
    for c in "hello world".chars() {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    assert_eq!(e.buffer().as_string(), "hello world");
    assert_eq!(e.cursor().char_idx(), 11);
}

#[test]
fn edit_then_undo_via_backspace() {
    let mut e = Editor::from_str("hello!");
    e.apply(Action::MoveBufferEnd).unwrap();
    e.apply(Action::DeleteBackward).unwrap(); // remove '!'
    e.apply(Action::InsertChar('?')).unwrap(); // replace with '?'
    assert_eq!(e.buffer().as_string(), "hello?");
}

#[test]
fn multiline_edit_flow() {
    let mut e = Editor::from_str("line one\nline two");
    e.apply(Action::MoveLineEnd).unwrap();
    e.apply(Action::InsertNewline).unwrap();
    e.apply(Action::InsertChar('X')).unwrap();
    assert_eq!(e.buffer().as_string(), "line one\nX\nline two");
    assert_eq!(e.cursor().line(e.buffer()), 1);
    assert_eq!(e.cursor().grapheme_col(e.buffer()), 1);
}

#[test]
fn enter_then_type_creates_expected_buffer() {
    use qdte::editor::{Action, Editor};

    let mut e = Editor::new();
    for c in ['a', '\t', 'b'] {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    assert_eq!(e.buffer().as_string(), "a\tb");
    assert_eq!(e.cursor().char_idx(), 3);

    e.apply(Action::InsertNewline).unwrap();
    assert_eq!(e.buffer().as_string(), "a\tb\n");
    assert_eq!(e.cursor().line(e.buffer()), 1);
    assert_eq!(e.cursor().char_idx(), 4);

    e.apply(Action::InsertChar('a')).unwrap();
    e.apply(Action::InsertChar('b')).unwrap();
    assert_eq!(e.buffer().as_string(), "a\tb\nab");

    e.apply(Action::MoveUp).unwrap();
    assert_eq!(e.cursor().line(e.buffer()), 0);
    assert_eq!(e.cursor().grapheme_col(e.buffer()), 2);
}

#[test]
fn enter_at_end_puts_cursor_on_new_last_line() {
    use qdte::editor::{Action, Editor};
    let mut e = Editor::new();
    for c in ['a', '\t', 'b'] {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    assert_eq!(e.cursor().line(e.buffer()), 0);

    e.apply(Action::InsertNewline).unwrap();
    assert_eq!(e.buffer().as_string(), "a\tb\n");
    assert_eq!(e.cursor().line(e.buffer()), 1);
    assert_eq!(e.cursor().grapheme_col(e.buffer()), 0);
}
