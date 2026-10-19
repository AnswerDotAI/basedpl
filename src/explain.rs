//! The events that `•explain` reports about a run: the fused groups that ran, with the blocks, runs and copies they made, the pending arrays that fused groups read only after computing them on their own, the pending arrays that calls forced, the fast combinations taken, and the accumulations, which add items at scattered positions, with the path each took. Events are recorded only while `•explain` runs. Fusion runs its graphs without a session, so every event goes to a sink on the thread that runs the expression.
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

/// A cell in an event's row.
#[derive(Clone)]
enum Field { Text(String), Count(usize) }

/// What a column holds.
#[derive(Clone, Copy)]
enum Column { Text, Count }

/// Each kind of event, in the order `•explain` reports them, with its columns:
/// - `fused`: each fused group that ran, with the glyphs of its calls in the order they ran, its item count, and the blocks, runs and copied items that its `Tally` counted.
/// - `stored`: each pending array that a fused group read as an input, computed on its own first, with the glyphs of its call, its item count, and why the group couldn't compute it.
/// - `forced`: each call that computed pending arrays' items, with the function, its line and column, and the number of items computed.
/// - `fast`: each fast combination taken, with its name and the function that computed it.
/// - `accumulations`: each accumulation that ran, with the items it added, the items of its result, and the path it took.
const KINDS: [(&str, &[(&str, Column)]); 5] = {
    use Column::*;
    [
        ("fused", &[("glyphs", Text), ("items", Count), ("blocks", Count), ("runs", Count), ("copied", Count)]),
        ("stored", &[("glyphs", Text), ("items", Count), ("reason", Text)]),
        ("forced", &[("function", Text), ("line", Count), ("column", Count), ("items", Count)]),
        ("fast", &[("combination", Text), ("function", Text)]),
        ("accumulations", &[("items", Count), ("size", Count), ("path", Text)]),
    ]
};

/// A kind of event, by its position in `KINDS`.
#[derive(Clone, Copy)]
enum Kind { Fused, Stored, Forced, Fast, Accumulations }

/// The events of one `•explain`: for each of `KINDS`, one row for each event.
#[derive(Clone, Default)]
pub(crate) struct Events { rows: [Vec<Vec<Field>>; KINDS.len()] }

/// Starts recording. Returns the events of an enclosing `•explain`, which `finish` restores.
pub(crate) fn start() -> Option<Events> { EVENTS.with(|e| e.replace(Some(Events::default()))) }

/// Stops recording and returns this run's events. An enclosing `•explain` gets them too, because they happened during its run.
pub(crate) fn finish(outer: Option<Events>) -> Events {
    EVENTS.with(|e| {
        let events = e.replace(None).unwrap_or_default();
        if let Some(mut outer) = outer {
            outer.extend(&events);
            e.replace(Some(outer));
        }
        events
    })
}

/// Adds the row that `row` gives to the events of `kind`, while an `•explain` records on this thread.
fn record(kind: Kind, row: impl FnOnce() -> Vec<Field>) { EVENTS.with(|e| if let Some(events) = e.borrow_mut().as_mut() { events.rows[kind as usize].push(row()) }) }

/// Records a fused group of `items` items, whose calls have the glyphs `glyphs`, in the order they ran, with what `tally` counted while it ran.
pub(crate) fn fused(glyphs: impl FnOnce() -> String, items: usize, tally: &Tally) {
    let Some(c) = &tally.0 else { return };
    let count = |n: &AtomicUsize| Field::Count(n.load(Ordering::Relaxed));
    record(Kind::Fused, || vec![Field::Text(glyphs()), Field::Count(items), count(&c.blocks), count(&c.runs), count(&c.copied)])
}

