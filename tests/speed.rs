//! How long each function takes, compared with copying the same data. The unit is one copy: allocating a vector of `N` 8-byte
//! numbers and copying `N` numbers into it. Moving data takes longer than simple arithmetic. A function that reads and writes each
//! item once should therefore cost about one copy.
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
    // Each item waits for the result before it. In float scans by `+` and `⌈`, only one item in each group of four waits for the group
    // before it.
    ("+\\v", 4.4),
    ("⌈\\v", 6.6),
    ("+\\jv", 13.0),
    ("+\\b", 7.4),
    ("≠\\b", 5.8),
    // Each item waits for a read from a random place.
    ("[i]⌷v", 16.0),
    // An approximate divisor reads the integers as floats. Each item then needs a float division and a floor within tolerance.
    ("3|jv", 21.0),
    // rustymath's `exp` evaluates a polynomial of 12 terms for each item, and `ln` one of 7 terms and a division.
    ("*s", 23.0),
    ("⍟v", 31.0),
    // rustymath's sine and cosine reduce each item by π in three fused multiply-adds, and evaluate a polynomial of 7 terms.
    ("1○v", 17.0),
    ("2○v", 20.0),
    // Each item goes into a hash table, and each item of the other argument is looked up in it. A tolerant search looks in two
    // buckets for each item, where an exact search looks in one.
    ("v⍳w", 710.0),
    // A radix sort moves every item once for each byte that differs between the keys.
    ("⍋v", 280.0),
    ("⍋jv", 110.0),
    // Unique sorts the reals, then compares each with the next one.
    ("∪v", 320.0),
];

/// Cases that run with the tolerance pref at 0, where approximate numbers compare exactly. CI hasn't run them yet. Each budget takes
/// its CI cost from the CI-to-Mac ratio of a case above with a similar loop.
const EXACT_CASES: &[(&str, f64)] = &[
    // Plain IEEE comparisons and `floor`, with nothing to scale by a tolerance.
    ("⌊v", 5.0),
    ("v<w", 3.0),
    ("v=w", 3.2),
    ("3|v", 5.8),
    // Each real indexes its `order_key` in a hash map, as exact data does.
    ("v⍳w", 340.0),
    ("∪v", 410.0),
];

/// Cases that run the same code on 64-bit and on 32-bit floats: in the session above, whose tolerance pref is now 0, and in one whose
/// `float` pref is 32. There `v`, `w`, `m` and `a` hold 32-bit floats, which compare exactly. The budget is the most that the 32-bit time
/// may be, as a fraction of the 64-bit time. One machine times both, so its caches and memory affect them alike. CI hasn't run them yet.
const WIDTH_CASES: &[(&str, f64)] = &[
    // Half the bytes to read and write, and twice the lanes in each SIMD instruction.
    ("-v", 0.7),
    ("v+w", 0.7),
    ("v×2", 0.7),
    ("v<w", 0.7),
    ("0.5<v", 0.7),
    ("√v", 0.7),
    ("+⌿m", 0.7),
    ("+/v", 0.7),
    ("⌈/v", 0.7),
    ("a+.×a", 0.7),
    // `⌊` reads half the bytes and writes 32-bit integers in place of 64-bit ones.
    ("⌊v", 0.8),
    // An LU decomposition in 32 bits.
    ("⌹a", 0.8),
    // rustymath's `exp` and `ln` run shorter polynomials at f32, with twice the lanes in each SIMD instruction.
    ("*s", 0.5),
    ("⍟v", 0.7),
    // rustymath's sine and cosine reduce in f32 with fused multiply-adds, and run a polynomial of 4 terms in place of 7.
    ("1○v", 0.6),
    ("2○v", 0.6),
    // Grade sorts keys and positions of half the bytes.
    ("⍋v", 0.75),
    // A hash lookup costs about the same at either width. These budgets catch a 32-bit search that costs much more than a 64-bit one.
    ("v⍳w", 1.15),
    ("∪v", 1.15),
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

/// A session holding the data that the cases use, made by code that starts with `prefs`.
fn session(prefs: &str) -> Session {
    let mut session = Session::new();
    let setup = session.eval(&format!(
        r#"{prefs}v←¿{N}⍴0 ⋄ w←¿{N}⍴0 ⋄ jv←¿{N}⍴1000ₓ ⋄ kv←¿{N}⍴1000000000000ₓ ⋄ b←0=¿{N}⍴3ₓ ⋄ i←¿{N}⍴{N}ₓ ⋄ s←v-0.5 ⋄ m←1000 500⍴v ⋄ a←300 300⍴v ⋄ c←{N}⍴"the quick brown fox jumps over the lazy dog""#
    ));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    session
}

/// The fastest time of `code` in `session`, in seconds.
fn time(session: &mut Session, code: &str) -> f64 {
    fastest(20, || {
        let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        assert!(result.error.is_none(), "{code}: {:?}", result.error);
    })
    .as_secs_f64()
}

#[test]
fn functions_cost_what_their_work_needs() {
    let data: Vec<f64> = (0..N).map(|i| i as f64).collect();
    let copy = || fastest(100, || drop(black_box(black_box(&data).clone()))).as_secs_f64();
    let mut session = session("");
    let print_all = std::env::var_os("BASEDPL_COSTS").is_some();
    let mut failures = Vec::new();
    let mut measure = |session: &mut Session, cases: &[(&str, f64)]| {
        for &(code, budget) in cases {
            let unit = copy();
            let copies = time(session, code) / unit;
            let line = format!("{code}: {copies:.1} copies, budget {budget}");
            if print_all { eprintln!("{line}"); }
            if copies > budget { failures.push(line); }
        }
    };
    measure(&mut session, CASES);
    assert!(session.eval("_←•prefs [\"tolerance\":0]").error.is_none());
    measure(&mut session, EXACT_CASES);
    let mut single = self::session("•prefs [\"float\":32] ⋄ ");
    for &(code, budget) in WIDTH_CASES {
        let fraction = time(&mut single, code) / time(&mut session, code);
        let line = format!("{code}: 32-bit takes {fraction:.2} of the 64-bit time, budget {budget}");
        if print_all { eprintln!("{line}"); }
        if fraction > budget { failures.push(line); }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
