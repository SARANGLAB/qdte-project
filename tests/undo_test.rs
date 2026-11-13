use qdte::editor::{Action, Editor};

#[test]
fn undo_single_insert() {
    let mut e = Editor::from_str("");
    e.apply(Action::InsertChar('a')).unwrap();
    e.apply(Action::InsertChar('b')).unwrap();
    e.apply(Action::InsertChar('c')).unwrap();
    assert_eq!(e.buffer().as_string(), "abc");

    e.apply(Action::Undo).unwrap();
    // All three coalesced into one change
    assert_eq!(e.buffer().as_string(), "");
    assert_eq!(e.cursor().char_idx(), 0);
}

#[test]
fn undo_typing_bursts_separated_by_navigation() {
    let mut e = Editor::from_str("");
    for c in ['a', 'b', 'c'] {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    // Cursor at 3, buffer "abc"
    e.apply(Action::MoveLineStart).unwrap();
    for c in ['X', 'Y'] {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    // Buffer "XYabc", cursor at 2

    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "abc");
    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "");
}

#[test]
fn undo_newline_is_separate() {
    let mut e = Editor::from_str("");
    e.apply(Action::InsertChar('a')).unwrap();
    e.apply(Action::InsertNewline).unwrap();
    e.apply(Action::InsertChar('b')).unwrap();
    assert_eq!(e.buffer().as_string(), "a\nb");

    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "a\n");
    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "a");
    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "");
}

#[test]
fn undo_backspace_restores_char() {
    let mut e = Editor::from_str("abc");
    e.apply(Action::MoveBufferEnd).unwrap();
    e.apply(Action::DeleteBackward).unwrap();
    assert_eq!(e.buffer().as_string(), "ab");

    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "abc");
    assert_eq!(e.cursor().char_idx(), 3);
}

#[test]
fn undo_delete_line_restores() {
    let mut e = Editor::from_str("one\ntwo\nthree");
    e.apply(Action::MoveDown).unwrap(); // on line 1 "two"
    let cursor_before = e.cursor().char_idx();
    e.apply(Action::DeleteLine).unwrap();
    assert_eq!(e.buffer().as_string(), "one\nthree");

    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "one\ntwo\nthree");
    assert_eq!(e.cursor().char_idx(), cursor_before);
}

#[test]
fn redo_after_undo() {
    let mut e = Editor::from_str("");
    e.apply(Action::InsertChar('a')).unwrap();
    e.apply(Action::InsertChar('b')).unwrap();
    assert_eq!(e.buffer().as_string(), "ab");

    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "");

    e.apply(Action::Redo).unwrap();
    assert_eq!(e.buffer().as_string(), "ab");
}

#[test]
fn new_change_clears_redo() {
    let mut e = Editor::from_str("");
    e.apply(Action::InsertChar('a')).unwrap();
    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "");

    e.apply(Action::InsertChar('b')).unwrap();
    assert_eq!(e.buffer().as_string(), "b");

    e.apply(Action::Redo).unwrap();
    assert_eq!(e.buffer().as_string(), "b"); // redo was cleared
}

#[test]
fn undo_at_empty_does_nothing() {
    let mut e = Editor::from_str("");
    e.apply(Action::Undo).unwrap();
    e.apply(Action::Redo).unwrap();
    assert_eq!(e.buffer().as_string(), "");
}

#[test]
fn undo_restores_after_multiple_ops() {
    let mut e = Editor::from_str("");
    for c in "hello".chars() {
        e.apply(Action::InsertChar(c)).unwrap();
    }
    e.apply(Action::MoveLineStart).unwrap();
    e.apply(Action::DeleteForward).unwrap(); // delete 'h'
    e.apply(Action::DeleteForward).unwrap(); // delete 'e'
    assert_eq!(e.buffer().as_string(), "llo");

    e.apply(Action::Undo).unwrap(); // restore 'e'
    e.apply(Action::Undo).unwrap(); // restore 'h'
    assert_eq!(e.buffer().as_string(), "hello");
    assert_eq!(e.cursor().char_idx(), 0);
}