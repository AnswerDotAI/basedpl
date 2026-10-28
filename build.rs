use std::{env, fs, path::Path};

fn page_text(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap();
    if path.extension().is_none_or(|s| s != "ipynb") { return text; }
    let notebook: serde_json::Value = serde_json::from_str(&text).unwrap();
    let language = notebook["metadata"]["kernelspec"]["language"].as_str().unwrap_or("bpl");
    let mut text = String::new();
    for cell in notebook["cells"].as_array().unwrap() {
        let source = match &cell["source"] {
            serde_json::Value::String(s) => s.clone(), lines => lines.as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect()
        };
        match cell["cell_type"].as_str().unwrap() {
            "markdown" => text.push_str(&source), "code" => text.push_str(&format!("```{language}\n{source}\n```")), _ => continue
        }
        text.push_str("\n\n");
    }
    text
}

/// Embeds the reference pages and guides, including notebook source but not outputs.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(web)");
    if env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32") && env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "unknown") {
        println!("cargo::rustc-cfg=web");
    }
    let mut entries = Vec::new();
    for dir in ["", "glyphs", "system"] {
        let folder = Path::new("nbs").join(dir);
        println!("cargo::rerun-if-changed={}", folder.display());
        let mut paths: Vec<_> = fs::read_dir(&folder).unwrap().map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|s| s == "qmd" || s == "ipynb")).collect();
        paths.sort();
        for path in &paths {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let name = if dir.is_empty() { stem.to_owned() } else { format!("{dir}/{stem}") };
            let mut help = page_text(path);
            if !dir.is_empty() {
                for target in &paths {
                    let file = target.file_name().unwrap().to_str().unwrap();
                    help = help.replace(&format!("]({file}"), &format!("]({dir}/{file}"));
                }
                help = help.replace("](../", "](");
            }
            let title = help.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or(stem);
            let aliases: Vec<&str> = if dir == "glyphs" { vec![stem] }
            else if dir == "system" {
                let mut fenced = false;
                help.lines().filter(|line| {
                    fenced ^= line.starts_with("```");
                    !fenced && line.starts_with('#')
                }).flat_map(|line| line.split('`').skip(1).step_by(2)).filter(|name| name.starts_with(['•', '$'])).collect()
            } else { Vec::new() };
            entries.push(format!("Page {{ name: {name:?}, title: {title:?}, aliases: &{aliases:?}, text: {help:?} }}"));
        }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("help.rs"), format!("const HELP: &[Page] = &[{}];", entries.join(",\n"))).unwrap();
}
