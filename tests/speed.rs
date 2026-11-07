//! The speed cases in `speed/*.bpl`, checked against their budgets for this machine's platform, which `rustymath_timing::NATIVE_COLUMN` picks: macOS or Linux. Each file has two tests. `timed_<file>` times its cases. `counts_<file>` counts the instructions and allocated bytes that each case takes for each item. `speed/README.md` describes the cases, their budgets and counts, and the data they use. `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! A timed test takes rustymath's shared timing hold on one thread and times each case in copies (`basedpl_testkit::timing::measured`), after a warm-up run that checks the case's required routes at full size. A cost over its budget is timed once more under the exclusive hold, where no build holds the lock, and fails if it's still over. Where builds hold the lock, the case is printed as `unconfirmed` and doesn't fail. The test fails on budgets only on the CPU they were set for, as `rustymath_timing::checks_budgets` decides. On any other CPU, and with `BASEDPL_COSTS` set, as in `BASEDPL_COSTS=1 cargo nextest run --cargo-profile release --test speed --no-capture`, the test calibrates: it holds the lock exclusively, writes every case's cost to stderr in the form that `basedpl.bpltests.logged_costs` reads, and retries each case over its budget after each pause in `rustymath_timing::RETRY_PAUSES`. Each line goes straight to stderr, so it appears even when the test passes. Counting copies corrects for a machine's overall speed but not for differences in caches and memory.
//!
//! A count test runs each case on one thread with compiling off (`basedpl_testkit::timing::counts`), so it checks the path that first runs and the browser build take, which timings of compiled stages hide. Counts don't vary with load, so it takes no timing hold. It fails on a count only where `checks_budgets` holds, since another CPU of the same platform can dispatch to other kernels. With `BASEDPL_COSTS` set, it writes each case's counts to stderr for `basedpl.bpltests.set_counts`. On macOS the counter counts the whole process, so counts need nextest's process for each test.
//!
//! Like `bpl` and the Python extensions, this binary registers rustymath's memory guard as its global allocator, so each case allocates as a user's process does, including the guard's request for huge pages on large blocks on Linux. `rustymath_timing::Counting` wraps the guard to count allocated bytes. `BASEDPL_SPEED_BATCH` selects rows of one file, as `bpl-tests prerelease` does, and the tests of other files then check nothing. `tests/reference.rs` checks that every case has its macOS, Linux and wasm budgets.
use basedpl::InterruptHandle;
use basedpl_testkit::{reference::{self, File}, timing};
use serde_json::Value as JsonValue;
use std::{io::Write, path::Path};

#[global_allocator]
static ALLOCATOR: rustymath_timing::Counting<rustymath::memory::Guard> = rustymath_timing::Counting(rustymath::memory::Guard::new());

/// The speed file `name` and the rows of it that `BASEDPL_SPEED_BATCH` selects, or `None` where the batch selects another file's rows.
fn load(name: &str) -> Option<(Vec<File>, Option<JsonValue>)> {
    let files = reference::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed").join(name)).unwrap();
    let batch: Option<JsonValue> = std::env::var("BASEDPL_SPEED_BATCH").ok().map(|batch| serde_json::from_str(&batch).unwrap());
    if let Some(batch) = &batch {
        if batch["file"] != name { return None; }
        let rows = timing::Selection::Rows(batch["rows"].as_array().unwrap());
        let file = &files[0];
        let named = file.cases.iter().filter(|c| timing::speed_case(c) && rows.matches(file, c))
            .map(|c| serde_json::json!([file.name, c["section"], c["code"]])).collect();
        assert_eq!(JsonValue::Array(named), batch["rows"], "speed batch fixture identities differ");
    }
    Some((files, batch))
}

fn timed(name: &str) {
    rustymath::memory::set_checks(true);
    let Some((files, batch)) = load(name) else { return };
    let calibrate = !timing::checks_budgets() || std::env::var_os("BASEDPL_COSTS").is_some();
    let only = batch.as_ref().map(|batch| timing::Selection::Rows(batch["rows"].as_array().unwrap()));
    let mut stderr = std::io::stderr();
    let over = timing::measured(&files, only, calibrate, |line| writeln!(stderr, "{line}").unwrap()).unwrap();
    if timing::checks_budgets() { assert!(over.is_empty(), "{over:#?}"); }
}

fn counts(name: &str) {
    let Some((files, batch)) = load(name) else { return };
    let only = batch.as_ref().map(|batch| timing::Selection::Rows(batch["rows"].as_array().unwrap()));
    let counted = timing::counts(&files, only, &InterruptHandle::default()).unwrap();
    let mut stderr = std::io::stderr();
    if std::env::var_os("BASEDPL_COSTS").is_some() { for c in &counted { writeln!(stderr, "{}", c.line()).unwrap(); } }
    let over: Vec<String> = counted.iter().flat_map(|c| c.over(rustymath_timing::NATIVE_COLUMN)).collect();
    for line in &over { writeln!(stderr, "{line}: over").unwrap(); }
    if timing::checks_budgets() { assert!(over.is_empty(), "{over:#?}"); }
}

macro_rules! speed_tests {
    ($($timed:ident $counts:ident $file:literal),* $(,)?) => {
        /// The speed files that have tests.
        const FILES: &[&str] = &[$($file),*];
        $(
            #[test]
            #[cfg_attr(debug_assertions, ignore = "debug assertions change the costs it times")]
            fn $timed() { timed($file) }
            #[test]
            #[cfg_attr(debug_assertions, ignore = "debug assertions change the counts")]
            fn $counts() { counts($file) }
        )*
    };
}

speed_tests!(
    timed_arrays counts_arrays "arrays.bpl", timed_calls counts_calls "calls.bpl", timed_growth counts_growth "growth.bpl", timed_images counts_images "images.bpl", timed_maps counts_maps "maps.bpl",
    timed_models counts_models "models.bpl", timed_steps counts_steps "steps.bpl", timed_structure counts_structure "structure.bpl", timed_tables counts_tables "tables.bpl",
);

#[test]
fn every_speed_file_has_tests() {
    let names: Vec<_> = reference::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap().into_iter().map(|f| f.name).collect();
    assert_eq!(names, FILES);
}
