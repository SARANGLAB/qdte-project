use sarang_qdte::editor::{Action, Editor};

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