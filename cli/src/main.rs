mod cli;
mod editor;
mod kernel;
mod worker;

/// The memory guard stops a runaway computation at half of physical memory.
#[global_allocator]
static GUARD: rustymath::memory::Guard = rustymath::memory::Guard::new();

fn main() { std::process::exit(cli::run(&std::env::args().skip(1).collect::<Vec<_>>())); }
