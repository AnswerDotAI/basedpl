//! How long each function takes, compared with copying the same data. The unit is one copy: allocating a vector of `N` 8-byte
//! numbers and copying `N` numbers into it. Moving data takes longer than simple arithmetic. A function that reads and writes each
//! item once should therefore cost about one copy.
//!
//! Each case has a budget in copies: twice the most it has cost on a Mac and on CI's Linux runner, and at least half a copy. Counting
//! copies corrects for a machine's overall speed. It doesn't correct for differences in caches and memory. A case can cost up to
//! three times as many copies on one machine as on the other. The test fails listing every case over its budget, with its cost.
//! To print every case's cost, set `BASEDPL_COSTS`: `BASEDPL_COSTS=1 cargo test --test speed -- --nocapture`.
//!
//! Timings come from the development build, which `cargo test` and `cargo develop` share. Each time is the fastest of several runs. The test divides each case's time by a copy timed just before it. A change in the machine's speed during the run then affects both times.
use basedpl::{EvalOptions, Session};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

const N: usize = 500_000;

/// Each case, with the most copies' worth of time that it may take.
const CASES: &[(&str, f64)] = &[
    // Sharing storage copies nothing.
    (",m", 0.5),
    ("500 1000⍴v", 0.5),
    // Reading each item once, with almost nothing to write.
    ("+/v", 1.8),
    ("+/jv", 1.4),
    ("⌈/v", 2.4),
    ("∧/b", 0.6),
    ("+/m", 2.0),
    ("+⌿m", 2.6),
    // Reading one argument and writing each item once.
    ("-v", 2.8),
    ("-jv", 1.4),
    ("~b", 0.5),
    ("∧\\b", 0.5),
    ("∨\\b", 0.5),
    ("⌽v", 2.8),
    ("1⌽v", 2.8),
    ("⊖m", 3.6),
    ("1↓v", 2.8),
    ("250000↑v", 1.4),
    ("500000⍴0", 1.2),
    // Reading two arguments and writing each item once.
    ("v+w", 4.6),
    ("jv+jv", 2.0),
    ("kv+kv", 4.6),
    ("v×2", 2.8),
    ("jv×2", 2.2),
    ("jv=jv", 1.2),
    ("b×jv", 1.8),
    ("b×kv", 3.8),
    ("v⌊w", 4.6),
    ("b∧b", 1.2),
    ("c=c", 1.0),
    ("(⍳1000)×⊗⍳500", 2.6),
    // Compressing by a mask can't be vectorised, because each kept item's place depends on the items before it.
    ("b#v", 5.8),
    ("⍸b", 4.8),
    // A range with an approximate length converts each position from an integer to a float.
    ("⍳500000", 1.6),
    // Comparing within tolerance takes a subtraction, the magnitudes, a scale and a comparison for each item.
    ("⌊v", 18.0),
    ("v<w", 11.0),
    ("jv<500", 5.2),
    // Writing twice as many items, or writing in a different order from reading.
    ("v,w", 4.6),
    ("⍉m", 8.2),
    // A small-table index takes one pass for the range of the items, then one pass to write each item's first position into the
    // table. Each of these passes costs 1.4 to 2.5 copies, because a table read or write can't be vectorised.
    ("≠jv", 8.8),
    // A search also looks each item of the other argument up in the table, and unique then compresses by the mask.
    ("jv⍳jv", 19.0),
    ("jv∊jv", 19.0),
    ("c⍳c", 19.0),
    ("∪jv", 15.0),
    ("∪c", 14.0),
    // Key classifies the keys, counts and places each group's positions, then gathers each group's items.
    ("{≢⍵}⌸b", 61.0),
    // Each item waits for the result before it.
    ("+\\v", 12.0),
    ("⌈\\v", 22.0),
    ("+\\jv", 13.0),
    ("+\\b", 7.4),
    ("≠\\b", 5.8),
    // Each item waits for a read from a random place.
    ("[i]⌷v", 16.0),
    // An approximate divisor reads the integers as floats. Each item then needs a float division and a floor within tolerance.
    ("3|jv", 21.0),
    // Each item needs a call to the maths library.
    ("*v", 54.0),
    ("⍟v", 65.0),
    // Each item goes into a hash table, and each item of the other argument is looked up in it. A tolerant search looks in two
    // buckets for each item, where an exact search looks in one.
    ("v⍳w", 710.0),
    // A radix sort moves every item once for each byte that differs between the keys.
    ("⍋v", 280.0),
    ("⍋jv", 110.0),
    // Unique sorts the reals, then compares each with the next one.
    ("∪v", 320.0),
];

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

#[test]
fn functions_cost_what_their_work_needs() {
    let data: Vec<f64> = (0..N).map(|i| i as f64).collect();
    let copy = || fastest(100, || drop(black_box(black_box(&data).clone()))).as_secs_f64();
    let mut session = Session::new();
    let setup = session.eval(&format!(
        r#"v←¿{N}⍴0 ⋄ w←¿{N}⍴0 ⋄ jv←¿{N}⍴1000ₓ ⋄ kv←¿{N}⍴1000000000000ₓ ⋄ b←0=¿{N}⍴3ₓ ⋄ i←¿{N}⍴{N}ₓ ⋄ m←1000 500⍴v ⋄ c←{N}⍴"the quick brown fox jumps over the lazy dog""#
    ));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    let print_all = std::env::var_os("BASEDPL_COSTS").is_some();
    let mut failures = Vec::new();
    for &(code, budget) in CASES {
        let unit = copy();
        let time = fastest(20, || {
            let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
        });
        let copies = time.as_secs_f64() / unit;
        let line = format!("{code}: {copies:.1} copies, budget {budget}");
        if print_all { eprintln!("{line}"); }
        if copies > budget { failures.push(line); }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
