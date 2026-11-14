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

        // Hitung lebar gutter (kolom nomor baris)
        let total_lines = buffer.len_lines();
        let num_width = total_lines.max(1).to_string().len();
        let gutter_width = num_width + 1; // 1 spasi setelah angka
        let text_width = cols.saturating_sub(gutter_width).max(1);

        let cursor_line = cursor.line(buffer);
        let cursor_gcol = cursor.grapheme_col(buffer);
        let cursor_visual_col = {
            let line_text = buffer.line(cursor_line).unwrap_or_default();
            visual_col(&line_text, cursor_gcol, self.tab_width)
        };

        self.adjust_scroll(cursor_line, cursor_visual_col, content_height, text_width);

        let mut stdout = io::stdout();

        queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        for row in 0..content_height {
            let line_idx = self.top_line + row;
            queue!(stdout, MoveTo(0, row as u16))?;

            if line_idx < total_lines {
                // 1. Gambar nomor baris
                let line_num_str = format!("{:>width$} ", line_idx + 1, width = num_width);
                queue!(
                    stdout,
                    SetForegroundColor(Color::DarkGrey),
                    Print(line_num_str),
                    ResetColor,
                )?;

                // 2. Siapkan rentang seleksi & pencarian
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

                // 3. Gambar teks baris
                let rendered = render_line_with_selection(
                    buffer,
                    line_idx,
                    self.left_col,
                    text_width,
                    self.tab_width,
                    char_range,
                    full_line,
                    &search_ranges,
                );
                if !rendered.is_empty() {
                    queue!(stdout, Print(rendered))?;
                }
            } else {
                // Baris kosong di luar EOF
                let tilde_str = format!("{:>width$} ~", "", width = num_width);
                queue!(
                    stdout,
                    SetForegroundColor(Color::DarkGrey),
                    Print(tilde_str),
                    ResetColor,
                )?;
            }
        }

        // 4. Gambar prompt ATAU status bar, lalu posisikan kursor
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

            // Offset kursor X ditambah lebar gutter
            let cx = (gutter_width + cursor_visual_col.saturating_sub(self.left_col) as usize)
                .min(cols.saturating_sub(1)) as u16;
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
    search_ranges: &[(usize, usize, bool)],
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
                StyleState::Search => out.push_str("\x1b[43;30m"),
                StyleState::ActiveSearch => out.push_str("\x1b[103;30m"),
                StyleState::Selection => out.push_str("\x1b[7m"),
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