pub mod dispatcher;
pub mod normal;
pub mod vim;

use crossterm::event::KeyEvent;

use crate::editor::Action;

/// A high-level intent, resolved from a key event.
///
/// `Editor` actions go to the editor core; the rest go to `App`.
#[derive(Debug, Clone)]
pub enum Command {
    Editor(Action),
    Save,
    Quit,
}

pub trait Keymap {
    fn resolve(&self, key: KeyEvent) -> Option<Command>;
}