//! `cargo wasm` builds the browser module and writes the npm package `basedpl` into `wasm/pkg/`. The package holds the module, its
//! JavaScript glue, the language bar (`lb.js`, `input.js` and `layout.json`), the TextMate grammar, `package.json` and `README.md`.
//! It also copies the module, the glue, the language bar and the grammar into `nbs/playground/`, where the docs playground loads them
//! on `localhost`. By default it builds with the incremental `release` profile.
//! `cargo wasm --profile wasm` builds the smaller module, which is the one published to npm.
//!
//! The glue comes from `wasm-bindgen-cli-support`, wasm-bindgen's generator as a library. It and the `wasm-bindgen` crate each pin an
//! exact version of `wasm-bindgen-shared`, and the workspace lock holds one version of that crate. The generator therefore always
//! matches the crate.
//!
//! `package.json` takes its version from the workspace. Its name, description, licence and repository come from `pyproject.toml`'s
//! `[project]` table, which PyPI also reads. The package's `README.md` is a copy of `wasm/README.md`.
use std::{env, fs, path::Path, process::Command};

const TARGET: &str = "wasm32-unknown-unknown";
/// The file stem that `basedpl-wasm` builds, which wasm-bindgen also gives the glue.
const MODULE: &str = "basedpl_wasm";

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let profile = match args.as_slice() {
        [] => "release",
        [flag, name] if flag == "--profile" => name,
        _ => anyhow::bail!("usage: cargo wasm [--profile NAME]"),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask is a workspace member");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let built = Command::new(cargo).current_dir(root).args(["build", "-p", "basedpl-wasm", "--target", TARGET, "--profile", profile]).status()?;
    anyhow::ensure!(built.success(), "the wasm build failed");
    let targets = env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), Into::into);
    let module = targets.join(TARGET).join(profile).join(format!("{MODULE}.wasm"));
    let pkg = root.join("wasm/pkg");
    wasm_bindgen_cli_support::Bindgen::new().input_path(module).web(true)?.typescript(false).omit_default_module_path(false).generate(&pkg)?;
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
    let assets = ["lb.js", "input.js", "layout.json", "bpl.tmLanguage.json"];
    for file in assets { fs::copy(root.join("python/basedpl").join(file), pkg.join(file))?; }
    for file in [format!("{MODULE}.js").as_str(), format!("{MODULE}_bg.wasm").as_str()].into_iter().chain(assets) {
        fs::copy(pkg.join(file), root.join("nbs/playground").join(file))?;
    }
    Ok(())
}
