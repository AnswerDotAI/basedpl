//! The events that `•explain` reports about a run: the fused groups that ran, with the blocks, runs and copies they made, the pending arrays that fused groups read only after computing them on their own, the pending arrays that calls forced, the fast combinations taken, the accumulations, which add items at scattered positions, with the path each took, the places where a batched run's calls fell back to slower code, the writes that left a compact array in mixed storage, and the routes that handled an array's items one at a time. Events are recorded only while `•explain` runs. Fusion runs its graphs without a session, so every event goes to a sink on the thread that runs the expression.
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
/// - `fallbacks`: each batched map that fell back to calling its operand once for each cell, with the reason and the number of cells it concerns.
/// - `mixed`: each write that left a compact array in mixed storage, with the code of the call that wrote, its line and column, the array's kind before the write, the kind written, and the array's item count.
/// - `itemwise`: each route that handled an array's items one at a time where whole-array code covers most calls, with the innermost call that ran it, that call's line and column, the route, and the items it handled. A route outside any call counts for the `•explain` call itself.
const KINDS: [(&str, &[(&str, Column)]); 8] = {
    use Column::*;
    [
        ("fused", &[("glyphs", Text), ("items", Count), ("blocks", Count), ("runs", Count), ("copied", Count)]),
        ("stored", &[("glyphs", Text), ("items", Count), ("reason", Text)]),
        ("forced", &[("function", Text), ("line", Count), ("column", Count), ("items", Count)]),
        ("fast", &[("combination", Text), ("function", Text)]),
        ("accumulations", &[("items", Count), ("size", Count), ("path", Text)]),
        ("fallbacks", &[("reason", Text), ("items", Count)]),
        ("mixed", &[("code", Text), ("line", Count), ("column", Count), ("target", Text), ("written", Text), ("items", Count)]),
        ("itemwise", &[("function", Text), ("line", Count), ("column", Count), ("route", Text), ("items", Count)]),
    ]
};

/// A kind of event, by its position in `KINDS`.
#[derive(Clone, Copy)]
enum Kind { Fused, Stored, Forced, Fast, Accumulations, Fallbacks, Mixed, Itemwise }

/// The kinds whose events mark a slow path, each with the column that names its event: a pending array that a fused group computed on its own (its reason), a call that computed pending arrays' items (its function), a batched map's fallback (its reason), a write that left a compact array mixed (its code), and a route that handled items one at a time (its route).
const SLOW: [(Kind, usize); 5] = [(Kind::Stored, 2), (Kind::Forced, 0), (Kind::Fallbacks, 0), (Kind::Mixed, 0), (Kind::Itemwise, 3)];

