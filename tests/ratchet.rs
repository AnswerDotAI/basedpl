//! Counts code in `src/` that infers a fact from incidental state instead of taking it from where the fact is established. Fails when a count differs from its budget. A rise is new inference. A fall needs the budget lowered in the same change. Comments don't count. Patterns are regexes, so qualified, imported and spaced spellings count too.

use fancy_regex::Regex;
use std::path::Path;

/// A pattern: its name for messages, its regex, the files that are its fact's home and aren't counted, its count elsewhere, and what to write instead.
struct Pattern { name: &'static str, regex: &'static str, homes: &'static [&'static str], budget: usize, instead: &'static str }

const PATTERNS: &[Pattern] = &[
    Pattern { name: "the `Pending` variant or the `.pending()` accessor", regex: r"\bPending\s*\(|\.pending\(\)",
        homes: &["graph.rs", "program.rs", "program/", "fusion.rs", "array.rs", "array/"], budget: 35,
        instead: "host code takes a stored value from readback rather than testing whether a value is pending" },
    Pattern { name: "an `Error` constructor passed to `ok_or_else` as a function item", regex: r"\bok_or_else\(\s*(?:\w+::)*Error::\w+\s*\)", homes: &[], budget: 0,
        instead: "call `Error::internal()` directly, or `or_internal()` on an `Option`: a function item loses the call site" },
    Pattern { name: "an associated `strong_count` call", regex: r"::strong_count\s*\(", homes: &[], budget: 0,
        instead: "the binding owner marks a value held: reference counts measure sharing, not holders (`Weak`'s `.strong_count()` liveness check isn't counted)" },
    Pattern { name: "`Numeric::current()`", regex: r"\bNumeric::current\s*\(", homes: &["number.rs"], budget: 6, instead: "read numeric settings from the call's origin" },
    Pattern { name: "`Tracing::planning()`", regex: r"\bTracing::planning\s*\(", homes: &["program/trace.rs"], budget: 0, instead: "propagate the stop that `Tracing::stop` returns" },
];

/// Adds each `.rs` file under `dir`, as its path relative to `src` and its code with line comments removed.
fn sources(src: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() { sources(src, &path, files); continue; }
        if path.extension().is_none_or(|ext| ext != "rs") { continue; }
        let code = std::fs::read_to_string(&path).unwrap().lines().map(|line| line.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
        files.push((path.strip_prefix(src).unwrap().to_string_lossy().replace('\\', "/"), code));
    }
}

#[test]
fn inference_pattern_counts_match_their_budgets() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &src, &mut files);
    let failures: Vec<String> = PATTERNS.iter().filter_map(|pattern| {
        let regex = Regex::new(pattern.regex).unwrap();
        let found: Vec<(&str, usize)> = files.iter()
            .filter(|(path, _)| !pattern.homes.iter().any(|&home| path == home || home.ends_with('/') && path.starts_with(home)))
            .map(|(path, code)| (path.as_str(), regex.find_iter(code).filter(Result::is_ok).count())).filter(|&(_, n)| n > 0).collect();
        let total: usize = found.iter().map(|&(_, n)| n).sum();
        let by_file = || found.iter().map(|(path, n)| format!("{path} {n}")).collect::<Vec<_>>().join(", ");
        match total.cmp(&pattern.budget) {
            std::cmp::Ordering::Greater => Some(format!("{} occurs {total} times, over its budget of {}: {}. By file: {}", pattern.name, pattern.budget, pattern.instead, by_file())),
            std::cmp::Ordering::Less => Some(format!("{} occurs {total} times, under its budget of {}: lower the budget to {total} in this change.", pattern.name, pattern.budget)),
            std::cmp::Ordering::Equal => None,
        }
    }).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
