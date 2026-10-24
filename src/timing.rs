//! The speed test's measurements, shared by `tests/speed.rs`, the browser build's `costs` and the worker's `timing` and `costs` requests, which serve `basedpl.bpltests.time_cases` and `speed_costs`. A time is the fastest of five evaluations of a case, after one to warm up. The case is parsed once, before them. Its unit is one copy of the `n` items in the data of the `[setup]` before the case's section, as `rustymath::timing::copy_time` gives it: this machine's stored copy time, or one measured once. `costs` and `request` hold the machine's timing lock exclusively while they run, as `rustymath::timing` describes.
use crate::{host::timing_lock, protocol::response, EvalOptions, Evaluation, InterruptHandle, Session, Source, Turn};
use rustymath::timing::{code_of, copy_time, header, sections, CopyTime, NATIVE_COLUMN};
pub use rustymath::timing::name_cpu;
use serde_json::{json, Value as JsonValue};
use rustymath::hashing::{HashMap, HashMapExt};

/// The slow-path events that a case's comment lines name, each as its record's name and its event's text. An `# allows RECORD "EVENT": principle` line names an event that a principle allows. A `# gap RECORD "EVENT": what should happen` line names an event that a faster route should remove, which the case tolerates until then. An event's text names it as `•explain`'s record does: the function for `forced`, the reason for `stored` and `fallbacks`, the code for `mixed`, the route for `itemwise`, and the route and reason for `declined`, as in `fusion: an exact argument`, with any `"` in it doubled. Each line applies to the next case line after it.
#[derive(Default)]
pub struct Named<'a> { pub allowed: Vec<(&'a str, String)>, pub gaps: Vec<(&'a str, String)> }

/// The events that each case's `# allows` and `# gap` lines name (`Named`), by section and code.
fn annotations(text: &str) -> HashMap<(&str, &str), Named<'_>> {
    let (mut section, mut pending, mut named) = ("", Named::default(), HashMap::new());
    for line in text.lines() {
        if let Some((name, _)) = header(line) { section = name; }
        else if let Some(rest) = line.strip_prefix("# allows ") { pending.allowed.push(event(rest)); }
        else if let Some(rest) = line.strip_prefix("# gap ") { pending.gaps.push(event(rest)); }
        else if !line.is_empty() && !line.starts_with('#') { named.insert((section, code_of(line)), std::mem::take(&mut pending)); }
    }
    named
}

/// The record and event text of an `# allows` or `# gap` line's `RECORD "EVENT": ...`.
fn event(rest: &str) -> (&str, String) {
    let (record, quoted) = rest.split_once(' ').unwrap_or((rest, ""));
    let (mut event, mut chars) = (String::new(), quoted.trim_start().strip_prefix('"').unwrap_or_default().chars().peekable());
    while let Some(c) = chars.next() {
        if c == '"' && chars.next_if_eq(&'"').is_none() { break; }
        event.push(c);
    }
    (record, event)
}

/// The time of the fastest of `rustymath::timing::fastest`'s calls of `f`, in seconds, after one call to warm up.
fn fastest(mut f: impl FnMut()) -> f64 {
    f();
    rustymath::timing::fastest(|_| f())
}

/// Puts the default settings in force, on one thread, as speed budgets are for one thread.
fn single_threaded_defaults() {
    crate::settings::reset();
    rustymath::parallel::set_threads(1);
}

