pub mod action;
pub mod buffer;
pub mod cursor;
pub mod mode;

use thiserror::Error;

pub use action::Action;
pub use buffer::{Buffer, BufferError};
pub use cursor::Cursor;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error(transparent)]
    Buffer(#[from] BufferError),
}

/// The editor core.
///
/// Owns the buffer and the cursor. Knows nothing about keybindings, modes,
/// terminal, or filesystem. Applies `Action`s and exposes read-only state.
pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::new(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            buffer: Buffer::from_str(s),
            cursor: Cursor::new(),
        }
    }

    pub fn with_buffer(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Apply an action. Mutates buffer and/or cursor as appropriate.
    pub fn apply(&mut self, action: Action) -> Result<(), EditorError> {
        match action {
            // ---- movement ----
            Action::MoveLeft => self.cursor.move_left(&self.buffer),
            Action::MoveRight => self.cursor.move_right(&self.buffer),
            Action::MoveUp => self.cursor.move_up(&self.buffer),
            Action::MoveDown => self.cursor.move_down(&self.buffer),
            Action::MoveLineStart => self.cursor.move_line_start(&self.buffer),
            Action::MoveLineEnd => self.cursor.move_line_end(&self.buffer),
            Action::MoveBufferStart => self.cursor.move_buffer_start(&self.buffer),
            Action::MoveBufferEnd => self.cursor.move_buffer_end(&self.buffer),

            // ---- text editing ----
            Action::InsertChar(c) => {
                let idx = self.cursor.char_idx();
                self.buffer.insert_char(idx, c)?;
                self.cursor = Cursor::at(idx + 1);
            }
            Action::InsertNewline => {
                let idx = self.cursor.char_idx();
                self.buffer.insert_char(idx, '\n')?;
                self.cursor = Cursor::at(idx + 1);
            }
            Action::DeleteBackward => {
                let cursor_idx = self.cursor.char_idx();
                if let Some(prev_start) = self.cursor.previous_grapheme_start(&self.buffer) {
                    self.buffer.remove(prev_start..cursor_idx)?;
                    self.cursor = Cursor::at(prev_start);
                }
            }
            Action::DeleteForward => {
                let cursor_idx = self.cursor.char_idx();
                if let Some(next_end) = self.cursor.next_grapheme_end(&self.buffer) {
                    self.buffer.remove(cursor_idx..next_end)?;
                    // cursor stays at same char index; content after it shrank
                }
            }
        }
        Ok(())
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(s: &str) -> Editor {
        Editor::from_str(s)
    }

    #[test]
    fn insert_chars_into_empty_buffer() {
        let mut e = ed("");
        for c in "hello".chars() {
            e.apply(Action::InsertChar(c)).unwrap();
        }
        assert_eq!(e.buffer().as_string(), "hello");
        assert_eq!(e.cursor().char_idx(), 5);
    }

    #[test]
    fn insert_char_in_middle() {
        let mut e = ed("ac");
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::InsertChar('b')).unwrap();
        assert_eq!(e.buffer().as_string(), "abc");
        assert_eq!(e.cursor().char_idx(), 2);
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut e = ed("abcd");
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::InsertNewline).unwrap();
        assert_eq!(e.buffer().as_string(), "ab\ncd");
        assert_eq!(e.cursor().char_idx(), 3);
        assert_eq!(e.cursor().line(e.buffer()), 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut e = ed("abc");
        e.apply(Action::DeleteBackward).unwrap();
        assert_eq!(e.buffer().as_string(), "abc");
        assert_eq!(e.cursor().char_idx(), 0);
    }

    #[test]
    fn backspace_removes_char_before_cursor() {
        let mut e = ed("abc");
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::DeleteBackward).unwrap();
        assert_eq!(e.buffer().as_string(), "ac");
        assert_eq!(e.cursor().char_idx(), 1);
    }

    #[test]
    fn backspace_joins_lines() {
        let mut e = ed("abc\ndef");
        for _ in 0..4 {
            e.apply(Action::MoveRight).unwrap();
        }
        e.apply(Action::DeleteBackward).unwrap();
        assert_eq!(e.buffer().as_string(), "abcdef");
        assert_eq!(e.cursor().char_idx(), 3);
    }

    #[test]
    fn backspace_removes_whole_grapheme() {
        // 👨‍👩‍👧 is 5 chars, 1 grapheme
        let mut e = ed("a👨‍👩‍👧");
        e.apply(Action::MoveBufferEnd).unwrap();
        e.apply(Action::DeleteBackward).unwrap();
        assert_eq!(e.buffer().as_string(), "a");
        assert_eq!(e.cursor().char_idx(), 1);
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut e = ed("abc");
        e.apply(Action::MoveBufferEnd).unwrap();
        e.apply(Action::DeleteForward).unwrap();
        assert_eq!(e.buffer().as_string(), "abc");
    }

    #[test]
    fn delete_forward_removes_char_at_cursor() {
        let mut e = ed("abc");
        e.apply(Action::DeleteForward).unwrap();
        assert_eq!(e.buffer().as_string(), "bc");
        assert_eq!(e.cursor().char_idx(), 0);
    }

    #[test]
    fn delete_forward_joins_lines() {
        let mut e = ed("abc\ndef");
        for _ in 0..3 {
            e.apply(Action::MoveRight).unwrap();
        }
        e.apply(Action::DeleteForward).unwrap();
        assert_eq!(e.buffer().as_string(), "abcdef");
        assert_eq!(e.cursor().char_idx(), 3);
    }

    #[test]
    fn movement_actions_dispatch() {
        let mut e = ed("abc\ndef");
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap();
        assert_eq!(e.cursor().char_idx(), 2);

        e.apply(Action::MoveDown).unwrap();
        assert_eq!(e.cursor().line(e.buffer()), 1);

        e.apply(Action::MoveUp).unwrap();
        assert_eq!(e.cursor().line(e.buffer()), 0);

        e.apply(Action::MoveLineEnd).unwrap();
        assert_eq!(e.cursor().char_idx(), 3);

        e.apply(Action::MoveBufferEnd).unwrap();
        assert_eq!(e.cursor().char_idx(), 7);

        e.apply(Action::MoveBufferStart).unwrap();
        assert_eq!(e.cursor().char_idx(), 0);
    }
    #[test]
    fn backspace_after_move_to_line_start_joins_lines() {
        let mut e = ed("abc\ndef");
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap();
        e.apply(Action::MoveRight).unwrap(); // cursor at 4, start of line 1
        e.apply(Action::DeleteBackward).unwrap();
        assert_eq!(e.buffer().as_string(), "abcdef");
        assert_eq!(e.cursor().char_idx(), 3);
    }
}