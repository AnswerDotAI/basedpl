//! The speed cases in `speed/*.bpl`, checked against their budgets for this machine's platform, which `rustymath_timing::NATIVE_COLUMN` picks: macOS or Linux. Each file has a count test for instructions and allocated bytes per item. Files with wall-time budgets also have a timing test. `speed/README.md` describes the cases, their budgets and counts, and the data they use. `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! A timed test takes rustymath's shared timing hold on one thread and times each case in copies (`basedpl_testkit::timing::measured`), after warming its execution; annotated cases also check their required routes at full size. A cost over its budget is timed once more under the exclusive hold, where no build holds the lock, and fails if it's still over. Where builds hold the lock, the case is printed as `unconfirmed` and doesn't fail. The test fails on budgets only on the CPU they were set for, as `rustymath_timing::checks_budgets` decides. On any other CPU, and with `BASEDPL_COSTS` set, as in `BASEDPL_COSTS=1 cargo nextest run --cargo-profile release --test speed --no-capture`, the test calibrates: it holds the lock exclusively, writes every case's cost to stderr in the form that `basedpl.bpltests.logged_costs` reads, and retries each case over its budget after each pause in `rustymath_timing::RETRY_PAUSES`. Each line goes straight to stderr, so it appears even when the test passes. Counting copies corrects for a machine's overall speed but not for differences in caches and memory.
//!
//! A count test runs each case on one thread with compiling off (`basedpl_testkit::timing::counts`), so it checks the path that first runs and the browser build take, which timings of compiled stages hide. It confirms continued retention of large allocations only when a warmed measurement increases the guard's held count. Counts don't vary with load, so it takes no timing hold. It fails on a count only where `checks_budgets` holds, since another CPU of the same platform can dispatch to other kernels. With `BASEDPL_COSTS` set, it writes each case's counts to stderr for `basedpl.bpltests.set_counts`. On macOS the counter counts the whole process, so counts need nextest's process for each test.
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
    ($($file:literal $counts:ident $($timed:ident)?),* $(,)?) => {
        /// The speed files that have tests.
        const FILES: &[&str] = &[$($file),*];
        $(
            #[test]
            #[cfg_attr(debug_assertions, ignore = "debug assertions change the counts")]
            fn $counts() { counts($file) }
            $(
                #[test]
                #[cfg_attr(debug_assertions, ignore = "debug assertions change the costs it times")]
                fn $timed() { timed($file) }
            )?
        )*
    };
}

speed_tests!(
    "arrays.bpl" counts_arrays timed_arrays, "bodies.bpl" counts_bodies timed_bodies, "calls.bpl" counts_calls timed_calls,
    "comparisons_mathish.bpl" counts_comparisons_mathish, "growth.bpl" counts_growth,
    "images.bpl" counts_images timed_images, "keyed.bpl" counts_keyed timed_keyed, "maps.bpl" counts_maps timed_maps, "models.bpl" counts_models timed_models,
    "moving.bpl" counts_moving timed_moving, "steps.bpl" counts_steps,
    "structure.bpl" counts_structure timed_structure, "tables.bpl" counts_tables timed_tables,
);


#[test]
fn every_speed_file_has_tests() {
    let names: Vec<_> = reference::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap().into_iter().map(|f| f.name).collect();
    let mut registered = FILES.to_vec();
    registered.sort_unstable();
    assert_eq!(names, registered);
}