/// A session that has run `prefs`, then `setup`, or the evaluation that failed. It starts from `single_threaded_defaults`.
pub fn session(prefs: &str, setup: &str) -> Result<Turn, Evaluation> {
    single_threaded_defaults();
    let session = Session::new();
    let result = session.eval(&format!("{prefs}\n{setup}"));
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

/// The number of items in the setup's data: the value of `n` in `session`.
pub fn items(session: &Session) -> Option<usize> {
    let n = session.eval("n").value?.as_number()?.to_float().ok()?;
    (n >= 0.0 && n.fract() == 0.0).then_some(n as usize)
}

/// The time of `code` in `session`, in seconds, or the evaluation that failed.
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

/// The column of a case's wasm budget in basedpl's `tests/speed.txt`, after the macOS and Linux budgets that `rustymath::timing::NATIVE_COLUMN` picks from.
pub const WASM_COLUMN: usize = 2;

/// One case of a file in `speed.txt`'s format, with its cost: the section it's in, its code, its budgets in the file's order, the cost those budgets limit, its time for each of the section's `n` items in seconds, and the copy that its cost counts, which `[widths]` costs don't use.
pub struct Case<'a> { pub section: &'a str, pub code: &'a str, pub budgets: Vec<&'a str>, pub cost: f64, pub seconds: f64, pub copy: Option<CopyTime> }
impl Case<'_> {
    /// This case's line in a speed test's log, with the budget in `column` of its budgets: `rustymath::timing::NATIVE_COLUMN` for this platform's native budget, or `WASM_COLUMN`. `basedpl.bpltests.logged_costs` reads these lines.
    pub fn line(&self, column: usize) -> String { rustymath::timing::line(self.section, self.code, self.cost, self.budgets.get(column).copied().unwrap_or("-"), self.seconds) }
    /// The copy line of the copy that this case's cost counts, as `rustymath::timing::copy_line` writes it. `None` in `[widths]`.
    pub fn copy_line(&self) -> Option<String> { self.copy.as_ref().map(rustymath::timing::copy_line) }
}

/// Each of `cases` as JSON, as the worker's `costs` reply and the browser build's `costs` give it: its section, code, log `line` with the budget in `column` of its budgets, `copy` line or null, budgets, cost, and time for each item in `seconds`.
pub fn rows(cases: &[Case], column: usize) -> JsonValue {
    cases.iter().map(|c| json!({"section": c.section, "code": c.code, "line": c.line(column), "copy": c.copy_line(), "budgets": c.budgets, "cost": c.cost, "seconds": c.seconds})).collect()
}

/// One group of sections, which share the group's `[setup]` and one session, set up for one section at a time: each section's `•prefs` header, the section the session is set up for, and the setup's `n`.
struct Group<'a> { session: Turn, setup: String, prefs: HashMap<&'a str, String>, current: &'a str, n: usize }

impl<'a> Group<'a> {
    /// The group, with its session set up for section `first`.
    fn new(setup: String, prefs: HashMap<&'a str, String>, first: &'a str) -> Result<Self, String> {
        let header = &prefs[first];
        let session = session(header, &setup).map_err(|failed| failure(header, failed))?;
        let n = items(&session).ok_or("each setup must define n, its number of items")?;
        Ok(Self { session, setup, prefs, current: first, n })
    }
    /// The session, set up for section `name`, with the settings reset, on one thread, and the section's header applied, so that no other section's settings stay in force. Moving to another section sets the session up again: a fresh session runs that section's header and then the group's setup.
    fn session(&mut self, name: &'a str) -> Result<&Session, String> {
        let header = self.prefs.get(name).ok_or_else(|| format!("speed's {name} section is missing"))?;
        if self.current != name {
            self.session = session(header, &self.setup).map_err(|failed| failure(header, failed))?;
            self.current = name;
        }
        single_threaded_defaults();
        self.session.eval(header);
        Ok(&self.session)
    }
}

/// A failed evaluation of `code`, as the speed checks report it.
fn failure(code: &str, failed: Evaluation) -> String { format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default()) }

