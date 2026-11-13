pub mod action;
pub mod buffer;
pub mod cursor;
pub mod mode;
pub mod history;
pub use mode::Mode;

use thiserror::Error;

pub use action::Action;
pub use buffer::{Buffer, BufferError};
pub use cursor::Cursor;

pub use history::{Change, ChangeKind, History};

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
    history: History,
    tab_width: usize,
    expand_tab: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::new(),
            history: History::default(),
            tab_width: 4,
            expand_tab: false,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            buffer: Buffer::from_str(s),
            cursor: Cursor::new(),
            history: History::default(),
            tab_width: 4,
            expand_tab: false,
        }
    }

    pub fn with_buffer(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            history: History::default(),
            tab_width: 4,
            expand_tab: false,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn set_tab_config(&mut self, tab_width: usize, expand_tab: bool) {
        self.tab_width = tab_width.max(1);
        self.expand_tab = expand_tab;
    }

    pub fn tab_width(&self) -> usize {
        self.tab_width
    }

    pub fn expand_tab(&self) -> bool {
        self.expand_tab
    }

    /// Apply an action. Mutates buffer and/or cursor as appropriate.
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
            Action::MoveLeftLineClamped => {
                if let Some(prev) = self.cursor.previous_grapheme_start_within_line(&self.buffer) {
                    self.cursor = Cursor::at(prev);
                }
            }
            Action::MoveRightLineClamped => {
                if let Some(next) = self.cursor.next_grapheme_end_within_line(&self.buffer) {
                    self.cursor = Cursor::at(next);
                }
            }

            // ---- text editing ----
            Action::InsertChar(c) => {
                let idx = self.cursor.char_idx();
                let cursor_before = idx;
                self.buffer.insert_char(idx, c)?;
                self.cursor = Cursor::at(idx + 1);
                let cursor_after = self.cursor.char_idx();
                self.history.record(Change::insert(
                    idx,
                    c.to_string(),
                    cursor_before,
                    cursor_after,
                    ChangeKind::Typing,
                ));
            }
            Action::InsertNewline => {
                let idx = self.cursor.char_idx();
                let cursor_before = idx;
                self.buffer.insert_char(idx, '\n')?;
                self.cursor = Cursor::at(idx + 1);
                let cursor_after = self.cursor.char_idx();
                self.history.record(Change::insert(
                    idx,
                    "\n",
                    cursor_before,
                    cursor_after,
                    ChangeKind::Structural,
                ));
            }
            Action::InsertTab => {
                let idx = self.cursor.char_idx();
                let cursor_before = idx;
                let text = if self.expand_tab {
                    " ".repeat(self.tab_width)
                } else {
                    "\t".to_string()
                };
                self.buffer.insert_str(idx, &text)?;
                self.cursor = Cursor::at(idx + text.chars().count());
                let cursor_after = self.cursor.char_idx();
                self.history.record(Change::insert(
                    idx,
                    text,
                    cursor_before,
                    cursor_after,
                    ChangeKind::Structural,
                ));
            }
            Action::DeleteBackward => {
                let cursor_idx = self.cursor.char_idx();
                if let Some(prev_start) = self.cursor.previous_grapheme_start(&self.buffer) {
                    let removed = self.buffer.remove(prev_start..cursor_idx)?;
                    self.cursor = Cursor::at(prev_start);
                    self.history.record(Change::delete(
                        prev_start,
                        removed,
                        cursor_idx,
                        prev_start,
                    ));
                }
            }
            Action::DeleteForward => {
                let cursor_idx = self.cursor.char_idx();
                if let Some(next_end) = self.cursor.next_grapheme_end(&self.buffer) {
                    let removed = self.buffer.remove(cursor_idx..next_end)?;
                    self.history
                        .record(Change::delete(cursor_idx, removed, cursor_idx, cursor_idx));
                }
            }
            Action::DeleteLine => {
                let cursor_line = self.cursor.line(&self.buffer);
                let cursor_before = self.cursor.char_idx();
                let line_start = self.buffer.line_to_char(cursor_line);
                let line_text = self.buffer.line(cursor_line).unwrap_or_default();
                let line_char_count = line_text.chars().count();
                let line_end = line_start + line_char_count;
                let buffer_len = self.buffer.len_chars();

                let (start, end) = if line_end < buffer_len {
                    (line_start, line_end + 1)
                } else if line_start > 0 {
                    (line_start - 1, buffer_len)
                } else {
                    (line_start, buffer_len)
                };

                let removed = self.buffer.remove(start..end)?;
                self.cursor = Cursor::at(start);
                self.cursor.clamp(&self.buffer);
                let cursor_after = self.cursor.char_idx();
                self.history
                    .record(Change::delete(start, removed, cursor_before, cursor_after));
            }
            Action::Undo => self.undo()?,
            Action::Redo => self.redo()?,
        }
        Ok(())
    }
    pub fn undo(&mut self) -> Result<(), EditorError> {
        let Some(change) = self.history.pop_undo() else {
            return Ok(());
        };
        if !change.inserted.is_empty() {
            let end = change.char_idx + change.inserted.chars().count();
            self.buffer.remove(change.char_idx..end)?;
        }
        if !change.removed.is_empty() {
            self.buffer.insert_str(change.char_idx, &change.removed)?;
        }
        self.cursor = Cursor::at(change.cursor_before);
        self.cursor.clamp(&self.buffer);
        self.history.push_redo(change);
        Ok(())
    }
    pub fn redo(&mut self) -> Result<(), EditorError> {
        let Some(change) = self.history.pop_redo() else {
            return Ok(());
        };
        if !change.removed.is_empty() {
            let end = change.char_idx + change.removed.chars().count();
            self.buffer.remove(change.char_idx..end)?;
        }
        if !change.inserted.is_empty() {
            self.buffer.insert_str(change.char_idx, &change.inserted)?;
        }
        self.cursor = Cursor::at(change.cursor_after);
        self.cursor.clamp(&self.buffer);
        self.history.push_undo(change);
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
