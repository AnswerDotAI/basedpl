use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=nbs/glyphs");
    let paths = fs::read_dir("nbs/glyphs").unwrap().map(|entry| entry.unwrap().path());
    let entries: Vec<_> = paths
        .filter(|path| path.extension().is_some_and(|s| s == "qmd"))
        .map(|path| format!("({:?}, include_str!({:?}))", path.file_stem().unwrap().to_str().unwrap(), path.canonicalize().unwrap()))
        .collect();
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[(&str, &str)] = &[{}];", entries.join(",\n"))).unwrap();
}
