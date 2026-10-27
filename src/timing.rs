//! BPL measurements share reference records and setups. Numerical timing, copy units and retries live in Rustymath.
use crate::{host::timing_lock, protocol::response, reference::{self, File}, EvalOptions, Evaluation, InterruptHandle, Session, Source, Turn};
use rustymath::timing::{copy_time, CopyTime, NATIVE_COLUMN};
pub use rustymath::timing::{name_cpu, RETRY_PAUSES};
use serde_json::{json, Value as JsonValue};
use rustymath::hashing::HashMap;

/// The record and event named by each permitted or pending slow-path annotation.
#[derive(Default)]
pub struct Named { pub allowed: Vec<(String, String)>, pub gaps: Vec<(String, String)> }

fn fastest(mut f: impl FnMut()) -> f64 { f(); rustymath::timing::fastest(|_| f()) }

fn single_threaded_defaults() { crate::settings::reset(); rustymath::parallel::set_threads(1); }

/// A fresh session which runs preferences before setup, on one thread, with its random generator seeded at 0. Every timing of a case then draws the same data, in each process and on each build.
pub fn session(prefs: &str, setup: &str) -> Result<Turn, Evaluation> {
    single_threaded_defaults();
    let session = Session::new();
    let result = session.eval(&format!("•prefs [\"seed\":0]\n{prefs}\n{setup}"));
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

pub fn items(session: &Session) -> Option<usize> {
    let n = session.eval("n").value?.as_real()?;
    (n >= 0.0 && n.fract() == 0.0).then_some(n as usize)
}

/// Parse once, warm once, then take the fastest of five evaluations, forcing the result each time.
pub fn time(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<f64, Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let mut failed = None;
    let seconds = fastest(|| {
        let result = session.eval_parsed(&parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
        if let Some(value) = &result.value { value.force(); }
        if result.error.is_some() { failed.get_or_insert(result); }
    });
    failed.map_or(Ok(seconds), Err)
}

pub const WASM_COLUMN: usize = 2;

/// One measured record. `label` is single-line; `code` retains its original layout.
pub struct Case { pub file: String, pub section: String, pub code: String, pub label: String, pub budgets: Vec<String>, pub cost: f64, pub seconds: f64, pub copy: Option<CopyTime> }
impl Case {
    pub fn line(&self, column: Option<usize>) -> String { rustymath::timing::line(&self.section, &self.label, self.cost, column.and_then(|i| self.budgets.get(i)).map_or("-", String::as_str), self.seconds) }
    pub fn copy_line(&self) -> Option<String> { self.copy.as_ref().map(rustymath::timing::copy_line) }
}

pub fn rows(cases: &[Case], column: Option<usize>) -> JsonValue {
    cases.iter().map(|c| json!({"file":c.file, "section":c.section, "code":c.code, "label":c.label, "line":c.line(column), "copy":c.copy_line(), "budgets":c.budgets, "cost":c.cost, "seconds":c.seconds})).collect()
}

/// A case's label in logs: `file:id` for a case with an ID, or else its code on one line. A multi-line case's lines join with `⋄`, as comments write a dfn, so its label stays the same when lines above it move: `{x←v ⋄ x.[i]←w ⋄ x}0`.
fn label(file: &File, case: &JsonValue) -> String {
    let (id, code) = (case["id"].as_str().unwrap_or_default(), case["code"].as_str().unwrap());
    if !id.is_empty() { format!("{}:{id}", file.name) }
    else if code.contains('\n') { code.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ⋄ ").replace("{ ⋄ ", "{").replace(" ⋄ }", "}") }
    else { code.into() }
}

/// One setup owner and warmed session, replaced only when moving between sections.
struct Group<'a> { session: Turn, setup: &'a str, prefs: HashMap<&'a str, String>, current: &'a str, n: usize }
impl<'a> Group<'a> {
    fn new(file: &'a File, start: &str, first: &'a str) -> Result<Self, String> {
        let prefs: HashMap<_, _> = file.cases.iter().map(|case| (case["section"].as_str().unwrap(), [start, case["prefs"].as_str().unwrap_or_default()].join("\n"))).collect();
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

fn each_case<'a>(files: &'a [File], only: Option<&[&str]>, start: &str, mut visit: impl FnMut(&mut Group<'a>, &'a File, &'a JsonValue) -> Result<(), String>) -> Result<(), String> {
    for file in files {
        let wanted = |case: &&JsonValue| case["budgets"].is_array() && only.is_none_or(|only| only.iter().any(|&s| s == case["code"].as_str().unwrap() || s == case["id"].as_str().unwrap_or_default() || s == label(file, case)));
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
        let total = timed(section)?;
        let cost = match copy { Some(copy) => total / n as f64 / copy.seconds, None => total / timed("exact")? };
        result.push(Case { file: file.name.clone(), section: section.into(), code: code.into(), label: label(file, case), budgets: case["budgets"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect(), cost, seconds: total / n as f64, copy });
        Ok(())
    })?;
    Ok(result)
}

pub struct Route { pub file: String, pub section: String, pub code: String, pub label: String, pub slow: Vec<(&'static str, String, String)>, pub named: Named }

/// Warm each timed record, then explain a forced evaluation in the same session.
pub fn routes(files: &[File], only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<Vec<Route>, String> {
    let mut result = Vec::new();
    each_case(files, only, "", |group, file, case| {
        let (section, code) = (case["section"].as_str().unwrap(), case["code"].as_str().unwrap());
        let session = group.session(section)?;
        let parsed = session.parse(Source::new("<input>", code)).complete().map_err(|e| failure(code, Evaluation::failed(e)))?;
        let run = || {
            let evaluation = session.eval_parsed(&parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
            if let Some(value) = &evaluation.value { value.force(); }
            evaluation
        };
        let warm = run();
        if warm.error.is_some() { return Err(failure(code, warm)); }
        let (evaluation, events) = crate::explain::explained(run);
        if evaluation.error.is_some() { return Err(failure(code, evaluation)); }
        let named = |key: &str| case[key].as_array().into_iter().flatten().map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into())).collect();
        result.push(Route { file:file.name.clone(), section:section.into(), code:code.into(), label:label(file, case), slow:events.slow(), named:Named { allowed:named("allows"), gaps:named("gap") } });
        Ok(())
    })?;
    Ok(result)
}

#[cfg(not(web))]
pub fn measured(files: &[File], all: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath::timing::retried;
    let line = |c: &Case| c.line(NATIVE_COLUMN);
    let over = |c: &Case| NATIVE_COLUMN.and_then(|i| c.budgets.get(i)).and_then(|b| b.parse::<f64>().ok()).is_some_and(|b| c.cost > b);
    let interrupt = InterruptHandle::default();
    let mut first = costs(files, None, "", &interrupt)?;
    if all {
        let mut copies: Vec<String> = Vec::new();
        for copy in first.iter().filter_map(Case::copy_line) { if !copies.contains(&copy) { print(&copy); copies.push(copy); } }
        for c in &first { print(&line(c)); }
    }
    first.retain(over);
    let mut failed = Ok(());
    let over_budget = retried(first, |cases, pause| {
        for c in &cases { print(&format!("{}: over, timed again after {pause} s", line(c))); }
        let codes: Vec<&str> = cases.iter().map(|c| c.label.as_str()).collect();
        match costs(files, Some(&codes), "", &interrupt) {
            Ok(mut retimed) => {
                retimed.retain(|c| cases.iter().any(|f| (f.file.as_str(), f.section.as_str(), f.code.as_str()) == (c.file.as_str(), c.section.as_str(), c.code.as_str())));
                for c in &retimed { print(&line(c)); }
                retimed.into_iter().filter(over).collect()
            }
            Err(e) => { failed = Err(e); Vec::new() }
        }
    });
    failed?;
    Ok(over_budget.iter().map(line).collect())
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
            Ok(seconds) => costs.push(json!({"code":code, "seconds":seconds, "copies":seconds / n as f64 / copy.seconds})), Err(failed) => return Ok(response(failed))
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

