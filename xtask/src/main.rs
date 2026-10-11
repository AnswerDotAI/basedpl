//! `cargo wasm` builds the browser module and writes the npm package `basedpl` into `wasm/pkg/`. The package holds the module, its
//! JavaScript glue, the language bar (`lb.js`, `input.js` and `layout.json`), the TextMate grammar, `canvas.js`, which draws `•canvas`
//! outputs, `package.json` and `README.md`. It also copies everything but `package.json` and `README.md` into `nbs/playground/`, where
//! the docs playground loads them on `localhost`. It builds with the incremental `release` profile.
//! `cargo wasm --strip` also drops the module's name section, which holds every function's name. CI publishes that module to npm.
//! `cargo wasm --out DIR` writes the package into `DIR` instead and leaves `nbs/playground/` alone. Builds that run at the same
//! time then don't overwrite each other's modules. `BASEDPL_WASM=DIR` points the Node tests at that directory.
//!
//! The glue comes from `wasm-bindgen-cli-support`, wasm-bindgen's generator as a library. It and the `wasm-bindgen` crate each pin an
//! exact version of `wasm-bindgen-shared`, and the workspace lock holds one version of that crate. The generator therefore always
//! matches the crate.
//!
//! `package.json` takes its version from the workspace. Its name, description, licence and repository come from `pyproject.toml`'s
//! `[project]` table, which PyPI also reads. The package's `README.md` is a copy of `wasm/README.md`.
use std::{env, fs, path::{Path, PathBuf}, process::Command};

const TARGET: &str = "wasm32-unknown-unknown";
/// The file stem that `basedpl-wasm` builds, which wasm-bindgen also gives the glue.
const MODULE: &str = "basedpl_wasm";

fn main() -> anyhow::Result<()> {
    let usage = || anyhow::anyhow!("usage: cargo wasm [--strip] [--out DIR]");
    let (mut strip, mut out) = (false, None);
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--strip" => strip = true,
            "--out" => out = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            _ => return Err(usage()),
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask is a workspace member");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let built = Command::new(cargo).current_dir(root).args(["build", "-p", "basedpl-wasm", "--target", TARGET, "--release"]).status()?;
    anyhow::ensure!(built.success(), "the wasm build failed");
    let targets = env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), Into::into);
    let module = targets.join(TARGET).join("release").join(format!("{MODULE}.wasm"));
    let pkg = out.clone().unwrap_or_else(|| root.join("wasm/pkg"));
    wasm_bindgen_cli_support::Bindgen::new().input_path(module).web(true)?.typescript(false).omit_default_module_path(false).remove_name_section(strip).generate(&pkg)?;
    let pyproject: toml::Table = fs::read_to_string(root.join("pyproject.toml"))?.parse()?;
    let project = &pyproject["project"];
    let repository = project["urls"]["Repository"].as_str().expect("pyproject.toml's Repository is a URL");
    let package = serde_json::json!({
        "name": project["name"],
        "version": env!("CARGO_PKG_VERSION"),
        "description": project["description"],
        "license": project["license"],
        "repository": { "type": "git", "url": format!("git+{repository}.git") },
        "type": "module",
        "main": format!("{MODULE}.js"),
    });
    fs::write(pkg.join("package.json"), serde_json::to_string_pretty(&package)?)?;
    fs::copy(root.join("wasm/README.md"), pkg.join("README.md"))?;
    let assets = ["python/basedpl/lb.js", "python/basedpl/input.js", "python/basedpl/layout.json", "python/basedpl/bpl.tmLanguage.json", "src/canvas.js"]
        .map(|a| root.join(a));
    for asset in &assets { fs::copy(asset, pkg.join(asset.file_name().expect("an asset is a file")))?; }
    if out.is_none() {
        let module = [format!("{MODULE}.js"), format!("{MODULE}_bg.wasm")].map(|m| pkg.join(m));
        for file in module.iter().chain(&assets) { fs::copy(file, root.join("nbs/playground").join(file.file_name().expect("a file")))?; }
    }
    Ok(())
}