/// Calls `visit` with the sessions of each case's group, the case's section and its line, for every case in `text`, in `speed.txt`'s format. The first section is `[setup]`. Each later `[setup]` starts a new group of sections. Each section's cases run in a session that has run `start`, the section's prefs and then the `[setup]` before it. A group has one session, set up for one section at a time: moving to another section sets it up again, in a fresh session. `only`, when given, lists the codes of the cases to visit. A group then sets up only the sections that hold those cases, plus `[exact]` for a `[widths]` case.
fn each_case<'a>(text: &'a str, only: Option<&[&str]>, start: &str, mut visit: impl FnMut(&mut Group<'a>, &'a str, &'a str) -> Result<(), String>) -> Result<(), String> {
    let wanted = |line: &str| only.is_none_or(|only| only.contains(&code_of(line)));
    for group in sections(text)?.chunk_by(|_, next| next.0 != "setup") {
        let [("setup", _, setup), cases @ ..] = group else { return Err("the first section must be [setup]".into()) };
        let mut used: Vec<&str> = cases.iter().filter(|(_, _, lines)| lines.iter().any(|l| wanted(l))).map(|s| s.0).collect();
        if used.contains(&"widths") { used.push("exact"); }
        if used.is_empty() { continue; }
        let prefs = cases.iter().filter(|s| used.contains(&s.0)).map(|&(name, header, _)| (name, [start, header].join("\n").trim().to_owned())).collect();
        let mut group = Group::new(setup.join("\n"), prefs, used[0])?;
        for &(section, _, ref lines) in cases { for &line in lines.iter().filter(|l| wanted(l)) { visit(&mut group, section, line)?; } }
    }
    Ok(())
}

/// Every case in `text`, in `speed.txt`'s format, with its cost, in the sessions that `each_case` gives. A cost is in copies of the group's `n` items, as `rustymath::timing::copy_time` gives them, except in `[widths]`. There it is the case's time as a fraction of its time in `[exact]`. `only`, when given, lists the codes of the cases to time. `prefs` runs before each section's header.
pub fn costs<'a>(text: &'a str, only: Option<&[&str]>, prefs: &str, interrupt: &InterruptHandle) -> Result<Vec<Case<'a>>, String> {
    let _lock = timing_lock()?;
    let mut result = Vec::new();
    each_case(text, only, prefs, |group, section, line| {
        let mut fields = line.split('\t');
        let code = fields.next().unwrap_or_default();
        let n = group.n;
        let copy = (section != "widths").then(|| copy_time::<f64>(n));
        let mut timed = |name: &'a str| time(group.session(name)?, code, interrupt).map_err(|failed| failure(code, failed));
        let total = timed(section)?;
        let cost = match copy { Some(copy) => total / n as f64 / copy.seconds, None => total / timed("exact")? };
        result.push(Case { section, code, budgets: fields.collect(), cost, seconds: total / n as f64, copy });
        Ok(())
    })?;
    Ok(result)
}

/// One case of a file in `speed.txt`'s format, with the slow-path events that `•explain` records for it (`explain::Events::slow`), each as its record's name and its fields, and the events that the case's comment lines name.
pub struct Route<'a> { pub section: &'a str, pub code: &'a str, pub slow: Vec<(&'static str, String, String)>, pub named: Named<'a> }

/// Every case in `text`, in `speed.txt`'s format, in the sessions that `each_case` gives, run once to warm up, as `time` does, and then once as `•explain` runs its text, computing its value. The warm-up computes the setup's pending arrays and records what a first call records, so the events are those of the run that the speed test times. `only`, when given, lists the codes of the cases to run.
pub fn routes<'a>(text: &'a str, only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<Vec<Route<'a>>, String> {
    let mut named = annotations(text);
    let mut result = Vec::new();
    each_case(text, only, "", |group, section, line| {
        let code = code_of(line);
        let session = group.session(section)?;
        let parsed = session.parse(Source::new("<input>", code)).complete().map_err(|e| failure(code, Evaluation::failed(e)))?;
        let options = || EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
        let run = |s: &Session| {
            let evaluation = s.eval_parsed(&parsed, options());
            if let Some(value) = &evaluation.value { value.force(); }
            evaluation
        };
        let warm = run(session);
        if warm.error.is_some() { return Err(failure(code, warm)); }
        let (evaluation, events) = crate::explain::explained(|| run(session));
        if evaluation.error.is_some() { return Err(failure(code, evaluation)); }
        result.push(Route { section, code, slow: events.slow(), named: named.remove(&(section, code)).unwrap_or_default() });
        Ok(())
    })?;
    Ok(result)
}

