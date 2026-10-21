//! The speed test's measurements, shared by `tests/speed.rs`, the browser build's `costs` and the worker's `timing` and `costs` requests, which serve `basedpl.bpltests.time_cases` and `speed_costs`. A time is the fastest of five evaluations of a case, after one to warm up. The case is parsed once, before them. Its unit is one copy, as `copy` times it, of the `n` items in the data of the `[setup]` before the case's section. `costs` and `request` hold the machine's timing lock exclusively while they run, as `rustymath::timing` describes.
use crate::{host::{timing_lock, Instant}, protocol::response, EvalOptions, Evaluation, InterruptHandle, Session, Source};
use serde_json::{json, Value as JsonValue};
use std::{collections::HashMap, hint::black_box};

/// One section of a file in `speed.txt`'s format: its name, the prefs its session starts with, and its lines.
pub type Section<'a> = (&'a str, &'a str, Vec<&'a str>);

/// The sections of `text`, in `speed.txt`'s format: a line `[name]` or `[name] prefs` starts each section. Blank lines and lines that start with `#` are skipped.
pub fn sections(text: &str) -> Result<Vec<Section<'_>>, String> {
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        match header(line) {
            Some((name, prefs)) => sections.push((name, prefs, Vec::new())),
            None => sections.last_mut().ok_or_else(|| format!("{line:?} comes before the first section"))?.2.push(line),
        }
    }
    Ok(sections)
}

/// The name and prefs of a section that `line` starts, in `speed.txt`'s format: `[name]` or `[name] prefs`.
fn header(line: &str) -> Option<(&str, &str)> {
    line.strip_prefix('[').and_then(|l| l.split_once(']')).filter(|(name, prefs)| !name.contains(' ') && (prefs.is_empty() || prefs.starts_with(' '))).map(|(name, prefs)| (name, prefs.trim()))
}

/// A case line's code: the text before its first tab.
fn code_of(line: &str) -> &str { line.split('\t').next().unwrap_or_default() }

/// The slow-path events that a case's comment lines name, each as its record's name and its event's text. An `# allows RECORD "EVENT": principle` line names an event that a principle allows. A `# gap RECORD "EVENT": what should happen` line names an event that a faster route should remove, which the case tolerates until then. An event's text names it as `•explain`'s record does: the function for `forced`, the reason for `stored` and `fallbacks`, the code for `mixed` and the route for `itemwise`, with any `"` in it doubled. Each line applies to the next case line after it.
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

/// The fastest of five calls of `f`, in seconds, after one call to warm up.
fn fastest(mut f: impl FnMut()) -> f64 {
    f();
    (0..5).map(|_| {
        let start = Instant::now();
        f();
        start.elapsed()
    }).min().unwrap().as_secs_f64()
}

/// A session that has run `prefs`, then `setup`, or the evaluation that failed.
pub fn session(prefs: &str, setup: &str) -> Result<Session, Evaluation> {
    let mut session = Session::new();
    let result = session.eval(&format!("{prefs}\n{setup}"));
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

/// The number of items in the setup's data: the value of `n` in `session`.
pub fn items(session: &mut Session) -> Option<usize> {
    let n = session.eval("n").value?.as_number()?.to_float().ok()?;
    (n >= 0.0 && n.fract() == 0.0).then_some(n as usize)
}

/// The time of `code` in `session`, in seconds, or the evaluation that failed.
pub fn time(session: &mut Session, code: &str, interrupt: &InterruptHandle) -> Result<f64, Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let mut failed = None;
    let seconds = fastest(|| {
        let result = session.eval_parsed(&parsed, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
        if let Some(value) = &result.value { value.force(); }
        if result.error.is_some() { failed.get_or_insert(result); }
    });
    failed.map_or(Ok(seconds), Err)
}

/// The time of one copy of `n` items, in seconds: writing each of `n` floats negated into another buffer, as rustymath's speed test times its unit. The loop reads and writes each item once. It calls no `memcpy` and allocates nothing, because their speed differs between platforms far more than a loop's does.
pub fn copy(n: usize) -> f64 {
    let data: Vec<f64> = (1..=n).map(|i| i as f64).collect();
    let mut out = vec![0.0; n];
    fastest(|| {
        for (o, &v) in out.iter_mut().zip(black_box(&data)) { *o = -v }
        black_box(&mut out);
    })
}

/// One case of a file in `speed.txt`'s format, with its cost: the section it's in, its code, its budgets in the file's order, and the cost those budgets limit.
pub struct Case<'a> { pub section: &'a str, pub code: &'a str, pub budgets: Vec<&'a str>, pub cost: f64 }

