//! BPL measurements share reference records and setups. Numerical timing, copy units, footprint copies and retries live in Rustymath.
use crate::{EvalOptions, Evaluation, InterruptHandle, Session, Source, Turn, host::timing_lock, protocol::response, reference::{self, File}};
use rustymath::hashing::HashMap;
use rustymath::timing::{Budget, CopyTime, Footprint, Cause, Held, NATIVE_COLUMN, copy_time, footprint};
pub use rustymath::timing::{RETRY_PAUSES, checks_budgets, name_cpu};
use serde_json::{Value as JsonValue, json};

/// The record and event named by each permitted or pending slow-path annotation.
#[derive(Default)]
pub struct Named { pub routes: Vec<String>, pub allowed: Vec<(String, String)>, pub gaps: Vec<(String, String)> }

fn single_threaded_defaults() {
    crate::settings::reset();
    rustymath::parallel::set_threads(1);
}

/// A fresh session which runs preferences before setup, on one thread, with its random generator seeded at 0. Every timing of a case then draws the same data, in each process and on each build.
pub fn session(prefs: &str, setup: &str) -> Result<Turn, Evaluation> {
    single_threaded_defaults();
    let session = Session::new();
    let result = session.eval(&format!("•prefs [\"seed\":0]\n{prefs}\n{setup}"));
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

pub fn items(session: &Session) -> Option<usize> { let n = session.eval("n").value?.as_real()?; (n >= 0.0 && n.fract() == 0.0).then_some(n as usize) }

/// Parse once, warm once, then take the fastest of five evaluations, forcing the result each time. Gives that time, the most bytes that the five evaluations held at once, as `rustymath::memory::peak_held` counts them, and the bytes that they left held in large allocations beyond what the first evaluation left.
pub fn time(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<(f64, usize, usize), Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let mut failed = None;
    let mut run = || {
        let result = session.eval_parsed(&parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
        if let Some(value) = &result.value { value.force(); }
        if result.error.is_some() { failed.get_or_insert(result); }
    };
    run();
    let start = rustymath::memory::held();
    let (seconds, held) = rustymath::memory::peak_held(|| rustymath::timing::fastest(|_| run()));
    let left = rustymath::memory::held().saturating_sub(start);
    failed.map_or(Ok((seconds, held, left)), Err)
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
fn label(file: &File, case: &JsonValue) -> String {
    let (id, code) = (case["id"].as_str().unwrap_or_default(), case["code"].as_str().unwrap());
    if !id.is_empty() { format!("{}:{id}", file.name) }
    else if code.contains('\n') {
        code.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ⋄ ").replace("{ ⋄ ", "{").replace(" ⋄ }", "}")
    } else { code.into() }
}

/// One setup owner and warmed session, replaced only when moving between sections.
struct Group<'a> { session: Turn, setup: &'a str, prefs: HashMap<&'a str, String>, current: &'a str, n: usize }
impl<'a> Group<'a> {
    fn new(file: &'a File, start: &str, first: &'a str) -> Result<Self, String> {
        let prefs: HashMap<_, _> =
            file.cases.iter().map(|case| (case["section"].as_str().unwrap(), [start, case["prefs"].as_str().unwrap_or_default()].join("\n"))).collect();
        let session = session(&prefs[first], &file.setup).map_err(|failed| failure(first, failed))?;
        let n = items(&session).ok_or("each setup must define n, its number of items")?;
        Ok(Self { session, setup: &file.setup, prefs, current: first, n })
    }
    fn session(&mut self, name: &'a str) -> Result<&Session, String> {
        let header = self.prefs.get(name).ok_or_else(|| format!("speed's {name} section is missing"))?;
        if self.current != name {
            self.session = session(header, self.setup).map_err(|failed| failure(header, failed))?;
            self.current = name;
        }
        single_threaded_defaults();
        self.session.eval(header);
        Ok(&self.session)
    }
}

fn failure(code: &str, failed: Evaluation) -> String { format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default()) }

fn each_case<'a>(
    files: &'a [File], only: Option<&[&str]>, start: &str, mut visit: impl FnMut(&mut Group<'a>, &'a File, &'a JsonValue) -> Result<(), String>
) -> Result<(), String> {
    for file in files {
        let wanted = |case: &&JsonValue| {
            case["budgets"].is_array() && only.is_none_or(|only| {
                only.iter().any(|&s| s == case["code"].as_str().unwrap() || s == case["id"].as_str().unwrap_or_default() || s == label(file, case))
            })
        };
        let mut cases = file.cases.iter().filter(wanted).peekable();
        let Some(first) = cases.peek() else { continue; };
        let mut group = Group::new(file, start, first["section"].as_str().unwrap())?;
        for case in cases { visit(&mut group, file, case)?; }
    }
    Ok(())
}

