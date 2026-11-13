pub mod dispatcher;
pub mod normal;
pub mod vim;

use crossterm::event::KeyEvent;

use crate::editor::{Action, Mode};

#[derive(Debug, Clone)]
pub enum Command {
    Editor(Action),
    /// Sequence of commands to apply in order.
    Seq(Vec<Command>),
    SetMode(Mode),
    Save,
    Quit,
}

pub trait Keymap {
    fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command>;

    fn initial_mode(&self) -> Mode {
        Mode::Normal
    }

    fn shows_mode(&self) -> bool {
        false
    }
}

impl Keymap for Box<dyn Keymap> {
    fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command> {
        (**self).resolve(mode, key)
    }
    fn initial_mode(&self) -> Mode {
        (**self).initial_mode()
    }
    fn shows_mode(&self) -> bool {
        (**self).shows_mode()
    }
}