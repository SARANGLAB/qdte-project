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
    pub dirty: bool,
    pub message: Option<String>,
    pub mode: Mode,
}

impl App {
    pub fn new_empty() -> Self {
        Self {
            editor: Editor::new(),
            path: None,
            line_ending: LineEnding::default_for_platform(),
            had_bom: false,
            dirty: false,
            message: None,
            mode: Mode::Normal,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self, IoError> {
        let loaded = io::reader::read(&path)?;
        Ok(Self {
            editor: Editor::from_str(&loaded.content),
            path: Some(path),
            line_ending: loaded.line_ending,
            had_bom: loaded.had_bom,
            dirty: false,
            message: None,
            mode: Mode::Normal,
        })
    }

    pub fn apply_editor(&mut self, action: Action) -> Result<(), AppError> {
        let is_mutation = action.is_mutation();
        self.editor.apply(action)?;
        if is_mutation {
            self.dirty = true;
        }
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        let path = self.path.clone().ok_or(AppError::NoPath)?;
        let content = self.editor.buffer().as_string();
        io::writer::write(&path, &content, self.line_ending, self.had_bom)?;
        self.dirty = false;
        self.message = Some(format!("saved: {}", path.display()));
        Ok(())
    }
}
