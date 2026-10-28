mod cli;
mod editor;
mod kernel;
mod worker;

#[global_allocator]
static GUARD: rustymath::memory::Guard = rustymath::memory::Guard::new();

fn main() { std::process::exit(cli::run(&std::env::args().skip(1).collect::<Vec<_>>())); }
