//! Execution diagnostics for `•explain`: fused groups, forced arrays, fast combinations, accumulations, slow paths and compilation. Successful routes, optional optimization declines and stopped traces share rustymath's aggregate event report. Every thread records into the process's active capture. A process supports one capture at a time except for nesting on one thread.
//!
//! The module also counts the live nodes that `•memory` reports: the record nodes of `∂` and the pending arrays. A node counts for the thread that made it, even when another thread drops it.
use crate::{keyed, Error, Described, Value};
use rustymath::events::{EventKind, Record, Report, Route};
use std::{any::Any, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, MutexGuard, PoisonError}};

/// The live nodes of each kind.
static MADE: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
/// The items that pending arrays have computed, which `forced` counts across one call.
static COMPUTED: AtomicUsize = AtomicUsize::new(0);

/// A cell in an event's row.
#[derive(Clone)]
enum Field { Text(String), Count(usize), Truth(bool) }

/// What a column holds.
#[derive(Clone, Copy)]
enum Column { Text, Count, Truth }

/// Each kind of event, in the order `•explain` reports them, with its columns:
/// - `fused`: each fused group that ran, with the glyphs of its calls in the order they ran, its item count, the blocks, runs and copied items that its `Tally` counted, and whether its stage ran compiled code.
/// - `stored`: each pending array that a fused group read as an input, computed on its own first, with the glyphs of its call, its item count, and why the group couldn't compute it.
/// - `forced`: each call that computed pending arrays' items, with the function, its line and column, and the number of items computed.
/// - `fast`: each fast combination taken, with its name and the function that computed it.
/// - `accumulations`: each accumulation that ran, with the items it added, the items of its result, and the path it took.
/// - `fallbacks`: each batched map that fell back to calling its operand once for each cell, with the reason and the number of cells it concerns.
/// - `mixed`: each write that left a compact array in mixed storage, with the code of the call that wrote, its line and column, the array's kind before the write, the kind written, and the array's item count.
/// - `itemwise`: each route that handled an array's items one at a time where whole-array code covers most calls, with the innermost call that ran it, that call's line and column, the route, and the items it handled. A route outside any call counts for the `•explain` call itself.
/// - `declined`: optional optimizations not taken, grouped by route and reason, with their total item counts.
/// - `programs`: each `∂` of a dfn that ran from a program, with `built` where its trace built the program first or `replayed` where it found one, the program's steps, how many of its block stages have compiled parts after the call (`compiled`), its block stages (`blocks`), and the program's listing, one line for each value (`program::call`).
/// - `compiles`: each compile of a block stage's kernel, with the glyphs of the stage's calls and the compile's time in microseconds (`program::blocks`).
/// - `routes`: successful fast paths, grouped by route, with call and item counts.
/// - `stopped`: interpreter fallbacks from a stopped trace, grouped by reason, including cached failures.
const KINDS: [(&str, &[(&str, Column)]); 13] = {
    use Column::*;
    [
        ("fused", &[("glyphs", Text), ("items", Count), ("blocks", Count), ("runs", Count), ("copied", Count), ("compiled", Truth)]),
        ("stored", &[("glyphs", Text), ("items", Count), ("reason", Text)]),
        ("forced", &[("function", Text), ("line", Count), ("column", Count), ("items", Count)]),
        ("fast", &[("combination", Text), ("function", Text)]),
        ("accumulations", &[("items", Count), ("size", Count), ("path", Text)]),
        ("fallbacks", &[("reason", Text), ("items", Count)]),
        ("mixed", &[("code", Text), ("line", Count), ("column", Count), ("target", Text), ("written", Text), ("items", Count)]),
        ("itemwise", &[("function", Text), ("line", Count), ("column", Count), ("route", Text), ("items", Count)]),
        ("declined", &[("route", Text), ("reason", Text), ("items", Count)]),
        ("programs", &[("event", Text), ("steps", Count), ("compiled", Count), ("blocks", Count), ("listing", Text)]),
        ("compiles", &[("glyphs", Text), ("microseconds", Count)]),
        ("routes", &[("route", Text), ("calls", Count), ("items", Count)]),
        ("stopped", &[("reason", Text), ("calls", Count), ("items", Count)]),
    ]
};