/// The events of one `•explain`: for each of `KINDS`, one row for each event. `unclaimed` holds the routes that have handled items one at a time since the innermost call that reports began (`itemwise`), with the items each handled, until a call claims them as `itemwise` rows (`claim_itemwise`).
#[derive(Clone, Default)]
pub(crate) struct Events { rows: [Vec<Vec<Field>>; KINDS.len()], unclaimed: Vec<(&'static str, usize)> }

/// Runs `run` as `•explain` runs its text: recording events, inside any `•explain` that encloses it. Gives what `run` gives and the events it recorded, which an enclosing `•explain` gets too, because they happened during its run. `•explain` and the speed check both run through it.
pub(crate) fn explained<T>(run: impl FnOnce() -> T) -> (T, Events) {
    let mut under_way = Run::start();
    let result = run();
    (result, under_way.end())
}

/// An `•explain` run under way on this thread, from `start` to `end`. Dropping it before `end`, as a panic in the run does, ends it too. A panic then leaves the count of runs (`EXPLAINING`) and this thread's events (`EVENTS`) as they were before the run, which matters because Python turns a panic into an exception and carries on.
struct Run { outer: Option<Events>, ended: bool }

impl Run {
    /// Counts a run under way, and gives this thread empty events to record into, keeping the enclosing run's.
    fn start() -> Self {
        EXPLAINING.fetch_add(1, Ordering::Relaxed);
        Self { outer: EVENTS.with(|e| e.replace(Some(Events::default()))), ended: false }
    }
    /// Lowers the count of runs under way, and gives the events this run recorded after adding them to the enclosing run's, which this thread records into again.
    fn end(&mut self) -> Events {
        self.ended = true;
        EXPLAINING.fetch_sub(1, Ordering::Relaxed);
        EVENTS.with(|e| {
            let events = e.replace(None).unwrap_or_default();
            if let Some(mut outer) = self.outer.take() {
                outer.extend(&events);
                e.replace(Some(outer));
            }
            events
        })
    }
}

impl Drop for Run { fn drop(&mut self) { if !self.ended { self.end(); } } }

/// The `•explain` runs under way on any thread. The count is process-wide so that `reporting`, which every call reads, costs one plain atomic load and no thread-local lookup. While any thread's `•explain` runs, calls on every other thread take the slower reporting path too, and record nothing.
static EXPLAINING: AtomicUsize = AtomicUsize::new(0);

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
pub(crate) fn recording() -> bool { EVENTS.with(|e| e.borrow().is_some()) }
/// Whether calls report the pending arrays they compute: under `•prefs ["fusion":$t]`, or while an `•explain` runs on any thread. A call on another thread then records nothing (`recording`).
#[inline]
pub(crate) fn reporting() -> bool { crate::settings::reports_fusion() || EXPLAINING.load(Ordering::Relaxed) > 0 }

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

/// Runs `f` without counting the items that pending arrays compute inside it (`computed`). A sample that a decision reads ends no fusion.
pub(crate) fn uncounted<T>(f: impl FnOnce() -> T) -> T {
    let before = computed_so_far();
    let result = f();
    COMPUTED.with(|c| c.set(before));
    result
}

/// Records that `function` computed the fast combination named `name`.
pub(crate) fn fast(name: &'static str, function: String) { record(Kind::Fast, || vec![Field::Text(name.into()), Field::Text(function)]) }

/// Records an accumulation that added `items` items into a result of `size` items, each at a position that a table of offsets or ids gives, as scattering adds them. `path` names how it added them, as `pervasive::Route` names a scatter's paths, or `"rounds"`, `"one call"` or `"item by item"` for the general paths.
pub(crate) fn accumulated(items: usize, size: usize, path: &'static str) { record(Kind::Accumulations, || vec![Field::Count(items), Field::Count(size), Field::Text(path.into())]) }

/// Records that a batched run's calls fell back to slower code, with `reason` and the number of `items` it concerns. The reason becomes text only while `•explain` records. A stop gives its cause (`transform::Stop`).
pub(crate) fn fallback(reason: impl std::fmt::Display, items: usize) { record(Kind::Fallbacks, || vec![Field::Text(reason.to_string()), Field::Count(items)]) }

/// Records a write by the call at `span` that left a compact array of `items` items in mixed storage, with the array's kind before the write and the kind written, as `•storage` names them.
pub(crate) fn mixed(span: &crate::Span, target: &'static str, written: &'static str, items: usize) {
    record(Kind::Mixed, || {
        let (line, column) = span.position();
        vec![Field::Text(span.text().into()), Field::Count(line), Field::Count(column), Field::Text(target.into()), Field::Text(written.into()), Field::Count(items)]
    })
}

/// Counts `items` items that `route` handled one at a time, where whole-array code covers most calls, while an `•explain` records on this thread. The innermost call that reports claims the count (`claim_itemwise`).
pub(crate) fn itemwise(route: &'static str, items: usize) {
    EVENTS.with(|e| if let Some(events) = e.borrow_mut().as_mut() {
        events.unclaimed.push((route, items))
    })
}

/// Where a call's `itemwise` counts begin, while an `•explain` records on this thread.
pub(crate) fn itemwise_mark() -> Option<usize> { EVENTS.with(|e| e.borrow().as_ref().map(|events| events.unclaimed.len())) }

/// Records the `itemwise` counts made since `mark`, as `itemwise_mark` gave it, as rows for the call at `span`, whose code `function` gives.
pub(crate) fn claim_itemwise(mark: usize, function: impl FnOnce() -> String, span: &crate::Span) {
    EVENTS.with(|e| if let Some(events) = e.borrow_mut().as_mut() && events.unclaimed.len() > mark {
        let entries = events.unclaimed.drain(mark..).collect();
        events.claimed(entries, function(), span);
    })
}

/// `entries` with each route's items summed, in the order of each route's first entry.
fn by_route(entries: impl IntoIterator<Item = (&'static str, usize)>) -> Vec<(&'static str, usize)> {
    let mut routes: Vec<(&'static str, usize)> = Vec::new();
    for (route, items) in entries { match routes.iter_mut().find(|(r, _)| *r == route) { Some((_, n)) => *n += items, None => routes.push((route, items)) } }
    routes
}

impl Events {
    /// `•explain`'s record: the expression's `value`, then a record for each of `KINDS`, holding its columns, with one row for each event.
    pub(crate) fn report(mut self, value: Value, span: &crate::Span) -> Result<Value, Error> {
        let unclaimed = std::mem::take(&mut self.unclaimed);
        self.claimed(unclaimed, span.text().into(), span);
        let record = |names: Vec<&str>, values: Vec<Value>| keyed::vector(names.into_iter().map(Arc::from).collect(), values).error_at(span, "invalid explain record");
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
        self.unclaimed.extend(other.unclaimed.iter().copied());
    }

    /// Adds an `itemwise` row for each route in `entries`, with its items summed, for the call at `span` whose code is `function`.
    fn claimed(&mut self, entries: Vec<(&'static str, usize)>, function: String, span: &crate::Span) {
        let (line, column) = span.position();
        for (route, items) in by_route(entries) { self.rows[Kind::Itemwise as usize].push(vec![Field::Text(function.clone()), Field::Count(line), Field::Count(column), Field::Text(route.into()), Field::Count(items)]) }
    }

    /// The rows of the records that mark a slow path (`SLOW`), each as its record's name, the text of the column that names the event, and all its fields, with the routes that no call claimed as `itemwise` rows. The speed check expects none for a case beyond the events its `# allows` lines name.
    pub(crate) fn slow(&self) -> Vec<(&'static str, String, String)> {
        let fields = |row: &[Field]| row.iter().map(|f| match f { Field::Text(t) => t.clone(), Field::Count(n) => n.to_string() }).collect::<Vec<_>>().join(" ");
        let mut rows: Vec<_> = SLOW.iter().flat_map(|&(kind, key)| self.rows[kind as usize].iter().map(move |row| (KINDS[kind as usize].0, row[key].text().to_owned(), fields(row)))).collect();
        rows.extend(by_route(self.unclaimed.iter().copied()).into_iter().map(|(route, items)| ("itemwise", route.to_owned(), format!("{route} {items}"))));
        rows
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
