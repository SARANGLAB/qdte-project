use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::editor::buffer::Buffer;

const TAB_STOP: usize = 4;

pub struct Renderer {
    /// First visible line (vertical scroll).
    top_line: usize,
    /// First visible visual column (horizontal scroll).
    left_col: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self { top_line: 0, left_col: 0 }
    }

    pub fn render(&mut self, app: &App, mode_label: &str) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let cols = cols as usize;
        let rows = rows as usize;

        if rows < 2 || cols < 1 {
            return Ok(());
        }

        let content_height = rows - 1;
        let buffer = app.editor.buffer();
        let cursor = app.editor.cursor();

        let cursor_line = cursor.line(buffer);
        let cursor_gcol = cursor.grapheme_col(buffer);
        let cursor_visual_col = {
            let line_text = buffer.line(cursor_line).unwrap_or_default();
            visual_col(&line_text, cursor_gcol, TAB_STOP)
        };

        self.adjust_scroll(cursor_line, cursor_visual_col, content_height, cols);

        let mut stdout = io::stdout();

        // Single full-screen clear at the top.
        queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        for row in 0..content_height {
            let line_idx = self.top_line + row;
            queue!(stdout, MoveTo(0, row as u16))?;
            if line_idx < buffer.len_lines() {
                let rendered = render_line(buffer, line_idx, self.left_col, cols);
                if !rendered.is_empty() {
                    queue!(stdout, Print(rendered))?;
                }
            } else {
                queue!(stdout, Print("~"))?;
            }
        }

        let status = format_status(app, mode_label, cols);
        queue!(
            stdout,
            MoveTo(0, content_height as u16),
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White),
            Print(status),
            ResetColor,
        )?;

        let cx = (cursor_visual_col.saturating_sub(self.left_col) as u16)
            .min(cols.saturating_sub(1) as u16);
        let cy = cursor_line.saturating_sub(self.top_line) as u16;
        queue!(stdout, MoveTo(cx, cy))?;

        stdout.flush()?;
        Ok(())
    }

    fn adjust_scroll(
        &mut self,
        cursor_line: usize,
        cursor_col: usize,
        height: usize,
        width: usize,
    ) {
        if cursor_line < self.top_line {
            self.top_line = cursor_line;
        } else if cursor_line >= self.top_line + height {
            self.top_line = cursor_line + 1 - height;
        }

        if cursor_col < self.left_col {
            self.left_col = cursor_col;
        } else if cursor_col >= self.left_col + width {
            self.left_col = cursor_col + 1 - width;
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---- pure helpers (tested) ------------------------------------------------

/// Visual width of the first `grapheme_count` graphemes of `line_text`,
/// with tabs expanded to the next multiple of `tab_stop`.
pub(crate) fn visual_col(line_text: &str, grapheme_count: usize, tab_stop: usize) -> usize {
    let mut w = 0usize;
    for (i, g) in line_text.graphemes(true).enumerate() {
        if i >= grapheme_count {
            break;
        }
        if g == "\t" {
            w += tab_stop - (w % tab_stop);
        } else {
            w += UnicodeWidthStr::width(g);
        }
    }
    w
}

/// Render a single line as a printable string, honoring left scroll offset
/// and maximum width. Tabs are expanded to spaces.
pub(crate) fn render_line(
    buffer: &Buffer,
    line_idx: usize,
    left_col: usize,
    max_width: usize,
) -> String {
    let text = buffer.line(line_idx).unwrap_or_default();
    let mut out = String::new();
    let mut visual = 0usize;
    let mut emitted = 0usize;

    for g in text.graphemes(true) {
        let gw = if g == "\t" {
            TAB_STOP - (visual % TAB_STOP)
        } else {
            UnicodeWidthStr::width(g)
        };

        // Entirely to the left of the viewport.
        if visual + gw <= left_col {
            visual += gw;
            continue;
        }

        // This grapheme is (partially) visible.
        let (skip, emit) = if visual < left_col {
            let skip = left_col - visual;
            (skip, gw - skip)
        } else {
            (0, gw)
        };

        if emitted + emit > max_width {
            break;
        }

        if skip > 0 {
            // Viewport cuts through a wide grapheme — pad with spaces.
            out.push_str(&" ".repeat(emit));
        } else if g == "\t" {
            out.push_str(&" ".repeat(gw));
        } else {
            out.push_str(g);
        }

        emitted += emit;
        visual += gw;
    }

    out
}

fn format_status(app: &App, mode_label: &str, width: usize) -> String {
    let name = app
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<no file>".to_string());
    let dirty = if app.is_dirty() { " [+]" } else { "" };

    let cursor = app.editor.cursor();
    let buffer = app.editor.buffer();
    let pos = format!(
        "char {}  Ln {}, Col {}",
        cursor.char_idx(),
        cursor.line(buffer) + 1,
        cursor.grapheme_col(buffer) + 1,
    );

    let mut left = String::new();
    if !mode_label.is_empty() {
        left.push('[');
        left.push_str(mode_label);
        left.push_str("] ");
    }
    left.push_str(&name);
    left.push_str(dirty);
    if let Some(msg) = &app.message {
        left.push_str("  ");
        left.push_str(msg);
    }

    let left_w = UnicodeWidthStr::width(left.as_str());
    let right_w = pos.len();
    let pad = width.saturating_sub(left_w + right_w);

    let mut s = String::with_capacity(width);
    s.push_str(&left);
    s.push_str(&" ".repeat(pad));
    s.push_str(&pos);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    // ---- visual_col ----

    #[test]
    fn visual_col_empty() {
        assert_eq!(visual_col("", 0, TAB_STOP), 0);
    }

    #[test]
    fn visual_col_ascii() {
        assert_eq!(visual_col("abc", 0, TAB_STOP), 0);
        assert_eq!(visual_col("abc", 1, TAB_STOP), 1);
        assert_eq!(visual_col("abc", 3, TAB_STOP), 3);
    }

    #[test]
    fn visual_col_tab_expanded() {
        // "a\t" — 'a' at col 0, tab jumps to col 4
        assert_eq!(visual_col("a\t", 2, TAB_STOP), 4);
    }

    #[test]
    fn visual_col_wide_char() {
        // Each CJK char is width 2
        assert_eq!(visual_col("日本語", 1, TAB_STOP), 2);
        assert_eq!(visual_col("日本語", 2, TAB_STOP), 4);
    }

    // ---- render_line ----

    #[test]
    fn render_line_simple() {
        let b = buf("hello");
        assert_eq!(render_line(&b, 0, 0, 80), "hello");
    }

    #[test]
    fn render_line_with_left_offset() {
        let b = buf("abcdef");
        assert_eq!(render_line(&b, 0, 2, 80), "cdef");
    }

    #[test]
    fn render_line_truncates_to_max_width() {
        let b = buf("abcdef");
        assert_eq!(render_line(&b, 0, 0, 3), "abc");
    }

    #[test]
    fn render_line_expands_tab() {
        let b = buf("a\tb");
        // 'a' + tab (3 spaces to reach col 4) + 'b'
        assert_eq!(render_line(&b, 0, 0, 80), "a   b");
    }

    #[test]
    fn render_line_handles_offset_cutting_tab() {
        let b = buf("a\tb");
        // Cols: 'a'=0, tab=1..3, 'b'=4
        // left_col=2 skips 'a' + first 2 spaces of tab, keeps last space + 'b'
        assert_eq!(render_line(&b, 0, 2, 80), "  b");
    }

    #[test]
    fn render_line_out_of_range_is_empty() {
        let b = buf("only");
        assert_eq!(render_line(&b, 5, 0, 80), "");
    }
}
