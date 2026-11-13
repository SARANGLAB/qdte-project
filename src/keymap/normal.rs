use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::editor::Action;

use super::{Command, Keymap};

use crate::editor::Mode;

pub struct NormalKeymap;

impl Keymap for NormalKeymap {
    fn resolve(&mut self, _mode: Mode, key: KeyEvent) -> Option<Command> {
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
            (Tab, _) => Command::Editor(Action::InsertChar('\t')),
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
    use crate::editor::Mode;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn resolve(key_event: KeyEvent) -> Option<Command> {
        NormalKeymap.resolve(Mode::Normal, key_event)
    }

    #[test]
    fn ctrl_s_is_save() {
        assert!(matches!(
            resolve(key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(Command::Save)
        ));
    }

    #[test]
    fn ctrl_x_is_quit() {
        assert!(matches!(
            resolve(key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            Some(Command::Quit)
        ));
    }

    #[test]
    fn plain_letter_is_insert() {
        match resolve(key(KeyCode::Char('a'), KeyModifiers::NONE)) {
            Some(Command::Editor(Action::InsertChar('a'))) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn unmapped_returns_none() {
        assert!(resolve(key(KeyCode::F(1), KeyModifiers::NONE)).is_none());
    }

    #[test]
    fn tab_inserts_tab_char() {
        match resolve(key(KeyCode::Tab, KeyModifiers::NONE)) {
            Some(Command::Editor(Action::InsertChar('\t'))) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }
}