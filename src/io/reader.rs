use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{IoError, LineEnding, UTF8_BOM};

#[derive(Debug)]
pub struct LoadedFile {
    pub path: PathBuf,
    /// Content with LF line endings and no BOM (normalized).
    pub content: String,
    /// Original line ending, preserved for write-back.
    pub line_ending: LineEnding,
    /// Whether the file started with a UTF-8 BOM, preserved for write-back.
    pub had_bom: bool,
    /// false if the file did not exist (new buffer).
    pub existed: bool,
}

pub fn read(path: impl AsRef<Path>) -> Result<LoadedFile, IoError> {
    let path = path.as_ref().to_path_buf();

    let mut bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(LoadedFile {
                path,
                content: String::new(),
                line_ending: LineEnding::default_for_platform(),
                had_bom: false,
                existed: false,
            });
        }
        Err(e) => return Err(IoError::Io(e)),
    };

    let had_bom = bytes.starts_with(UTF8_BOM);
    if had_bom {
        bytes.drain(..UTF8_BOM.len());
    }

    let raw = String::from_utf8(bytes)?;

    let line_ending = if raw.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    };

    let content = if line_ending == LineEnding::Crlf {
        raw.replace("\r\n", "\n")
    } else {
        raw
    };

    Ok(LoadedFile {
        path,
        content,
        line_ending,
        had_bom,
        existed: true,
    })
}
