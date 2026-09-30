//! How time and memory grow with the size of the arguments. Each case runs at `N` and `8 * N` items. Its time, allocated bytes and peak
//! bytes may each grow by at most `LIMIT` times. A linear or n log n cost stays below that. A quadratic cost grows about 64 times. A
//! global allocator counts the bytes that each call allocates, its peak and the bytes it keeps. It counts each thread separately, so
//! the test harness's own allocations stay out of the measurements.
use basedpl::{EvalOptions, Session};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    time::{Duration, Instant},
};

thread_local! {
    /// A thread can free memory that another thread allocated, so its live count can fall below zero.
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static TOTAL: Cell<usize> = const { Cell::new(0) };
}

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            TOTAL.set(TOTAL.get() + layout.size());
            LIVE.set(LIVE.get() + layout.size() as isize);
            PEAK.set(PEAK.get().max(LIVE.get()));
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.set(LIVE.get() - layout.size() as isize);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const N: usize = 1000;
const LIMIT: f64 = 16.0;
/// The most bytes that a call's peak may grow by for each added item. `⍕v` needs the most: about 19 characters of 4 bytes for each
/// number, beside the text they're read from.
const PEAK_PER_ITEM: usize = 128;

struct Cost {
    time: Duration,
    allocated: usize,
    peak: usize,
    kept: usize,
}

/// A session whose arguments have `n` items.
fn session(n: usize) -> Session {
    let mut session = Session::new();
    let setup = session.eval(&format!("v←?{n}⍴0 ⋄ w←?{n}⍴0 ⋄ iv←⍳{n}ₓ ⋄ b←0=3|iv ⋄ i←?{n}⍴{n}ₓ ⋄ m←{} 2⍴v ⋄ q←iv÷7", n / 2));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    session
}

fn cost(session: &mut Session, code: &str) -> Cost {
    let mut run = || {
        let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        assert!(result.error.is_none(), "{code}: {:?}", result.error);
    };
    run();
    let (live, total) = (LIVE.get(), TOTAL.get());
    PEAK.set(live);
    run();
    let (allocated, peak, kept) = (TOTAL.get() - total, (PEAK.get() - live) as usize, (LIVE.get() - live).max(0) as usize);
    let time = (0..3)
        .map(|_| {
            let start = Instant::now();
            run();
            start.elapsed()
        })
        .min()
        .unwrap();
    Cost { time, allocated, peak, kept }
}

/// The ways in which each of `codes` grows faster than `LIMIT` allows, or keeps memory.
fn growth_failures(codes: &[&str]) -> Vec<String> {
    let (mut small, mut large) = (session(N), session(8 * N));
    let ratio = |a: usize, b: usize| a as f64 / b.max(1) as f64;
    let mut failures = Vec::new();
    for &code in codes {
        let (s, l) = (cost(&mut small, code), cost(&mut large, code));
        let growth = [
            ("time", l.time.as_secs_f64() / s.time.as_secs_f64()),
            ("allocated bytes", ratio(l.allocated, s.allocated)),
            ("peak bytes", ratio(l.peak, s.peak)),
        ];
        for (what, r) in growth { if r > LIMIT { failures.push(format!("{code}: {what} grow {r:.1} times")); } }
        let per_item = l.peak.saturating_sub(s.peak) / (7 * N);
        if per_item > PEAK_PER_ITEM { failures.push(format!("{code}: peaks at {per_item} bytes per item")); }
        if s.kept + l.kept > 0 { failures.push(format!("{code}: keeps {} and {} bytes", s.kept, l.kept)); }
    }
    failures
}

