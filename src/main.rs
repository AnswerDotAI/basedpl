fn main() { std::process::exit(miniapl::cli::run(&std::env::args().skip(1).collect::<Vec<_>>())); }
