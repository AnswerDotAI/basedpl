use std::{env, fs, path::Path};

/// Help text for each glyph page in `nbs/glyphs`, and for each named block of `nbs/system-functions.qmd`. A `<!-- help •x •y -->`
/// line starts the help for `•x` and `•y`, and a bare `<!-- help -->` starts an unnamed block. Each block runs to the next marker,
/// and its first line, the block's heading, is left out of the help.
fn main() {
    println!("cargo:rerun-if-changed=nbs/glyphs");
    println!("cargo:rerun-if-changed=nbs/system-functions.qmd");
    let paths = fs::read_dir("nbs/glyphs").unwrap().map(|entry| entry.unwrap().path());
    let mut entries: Vec<_> = paths
        .filter(|path| path.extension().is_some_and(|s| s == "qmd"))
        .map(|path| format!("({:?}, include_str!({:?}))", path.file_stem().unwrap().to_str().unwrap(), path.canonicalize().unwrap()))
        .collect();
    let page = fs::read_to_string("nbs/system-functions.qmd").unwrap();
    for block in page.split("<!-- help").skip(1) {
        let (marker, text) = block.split_once("-->").unwrap();
        let help = text.trim().split_once('\n').map_or("", |(_, body)| body).trim();
        for name in marker.split_whitespace() { entries.push(format!("({name:?}, {help:?})")); }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[(&str, &str)] = &[{}];", entries.join(",\n"))).unwrap();
}
