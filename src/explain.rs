//! BPL report rendering and cost counts. `events` owns its vocabulary; rustymath supplies the shared process-wide capture, including worker threads and captures nested on one thread.
//!
//! The module also counts the pending arrays alive in the process, which `•memory` reports.
use crate::events::{self, Column, Detail, Event, EventKind, Field, Report, Route};
use crate::{Described, Error, Value, keyed};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, atomic::{AtomicUsize, Ordering}};

/// The live pending arrays.
static PENDING: AtomicUsize = AtomicUsize::new(0);
/// The items that pending arrays have computed, which `forced` counts across one call.
static COMPUTED: AtomicUsize = AtomicUsize::new(0);
/// The routes that have handled items one at a time, with the items each handled, until a call claims them (`claim_itemwise`). The process holds one list, as it holds one event store.
static UNCLAIMED: Mutex<Vec<(Route, usize)>> = Mutex::new(Vec::new());

fn unclaimed() -> MutexGuard<'static, Vec<(Route, usize)>> { UNCLAIMED.lock().unwrap_or_else(PoisonError::into_inner) }

/// The events of one `•explain`, with the item-by-item counts that no call inside it claimed.
#[derive(Clone, Default)]
pub struct Events { report: Report, unclaimed: Vec<(Route, usize)> }

