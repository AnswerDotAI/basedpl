//! BPL rendering of the shared producer-event report, and live graph-node counts for `•memory`.
use crate::{Described, Error, Value, keyed};
use rustymath::events::{self, Column, Event, EventKind, Field, Report};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, atomic::{AtomicUsize, Ordering::Relaxed}};

static MADE: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

#[derive(Clone, Default)]
pub(crate) struct Events(Report);
struct Store { report: Mutex<Report>, sites: bool }
impl Store { fn lock(&self) -> MutexGuard<'_, Report> { self.report.lock().unwrap_or_else(PoisonError::into_inner) } }
impl events::Sink for Store {
    fn sites(&self) -> bool { self.sites }
    fn record(&self, event: Event) { self.lock().record(event); }
}

pub(crate) fn explained<T>(sites: bool, run: impl FnOnce() -> T) -> (T, Events) {
    let sink = Arc::new(Store { report: Mutex::default(), sites });
    let capture = events::Run::start(sink.clone());
    let result = run();
    let outer = capture.outer().cloned();
    drop(capture);
    let report = std::mem::take(&mut *sink.lock());
    if let Some(outer) = outer { for event in &report.records { outer.record(event.clone()); } }
    (result, Events(report))
}

#[inline]
pub(crate) fn recording() -> bool { events::running() }

pub(crate) struct Tally(Option<Counts>);
#[derive(Default)]
struct Counts { blocks: AtomicUsize, runs: AtomicUsize, copied: AtomicUsize }
impl Tally {
    pub(crate) fn new() -> Self { Self(recording().then(Counts::default)) }
    pub(crate) fn block(&self) { self.add(|c| &c.blocks, 1) }
    pub(crate) fn run(&self) { self.add(|c| &c.runs, 1) }
    pub(crate) fn copied(&self, items: usize) { self.add(|c| &c.copied, items) }
    fn add(&self, count: fn(&Counts) -> &AtomicUsize, n: usize) { if let Some(c) = &self.0 { count(c).fetch_add(n, Relaxed); } }
}
pub(crate) fn fused(glyphs: impl FnOnce() -> String, items: usize, tally: &Tally) {
    events::emit(|_| {
        let count = |f: fn(&Counts) -> &AtomicUsize| tally.0.as_ref().map_or(0, |c| f(c).load(Relaxed));
        Event::Fused { glyphs: glyphs(), items, blocks: count(|c| &c.blocks), runs: count(|c| &c.runs), copied: count(|c| &c.copied) }
    });
}
pub(crate) fn fast(rule: &'static str, function: impl FnOnce() -> String) { events::emit(|_| Event::Fast { rule, function: function() }); }
pub(crate) fn accumulated(items: usize, size: usize, path: &'static str) { events::emit(|_| Event::Accumulation { items, size, path }); }
pub(crate) fn fallback(reason: impl std::fmt::Display, items: usize) { events::emit(|_| Event::Fallback { reason: reason.to_string(), items }); }
pub(crate) fn program(event: &'static str, steps: usize, compiled: usize, blocks: usize, listing: impl FnOnce() -> String) {
    events::emit(|_| Event::Program { event, steps, compiled, blocks, listing: listing() });
}
#[cfg(not(web))]
pub(crate) fn compiled(glyphs: impl FnOnce() -> String, time: std::time::Duration) {
    events::emit(|_| Event::Compile { glyphs: glyphs(), microseconds: time.as_micros() as usize });
}
pub(crate) fn mixed(span: &crate::Span, target: &'static str, written: &'static str, items: usize) {
    events::emit(|_| {
        let (line, column) = span.position();
        Event::Mixed { code: span.text().into(), line, column, target, written, items }
    });
}

/// Captured only from an event builder that was asked for sites.
pub(crate) fn site() -> Option<String> {
    let trace = std::backtrace::Backtrace::force_capture().to_string();
    let named = trace.lines().filter_map(|line| line.trim_start().split_once(": ").filter(|(n, _)| n.bytes().all(|b| b.is_ascii_digit())).and_then(|(_, s)| frame(s)));
    let mut frames: Vec<String> = Vec::new();
    for name in named.skip_while(|name| ["explain::", "graph::Node::", "array::", "program::trace::"].iter().any(|p| name.starts_with(p))) {
        if frames.last() != Some(&name) { frames.push(name); }
        if frames.len() == 6 { break; }
    }
    Some(frames.join(" <- "))
}
fn frame(symbol: &str) -> Option<String> {
    let symbol = match symbol.strip_prefix('<') {
        Some(rest) => {
            let mut depth = 1usize;
            let close = rest.char_indices().find(|&(_, c)| { depth = match c { '<' => depth + 1, '>' => depth - 1, _ => depth }; depth == 0 })?.0;
            format!("{}{}", rest[..close].split(" as ").next()?, &rest[close + 1..])
        }
        None => symbol.to_owned(),
    };
    let own = symbol.strip_prefix("basedpl::")?;
    let (mut plain, mut depth) = (String::new(), 0usize);
    for c in own.chars() {
        match c { '<' | '{' => depth += 1, '>' | '}' => depth = depth.saturating_sub(1), _ if depth == 0 => plain.push(c), _ => {} }
    }
    let hash = |part: &&str| part.len() == 17 && part.starts_with('h') && part[1..].bytes().all(|b| b.is_ascii_hexdigit());
    Some(plain.split("::").filter(|part| !part.is_empty() && !hash(part)).collect::<Vec<_>>().join("::"))
}

fn explain_record(names: Vec<&str>, values: Vec<Value>) -> Result<Value, Error> {
    keyed::vector(names.into_iter().map(Arc::from).collect(), values).described("invalid explain record")
}
impl Events {
    pub(crate) fn report(self, value: Value) -> Result<Value, Error> {
        let mut tables = self.tables()?;
        tables.insert(0, value);
        explain_record(std::iter::once("value").chain(events::TABLES.iter().map(|&(name, _)| name)).collect(), tables)
    }
    pub(crate) fn records(self) -> Result<Value, Error> { explain_record(events::TABLES.iter().map(|&(name, _)| name).collect(), self.tables()?) }
    fn tables(&self) -> Result<Vec<Value>, Error> {
        events::TABLES.iter().enumerate().map(|(table, &(_, columns))| {
            let rows: Vec<_> = self.0.records.iter().filter(|event| event.table() == table).map(Event::fields).collect();
            let n = rows.len();
            let values = columns.iter().enumerate().map(|(c, &(_, column))| match column {
                Column::Text => keyed::texts(&[n], rows.iter().map(|row| keyed::text(row[c].text())).collect()).described("invalid explain texts"),
                Column::Count => Value::integers(vec![n], rows.iter().map(|row| row[c].count() as i64).collect()).described("invalid explain counts"),
                Column::Truth => Value::booleans(vec![n], rows.iter().map(|row| row[c].truth()).collect()).described("invalid explain truth values"),
            }).collect::<Result<_, _>>()?;
            explain_record(columns.iter().map(|&(name, _)| name).collect(), values)
        }).collect()
    }
    pub(crate) fn observed(&self) -> &Report { &self.0 }
    pub(crate) fn slow(&self) -> Vec<(&'static str, String, String)> {
        self.0.records.iter().filter(|event| event.slow() || matches!(event, Event::Route(r) if r.kind == EventKind::Declined)).map(|event| {
            let key = match event {
                Event::Stored { reason, .. } => (*reason).to_owned(), Event::Forced { function, .. } => function.clone(), Event::Fallback { reason, .. } => reason.clone(),
                Event::Route(r) if r.kind == EventKind::Declined => format!("{}: {}", r.route.name(), r.reason), Event::Route(r) => r.reason.to_owned(),
                _ => unreachable!("slow event"),
            };
            let fields = event.fields().iter().map(|field| match field {
                Field::Text(t) => (*t).to_owned(), Field::Count(n) => n.to_string(), Field::Truth(t) => t.to_string(),
            }).collect::<Vec<_>>().join(" ");
            (event.name(), key, fields)
        }).collect()
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Counted { Record, Pending }
pub(crate) struct Live(Counted);
impl Live { pub(crate) fn new(kind: Counted) -> Self { MADE[kind as usize].fetch_add(1, Relaxed); Self(kind) } }
impl Drop for Live { fn drop(&mut self) { MADE[self.0 as usize].fetch_sub(1, Relaxed); } }
pub(crate) fn live(kind: Counted) -> usize { MADE[kind as usize].load(Relaxed) }
