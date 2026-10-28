//! How big the largest codegen unit of basedpl's development build is. An incremental build splits the crate into codegen units, and an edit recompiles only the units whose code changed. LLVM compiles each unit on one core, so the largest unit sets how long an edit to its code takes to rebuild. In an incremental build, rustc compiles each instance of a generic function once, in the unit of the module that defines it, or of the enclosing module for a function nested in another. `#[inline]` copies the instances into the units that call them instead. A generic function without it that receives a kernel therefore pulls every loop it reaches into one unit.
//!
//! The test reads the object files of the newest incremental build of basedpl's library, which `cargo develop` and `cargo test` make, and builds nothing itself. It fails when the largest unit is over `BUDGET`, and lists that unit's biggest functions. To list them for a passing build too, set `BASEDPL_UNITS`: `BASEDPL_UNITS=1 cargo test --test units -- --nocapture`.
//!
//! `BASEDPL_SIZES` measures what a change costs in machine code. Set it to a regex, and the test prints how many bytes the functions whose demangled names match hold, against the whole build's, with the biggest of those functions. Run it before and after the change, as in `BASEDPL_SIZES='primitive::numeric' cargo test --test units -- --nocapture`.
//!
//! `BASEDPL_SIZES_DIFF` compares two builds. Set it to two object files or binaries, `old:new`, such as copies of `bpl` from before and after a change, and the test prints each one's total and the 30 function names whose machine code changed most. Unlike `BASEDPL_SIZES`, it covers every crate in the binary, rustymath and dependencies included.
//!
//! `BASEDPL_CALLS` finds calls that LLVM can't inline. Set it to a regex, and the test lists each call from a function whose demangled name matches to a function that another codegen unit defines, in basedpl's build or in the rustymath rlib that it links. Without LTO, such a call stays a call. rustymath's DEV.md gives the cause, under "Loops and codegen". Run it as `BASEDPL_CALLS='Steps.*combined' cargo test --test units -- --nocapture`.
//!
//! `BASEDPL_RECOMPILED` shows what an edit costs to rebuild. After the edit, set it to a regex, as in `BASEDPL_RECOMPILED=structural cargo test --test units -- --nocapture`. Cargo rebuilds the library first. The test then lists the codegen units that build compiled again, biggest first, each with its biggest functions whose demangled names match. `.` matches every function. A large unit in the list usually holds generic instances whose source lines the edit moved: basedpl's own generic code, or the standard library's iterators built with its closures. Moving those lines changes every such instance, because each bounds check and panic records its line. A pattern that names the edited module shows which of its functions sit in each unit. DEV.md's "Codegen and rebuilds" gives the fix.
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::{collections::{HashMap, HashSet}, fs, path::{Path, PathBuf}, time::SystemTime};

/// The most bytes a codegen unit's object file may hold: about twice the largest unit of a healthy build on the Mac, 2.3 MB.
const BUDGET: u64 = 4_600_000;

/// The newest incremental session of basedpl's library, in the target directory that holds this test. A session holds one build's object files. Sessions with fewer object files belong to other targets of the crate.
fn newest_session() -> Option<PathBuf> {
    let incremental = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("incremental");
    fs::read_dir(incremental).into_iter().flatten().flatten().filter(|crate_dir| crate_dir.file_name().to_string_lossy().starts_with("basedpl-"))
        .flat_map(|crate_dir| fs::read_dir(crate_dir.path()).into_iter().flatten().flatten().map(|session| session.path()))
        .filter(|session| objects(session).len() > 64).max_by_key(|session| modified(session))
}

/// The object files in the directory `dir`.
fn objects(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "o")).collect()
}

/// The object files of the newest incremental build of basedpl's library.
fn newest_build() -> Vec<PathBuf> { newest_session().map_or_else(Vec::new, |session| objects(&session)) }

/// When the file at `path` was last modified.
fn modified(path: &Path) -> SystemTime { fs::metadata(path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH) }

/// The object files of the newest rustymath rlib in this test's target directory, which basedpl's library links, each with its name.
fn rustymath_units() -> Vec<(String, Vec<u8>)> {
    let deps = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let rlib = |p: &PathBuf| {
        p.file_name().is_some_and(|n| {
            let n = n.to_string_lossy();
            n.starts_with("librustymath-") && n.ends_with(".rlib")
        })
    };
    let Some(path) = fs::read_dir(deps).into_iter().flatten().flatten().map(|e| e.path()).filter(rlib).max_by_key(|p| modified(p)) else { return Vec::new() };
    let data = fs::read(path).unwrap();
    let archive = object::read::archive::ArchiveFile::parse(&*data).unwrap();
    archive.members().flatten().filter(|m| m.name().ends_with(b".o"))
        .map(|m| (format!("rustymath {}", String::from_utf8_lossy(m.name()).split('.').nth(1).unwrap_or_default()), m.data(&*data).unwrap().to_vec())).collect()
}

