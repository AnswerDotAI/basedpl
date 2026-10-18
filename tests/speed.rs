//! How long each function takes, compared with copying the same data. The unit is one copy, as `basedpl::timing` measures it. Moving data takes longer than simple arithmetic. A function that reads and writes each item once should therefore cost about one copy. The cases, their budgets and the data they use are in `speed.txt`. Most cases use a small `n` to keep the test fast. The `[images]` cases use an image of 36 million floats, far more than the caches hold, to measure chains whose cost is memory traffic. The `[objectives]` cases time functions that `∂` will differentiate: a small network, a Gaussian mixture, k-means and a stencil. `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! Each case has a budget in copies: twice the most it has cost on a Mac and on CI's Linux runner, and at least half a copy. Counting copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to three times as many copies on one machine as on the other. Cases that compare the float widths instead budget the 32-bit time as a fraction of the 64-bit time. A case over its budget is timed again after the rest, by `timing::retried`. Each retry writes the case's earlier and new costs straight to stderr. Unlike `eprintln!` output, which the test harness captures, these lines appear even when the test passes. The test fails listing every case still over its budget after the last retry, as its section, code, cost and budget, in the form that `tests/wasm-speed.mjs` prints and `basedpl.bpltests.logged_costs` reads. With `BASEDPL_COSTS` set, the test also writes every case's cost in that form straight to stderr, as in `BASEDPL_COSTS=1 cargo test --test speed`. CI's `Test` workflow sets it, so each push's log holds every native cost.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share. `basedpl::timing` divides each case's time by a copy timed just before it. A change in the machine's speed during the run then affects both times.
use basedpl::timing;
use std::io::Write;

#[test]
fn functions_cost_what_their_work_needs() {
    let print_all = std::env::var_os("BASEDPL_COSTS").is_some();
    let speed = include_str!("speed.txt");
    let line = |c: &timing::Case| format!("{} {}: {:.2}, budget {}", c.section, c.code, c.cost, c.budgets[0]);
    let over = |c: &timing::Case| c.cost > c.budgets[0].parse::<f64>().unwrap();
    let mut failures = timing::costs(speed, None, &Default::default()).unwrap();
    let mut stderr = std::io::stderr();
    if print_all { for c in &failures { writeln!(stderr, "{}", line(c)).unwrap(); } }
    failures.retain(over);
    let failures = rustymath::timing::retried(failures, |failed, pause| {
        for c in &failed { writeln!(stderr, "{}: over, timed again after {pause} s", line(c)).unwrap(); }
        let codes: Vec<&str> = failed.iter().map(|c| c.code).collect();
        let mut retimed = timing::costs(speed, Some(&codes), &Default::default()).unwrap();
        retimed.retain(|c| failed.iter().any(|f| (f.section, f.code) == (c.section, c.code)));
        for c in &retimed { writeln!(stderr, "{}", line(c)).unwrap(); }
        retimed.into_iter().filter(over).collect()
    });
    let failures: Vec<_> = failures.iter().map(line).collect();
    assert!(failures.is_empty(), "{failures:#?}");
}
