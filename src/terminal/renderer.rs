use std::io::{self, Write};

use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::editor::buffer::Buffer;

pub struct Renderer {
    top_line: usize,
    left_col: usize,
    tab_width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleState {
    Normal,
    Selection,
    Search,
    ActiveSearch,
}

impl Renderer {
    pub fn new(tab_width: usize) -> Self {
        Self {
            top_line: 0,
            left_col: 0,
            tab_width: tab_width.max(1),
        }
    }

    pub fn render(&mut self, app: &App, mode_label: &str, block_cursor: bool) -> io::Result<()> {
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
            visual_col(&line_text, cursor_gcol, self.tab_width)
        };

        self.adjust_scroll(cursor_line, cursor_visual_col, content_height, cols);

        let mut stdout = io::stdout();

        queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        for row in 0..content_height {
            let line_idx = self.top_line + row;
            queue!(stdout, MoveTo(0, row as u16))?;
            if line_idx < buffer.len_lines() {
                let sel = app.visual_range();
                let (char_range, full_line) = if let Some((s, e, lw)) = sel {
                    if lw {
                        let line_start = buffer.line_to_char(line_idx);
                        let line_end = line_start + buffer.line(line_idx).unwrap_or_default().chars().count();
                        (Some((line_start, line_end)), s <= line_start && line_end <= e)
                    } else {
                        (Some((s, e)), false)
                    }
                } else {
                    (None, false)
                };

                let mut search_ranges = Vec::new();
                if let Some(search) = &app.search {
                    let line_start = buffer.line_to_char(line_idx);
                    let line_end = line_start + buffer.line(line_idx).unwrap_or_default().chars().count();

                    for (i, m) in search.matches.iter().enumerate() {
                        if m.start >= line_start && m.start < line_end {
                            let is_active = i == search.current_idx;
                            search_ranges.push((m.start, m.end, is_active));
                        }
                    }
                }

                let rendered = render_line_with_selection(
                    buffer,
                    line_idx,
                    self.left_col,
                    cols,
                    self.tab_width,
                    char_range,
                    full_line,
                    &search_ranges,
                );
                if !rendered.is_empty() {
                    queue!(stdout, Print(rendered))?;
                }
            } else {
                queue!(stdout, Print("~"))?;
            }
        }

        // Draw prompt OR status bar (exactly one), and compute cursor position.
        let (cx, cy, use_block) = if let Some(prompt) = &app.prompt {
            let prompt_line = format_prompt(prompt, cols);
            queue!(
                stdout,
                MoveTo(0, content_height as u16),
                SetBackgroundColor(Color::Blue),
                SetForegroundColor(Color::White),
                Print(prompt_line),
                ResetColor,
            )?;

            let prefix = format!("[{}] ", prompt.kind.label());
            let prefix_w = UnicodeWidthStr::width(prefix.as_str());
            let input_before: String = prompt.input.chars().take(prompt.cursor).collect();
            let input_w = UnicodeWidthStr::width(input_before.as_str());
            let cx = ((prefix_w + input_w).min(cols.saturating_sub(1))) as u16;
            (cx, content_height as u16, false)
        } else {
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
            (cx, cy, block_cursor)
        };

