//! BPL measurements share reference records and setups. Numerical timing, copy units, footprint copies and retries live in Rustymath.
use crate::{EvalOptions, Evaluation, InterruptHandle, Session, Source, Turn, host::{Instant, timing_lock}, protocol::response, reference::{self, File}};
use rustymath::hashing::HashMap;
use rustymath::timing::{Budget, Cause, CopyTime, Footprint, Held, NATIVE_COLUMN, copy_time, footprint};
pub use rustymath::timing::{RETRY_PAUSES, checks_budgets, name_cpu};
use serde_json::{Value as JsonValue, json};
use std::time::Duration;

/// The record and event named by each permitted or pending slow-path annotation.
#[derive(Default)]
pub struct Named { pub routes: Vec<String>, pub allowed: Vec<(String, String)>, pub gaps: Vec<(String, String)> }

fn single_threaded_defaults() {
    crate::settings::reset();
    rustymath::parallel::set_threads(1);
}

/// A fresh session which runs preferences before setup, on one thread, with its random generator seeded at 0. Every timing of a case then draws the same data, in each process and on each build. The setup stops as `interrupt` stops.
pub fn session(prefs: &str, setup: &str, interrupt: &InterruptHandle) -> Result<Turn, Evaluation> {
    single_threaded_defaults();
    let session = Session::new();
    let result = session.eval_with(&format!("•prefs [\"seed\":0]\n{prefs}\n{setup}"), EvalOptions { interrupt: interrupt.clone(), ..EvalOptions::default() });
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

/// Starts a timing request's time limit once it holds the timing lock: it notes `started` on stderr, from which the worker's client counts the limit too (`basedpl.worker.Worker`), then sets the deadline.
fn started(interrupt: &InterruptHandle, limit: Option<Duration>) {
    if let Some(limit) = limit {
        eprintln!("started");
        interrupt.expire_after(limit);
    }
}

pub fn items(session: &Session) -> Result<Option<usize>, crate::Error> {
    let result = session.eval("n");
    if let Some(error) = result.error && error.kind.is_control() { return Err(error); }
    let Some(value) = result.value else { return Ok(None) };
    let value = value.read(crate::ReadKind::Count)?;
    session.raise()?;
    Ok(value.as_real().filter(|&n| n >= 0.0 && n.fract() == 0.0).map(|n| n as usize))
}

fn evaluated(session: &Session, parsed: &crate::Parsed, interrupt: &InterruptHandle) -> Evaluation {
    let mut result = session.eval_parsed(parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
    if result.error.is_none() && let Some(value) = result.value.take() {
        match value.read(crate::ReadKind::Value).and_then(|value| { session.raise()?; Ok(value) }) { Ok(value) => result.value = Some(value), Err(error) => result.error = Some(error) }
    }
    result
}
/// Parse once and force each evaluation. Return the fastest of five warm times, the first evaluation's time, peak large-allocation bytes during the warm runs, and bytes retained beyond the first evaluation. Parsing and setup are excluded.
pub fn time(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<(f64, f64, usize, usize), Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let mut failed = None;
    let mut run = || {
        let result = evaluated(session, &parsed, interrupt);
        if result.error.is_some() { failed.get_or_insert(result); }
    };
    let cold = Instant::now();
    run();
    let cold_seconds = cold.elapsed().as_secs_f64();
    let start = rustymath::memory::held();
    let (seconds, held) = rustymath::memory::peak_held(|| rustymath::timing::fastest(|_| run()));
    let left = rustymath::memory::held().saturating_sub(start);
    failed.map_or(Ok((seconds, cold_seconds, held, left)), Err)
}

/// The browser build's budget column: a case's third budget, after its macOS and Linux budgets.
pub const WASM_COLUMN: usize = 2;

/// One measured record. `label` is single-line; `code` retains its original layout. `holds` are the columns that the case's `held` and `busy` lines hold. `footprint` is the case's footprint, where it passes one core's cache, with a copy of it measured beside the case.
pub struct Case {
    pub file: String, pub section: String, pub code: String, pub label: String, pub budgets: Vec<String>, pub holds: Vec<Held>, pub cost: f64, pub seconds: f64,
    pub copy: Option<CopyTime>, pub footprint: Option<Footprint>
}
impl Case {
    /// The budget that the case meets in `column`, with any hold, as `rustymath::timing::Budget::of` gives it.
    pub fn budget(&self, column: Option<usize>) -> Option<Budget> { Budget::of(&self.section, &self.budgets, &self.holds, column) }
    pub fn line(&self, column: Option<usize>) -> String {
        rustymath::timing::line(&self.section, &self.label, self.cost, &self.budget(column).map_or("-".into(), |b| b.to_string()), self.seconds, self.footprint)
    }
    pub fn copy_line(&self) -> Option<String> { self.copy.as_ref().map(rustymath::timing::copy_line) }
}

/// Each case as JSON, with its budget in `column`: its `budget`, its `rule` budget and its `hold`, `held` or `busy`, where a line holds that column, and whether its cost is `over` the budget or its hold is `stale`, as `rustymath::timing::Budget` decides.
pub fn rows(cases: &[Case], column: Option<usize>) -> JsonValue {
    cases.iter().map(|c| {
        let b = c.budget(column);
        json!({"file":c.file, "section":c.section, "code":c.code, "label":c.label, "line":c.line(column), "copy":c.copy_line(), "budgets":c.budgets,
    "budget":b.map(|b| b.value), "rule":b.map(|b| b.rule), "hold":b.and_then(|b| b.cause).map(Cause::name),
    "over":b.is_some_and(|b| b.over(c.cost)), "stale":b.is_some_and(|b| b.stale(c.cost)),
    "cost":c.cost, "seconds":c.seconds, "footprint":c.footprint.map(|f| json!({"bytes":f.bytes, "seconds":f.seconds}))})
    }).collect()
}

/// A case's label in logs: `file:id` for a case with an ID, or else its code on one line. A multi-line case's lines join with `⋄`, as comments write a dfn, so its label stays the same when lines above it move: `{x←v ⋄ x.[i]←w ⋄ x}0`.
pub(crate) fn label(file: &File, case: &JsonValue) -> String {
    let (id, code) = (case["id"].as_str().unwrap_or_default(), case["code"].as_str().unwrap());
    if !id.is_empty() { format!("{}:{id}", file.name) }
    else if code.contains('\n') {
        code.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ⋄ ").replace("{ ⋄ ", "{").replace(" ⋄ }", "}")
    } else { code.into() }
}

/// One setup owner and warmed session, replaced only when moving between sections. Its setups stop as `interrupt` stops.
struct Group<'a> { session: Turn, setup: &'a str, prefs: HashMap<&'a str, String>, current: &'a str, n: usize, interrupt: &'a InterruptHandle }
impl<'a> Group<'a> {
    fn new(file: &'a File, start: &str, first: &'a str, interrupt: &'a InterruptHandle) -> Result<Self, String> {
        let prefs: HashMap<_, _> =
            file.cases.iter().map(|case| (case["section"].as_str().unwrap(), [start, case["prefs"].as_str().unwrap_or_default()].join("\n"))).collect();
        let session = session(&prefs[first], &file.setup, interrupt).map_err(|failed| failure(first, failed))?;
        let n = items(&session).map_err(|error| error.to_string())?.ok_or("each setup must define n, its number of items")?;
        Ok(Self { session, setup: &file.setup, prefs, current: first, n, interrupt })
    }
    fn session(&mut self, name: &'a str) -> Result<&Session, String> {
        let header = self.prefs.get(name).ok_or_else(|| format!("speed's {name} section is missing"))?;
        if self.current != name {
            self.session = session(header, self.setup, self.interrupt).map_err(|failed| failure(header, failed))?;
            self.current = name;
        }
        single_threaded_defaults();
        self.session.eval(header);
        Ok(&self.session)
    }
}

fn failure(code: &str, failed: Evaluation) -> String { format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default()) }

fn each_case<'a>(
    files: &'a [File], only: Option<&[&str]>, start: &str, interrupt: &'a InterruptHandle,
    mut visit: impl FnMut(&mut Group<'a>, &'a File, &'a JsonValue) -> Result<(), String>) -> Result<(), String> {
    for file in files {
        let wanted = |case: &&JsonValue| {
            case["budgets"].is_array() && only.is_none_or(|only| {
                only.iter().any(|&s| s == case["code"].as_str().unwrap() || s == case["id"].as_str().unwrap_or_default() || s == label(file, case))
            })
        };
        let mut cases = file.cases.iter().filter(wanted).peekable();
        let Some(first) = cases.peek() else { continue; };
        let mut group = Group::new(file, start, first["section"].as_str().unwrap(), interrupt)?;
        for case in cases { visit(&mut group, file, case)?; }
    }
    Ok(())
}

/// Time selected records in copies, or as widths/exact ratios. Check optional expectations once, outside timing. `limit` bounds the timing from when it holds the timing lock (`started`).
pub fn costs(files: &[File], only: Option<&[&str]>, prefs: &str, interrupt: &InterruptHandle, limit: Option<Duration>) -> Result<Vec<Case>, String> {
    let _lock = timing_lock()?;
    started(interrupt, limit);
    let mut result = Vec::new();
    each_case(files, only, prefs, interrupt, |group, file, case| {
        let (section, code) = (case["section"].as_str().unwrap(), case["code"].as_str().unwrap());
        if reference::checked(case) {
            let mut checked = case.clone();
            checked["prefs"] = json!(group.prefs[section]);
            reference::check_once(&checked, &file.setup, interrupt).map_err(|e| format!("{}: {}", label(file, case), e["message"]))?;
        }
        let n = group.n;
        let copy = (section != "widths").then(|| copy_time::<f64>(n));
        let mut timed = |name| time(group.session(name)?, code, interrupt).map_err(|failed| failure(code, failed));
        let (total, _, held, left) = timed(section)?;
        if rustymath::memory::checks() && left > 0 {
            return Err(format!("{}: the timed runs left {left} more bytes held in large allocations than the first run", label(file, case)));
        }
        let (cost, footprint) = match copy {
            Some(copy) => (total / n as f64 / copy.seconds, footprint::<f64>(n, held)), None => (total / timed("exact")?.0, None)
        };
        let mut holds = Vec::new();
        for held in case["held"].as_array().into_iter().flatten() {
            rustymath::timing::hold(held.as_str().unwrap(), &mut holds).expect("`reference::file` checks each line");
        }
        result.push(Case { file: file.name.clone(), section: section.into(), code: code.into(), label: label(file, case),
            budgets: case["budgets"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect(), holds, cost, seconds: total / n as f64, copy,
            footprint });
        Ok(())
    })?;
    Ok(result)
}

/// What one run of a case recorded: its slow-path events (`explain::Events::slow`), and rustymath's aggregate records of it (`events::Report`).
pub struct Observed { pub slow: Vec<(&'static str, String, String)>, pub report: rustymath::events::Report }

impl Observed {
    /// What `events` holds of one run.
    fn of(events: &crate::explain::Events) -> Self { Self { slow: events.slow(), report: events.observed().clone() } }
    /// Whether this run recorded a taken, declined or stopped route.
    pub fn exercised(&self, kind: &str, route: rustymath::events::Route) -> bool { self.report.routes().any(|e| e.kind.name() == kind && e.route == route) }
}

/// A case run once as `•explain` runs its text (`run`), after a warm-up run (`warmup`), with the routes, `allows` lines and `gap` lines that the case names.
pub struct Capture { pub file: String, pub section: String, pub code: String, pub label: String, pub run: Observed, pub warmup: Observed, pub named: Named }

impl Capture {
    /// Missing required routes, each with the run's declines, unexpected slow events, and annotations whose events disappeared.
    pub fn violations(&self) -> Vec<String> {
        let mut wrong = Vec::new();
        for required in &self.named.routes {
            use rustymath::events::EventKind::{Declined, Taken};
            if !self.run.report.routes().any(|r| r.kind == Taken && r.route.name() == required.as_str()) {
                let declines: Vec<_> = self.run.report.routes().filter(|r| r.kind == Declined).map(|r| format!("{}: {}", r.route.name(), r.reason)).collect();
                wrong.push(if declines.is_empty() { format!("required route {required} was not taken") }
                else { format!("required route {required} was not taken, with declines {}", declines.join("; ")) });
            }
        }
        let named: Vec<_> = self.named.allowed.iter().chain(&self.named.gaps).collect();
        for (record, event, fields) in &self.run.slow {
            let permitted = named.iter().any(|(a, e)| a.as_str() == *record && e == event);
            if *record != "declined" && !permitted { wrong.push(format!("{record} \"{event}\" ({fields})")); }
        }
        wrong.extend(
            named.iter().filter(|(a, e)| !self.run.slow.iter().any(|(record, event, _)| a.as_str() == *record && e == event))
                .map(|(a, e)| format!("{a} \"{e}\" no longer appears")));
        wrong
    }
}

/// Why `route` captured no run of a case: its evaluation failed, or it panicked. Each variant holds its message. A panic's message starts `route capture: panicked:`.
pub enum Uncaptured { Failed(String), Panicked(String) }

impl std::fmt::Display for Uncaptured {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { match self { Self::Failed(message) | Self::Panicked(message) => f.write_str(message) } }
}

/// Warm one case, then capture a forced evaluation in the same session (`warmed`). A case that panics gives `Uncaptured::Panicked` instead of unwinding into its caller.
pub fn route(session: &Session, case: &JsonValue, interrupt: &InterruptHandle) -> Result<Capture, Uncaptured> {
    match crate::reference::caught("route capture", || captured(session, case, interrupt)) {
        Ok(captured) => captured.map_err(Uncaptured::Failed),
        Err(panic) => Err(Uncaptured::Panicked(panic)),
    }
}

/// `captured`, a route of `file`'s `case`, named by `label` in its result and in any error.
pub(crate) fn named(captured: Result<Capture, String>, file: &File, case: &JsonValue) -> Result<Capture, String> {
    let mut capture = captured.map_err(|message| format!("{}: {message}", label(file, case)))?;
    capture.file = file.name.clone();
    capture.label = label(file, case);
    Ok(capture)
}

/// `route`'s capture of `case`.
fn captured(session: &Session, case: &JsonValue, interrupt: &InterruptHandle) -> Result<Capture, String> {
    let code = case["code"].as_str().unwrap();
    let (_, events, warmup) = warmed(session, code, false, interrupt).map_err(|failed| failure(code, failed))?;
    let named = |key: &str| case[key].as_array().into_iter().flatten().map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into())).collect();
    Ok(Capture { file: String::new(), section: case["section"].as_str().unwrap_or_default().into(), code: code.into(), label: code.into(),
        run: Observed::of(&events), warmup: Observed::of(&warmup), named: Named {
            routes: case["route"].as_array().into_iter().flatten().map(|s| s.as_str().unwrap().into()).collect(), allowed: named("allows"), gaps: named("gap") }
    })
}

/// `code` run twice in `session`, each run as `•explain` runs its text, with `site` columns where `sites` holds, and with its value forced: a warm-up, then the run that a route check reads. Gives that run's evaluation and events and the warm-up's events, or the evaluation that failed. Where this platform's costs decide which calls wait, both run under the Mac's costs instead (`fusion::Waits::Reference`): a route check tests the design's routes, so a directive observed on the Mac holds everywhere. A run with every call waiting, as the coverage matrix's, keeps that. The route test, `check_file` and the worker's `route` request (`route_request`) all capture through it.
fn warmed(
    session: &Session, code: &str, sites: bool, interrupt: &InterruptHandle
) -> Result<(Evaluation, crate::explain::Events, crate::explain::Events), Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let run = || evaluated(session, &parsed, interrupt);
    let (warm, warmup) = crate::explain::explained(sites, run);
    if warm.error.is_some() { return Err(warm); }
    let (evaluation, events) = crate::explain::explained(sites, run);
    if evaluation.error.is_some() { return Err(evaluation); }
    Ok((evaluation, events, warmup))
}

/// The worker's `route` request: `•explain`'s record, without its `value`, of the run of `spec`'s `code` that a route check reads (`warmed`), in a fresh session that runs `prefs` and `setup` first (`session`), with `site` columns where `sites` holds. `bpltests.explain_diff` and `route_diff` read it.
pub fn route_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let text = |key| spec[key].as_str().ok_or_else(|| format!("route needs {key} as a string"));
    let session = match session(text("prefs")?, text("setup")?, interrupt) {
        Ok(session) => session, Err(failed) => return response(failed).map_err(|error| error.to_string())
    };
    let (mut evaluation, events, _) = match warmed(&session, text("code")?, spec["sites"].as_bool().unwrap_or(false), interrupt) {
        Ok(runs) => runs, Err(failed) => return response(failed).map_err(|error| error.to_string())
    };
    evaluation.value = Some(events.records().map_err(|e| e.to_string())?);
    response(evaluation).map_err(|error| error.to_string())
}

