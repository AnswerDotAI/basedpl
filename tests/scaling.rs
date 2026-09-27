//! How time and memory grow with the size of the arguments. Each case runs at `N` and `8 * N` items. Its time, allocated bytes and peak
//! bytes may each grow by at most `LIMIT` times. A linear or n log n cost stays below that. A quadratic cost grows about 64 times. A
//! global allocator counts the bytes that each call allocates, its peak and the bytes it keeps.
use basedpl::{EvalOptions, Session};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        Mutex,
    },
    time::{Duration, Instant},
};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            TOTAL.fetch_add(layout.size(), Relaxed);
            PEAK.fetch_max(LIVE.fetch_add(layout.size(), Relaxed) + layout.size(), Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The counts are global, so the tests in this file take turns.
static SERIAL: Mutex<()> = Mutex::new(());

const N: usize = 1000;
const LIMIT: f64 = 16.0;
/// The most bytes that a call's peak may grow by for each added item. `⍕v` needs the most: about 19 characters of 4 bytes for each
/// number, beside the text they're read from.
const PEAK_PER_ITEM: usize = 128;

struct Cost { time: Duration, allocated: usize, peak: usize, kept: usize }

/// A session whose arguments have `n` items.
fn session(n: usize) -> Session {
    let mut session = Session::new();
    let setup = session.eval(&format!("v←?{n}⍴0 ⋄ w←?{n}⍴0 ⋄ iv←⍳{n}ₓ ⋄ b←0=3|iv ⋄ i←?{n}⍴{n}ₓ"));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    session
}

fn cost(session: &mut Session, code: &str) -> Cost {
    let mut run = || {
        let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        assert!(result.error.is_none(), "{code}: {:?}", result.error);
    };
    run();
    let (live, total) = (LIVE.load(Relaxed), TOTAL.load(Relaxed));
    PEAK.store(live, Relaxed);
    run();
    let (allocated, peak, kept) = (TOTAL.load(Relaxed) - total, PEAK.load(Relaxed) - live, LIVE.load(Relaxed).saturating_sub(live));
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
fn failures(codes: &[&str]) -> Vec<String> {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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

/// Scalar functions, reductions, catenation, indexing, search, Key, partitions, display and each are linear. Grade is n log n.
#[test]
fn costs_grow_as_expected() {
    let failures = failures(&["v+w", "+/v", "v,w", "v i", "v⍳w", "{≢⍵}⌸b", "b⊂iv", "⍕v", "{⍵+1}¨iv", "⍋v"]);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A loop of single-item updates, or of appends, is linear in its count.
#[test]
#[ignore = "each update copies the array until step 1 of meta/inplace.md"]
fn updates_grow_linearly() {
    let failures = failures(&["x←0×iv ⋄ {x(⍵)←1}¨iv", "r←⍬ ⋄ {r,←⍵}¨iv"]);
    assert!(failures.is_empty(), "{failures:#?}");
}
