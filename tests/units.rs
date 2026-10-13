//! How big the largest codegen unit of basedpl's development build is. An incremental build splits the crate into codegen units, and
//! an edit recompiles only the units whose code changed. LLVM compiles each unit on one core, so the largest unit sets how long an edit
//! to its code takes to rebuild. In an incremental build, rustc compiles each instance of a generic function once, in the unit of the
//! module that defines it, or of the enclosing module for a function nested in another. `#[inline]` copies the instances into the units
//! that call them instead. A generic function without it that receives a kernel therefore pulls every loop it reaches into one unit.
//!
//! The test reads the object files of the newest incremental build of basedpl's library, which `cargo develop` and `cargo test` make,
//! and builds nothing itself. It fails when the largest unit is over `BUDGET`, and lists that unit's biggest functions. To list them
//! for a passing build too, set `BASEDPL_UNITS`: `BASEDPL_UNITS=1 cargo test --test units -- --nocapture`.
//!
//! `BASEDPL_SIZES` measures what a change costs in machine code. Set it to a regex, and the test prints how many bytes the functions
//! whose demangled names match hold, against the whole build's, with the biggest of those functions. Run it before and after the
//! change, as in `BASEDPL_SIZES='primitive::numeric' cargo test --test units -- --nocapture`.
//!
//! `BASEDPL_SIZES_DIFF` compares two builds. Set it to two object files or binaries, `old:new`, such as copies of `bpl` from before and
//! after a change, and the test prints each one's total and the 30 function names whose machine code changed most. Unlike
//! `BASEDPL_SIZES`, it covers every crate in the binary, rustymath and dependencies included.
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::{
    collections::{HashMap, HashSet},
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

/// Each function in the object file at `path`, as its size in bytes and its demangled name. A function's size is the distance to the
/// next symbol in its section, because Mach-O records no sizes.
fn functions(path: &Path) -> Vec<(u64, String)> {
    let data = fs::read(path).unwrap();
    let file = object::File::parse(&*data).unwrap();
    let mut symbols: Vec<_> = file
        .symbols()
        .filter(|s| s.kind() == SymbolKind::Text && s.is_definition())
        .filter_map(|s| Some((s.section_index()?.0, s.address(), s.size(), s.name().ok()?)))
        .collect();
    symbols.sort_by_key(|&(section, address, ..)| (section, address));
    let end = |section| file.section_by_index(object::SectionIndex(section)).map_or(0, |s| s.address() + s.size());
    symbols
        .iter()
        .enumerate()
        .map(|(i, &(section, address, size, name))| {
            let next = symbols.get(i + 1).filter(|n| n.0 == section).map_or_else(|| end(section), |n| n.1);
            (if size > 0 { size } else { next - address }, format!("{:#}", rustc_demangle::demangle(name)))
        })
        .collect()
}

/// The 20 biggest of `functions`, one per line with its size in bytes.
fn biggest(mut functions: Vec<(u64, String)>) -> String {
    functions.sort_by(|a, b| b.0.cmp(&a.0));
    functions.iter().take(20).map(|(size, name)| format!("{size:>8} {}\n", name.chars().take(240).collect::<String>())).collect()
}

#[test]
fn codegen_units_stay_small() {
    let objects = newest_build();
    let Some((size, largest)) = objects.iter().map(|p| (fs::metadata(p).unwrap().len(), p)).max() else {
        return eprintln!("no incremental build of basedpl's library under target/");
    };
    let listing = || format!("the largest codegen unit holds {size} bytes, against a budget of {BUDGET}. Its biggest functions:\n{}", biggest(functions(largest)));
    if std::env::var_os("BASEDPL_UNITS").is_some() { println!("{}", listing()); }
    assert!(size <= BUDGET, "{}", listing());
}

/// The regex in `BASEDPL_SIZES`, or `None` where it isn't set.
fn sizes_pattern() -> Option<regex::Regex> {
    std::env::var_os("BASEDPL_SIZES").map(|p| regex::Regex::new(&p.to_string_lossy()).expect("`BASEDPL_SIZES` must be a regex"))
}

/// The machine code of the newest build's functions whose demangled names match the regex in `BASEDPL_SIZES`. Without it, this test
/// does nothing.
#[test]
fn machine_code_by_name() {
    let Some(pattern) = sizes_pattern() else { return };
    let all: Vec<_> = newest_build().iter().flat_map(|p| functions(p)).collect();
    let matched: Vec<_> = all.iter().filter(|(_, name)| pattern.is_match(name)).cloned().collect();
    let (count, bytes) = (matched.len(), matched.iter().map(|(size, _)| size).sum::<u64>());
    let total: u64 = all.iter().map(|(size, _)| size).sum();
    println!("{count} functions matching `{pattern}` hold {bytes} of the build's {total} bytes of machine code. The biggest:\n{}", biggest(matched));
}

/// With `BASEDPL_SIZES_DIFF` set to two object files or binaries, `old:new`, the total machine code of each and the 30 generic functions
/// whose bytes changed most between them, with each function's instances counted together. With `BASEDPL_SIZES` set too, only the
/// functions whose full demangled names match its regex count. Without `BASEDPL_SIZES_DIFF`, this test does nothing.
#[test]
fn machine_code_growth() {
    let Some(paths) = std::env::var_os("BASEDPL_SIZES_DIFF") else { return };
    let paths = paths.to_string_lossy().into_owned();
    let (old, new) = paths.split_once(':').expect("`BASEDPL_SIZES_DIFF` must be `old:new`");
    let only = sizes_pattern();
    let by_name = |path: &str| {
        let kept = functions(Path::new(path)).into_iter().filter(|(_, name)| only.as_ref().is_none_or(|p| p.is_match(name)));
        kept.fold(HashMap::<String, i64>::new(), |mut sizes, (size, name)| {
            *sizes.entry(generic(&name)).or_default() += size as i64;
            sizes
        })
    };
    let (old, new) = (by_name(old), by_name(new));
    let names: HashSet<&String> = old.keys().chain(new.keys()).collect();
    let mut changes: Vec<_> = names.into_iter().map(|n| (new.get(n).unwrap_or(&0) - old.get(n).unwrap_or(&0), n)).collect();
    changes.sort_by_key(|&(change, _)| -change.abs());
    let total = |sizes: &HashMap<String, i64>| sizes.values().sum::<i64>();
    let rows: Vec<_> = changes.iter().take(30).map(|(change, name)| format!("{change:>+9} {name}")).collect();
    println!("{} bytes before and {} after. The biggest changes:\n{}", total(&old), total(&new), rows.join("\n"));
}

/// `name` without its generic argument lists, `::<…>`, so that the instances of one generic function share it.
fn generic(name: &str) -> String {
    let (mut out, mut depth, mut previous, mut rest) = (String::new(), 0usize, ' ', name);
    while let Some(c) = rest.chars().next() {
        if depth == 0 && rest.starts_with("::<") {
            (depth, previous, rest) = (1, '<', &rest[3..]);
            continue;
        }
        match c {
            '<' if depth > 0 => depth += 1,
            '>' if depth > 0 && previous != '-' => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
        previous = c;
        rest = &rest[c.len_utf8()..];
    }
    out
}
