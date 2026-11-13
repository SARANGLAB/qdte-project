use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;

/// Quick & Dirty Text Editor — a terminal-based text editor.
///
/// If FILE is provided, QDTE opens it for editing. If the file does not exist,
/// an empty buffer is created; the file is only written to disk on save.
#[derive(Parser, Debug)]
#[command(
    name = "qdte",
    bin_name = "qdte",
    version,
    about = "Quick & Dirty Text Editor"
)]
pub struct Cli {
    pub file: Option<PathBuf>,
}

/// How QDTE was invoked, detected from argv[0].
///
/// Digunakan nanti untuk memilih keybinding profile default saat dipanggil
/// lewat symlink (mis. `nano` -> normal, `vim` -> vim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invocation {
    Qdte,
    Nano,
    Vim,
    Other,
}

impl Invocation {
    pub fn detect(argv0: &OsString) -> Self {
        let name = std::path::Path::new(argv0)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match name {
            "qdte" => Self::Qdte,
            "nano" => Self::Nano,
            "vim" | "nvim" | "vi" => Self::Vim,
            _ => Self::Other,
        }
    }
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub cli: Cli,
    pub invocation: Invocation,
}

pub fn parse() -> ParsedArgs {
    let argv0 = std::env::args_os()
        .next()
        .unwrap_or_else(|| OsString::from("qdte"));
    let invocation = Invocation::detect(&argv0);
    let cli = Cli::parse();
    ParsedArgs { cli, invocation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_file() {
        let cli = Cli::try_parse_from(["qdte"]).unwrap();
        assert!(cli.file.is_none());
    }

    #[test]
    fn parses_file_argument() {
        let cli = Cli::try_parse_from(["qdte", "notes.txt"]).unwrap();
        assert_eq!(cli.file.unwrap().to_str().unwrap(), "notes.txt");
    }

    #[test]
    fn detects_qdte() {
        assert_eq!(Invocation::detect(&OsString::from("qdte")), Invocation::Qdte);
        assert_eq!(
            Invocation::detect(&OsString::from("/usr/local/bin/qdte")),
            Invocation::Qdte
        );
    }

    #[test]
    fn detects_nano_symlink() {
        assert_eq!(Invocation::detect(&OsString::from("nano")), Invocation::Nano);
        assert_eq!(
            Invocation::detect(&OsString::from("/home/me/.local/bin/nano")),
            Invocation::Nano
        );
    }

    #[test]
    fn detects_vim_symlink() {
        assert_eq!(Invocation::detect(&OsString::from("vim")), Invocation::Vim);
        assert_eq!(Invocation::detect(&OsString::from("nvim")), Invocation::Vim);
        assert_eq!(Invocation::detect(&OsString::from("vi")), Invocation::Vim);
    }

    #[test]
    fn detects_other() {
        assert_eq!(Invocation::detect(&OsString::from("emacs")), Invocation::Other);
    }
}
