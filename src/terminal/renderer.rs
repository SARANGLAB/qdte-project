use std::io::{self, Write};
use std::path::PathBuf;
use syntect::highlighting::Style;

use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use syntect::easy::HighlightLines;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::editor::buffer::Buffer;
use crate::terminal::highlight::{get_syntax_for_path, PS};

pub struct Renderer {
    top_line: usize,
    left_col: usize,
    tab_width: usize,
    // Cache Syntax Highlight
    cached_path: Option<PathBuf>,
    cached_theme: String,
    cached_hash: u64,
    cached_spans: Vec<Vec<(Style, String)>>,
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
            cached_path: None,
            cached_theme: String::new(),
            cached_hash: 0,
            cached_spans: Vec::new(),
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

        let total_lines = buffer.len_lines();
        let num_width = total_lines.max(1).to_string().len();
        let gutter_width = num_width + 1;
        let text_width = cols.saturating_sub(gutter_width).max(1);

        let cursor_line = cursor.line(buffer);
        let cursor_gcol = cursor.grapheme_col(buffer);
        let cursor_visual_col = {
            let line_text = buffer.line(cursor_line).unwrap_or_default();
            visual_col(&line_text, cursor_gcol, self.tab_width)
        };

        self.adjust_scroll(cursor_line, cursor_visual_col, content_height, text_width);

        let mut stdout = io::stdout();
        
        let active_theme = app.theme_manager.active();
        let ui = &active_theme.ui;

        queue!(stdout, SetBackgroundColor(ui.bg), SetForegroundColor(ui.fg), MoveTo(0, 0), Clear(ClearType::All))?;

        // --- CACHE SYNTAX HIGHLIGHTING ---
        let current_hash = app.editor.hash();
        let current_theme = app.theme_manager.active().name.clone();

        if self.cached_path != app.path || self.cached_theme != current_theme || self.cached_hash != current_hash {
            self.cached_spans.clear();
            let syntax = get_syntax_for_path(app.path.as_deref());
            let mut highlighter = HighlightLines::new(syntax, &active_theme.syntect);
            
            for line_idx in 0..total_lines {
                let line_text = buffer.line(line_idx).unwrap_or_default();
                let line_nl = format!("{}\n", line_text);
                let spans = highlighter.highlight_line(&line_nl, &PS).unwrap_or_default();
                let owned_spans = spans.into_iter().map(|(s, t)| (s, t.to_string())).collect();
                self.cached_spans.push(owned_spans);
            }
            
            self.cached_path = app.path.clone();
            self.cached_theme = current_theme;
            self.cached_hash = current_hash;
        }
        // ---------------------------------

        for row in 0..content_height {
            let line_idx = self.top_line + row;
            queue!(stdout, MoveTo(0, row as u16))?;

            if line_idx < total_lines {
                let line_num_str = format!("{:>width$} ", line_idx + 1, width = num_width);
                queue!(
                    stdout,
                    SetForegroundColor(ui.gutter),
                    Print(line_num_str),
                    SetForegroundColor(ui.fg),
                )?;

                let line_text = buffer.line(line_idx).unwrap_or_default();
                
                // Baca dari cache
                let spans = &self.cached_spans[line_idx];
                let span_refs: Vec<(Style, &str)> = spans.iter().map(|(s, t)| (*s, t.as_str())).collect();

                let sel = app.visual_range();
                let (char_range, full_line) = if let Some((s, e, lw)) = sel {
                    if lw {
                        let line_start = buffer.line_to_char(line_idx);
                        let is_selected = line_start >= s && line_start < e;
                        (None, is_selected) 
                    } else {
                        (Some((s, e)), false)
                    }
                } else {
                    (None, false)
                };

                let mut search_ranges = Vec::new();
                if let Some(search) = &app.search {
                    let line_start = buffer.line_to_char(line_idx);
                    let line_end = line_start + line_text.chars().count();

                    for (i, m) in search.matches.iter().enumerate() {
                        if m.start >= line_start && m.start < line_end {
                            let is_active = i == search.current_idx;
                            search_ranges.push((m.start, m.end, is_active));
                        }
                    }
                }

                let rendered = render_line_highlighted(
                    buffer,
                    line_idx,
                    self.left_col,
                    text_width,
                    self.tab_width,
                    char_range,
                    full_line,
                    &search_ranges,
                    &span_refs, // Gunakan span_refs
                    ui,
                );
                if !rendered.is_empty() {
                    queue!(stdout, Print(rendered))?;
                }
            } else {
                let tilde_str = format!("{:>width$} ~", "", width = num_width);
                queue!(
                    stdout,
                    SetForegroundColor(ui.gutter),
                    Print(tilde_str),
                )?;
            }
        }