/// A kind of event, by its position in `KINDS`.
#[derive(Clone, Copy)]
enum Kind { Fused, Stored, Forced, Fast, Accumulations, Fallbacks, Mixed, Itemwise, Declined, Programs, #[cfg_attr(web, allow(dead_code))] Compiles, Routes, Stopped }

/// The kinds whose events mark a slow path, each with the column that names its event: a pending array that a fused group computed on its own (its reason), a call that computed pending arrays' items (its function), a batched map's fallback (its reason), a write that left a compact array mixed (its code), and a route that handled items one at a time (its route).
const SLOW: [(Kind, usize); 5] = [(Kind::Stored, 2), (Kind::Forced, 0), (Kind::Fallbacks, 0), (Kind::Mixed, 0), (Kind::Itemwise, 3)];

/// The events of one `•explain`: for each of `KINDS`, one row for each event. `unclaimed` holds the routes that have handled items one at a time since the innermost call that reports began (`itemwise`), with the items each handled, until a call claims them as `itemwise` rows (`claim_itemwise`).
#[derive(Clone, Default)]
pub(crate) struct Events { rows: [Vec<Vec<Field>>; KINDS.len()], unclaimed: Vec<(&'static str, usize)>, native: Report }

/// Runs `run` as `•explain` runs its text: recording events, inside any `•explain` that encloses it. Gives what `run` gives and the events it recorded, which an enclosing `•explain` gets too, because they happened during its run. `•explain` and the speed check both run through it.
pub(crate) fn explained<T>(run: impl FnOnce() -> T) -> (T, Events) {
    let sink = Arc::new(Store::default());
    let under_way = rustymath::events::Run::start(sink.clone());
    let result = run();
    let outer = under_way.outer().cloned();
    drop(under_way);
    let events = std::mem::take(&mut *sink.lock());
    if let Some(outer) = outer {
        if let Some(store) = Store::of(&*outer) { store.lock().extend(&events); }
        else { for event in &events.native.records { outer.record(event.clone()); } }
    }
    (result, events)
}

/// The sink of one `•explain` run (`rustymath::events`): its events, behind a lock because code on every thread records into it.
#[derive(Default)]
struct Store(Mutex<Events>);

impl Store {
    /// `sink` as an `•explain` run's store, if it is one.
    fn of(sink: &dyn rustymath::events::Sink) -> Option<&Store> { (sink as &dyn Any).downcast_ref() }
    fn lock(&self) -> MutexGuard<'_, Events> { self.0.lock().unwrap_or_else(PoisonError::into_inner) }
}

impl rustymath::events::Sink for Store {
    fn record(&self, event: Record) { self.lock().native.record(event) }
}

/// `f` on the events of the `•explain` under way, or `None` when none is.
fn store<T>(f: impl FnOnce(&mut Events) -> T) -> Option<T> { rustymath::events::with(|sink| Store::of(&**sink).map(|s| f(&mut s.lock()))).flatten() }

/// Adds the row that `row` gives to the events of `kind`, while an `•explain` records. `row` runs before the store's lock is taken.
fn record(kind: Kind, row: impl FnOnce() -> Vec<Field>) {
    if recording() { let row = row(); store(|events| events.rows[kind as usize].push(row)); }
}

/// Records a fused group of `items` items, whose calls have the glyphs `glyphs`, in the order they ran, with what `tally` counted while it ran and whether its stage ran compiled code.
pub(crate) fn fused(glyphs: impl FnOnce() -> String, items: usize, compiled: bool, tally: &Tally) {
    rustymath::events::taken(Route::Fusion, items);
    let Some(c) = &tally.0 else { return };
    let count = |n: &AtomicUsize| Field::Count(n.load(Ordering::Relaxed));
    record(Kind::Fused, || vec![Field::Text(glyphs()), Field::Count(items), count(&c.blocks), count(&c.runs), count(&c.copied), Field::Truth(compiled)])
}

/// Records a pending array that a fused group read as an input, computed on its own first. `event` gives the glyphs of its call, its item count and the reason the group couldn't compute it as one of its calls.
pub(crate) fn stored(event: impl FnOnce() -> (String, usize, &'static str)) {
    record(Kind::Stored, || { let (glyphs, items, reason) = event(); vec![Field::Text(glyphs), Field::Count(items), Field::Text(reason.into())] })
}

/// Whether an `•explain` is recording.
pub(crate) fn recording() -> bool { store(|_| ()).is_some() }
/// Whether calls report the pending arrays they compute: under `•prefs ["fusion":$t]`, or while an `•explain` runs (`rustymath::events::running`).
#[inline]
pub(crate) fn reporting() -> bool { crate::settings::reports_fusion() || rustymath::events::running() }

/// What a fused group's blocks did while `•explain` records: the blocks, the runs that they split into where an input repeats or cycles, and the items copied out of inputs before the kernels read them. Blocks run on worker threads, so each count is atomic. Outside `•explain` a tally holds no counts, so the threads share no counters.
pub(crate) struct Tally(Option<Counts>);

#[derive(Default)]
struct Counts { blocks: AtomicUsize, runs: AtomicUsize, copied: AtomicUsize }

impl Tally {
    /// A tally that counts while an `•explain` records, and otherwise counts nothing.
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

/// Counts `items` items that a pending array computed, on any thread.
pub(crate) fn computed(items: usize) { COMPUTED.fetch_add(items, Ordering::Relaxed); }

/// The items that pending arrays have computed so far. A call's `forced` row takes the difference across the call.
pub(crate) fn computed_so_far() -> usize { COMPUTED.load(Ordering::Relaxed) }

/// Runs `f` without counting the items that pending arrays compute inside it (`computed`). A sample that a decision reads ends no fusion.
pub(crate) fn uncounted<T>(f: impl FnOnce() -> T) -> T {
    let before = computed_so_far();
    let result = f();
    COMPUTED.store(before, Ordering::Relaxed);
    result
}

/// Records that `function` computed the fast combination named `name`.
pub(crate) fn fast(route: Route, function: String) {
    rustymath::events::taken(route, 0);
    fast_detail(route, function)
}
/// Adds function details for a route already counted by its numerical implementation.
pub(crate) fn fast_detail(route: Route, function: String) { record(Kind::Fast, || vec![Field::Text(route.name().into()), Field::Text(function)]) }

/// Records an accumulation that added `items` items into a result of `size` items, each at a position that a table of offsets or ids gives, as scattering adds them. `path` names how it added them, as `pervasive::Route` names a scatter's paths, or `"rounds"`, `"one call"` or `"item by item"` for the general paths.
pub(crate) fn accumulated(items: usize, size: usize, path: &'static str) { record(Kind::Accumulations, || vec![Field::Count(items), Field::Count(size), Field::Text(path.into())]) }

/// Records that a batched run's calls fell back to slower code, with `reason` and the number of `items` it concerns. The reason becomes text only while `•explain` records. A stop gives its cause (`transform::Stop`).
pub(crate) fn fallback(reason: impl std::fmt::Display, items: usize) { record(Kind::Fallbacks, || vec![Field::Text(reason.to_string()), Field::Count(items)]) }
/// Records a dfn call that ran from a program: `"built"` where its trace built the program first, and `"replayed"` where it found one, with the program's `steps`, its block stages (`blocks`), how many of them have compiled bodies after the call (`compiled`), and the program's `listing`, which is built only while `•explain` records.
pub(crate) fn program(event: &'static str, steps: usize, compiled: usize, blocks: usize, listing: impl FnOnce() -> String) {
    record(Kind::Programs, || vec![Field::Text(event.into()), Field::Count(steps), Field::Count(compiled), Field::Count(blocks), Field::Text(listing())])
}

/// Records the compile of a block stage's kernel, with the glyphs of the stage's calls that `glyphs` gives and the compile's `time`.
#[cfg(not(web))]
pub(crate) fn compiled(glyphs: impl FnOnce() -> String, time: std::time::Duration) { record(Kind::Compiles, || vec![Field::Text(glyphs()), Field::Count(time.as_micros() as usize)]) }

/// Records a write by the call at `span` that left a compact array of `items` items in mixed storage, with the array's kind before the write and the kind written, as `•storage` names them.
pub(crate) fn mixed(span: &crate::Span, target: &'static str, written: &'static str, items: usize) {
    record(Kind::Mixed, || {
        let (line, column) = span.position();
        vec![Field::Text(span.text().into()), Field::Count(line), Field::Count(column), Field::Text(target.into()), Field::Text(written.into()), Field::Count(items)]
    })
}

/// Counts `items` items that `route` handled one at a time, where whole-array code covers most calls, while an `•explain` records. The innermost call that reports claims the count (`claim_itemwise`).
pub(crate) fn itemwise(route: Route, items: usize) {
    rustymath::events::itemwise(route, items);
    store(|events| events.unclaimed.push((route.name(), items)));
}

/// Where a call's `itemwise` counts begin, while an `•explain` records.
pub(crate) fn itemwise_mark() -> Option<usize> { store(|events| events.unclaimed.len()) }

/// Records the `itemwise` counts made since `mark`, as `itemwise_mark` gave it, as rows for the call at `span`, whose code `function` gives.
pub(crate) fn claim_itemwise(mark: usize, function: impl FnOnce() -> String, span: &crate::Span) {
    store(|events| if events.unclaimed.len() > mark {
        let entries = events.unclaimed.drain(mark..).collect();
        events.claimed(entries, function(), span);
    });
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
        for event in &self.native.records {
            let (kind, row) = match event.kind {
                EventKind::Taken => (Kind::Routes, vec![Field::Text(event.route.name().into()), Field::Count(event.calls), Field::Count(event.items)]),
                EventKind::Declined => (Kind::Declined, vec![Field::Text(event.route.name().into()), Field::Text(event.reason.into()), Field::Count(event.items)]),
                EventKind::Stopped => (Kind::Stopped, vec![Field::Text(event.reason.into()), Field::Count(event.calls), Field::Count(event.items)]),
                EventKind::Itemwise => continue,
            };
            self.rows[kind as usize].push(row);
        }
        let unclaimed = std::mem::take(&mut self.unclaimed);
        self.claimed(unclaimed, span.text().into(), span);
        let record = |names: Vec<&str>, values: Vec<Value>| keyed::vector(names.into_iter().map(Arc::from).collect(), values).described("invalid explain record");
        let mut tables = vec![value];
        for (&(_, columns), rows) in KINDS.iter().zip(&self.rows) {
            let n = rows.len();
            let values = columns.iter().enumerate().map(|(c, &(_, column))| match column {
                Column::Text => keyed::texts(&[n], rows.iter().map(|row| keyed::text(row[c].text())).collect()).described("invalid explain texts"),
                Column::Count => Value::integers(vec![n], rows.iter().map(|row| row[c].count() as i64).collect()).described("invalid explain counts"),
                Column::Truth => Value::booleans(vec![n], rows.iter().map(|row| row[c].truth()).collect()).described("invalid explain truth values"),
            }).collect::<Result<_, _>>()?;
            tables.push(record(columns.iter().map(|&(name, _)| name).collect(), values)?);
        }
        record(std::iter::once("value").chain(KINDS.iter().map(|&(name, _)| name)).collect(), tables)
    }

    /// Adds `other`'s events after these, as `finish` gives an enclosing `•explain` the events of one inside it.
    fn extend(&mut self, other: &Events) {
        for (rows, more) in self.rows.iter_mut().zip(&other.rows) { rows.extend(more.iter().cloned()); }
        self.unclaimed.extend(other.unclaimed.iter().copied());
        self.native.extend(&other.native);
    }

    /// Adds an `itemwise` row for each route in `entries`, with its items summed, for the call at `span` whose code is `function`.
    fn claimed(&mut self, entries: Vec<(&'static str, usize)>, function: String, span: &crate::Span) {
        let (line, column) = span.position();
        for (route, items) in by_route(entries) { self.rows[Kind::Itemwise as usize].push(vec![Field::Text(function.clone()), Field::Count(line), Field::Count(column), Field::Text(route.into()), Field::Count(items)]) }
    }

    pub(crate) fn observed(&self) -> &Report { &self.native }

    /// Slow-path rows and optional declines, as kind, event name and details. Stopped traces count as slow paths. Test contracts can name a decline without requiring every declined optimization to succeed.
    pub(crate) fn slow(&self) -> Vec<(&'static str, String, String)> {
        let fields = |row: &[Field]| row.iter().map(|f| match f { Field::Text(t) => t.clone(), Field::Count(n) => n.to_string(), Field::Truth(t) => t.to_string() }).collect::<Vec<_>>().join(" ");
        let mut rows: Vec<_> = SLOW.iter().flat_map(|&(kind, key)| self.rows[kind as usize].iter().map(move |row| (KINDS[kind as usize].0, row[key].text().to_owned(), fields(row)))).collect();
        rows.extend(by_route(self.unclaimed.iter().copied()).into_iter().map(|(route, items)| ("itemwise", route.to_owned(), format!("{route} {items}"))));
        rows.extend(self.native.records.iter().filter_map(|r| match r.kind {
            EventKind::Declined => Some(("declined", format!("{}: {}", r.route.name(), r.reason), format!("{} {} {}", r.route.name(), r.reason, r.items))),
            EventKind::Stopped => Some(("stopped", r.reason.into(), format!("{} {}", r.reason, r.items))),
            EventKind::Taken | EventKind::Itemwise => None,
        }));
        rows
    }
}

impl Field {
    /// The text in a text column's cell.
    fn text(&self) -> &str { match self { Field::Text(t) => t, _ => unreachable!("a text column holds text") } }
    /// The count in a count column's cell.
    fn count(&self) -> usize { match *self { Field::Count(n) => n, _ => unreachable!("a count column holds counts") } }
    /// The truth value in a truth column's cell.
    fn truth(&self) -> bool { match *self { Field::Truth(t) => t, _ => unreachable!("a truth column holds truth values") } }
}

/// The kinds of node that `•memory` counts.
#[derive(Clone, Copy)]
pub(crate) enum Counted { Record, Pending }

/// One live node, counted until it's dropped.
pub(crate) struct Live(Counted);

impl Live {
    pub(crate) fn new(kind: Counted) -> Self { MADE[kind as usize].fetch_add(1, Ordering::Relaxed); Self(kind) }
}

impl Drop for Live {
    fn drop(&mut self) { MADE[self.0 as usize].fetch_sub(1, Ordering::Relaxed); }
}

/// The live nodes of `kind` in the process.
pub(crate) fn live(kind: Counted) -> usize { MADE[kind as usize].load(Ordering::Relaxed) }
