use qdte::editor::{Action, Editor};

#[test]
fn navigate_by_words() {
    let mut e = Editor::from_str("the quick brown fox");
    e.apply(Action::WordForward).unwrap();
    assert_eq!(e.cursor().char_idx(), 4);
    e.apply(Action::WordForward).unwrap();
    assert_eq!(e.cursor().char_idx(), 10);
    e.apply(Action::WordBackward).unwrap();
    assert_eq!(e.cursor().char_idx(), 4);
    e.apply(Action::MoveBufferStart).unwrap();
    e.apply(Action::WordEnd).unwrap();
    assert_eq!(e.cursor().char_idx(), 2); // 'e' of "the"
}

#[test]
fn yank_and_paste_roundtrip() {
    let mut e = Editor::from_str("alpha\nbeta\ngamma");
    // Yank line 0 ("alpha")
    e.apply(Action::YankLine).unwrap();
    // Paste below → "alpha\nalpha\nbeta\ngamma", cursor on new line 1
    e.apply(Action::PasteAfter).unwrap();
    assert_eq!(e.buffer().as_string(), "alpha\nalpha\nbeta\ngamma");
    assert_eq!(e.cursor().line(e.buffer()), 1);

    // Move down to "beta" (line 2). One step, not two.
    e.apply(Action::MoveDown).unwrap();
    assert_eq!(e.cursor().line(e.buffer()), 2);

    // Paste above → "alpha\nalpha\nalpha\nbeta\ngamma"
    e.apply(Action::PasteBefore).unwrap();
    assert_eq!(
        e.buffer().as_string(),
        "alpha\nalpha\nalpha\nbeta\ngamma"
    );
}

#[test]
fn yank_paste_undo_restores() {
    let mut e = Editor::from_str("one\ntwo");
    e.apply(Action::YankLine).unwrap();
    e.apply(Action::PasteAfter).unwrap();
    e.apply(Action::Undo).unwrap();
    assert_eq!(e.buffer().as_string(), "one\ntwo");
    e.apply(Action::Redo).unwrap();
    assert_eq!(e.buffer().as_string(), "one\none\ntwo");
}

#[test]
fn big_word_skips_punctuation() {
    let mut e = Editor::from_str("foo.bar baz");
    e.apply(Action::WordForwardBig).unwrap();
    assert_eq!(e.cursor().char_idx(), 8); // 'b' of baz
}