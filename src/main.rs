mod app;
mod cli;
mod editor;
mod io;
mod keymap;
mod terminal;

fn main() {
    let args = cli::args::parse();

    // Placeholder sementara — dihapus setelah Application layer terhubung.
    println!("sarang-qdte v{}", env!("CARGO_PKG_VERSION"));
    println!("invocation = {:?}", args.invocation);
    println!("file       = {:?}", args.cli.file);
}