        // Gambar Theme Picker di atas segalanya
        if let Some(picker) = &app.theme_picker {
            draw_theme_picker(&mut stdout, picker, ui, cols, rows)?;
        }

        // 5. Status Bar / Prompt
        let (cx, cy, use_block) = if let Some(prompt) = &app.prompt {
            let prompt_line = format_prompt(prompt, cols);
            queue!(
                stdout,
                MoveTo(0, content_height as u16),
                SetBackgroundColor(Color::Blue),
                SetForegroundColor(Color::White),
                Print(prompt_line),
                SetBackgroundColor(ui.bg),
                SetForegroundColor(ui.fg),
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
                SetBackgroundColor(ui.statusbar_bg),
                SetForegroundColor(ui.statusbar_fg),
                Print(status),
                SetBackgroundColor(ui.bg),
                SetForegroundColor(ui.fg),
            )?;

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
        
        // Sembunyikan kursor blok saat Theme Picker terbuka
        let final_style = if app.theme_picker.is_some() {
            SetCursorStyle::DefaultUserShape
        } else {
            style
        };

        queue!(stdout, MoveTo(cx, cy), SetForegroundColor(ui.cursor), final_style)?;

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

fn ansi_fg(c: syntect::highlighting::Color) -> String {
    format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b)
}

fn ansi_bg(c: crossterm::style::Color) -> String {
    if let crossterm::style::Color::Rgb { r, g, b } = c {
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    } else {
        "\x1b[49m".to_string()
    }
}

pub(crate) fn render_line_highlighted(
    buffer: &Buffer,
    line_idx: usize,
    left_col: usize,
    max_width: usize,
    tab_width: usize,
    selection: Option<(usize, usize)>,
    full_line: bool,
    search_ranges: &[(usize, usize, bool)],
    spans: &[(syntect::highlighting::Style, &str)],
    ui: &crate::theme::UiColors,
) -> String {
    let line_start = buffer.line_to_char(line_idx);
    let mut out = String::new();
    let mut visual = 0usize;
    let mut emitted = 0usize;
    let mut char_offset = 0usize;

    let mut current_style = StyleState::Normal;
    let mut current_fg = String::from("\x1b[39m");

    for (syn_style, text) in spans {
        let syn_fg = ansi_fg(syn_style.foreground);

        for g in text.graphemes(true) {
            if g == "\n" {
                continue;
            }

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

            if new_style != current_style || syn_fg != current_fg {
                out.push_str("\x1b[0m");
                
                match new_style {
                    StyleState::Normal => {
                        out.push_str(&ansi_bg(ui.bg));
                        out.push_str(&syn_fg);
                    }
                    StyleState::Selection => {
                        out.push_str(&ansi_bg(ui.selection));
                        out.push_str(&syn_fg);
                    }
                    StyleState::Search => {
                        out.push_str("\x1b[43;30m");
                    }
                    StyleState::ActiveSearch => {
                        out.push_str("\x1b[103;30m");
                    }
                }
                current_style = new_style;
                current_fg = syn_fg.clone();
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
    }

    if current_style != StyleState::Normal || current_fg != "\x1b[39m" {
        out.push_str("\x1b[0m");
        out.push_str(&ansi_bg(ui.bg));
    }

    if full_line {
        if emitted == 0 {
            out.push_str(&ansi_bg(ui.selection));
            out.push(' ');
            out.push_str("\x1b[0m");
            out.push_str(&ansi_bg(ui.bg));
        } else {
            out.push_str(&ansi_bg(ui.selection));
            out.push(' ');
            out.push_str("\x1b[0m");
            out.push_str(&ansi_bg(ui.bg));
        }
    }

    out
}

fn draw_theme_picker(
    stdout: &mut io::Stdout,
    picker: &crate::app::ThemePickerState,
    ui: &crate::theme::UiColors,
    cols: usize,
    rows: usize,
) -> io::Result<()> {
    let width = 40.min(cols.saturating_sub(4));
    let height = (picker.themes.len() + 4).min(rows.saturating_sub(4));
    
    let start_x = (cols.saturating_sub(width)) / 2;
    let start_y = (rows.saturating_sub(height)) / 2;

    let popup_bg = ui.statusbar_bg;
    let popup_fg = ui.statusbar_fg;
    let sel_bg = ui.selection;

    // Gambar background kotak
    for y in 0..height {
        queue!(
            stdout,
            MoveTo(start_x as u16, (start_y + y) as u16),
            SetBackgroundColor(popup_bg),
            SetForegroundColor(popup_fg),
            Print(" ".repeat(width)),
        )?;
    }

    // Gambar Border Atas
    queue!(
        stdout,
        MoveTo(start_x as u16, start_y as u16),
        SetBackgroundColor(popup_bg),
        SetForegroundColor(ui.gutter),
        Print(format!("┌{}┐", "─".repeat(width.saturating_sub(2)))),
    )?;

    // Gambar Judul
    let title = " QDTE Theme Manager ";
    let title_x = start_x + (width.saturating_sub(title.len())) / 2;
    queue!(
        stdout,
        MoveTo(title_x as u16, start_y as u16),
        SetForegroundColor(popup_fg),
        Print(title),
    )?;

    // Gambar List Tema
    let list_y = start_y + 2;
    for (i, theme_name) in picker.themes.iter().enumerate() {
        if list_y + i >= start_y + height - 2 {
            break;
        }
        
        let is_selected = i == picker.selected_idx;
        let prefix = if is_selected { " > " } else { "   " };
        
        let mut display_name = theme_name.clone();
        if display_name.len() > width.saturating_sub(6) {
            display_name.truncate(width.saturating_sub(9));
            display_name.push_str("...");
        }
        
        let line = format!("{}{:<width$}", prefix, display_name, width = width.saturating_sub(4));

        queue!(
            stdout,
            MoveTo((start_x + 2) as u16, (list_y + i) as u16),
            SetBackgroundColor(if is_selected { sel_bg } else { popup_bg }),
            SetForegroundColor(if is_selected { popup_fg } else { ui.gutter }),
            Print(line),
            SetBackgroundColor(popup_bg),
        )?;
    }

    // Gambar Border Bawah
    queue!(
        stdout,
        MoveTo(start_x as u16, (start_y + height - 1) as u16),
        SetForegroundColor(ui.gutter),
        Print(format!("└{}┘", "─".repeat(width.saturating_sub(2)))),
    )?;

    // Gambar Petunjuk
    let hint_y = start_y + height;
    if hint_y < rows {
        let hint = "Enter: Apply  Esc: Cancel";
        let hint_x = start_x + (width.saturating_sub(hint.len())) / 2;
        queue!(
            stdout,
            MoveTo(hint_x as u16, hint_y as u16),
            SetForegroundColor(ui.gutter),
            SetBackgroundColor(ui.bg),
            Print(hint),
        )?;
    }

    queue!(
        stdout,
        SetBackgroundColor(ui.bg),
        SetForegroundColor(ui.fg),
    )?;

    Ok(())
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