use std::ops::Range;

use ropey::Rope;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("char index {idx} out of bounds (len = {len})")]
    CharIndexOutOfBounds { idx: usize, len: usize },

    #[error("invalid range {start}..{end} (len = {len})")]
    InvalidRange { start: usize, end: usize, len: usize },
}

/// In-memory text buffer.
///
/// Char-index based (Ropey's native unit). Line endings are normalized to LF
/// by the I/O layer; BOM is stripped there too. This type has no concept of
/// either. Grapheme-cluster awareness belongs to the cursor layer.
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    pub fn from_str(s: &str) -> Self {
        Self { rope: Rope::from_str(s) }
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Number of lines. A trailing '\n' yields an extra empty last line,
    /// matching convention of vim / VS Code.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        if char_idx >= self.rope.len_chars() {
            None
        } else {
            Some(self.rope.char(char_idx))
        }
    }

    /// Returns the given line without its trailing '\n' (if present).
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.rope.len_lines() {
            return None;
        }
        let mut s: String = self.rope.line(line_idx).chars().collect();
        if s.ends_with('\n') {
            s.pop();
        }
        Some(s)
    }

    /// 0-based line containing the given char index.
    /// Clamps to the last valid char if `char_idx == len_chars`.
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        let len = self.rope.len_chars();
        if len == 0 {
            return 0;
        }
        if char_idx >= len {
            // Past-end positions live on the last (possibly empty) line.
            return self.rope.len_lines() - 1;
        }
        self.rope.char_to_line(char_idx)
    }

    /// Char index of the start of `line_idx`, clamped to buffer length.
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        if line_idx >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        self.rope.line_to_char(line_idx)
    }

    pub fn insert_char(&mut self, char_idx: usize, c: char) -> Result<(), BufferError> {
        self.insert_str(char_idx, c.encode_utf8(&mut [0u8; 4]))
    }

    pub fn insert_str(&mut self, char_idx: usize, s: &str) -> Result<(), BufferError> {
        let len = self.rope.len_chars();
        if char_idx > len {
            return Err(BufferError::CharIndexOutOfBounds { idx: char_idx, len });
        }
        self.rope.insert(char_idx, s);
        Ok(())
    }

    /// Removes chars in `range` and returns them.
    pub fn remove(&mut self, range: Range<usize>) -> Result<String, BufferError> {
        let len = self.rope.len_chars();
        if range.start > range.end || range.end > len {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len,
            });
        }
        let removed: String = self.rope.slice(range.clone()).chars().collect();
        self.rope.remove(range);
        Ok(removed)
    }

    pub fn as_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn slice_to_string(&self, range: Range<usize>) -> Result<String, BufferError> {
        let len = self.rope.len_chars();
        if range.start > range.end || range.end > len {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len,
            });
        }
        Ok(self.rope.slice(range).chars().collect())
    }
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.rope.chunks()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let b = Buffer::new();
        assert!(b.is_empty());
        assert_eq!(b.len_chars(), 0);
        assert_eq!(b.len_lines(), 1); // Ropey: empty rope has 1 line
    }

    #[test]
    fn from_str_preserves_content() {
        let b = Buffer::from_str("hello\nworld\n");
        assert_eq!(b.len_chars(), 12);
        assert_eq!(b.len_lines(), 3); // trailing \n produces empty last line
        assert_eq!(b.as_string(), "hello\nworld\n");
    }

    #[test]
    fn line_strips_trailing_newline() {
        let b = Buffer::from_str("alpha\nbeta\ngamma");
        assert_eq!(b.line(0).unwrap(), "alpha");
        assert_eq!(b.line(1).unwrap(), "beta");
        assert_eq!(b.line(2).unwrap(), "gamma");
        assert!(b.line(3).is_none());
    }

    #[test]
    fn trailing_newline_creates_empty_last_line() {
        let b = Buffer::from_str("a\n");
        assert_eq!(b.len_lines(), 2);
        assert_eq!(b.line(0).unwrap(), "a");
        assert_eq!(b.line(1).unwrap(), "");
    }

    #[test]
    fn insert_char_at_start_middle_end() {
        let mut b = Buffer::from_str("ac");
        b.insert_char(1, 'b').unwrap();
        assert_eq!(b.as_string(), "abc");

        b.insert_char(0, 'X').unwrap();
        assert_eq!(b.as_string(), "Xabc");

        b.insert_char(4, '!').unwrap();
        assert_eq!(b.as_string(), "Xabc!");
    }

    #[test]
    fn insert_at_end_ok() {
        let mut b = Buffer::from_str("hi");
        b.insert_str(2, "!").unwrap();
        assert_eq!(b.as_string(), "hi!");
    }

    #[test]
    fn insert_oob_errors() {
        let mut b = Buffer::from_str("hi");
        assert!(matches!(
            b.insert_char(3, '!'),
            Err(BufferError::CharIndexOutOfBounds { .. })
        ));
        assert_eq!(b.as_string(), "hi");
    }

    #[test]
    fn insert_multibyte_uses_char_index() {
        let mut b = Buffer::from_str("héllo"); // é is 2 bytes, 1 char
        assert_eq!(b.len_chars(), 5);
        assert_eq!(b.len_bytes(), 6);
        b.insert_char(1, 'X').unwrap();
        assert_eq!(b.as_string(), "hXéllo");
    }

    #[test]
    fn remove_range() {
        let mut b = Buffer::from_str("abcdef");
        let removed = b.remove(1..4).unwrap();
        assert_eq!(removed, "bcd");
        assert_eq!(b.as_string(), "aef");
    }

    #[test]
    fn remove_empty_range_ok() {
        let mut b = Buffer::from_str("abc");
        let removed = b.remove(1..1).unwrap();
        assert_eq!(removed, "");
        assert_eq!(b.as_string(), "abc");
    }

    #[test]
    fn remove_invalid_range_errors() {
        let mut b = Buffer::from_str("abc");
        assert!(matches!(
            b.remove(2..5),
            Err(BufferError::InvalidRange { .. })
        ));
        assert!(matches!(
            b.remove(2..1),
            Err(BufferError::InvalidRange { .. })
        ));
        assert_eq!(b.as_string(), "abc");
    }

    #[test]
    fn char_to_line_basic() {
        let b = Buffer::from_str("aaa\nbbb\nccc");
        assert_eq!(b.char_to_line(0), 0);
        assert_eq!(b.char_to_line(2), 0);
        assert_eq!(b.char_to_line(3), 0); // the '\n'
        assert_eq!(b.char_to_line(4), 1); // first char of line 1
        assert_eq!(b.char_to_line(11), 2);
    }

    #[test]
    fn char_to_line_clamps_at_end() {
        let b = Buffer::from_str("a\nb");
        // len_chars = 3, last valid char index is 2
        assert_eq!(b.char_to_line(2), 1);
        assert_eq!(b.char_to_line(100), 1);
    }

    #[test]
    fn line_to_char_basic() {
        let b = Buffer::from_str("aaa\nbbb\nccc");
        assert_eq!(b.line_to_char(0), 0);
        assert_eq!(b.line_to_char(1), 4);
        assert_eq!(b.line_to_char(2), 8);
    }

    #[test]
    fn unicode_round_trip() {
        let s = "日本語\nemoji: 🦀\n";
        let b = Buffer::from_str(s);
        assert_eq!(b.as_string(), s);
    }
}