/// A loop of single-item updates, or of appends, is linear in its count.
#[test]
#[ignore = "each update copies the array until step 1 of meta/inplace.md"]
fn updates_grow_linearly() {
    let failures = growth_failures(&["x←0×iv ⋄ {x(⍵)←1}¨iv", "r←⍬ ⋄ {r,←⍵}¨iv"]);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Each case may allocate `times` its result's bytes, plus `scratch` bytes for each argument item. Scalar and structural functions write
/// their result once. A reshape that keeps the number of items shares its argument's storage. Search builds a table of the items it
/// searches. Grade's radix sort moves pairs of a key and a position between two buffers, and returns its positions in one of them.
/// Format builds each number's text once, then converts it to characters. Its data is fixed, because the text's length decides where
/// its buffer doubles.
const BUDGETS: &[(&str, f64, usize)] = &[
    ("-v", 1.0, 0),
    ("v+w", 1.0, 0),
    ("v<w", 1.0, 0),
    ("⌊v", 1.0, 0),
    ("*v", 1.0, 0),
    ("iv+1ₓ", 1.0, 0),
    ("iv×0.5", 1.0, 0),
    ("v×iv", 1.0, 0),
    ("+/v", 1.0, 0),
    ("⌈/iv", 1.0, 0),
    ("+\\v", 1.0, 0),
    ("v,w", 1.0, 0),
    ("⌽v", 1.0, 0),
    ("1⌽v", 1.0, 0),
    ("⍉m", 1.0, 0),
    ("⊖m", 1.0, 0),
    ("((≢v)÷2)↑v", 1.0, 0),
    ("((≢v)÷2)↓v", 1.0, 0),
    ("v i", 1.0, 0),
    ("b#v", 1.0, 0),
    ("⍸b", 1.0, 0),
    (",m", 0.0, 0),
    ("(⍴m)⍴v", 0.0, 0),
    ("v⍳w", 1.0, 48),
    ("iv⍳i", 1.0, 16),
    ("i∊iv", 1.0, 16),
    ("≠i", 1.0, 16),
    ("∪i", 1.0, 24),
    ("⍋v", 1.0, 24),
    ("⍕q", 1.5, 0),
];

/// The bytes that the result of `code` holds: its number of items times the size of one item in its storage.
fn result_bytes(session: &mut Session, code: &str) -> usize {
    let sizes = format!("[1 8 8 16 4 {}]ₓ", std::mem::size_of::<basedpl::Value>());
    let names = r#""boolean" "integer" "float" "complex" "character" "mixed""#;
    let result = session.eval(&format!("r←{code} ⋄ s←•storage r ⋄ (≢,r)×{sizes}({names}⍳⊂s)"));
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    result.value.and_then(|v| v.as_number()).and_then(|n| n.as_integer()).unwrap() as usize
}

/// The cases whose bytes allocated for each added item exceed their budget, with 25% and one byte per item to spare. Fixed costs, such
/// as parsing the code, cancel in the difference between the runs at `N` and `8 * N` items.
fn allocation_failures() -> Vec<String> {
    let (mut small, mut large) = (session(N), session(8 * N));
    let added = (7 * N) as f64;
    let mut failures = Vec::new();
    for &(code, times, scratch) in BUDGETS {
        let allocated = cost(&mut large, code).allocated as f64 - cost(&mut small, code).allocated as f64;
        let result = result_bytes(&mut large, code) as f64 - result_bytes(&mut small, code) as f64;
        let (per_item, budget) = (allocated / added, (times * result / added + scratch as f64) * 1.25 + 1.0);
        if per_item > budget { failures.push(format!("{code}: {per_item:.1} bytes per item, budget {budget:.1}, result {:.1}", result / added)); }
    }
    failures
}

/// Scalar functions, reductions, catenation, indexing, search, Key, partitions, display and each are linear. Grade is n log n. Each case
/// in `BUDGETS` allocates within its budget.
#[test]
fn costs_grow_as_expected() {
    let mut failures = growth_failures(&["v+w", "+/v", "v,w", "v i", "v⍳w", "{≢⍵}⌸b", "b⊂iv", "⍕v", "{⍵+1}¨iv", "⍋v"]);
    failures.extend(allocation_failures());
    assert!(failures.is_empty(), "{failures:#?}");
}
