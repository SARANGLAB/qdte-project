mod app;
mod cli;
mod editor;
mod io;
mod keymap;
mod terminal;

fn main() {
    println!("sarang-qdte v{}", env!("CARGO_PKG_VERSION"));
}