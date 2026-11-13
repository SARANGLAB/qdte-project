use sarang_qdte::cli;
use sarang_qdte::editor::buffer::Buffer;
use sarang_qdte::editor::cursor::Cursor;
use sarang_qdte::editor::{Action, Editor};
use sarang_qdte::io;

fn main() {
    let args = cli::args::parse();

    println!("sarang-qdte v{}", env!("CARGO_PKG_VERSION"));
    println!("invocation = {:?}", args.invocation);

    match args.cli.file {
        Some(path) => match io::reader::read(&path) {
            Ok(loaded) => {
                let buffer = Buffer::from_str(&loaded.content);
                let mut cursor = Cursor::new();
                cursor.clamp(&buffer);

                println!("path        = {:?}", loaded.path);
                println!("existed     = {}", loaded.existed);
                println!("line ending = {:?}", loaded.line_ending);
                println!("bom         = {}", loaded.had_bom);
                println!("lines       = {}", buffer.len_lines());
                println!("chars       = {}", buffer.len_chars());
                println!(
                    "cursor      = char {} (line {}, col {})",
                    cursor.char_idx(),
                    cursor.line(&buffer),
                    cursor.grapheme_col(&buffer),
                );

                // Editor demo: apply a small script of actions.
                let mut ed = Editor::from_str("hello");
                ed.apply(Action::MoveBufferEnd).ok();
                ed.apply(Action::InsertChar('!')).ok();
                println!("editor demo = {:?}", ed.buffer().as_string());
            }
            Err(e) => {
                eprintln!("error reading {:?}: {}", path, e);
                std::process::exit(1);
            }
        },
        None => {
            let mut ed = Editor::new();
            for c in "Quick & Dirty".chars() {
                ed.apply(Action::InsertChar(c)).ok();
            }
            println!("file        = None (empty buffer)");
            println!("editor demo = {:?}", ed.buffer().as_string());
        }
    }
}