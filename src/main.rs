use std::process::ExitCode;

use sarang_qdte::app::App;
use sarang_qdte::cli;
use sarang_qdte::keymap::dispatcher::Dispatcher;
use sarang_qdte::keymap::normal::NormalKeymap;
use sarang_qdte::keymap::Command;
use sarang_qdte::terminal::input::read_key;
use sarang_qdte::terminal::TerminalGuard;

fn main() -> ExitCode {
    let args = cli::args::parse();

    let mut app = match args.cli.file {
        Some(path) => match App::from_file(path.clone()) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error reading {:?}: {}", path, e);
                return ExitCode::FAILURE;
            }
        },
        None => App::new_empty(),
    };

    let label = app
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<no file>".to_string());
    println!("QDTE — file: {}", label);
    println!("Ctrl+S save · Ctrl+X quit · arrows/backspace/enter as expected");
    println!();

    if let Err(e) = run(&mut app) {
        eprintln!("terminal error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(app: &mut App) -> std::io::Result<()> {
    let _guard = TerminalGuard::new()?;
    let dispatcher = Dispatcher::new(NormalKeymap);
    let mut quit_armed = false;

    loop {
        let key = read_key()?;
        let Some(cmd) = dispatcher.resolve(key) else {
            continue;
        };

        match cmd {
            Command::Editor(action) => {
                let repr = format!("{:?}", action);
                match app.apply_editor(action) {
                    Ok(()) => log_state(app, &repr),
                    Err(e) => println!("[editor error: {}]", e),
                }
                quit_armed = false;
            }
            Command::Save => {
                match app.save() {
                    Ok(()) => println!("[saved {}]", app.path.as_ref().unwrap().display()),
                    Err(e) => println!("[save error: {}]", e),
                }
                quit_armed = false;
            }
            Command::Quit => {
                if app.dirty && !quit_armed {
                    println!("[unsaved changes — press Ctrl+X again to quit without saving]");
                    quit_armed = true;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn log_state(app: &App, action_repr: &str) {
    let text = app.editor.buffer().as_string();
    let preview = if text.chars().count() > 60 {
        let s: String = text.chars().take(60).collect();
        format!("{}…", s)
    } else {
        text
    };
    println!(
        "{}  →  cursor {}  buf {:?}",
        action_repr,
        app.editor.cursor().char_idx(),
        preview
    );
}