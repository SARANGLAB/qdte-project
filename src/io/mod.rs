pub mod reader;
pub mod writer;

use std::io;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn default_for_platform() -> Self {
        if cfg!(windows) {
            Self::Crlf
        } else {
            Self::Lf
        }
    }
}

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("file is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

pub const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
