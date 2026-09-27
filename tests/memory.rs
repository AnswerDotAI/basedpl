//! Memory use of each way of building a result, counted by a global allocator that tracks live and peak bytes.
use basedpl::{EvalOptions, Session};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() { PEAK.fetch_max(LIVE.fetch_add(layout.size(), Relaxed) + layout.size(), Relaxed); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const N: usize = 100_000;

/// Each expression builds its result a different way. A call must free everything it allocates, and its peak must stay within a
/// fixed number of bytes per item of the `N`-item arguments.
#[test]
fn results_free_their_memory() {
    let mut session = Session::new();
    let setup = session.eval(&format!("v←?{N}⍴0 ⋄ w←?{N}⍴0 ⋄ t←{N}⍴10 ⋄ iv←⍳{N} ⋄ b←0=3|iv ⋄ i←?{N}⍴{N}"));
    assert!(setup.error.is_none(), "{:?}", setup.error);
    let codes = ["v+w", "1,v", "b/v", "v i", "v@(i)⊢w", "v⍳w", "?t", "⍸b", "∊[v w]", "b⊂iv", "{≢⍵}⌸b", "{⍵+1}¨iv"];
    let mut run = |code: &str| {
        let result = session.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        assert!(result.error.is_none(), "{code}: {:?}", result.error);
    };
    let mut failures = Vec::new();
    for code in codes {
        run(code);
        let before = LIVE.load(Relaxed);
        PEAK.store(before, Relaxed);
        run(code);
        let peak = (PEAK.load(Relaxed) - before) / N;
        for _ in 0..3 { run(code); }
        let kept = LIVE.load(Relaxed).saturating_sub(before);
        if kept > 1 << 16 || peak > 64 { failures.push(format!("{code}: keeps {kept} bytes, peaks at {peak} bytes per item")); }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
