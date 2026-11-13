use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::editor::Action;

use super::{Command, Keymap};

pub struct NormalKeymap;

impl Keymap for NormalKeymap {
    fn resolve(&self, key: KeyEvent) -> Option<Command> {
        use KeyCode::*;
        use KeyModifiers as M;

        Some(match (key.code, key.modifiers) {
            // App commands
            (Char('s'), M::CONTROL) => Command::Save,
            (Char('x'), M::CONTROL) => Command::Quit,
            (Char('q'), M::CONTROL) => Command::Quit,
            (Char('c'), M::CONTROL) => Command::Quit,

            // Text input
            (Char(c), M::NONE) | (Char(c), M::SHIFT) => Command::Editor(Action::InsertChar(c)),
            (Enter, _) => Command::Editor(Action::InsertNewline),
            (Backspace, _) => Command::Editor(Action::DeleteBackward),
            (Delete, _) => Command::Editor(Action::DeleteForward),

            // Movement
            (Left, _) => Command::Editor(Action::MoveLeft),
            (Right, _) => Command::Editor(Action::MoveRight),
            (Up, _) => Command::Editor(Action::MoveUp),
            (Down, _) => Command::Editor(Action::MoveDown),
            (Home, _) => Command::Editor(Action::MoveLineStart),
            (End, _) => Command::Editor(Action::MoveLineEnd),

            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn ctrl_s_is_save() {
        assert!(matches!(
            NormalKeymap.resolve(key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(Command::Save)
        ));
    }

    #[test]
    fn ctrl_x_is_quit() {
        assert!(matches!(
            NormalKeymap.resolve(key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            Some(Command::Quit)
        ));
    }

    #[test]
    fn plain_letter_is_insert() {
        match NormalKeymap.resolve(key(KeyCode::Char('a'), KeyModifiers::NONE)) {
            Some(Command::Editor(Action::InsertChar('a'))) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn unmapped_returns_none() {
        assert!(NormalKeymap
            .resolve(key(KeyCode::F(1), KeyModifiers::NONE))
            .is_none());
    }
}