/// Records a pending array that a fused group read as an input, computed on its own first. `event` gives the glyphs of its call, its item count and the reason the group couldn't compute it as one of its calls.
pub(crate) fn stored(event: impl FnOnce() -> (String, usize, &'static str)) {
    record(Kind::Stored, || { let (glyphs, items, reason) = event(); vec![Field::Text(glyphs), Field::Count(items), Field::Text(reason.into())] })
}

/// Whether an `•explain` records on this thread.
fn recording() -> bool { EVENTS.with(|e| e.borrow().is_some()) }

/// What a fused group's blocks did while `•explain` records: the blocks, the runs that they split into where an input repeats or cycles, and the items copied out of inputs before the kernels read them. Blocks run on worker threads, so each count is atomic. Outside `•explain` a tally holds no counts, so the threads share no counters.
pub(crate) struct Tally(Option<Counts>);

#[derive(Default)]
struct Counts { blocks: AtomicUsize, runs: AtomicUsize, copied: AtomicUsize }

impl Tally {
    /// A tally that counts while an `•explain` records on this thread, and otherwise counts nothing.
    pub(crate) fn new() -> Self { Tally(recording().then(Counts::default)) }
    pub(crate) fn block(&self) { self.add(|c| &c.blocks, 1) }
    pub(crate) fn run(&self) { self.add(|c| &c.runs, 1) }
    pub(crate) fn copied(&self, items: usize) { self.add(|c| &c.copied, items) }
    fn add(&self, count: fn(&Counts) -> &AtomicUsize, n: usize) { if let Some(c) = &self.0 { count(c).fetch_add(n, Ordering::Relaxed); } }
}

/// Records that `function`, at `line` and `column`, computed `items` items of pending arrays: its arguments, or only the items it read from them.
pub(crate) fn forced(function: String, line: usize, column: usize, items: usize) {
    record(Kind::Forced, || vec![Field::Text(function), Field::Count(line), Field::Count(column), Field::Count(items)])
}

/// Counts `items` items that a pending array computed. Pending arrays compute on the thread that reads them, so the count stays on that thread.
pub(crate) fn computed(items: usize) { COMPUTED.with(|c| c.set(c.get() + items)) }

/// The items that pending arrays have computed on this thread so far. A call's `forced` row takes the difference across the call.
pub(crate) fn computed_so_far() -> usize { COMPUTED.with(Cell::get) }

/// Records that `function` computed the fast combination named `name`.
pub(crate) fn fast(name: &'static str, function: String) { record(Kind::Fast, || vec![Field::Text(name.into()), Field::Text(function)]) }

/// Records an accumulation that added `items` items into a result of `size` items, each at a position that a table of offsets or ids gives, as scattering adds them. `path` names how it added them, as `pervasive::Route` names a scatter's paths, or `"rounds"`, `"one call"` or `"item by item"` for the general paths.
pub(crate) fn accumulated(items: usize, size: usize, path: &'static str) { record(Kind::Accumulations, || vec![Field::Count(items), Field::Count(size), Field::Text(path.into())]) }

impl Events {
    /// `•explain`'s record: the expression's `value`, then a record for each of `KINDS`, holding its columns, with one row for each event.
    pub(crate) fn report(self, value: Value, span: &crate::Span) -> Result<Value, Error> {
        let record = |names: Vec<&str>, values: Vec<Value>| keyed::record(names.into_iter().map(Arc::from).collect(), values).error_at(span, "invalid explain record");
        let mut tables = vec![value];
        for (&(_, columns), rows) in KINDS.iter().zip(&self.rows) {
            let n = rows.len();
            let values = columns.iter().enumerate().map(|(c, &(_, column))| match column {
                Column::Text => keyed::texts(&[n], rows.iter().map(|row| keyed::text(row[c].text())).collect()).error_at(span, "invalid explain texts"),
                Column::Count => Value::integers(vec![n], rows.iter().map(|row| row[c].count() as i64).collect()).error_at(span, "invalid explain counts"),
            }).collect::<Result<_, _>>()?;
            tables.push(record(columns.iter().map(|&(name, _)| name).collect(), values)?);
        }
        record(std::iter::once("value").chain(KINDS.iter().map(|&(name, _)| name)).collect(), tables)
    }

    /// Adds `other`'s events after these, as `finish` gives an enclosing `•explain` the events of one inside it.
    fn extend(&mut self, other: &Events) {
        for (rows, more) in self.rows.iter_mut().zip(&other.rows) { rows.extend(more.iter().cloned()); }
    }
}

impl Field {
    /// The text in a text column's cell.
    fn text(&self) -> &str { match self { Field::Text(t) => t, Field::Count(_) => unreachable!("a text column holds text") } }
    /// The count in a count column's cell.
    fn count(&self) -> usize { match *self { Field::Count(n) => n, Field::Text(_) => unreachable!("a count column holds counts") } }
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
