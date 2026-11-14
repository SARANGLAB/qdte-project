pub mod action;
pub mod buffer;
pub mod cursor;
pub mod mode;
pub mod history;
pub mod register;
pub use mode::Mode;
pub use register::Register;

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
    register: Register,
    tab_width: usize,
    expand_tab: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::new(),
            history: History::default(),
            register: Register::default(),
            tab_width: 4,
            expand_tab: false,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            buffer: Buffer::from_str(s),
            cursor: Cursor::new(),
            history: History::default(),
            register: Register::default(),
            tab_width: 4,
            expand_tab: false,
        }
    }

    pub fn with_buffer(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            history: History::default(),
            register: Register::default(),
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

    pub fn register(&self) -> &Register {
        &self.register
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
            Action::MoveLineEndBlock => {
                let line = self.cursor.line(&self.buffer);
                let line_start = self.buffer.line_to_char(line);
                let line_text = self.buffer.line(line).unwrap_or_default();
                let char_count = line_text.chars().count();
                let target = if char_count == 0 {
                    line_start
                } else {
                    line_start + char_count - 1
                };
                self.cursor = Cursor::at(target);
                self.cursor.clamp(&self.buffer);
            }
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
            // ---- word motions ----
            Action::WordForward => self.cursor.move_word_forward(&self.buffer),
            Action::WordBackward => self.cursor.move_word_backward(&self.buffer),
            Action::WordEnd => self.cursor.move_word_end(&self.buffer),
            Action::WordForwardBig => self.cursor.move_word_forward_big(&self.buffer),
            Action::WordBackwardBig => self.cursor.move_word_backward_big(&self.buffer),
            Action::WordEndBig => self.cursor.move_word_end_big(&self.buffer),

            // ---- yank / paste ----
            Action::YankLine => {
                let line = self.cursor.line(&self.buffer);
                let line_start = self.buffer.line_to_char(line);
                let line_text = self.buffer.line(line).unwrap_or_default();
                let line_char_count = line_text.chars().count();
                let line_end = line_start + line_char_count;
                let mut content = line_text.clone();
                // Linewise content always ends with '\n' so paste is predictable.
                content.push('\n');
                let _ = line_end;
                self.register.set_linewise(content);
            }
            Action::PasteAfter => {
                if self.register.is_empty() {
                    return Ok(());
                }
                let line = self.cursor.line(&self.buffer);
                let line_start = self.buffer.line_to_char(line);
                let line_text = self.buffer.line(line).unwrap_or_default();
                let line_end = line_start + line_text.chars().count();
                let buffer_len = self.buffer.len_chars();

                let (insert_at, to_insert, cursor_after) = if line_end < buffer_len {
                    // There is a '\n' at line_end; insert after it.
                    (line_end + 1, self.register.content.clone(), line_end + 1)
                } else {
                    // Last line: prepend '\n' so the paste starts on a new line.
                    (
                        line_end,
                        format!("\n{}", self.register.content),
                        line_end + 1,
                    )
                };

                let cursor_before = self.cursor.char_idx();
                self.buffer.insert_str(insert_at, &to_insert)?;
                self.cursor = Cursor::at(cursor_after);
                self.cursor.clamp(&self.buffer);
                self.history.record(Change::insert(
                    insert_at,
                    to_insert,
                    cursor_before,
                    cursor_after,
                    ChangeKind::Structural,
                ));
            }
            Action::PasteBefore => {
                if self.register.is_empty() {
                    return Ok(());
                }
                let line = self.cursor.line(&self.buffer);
                let line_start = self.buffer.line_to_char(line);
                let cursor_before = self.cursor.char_idx();
                let to_insert = self.register.content.clone();
                let cursor_after = line_start;
                self.buffer.insert_str(line_start, &to_insert)?;
                self.cursor = Cursor::at(cursor_after);
                self.cursor.clamp(&self.buffer);
                self.history.record(Change::insert(
                    line_start,
                    to_insert,
                    cursor_before,
                    cursor_after,
                    ChangeKind::Structural,
                ));
            }
            Action::DeleteRange { start, end } => {
                if start < end {
                    let cursor_before = self.cursor.char_idx();
                    let removed = self.buffer.remove(start..end)?;
                    self.cursor = Cursor::at(start);
                    self.cursor.clamp(&self.buffer);
                    let cursor_after = self.cursor.char_idx();
                    self.history
                        .record(Change::delete(start, removed, cursor_before, cursor_after));
                }
            }
            Action::YankRange { start, end, linewise } => {
                if start < end {
                    let content = self.buffer.slice_to_string(start..end)?;
                    if linewise {
                        self.register.set_linewise(content);
                    } else {
                        self.register.set_charwise(content);
                    }
                    self.cursor = Cursor::at(start);
                    self.cursor.clamp(&self.buffer);
                }
            }
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
    #[test]
    fn yy_p_duplicates_line_below() {
        let mut e = ed("one\ntwo\nthree");
        // cursor on line 0
        e.apply(Action::YankLine).unwrap();
        e.apply(Action::PasteAfter).unwrap();
        assert_eq!(e.buffer().as_string(), "one\none\ntwo\nthree");
        assert_eq!(e.cursor().line(e.buffer()), 1);
    }

    #[test]
    fn yy_p_on_last_line_appends_new_line() {
        let mut e = ed("one\ntwo");
        e.apply(Action::MoveDown).unwrap();
        assert_eq!(e.cursor().line(e.buffer()), 1);
        e.apply(Action::YankLine).unwrap();
        e.apply(Action::PasteAfter).unwrap();
        assert_eq!(e.buffer().as_string(), "one\ntwo\ntwo\n");
        assert_eq!(e.cursor().line(e.buffer()), 2);
    }

    #[test]
    fn yy_capital_p_pastes_above() {
        let mut e = ed("one\ntwo");
        e.apply(Action::MoveDown).unwrap();
        e.apply(Action::YankLine).unwrap();
        e.apply(Action::PasteBefore).unwrap();
        assert_eq!(e.buffer().as_string(), "one\ntwo\ntwo");
        assert_eq!(e.cursor().line(e.buffer()), 1);
    }

    #[test]
    fn paste_without_yank_is_noop() {
        let mut e = ed("abc");
        e.apply(Action::PasteAfter).unwrap();
        assert_eq!(e.buffer().as_string(), "abc");
    }

    #[test]
    fn paste_is_undoable() {
        let mut e = ed("hello");
        e.apply(Action::YankLine).unwrap();
        e.apply(Action::PasteAfter).unwrap();
        assert_eq!(e.buffer().as_string(), "hello\nhello\n");
        e.apply(Action::Undo).unwrap();
        assert_eq!(e.buffer().as_string(), "hello");
    }
    #[test]
    fn move_line_end_block_lands_on_last_char() {
        let mut e = ed("abc\ndef");
        e.apply(Action::MoveLineEndBlock).unwrap();
        assert_eq!(e.cursor().char_idx(), 2); // 'c'
        e.apply(Action::MoveDown).unwrap();
        e.apply(Action::MoveLineEndBlock).unwrap();
        assert_eq!(e.cursor().char_idx(), 6); // 'f'
    }

    #[test]
    fn move_line_end_block_on_empty_line() {
        let mut e = ed("abc\n\ndef");
        e.apply(Action::MoveDown).unwrap(); // to empty line
        e.apply(Action::MoveLineEndBlock).unwrap();
        // cursor stays at line start
        assert_eq!(e.cursor().char_idx(), 4);
    }
    #[test]
    fn delete_range_removes_chars() {
        let mut e = ed("hello world");
        e.apply(Action::DeleteRange { start: 5, end: 11 }).unwrap();
        assert_eq!(e.buffer().as_string(), "hello");
        assert_eq!(e.cursor().char_idx(), 5);
    }

    #[test]
    fn delete_range_is_undoable() {
        let mut e = ed("abcdef");
        e.apply(Action::DeleteRange { start: 1, end: 4 }).unwrap();
        assert_eq!(e.buffer().as_string(), "aef");
        e.apply(Action::Undo).unwrap();
        assert_eq!(e.buffer().as_string(), "abcdef");
    }

    #[test]
    fn yank_range_charwise() {
        let mut e = ed("hello world");
        e.apply(Action::YankRange {
            start: 0,
            end: 5,
            linewise: false,
        })
        .unwrap();
        assert_eq!(e.register().content, "hello");
        assert!(!e.register().linewise);
        assert_eq!(e.buffer().as_string(), "hello world"); // unchanged
        assert_eq!(e.cursor().char_idx(), 0);
    }

    #[test]
    fn yank_range_linewise() {
        let mut e = ed("one\ntwo\nthree");
        e.apply(Action::YankRange {
            start: 0,
            end: 8,
            linewise: true,
        })
        .unwrap();
        assert_eq!(e.register().content, "one\ntwo\n");
        assert!(e.register().linewise);
    }
}
