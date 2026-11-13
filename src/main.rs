use std::process::ExitCode;

use sarang_qdte::cli;
use sarang_qdte::editor::Editor;
use sarang_qdte::io;
use sarang_qdte::terminal::input::{map_key, read_key, InputOutcome};
use sarang_qdte::terminal::TerminalGuard;

fn main() -> ExitCode {
    let args = cli::args::parse();

    let (initial, path_display) = match args.cli.file {
        Some(path) => match io::reader::read(&path) {
            Ok(loaded) => (loaded.content, format!("{:?}", loaded.path)),
            Err(e) => {
                eprintln!("error reading {:?}: {}", path, e);
                return ExitCode::FAILURE;
            }
        },
        None => (String::new(), "<no file>".to_string()),
    };

    println!("QDTE — file: {}", path_display);
    println!("Press Ctrl+Q to quit.");
    println!();

    if let Err(e) = run_editor(&initial) {
        eprintln!("terminal error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run_editor(initial: &str) -> std::io::Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut editor = Editor::from_str(initial);

    loop {
        let Some(key) = read_key()? else { continue };
        match map_key(key) {
            InputOutcome::Quit => break,
            InputOutcome::Action(action) => {
                let repr = format!("{:?}", action);
                let _ = editor.apply(action);
                log_state(&editor, &repr);
            }
            InputOutcome::Ignore => {}
        }
    }

    // _guard drops here, restoring raw mode + leaving alternate screen.
    // Buffer content survives because Editor is owned by this function.
    println!("--- final buffer ---");
    print!("{}", editor.buffer().as_string());
    if !editor.buffer().as_string().ends_with('\n') && !editor.buffer().as_string().is_empty() {
        println!();
    }
    println!("--- cursor: char {} (line {}, col {}) ---",
        editor.cursor().char_idx(),
        editor.cursor().line(editor.buffer()),
        editor.cursor().grapheme_col(editor.buffer()),
    );
    Ok(())
}

fn log_state(editor: &Editor, action_repr: &str) {
    let text = editor.buffer().as_string();
    let preview = if text.len() > 60 {
        format!("{}…", &text[..60])
    } else {
        text
    };
    println!(
        "{}  →  cursor {}  buf {:?}",
        action_repr,
        editor.cursor().char_idx(),
        preview
    );
}