/// The sessions of one group of sections, by section name, and the `n` of each.
struct Group<'a> { sessions: HashMap<&'a str, Session>, sizes: HashMap<&'a str, usize> }

impl Group<'_> {
    /// The session of section `name`.
    fn session(&mut self, name: &str) -> Result<&mut Session, String> { self.sessions.get_mut(name).ok_or_else(|| format!("speed's {name} section is missing")) }
}

/// A failed evaluation of `code`, as the speed checks report it.
fn failure(code: &str, failed: Evaluation) -> String { format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default()) }

/// Calls `visit` with the sessions of each case's group, the case's section and its line, for every case in `text`, in `speed.txt`'s format. The first section is `[setup]`. Each later `[setup]` starts a new group of sections. Each section's cases run in a session that has run the section's prefs and then the `[setup]` before it. Each group makes its sessions when its turn comes and drops them before the next group starts. `only`, when given, lists the codes of the cases to visit. A group then makes sessions only for the sections that hold those cases, plus `[exact]` for a `[widths]` case.
fn each_case<'a>(text: &'a str, only: Option<&[&str]>, mut visit: impl FnMut(&mut Group<'a>, &'a str, &'a str) -> Result<(), String>) -> Result<(), String> {
    let wanted = |line: &str| only.is_none_or(|only| only.contains(&code_of(line)));
    for group in sections(text)?.chunk_by(|_, next| next.0 != "setup") {
        let [("setup", _, setup), cases @ ..] = group else { return Err("the first section must be [setup]".into()) };
        let mut used: Vec<&str> = cases.iter().filter(|(_, _, lines)| lines.iter().any(|l| wanted(l))).map(|s| s.0).collect();
        if used.contains(&"widths") { used.push("exact"); }
        if used.is_empty() { continue; }
        let setup = setup.join("\n");
        let mut sessions = Group { sessions: HashMap::new(), sizes: HashMap::new() };
        for &(name, prefs, _) in cases.iter().filter(|s| used.contains(&s.0)) {
            let mut session = session(prefs, &setup).map_err(|failed| failure(prefs, failed))?;
            sessions.sizes.insert(name, items(&mut session).ok_or("each setup must define n, its number of items")?);
            sessions.sessions.insert(name, session);
        }
        for &(section, _, ref lines) in cases { for &line in lines.iter().filter(|l| wanted(l)) { visit(&mut sessions, section, line)?; } }
    }
    Ok(())
}

