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

pub struct App {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub had_bom: bool,
    pub message: Option<String>,
    pub mode: Mode,
    pub config: Config,
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
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}