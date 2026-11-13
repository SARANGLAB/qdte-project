use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::editor::Action;

/// Result of mapping a key event.
///
/// This is intentionally temporary for Step 7. In Step 8, the mapping
/// moves to `keymap::dispatcher` and gains app-level commands
/// (Save/Open/Quit/Undo/Redo/Find).
#[derive(Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Action(Action),
    Quit,
    Ignore,
}

/// Blocks until a key press or repeat event arrives. Ignores other events
/// (mouse, resize, focus, paste) for now.
pub fn read_key() -> io::Result<KeyEvent> {
    loop {
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat {
                return Ok(k);
            }
        }
    }
}

/// Temporary minimal key map. Will be replaced by keymap::dispatcher in Step 8.
pub fn map_key(key: KeyEvent) -> InputOutcome {
    use KeyCode::*;
    use KeyModifiers as M;

    match (key.code, key.modifiers) {
        // Quit
        (Char('q'), M::CONTROL) | (Char('c'), M::CONTROL) => InputOutcome::Quit,

        // Text
        (Char(c), M::NONE) | (Char(c), M::SHIFT) => InputOutcome::Action(Action::InsertChar(c)),
        (Enter, _) => InputOutcome::Action(Action::InsertNewline),
        (Backspace, _) => InputOutcome::Action(Action::DeleteBackward),
        (Delete, _) => InputOutcome::Action(Action::DeleteForward),

        // Movement
        (Left, _) => InputOutcome::Action(Action::MoveLeft),
        (Right, _) => InputOutcome::Action(Action::MoveRight),
        (Up, _) => InputOutcome::Action(Action::MoveUp),
        (Down, _) => InputOutcome::Action(Action::MoveDown),
        (Home, _) => InputOutcome::Action(Action::MoveLineStart),
        (End, _) => InputOutcome::Action(Action::MoveLineEnd),

        _ => InputOutcome::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn ctrl_q_quits() {
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            InputOutcome::Quit
        );
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(
            map_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            InputOutcome::Quit
        );
    }

    #[test]
    fn printable_char_inserts() {
        assert_eq!(
            map_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            InputOutcome::Action(Action::InsertChar('a'))
        );
        assert_eq!(
            map_key(key(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            InputOutcome::Action(Action::InsertChar('A'))
        );
    }

    #[test]
    fn arrows_map_to_movement() {
        assert_eq!(
            map_key(key(KeyCode::Left, KeyModifiers::NONE)),
            InputOutcome::Action(Action::MoveLeft)
        );
        assert_eq!(
            map_key(key(KeyCode::Down, KeyModifiers::NONE)),
            InputOutcome::Action(Action::MoveDown)
        );
    }

    #[test]
    fn backspace_and_delete() {
        assert_eq!(
            map_key(key(KeyCode::Backspace, KeyModifiers::NONE)),
            InputOutcome::Action(Action::DeleteBackward)
        );
        assert_eq!(
            map_key(key(KeyCode::Delete, KeyModifiers::NONE)),
            InputOutcome::Action(Action::DeleteForward)
        );
    }

    #[test]
    fn enter_inserts_newline() {
        assert_eq!(
            map_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            InputOutcome::Action(Action::InsertNewline)
        );
    }

    #[test]
    fn unmapped_key_is_ignored() {
        assert_eq!(
            map_key(key(KeyCode::F(1), KeyModifiers::NONE)),
            InputOutcome::Ignore
        );
        assert_eq!(
            map_key(key(KeyCode::Tab, KeyModifiers::NONE)),
            InputOutcome::Ignore
        );
    }
}