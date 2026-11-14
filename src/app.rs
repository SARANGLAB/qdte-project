use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use thiserror::Error;

use crate::editor::{Action, Editor, EditorError, Mode};
use crate::io::{self, IoError, LineEnding};
use crate::config::Config;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Editor(#[from] EditorError),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error("no file path set; open QDTE with a filename to enable save")]
    NoPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Open,
    SaveAs,
}

impl PromptKind {
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::Open => "Open",
            PromptKind::SaveAs => "Save As",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    pub input: String,
    /// Cursor position in `input`, measured in chars.
    pub cursor: usize,
    /// Transient note: either an error from the last confirm, or a warning
    /// shown when opening. Cleared on next input.
    pub note: Option<String>,
}

impl Prompt {
    pub fn new(kind: PromptKind, initial: String) -> Self {
        let cursor = initial.chars().count();
        Self {
            kind,
            input: initial,
            cursor,
            note: None,
        }
    }

    pub fn input_len(&self) -> usize {
        self.input.chars().count()
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_idx = self.char_to_byte(self.cursor);
        self.input.insert(byte_idx, c);
        self.cursor += 1;
        self.note = None;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_idx = self.char_to_byte(self.cursor - 1);
        let next_byte = self.char_to_byte(self.cursor);
        self.input.replace_range(byte_idx..next_byte, "");
        self.cursor -= 1;
        self.note = None;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.input_len() {
            return;
        }
        let byte_idx = self.char_to_byte(self.cursor);
        let next_byte = self.char_to_byte(self.cursor + 1);
        self.input.replace_range(byte_idx..next_byte, "");
        self.note = None;
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input_len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input_len();
    }

    fn char_to_byte(&self, char_idx: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.input.len())
    }
}

pub struct App {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub had_bom: bool,
    pub message: Option<String>,
    pub mode: Mode,
    pub config: Config,
    pub prompt: Option<Prompt>,
    pub selection_anchor: Option<usize>,
    saved_hash: Option<u64>,
}

impl App {
    pub fn new_empty(config: Config) -> Self {
        let mut editor = Editor::new();
        editor.set_tab_config(config.tab_width, config.expand_tab);
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        Self {
            editor,
            path: None,
            line_ending: LineEnding::default_for_platform(),
            had_bom: false,
            message: None,
            mode: Mode::Normal,
            config,
            prompt: None,
            selection_anchor: None,
            saved_hash,
        }
    }

    pub fn from_file(path: PathBuf, config: Config) -> Result<Self, IoError> {
        let loaded = io::reader::read(&path)?;
        let mut editor = Editor::from_str(&loaded.content);
        editor.set_tab_config(config.tab_width, config.expand_tab);
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        Ok(Self {
            editor,
            path: Some(path),
            line_ending: loaded.line_ending,
            had_bom: loaded.had_bom,
            message: None,
            mode: Mode::Normal,
            config,
            prompt: None,
            selection_anchor: None,
            saved_hash,
        })
    }

    pub fn is_dirty(&self) -> bool {
        match self.saved_hash {
            Some(saved) => saved != hash_str(&self.editor.buffer().as_string()),
            None => self.editor.buffer().len_chars() > 0,
        }
    }

    pub fn apply_editor(&mut self, action: Action) -> Result<(), AppError> {
        self.editor.apply(action)?;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        let path = self.path.clone().ok_or(AppError::NoPath)?;
        let mut content = self.editor.buffer().as_string();
        if self.config.fixeol && !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        io::writer::write(&path, &content, self.line_ending, self.had_bom)?;
        // Hash of buffer content (unmodified by fixeol).
        self.saved_hash = Some(hash_str(&self.editor.buffer().as_string()));
        self.message = Some(format!("saved: {}", path.display()));
        Ok(())
    }
    pub fn open_prompt(&mut self, kind: PromptKind) {
        self.message = None;
        let initial = match kind {
            PromptKind::SaveAs => self
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            PromptKind::Open => String::new(),
        };
        let mut prompt = Prompt::new(kind.clone(), initial);
        if kind == PromptKind::Open && self.is_dirty() {
            prompt.note = Some("unsaved changes will be lost".to_string());
        }
        self.prompt = Some(prompt);
    }

    pub fn cancel_prompt(&mut self) {
        self.prompt = None;
    }

    pub fn visual_range(&self) -> Option<(usize, usize, bool)> {
        let anchor = self.selection_anchor?;
        let cursor_idx = self.editor.cursor().char_idx();
        let buffer = self.editor.buffer();
        let linewise = matches!(self.mode, Mode::VisualLine);

        if linewise {
            let anchor_line = buffer.char_to_line(anchor);
            let cursor_line = buffer.char_to_line(cursor_idx);
            let (start_line, end_line) = if anchor_line <= cursor_line {
                (anchor_line, cursor_line)
            } else {
                (cursor_line, anchor_line)
            };
            let start = buffer.line_to_char(start_line);
            let end = if end_line + 1 < buffer.len_lines() {
                buffer.line_to_char(end_line + 1)
            } else {
                buffer.len_chars()
            };
            Some((start, end, true))
        } else {
            let (lo, hi) = if anchor <= cursor_idx {
                (anchor, cursor_idx)
            } else {
                (cursor_idx, anchor)
            };
            let hi_end = crate::editor::Cursor::at(hi)
                .next_grapheme_end(buffer)
                .unwrap_or(hi + 1);
            let end = hi_end.min(buffer.len_chars()).max(hi + 1).min(buffer.len_chars());
            Some((lo, end, false))
        }
    }

    pub fn confirm_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };

        let path_str = prompt.input.trim().to_string();
        if path_str.is_empty() {
            return;
        }
        let path = PathBuf::from(&path_str);

        match prompt.kind {
            PromptKind::Open => {
                // If the file doesn't exist and we have unsaved changes,
                // refuse rather than silently discard.
                let existed = path.exists();
                if !existed && self.is_dirty() {
                    let mut prompt = prompt;
                    prompt.note =
                        Some("file does not exist; save current buffer first".to_string());
                    self.prompt = Some(prompt);
                    return;
                }
                match Self::from_file(path.clone(), self.config.clone()) {
                    Ok(new_app) => {
                        self.editor = new_app.editor;
                        self.path = new_app.path;
                        self.line_ending = new_app.line_ending;
                        self.had_bom = new_app.had_bom;
                        self.saved_hash = new_app.saved_hash;
                        self.mode = Mode::Normal;
                        self.message = Some(format!("opened: {}", path.display()));
                    }
                    Err(e) => {
                        let mut prompt = prompt;
                        prompt.note = Some(format!("open failed: {}", e));
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::SaveAs => {
                let content = self.editor.buffer().as_string();
                let content_to_write =
                    if self.config.fixeol && !content.is_empty() && !content.ends_with('\n') {
                        let mut s = content.clone();
                        s.push('\n');
                        s
                    } else {
                        content.clone()
                    };
                match io::writer::write(&path, &content_to_write, self.line_ending, self.had_bom) {
                    Ok(()) => {
                        self.path = Some(path.clone());
                        self.saved_hash = Some(hash_str(&content));
                        self.message = Some(format!("saved: {}", path.display()));
                    }
                    Err(e) => {
                        let mut prompt = prompt;
                        prompt.note = Some(format!("save failed: {}", e));
                        self.prompt = Some(prompt);
                    }
                }
            }
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}