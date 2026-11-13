use qdte::app::App;
use qdte::config::Config;
use qdte::editor::Action;

#[test]
fn save_writes_file_and_clears_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let mut app = App::from_file(path.clone(), Config::default()).unwrap();
    assert!(!app.is_dirty());

    for c in "hello".chars() {
        app.apply_editor(Action::InsertChar(c)).unwrap();
    }
    assert!(app.is_dirty());

    app.save().unwrap();
    assert!(!app.is_dirty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
}

#[test]
fn save_without_path_fails() {
    let mut app = App::new_empty(Config::default());
    app.apply_editor(Action::InsertChar('x')).unwrap();
    assert!(app.save().is_err());
}

#[test]
fn movement_does_not_mark_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.txt");
    std::fs::write(&path, "abc").unwrap();

    let mut app = App::from_file(path, Config::default()).unwrap();
    assert!(!app.is_dirty());
    app.apply_editor(Action::MoveRight).unwrap();
    app.apply_editor(Action::MoveLineEnd).unwrap();
    assert!(!app.is_dirty());
}

#[test]
fn undo_back_to_saved_state_is_clean() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.txt");
    std::fs::write(&path, "original").unwrap();

    let mut app = App::from_file(path, Config::default()).unwrap();
    assert!(!app.is_dirty());

    app.apply_editor(Action::MoveBufferEnd).unwrap();
    app.apply_editor(Action::InsertChar('X')).unwrap();
    assert!(app.is_dirty());

    app.apply_editor(Action::Undo).unwrap();
    assert!(!app.is_dirty()); // back to saved state
}

#[test]
fn fixeol_appends_trailing_newline_on_save() {
    use qdte::io::LineEnding;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let mut config = Config::default();
    config.fixeol = true;

    let mut app = App::new_empty(config);
    app.path = Some(path.clone());
    // Force LF so the test is platform-independent.
    app.line_ending = LineEnding::Lf;

    for c in "hello".chars() {
        app.apply_editor(Action::InsertChar(c)).unwrap();
    }
    app.save().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
    // Buffer tidak berubah
    assert_eq!(app.editor.buffer().as_string(), "hello");
    assert!(!app.is_dirty());
}

#[test]
fn fixeol_respects_line_ending() {
    use qdte::io::LineEnding;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let mut config = Config::default();
    config.fixeol = true;

    let mut app = App::new_empty(config);
    app.path = Some(path.clone());
    app.line_ending = LineEnding::Crlf;

    for c in "hello".chars() {
        app.apply_editor(Action::InsertChar(c)).unwrap();
    }
    app.save().unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\r\n");
}