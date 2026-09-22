use std::{env, fs, path::Path};

fn main() {
    let mut entries = Vec::new();
    for dir in ["docs", "docs/glyphs"] {
        println!("cargo:rerun-if-changed={dir}");
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|s| s == "md") {
                entries.push(format!("({:?}, include_str!({:?}))", path.file_stem().unwrap().to_str().unwrap(), path.canonicalize().unwrap()));
            }
        }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[(&str, &str)] = &[{}];", entries.join(",\n"))).unwrap();
}
