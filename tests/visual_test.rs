use qdte::app::App;
use qdte::config::Config;
use qdte::editor::{Action, Mode};

fn app_with(s: &str) -> App {
    let mut app = App::new_empty(Config::default());
    app.editor = qdte::editor::Editor::from_str(s);
    app
}

#[test]
fn delete_visual_char_range() {
    let mut app = app_with("hello world");
    app.selection_anchor = Some(0);
    app.mode = Mode::Visual;
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap(); // cursor at 4 ('o')
    let (start, end, linewise) = app.visual_range().unwrap();
    assert!(!linewise);
    app.editor.apply(Action::DeleteRange { start, end }).unwrap();
    assert_eq!(app.editor.buffer().as_string(), " world");
}

#[test]
fn yank_visual_char_range() {
    let mut app = app_with("hello world");
    app.selection_anchor = Some(0);
    app.mode = Mode::Visual;
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap();
    app.editor.apply(Action::MoveRight).unwrap(); // cursor at 4
    let (start, end, linewise) = app.visual_range().unwrap();
    app.editor.apply(Action::YankRange { start, end, linewise }).unwrap();
    assert_eq!(app.editor.register().content, "hello");
    assert_eq!(app.editor.buffer().as_string(), "hello world");
}

#[test]
fn visual_line_range_covers_full_lines() {
    let mut app = app_with("one\ntwo\nthree");
    app.selection_anchor = Some(0); // line 0
    app.mode = Mode::VisualLine;
    app.editor.apply(Action::MoveDown).unwrap(); // line 1
    let (start, end, linewise) = app.visual_range().unwrap();
    assert!(linewise);
    assert_eq!(start, 0);
    assert_eq!(end, 8); // end of "two\n"
}

#[test]
fn visual_line_yank_then_paste_duplicates() {
    let mut app = app_with("a\nb\nc");
    app.selection_anchor = Some(0);
    app.mode = Mode::VisualLine;
    // Yank line 0 only
    let (s, e, lw) = app.visual_range().unwrap();
    app.editor.apply(Action::YankRange { start: s, end: e, linewise: lw }).unwrap();
    // Move down twice and paste
    app.editor.apply(Action::MoveDown).unwrap();
    app.editor.apply(Action::MoveDown).unwrap();
    app.editor.apply(Action::PasteAfter).unwrap();
    assert_eq!(app.editor.buffer().as_string(), "a\nb\nc\na\n");
}