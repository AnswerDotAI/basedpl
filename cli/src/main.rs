mod cli;
mod editor;
mod kernel;
mod worker;

fn main() { std::process::exit(cli::run(&std::env::args().skip(1).collect::<Vec<_>>())); }
