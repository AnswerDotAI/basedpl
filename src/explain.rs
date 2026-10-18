//! The events that `•explain` reports about a run: the fused groups that ran, the pending arrays that calls forced, the fast combinations taken, and the accumulations, which add items at scattered positions. Events are recorded only while `•explain` runs. Fusion runs its graphs without a session, so every event goes to a sink on the thread that runs the expression.
//!
//! The module also counts the live nodes that `•memory` reports: the record nodes of `∂` and the pending arrays. A node counts for the thread that made it, even when another thread drops it.
use crate::{keyed, Error, ErrorAt, Value};
use std::{cell::{Cell, RefCell}, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

thread_local! {
    /// The events of the `•explain` running on this thread, if any.
    static EVENTS: RefCell<Option<Events>> = const { RefCell::new(None) };
    /// The live nodes of each kind that this thread made.
    static MADE: Arc<[AtomicUsize; 2]> = Arc::default();
    /// The items that pending arrays have computed on this thread, which `forced` counts across one call.
    static COMPUTED: Cell<usize> = const { Cell::new(0) };
}

/// The events of one `•explain`.
#[derive(Clone, Default)]
pub(crate) struct Events {
    /// Each fused group that ran: the glyphs of its calls in the order they ran, and its item count.
    fused: Vec<(String, usize)>,
    /// Each call that computed pending arrays' items: the function, its line and column, and the number of items computed.
    forced: Vec<(String, usize, usize, usize)>,
    /// Each fast combination taken: its name, and the function that computed it.
    fast: Vec<(&'static str, String)>,
    /// Each accumulation that ran: the items it added, and the items of its result.
    accumulations: Vec<(usize, usize)>,
}

/// Starts recording. Returns the events of an enclosing `•explain`, which `finish` restores.
pub(crate) fn start() -> Option<Events> { EVENTS.with(|e| e.replace(Some(Events::default()))) }

/// Stops recording and returns this run's events. An enclosing `•explain` gets them too, because they happened during its run.
pub(crate) fn finish(outer: Option<Events>) -> Events {
    EVENTS.with(|e| {
        let events = e.replace(None).unwrap_or_default();
        if let Some(mut outer) = outer {
            outer.fused.extend(events.fused.iter().cloned());
            outer.forced.extend(events.forced.iter().cloned());
            outer.fast.extend(events.fast.iter().cloned());
            outer.accumulations.extend(events.accumulations.iter().cloned());
            e.replace(Some(outer));
        }
        events
    })
}

fn record(f: impl FnOnce(&mut Events)) { EVENTS.with(|e| if let Some(events) = e.borrow_mut().as_mut() { f(events) }) }

/// Records a fused group of `items` items, whose calls have the glyphs `glyphs`, in the order they ran.
pub(crate) fn fused(glyphs: impl FnOnce() -> String, items: usize) { record(|e| e.fused.push((glyphs(), items))) }

/// Records that `function`, at `line` and `column`, computed `items` items of pending arrays: its arguments, or only the items it read from them.
pub(crate) fn forced(function: String, line: usize, column: usize, items: usize) { record(|e| e.forced.push((function, line, column, items))) }

/// Counts `items` items that a pending array computed. Pending arrays compute on the thread that reads them, so the count stays on that thread.
pub(crate) fn computed(items: usize) { COMPUTED.with(|c| c.set(c.get() + items)) }

/// The items that pending arrays have computed on this thread so far. A call's `forced` row takes the difference across the call.
pub(crate) fn computed_so_far() -> usize { COMPUTED.with(Cell::get) }

/// Records that `function` computed the fast combination named `name`.
pub(crate) fn fast(name: &'static str, function: String) { record(|e| e.fast.push((name, function))) }

/// Records an accumulation that added `items` items into a result of `size` items, each at a position that a table of offsets or ids gives, as scattering adds them.
pub(crate) fn accumulated(items: usize, size: usize) { record(|e| e.accumulations.push((items, size))) }

impl Events {
    /// `•explain`'s record: the expression's `value`, then `fused`, `forced`, `fast` and `accumulations`, each a record of columns with one row for each event.
    pub(crate) fn report(self, value: Value, span: &crate::Span) -> Result<Value, Error> {
        let texts = |items: Vec<String>| keyed::texts(&[items.len()], items.iter().map(|t| keyed::text(t)).collect()).error_at(span, "invalid explain texts");
        let counts = |items: Vec<usize>| Value::integers(vec![items.len()], items.into_iter().map(|n| n as i64).collect()).error_at(span, "invalid explain counts");
        let entries = |names: &[&str], values: Vec<Value>| keyed::record(names.iter().map(|&n| Arc::from(n)).collect(), values).error_at(span, "invalid explain record");
        let (glyphs, items) = self.fused.into_iter().unzip();
        let fused = entries(&["glyphs", "items"], vec![texts(glyphs)?, counts(items)?])?;
        let forced = entries(&["function", "line", "column", "items"], vec![
            texts(self.forced.iter().map(|(function, ..)| function.clone()).collect())?,
            counts(self.forced.iter().map(|&(_, line, ..)| line).collect())?,
            counts(self.forced.iter().map(|&(_, _, column, _)| column).collect())?,
            counts(self.forced.iter().map(|&(.., items)| items).collect())?,
        ])?;
        let (names, functions) = self.fast.into_iter().map(|(name, function)| (name.to_owned(), function)).unzip();
        let fast = entries(&["combination", "function"], vec![texts(names)?, texts(functions)?])?;
        let (added, sizes) = self.accumulations.into_iter().unzip();
        let accumulations = entries(&["items", "size"], vec![counts(added)?, counts(sizes)?])?;
        entries(&["value", "fused", "forced", "fast", "accumulations"], vec![value, fused, forced, fast, accumulations])
    }
}

/// The kinds of node that `•memory` counts.
#[derive(Clone, Copy)]
pub(crate) enum Counted { Record, Pending }

/// One live node, counted for the thread that made it until it's dropped, on whichever thread.
pub(crate) struct Live(Arc<[AtomicUsize; 2]>, Counted);

impl Live {
    pub(crate) fn new(kind: Counted) -> Self { MADE.with(|made| { made[kind as usize].fetch_add(1, Ordering::Relaxed); Self(made.clone(), kind) }) }
}

impl Drop for Live {
    fn drop(&mut self) { self.0[self.1 as usize].fetch_sub(1, Ordering::Relaxed); }
}

/// The live nodes of `kind` that this thread made.
pub(crate) fn live(kind: Counted) -> usize { MADE.with(|made| made[kind as usize].load(Ordering::Relaxed)) }
