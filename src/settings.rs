//! The settings that `•prefs` changes, held once for the whole process. Code on any thread reads them, and a computation reads the settings in force when it runs. `•prefs` writes them, the lexer writes the settings in a source's leading `•prefs` header as it reads them, and `reset` restores the defaults, as `]clear` does. Nothing keeps a setting to put back later. Rustymath holds the thread count, the one setting that it reads itself (`rustymath::parallel::set_threads`).
//!
//! Each setting that calls and scalar operations read is an atomic, read with a relaxed load. A read of several settings can see some of them from before a concurrent write and some from after it. The display settings, which only display reads, sit behind a lock.
use crate::{display, fusion::Sizes, number::{Kinds, Numeric}};
use rustymath::{buffer::{FloatWidth, Width}, tolerant::Tolerance};
use std::sync::{LazyLock, PoisonError, RwLock, atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering::Relaxed}};

/// The defaults of the flags, which their statics start from and `reset` restores: whether hanzi spell glyphs, whether `batch` is on, whether calls report the pending arrays they compute, and whether fusion takes its small sizes.
const HANZI: bool = false;
const BATCH: bool = true;
const REPORTS: bool = false;
const SMALL: bool = false;

static TOLERANCE: AtomicU64 = AtomicU64::new(Numeric::DEFAULT.tolerance.value().to_bits());
static FLOAT: AtomicU8 = AtomicU8::new(Numeric::DEFAULT.width as u8);
static STATIC_KINDS: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.kinds, Kinds::Static));
static INT32: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.int, Width::I32));
static HANZI_ALIASES: AtomicBool = AtomicBool::new(HANZI);
static BATCHING: AtomicBool = AtomicBool::new(BATCH);
static REPORTS_FUSION: AtomicBool = AtomicBool::new(REPORTS);
static SMALL_SIZES: AtomicBool = AtomicBool::new(SMALL);
/// The display settings in force, then the ones that `reset` restores.
static DISPLAY: LazyLock<RwLock<[display::Settings; 2]>> = LazyLock::new(|| RwLock::new([display::Settings::default(); 2]));

/// The numeric settings in force.
#[inline]
pub(crate) fn numeric() -> Numeric {
    let tolerance = Tolerance::new(f64::from_bits(TOLERANCE.load(Relaxed))).expect("a stored tolerance");
    let kinds = if STATIC_KINDS.load(Relaxed) { Kinds::Static } else { Kinds::Dynamic };
    Numeric { tolerance, width: float_width(), kinds, int: if INT32.load(Relaxed) { Width::I32 } else { Width::I64 }, ..Numeric::DEFAULT }
}
/// Makes `numeric` the numeric settings in force. Only `Numeric::convergence` changes `units`, for its own comparisons, so the store leaves them out.
pub(crate) fn set_numeric(numeric: Numeric) {
    TOLERANCE.store(numeric.tolerance.value().to_bits(), Relaxed);
    FLOAT.store(numeric.width as u8, Relaxed);
    STATIC_KINDS.store(numeric.kinds == Kinds::Static, Relaxed);
    INT32.store(numeric.int == Width::I32, Relaxed);
}
/// The width of new floats, which the `float` pref sets.
#[inline]
pub fn float_width() -> FloatWidth {
    match FLOAT.load(Relaxed) { w if w == FloatWidth::F16 as u8 => FloatWidth::F16, w if w == FloatWidth::F32 as u8 => FloatWidth::F32, _ => FloatWidth::F64 }
}
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
/// The sizes that decide which calls wait and how a graph splits its items: `Sizes::SMALL` while the reference harness asks for them, and the defaults otherwise.
pub(crate) fn sizes() -> Sizes { if SMALL_SIZES.load(Relaxed) { Sizes::SMALL } else { Sizes::DEFAULT } }
pub(crate) fn set_small_sizes(on: bool) { SMALL_SIZES.store(on, Relaxed) }

/// Restores every setting's default, with the display defaults that `set_display_defaults` gave, and rustymath's default thread count. `]clear` does this, and the test harnesses do it before each test.
pub fn reset() {
    set_numeric(Numeric::DEFAULT);
    for (flag, default) in [(&HANZI_ALIASES, HANZI), (&BATCHING, BATCH), (&REPORTS_FUSION, REPORTS), (&SMALL_SIZES, SMALL)] { flag.store(default, Relaxed); }
    let mut display = DISPLAY.write().unwrap_or_else(PoisonError::into_inner);
    display[0] = display[1];
    rustymath::parallel::set_threads(0);
}
