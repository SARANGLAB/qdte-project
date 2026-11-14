use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::editor::{Action, Mode};

use super::{Command, Keymap};

pub struct NormalKeymap;

impl Keymap for NormalKeymap {
    fn resolve(&mut self, _mode: Mode, key: KeyEvent) -> Option<Command> {
        use KeyCode::*;
        use KeyModifiers as M;

        Some(match (key.code, key.modifiers) {
            // App commands
            (Char('s'), M::CONTROL) => Command::Save,
            (Char('S'), m) if m.contains(M::CONTROL) => Command::SaveAs,
            (F(2), M::NONE) => Command::SaveAs,
            (Char('z'), M::CONTROL) => Command::Editor(Action::Undo),
            (Char('y'), M::CONTROL) => Command::Editor(Action::Redo),
            (Char('o'), M::CONTROL) => Command::Open,
            (Char('x'), M::CONTROL) | (Char('q'), M::CONTROL) | (Char('c'), M::CONTROL) => Command::Quit,
            
            // Pencarian & Tema
            (Char('f'), M::CONTROL) => Command::Search,
            (F(3), M::NONE) => Command::SearchNext,
            (F(3), M::SHIFT) => Command::SearchPrev,
            (Esc, _) => Command::ClearSearch,
            (Char('t'), M::CONTROL) => Command::OpenThemePicker,

            // Text input
            (Char(c), M::NONE) | (Char(c), M::SHIFT) => Command::Editor(Action::InsertChar(c)),
            (Enter, _) => Command::Editor(Action::InsertNewline),
            (Tab, _) => Command::Editor(Action::InsertTab),
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
    fn tab_maps_to_insert_tab() {
        match resolve(key(KeyCode::Tab, KeyModifiers::NONE)) {
            Some(Command::Editor(Action::InsertTab)) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }
    
    #[test]
    fn ctrl_t_is_theme_picker() {
        assert!(matches!(
            resolve(key(KeyCode::Char('t'), KeyModifiers::CONTROL)),
            Some(Command::OpenThemePicker)
        ));
    }
}