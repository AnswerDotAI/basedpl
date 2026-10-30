//! The settings that `•prefs` changes, held once for the whole process. Code on any thread reads them, and a computation reads the settings in force when it runs. `•prefs` writes them, the lexer writes the settings in a source's leading `•prefs` header as it reads them, and `reset` restores the defaults, as `]clear` does. Nothing keeps a setting to put back later. Rustymath holds the thread count, the one setting that it reads itself (`rustymath::parallel::set_threads`).
//!
//! Each setting that calls and scalar operations read is an atomic, read with a relaxed load. A read of several settings can see some of them from before a concurrent write and some from after it. The display settings, which only display reads, sit behind a lock.
pub use crate::fusion::Sizes;
use crate::{display, number::{Kinds, Numeric}};
use rustymath::buffer::{FloatWidth, Width};
use std::sync::{LazyLock, PoisonError, RwLock, atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering::Relaxed}};

/// The defaults of the flags, which their statics start from and `reset` restores: whether hanzi spell glyphs, whether `batch` is on, whether calls report the pending arrays they compute, and whether each program is checked before it runs.
const HANZI: bool = false;
const BATCH: bool = true;
const REPORTS: bool = false;
const VERIFY: bool = false;

static MATHISH: AtomicBool = AtomicBool::new(Numeric::DEFAULT.mathish);
static FLOAT: AtomicU8 = AtomicU8::new(Numeric::DEFAULT.width as u8);
static FLOAT_SET: AtomicBool = AtomicBool::new(false);
static STATIC_KINDS: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.kinds, Kinds::Static));
static INT32: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.int, Width::I32));
static HANZI_ALIASES: AtomicBool = AtomicBool::new(HANZI);
static BATCHING: AtomicBool = AtomicBool::new(BATCH);
static REPORTS_FUSION: AtomicBool = AtomicBool::new(REPORTS);
/// Fusion's sizes, one atomic for each field of `Sizes` (`sizes`). `MIN_ITEMS` holds `UNSET_ITEMS` for its default, which `sizes` reads from the environment.
static MIN_ITEMS: AtomicUsize = AtomicUsize::new(UNSET_ITEMS);
const UNSET_ITEMS: usize = usize::MAX;
/// The sizes that `reset` restores: `Sizes::DEFAULT`, with the fewest items that wait left to `sizes`.
const UNSET_SIZES: Sizes = Sizes { min_items: UNSET_ITEMS, ..Sizes::DEFAULT };
static BLOCK: AtomicUsize = AtomicUsize::new(Sizes::DEFAULT.block);
static RUN: AtomicUsize = AtomicUsize::new(Sizes::DEFAULT.run);
static VERIFIES: AtomicBool = AtomicBool::new(VERIFY);
/// When block stages compile their parts, as `Compiles` numbers its variants, or `UNSET` for the default (`compiles`).
static COMPILES: AtomicU8 = AtomicU8::new(UNSET);
const UNSET: u8 = u8::MAX;
/// The display settings in force, then the ones that `reset` restores.
static DISPLAY: LazyLock<RwLock<[display::Settings; 2]>> = LazyLock::new(|| RwLock::new([display::Settings::default(); 2]));

