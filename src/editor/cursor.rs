use unicode_segmentation::UnicodeSegmentation;

use super::buffer::Buffer;

/// A text cursor.
///
/// Internally stores a char index. Conversions to/from (line, grapheme_col)
/// go through the buffer, which is the source of truth for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    char_idx: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self { char_idx: 0 }
    }

    pub fn at(char_idx: usize) -> Self {
        Self { char_idx }
    }

    pub fn char_idx(&self) -> usize {
        self.char_idx
    }

    /// Clamp the cursor to a valid char position in the given buffer.
    /// Call this after any buffer mutation.
    pub fn clamp(&mut self, buffer: &Buffer) {
        let len = buffer.len_chars();
        if self.char_idx > len {
            self.char_idx = len;
        }
    }

    /// Current line (0-based).
    pub fn line(&self, buffer: &Buffer) -> usize {
        buffer.char_to_line(self.char_idx)
    }

    /// Column in grapheme clusters within the current line.
    /// `col == line_grapheme_len` means cursor is after the last grapheme.
    pub fn grapheme_col(&self, buffer: &Buffer) -> usize {
        let line_start = buffer.line_to_char(self.line(buffer));
        let prefix = buffer
            .slice_to_string(line_start..self.char_idx)
            .unwrap_or_default();
        prefix.graphemes(true).count()
    }

    // ---- movement ---------------------------------------------------------

    pub fn move_left(&mut self, buffer: &Buffer) {
        if self.char_idx == 0 {
            return;
        }
        let prev = previous_grapheme_boundary(buffer, self.char_idx);
        self.char_idx = prev;
    }

    pub fn move_right(&mut self, buffer: &Buffer) {
        let len = buffer.len_chars();
        if self.char_idx >= len {
            return;
        }
        let next = next_grapheme_boundary(buffer, self.char_idx);
        self.char_idx = next;
    }

    pub fn move_up(&mut self, buffer: &Buffer) {
        let line = self.line(buffer);
        if line == 0 {
            return;
        }
        let col = self.grapheme_col(buffer);
        self.char_idx = position_for_line_grapheme_col(buffer, line - 1, col);
    }

    pub fn move_down(&mut self, buffer: &Buffer) {
        let line = self.line(buffer);
        if line + 1 >= buffer.len_lines() {
            return;
        }
        let col = self.grapheme_col(buffer);
        self.char_idx = position_for_line_grapheme_col(buffer, line + 1, col);
    }

    pub fn move_line_start(&mut self, buffer: &Buffer) {
        let line = self.line(buffer);
        self.char_idx = buffer.line_to_char(line);
    }

    pub fn move_line_end(&mut self, buffer: &Buffer) {
        let line = self.line(buffer);
        let line_start = buffer.line_to_char(line);
        let line_text = buffer.line(line).unwrap_or_default();
        let line_graphemes = line_text.graphemes(true).count();
        self.char_idx = line_start + line_text.chars().count();
        // (using chars count, not graphemes, because char_idx is in chars)
        let _ = line_graphemes;
    }

    pub fn move_buffer_start(&mut self, _buffer: &Buffer) {
        self.char_idx = 0;
    }

    pub fn move_buffer_end(&mut self, buffer: &Buffer) {
        self.char_idx = buffer.len_chars();
    }
    pub fn previous_grapheme_start(&self, buffer: &Buffer) -> Option<usize> {
        if self.char_idx == 0 {
            return None;
        }
        Some(previous_grapheme_boundary(buffer, self.char_idx))
    }

    /// Char index one past the grapheme immediately after the cursor.
    /// `None` if the cursor is at the end of the buffer.
    pub fn next_grapheme_end(&self, buffer: &Buffer) -> Option<usize> {
        let len = buffer.len_chars();
        if self.char_idx >= len {
            return None;
        }
        Some(next_grapheme_boundary(buffer, self.char_idx))
    }
    pub fn next_grapheme_end_within_line(&self, buffer: &Buffer) -> Option<usize> {
        let line = buffer.char_to_line(self.char_idx);
        let line_start = buffer.line_to_char(line);
        let line_text = buffer.line(line).unwrap_or_default();
        let line_end = line_start + line_text.chars().count();

        if self.char_idx >= line_end {
            return None;
        }

        let next = next_grapheme_boundary(buffer, self.char_idx);
        Some(next.min(line_end))
    }

    pub fn previous_grapheme_start_within_line(&self, buffer: &Buffer) -> Option<usize> {
        let line = buffer.char_to_line(self.char_idx);
        let line_start = buffer.line_to_char(line);

        if self.char_idx <= line_start {
            return None;
        }

        Some(previous_grapheme_boundary(buffer, self.char_idx))
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

// ---- helpers ---------------------------------------------------------------

fn previous_grapheme_boundary(buffer: &Buffer, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }

    let line = buffer.char_to_line(char_idx);
    let line_start = buffer.line_to_char(line);

    // Cursor is at the start of line > 0. The grapheme immediately
    // before the cursor is the '\n' that ends the previous line.
    if char_idx == line_start {
        return line_start - 1;
    }

    let prefix = buffer
        .slice_to_string(line_start..char_idx)
        .unwrap_or_default();

    let mut boundary = 0usize;
    let mut char_pos = 0usize;
    for g in prefix.graphemes(true) {
        char_pos += g.chars().count();
        if char_pos < prefix.chars().count() {
            boundary = char_pos;
        } else {
            return line_start + boundary;
        }
    }
    line_start
}

