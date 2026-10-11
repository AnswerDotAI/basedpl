//! How long each function takes, compared with copying the same data. The unit is one copy, as `basedpl::timing` measures it. Moving
//! data takes longer than simple arithmetic. A function that reads and writes each item once should therefore cost about one copy.
//! The cases, their budgets and the data they use are in `speed.txt`, whose small `n` keeps the whole test to a fraction of a second.
//! `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! Each case has a budget in copies: twice the most it has cost on a Mac and on CI's Linux runner, and at least half a copy. Counting
//! copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to
//! three times as many copies on one machine as on the other. Cases that compare the float widths instead budget the 32-bit time as a
//! fraction of the 64-bit time. The test fails listing every case over its budget, with its cost.
//! To print every case's cost, set `BASEDPL_COSTS`: `BASEDPL_COSTS=1 cargo test --test speed -- --nocapture`.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share. `basedpl::timing` divides each case's time
//! by a copy timed just before it. A change in the machine's speed during the run then affects both times.
use basedpl::timing;

#[test]
fn functions_cost_what_their_work_needs() {
    let print_all = std::env::var_os("BASEDPL_COSTS").is_some();
    let mut failures = Vec::new();
    for timing::Case { section, code, budgets, cost } in timing::costs(include_str!("speed.txt"), &Default::default()).unwrap() {
        let budget: f64 = budgets[0].parse().unwrap();
        let line = if section == "widths" { format!("{code}: 32-bit takes {cost:.2} of the 64-bit time, budget {budget}") }
            else { format!("{code}: {cost:.1} copies, budget {budget}") };
        if print_all { eprintln!("{line}"); }
        if cost > budget { failures.push(line); }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
