pub mod input;
pub mod renderer;

use std::io::{self, stdout};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

/// RAII guard for terminal raw mode + alternate screen.
///
/// Restoration happens in `Drop`, so it works even on panic (during
/// unwinding) and on early return. If the guard is dropped twice or
/// explicitly, restoration still runs only once.
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(e) = execute!(stdout(), EnterAlternateScreen) {
            // Roll back raw mode if alt screen failed.
            let _ = disable_raw_mode();
            return Err(e);
        }
        Ok(Self { active: true })
    }

    /// Explicitly restore the terminal. Called automatically on drop.
    pub fn restore(&mut self) {
        if self.active {
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
