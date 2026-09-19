fn main() { std::process::exit(basedpl::cli::run(&std::env::args().skip(1).collect::<Vec<_>>())); }
