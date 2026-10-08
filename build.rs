use std::{env, fs, path::Path};

/// Sets `cfg(web)` for the browser target, `wasm32-unknown-unknown`, and embeds the help text.
///
/// The help is the glyph pages in `nbs/glyphs` and the system pages in `nbs/system`, each embedded whole. A glyph page's key is its
/// file stem. A system page's keys are the `•` and `$` names in backticks in its headings, such as `•json` in a title, or `$a` in a
/// section of the constants page. Embedded links are relative to the documentation root.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(web)");
    if env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32") && env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "unknown") {
        println!("cargo::rustc-cfg=web");
    }
    let mut entries = Vec::new();
    for dir in ["glyphs", "system"] {
        println!("cargo::rerun-if-changed=nbs/{dir}");
        let paths: Vec<_> = fs::read_dir(format!("nbs/{dir}"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|s| s == "qmd"))
            .collect();
        for path in &paths {
            let mut help = fs::read_to_string(path).unwrap();
            for target in &paths {
                let name = target.file_name().unwrap().to_str().unwrap();
                help = help.replace(&format!("]({name}"), &format!("]({dir}/{name}"));
            }
            let help = help.replace("](../", "](");
            let keys: Vec<&str> = if dir == "glyphs" { vec![path.file_stem().unwrap().to_str().unwrap()] } else {
                let mut fenced = false;
                let headings = help.lines().filter(|line| { fenced ^= line.starts_with("```"); !fenced && line.starts_with('#') });
                headings.flat_map(|line| line.split('`').skip(1).step_by(2)).filter(|name| name.starts_with(['•', '$'])).collect()
            };
            for key in keys { entries.push(format!("({key:?}, {help:?})")); }
        }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[(&str, &str)] = &[{}];", entries.join(",\n"))).unwrap();
}
