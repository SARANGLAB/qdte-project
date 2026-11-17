use std::io::{self, Write};
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

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use unicode_width::UnicodeWidthStr;

fn main() -> ExitCode {
    let args = cli::args::parse();

    if let Some(path) = &args.cli.file {
        if let Some(name) = path.to_str() {
            if name == "tutor" {
                run_tutor(false);
                return ExitCode::SUCCESS;
            } else if name == "vi-tutor" {
                run_tutor(true);
                return ExitCode::SUCCESS;
            }
        }
    }

    let (mut config, warning) = Config::load();
    if let Some(w) = warning {
        eprintln!("qdte: {}", w);
    }

    if Config::path().map_or(false, |p| !p.exists()) {
        run_setup(&mut config);
    }

    let mut app = match args.cli.file {
        Some(path) => {
            if path.is_dir() {
                eprintln!("qdte: '{}' is a directory, not a file.", path.display());
                return ExitCode::FAILURE;
            }
            match App::from_file(path.clone(), config.clone()) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("error reading {:?}: {}", path, e);
                    return ExitCode::FAILURE;
                }
            }
        }
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

fn run_setup(config: &mut Config) {
    println!();
    println!("Welcome to QDTE!");
    println!("It looks like this is your first time running QDTE.");
    println!();
    println!("Please choose your default keymap:");
    println!("  1. Normal (for average users) [Default]");
    println!("  2. Vi (for advanced users)");
    print!("Choice (1/2): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim() == "2" {
        config.keymap = Some(KeymapKind::Vim);
    } else {
        config.keymap = Some(KeymapKind::Normal);
    }

    if let Err(e) = config.save_default() {
        eprintln!("\nFailed to save config: {}", e);
    } else {
        println!("\nConfiguration saved!");
    }

    println!();
    println!("To view the keymap tutorial in the future, run:");
    println!("  qdte tutor");
    println!("  qdte vi-tutor");
    println!();
    println!("Press Enter to continue to the editor...");
    let mut _dummy = String::new();
    io::stdin().read_line(&mut _dummy).unwrap();
}

fn run_tutor(is_vim: bool) {
    if is_vim {
        println!("=== QDTE Vim Keybindings ===");
        println!("\n[Normal Mode]");
        println!("  h j k l   : Move cursor");
        println!("  w b e     : Next/Prev word, End of word");
        println!("  i a I A   : Insert/Append (start/end of line)");
        println!("  o O       : Open line below/above");
        println!("  x dd      : Delete char / line");
        println!("  yy p P    : Yank line / Paste below / above");
        println!("  v V       : Visual / Visual Line mode");
        println!("  u Ctrl+r  : Undo / Redo");
        println!("  :         : Command mode (w, q, wq, e)");
        println!("  / n N     : Search, Next match, Prev match");
        println!("\n[Insert Mode]");
        println!("  Esc       : Return to Normal mode");
        println!("  Ctrl+s    : Save");
    } else {
        println!("=== QDTE Normal Keybindings ===");
        println!("\n[Editing]");
        println!("  Ctrl+s    : Save");
        println!("  Ctrl+Shift+s / F2 : Save As");
        println!("  Ctrl+o    : Open file");
        println!("  Ctrl+z/y  : Undo / Redo");
        println!("  Ctrl+x/q/c: Quit");
        println!("\n[Navigation]");
        println!("  Arrows    : Move cursor");
        println!("  Home End  : Start / End of line");
        println!("  Backspace : Delete before");
        println!("  Delete    : Delete after");
        println!("\n[Search]");
        println!("  Ctrl+f    : Search");
        println!("  F3 / Shift+F3 : Next / Prev match");
        println!("  Esc       : Clear search highlight");
    }
    println!("\nPress Enter to exit...");
    let mut _dummy = String::new();
    io::stdin().read_line(&mut _dummy).unwrap();
}

