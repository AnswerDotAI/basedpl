//! How long each function takes, compared with copying the same data. The unit is one copy, as `basedpl::timing` measures it. Moving data takes longer than simple arithmetic. A function that reads and writes each item once should therefore cost about one copy. The cases, their budgets and the data they use are in `speed.txt`. Most cases use a small `n` to keep the test fast. The `[images]` cases use an image of 36 million floats, far more than the caches hold, to measure chains whose cost is memory traffic. The `[objectives]` cases time functions that `∂` will differentiate: a small network, a Gaussian mixture, k-means and a stencil. `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! This test checks each case against its native budget for this machine's platform, which `rustymath::timing::NATIVE_COLUMN` picks from the case's macOS and Linux budgets. A native budget counts copies, set as `speed.txt`'s header describes: at first twice what a first-principles analysis says the case should cost, and at least half a copy. Counting copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to three times as many copies on one machine as on the other. Cases that compare the float widths instead budget the 32-bit time as a fraction of the 64-bit time. A case over its budget is timed again after the rest, by `timing::measured`, which `bpl --costs` also uses. Each retry writes the case's earlier and new costs straight to stderr. Unlike `eprintln!` output, which the test harness captures, these lines appear even when the test passes. The test fails listing every case still over its budget after the last retry, as its section, code, cost and budget, in the form that `tests/wasm-speed.mjs` prints and `basedpl.bpltests.logged_costs` reads. With `BASEDPL_COSTS` set, the test also writes every case's cost in that form straight to stderr, as in `BASEDPL_COSTS=1 cargo test --test speed`. CI's `Test` workflow sets it, so each push's log holds every native cost.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share. `basedpl::timing` divides each case's time by a copy timed just before it. A change in the machine's speed during the run then affects both times.
//!
//! A second test checks that every case has its macOS, Linux and wasm budgets. Both speed tests pass a case that lacks the budget they read, without checking it.
use basedpl::timing;
use std::io::Write;

#[test]
fn functions_cost_what_their_work_needs() {
    let all = std::env::var_os("BASEDPL_COSTS").is_some();
    let mut stderr = std::io::stderr();
    let failures = timing::measured(include_str!("speed.txt"), all, |line| writeln!(stderr, "{line}").unwrap()).unwrap();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_case_has_its_three_budgets() {
    let sections = timing::sections(include_str!("speed.txt")).unwrap();
    let budgets = |line: &str| line.split('\t').skip(1).map(str::parse::<f64>).collect::<Result<Vec<_>, _>>().map_or(0, |b| b.len());
    let missing: Vec<&str> = sections.iter().filter(|s| s.0 != "setup").flat_map(|s| s.2.iter().copied()).filter(|l| budgets(l) != 3).collect();
    assert!(missing.is_empty(), "each case needs a macOS, a Linux and a wasm budget: {missing:#?}");
}
