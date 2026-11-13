use std::process::ExitCode;

use qdte::app::App;
use qdte::cli::args::Invocation;
use qdte::cli;
use qdte::config::{Config, KeymapKind};
use qdte::keymap::dispatcher::Dispatcher;
use qdte::keymap::Command;
use qdte::terminal::input::read_key;
use qdte::terminal::renderer::Renderer;
use qdte::terminal::TerminalGuard;

fn main() -> ExitCode {
    let (config, warning) = Config::load();
    if let Some(w) = warning {
        eprintln!("qdte: {}", w);
    }

    let args = cli::args::parse();

    let mut app = match args.cli.file {
        Some(path) => match App::from_file(path.clone(), config.clone()) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error reading {:?}: {}", path, e);
                return ExitCode::FAILURE;
            }
        },
        None => App::new_empty(config.clone()),
    };

    let vim = match config.keymap {
        Some(KeymapKind::Vim) => true,
        Some(KeymapKind::Normal) => false,
        None => matches!(args.invocation, Invocation::Vim),
    };

    if let Err(e) = run(&mut app, vim, &config) {
        eprintln!("terminal error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(app: &mut App, vim: bool, config: &Config) -> std::io::Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut dispatcher = Dispatcher::for_vim_mode(vim);
    let mut renderer = Renderer::new(config.tab_width);
    let mut quit_armed = false;

    app.mode = dispatcher.initial_mode();

    loop {
        let mode_label = if dispatcher.shows_mode() {
            app.mode.label()
        } else {
            ""
        };
        renderer.render(app, mode_label)?;

        let key = read_key()?;
        let Some(cmd) = dispatcher.resolve(app.mode, key) else {
            continue;
        };

        if dispatch_command(app, cmd, &mut quit_armed) {
            break;
        }
    }
    Ok(())
}

/// Returns true if the app should quit.
fn dispatch_command(app: &mut App, cmd: Command, quit_armed: &mut bool) -> bool {
    match cmd {
        Command::Seq(cmds) => {
            for c in cmds {
                if dispatch_command(app, c, quit_armed) {
                    return true;
                }
            }
            false
        }
        Command::Editor(action) => {
            app.message = None;
            if let Err(e) = app.apply_editor(action) {
                app.message = Some(format!("error: {}", e));
            }
            *quit_armed = false;
            false
        }
        Command::SetMode(mode) => {
            app.mode = mode;
            app.message = None;
            *quit_armed = false;
            false
        }
        Command::Save => {
            match app.save() {
                Ok(()) => {}
                Err(e) => app.message = Some(format!("save failed: {}", e)),
            }
            *quit_armed = false;
            false
        }
        Command::Quit => {
            if app.is_dirty() && !*quit_armed {
                app.message = Some("unsaved changes — Ctrl+X again to quit".into());
                *quit_armed = true;
                false
            } else {
                true
            }
        }
    }
}