fn run(app: &mut App, vim: bool, config: &Config) -> std::io::Result<()> {
    let mut _guard = TerminalGuard::new()?;
    let mut dispatcher = Dispatcher::for_vim_mode(vim);
    let mut renderer = Renderer::new(config.tab_width);
    let mut quit_armed = false;

    app.mode = dispatcher.initial_mode();

    // Tampilkan Splash Screen jika ini adalah buffer kosong (tanpa file)
    if app.path.is_none() && app.editor.buffer().is_empty() {
        show_splash_screen(app)?;
    }

    loop {
        if app.should_quit {
            break;
        }

        // --- PINDAHKAN BLOK SHELL COMMAND KE SINI ---
        if let Some(cmd) = app.pending_shell_command.take() {
            // 1. Keluar dari TUI (raw mode & alternate screen)
            _guard.suspend()?;

            // 2. Bersihkan layar dan tampilkan teks segera
            use crossterm::{execute, cursor::MoveTo, terminal::{Clear, ClearType}, style::Print};
            let mut out = std::io::stdout();
            let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));
            let _ = execute!(out, Print(format!("Executing: {}\n\n", cmd)));
            let _ = out.flush();

            // 3. Jalankan perintah shell
            #[cfg(target_os = "windows")]
            let status = std::process::Command::new("cmd")
                .arg("/C")
                .arg(&cmd)
                .status();

            #[cfg(not(target_os = "windows"))]
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .status();

            // Tangkap Status Exit
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        app.message = Some("Command executed successfully".to_string());
                    } else {
                        let code = exit_status.code().unwrap_or(-1);
                        app.message = Some(format!("Command exited with code: {}", code));
                    }
                }
                Err(e) => {
                    let _ = execute!(out, Print(format!("\nFailed to execute: {}\n", e)));
                    app.message = Some(format!("Execution error: {}", e));
                }
            }

            // 4. Tampilkan pesan untuk kembali
            let _ = execute!(out, Print("\nPress any key to return to QDTE..."));
            let _ = out.flush();

            // 5. Tunggu user menekan tombol apa saja
            let _ = crossterm::terminal::enable_raw_mode();
            let _ = read_key();
            let _ = crossterm::terminal::disable_raw_mode();

            // 6. Kembali ke TUI
            _guard.resume()?;
            
            // 7. Paksa render ulang editor pada iterasi berikutnya
            renderer.invalidate();
            
            continue;
        }
        // ---------------------------------------------

        let mode_label = if dispatcher.shows_mode() {
            app.mode.label()
        } else {
            ""
        };
        let block_cursor = dispatcher.block_cursor(app.mode);
        renderer.render(app, mode_label, block_cursor)?;

        let key = read_key()?;

        // 1. Handle Theme Picker input
        if app.theme_picker.is_some() {
            use crossterm::event::KeyCode;
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => app.theme_picker_up(),
                KeyCode::Down | KeyCode::Char('j') => app.theme_picker_down(),
                KeyCode::Enter => app.theme_picker_select(),
                KeyCode::Esc => app.close_theme_picker(),
                _ => {}
            }
            continue;
        }

        // 2. Handle Prompt input
        if app.prompt.is_some() {
            handle_prompt_key(app, key);
            continue;
        }

        // 3. Handle Editor input
        let Some(cmd) = dispatcher.resolve(app.mode, key) else {
            continue;
        };

        if dispatch_command(app, cmd, &mut quit_armed) {
            break;
        }
    }
    Ok(())
}