/// Time selected records in copies, or as widths/exact ratios. Check optional expectations once, outside timing.
pub fn costs(files: &[File], only: Option<&[&str]>, prefs: &str, interrupt: &InterruptHandle) -> Result<Vec<Case>, String> {
    let _lock = timing_lock()?;
    let mut result = Vec::new();
    each_case(files, only, prefs, |group, file, case| {
        let (section, code) = (case["section"].as_str().unwrap(), case["code"].as_str().unwrap());
        if reference::checked(case) {
            let mut checked = case.clone();
            checked["prefs"] = json!(group.prefs[section]);
            reference::check_once(&checked, &file.setup, interrupt).map_err(|e| format!("{}: {}", label(file, case), e["message"]))?;
        }
        let n = group.n;
        let copy = (section != "widths").then(|| copy_time::<f64>(n));
        let mut timed = |name| time(group.session(name)?, code, interrupt).map_err(|failed| failure(code, failed));
        let (total, held, left) = timed(section)?;
        if rustymath::memory::checks() && left > 0 {
            return Err(format!("{}: the timed runs left {left} more bytes held in large allocations than the first run", label(file, case)));
        }
        let (cost, footprint) = match copy {
            Some(copy) => (total / n as f64 / copy.seconds, footprint::<f64>(n, held)),
            None => (total / timed("exact")?.0, None),
        };
        let mut holds = Vec::new();
        for held in case["held"].as_array().into_iter().flatten() { rustymath::timing::hold(held.as_str().unwrap(), &mut holds).expect("`reference::file` checks each line"); }
        result.push(Case { file: file.name.clone(), section: section.into(), code: code.into(), label: label(file, case),
            budgets: case["budgets"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect(), holds, cost, seconds: total / n as f64, copy, footprint });
        Ok(())
    })?;
    Ok(result)
}

/// What one run of a case recorded: its slow-path events (`explain::Events::slow`), and rustymath's aggregate records of it (`events::Report`).
pub struct Observed { pub slow: Vec<(&'static str, String, String)>, pub report: rustymath::events::Report }

impl Observed {
    /// What `events` holds of one run.
    fn of(events: &crate::explain::Events) -> Self { Self { slow: events.slow(), report: events.observed().clone() } }
    /// Whether this run recorded `route` as an event of `kind`: "taken", "declined" or "itemwise". An itemwise route appears in `slow`, as `•explain`'s own rows hold it.
    pub fn exercised(&self, kind: &str, route: rustymath::events::Route) -> bool {
        self.report.records.iter().any(|e| e.kind.name() == kind && e.route == route)
            || self.slow.iter().any(|(record, event, _)| *record == kind && event == route.name())
    }
}

/// A case run once as `•explain` runs its text (`run`), after a warm-up run (`warmup`), with the routes, `allows` lines and `gap` lines that the case names.
pub struct Capture { pub file: String, pub section: String, pub code: String, pub label: String, pub run: Observed, pub warmup: Observed, pub named: Named }

impl Capture {
    /// Missing required routes, each with the run's declines, unexpected slow events, and annotations whose events disappeared.
    pub fn violations(&self) -> Vec<String> {
        let mut wrong = Vec::new();
        for required in &self.named.routes {
            use rustymath::events::EventKind::{Declined, Taken};
            let records = self.run.report.records.iter();
            if !records.clone().any(|r| r.kind == Taken && r.route.name() == required.as_str()) {
                let declines: Vec<_> = records.filter(|r| r.kind == Declined).map(|r| format!("{}: {}", r.route.name(), r.reason)).collect();
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

/// Warm one case, then capture a forced evaluation in the same session.
pub fn route(session: &Session, case: &JsonValue, interrupt: &InterruptHandle) -> Result<Capture, String> {
    let code = case["code"].as_str().unwrap();
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(|e| failure(code, Evaluation::failed(e)))?;
    let run = || {
        let evaluation = session.eval_parsed(&parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
        if let Some(value) = &evaluation.value { value.force(); }
        evaluation
    };
    let (warm, warmup) = crate::explain::explained(run);
    if warm.error.is_some() { return Err(failure(code, warm)); }
    let (evaluation, events) = crate::explain::explained(run);
    if evaluation.error.is_some() { return Err(failure(code, evaluation)); }
    let named = |key: &str| case[key].as_array().into_iter().flatten().map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into())).collect();
    Ok(Capture { file: String::new(), section: case["section"].as_str().unwrap_or_default().into(), code: code.into(),
        label: case["id"].as_str().unwrap_or(code).into(), run: Observed::of(&events), warmup: Observed::of(&warmup), named: Named {
            routes: case["route"].as_array().into_iter().flatten().map(|s| s.as_str().unwrap().into()).collect(), allowed: named("allows"), gaps: named("gap") }
    })
}

/// Capture every timed case outside its timed loop, with its ordinary setup and size.
pub fn routes(files: &[File], only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<Vec<Capture>, String> {
    let mut result = Vec::new();
    each_case(files, only, "", |group, file, case| {
        let mut route = route(group.session(case["section"].as_str().unwrap())?, case, interrupt)?;
        route.file = file.name.clone();
        route.label = label(file, case);
        result.push(route);
        Ok(())
    })?;
    Ok(result)
}

/// Times the cases in `files`, retimes each one over its native budget after each pause in `RETRY_PAUSES`, and gives the lines of those still over it. `print` gets the copy lines and every cost where `all` is set, each retry's lines, then each case still over its budget, marked `over`, and last each case whose gap's hold has outlived the gap, as `rustymath::timing::Budget::stale` decides from its first cost, marked `stale`.
#[cfg(not(web))]
pub fn measured(files: &[File], all: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath::timing::retried;
    let line = |c: &Case| c.line(NATIVE_COLUMN);
    let over = |c: &Case| c.budget(NATIVE_COLUMN).is_some_and(|b| b.over(c.cost));
    let interrupt = InterruptHandle::default();
    let mut first = costs(files, None, "", &interrupt)?;
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
        match costs(files, Some(&codes), "", &interrupt) {
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

/// Ad hoc timing takes setup directly, not a synthetic benchmark file.
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let _lock = timing_lock()?;
    let text = |key| spec[key].as_str().ok_or_else(|| format!("timing needs {key} as a string"));
    let cases = spec["cases"].as_array().ok_or("timing needs cases as an array")?;
    let session = match session(text("prefs")?, text("setup")?) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    if let Some(threads) = spec["threads"].as_u64() { rustymath::parallel::set_threads(threads as usize); }
    let n = items(&session).ok_or("the setup must define n, its number of items")?;
    let copy = copy_time::<f64>(n);
    let mut costs = Vec::new();
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        match time(&session, code, interrupt) {
            Ok((seconds, ..)) => costs.push(json!({"code":code, "seconds":seconds, "copies":seconds / n as f64 / copy.seconds})),
            Err(failed) => return Ok(response(failed)),
        }
    }
    Ok(json!({"n":n, "costs":costs, "output":[], "error":null}))
}

pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    let files = reference::sources(&spec["files"])?;
    Ok(json!({"costs":rows(&costs(&files, only.as_deref(), spec["prefs"].as_str().unwrap_or_default(), interrupt)?, NATIVE_COLUMN), "error":null}))
}