fn next_grapheme_boundary(buffer: &Buffer, char_idx: usize) -> usize {
    let len = buffer.len_chars();
    if char_idx >= len {
        return len;
    }
    let line = buffer.char_to_line(char_idx);
    let line_start = buffer.line_to_char(line);
    let suffix = buffer
        .slice_to_string(char_idx..(line_start + buffer.line(line).map(|s| s.chars().count()).unwrap_or(0) + 1).min(len))
        .unwrap_or_default();

    if let Some(first) = suffix.graphemes(true).next() {
        (char_idx + first.chars().count()).min(len)
    } else {
        // Fallback: advance one char to avoid getting stuck.
        char_idx + 1
    }
}

fn position_for_line_grapheme_col(buffer: &Buffer, line: usize, target_col: usize) -> usize {
    let line_start = buffer.line_to_char(line);
    let line_text = buffer.line(line).unwrap_or_default();

    let mut col = 0usize;
    let mut char_offset = 0usize;
    for g in line_text.graphemes(true) {
        if col >= target_col {
            break;
        }
        col += 1;
        char_offset += g.chars().count();
    }
    line_start + char_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    #[test]
    fn new_cursor_is_at_zero() {
        let c = Cursor::new();
        assert_eq!(c.char_idx(), 0);
    }

    #[test]
    fn move_right_advances_one_char() {
        let b = buf("abc");
        let mut c = Cursor::new();
        c.move_right(&b);
        assert_eq!(c.char_idx(), 1);
        c.move_right(&b);
        assert_eq!(c.char_idx(), 2);
    }

    #[test]
    fn move_right_stops_at_end() {
        let b = buf("ab");
        let mut c = Cursor::at(2);
        c.move_right(&b);
        assert_eq!(c.char_idx(), 2);
    }

    #[test]
    fn move_left_stops_at_start() {
        let b = buf("ab");
        let mut c = Cursor::new();
        c.move_left(&b);
        assert_eq!(c.char_idx(), 0);
    }

    #[test]
    fn move_left_one_char() {
        let b = buf("abc");
        let mut c = Cursor::at(2);
        c.move_left(&b);
        assert_eq!(c.char_idx(), 1);
    }

    #[test]
    fn move_right_skips_emoji_zwj_sequence() {
        // 👨‍👩‍👧 is 5 chars, 1 grapheme
        let s = "a👨‍👩‍👧b";
        let b = buf(s);
        let mut c = Cursor::new();
        c.move_right(&b); // skip 'a' → idx 1
        assert_eq!(c.char_idx(), 1);
        c.move_right(&b); // skip whole emoji → idx 6
        assert_eq!(c.char_idx(), 6);
        c.move_right(&b); // skip 'b' → idx 7
        assert_eq!(c.char_idx(), 7);
    }

    #[test]
    fn move_left_skips_emoji_zwj_sequence() {
        let s = "a👨‍👩‍👧b";
        let b = buf(s);
        let mut c = Cursor::at(7);
        c.move_left(&b); // skip 'b' → idx 6
        assert_eq!(c.char_idx(), 6);
        c.move_left(&b); // skip emoji → idx 1
        assert_eq!(c.char_idx(), 1);
        c.move_left(&b); // skip 'a' → idx 0
        assert_eq!(c.char_idx(), 0);
    }

    #[test]
    fn grapheme_col_counts_graphemes() {
        let b = buf("a👨‍👩‍👧b");
        let c = Cursor::at(6); // after emoji
        assert_eq!(c.grapheme_col(&b), 2);
        let c = Cursor::at(7);
        assert_eq!(c.grapheme_col(&b), 3);
    }

    #[test]
    fn line_reports_current_line() {
        let b = buf("aaa\nbbb\nccc");
        let c = Cursor::at(4);
        assert_eq!(c.line(&b), 1);
    }

    #[test]
    fn move_up_preserves_grapheme_col() {
        let b = buf("abcd\nxy");
        let mut c = Cursor::at(6); // line 1, col 1 ('y')
        c.move_up(&b);
        assert_eq!(c.char_idx(), 1); // line 0, col 1 ('b')
    }

    #[test]
    fn move_up_clamps_at_short_line() {
        let b = buf("x\nabcdef");
        let mut c = Cursor::at(2 + 4); // line 1, col 4
        c.move_up(&b);
        assert_eq!(c.char_idx(), 1); // line 0 only has 1 char, col clamped to 1
    }

    #[test]
    fn move_down_skips_emoji() {
        let b = buf("ab\n👨‍👩‍👧x");
        let mut c = Cursor::at(1); // line 0, col 1
        c.move_down(&b);
        // line 1 start = 3. col 1 → after the emoji (5 chars) → idx 3+5 = 8
        assert_eq!(c.char_idx(), 8);
    }

    #[test]
    fn move_up_at_first_line_is_noop() {
        let b = buf("abc");
        let mut c = Cursor::at(2);
        c.move_up(&b);
        assert_eq!(c.char_idx(), 2);
    }

    #[test]
    fn move_down_at_last_line_is_noop() {
        let b = buf("abc");
        let mut c = Cursor::at(1);
        c.move_down(&b);
        assert_eq!(c.char_idx(), 1);
    }

    #[test]
    fn move_line_start_and_end() {
        let b = buf("aaa\nbbbb\ncc");
        let mut c = Cursor::at(6);
        c.move_line_start(&b);
        assert_eq!(c.char_idx(), 4);
        c.move_line_end(&b);
        assert_eq!(c.char_idx(), 8); // end of "bbbb" is idx 8
    }

    #[test]
    fn move_buffer_start_and_end() {
        let b = buf("abc\ndef");
        let mut c = Cursor::at(4);
        c.move_buffer_start(&b);
        assert_eq!(c.char_idx(), 0);
        c.move_buffer_end(&b);
        assert_eq!(c.char_idx(), 7);
    }

    #[test]
    fn clamp_after_shrink() {
        let b1 = buf("abcdef");
        let mut c = Cursor::at(6);
        let b2 = buf("abc");
        c.clamp(&b2);
        assert_eq!(c.char_idx(), 3);
        let _ = b1;
    }
    #[test]
    fn move_left_crosses_line_boundary() {
        let b = buf("abc\ndef");
        let mut c = Cursor::at(4); // start of line 1
        c.move_left(&b);
        assert_eq!(c.char_idx(), 3); // on the '\n'
        c.move_left(&b);
        assert_eq!(c.char_idx(), 2);
    }

    #[test]
    fn previous_grapheme_start_at_line_start() {
        let b = buf("abc\ndef");
        let c = Cursor::at(4);
        assert_eq!(c.previous_grapheme_start(&b), Some(3));
    }
}