fn show_splash_screen(app: &App) -> std::io::Result<()> {
    let (cols, rows) = crossterm::terminal::size()?;
    let ui = &app.theme_manager.active().ui;
    let mut stdout = io::stdout();

    queue!(
        stdout,
        Clear(ClearType::All),
        SetBackgroundColor(ui.bg),
        SetForegroundColor(ui.fg)
    )?;

    let art = r#"
       ██████╗ ██████╗ ████████╗███████╗
      ██╔═══██╗██╔══██╗╚══██╔══╝██╔════╝
      ██║   ██║██║  ██║   ██║   █████╗  
      ██║▄▄ ██║██║  ██║   ██║   ██╔══╝  
      ╚██████╔╝██████╔╝   ██║   ███████╗
       ╚══▀▀═╝ ╚═════╝    ╚═╝   ╚══════╝
    "#;

    let quotes = [
        "\"The quick brown fox jumps over the lazy dog.\"",
        "\"Also try Sky CLI Music Player!\"",
        "\"Quick & Dirty Text Editor. Simple, lightweight, native.\"",
        "\"Talk is cheap. Show me the code. — Linus Torvalds\"",
        "\"First, solve the problem. Then, write the code.\"",
        "\"Code is like humor. When you have to explain it, it’s bad.\"",
        "\"Explore more projects from SARANGLAB!\"",
        "\"Build what you need. Keep what works. — SARANGLAB Community\"",
        "\"And the universe said I love you. — Julian Gough\"",
        "\"Complexity is easy to create. Simplicity takes effort.\"",
    ];

    // Pilih quote acak berdasarkan waktu saat ini
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let quote = quotes[(now as usize) % quotes.len()];

    let art_lines: Vec<&str> = art.lines().collect();
    let art_height = art_lines.len();
    
    // Hitung posisi vertikal (ditambah offset karena ada teks donasi)
    let start_y = rows.saturating_sub(art_height as u16 + 9) / 2;
    
    // 1. Gambar ASCII Art
    for (i, line) in art_lines.iter().enumerate() {
        let width = UnicodeWidthStr::width(*line);
        let start_x = cols.saturating_sub(width as u16) / 2;
        queue!(
            stdout,
            MoveTo(start_x, start_y + i as u16),
            SetForegroundColor(ui.selection), // Warna ASCII Art pakai warna seleksi tema
            Print(line)
        )?;
    }

    // 2. Gambar Quote Acak
    let q_width = UnicodeWidthStr::width(quote);
    let q_x = cols.saturating_sub(q_width as u16) / 2;
    queue!(
        stdout,
        MoveTo(q_x, start_y + art_height as u16 + 2),
        SetForegroundColor(ui.fg),
        Print(quote)
    )?;

    // 3. Gambar Teks Donasi Persisten
    let donation_text = "Help donate to victims of forest fires & national disasters in Indonesia";
    let donation_link = "https://digital.dompetdhuafa.org/"; 

    let d_width = UnicodeWidthStr::width(donation_text);
    let d_x = cols.saturating_sub(d_width as u16) / 2;
    queue!(
        stdout,
        MoveTo(d_x, start_y + art_height as u16 + 4),
        SetForegroundColor(ui.statusbar_fg), 
        Print(donation_text)
    )?;

    let link_width = UnicodeWidthStr::width(donation_link);
    let link_x = cols.saturating_sub(link_width as u16) / 2;
    queue!(
        stdout,
        MoveTo(link_x, start_y + art_height as u16 + 5),
        SetForegroundColor(ui.cursor), 
        Print(donation_link)
    )?;

    // 4. Gambar Petunjuk "Press any key"
    let hint = "Press any key to continue...";
    let hint_width = UnicodeWidthStr::width(hint);
    let hint_x = cols.saturating_sub(hint_width as u16) / 2;
    queue!(
        stdout,
        MoveTo(hint_x, start_y + art_height as u16 + 7),
        SetForegroundColor(ui.gutter),
        Print(hint)
    )?;

    stdout.flush()?;

    // Tunggu pengguna menekan tombol apa saja
    let _ = read_key()?;

    // Bersihkan layar sebelum masuk ke editor
    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    stdout.flush()?;

    Ok(())
}