/// The numeric settings in force.
#[inline]
pub(crate) fn numeric() -> Numeric {
    let mathish = MATHISH.load(Relaxed);
    let kinds = if STATIC_KINDS.load(Relaxed) { Kinds::Static } else { Kinds::Dynamic };
    Numeric { mathish, width: float_width(), kinds, int: if INT32.load(Relaxed) { Width::I32 } else { Width::I64 }, ..Numeric::DEFAULT }.with_public_tolerance()
}
/// The `int` width under static kinds, and `None` under dynamic kinds. Static kinds store every ordinary integer array at the `int` width, and dynamic kinds store each one at the narrowest width that holds its values (`array::ordinary_width`). It reads only the two settings that array construction needs.
#[inline]
pub(crate) fn int_storage() -> Option<Width> { STATIC_KINDS.load(Relaxed).then(|| if INT32.load(Relaxed) { Width::I32 } else { Width::I64 }) }
/// Makes `numeric` the numeric settings in force. Only `Numeric::convergence` changes `units`, for its own comparisons, so the store leaves them out.
pub(crate) fn set_numeric(numeric: Numeric) {
    MATHISH.store(numeric.mathish, Relaxed);
    FLOAT.store(numeric.width as u8, Relaxed);
    STATIC_KINDS.store(numeric.kinds == Kinds::Static, Relaxed);
    INT32.store(numeric.int == Width::I32, Relaxed);
}
/// The width of new floats. Until the `float` pref sets one, `Numeric::set` keeps it at the width the kinds give.
#[inline]
pub fn float_width() -> FloatWidth {
    match FLOAT.load(Relaxed) { w if w == FloatWidth::F16 as u8 => FloatWidth::F16, w if w == FloatWidth::F32 as u8 => FloatWidth::F32, _ => FloatWidth::F64 }
}
/// Whether the `float` pref has set the width of new floats, which a change of `kinds` then leaves alone.
pub(crate) fn float_set() -> bool { FLOAT_SET.load(Relaxed) }
pub(crate) fn set_float_set(on: bool) { FLOAT_SET.store(on, Relaxed) }
/// The display settings in force.
pub(crate) fn display() -> display::Settings { DISPLAY.read().unwrap_or_else(PoisonError::into_inner)[0] }
pub(crate) fn set_display(display: display::Settings) { DISPLAY.write().unwrap_or_else(PoisonError::into_inner)[0] = display; }
/// Makes `display` the display settings, and the ones that `reset` restores, as an interactive frontend does.
pub(crate) fn set_display_defaults(display: display::Settings) { *DISPLAY.write().unwrap_or_else(PoisonError::into_inner) = [display; 2]; }
/// Whether calls report the pending arrays they compute, as `•prefs ["fusion":$t]` asks.
#[inline]
pub(crate) fn reports_fusion() -> bool { REPORTS_FUSION.load(Relaxed) }
pub(crate) fn set_reports_fusion(on: bool) { REPORTS_FUSION.store(on, Relaxed) }
/// Whether maps first try one call of their operand on batches of all their cells (`batch::tried`), as the `batch` pref asks.
#[inline]
pub(crate) fn batch() -> bool { BATCHING.load(Relaxed) }
pub(crate) fn set_batch(on: bool) { BATCHING.store(on, Relaxed) }
/// Whether hanzi spell glyphs, as a leading `•prefs ["hanzi_aliases":$t]` declares.
pub fn hanzi_aliases() -> bool { HANZI_ALIASES.load(Relaxed) }
pub(crate) fn set_hanzi_aliases(on: bool) { HANZI_ALIASES.store(on, Relaxed) }
/// Fusion's sizes in force, as `set_sizes` set them. Until then, and after each `reset`, the fewest items of a call that waits come from `BASEDPL_MIN_ITEMS`, read once for the process, or else from `Sizes::DEFAULT`, so that timings can defer calls smaller than the default.
pub(crate) fn sizes() -> Sizes {
    static FROM_ENV: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let min_items = match MIN_ITEMS.load(Relaxed) {
        UNSET_ITEMS => *FROM_ENV.get_or_init(|| std::env::var("BASEDPL_MIN_ITEMS").ok().and_then(|n| n.parse().ok()).unwrap_or(Sizes::DEFAULT.min_items)),
        n => n,
    };
    Sizes { min_items, block: BLOCK.load(Relaxed), run: RUN.load(Relaxed) }
}
/// Sets the sizes that decide which calls fusion defers and how a block stage splits its items, for the whole process. `reset` restores the defaults (`sizes`).
pub fn set_sizes(sizes: Sizes) {
    MIN_ITEMS.store(sizes.min_items, Relaxed);
    BLOCK.store(sizes.block, Relaxed);
    RUN.store(sizes.run, Relaxed);
}
/// `set_sizes` with `Sizes::SMALL`, or with the defaults that `reset` restores where `on` is false. The reference harness runs each case under the small sizes too.
pub fn set_small_sizes(on: bool) { set_sizes(if on { Sizes::SMALL } else { UNSET_SIZES }) }
/// Whether each program is checked before it runs (`program::verify::check`). The reference harness turns it on for every case and page.
pub(crate) fn verifies() -> bool { VERIFIES.load(Relaxed) }
pub(crate) fn set_verifies(on: bool) { VERIFIES.store(on, Relaxed) }

/// How block stages compile their parts (`blocks::Blocks::compiles`): by the compile policy, every stage on its first run, or none. The reference harness sets it for each run, and `tests/jit.rs` sets it to compare compiled stages with uncompiled ones (`compiled_runs`). Timings compare compiled stages with none through `BASEDPL_COMPILE`. The browser build compiles nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Compiles { Policy, Always, Never }

#[cfg(not(web))]
impl Compiles {
    /// `always` or `never` from `BASEDPL_COMPILE`, read once for the process, and otherwise the policy.
    fn from_env() -> Self {
        static FROM_ENV: std::sync::OnceLock<Compiles> = std::sync::OnceLock::new();
        *FROM_ENV.get_or_init(|| match std::env::var("BASEDPL_COMPILE").as_deref() {
            Ok("always") => Self::Always, Ok("never") => Self::Never, _ => Self::Policy
        })
    }
}

/// When block stages compile their parts: as `set_compiles` set it, or as `Compiles::from_env` gives it until then and after each `reset`.
#[cfg(not(web))]
pub(crate) fn compiles() -> Compiles {
    match COMPILES.load(Relaxed) { 0 => Compiles::Policy, 1 => Compiles::Always, 2 => Compiles::Never, _ => Compiles::from_env() }
}
pub fn set_compiles(compiles: Compiles) { COMPILES.store(compiles as u8, Relaxed) }
pub use crate::program::blocks::compiled_runs;

/// Restores every setting's default, with the display defaults that `set_display_defaults` gave, and rustymath's default thread count. `]clear` does this, and the test harnesses do it before each test.
pub fn reset() {
    set_numeric(Numeric::DEFAULT);
    set_float_set(false);
    for (flag, default) in [(&HANZI_ALIASES, HANZI), (&BATCHING, BATCH), (&REPORTS_FUSION, REPORTS), (&VERIFIES, VERIFY)] { flag.store(default, Relaxed); }
    set_sizes(UNSET_SIZES);
    COMPILES.store(UNSET, Relaxed);
    let mut display = DISPLAY.write().unwrap_or_else(PoisonError::into_inner);
    display[0] = display[1];
    rustymath::parallel::set_threads(0);
}