/// Capture every timed case outside its timed loop, with its ordinary setup and size. Gives the captures and a message for each case that fails to evaluate.
pub fn routes(files: &[File], only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<(Vec<Capture>, Vec<String>), String> {
    let (mut result, mut failed) = (Vec::new(), Vec::new());
    each_case(files, only, "", interrupt, |group, file, case| {
        let session = group.session(case["section"].as_str().unwrap())?;
        let captured = route(session, case, interrupt).map_err(|uncaptured| uncaptured.to_string());
        match named(captured, file, case) {
            Ok(capture) => result.push(capture), Err(failure) => failed.push(failure),
        }
        Ok(())
    })?;
    Ok((result, failed))
}

/// Fails, listing them, on `wrong` and on each capture's violations (`Capture::violations`), named `[section] label`. First prints the gaps the captures tolerate (`cargo test -- --nocapture` shows them). The route tests in `tests/explain.rs` and `tests/routes.rs` end with it.
pub fn contracts(routes: &[Capture], mut wrong: Vec<String>) {
    let mut gaps = Vec::new();
    for r in routes {
        let case = format!("[{}] {}", r.section, r.label);
        wrong.extend(r.violations().into_iter().map(|message| format!("{case}: {message}")));
        gaps.extend(r.named.gaps.iter().map(|(record, event)| format!("{case}: {record} \"{event}\"")));
    }
    println!("gaps that route cases tolerate: {gaps:#?}");
    assert!(wrong.is_empty(), "route contracts: {wrong:#?}");
}

/// Times the cases in `files`, retimes each one over its native budget after each pause in `RETRY_PAUSES`, and gives the lines of those still over it. `print` gets the copy lines and every cost where `all` is set, each retry's lines, then each case still over its budget, marked `over`, and last each case whose gap's hold has outlived the gap, as `rustymath::timing::Budget::stale` decides from its first cost, marked `stale`.
#[cfg(not(web))]
pub fn measured(files: &[File], all: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath::timing::retried;
    let line = |c: &Case| c.line(NATIVE_COLUMN);
    let over = |c: &Case| c.budget(NATIVE_COLUMN).is_some_and(|b| b.over(c.cost));
    let interrupt = InterruptHandle::default();
    let mut first = costs(files, None, "", &interrupt, None)?;
    if all {
        let mut copies: Vec<String> = Vec::new();
        for copy in first.iter().filter_map(Case::copy_line) {
            if !copies.contains(&copy) {
                print(&copy);
                copies.push(copy);
            }
        }
        for c in &first { print(&line(c)); }
    }
    let stale: Vec<String> = first.iter().filter(|c| c.budget(NATIVE_COLUMN).is_some_and(|b| b.stale(c.cost))).map(line).collect();
    first.retain(over);
    let mut failed = Ok(());
    let over_budget = retried(first, |cases, pause| {
        for c in &cases { print(&format!("{}: over, timed again after {pause} s", line(c))); }
        let codes: Vec<&str> = cases.iter().map(|c| c.label.as_str()).collect();
        match costs(files, Some(&codes), "", &interrupt, None) {
            Ok(mut retimed) => {
                retimed.retain(|c| {
                    cases.iter().any(|f| (f.file.as_str(), f.section.as_str(), f.code.as_str()) == (c.file.as_str(), c.section.as_str(), c.code.as_str()))
                });
                for c in &retimed { print(&line(c)); }
                retimed.into_iter().filter(over).collect()
            }
            Err(e) => { failed = Err(e); Vec::new() }
        }
    });
    failed?;
    let over: Vec<String> = over_budget.iter().map(line).collect();
    for l in &over { print(&format!("{l}: over")); }
    for l in &stale { print(&format!("{l}: stale")); }
    Ok(over)
}

/// Ad hoc timing takes setup directly, not a synthetic benchmark file. `limit` bounds the request from when it holds the timing lock (`started`).
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle, limit: Option<Duration>) -> Result<JsonValue, String> {
    let _lock = timing_lock()?;
    started(interrupt, limit);
    let text = |key| spec[key].as_str().ok_or_else(|| format!("timing needs {key} as a string"));
    let cases = spec["cases"].as_array().ok_or("timing needs cases as an array")?;
    let session = match session(text("prefs")?, text("setup")?, interrupt) {
        Ok(session) => session, Err(failed) => return response(failed).map_err(|error| error.to_string())
    };
    if let Some(threads) = spec["threads"].as_u64() { rustymath::parallel::set_threads(threads as usize); }
    let n = items(&session).map_err(|error| error.to_string())?.ok_or("the setup must define n, its number of items")?;
    let copy = copy_time::<f64>(n);
    let mut costs = Vec::new();
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        match time(&session, code, interrupt) {
            Ok((seconds, cold_seconds, ..)) => costs.push(json!({"code":code, "seconds":seconds, "cold_seconds":cold_seconds, "copies":seconds / n as f64 / copy.seconds})),
            Err(failed) => return response(failed).map_err(|error| error.to_string()),
        }
    }
    Ok(json!({"n":n, "costs":costs, "output":[], "error":null}))
}

/// The worker's `costs` request: each cost of `spec`'s `files`, or of its `cases` alone, as `costs` times them. A deadline that `limit` sets, and that passes, gives a `TIMEOUT` error.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle, limit: Option<Duration>) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    let files = reference::sources(&spec["files"])?;
    match costs(&files, only.as_deref(), spec["prefs"].as_str().unwrap_or_default(), interrupt, limit) {
        Ok(cases) => Ok(json!({"costs":rows(&cases, NATIVE_COLUMN), "error":null})),
        Err(message) if interrupt.expired() => Ok(json!({"costs":[], "error":{"kind":"TIMEOUT", "message":message}})), Err(message) => Err(message)
    }
}