/// The sink of one `•explain` run, behind a lock because code on every thread records into it.
struct Store(Mutex<Report>);
impl Store { fn lock(&self) -> MutexGuard<'_, Report> { self.0.lock().unwrap_or_else(PoisonError::into_inner) } }
impl events::Sink for Store { fn record(&self, event: Event) { self.lock().record(event) } }

/// Runs `run` as `•explain` runs its text, recording events inside any `•explain` that encloses it. Gives what `run` gives and the events it recorded, which an enclosing `•explain` gets too, because they happened during its run. `•explain` and the speed check both run through it.
pub fn explained<T>(run: impl FnOnce() -> T) -> (T, Events) {
    let sink = Arc::new(Store(Mutex::default()));
    let mark = unclaimed().len();
    let capture = events::Run::start(sink.clone());
    let result = run();
    let outer = capture.outer().cloned();
    drop(capture);
    let report = std::mem::take(&mut *sink.lock());
    let pending = {
        let mut list = unclaimed();
        let mine = list[mark.min(list.len())..].to_vec();
        if outer.is_none() { list.truncate(mark); }
        mine
    };
    if let Some(outer) = outer { for event in &report.records { outer.record(event.clone()); } }
    (result, Events { report, unclaimed: pending })
}

/// Whether an `•explain` is recording.
#[inline]
pub(crate) fn recording() -> bool { events::running() }
/// Whether calls report the pending arrays they compute: under `•prefs ["fusion":$t]`, or while an `•explain` runs.
#[inline]
pub(crate) fn reporting() -> bool { crate::settings::reports_fusion() || events::running() }

/// Records a fused group of `items` items, whose calls have the glyphs `glyphs`, in the order they ran, with what `tally` counted while it ran.
pub(crate) fn fused(glyphs: impl FnOnce() -> String, items: usize, tally: &Tally) {
    events::taken(crate::events::routes::Fusion, items);
    let Some(c) = &tally.0 else { return };
    let count = |n: &AtomicUsize| n.load(Ordering::Relaxed);
    events::emit(|_| Detail::Fused { glyphs: glyphs(), items, blocks: count(&c.blocks), runs: count(&c.runs), copied: count(&c.copied) });
}

/// Records a pending array that a fused group read as an input, computed on its own first. `event` gives the glyphs of its call, its item count and the reason the group couldn't compute it as one of its calls.
pub(crate) fn stored(event: impl FnOnce() -> (String, usize, &'static str)) {
    events::emit(|_| {
        let (glyphs, items, reason) = event();
        Detail::Stored { glyphs, items, reason, partway: false }
    });
}

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
    events::emit(|_| Detail::Forced { function, line, column, items, reason: "", site: None });
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

/// Records that `function` computed the fast combination `route`.
pub(crate) fn fast(route: Route, function: String) { events::taken(route, 0); fast_detail(route, function) }
/// Adds function details for a route already counted by its numerical implementation.
pub(crate) fn fast_detail(route: Route, function: String) { events::emit(|_| Detail::Fast { rule: route.name(), function }) }

/// Records an accumulation that added `items` items into a result of `size` items, each at a position that a table of offsets or ids gives, as scattering adds them. `path` names how it added them, as `pervasive::Route` names a scatter's paths, or `"rounds"`, `"one call"` or `"item by item"` for the general paths.
pub(crate) fn accumulated(items: usize, size: usize, path: &'static str) { events::emit(|_| Detail::Accumulation { items, size, path }) }

/// Records that a batched run's calls fell back to slower code, with `reason` and the number of `items` it concerns. The reason becomes text only while `•explain` records. A stop gives its cause (`transform::Stop`).
pub(crate) fn fallback(reason: impl std::fmt::Display, items: usize) { events::emit(|_| Detail::Fallback { reason: reason.to_string(), items }) }

/// Records a dfn call that ran from a program: `"built"` where its trace built the program first, and `"replayed"` where it found one, with the program's `steps` and its `listing`, which is built only while `•explain` records.
pub(crate) fn program(event: &'static str, steps: usize, listing: impl FnOnce() -> String) {
    events::emit(|_| Detail::Program { event, steps, listing: listing() });
}

/// Records the compile of a block stage's kernel, with the glyphs of the stage's calls that `glyphs` gives and the compile's `time`.
#[cfg(not(web))]
pub(crate) fn compiled(glyphs: impl FnOnce() -> String, time: std::time::Duration) {
    events::emit(|_| Detail::Compile { glyphs: glyphs(), microseconds: time.as_micros() as usize });
}

/// Records a write by the call at `span` that left a compact array of `items` items in mixed storage, with the array's kind before the write and the kind written, as `•storage` names them.
pub(crate) fn mixed(span: &crate::Span, target: &'static str, written: &'static str, items: usize) {
    events::emit(|_| {
        let (line, column) = span.position();
        Detail::Mixed { code: span.text().into(), line, column, target, written, items }
    });
}

/// Counts `items` items that `route` handled one at a time, where whole-array code covers most calls, while an `•explain` records. The innermost call that reports claims the count (`claim_itemwise`).
pub(crate) fn itemwise(route: Route, items: usize) { if recording() { unclaimed().push((route, items)); } }

/// Where a call's `itemwise` counts begin, while an `•explain` records.
pub(crate) fn itemwise_mark() -> Option<usize> { recording().then(|| unclaimed().len()) }

/// Records the `itemwise` counts made since `mark`, as `itemwise_mark` gave it, as events for the call at `span`, whose code `function` gives.
pub(crate) fn claim_itemwise(mark: usize, function: impl FnOnce() -> String, span: &crate::Span) {
    let entries: Vec<_> = {
        let mut list = unclaimed();
        if list.len() <= mark { return; }
        list.drain(mark..).collect()
    };
    for event in claimed(entries, function(), span) { events::emit(move |_| event); }
}

/// An `itemwise` event for each route in `entries`, with its items summed in the order of each route's first entry, for the call at `span` whose code is `function`.
fn claimed(entries: Vec<(Route, usize)>, function: String, span: &crate::Span) -> impl Iterator<Item = Detail> {
    let (line, column) = span.position();
    by_route(entries).into_iter().map(move |(route, items)| Detail::Itemwise { function: function.clone(), line, column, route, items })
}

/// `entries` with each route's items summed, in the order of each route's first entry.
fn by_route(entries: impl IntoIterator<Item = (Route, usize)>) -> Vec<(Route, usize)> {
    let mut routes: Vec<(Route, usize)> = Vec::new();
    for (route, items) in entries {
        match routes.iter_mut().find(|(r, _)| *r == route) { Some((_, n)) => *n += items, None => routes.push((route, items)) }
    }
    routes
}

impl Events {
    /// `•explain`'s record: the expression's `value`, then a record for each of `events::TABLES`, holding its columns, with one row for each event. Counts that no call claimed count for the `•explain` call at `span`.
    pub fn report(mut self, value: Value, span: &crate::Span) -> Result<Value, Error> {
        for event in claimed(std::mem::take(&mut self.unclaimed), span.text().into(), span) { self.report.record(event.event()); }
        let record =
            |names: Vec<&str>, values: Vec<Value>| keyed::vector(names.into_iter().map(Arc::from).collect(), values).described("invalid explain record");
        let mut tables = vec![value];
        for &table in events::TABLES {
            let rows: Vec<_> = self.report.records.iter().filter(|event| event.table() == table).map(Event::fields).collect();
            let n = rows.len();
            let values = table.1.iter().enumerate().map(|(c, &(_, column))| match column {
                Column::Text => keyed::texts(&[n], rows.iter().map(|row| keyed::text(row[c].text())).collect()).described("invalid explain texts"),
                Column::Count => Value::integers(vec![n], rows.iter().map(|row| row[c].count() as i64).collect()).described("invalid explain counts"),
                Column::Truth => Value::booleans(vec![n], rows.iter().map(|row| row[c].truth()).collect()).described("invalid explain truth values"),
            }).collect::<Result<_, _>>()?;
            tables.push(record(table.1.iter().map(|&(name, _)| name).collect(), values)?);
        }
        record(std::iter::once("value").chain(events::TABLES.iter().map(|table| table.0)).collect(), tables)
    }

    pub fn observed(&self) -> &Report { &self.report }

    /// The events that mark a slow path (`Event::slow_name`), each as its table's name, the text that names the event and all its fields, with the routes that no call claimed as `itemwise` events. Each decline follows, named by its route and reason, as in `"fusion: an exact argument"`. The route check expects none for a case beyond the events its `⍝⍝⍝ allows` and `⍝⍝⍝ gap` lines name, and a stopped trace needs a `gap` line. A case needn't name a decline, because each one explains another event, but a line that names one must still find it.
    pub fn slow(&self) -> Vec<(&'static str, String, String)> {
        let text = |event: &Event| {
            event.fields().iter().map(|f| match f {
                Field::Text(t) => t.as_ref().to_owned(), Field::Count(n) => n.to_string(), Field::Truth(t) => t.to_string()
            }).collect::<Vec<_>>().join(" ")
        };
        let mut rows: Vec<_> = self.report.records.iter().filter_map(|event| Some((event.name(), event.slow_name()?.to_owned(), text(event)))).collect();
        rows.extend(
            by_route(self.unclaimed.iter().copied()).into_iter()
                .map(|(route, items)| ("itemwise", route.name().to_owned(), format!("{} {items}", route.name()))));
        rows.extend(self.report.records.iter().filter_map(|event| match event {
            Event::Route(r) if r.kind == EventKind::Declined => Some((event.name(), format!("{}: {}", r.route.name(), r.reason), text(event))), _ => None
        }));
        rows
    }
}

/// A live pending array, counted from when it's made until it's dropped.
pub(crate) struct Live(());

impl Live { pub(crate) fn new() -> Self { PENDING.fetch_add(1, Ordering::Relaxed); Self(()) } }

impl Drop for Live { fn drop(&mut self) { PENDING.fetch_sub(1, Ordering::Relaxed); } }

/// The live pending arrays.
pub(crate) fn live() -> usize { PENDING.load(Ordering::Relaxed) }
