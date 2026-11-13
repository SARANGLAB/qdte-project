use std::process::ExitCode;

use sarang_qdte::app::App;
use sarang_qdte::cli;
use sarang_qdte::keymap::dispatcher::Dispatcher;
use sarang_qdte::keymap::normal::NormalKeymap;
use sarang_qdte::keymap::Command;
use sarang_qdte::terminal::input::read_key;
use sarang_qdte::terminal::renderer::Renderer;
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

    if let Err(e) = run(&mut app) {
        eprintln!("terminal error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(app: &mut App) -> std::io::Result<()> {
    let _guard = TerminalGuard::new()?;
    let dispatcher = Dispatcher::new(NormalKeymap);
    let mut renderer = Renderer::new();
    let mut quit_armed = false;

    loop {
        renderer.render(app)?;

        let key = read_key()?;
        let Some(cmd) = dispatcher.resolve(key) else {
            continue;
        };

        match cmd {
            Command::Editor(action) => {
                app.message = None;
                if let Err(e) = app.apply_editor(action) {
                    app.message = Some(format!("error: {}", e));
                }
                quit_armed = false;
            }
            Command::Save => {
                match app.save() {
                    Ok(()) => {} // save() sets app.message
                    Err(e) => app.message = Some(format!("save failed: {}", e)),
                }
                quit_armed = false;
            }
            Command::Quit => {
                if app.dirty && !quit_armed {
                    app.message =
                        Some("unsaved changes — Ctrl+X again to quit".into());
                    quit_armed = true;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}