/// Each function in the object file at `path`, as its size in bytes and its demangled name. A function's size is the distance to the next symbol in its section, because Mach-O records no sizes.
fn functions(path: &Path) -> Vec<(u64, String)> {
    let data = fs::read(path).unwrap();
    let file = object::File::parse(&*data).unwrap();
    let mut symbols: Vec<_> = file.symbols().filter(|s| s.kind() == SymbolKind::Text && s.is_definition())
        .filter_map(|s| Some((s.section_index()?.0, s.address(), s.size(), s.name().ok()?))).collect();
    symbols.sort_by_key(|&(section, address, ..)| (section, address));
    let end = |section| file.section_by_index(object::SectionIndex(section)).map_or(0, |s| s.address() + s.size());
    symbols.iter().enumerate().map(|(i, &(section, address, size, name))| {
        let next = symbols.get(i + 1).filter(|n| n.0 == section).map_or_else(|| end(section), |n| n.1);
        (if size > 0 { size } else { next - address }, demangled(name))
    }).collect()
}

/// The `n` biggest of `functions`, one per line with its size in bytes.
fn biggest(mut functions: Vec<(u64, String)>, n: usize) -> String {
    functions.sort_by(|a, b| b.0.cmp(&a.0));
    functions.iter().take(n).map(|(size, name)| format!("{size:>8} {}\n", shortened(name))).collect()
}

#[test]
fn codegen_units_stay_small() {
    let objects = newest_build();
    let Some((size, largest)) = objects.iter().map(|p| (fs::metadata(p).unwrap().len(), p)).max() else {
        return eprintln!("no incremental build of basedpl's library under target/");
    };
    let listing =
        || format!("the largest codegen unit holds {size} bytes, against a budget of {BUDGET}. Its biggest functions:\n{}", biggest(functions(largest), 20));
    if std::env::var_os("BASEDPL_UNITS").is_some() { println!("{}", listing()); }
    assert!(size <= BUDGET, "{}", listing());
}

/// The regex in the environment variable `var`, or `None` where it isn't set.
fn env_pattern(var: &str) -> Option<fancy_regex::Regex> {
    std::env::var_os(var).map(|p| fancy_regex::Regex::new(&p.to_string_lossy()).unwrap_or_else(|e| panic!("`{var}` must be a regex: {e}")))
}

/// The machine code of the newest build's functions whose demangled names match the regex in `BASEDPL_SIZES`. Without it, this test does nothing.
#[test]
fn machine_code_by_name() {
    let Some(pattern) = env_pattern("BASEDPL_SIZES") else { return };
    let all: Vec<_> = newest_build().iter().flat_map(|p| functions(p)).collect();
    let matched: Vec<_> = all.iter().filter(|(_, name)| pattern.is_match(name).unwrap()).cloned().collect();
    let (count, bytes) = (matched.len(), matched.iter().map(|(size, _)| size).sum::<u64>());
    let total: u64 = all.iter().map(|(size, _)| size).sum();
    println!("{count} functions matching `{pattern}` hold {bytes} of the build's {total} bytes of machine code. The biggest:\n{}", biggest(matched, 20));
}

