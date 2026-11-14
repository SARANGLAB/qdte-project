pub mod dispatcher;
pub mod normal;
pub mod vim;

use crossterm::event::KeyEvent;

use crate::editor::{Action, Mode};

#[derive(Debug, Clone)]
pub enum Command {
    Editor(Action),
    Seq(Vec<Command>),
    SetMode(Mode),
    Save,
    SaveAs,
    Open,
    Quit,
    EnterVisual(Mode),
    ExitVisual,
    DeleteVisualSelection,
    YankVisualSelection,
    EnterCommandPrompt,
    SwapAnchor,
    Search,
    SearchNext,
    SearchPrev,
    ClearSearch,
}

pub trait Keymap {
    fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command>;

    fn initial_mode(&self) -> Mode {
        Mode::Normal
    }

    fn shows_mode(&self) -> bool {
        false
    }

    fn block_cursor(&self, _mode: Mode) -> bool {
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
    fn block_cursor(&self, mode: Mode) -> bool {
        (**self).block_cursor(mode)
    }
}