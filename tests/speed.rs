//! How long each function takes, compared with copying the same data. The unit is one copy: allocating a vector of `N` 8-byte
//! numbers and copying `N` numbers into it. Moving data takes longer than simple arithmetic. A function that reads and writes each
//! item once should therefore cost about one copy. Each case has a budget in copies, set from the work that the function must do.
//! Plain Rust loops that do the same work fit inside each budget. The test lists every case over its budget.
//!
//! Timings come from the development build, which `cargo test` and `maturin develop` share. Each time is the fastest of several runs.
use basedpl::{EvalOptions, Session};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

const N: usize = 500_000;

/// Each group of cases, with the most copies' worth of time that each case may take.
const CASES: &[(f64, &[&str])] = &[
    // Sharing storage copies nothing.
    (0.25, &[",m", "500 1000⍴v"]),
    // Reading each item once, with almost nothing to write.
    (1.5, &["+/v", "+/jv", "⌈/v", "∧/b", "+/m", "+⌿m"]),
    // Reading one argument and writing each item once.
    (2.0, &["-v", "-jv", "~b", "⌽v", "1⌽v", "⊖m", "1↓v", "250000↑v", "⍳500000", "500000⍴0"]),
    // Reading two arguments and writing each item once.
    (2.5, &["v+w", "jv+jv", "v×2", "jv×2", "jv=jv", "v⌊w", "b∧b", "c=c", "(⍳1000)×⊗⍳500"]),
    // Compressing by a mask can't be vectorised, because each kept item's place depends on the items before it.
    (3.0, &["b#v", "⍸b"]),
    // Comparing within tolerance takes a subtraction, the magnitudes, a scale and a comparison for each item.
    (6.0, &["⌊v", "v<w", "jv<500"]),
    // Writing twice as many items, or writing in a different order from reading.
    (4.0, &["v,w", "⍉m"]),
    // A small-table index takes one pass for the range of the items, then one pass to write each item's first position into the
    // table. Each of these passes costs 1.4 to 2.5 copies, because a table read or write can't be vectorised.
    (6.0, &["≠jv"]),
    // A search also looks each item of the other argument up in the table, and unique then compresses by the mask.
    (10.0, &["jv⍳jv", "jv∊jv", "c⍳c", "∪jv", "∪c"]),
    // Key classifies the keys, counts and places each group's positions, then gathers each group's items.
    (32.0, &["{≢⍵}⌸b"]),
    // Each item waits for the result before it.
    (12.0, &["+\\v", "⌈\\v", "+\\jv", "≠\\b"]),
    // Each item waits for a read from a random place.
    (10.0, &["v i"]),
    // An approximate divisor reads the integers as floats. Each item then needs a float division and a floor within tolerance.
    (11.0, &["3|jv"]),
    // Each item needs a call to the maths library.
    (45.0, &["*v", "⍟v"]),
    // Each item goes into a hash table, and each item of the other argument is looked up in it. A tolerant search looks in two
    // buckets for each item, where an exact search looks in one, and a plain exact hash search costs about 60 copies.
    (300.0, &["v⍳w", "∪v"]),
    // About twenty comparisons for each item.
    (200.0, &["⍋v", "⍋jv"]),
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
    let copy = fastest(100, || drop(black_box(black_box(&data).clone()))).as_secs_f64();
    let mut session = Session::new();
    let setup = session.eval(&format!(
        r#"v←?{N}⍴0 ⋄ w←?{N}⍴0 ⋄ jv←?{N}⍴1000ₓ ⋄ b←0=?{N}⍴3ₓ ⋄ i←?{N}⍴{N}ₓ ⋄ m←1000 500⍴v ⋄ c←{N}⍴"the quick brown fox jumps over the lazy dog""#
    ));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    let mut failures = Vec::new();
    for &(budget, codes) in CASES {
        for &code in codes {
            let time = fastest(20, || {
                let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
                assert!(result.error.is_none(), "{code}: {:?}", result.error);
            });
            let copies = time.as_secs_f64() / copy;
            if copies > budget { failures.push(format!("{code}: {copies:.1} copies, budget {budget}")); }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