/// Every case in `text`, in `speed.txt`'s format, with its cost, in the sessions that `each_case` gives. A cost is in copies of the group's `n` items, timed just before the case, except in `[widths]`. There it is the case's time as a fraction of its time in `[exact]`. `only`, when given, lists the codes of the cases to time.
pub fn costs<'a>(text: &'a str, only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<Vec<Case<'a>>, String> {
    let _lock = timing_lock()?;
    let mut result = Vec::new();
    each_case(text, only, |group, section, line| {
        let mut fields = line.split('\t');
        let code = fields.next().unwrap_or_default();
        let unit = (section != "widths").then(|| copy(group.sizes[section]));
        let mut timed = |name: &str| time(group.session(name)?, code, interrupt).map_err(|failed| failure(code, failed));
        let cost = match unit { Some(unit) => timed(section)? / unit, None => timed(section)? / timed("exact")? };
        result.push(Case { section, code, budgets: fields.collect(), cost });
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
    each_case(text, only, |group, section, line| {
        let code = code_of(line);
        let session = group.session(section)?;
        let parsed = session.parse(Source::new("<input>", code)).complete().map_err(|e| failure(code, Evaluation::failed(e)))?;
        let options = || EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
        let run = |s: &mut Session| {
            let evaluation = s.eval_parsed(&parsed, options());
            if let Some(value) = &evaluation.value { value.force(); }
            evaluation
        };
        let warm = run(session);
        if warm.error.is_some() { return Err(failure(code, warm)); }
        let (evaluation, events) = session.explained(run);
        if evaluation.error.is_some() { return Err(failure(code, evaluation)); }
        result.push(Route { section, code, slow: events.slow(), named: named.remove(&(section, code)).unwrap_or_default() });
        Ok(())
    })?;
    Ok(result)
}

/// Times every case in `text` as `costs` does, then times each case over this machine's native budget again, after each pause in `rustymath::timing::retried`. `print` gets every first cost when `all` is set, and each retry's earlier and new costs. Each line gives a case's section, code, cost and native budget, in the form that `basedpl.bpltests.logged_costs` reads. A case with no native budget is never over. Gives the lines of the cases still over budget after the last retry. `tests/speed.rs` and `bpl --costs` both measure with it.
#[cfg(not(web))]
pub fn measured(text: &str, all: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath::timing::{retried, NATIVE_COLUMN};
    fn budget<'a>(c: &Case<'a>) -> Option<&'a str> { c.budgets.get(NATIVE_COLUMN).copied() }
    let line = |c: &Case| format!("{} {}: {:.2}, budget {}", c.section, c.code, c.cost, budget(c).unwrap_or("-"));
    let over = |c: &Case| budget(c).and_then(|b| b.parse::<f64>().ok()).is_some_and(|b| c.cost > b);
    let interrupt = InterruptHandle::default();
    let mut first = costs(text, None, &interrupt)?;
    if all { for c in &first { print(&line(c)); } }
    first.retain(over);
    let mut failed = Ok(());
    let over_budget = retried(first, |cases, pause| {
        for c in &cases { print(&format!("{}: over, timed again after {pause} s", line(c))); }
        let codes: Vec<&str> = cases.iter().map(|c| c.code).collect();
        match costs(text, Some(&codes), &interrupt) {
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

/// The reply to the worker's `timing` request, which gives `speed`, text in `speed.txt`'s format whose first section is `[setup]`, the `prefs` its session starts with, and the `cases` to time. The reply gives the setup's `n` and each case's time in seconds and in copies, or the evaluation that failed. Each case's copy is timed just before the case. An optional `threads` sets how many threads each case's rustymath calls split across, for a scaling check. Without it they run on the timing thread alone, as in the speed test.
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let _lock = timing_lock()?;
    if let Some(threads) = spec["threads"].as_u64() { rustymath::parallel::set_threads(threads as usize); }
    let text = |key| spec[key].as_str().ok_or_else(|| format!("timing needs {key} as a string"));
    let cases = spec["cases"].as_array().ok_or("timing needs cases as an array")?;
    let sections = sections(text("speed")?)?;
    let setup = match sections.first() { Some(("setup", _, lines)) => lines.join("\n"), _ => return Err("speed's first section must be [setup]".into()) };
    let mut session = match session(text("prefs")?, &setup) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    let n = items(&mut session).ok_or("the setup must define n, its number of items")?;
    let mut costs = Vec::new();
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        let unit = copy(n);
        match time(&mut session, code, interrupt) {
            Ok(seconds) => costs.push(json!({"code": code, "seconds": seconds, "copies": seconds / unit})), Err(failed) => return Ok(response(failed))
        }
    }
    Ok(json!({"n": n, "costs": costs, "output": [], "error": null}))
}

/// The reply to the worker's `costs` request, which gives `speed`, text in `speed.txt`'s format, and optionally `cases`, a list of the codes to time. The reply gives each case with its cost, as `costs` times them: its section, code, budgets and cost.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    let cases = costs(spec["speed"].as_str().ok_or("costs needs speed as a string")?, only.as_deref(), interrupt)?;
    let cases: Vec<_> = cases.iter().map(|c| json!({"section": c.section, "code": c.code, "budgets": c.budgets, "cost": c.cost})).collect();
    Ok(json!({"costs": cases, "error": null}))
}
