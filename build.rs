use std::{env, fs, path::Path};

/// Sets `cfg(web)` for the browser target, `wasm32-unknown-unknown`, and embeds the help text.
///
/// The help covers each glyph page in `nbs/glyphs`, and each named block of `nbs/system-functions.qmd`. A `<!-- help •x •y -->`
/// line starts the help for `•x` and `•y`, and a bare `<!-- help -->` starts an unnamed block. Each block runs to the next marker,
/// and its first line, the block's heading, is left out of the help. Embedded glyph links are relative to the documentation root.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(web)");
    if env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32") && env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "unknown") {
        println!("cargo::rustc-cfg=web");
    }
    println!("cargo::rerun-if-changed=nbs/glyphs");
    println!("cargo::rerun-if-changed=nbs/system-functions.qmd");
    let paths: Vec<_> =
        fs::read_dir("nbs/glyphs").unwrap().map(|entry| entry.unwrap().path()).filter(|path| path.extension().is_some_and(|s| s == "qmd")).collect();
    let mut entries: Vec<_> = paths
        .iter()
        .map(|path| {
            let mut help = fs::read_to_string(path).unwrap().replace("](../", "](");
            for target in &paths {
                let name = target.file_name().unwrap().to_str().unwrap();
                help = help.replace(&format!("]({name}"), &format!("](glyphs/{name}"));
            }
            format!("({:?}, {help:?})", path.file_stem().unwrap().to_str().unwrap())
        })
        .collect();
    let page = fs::read_to_string("nbs/system-functions.qmd").unwrap();
    for block in page.split("<!-- help").skip(1) {
        let (marker, text) = block.split_once("-->").unwrap();
        let help = text.trim().split_once('\n').map_or("", |(_, body)| body).trim();
        for name in marker.split_whitespace() { entries.push(format!("({name:?}, {help:?})")); }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[(&str, &str)] = &[{}];", entries.join(",\n"))).unwrap();
}