        let style = if use_block {
            SetCursorStyle::SteadyBlock
        } else {
            SetCursorStyle::SteadyBar
        };
        queue!(stdout, MoveTo(cx, cy), style)?;

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

// ---- pure helpers (tested) ------------------------------------------------

pub(crate) fn visual_col(line_text: &str, grapheme_count: usize, tab_width: usize) -> usize {
    let mut w = 0usize;
    for (i, g) in line_text.graphemes(true).enumerate() {
        if i >= grapheme_count {
            break;
        }
        if g == "\t" {
            w += tab_width - (w % tab_width);
        } else {
            w += UnicodeWidthStr::width(g);
        }
    }
    w
}

pub(crate) fn render_line_with_selection(
    buffer: &Buffer,
    line_idx: usize,
    left_col: usize,
    max_width: usize,
    tab_width: usize,
    selection: Option<(usize, usize)>,
    full_line: bool,
    search_ranges: &[(usize, usize, bool)], // (start, end, is_active)
) -> String {
    let text = buffer.line(line_idx).unwrap_or_default();
    let line_start = buffer.line_to_char(line_idx);

    let mut out = String::new();
    let mut visual = 0usize;
    let mut emitted = 0usize;
    let mut char_offset = 0usize;
    let mut current_style = StyleState::Normal;

    for g in text.graphemes(true) {
        let gw = if g == "\t" {
            tab_width - (visual % tab_width)
        } else {
            UnicodeWidthStr::width(g)
        };

        let abs_pos = line_start + char_offset;

        let is_selected = if full_line {
            true
        } else if let Some((s, e)) = selection {
            abs_pos >= s && abs_pos < e
        } else {
            false
        };

        let mut is_search = false;
        let mut is_active_search = false;
        for &(s, e, active) in search_ranges {
            if abs_pos >= s && abs_pos < e {
                if active {
                    is_active_search = true;
                } else {
                    is_search = true;
                }
                break;
            }
        }

        let new_style = if is_active_search {
            StyleState::ActiveSearch
        } else if is_search {
            StyleState::Search
        } else if is_selected {
            StyleState::Selection
        } else {
            StyleState::Normal
        };

        if new_style != current_style {
            match new_style {
                StyleState::Normal => out.push_str("\x1b[0m"),
                StyleState::Search => out.push_str("\x1b[43;30m"), // bg kuning, fg hitam
                StyleState::ActiveSearch => out.push_str("\x1b[103;30m"), // bg kuning terang, fg hitam
                StyleState::Selection => out.push_str("\x1b[7m"), // reverse video
            }
            current_style = new_style;
        }

        if visual + gw <= left_col {
            visual += gw;
            char_offset += g.chars().count();
            continue;
        }
        if emitted + gw > max_width {
            break;
        }

        if visual < left_col {
            let skip = left_col - visual;
            out.push_str(&" ".repeat(gw - skip));
        } else if g == "\t" {
            out.push_str(&" ".repeat(gw));
        } else {
            out.push_str(g);
        }

        emitted += gw;
        visual += gw;
        char_offset += g.chars().count();
    }

    if current_style != StyleState::Normal {
        out.push_str("\x1b[0m");
    }

    if full_line && emitted < max_width {
        out.push_str("\x1b[7m");
        out.push_str(&" ".repeat(max_width - emitted));
        out.push_str("\x1b[0m");
    }

    out
}

fn format_prompt(prompt: &crate::app::Prompt, width: usize) -> String {
    let prefix = format!("[{}] ", prompt.kind.label());
    let mut line = String::new();
    line.push_str(&prefix);
    line.push_str(&prompt.input);
    if let Some(note) = &prompt.note {
        line.push_str("  ");
        line.push_str(note);
    }

    let mut out = String::new();
    let mut w = 0usize;
    for g in line.graphemes(true) {
        let gw = UnicodeWidthStr::width(g);
        if w + gw > width {
            break;
        }
        out.push_str(g);
        w += gw;
    }
    while w < width {
        out.push(' ');
        w += 1;
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

    const TAB_WIDTH: usize = 4;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    #[test]
    fn visual_col_empty() {
        assert_eq!(visual_col("", 0, TAB_WIDTH), 0);
    }

    #[test]
    fn visual_col_ascii() {
        assert_eq!(visual_col("abc", 0, TAB_WIDTH), 0);
        assert_eq!(visual_col("abc", 1, TAB_WIDTH), 1);
        assert_eq!(visual_col("abc", 3, TAB_WIDTH), 3);
    }

    #[test]
    fn visual_col_tab_expanded() {
        assert_eq!(visual_col("a\t", 2, TAB_WIDTH), 4);
    }

    #[test]
    fn visual_col_wide_char() {
        assert_eq!(visual_col("日本語", 1, TAB_WIDTH), 2);
        assert_eq!(visual_col("日本語", 2, TAB_WIDTH), 4);
    }

    #[test]
    fn visual_col_custom_tab_width() {
        assert_eq!(visual_col("a\t", 2, 8), 8);
    }

    #[test]
    fn render_line_simple() {
        let b = buf("hello");
        assert_eq!(
            render_line_with_selection(&b, 0, 0, 80, TAB_WIDTH, None, false, &[]),
            "hello"
        );
    }

    #[test]
    fn render_line_with_left_offset() {
        let b = buf("abcdef");
        assert_eq!(
            render_line_with_selection(&b, 0, 2, 80, TAB_WIDTH, None, false, &[]),
            "cdef"
        );
    }

    #[test]
    fn render_line_truncates_to_max_width() {
        let b = buf("abcdef");
        assert_eq!(
            render_line_with_selection(&b, 0, 0, 3, TAB_WIDTH, None, false, &[]),
            "abc"
        );
    }

    #[test]
    fn render_line_expands_tab() {
        let b = buf("a\tb");
        assert_eq!(
            render_line_with_selection(&b, 0, 0, 80, TAB_WIDTH, None, false, &[]),
            "a   b"
        );
    }

    #[test]
    fn render_line_handles_offset_cutting_tab() {
        let b = buf("a\tb");
        assert_eq!(
            render_line_with_selection(&b, 0, 2, 80, TAB_WIDTH, None, false, &[]),
            "  b"
        );
    }

    #[test]
    fn render_line_out_of_range_is_empty() {
        let b = buf("only");
        assert_eq!(
            render_line_with_selection(&b, 5, 0, 80, TAB_WIDTH, None, false, &[]),
            ""
        );
    }

    #[test]
    fn render_line_respects_custom_tab_width() {
        let b = buf("a\tb");
        assert_eq!(
            render_line_with_selection(&b, 0, 0, 80, 8, None, false, &[]),
            "a       b"
        );
    }

    #[test]
    fn render_line_with_char_selection_highlights_range() {
        let b = buf("abcdef");
        let out = render_line_with_selection(&b, 0, 0, 80, TAB_WIDTH, Some((1, 4)), false, &[]);
        assert!(out.contains("\x1b[7m"));
        assert!(out.contains("\x1b[0m"));
        assert!(out.contains("bcd"));
    }

    #[test]
    fn render_line_full_line_highlight() {
        let b = buf("abc");
        let out = render_line_with_selection(&b, 0, 0, 10, TAB_WIDTH, Some((0, 3)), true, &[]);
        assert!(out.starts_with("\x1b[7m"));
        assert!(out.contains("\x1b[0m"));
    }

    #[test]
    fn format_prompt_basic() {
        use crate::app::{Prompt, PromptKind};
        let p = Prompt::new(PromptKind::Open, "hello".to_string());
        let s = format_prompt(&p, 20);
        assert!(s.starts_with("[Open] hello"));
        assert_eq!(s.chars().count(), 20);
    }

    #[test]
    fn format_prompt_with_note() {
        use crate::app::{Prompt, PromptKind};
        let mut p = Prompt::new(PromptKind::SaveAs, "out.txt".to_string());
        p.note = Some("save failed".to_string());
        let s = format_prompt(&p, 40);
        assert!(s.contains("save failed"));
        assert_eq!(s.chars().count(), 40);
    }
}