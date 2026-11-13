use sarang_qdte::cli;
use sarang_qdte::editor::buffer::Buffer;
use sarang_qdte::editor::cursor::Cursor;
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
                println!("cursor      = char {} (line {}, col {})",
                    cursor.char_idx(),
                    cursor.line(&buffer),
                    cursor.grapheme_col(&buffer),
                );

                // Demo navigation
                cursor.move_buffer_end(&buffer);
                println!("after end   = char {} (line {}, col {})",
                    cursor.char_idx(),
                    cursor.line(&buffer),
                    cursor.grapheme_col(&buffer),
                );
            }
            Err(e) => {
                eprintln!("error reading {:?}: {}", path, e);
                std::process::exit(1);
            }
        },
        None => {
            println!("file        = None (empty buffer)");
        }
    }
}