/// Times every case in `text` as `costs` does, then times each case over this machine's native budget again, after each pause in `rustymath::timing::retried`. `print` gets every first cost when `all` is set, and each retry's earlier and new costs. With `all`, the copy line of each size of data comes first. Each case's line gives its section, code, cost, native budget and time for each item, in the form that `basedpl.bpltests.logged_costs` reads. A case with no native budget is never over. Gives the lines of the cases still over budget after the last retry. `tests/speed.rs` and `bpl --costs` both measure with it.
#[cfg(not(web))]
pub fn measured(text: &str, all: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath::timing::{retried, NATIVE_COLUMN};
    fn budget<'a>(c: &Case<'a>) -> Option<&'a str> { c.budgets.get(NATIVE_COLUMN).copied() }
    let line = |c: &Case| c.line(NATIVE_COLUMN);
    let over = |c: &Case| budget(c).and_then(|b| b.parse::<f64>().ok()).is_some_and(|b| c.cost > b);
    let interrupt = InterruptHandle::default();
    let mut first = costs(text, None, "", &interrupt)?;
    if all {
        let mut copies: Vec<String> = Vec::new();
        for copy in first.iter().filter_map(Case::copy_line) { if !copies.contains(&copy) { print(&copy); copies.push(copy); } }
        for c in &first { print(&line(c)); }
    }
    first.retain(over);
    let mut failed = Ok(());
    let over_budget = retried(first, |cases, pause| {
        for c in &cases { print(&format!("{}: over, timed again after {pause} s", line(c))); }
        let codes: Vec<&str> = cases.iter().map(|c| c.code).collect();
        match costs(text, Some(&codes), "", &interrupt) {
            Ok(mut retimed) => {
                retimed.retain(|c| cases.iter().any(|f| (f.section, f.code) == (c.section, c.code)));
                for c in &retimed { print(&line(c)); }
                retimed.into_iter().filter(over).collect()
            }
            Err(e) => { failed = Err(e); Vec::new() }
        }
    });
    failed?;
    Ok(over_budget.iter().map(line).collect())
}

/// The reply to the worker's `timing` request, which gives `speed`, text in `speed.txt`'s format whose first section is `[setup]`, the `prefs` its session starts with, and the `cases` to time. The reply gives the setup's `n` and each case's time in seconds and in copies, or the evaluation that failed. An optional `threads` sets how many threads each case's rustymath calls split across, for a scaling check. Without it they run on the timing thread alone, as in the speed test.
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let _lock = timing_lock()?;
    let text = |key| spec[key].as_str().ok_or_else(|| format!("timing needs {key} as a string"));
    let cases = spec["cases"].as_array().ok_or("timing needs cases as an array")?;
    let sections = sections(text("speed")?)?;
    let setup = match sections.first() { Some(("setup", _, lines)) => lines.join("\n"), _ => return Err("speed's first section must be [setup]".into()) };
    let session = match session(text("prefs")?, &setup) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    if let Some(threads) = spec["threads"].as_u64() { rustymath::parallel::set_threads(threads as usize); }
    let n = items(&session).ok_or("the setup must define n, its number of items")?;
    let copy = copy_time::<f64>(n);
    let mut costs = Vec::new();
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        match time(&session, code, interrupt) {
            Ok(seconds) => costs.push(json!({"code": code, "seconds": seconds, "copies": seconds / n as f64 / copy.seconds})), Err(failed) => return Ok(response(failed))
        }
    }
    Ok(json!({"n": n, "costs": costs, "output": [], "error": null}))
}

/// The reply to the worker's `costs` request, which gives `speed`, text in `speed.txt`'s format, and optionally `cases`, a list of the codes to time, and `prefs`, BPL that runs before each section's header. The reply gives each case with its cost, as `costs` times them and `rows` encodes them, with this platform's native budget in each log line.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    Ok(json!({"costs": rows(&costs(spec["speed"].as_str().ok_or("costs needs speed as a string")?, only.as_deref(), spec["prefs"].as_str().unwrap_or_default(), interrupt)?, NATIVE_COLUMN), "error": null}))
}