/// With `BASEDPL_SIZES_DIFF` set to two object files or binaries, `old:new`, the total machine code of each and the 30 generic functions whose bytes changed most between them, with each function's instances counted together. With `BASEDPL_SIZES` set too, only the functions whose full demangled names match its regex count. Without `BASEDPL_SIZES_DIFF`, this test does nothing.
#[test]
fn machine_code_growth() {
    let Some(paths) = std::env::var_os("BASEDPL_SIZES_DIFF") else { return };
    let paths = paths.to_string_lossy().into_owned();
    let (old, new) = paths.split_once(':').expect("`BASEDPL_SIZES_DIFF` must be `old:new`");
    let only = env_pattern("BASEDPL_SIZES");
    let by_name = |path: &str| {
        let kept = functions(Path::new(path)).into_iter().filter(|(_, name)| only.as_ref().is_none_or(|p| p.is_match(name).unwrap()));
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

/// `name` demangled, without its hash.
fn demangled(name: &str) -> String { format!("{:#}", rustc_demangle::demangle(name)) }

/// The first 240 characters of `name`, which tell functions apart on one line.
fn shortened(name: &str) -> String { name.chars().take(240).collect() }

/// Each function that `file` defines, as its section's index, its address and its mangled name.
fn defined<'a>(file: &object::File<'a>) -> Vec<(usize, u64, &'a str)> {
    file.symbols().filter(|s| s.kind() == SymbolKind::Text && s.is_definition()).filter_map(|s| Some((s.section_index()?.0, s.address(), s.name().ok()?)))
        .collect()
}

/// Each call from a function in one of `units` to a function that another of `units` defines, as the caller, the caller's unit, the callee and the callee's unit. A call into code that none of `units` defines, such as the standard library's own build, isn't listed.
fn calls_between(units: &[(String, Vec<u8>)]) -> Vec<(String, String, String, String)> {
    let files: Vec<_> = units.iter().map(|(unit, data)| (unit.as_str(), object::File::parse(&**data).unwrap())).collect();
    let home: HashMap<&str, &str> = files.iter().flat_map(|(unit, file)| defined(file).into_iter().map(move |(.., name)| (name, *unit))).collect();
    let mut calls = Vec::new();
    for (from, file) in &files {
        let mut starts = defined(file);
        starts.sort();
        for section in file.sections() {
            let index = section.index().0;
            for (offset, relocation) in section.relocations() {
                let object::RelocationTarget::Symbol(target) = relocation.target() else { continue };
                let Some(callee) = file.symbol_by_index(target).ok().filter(|t| t.is_undefined()).and_then(|t| t.name().ok()) else { continue };
                let Some(unit) = home.get(callee) else { continue };
                let at = section.address() + offset;
                let Some(&(s, _, caller)) = starts.partition_point(|&(s, a, _)| (s, a) <= (index, at)).checked_sub(1).map(|i| &starts[i]) else { continue };
                if s == index { calls.push((demangled(caller), from.to_string(), demangled(callee), unit.to_string())); }
            }
        }
    }
    calls
}

/// With `BASEDPL_CALLS` set to a regex, each call from a function whose demangled name matches to a function that another codegen unit defines, in the newest builds of basedpl's library and of rustymath. Without it, this test does nothing.
#[test]
fn calls_between_units() {
    let Some(pattern) = env_pattern("BASEDPL_CALLS") else { return };
    let mut units: Vec<_> =
        newest_build().into_iter().map(|p| (format!("basedpl {}", p.file_stem().unwrap().to_string_lossy()), fs::read(&p).unwrap())).collect();
    units.extend(rustymath_units());
    let mut calls: Vec<_> = calls_between(&units).into_iter().filter(|(caller, ..)| pattern.is_match(caller).unwrap()).collect();
    calls.sort();
    calls.dedup();
    let rows: Vec<_> =
        calls.iter().map(|(caller, from, callee, to)| format!("{}\n    in {from}, calls into {to}:\n    {}", shortened(caller), shortened(callee))).collect();
    println!("{} calls from functions matching `{pattern}` go to other codegen units:\n{}", calls.len(), rows.join("\n"));
}

/// With `BASEDPL_RECOMPILED` set to a regex, the codegen units that the newest build of basedpl's library compiled again, biggest first, each with its five biggest functions whose demangled names match. A build hard-links each unit it reuses from the previous build, so only the units it compiled again have files written after its session began. Without it, this test does nothing.
#[test]
fn recompiled_units() {
    let Some(pattern) = env_pattern("BASEDPL_RECOMPILED") else { return };
    let Some(session) = newest_session() else { return eprintln!("no incremental build of basedpl's library under target/") };
    let began = fs::metadata(&session).and_then(|m| m.created()).unwrap();
    let all = objects(&session);
    let mut fresh: Vec<_> = all.iter().filter(|p| modified(p) >= began).map(|p| (fs::metadata(p).unwrap().len(), p)).collect();
    fresh.sort_by(|a, b| b.0.cmp(&a.0));
    let bytes: u64 = fresh.iter().map(|(size, _)| size).sum();
    let seconds = modified(&session).duration_since(began).map_or(0.0, |d| d.as_secs_f64());
    println!("The newest build compiled {} of its {} codegen units again, {bytes} bytes, over {seconds:.1} s:", fresh.len(), all.len());
    for (size, path) in fresh {
        let matched: Vec<_> = functions(path).into_iter().filter(|(_, name)| pattern.is_match(name).unwrap()).collect();
        println!("{size:>9} {}, with {} functions matching `{pattern}`:\n{}", path.file_stem().unwrap().to_string_lossy(), matched.len(), biggest(matched, 5));
    }
}

/// `name` without its generic argument lists, `::<…>`, so that the instances of one generic function share it.
fn generic(name: &str) -> String {
    let (mut out, mut depth, mut previous, mut rest) = (String::new(), 0usize, ' ', name);
    while let Some(c) = rest.chars().next() {
        if depth == 0 && rest.starts_with("::<") { (depth, previous, rest) = (1, '<', &rest[3..]); continue; }
        match c { '<' if depth > 0 => depth += 1, '>' if depth > 0 && previous != '-' => depth -= 1, _ if depth == 0 => out.push(c), _ => {} }
        previous = c;
        rest = &rest[c.len_utf8()..];
    }
    out
}
