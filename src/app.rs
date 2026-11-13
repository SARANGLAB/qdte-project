use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use thiserror::Error;

use crate::editor::{Action, Editor, EditorError, Mode};
use crate::io::{self, IoError, LineEnding};

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
    saved_hash: Option<u64>,
}

impl App {
    pub fn new_empty() -> Self {
        let editor = Editor::new();
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        Self {
            editor,
            path: None,
            line_ending: LineEnding::default_for_platform(),
            had_bom: false,
            message: None,
            mode: Mode::Normal,
            saved_hash,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self, IoError> {
        let loaded = io::reader::read(&path)?;
        let editor = Editor::from_str(&loaded.content);
        let saved_hash = Some(hash_str(&editor.buffer().as_string()));
        Ok(Self {
            editor,
            path: Some(path),
            line_ending: loaded.line_ending,
            had_bom: loaded.had_bom,
            message: None,
            mode: Mode::Normal,
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
        let content = self.editor.buffer().as_string();
        io::writer::write(&path, &content, self.line_ending, self.had_bom)?;
        self.saved_hash = Some(hash_str(&content));
        self.message = Some(format!("saved: {}", path.display()));
        Ok(())
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}