fn handle_prompt_key(app: &mut App, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => app.cancel_prompt(),
        (KeyCode::Enter, _) => app.confirm_prompt(),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.insert_char(c);
            }
            app.update_live_search();
        }
        (KeyCode::Backspace, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.backspace();
            }
            app.update_live_search();
        }
        (KeyCode::Delete, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.delete();
            }
            app.update_live_search();
        }
        (KeyCode::Left, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.move_left();
            }
        }
        (KeyCode::Right, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.move_right();
            }
        }
        (KeyCode::Home, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.move_home();
            }
        }
        (KeyCode::End, _) => {
            if let Some(prompt) = app.prompt.as_mut() {
                prompt.move_end();
            }
        }
        _ => {}
    }
}

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
            if app.mode == qdte::editor::Mode::Insert && mode == qdte::editor::Mode::Normal {
                let _ = app.editor.apply(qdte::editor::Action::MoveLeftLineClamped);
            }
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
        Command::SaveAs => {
            app.open_prompt(qdte::app::PromptKind::SaveAs);
            *quit_armed = false;
            false
        }
        Command::Open => {
            app.open_prompt(qdte::app::PromptKind::Open);
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
        Command::EnterVisual(mode) => {
            app.selection_anchor = Some(app.editor.cursor().char_idx());
            app.mode = mode;
            app.message = None;
            *quit_armed = false;
            false
        }
        Command::ExitVisual => {
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            app.message = None;
            *quit_armed = false;
            false
        }
        Command::DeleteVisualSelection => {
            if let Some((start, end, _)) = app.visual_range() {
                if let Err(e) = app.editor.apply(qdte::editor::Action::DeleteRange { start, end }) {
                    app.message = Some(format!("error: {}", e));
                }
            }
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            *quit_armed = false;
            false
        }
        Command::YankVisualSelection => {
            if let Some((start, end, linewise)) = app.visual_range() {
                if let Err(e) = app.editor.apply(qdte::editor::Action::YankRange {
                    start,
                    end,
                    linewise,
                }) {
                    app.message = Some(format!("error: {}", e));
                }
            }
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            *quit_armed = false;
            false
        }
        Command::EnterCommandPrompt => {
            app.prompt = Some(qdte::app::Prompt::new(qdte::app::PromptKind::Command, String::new()));
            *quit_armed = false;
            false
        }
        Command::SwapAnchor => {
            if let Some(anchor) = app.selection_anchor {
                let cursor = app.editor.cursor().char_idx();
                app.selection_anchor = Some(cursor);
                app.editor.set_cursor(anchor);
            }
            *quit_armed = false;
            false
        }
        Command::Search => {
            app.open_prompt(qdte::app::PromptKind::Search);
            *quit_armed = false;
            false
        }
        Command::SearchNext => {
            app.search_next();
            *quit_armed = false;
            false
        }
        Command::SearchPrev => {
            app.search_prev();
            *quit_armed = false;
            false
        }
        Command::ClearSearch => {
            app.search = None;
            app.message = None;
            *quit_armed = false;
            false
        }
        Command::OpenThemePicker => {
            app.open_theme_picker();
            *quit_armed = false;
            false
        }
        Command::ToggleCase => {
            app.message = None;
            let _ = app.editor.apply(qdte::editor::Action::ToggleCase);
            *quit_armed = false;
            false
        }
        Command::ToggleVisualSelection => {
            if let Some((start, end, _)) = app.visual_range() {
                let _ = app.editor.apply(qdte::editor::Action::ToggleCaseRange { start, end });
            }
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            *quit_armed = false;
            false
        }
        Command::Indent => {
            let _ = app.editor.apply(qdte::editor::Action::IndentLine);
            *quit_armed = false;
            false
        }
        Command::Unindent => {
            let _ = app.editor.apply(qdte::editor::Action::UnindentLine);
            *quit_armed = false;
            false
        }
        Command::IndentVisual => {
            if let Some((start, end, _)) = app.visual_range() {
                let start_line = app.editor.buffer().char_to_line(start);
                let end_line = app.editor.buffer().char_to_line(end.saturating_sub(1));
                let _ = app.editor.apply(qdte::editor::Action::IndentRange { start_line, end_line });
            }
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            *quit_armed = false;
            false
        }
        Command::UnindentVisual => {
            if let Some((start, end, _)) = app.visual_range() {
                let start_line = app.editor.buffer().char_to_line(start);
                let end_line = app.editor.buffer().char_to_line(end.saturating_sub(1));
                let _ = app.editor.apply(qdte::editor::Action::UnindentRange { start_line, end_line });
            }
            app.mode = qdte::editor::Mode::Normal;
            app.selection_anchor = None;
            *quit_armed = false;
            false
        }
    }
}