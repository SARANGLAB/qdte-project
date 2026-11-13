use sarang_qdte::app::App;
use sarang_qdte::editor::Action;

#[test]
fn save_writes_file_and_clears_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let mut app = App::from_file(path.clone()).unwrap();
    assert!(!app.dirty);

    for c in "hello".chars() {
        app.apply_editor(Action::InsertChar(c)).unwrap();
    }
    assert!(app.dirty);

    app.save().unwrap();
    assert!(!app.dirty);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
}

#[test]
fn save_without_path_fails() {
    let mut app = App::new_empty();
    app.apply_editor(Action::InsertChar('x')).unwrap();
    assert!(app.save().is_err());
}

#[test]
fn movement_does_not_mark_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.txt");
    std::fs::write(&path, "abc").unwrap();

    let mut app = App::from_file(path).unwrap();
    assert!(!app.dirty);
    app.apply_editor(Action::MoveRight).unwrap();
    app.apply_editor(Action::MoveLineEnd).unwrap();
    assert!(!app.dirty);
}