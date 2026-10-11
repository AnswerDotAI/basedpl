//! How big the largest codegen unit of basedpl's development build is. An incremental build splits the crate into codegen units, and
//! an edit recompiles only the units whose code changed. LLVM compiles each unit on one core, so the largest unit sets how long an edit
//! to its code takes to rebuild. In an incremental build, rustc compiles each instance of a generic function once, in the unit of the
//! module that defines it, or of the enclosing module for a function nested in another. `#[inline]` copies the instances into the units
//! that call them instead. A generic function without it that receives a kernel therefore pulls every loop it reaches into one unit.
//!
//! The test reads the object files of the newest incremental build of basedpl's library, which `cargo develop` and `cargo test` make,
//! and builds nothing itself. It fails when the largest unit is over `BUDGET`, and lists that unit's biggest functions. To list them
//! for a passing build too, set `BASEDPL_UNITS`: `BASEDPL_UNITS=1 cargo test --test units -- --nocapture`.
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// The most bytes a codegen unit's object file may hold: about twice the largest unit of a healthy build on the Mac, 2.3 MB.
const BUDGET: u64 = 4_600_000;

/// The object files of the newest incremental build of basedpl's library, in the target directory that holds this test. Builds with
/// fewer object files belong to other targets of the crate.
fn newest_build() -> Vec<PathBuf> {
    let incremental = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("incremental");
    let objects = |dir: &Path| -> Vec<PathBuf> {
        fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "o")).collect()
    };
    let modified = |p: &PathBuf| fs::metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
    fs::read_dir(incremental)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|crate_dir| crate_dir.file_name().to_string_lossy().starts_with("basedpl-"))
        .flat_map(|crate_dir| fs::read_dir(crate_dir.path()).into_iter().flatten().flatten().map(|session| session.path()))
        .map(|session| (objects(&session), session))
        .filter(|(objects, _)| objects.len() > 64)
        .max_by_key(|(_, session)| modified(session))
        .map(|(objects, _)| objects)
        .unwrap_or_default()
}

/// The biggest functions in the object file at `path`, one per line with its size in bytes. A function's size is the distance to the
/// next symbol in its section, because Mach-O records no sizes.
fn biggest(path: &Path) -> String {
    let data = fs::read(path).unwrap();
    let file = object::File::parse(&*data).unwrap();
    let mut symbols: Vec<_> = file
        .symbols()
        .filter(|s| s.kind() == SymbolKind::Text && s.is_definition())
        .filter_map(|s| Some((s.section_index()?.0, s.address(), s.size(), s.name().ok()?)))
        .collect();
    symbols.sort_by_key(|&(section, address, ..)| (section, address));
    let end = |section| file.section_by_index(object::SectionIndex(section)).map_or(0, |s| s.address() + s.size());
    let mut sized: Vec<(u64, String)> = symbols
        .iter()
        .enumerate()
        .map(|(i, &(section, address, size, name))| {
            let next = symbols.get(i + 1).filter(|n| n.0 == section).map_or_else(|| end(section), |n| n.1);
            (if size > 0 { size } else { next - address }, format!("{:#}", rustc_demangle::demangle(name)).chars().take(240).collect())
        })
        .collect();
    sized.sort_by(|a, b| b.0.cmp(&a.0));
    sized.iter().take(20).map(|(size, name)| format!("{size:>8} {name}\n")).collect()
}

#[test]
fn codegen_units_stay_small() {
    let objects = newest_build();
    let Some((size, largest)) = objects.iter().map(|p| (fs::metadata(p).unwrap().len(), p)).max() else {
        return eprintln!("no incremental build of basedpl's library under target/");
    };
    let listing = || format!("the largest codegen unit holds {size} bytes, against a budget of {BUDGET}. Its biggest functions:\n{}", biggest(largest));
    if std::env::var_os("BASEDPL_UNITS").is_some() { println!("{}", listing()); }
    assert!(size <= BUDGET, "{}", listing());
}
