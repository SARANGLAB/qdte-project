pub mod input;
pub mod renderer;

use std::io::{self, stdout};

use crossterm::{
    cursor::SetCursorStyle,
    execute, queue,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(e) = execute!(stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(e);
        }
        Ok(Self { active: true })
    }

    pub fn restore(&mut self) {
        if self.active {
            let _ = queue!(stdout(), SetCursorStyle::DefaultUserShape);
            let _ = execute!(stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            self.active = false;
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.restore();
    }
}