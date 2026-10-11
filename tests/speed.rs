//! How long each function takes, compared with copying the same data. The unit is one copy: allocating a vector of `n` 8-byte
//! numbers and copying `n` numbers into it. Moving data takes longer than simple arithmetic. A function that reads and writes each
//! item once should therefore cost about one copy.
//! The cases, their budgets and the data they use are in `speed.txt`, whose small `n` keeps the whole test to a fraction of a second.
//! `tests/wasm-speed.mjs` times the same cases in the browser build, against their wasm budgets.
//!
//! Each case has a budget in copies: twice the most it has cost on a Mac and on CI's Linux runner, and at least half a copy. Counting
//! copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to
//! three times as many copies on one machine as on the other. Cases that compare the float widths instead budget the 32-bit time as a
//! fraction of the 64-bit time. The test fails listing every case over its budget, with its cost.
//! To print every case's cost, set `BASEDPL_COSTS`: `BASEDPL_COSTS=1 cargo test --test speed -- --nocapture`.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share. Each time is the fastest of several runs. The test divides each case's time by a copy timed just before it. A change in the machine's speed during the run then affects both times.
use basedpl::{EvalOptions, Session};
use std::{
    collections::HashMap,
    hint::black_box,
    time::{Duration, Instant},
};

const RUNS: usize = 5;

/// The fastest of `runs` calls of `f`, after one call to warm up.
fn fastest(runs: usize, mut f: impl FnMut()) -> Duration {
    f();
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .min()
        .unwrap()
}

/// Each section of `speed.txt`: its name, the prefs its session starts with, and its lines.
fn sections() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    let mut sections = Vec::new();
    for line in include_str!("speed.txt").lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let header = line.strip_prefix('[').and_then(|l| l.split_once(']'));
        match header.filter(|(name, prefs)| !name.contains(' ') && (prefs.is_empty() || prefs.starts_with(' '))) {
            Some((name, prefs)) => sections.push((name, prefs.trim(), Vec::new())),
            None => sections.last_mut().unwrap().2.push(line),
        }
    }
    sections
}

/// A session that runs `prefs`, then `setup`.
fn session(prefs: &str, setup: &str) -> Session {
    let mut session = Session::new();
    let result = session.eval(&format!("{prefs}\n{setup}"));
    assert!(result.error.is_none(), "{:?}", result.error);
    session
}

/// The fastest time of `code` in `session`, in seconds.
fn time(session: &mut Session, code: &str) -> f64 {
    fastest(RUNS, || {
        let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        assert!(result.error.is_none(), "{code}: {:?}", result.error);
    })
    .as_secs_f64()
}

#[test]
fn functions_cost_what_their_work_needs() {
    let sections = sections();
    let (setup, cases) = sections.split_first().unwrap();
    let setup = setup.2.join("\n");
    let mut sessions: HashMap<_, _> = cases.iter().map(|&(name, prefs, _)| (name, session(prefs, &setup))).collect();
    let n: usize = sessions.get_mut("copies").unwrap().eval("n").value.unwrap().to_string().parse().unwrap();
    let data: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let copy = || fastest(RUNS, || drop(black_box(black_box(&data).clone()))).as_secs_f64();
    let print_all = std::env::var_os("BASEDPL_COSTS").is_some();
    let mut failures = Vec::new();
    for &(name, _, ref lines) in cases {
        for line in lines {
            let mut fields = line.split('\t');
            let (code, budget): (&str, f64) = (fields.next().unwrap(), fields.next().unwrap().parse().unwrap());
            let (cost, line) = if name == "widths" {
                let fraction = time(sessions.get_mut(name).unwrap(), code) / time(sessions.get_mut("exact").unwrap(), code);
                (fraction, format!("{code}: 32-bit takes {fraction:.2} of the 64-bit time, budget {budget}"))
            } else {
                let unit = copy();
                let copies = time(sessions.get_mut(name).unwrap(), code) / unit;
                (copies, format!("{code}: {copies:.1} copies, budget {budget}"))
            };
            if print_all { eprintln!("{line}"); }
            if cost > budget { failures.push(line); }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
