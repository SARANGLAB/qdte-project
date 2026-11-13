use std::fs;
use std::path::Path;

use super::{IoError, LineEnding, UTF8_BOM};

pub fn write(
    path: impl AsRef<Path>,
    content: &str,
    line_ending: LineEnding,
    had_bom: bool,
) -> Result<(), IoError> {
    let mut bytes: Vec<u8> = Vec::with_capacity(content.len() + 3);

    if had_bom {
        bytes.extend_from_slice(UTF8_BOM);
    }

    match line_ending {
        LineEnding::Lf => bytes.extend_from_slice(content.as_bytes()),
        LineEnding::Crlf => {
            for chunk in content.split_inclusive('\n') {
                if let Some(stripped) = chunk.strip_suffix('\n') {
                    bytes.extend_from_slice(stripped.as_bytes());
                    bytes.extend_from_slice(b"\r\n");
                } else {
                    bytes.extend_from_slice(chunk.as_bytes());
                }
            }
        }
    }

    fs::write(path, bytes)?;
    Ok(())
}