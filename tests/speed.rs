//! How long each function takes, compared with copying the same data. The unit is one copy, as `basedpl::timing` defines it. Moving data takes longer than simple arithmetic. A function that reads and writes each item once should therefore cost about one copy. The cases, their budgets, the data they use and what each section measures are in `speed/*.bpl`. `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! This test checks each case against its native budget for this machine's platform, which `rustymath::timing::NATIVE_COLUMN` picks from the case's macOS and Linux budgets. A machine with neither, such as Linux on Arm, reports costs with `budget -` and checks none. The test fails on budgets only on the CPU they were set for, as `rustymath::timing::checks_budgets` decides. On any other CPU, it writes every case's cost, then each case still over its budget after the retries, marked `over`, and fails on none. A native budget counts copies, set as `speed/README.md` describes: at first twice what a first-principles analysis says the case should cost, and at least half a copy. Counting copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to three times as many copies on one machine as on the other. Cases that compare the float widths instead budget the 32-bit time as a fraction of the 64-bit time. A case over its budget is timed again after the rest, by `timing::measured`, which `bpl --costs` also uses. Each retry writes the case's earlier and new costs straight to stderr. Unlike `eprintln!` output, which the test harness captures, these lines appear even when the test passes. The test fails listing every case still over its budget after the last retry, as its section, code, cost and budget, in the form that `tests/wasm-speed.mjs` prints and `basedpl.bpltests.logged_costs` reads. With `BASEDPL_COSTS` set, the test also writes every case's cost in that form straight to stderr, as in `BASEDPL_COSTS=1 cargo test --test speed`. CI's `Test` workflow sets it, so each push's log holds every native cost.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share.
//!
//! A second test checks that every case has its macOS, Linux and wasm budgets. Both speed tests pass a case that lacks the budget they read, without checking it.
use basedpl::timing;
use std::io::Write;

#[test]
#[cfg_attr(debug_assertions, ignore = "debug assertions change the costs it times")]
fn functions_cost_what_their_work_needs() {
    let checks = timing::checks_budgets();
    let all = !checks || std::env::var_os("BASEDPL_COSTS").is_some();
    let mut stderr = std::io::stderr();
    let files = basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap();
    let over = timing::measured(&files, all, |line| writeln!(stderr, "{line}").unwrap()).unwrap();
    if checks { assert!(over.is_empty(), "{over:#?}"); }
}

#[test]
fn every_case_has_its_three_budgets() {
    let files = basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap();
    let missing: Vec<_> = files.iter().flat_map(|f| &f.cases).filter(|c| c["budgets"].as_array().is_none_or(|b| b.len() != 3)).collect();
    assert!(missing.is_empty(), "each case needs a macOS, a Linux and a wasm budget: {missing:#?}");
}
