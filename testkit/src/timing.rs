//! BPL measurements share reference records and setups. Numerical timing, copy units, footprint copies and retries live in Rustymath.
use crate::{DEFAULT_PREFS, reference::{self, File}, timing_lock};
use basedpl::{EvalOptions, Evaluation, InterruptHandle, Session, Source, Turn, protocol::response};
use rustymath::hashing::HashMap;
use rustymath_timing::{Budget, COUNT_MARGIN, Cause, CopyTime, Count, Footprint, Held, MEASURES, NATIVE_COLUMN, copy_time, footprint};
pub use rustymath_timing::{RETRY_PAUSES, checks_budgets, name_cpu};
use serde_json::{Value as JsonValue, json};

/// The record and event named by each permitted or pending slow-path annotation.
#[derive(Default)]
pub struct Named { pub routes: Vec<String>, pub allowed: Vec<(String, String)>, pub gaps: Vec<(String, String)> }

fn single_threaded_defaults() {
    basedpl::settings::reset();
    rustymath::parallel::set_threads(1);
}
/// The fixture size that a setup's `full size small` pairs take: full for timings, small for route checks under `Sizes::SMALL`, and for counts the power of two nearest the pairs' geometric mean. Counts give each item's instructions and bytes, which don't need full-size data.
#[derive(Clone, Copy, PartialEq)]
enum Size { Full, Counted, Small }

fn sized_setup(setup: &str, size: Size) -> String {
    let select = match size { Size::Full => "⊣", Size::Small => "⊢", Size::Counted => "{2ₓ*⌊ₓ0.5+0.5×(2⍟⍺)+2⍟⍵}" };
    format!("size←{select}\n{setup}")
}

/// A fresh session which runs preferences before full-size setup, on one thread, with its random generator seeded at 0.
pub fn session(prefs: &str, setup: &str) -> Result<Turn, Evaluation> { configured(prefs, &sized_setup(setup, Size::Full)) }

fn configured(prefs: &str, setup: &str) -> Result<Turn, Evaluation> {
    single_threaded_defaults();
    let session = Session::new();
    let result = session.eval(&format!("{DEFAULT_PREFS}\n•prefs [\"seed\":0]\n{prefs}\n{setup}"));
    if result.error.is_some() { Err(result) } else { Ok(session) }
}

pub fn items(session: &Session) -> Option<usize> { let n = session.eval("n").value?.as_real()?; (n >= 0.0 && n.fract() == 0.0).then_some(n as usize) }

/// A run of `code` as timings run it: evaluated at top level with no echo, then its value forced in full through `Session::forced`, which the interrupt and deadline stop as they stop the evaluation. Each call keeps the first failed evaluation in its argument. `time` and `steps_request` run cases through it.
fn runner(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<impl FnMut(&mut Option<Evaluation>), Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let options = move || EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    Ok(move |failed: &mut Option<Evaluation>| {
        let result = session.eval_parsed(&parsed, options());
        let forced = result.value.as_ref().map_or(Ok(()), |value| session.forced(value, options()));
        if result.error.is_some() { failed.get_or_insert(result); } else if let Err(error) = forced { failed.get_or_insert(Evaluation::failed(error)); }
    })
}

/// Parse once, time the first fully forced evaluation, then take the fastest warm evaluation. Gives the warm time, peak bytes held during warm runs, bytes left held, and first-evaluation time. Parsing and setup are outside both timings.
pub fn time(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<(f64, usize, usize, f64), Evaluation> {
    timed_runs(session, code, interrupt, true)
}

/// The least time that a case's timed evaluations take together: 5 ms. A case that runs in microseconds then gives the fastest of hundreds of evaluations, which varies less between test runs than the fastest of `rustymath_timing::RUNS`.
const LEAST: std::time::Duration = std::time::Duration::from_millis(5);

fn timed_runs(session: &Session, code: &str, interrupt: &InterruptHandle, warmup: bool) -> Result<(f64, usize, usize, f64), Evaluation> {
    let (mut run, mut failed) = (runner(session, code, interrupt)?, None);
    let cold_seconds = if warmup {
        let first = rustymath::parallel::Instant::now();
        run(&mut failed);
        first.elapsed().as_secs_f64()
    } else { 0.0 };
    let mut start = 0;
    let (seconds, held) = rustymath::memory::peak_held(|| {
        rustymath_timing::fastest(LEAST, |k| { run(&mut failed); if k == 0 { start = rustymath::memory::held(); } })
    });
    let left = rustymath::memory::held().saturating_sub(start);
    failed.map_or(Ok((seconds, held, left, cold_seconds)), Err)
}

/// The browser build's budget column: a case's third budget, after its macOS and Linux budgets.
pub const WASM_COLUMN: usize = 2;

/// One measured record. `label` is single-line; `code` retains its original layout. `holds` are the columns that the case's `held` and `busy` lines hold. `footprint` is the case's footprint, where it passes one core's cache, with a copy of it measured beside the case. `obligations` are the full-size warm-up route checks that remain pending.
pub struct Case {
    pub file: String, pub section: String, pub code: String, pub label: String, pub budgets: Vec<String>, pub holds: Vec<Held>, pub cost: f64, pub seconds: f64,
    pub copy: Option<CopyTime>, pub footprint: Option<Footprint>, pub obligations: Vec<String>
}
impl Case {
    /// The budget that the case meets in `column`, with any hold, as `rustymath_timing::Budget::of` gives it.
    pub fn budget(&self, column: Option<usize>) -> Option<Budget> { Budget::of(&self.section, &self.budgets, &self.holds, column) }
    pub fn line(&self, column: Option<usize>) -> String {
        rustymath_timing::line(
            &self.section, &self.label, self.cost, &self.budget(column).map_or("-".into(), |b| b.to_string()), self.seconds, self.footprint, [None; 2])
    }
    pub fn copy_line(&self) -> Option<String> { self.copy.as_ref().map(rustymath_timing::copy_line) }
}

/// Each case as JSON, with its budget in `column`: its `budget`, its `rule` budget and its `hold`, `held` or `busy`, where a line holds that column, and whether its cost is `over` the budget or its hold is `stale`, as `rustymath_timing::Budget` decides.
pub fn rows(cases: &[Case], column: Option<usize>) -> JsonValue {
    cases.iter().map(|c| {
        let b = c.budget(column);
        json!({"file":c.file, "section":c.section, "code":c.code, "label":c.label, "line":c.line(column), "copy":c.copy_line(), "budgets":c.budgets,
    "budget":b.map(|b| b.value), "rule":b.map(|b| b.rule), "hold":b.and_then(|b| b.cause).map(Cause::name),
    "over":b.is_some_and(|b| b.over(c.cost)), "stale":b.is_some_and(|b| b.stale(c.cost)),
    "obligations":c.obligations, "cost":c.cost, "seconds":c.seconds, "footprint":c.footprint.map(|f| json!({"bytes":f.bytes, "seconds":f.seconds}))})
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
struct Group<'a> { session: Turn, setup: String, size: Size, prefs: HashMap<&'a str, String>, current: &'a str, n: usize }
impl<'a> Group<'a> {
    fn new(file: &'a File, start: &str, first: &'a str, size: Size) -> Result<Self, String> {
        let prefs: HashMap<_, _> =
            file.cases.iter().map(|case| (case["section"].as_str().unwrap(), [start, case["prefs"].as_str().unwrap_or_default()].join("\n"))).collect();
        let setup = sized_setup(&file.setup, size);
        let session = configured(section_prefs(&prefs, first)?, &setup).map_err(|failed| failure(first, failed))?;
        let n = items(&session).ok_or("each setup must define n, its number of items")?;
        Ok(Self { session, setup, size, prefs, current: first, n })
    }
    fn session(&mut self, name: &'a str) -> Result<&Session, String> {
        let header = section_prefs(&self.prefs, name)?;
        if self.current != name {
            self.session = configured(header, &self.setup).map_err(|failed| failure(header, failed))?;
            self.current = name;
        }
        single_threaded_defaults();
        self.session.eval(&format!("{DEFAULT_PREFS}\n{header}"));
        if self.size == Size::Small { basedpl::settings::set_small_sizes(true); }
        Ok(&self.session)
    }
}

/// The preferences that start section `name`'s session.
fn section_prefs<'p>(prefs: &'p HashMap<&str, String>, name: &str) -> Result<&'p String, String> {
    prefs.get(name).ok_or_else(|| format!("speed's {name} section is missing"))
}

fn failure(code: &str, failed: Evaluation) -> String { format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default()) }

/// Select execution without discarding the catalogue's section preferences.
#[derive(Clone, Copy)]
pub enum Selection<'a> { Codes(&'a [&'a str]), Rows(&'a [JsonValue]) }

impl Selection<'_> {
    pub fn matches(self, file: &File, case: &JsonValue) -> bool {
        match self {
            Self::Codes(codes) => {
                codes.iter().any(|&s| s == case["code"].as_str().unwrap() || s == case["id"].as_str().unwrap_or_default() || s == label(file, case))
            }
            Self::Rows(rows) => rows.iter().any(|r| r[0] == file.name && r[1] == case["section"] && r[2] == case["code"]),
        }
    }
}

fn each_case<'a>(
    files: &'a [File], only: Option<Selection<'_>>, start: &str, size: Size, keep: fn(&JsonValue) -> bool,
    mut visit: impl FnMut(&mut Group<'a>, &'a File, &'a JsonValue) -> Result<(), String>) -> Result<(), String> {
    for file in files {
        let wanted = |case: &&JsonValue| keep(case) && only.is_none_or(|only| only.matches(file, case));
        let mut cases = file.cases.iter().filter(wanted).peekable();
        let Some(first) = cases.peek() else { continue; };
        let mut group = Group::new(file, start, first["section"].as_str().unwrap(), size)?;
        for case in cases { visit(&mut group, file, case)?; }
    }
    Ok(())
}

/// Whether a record is a speed case: one with a footer.
pub fn speed_case(case: &JsonValue) -> bool { case["budgets"].is_array() }

/// Whether a speed case has a time budget in some column. A count-only case gives `-` in every time column.
pub fn timed(case: &JsonValue) -> bool { case["budgets"].as_array().is_some_and(|b| b.iter().any(|b| b != "-")) }

/// Time selected records in copies, or as widths/exact ratios, within the timing hold that the caller takes. With `footprints`, a case whose footprint passes one core's cache also times a copy of its footprint (`rustymath_timing::footprint`).
pub fn costs(files: &[File], only: Option<Selection<'_>>, prefs: &str, footprints: bool, interrupt: &InterruptHandle) -> Result<Vec<Case>, String> {
    let mut result = Vec::new();
    each_case(files, only, prefs, Size::Full, timed, |group, file, case| {
        let (section, code) = (case["section"].as_str().unwrap(), case["code"].as_str().unwrap());
        let n = group.n;
        let copy = (section != "widths").then(|| copy_time::<f64>(n));
        let session = group.session(section)?;
        let obligations = route(session, case, interrupt)?.violations();
        let (total, held, left, _) = timed_runs(session, code, interrupt, false).map_err(|failed| failure(code, failed))?;
        if rustymath::memory::checks() && left > 0 {
            return Err(format!("{}: the timed runs left {left} more bytes held in large allocations than the first run", label(file, case)));
        }
        let (cost, footprint) = match copy {
            Some(copy) => (total / n as f64 / copy.seconds, footprints.then(|| footprint::<f64>(n, held)).flatten()), None => (total, None)
        };
        let mut holds = Vec::new();
        for held in case["held"].as_array().into_iter().flatten() {
            rustymath_timing::hold(held.as_str().unwrap(), &mut holds).expect("`reference::file` checks each line");
        }
        result.push(Case { file: file.name.clone(), section: section.into(), code: code.into(), label: label(file, case),
            budgets: case["budgets"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect(), holds, cost, seconds: total / n as f64, copy,
            footprint, obligations });
        Ok(())
    })?;
    // A widths case's cost is its time over the same code's time in the exact section. Each change of section reruns the file's setup in the process's one session. A file's widths cases therefore take their exact times together, in one exact session, after its other cases.
    for file in files {
        let widths: Vec<usize> = (0..result.len()).filter(|&i| result[i].file == file.name && result[i].section == "widths").collect();
        if widths.is_empty() { continue; }
        let mut group = Group::new(file, prefs, "exact", Size::Full)?;
        let session = group.session("exact")?;
        for i in widths {
            let code = &result[i].code;
            let exact = time(session, code, interrupt).map_err(|failed| failure(code, failed))?.0;
            result[i].cost /= exact;
        }
    }
    Ok(result)
}

/// One case's instructions and allocated bytes for each item, in the order of `MEASURES`, as `counts` measures them, and the counts that its footer gives.
pub struct Counted { pub section: String, pub label: String, pub measured: [Option<f64>; 2], pub counts: Vec<Count> }

/// How far a measure in a `calls` or `steps` section may rise above its count. Each of their cases counts calls of dfns or steps of operators, the costs that Each, Power and the other operators repeat.
const CALL_MARGIN: f64 = 1.1;

impl Counted {
    /// Each measure that rises past its count in `column`, as `Count::over` decides with `COUNT_MARGIN`, or `CALL_MARGIN` in a `calls` or `steps` section.
    pub fn over(&self, column: Option<usize>) -> Vec<String> {
        let margin = if matches!(self.section.as_str(), "calls" | "steps") { CALL_MARGIN } else { COUNT_MARGIN };
        self.counts.iter().filter(|c| Some(c.column) == column).filter_map(|c| {
            let measured = self.measured[c.measure]?;
            c.over(measured, margin).then(|| format!("{} {}: {} {measured:.3}, count {}", self.section, self.label, MEASURES[c.measure], c.value))
        }).collect()
    }
    /// The case's measures as a line that `basedpl.bpltests.set_counts` reads, such as `arrays -v: instructions 1.250, allocated 8.000`.
    pub fn line(&self) -> String { format!("{} {}: {}", self.section, self.label, rustymath_timing::counted(self.measured)) }
}

/// Counts each selected speed case at its fixture's counted size (`Size::Counted`), on one thread with compiling off. Each case runs once to warm, then once more, or `n` times in a `steps` section, each run forcing its value. Those runs' instructions and allocated bytes, divided by the fixture's `n`, give the case's measures for each item, or for each step. Counts don't vary with load, so this takes no timing hold. Allocated bytes are 0 in a binary that doesn't register `rustymath_timing::Counting`.
pub fn counts(files: &[File], only: Option<Selection<'_>>, interrupt: &InterruptHandle) -> Result<Vec<Counted>, String> {
    let mut result = Vec::new();
    each_case(files, only, "", Size::Counted, speed_case, |group, file, case| {
        let (section, code) = (case["section"].as_str().unwrap(), case["code"].as_str().unwrap());
        let (n, runs) = (group.n as f64, if section == "steps" { group.n } else { 1 });
        let session = group.session(section)?;
        basedpl::settings::set_compiles(basedpl::settings::Compiles::Never);
        let (mut run, mut failed) = (runner(session, code, interrupt).map_err(|failed| failure(code, failed))?, None);
        run(&mut failed);
        let (((), bytes), instructions) = rustymath_timing::instructions(|| {
            rustymath_timing::bytes(|| { for _ in 0..runs { run(&mut failed) } })
        });
        if let Some(failed) = failed { return Err(failure(code, failed)); }
        let mut counts = Vec::new();
        for field in case["counts"].as_array().into_iter().flatten() {
            rustymath_timing::count(field.as_str().unwrap(), &mut counts).expect("`reference::file` checks each field");
        }
        result.push(Counted { section: section.into(), label: label(file, case),
            measured: [instructions.map(|i| i as f64 / n), Some(bytes.allocated as f64 / n)], counts });
        Ok(())
    })?;
    Ok(result)
}

/// What one run of a case recorded: its slow-path events (`explain::Events::slow`), and rustymath's aggregate records of it (`events::Report`).
pub struct Observed { pub slow: Vec<(&'static str, String, String)>, pub report: rustymath::events::Report }

impl Observed {
    /// What `events` holds of one run.
    fn of(events: &basedpl::hooks::Events) -> Self { Self { slow: events.slow(), report: events.observed().clone() } }
    /// Whether this run recorded `route` as an event of `kind`: "taken", "declined" or "itemwise". An itemwise route appears in `slow`, as `•explain`'s own rows hold it.
    pub fn exercised(&self, kind: &str, route: rustymath::events::Route) -> bool {
        self.report.records.iter().filter_map(rustymath::events::Event::route).any(|r| r.kind.name() == kind && r.route == route)
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
            let records = self.run.report.records.iter().filter_map(rustymath::events::Event::route);
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

/// `code` parsed once and run twice in `session`, each run as `•explain` runs its text, with its value forced: a warm-up, then the run that a route check reads. Gives that run's evaluation and events and the warm-up's events, or the evaluation that failed. The route test (`route`) and the worker's `route` request (`route_request`) both capture through it.
fn warmed(
    session: &Session, code: &str, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle
) -> Result<(Evaluation, basedpl::hooks::Events, basedpl::hooks::Events), Evaluation> {
    let parsed = session.parse(Source::new("<input>", code)).complete().map_err(Evaluation::failed)?;
    let options = || EvalOptions { timeout, echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    let run = || {
        let evaluation = session.eval_parsed(&parsed, options());
        match evaluation.value.as_ref().map_or(Ok(()), |value| session.forced(value, options())) {
            Err(error) if evaluation.error.is_none() => Evaluation::failed(error), _ => evaluation
        }
    };
    let (warm, warmup) = basedpl::hooks::explained(run);
    if warm.error.is_some() { return Err(warm); }
    let (evaluation, events) = basedpl::hooks::explained(run);
    if evaluation.error.is_some() { return Err(evaluation); }
    Ok((evaluation, events, warmup))
}

/// Warm one case, then capture a forced evaluation in the same session (`warmed`).
pub fn route(session: &Session, case: &JsonValue, interrupt: &InterruptHandle) -> Result<Capture, String> {
    let code = case["code"].as_str().unwrap();
    let (_, events, warmup) = warmed(session, code, Some(reference::TIMEOUT), interrupt).map_err(|failed| failure(code, failed))?;
    let named = |key: &str| case[key].as_array().into_iter().flatten().map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into())).collect();
    Ok(Capture { file: String::new(), section: case["section"].as_str().unwrap_or_default().into(), code: code.into(),
        label: case["id"].as_str().unwrap_or(code).into(), run: Observed::of(&events), warmup: Observed::of(&warmup), named: Named {
            routes: case["route"].as_array().into_iter().flatten().map(|s| s.as_str().unwrap().into()).collect(), allowed: named("allows"), gaps: named("gap") }
    })
}

/// The worker's `route` request: the `•explain` record of the run of `spec`'s `code` that a route check reads (`warmed`), in a fresh session that runs `prefs` and `setup` first (`session`). `bpltests.explained` reads it.
pub fn route_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let text = |key| spec[key].as_str().ok_or_else(|| format!("route needs {key} as a string"));
    let code = text("code")?;
    let session = match session(text("prefs")?, text("setup")?) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    let (mut evaluation, events, _) = match warmed(&session, code, None, interrupt) { Ok(runs) => runs, Err(failed) => return Ok(response(failed)) };
    let value = evaluation.value.take().ok_or("route needs code that gives a value")?;
    evaluation.value = Some(events.report(value, &basedpl::Span::whole(Source::new("<input>", code))).map_err(|error| error.to_string())?);
    Ok(response(evaluation))
}

/// The route capture of `case`, a speed case of `file` (`route`), at the fixture's small size under `Sizes::SMALL`, in a fresh session with the case's section preferences. `reference::speed_route` checks it as the reference suite checks its routed cases.
pub fn small_route<'a>(file: &'a File, case: &'a JsonValue, interrupt: &InterruptHandle) -> Result<Capture, String> {
    let section = case["section"].as_str().unwrap();
    let mut group = Group::new(file, "", section, Size::Small)?;
    let capture = route(group.session(section)?, case, interrupt)?;
    Ok(Capture { file: file.name.clone(), label: label(file, case), ..capture })
}

/// Times the cases in `files` and gives the lines of those still over their native budgets. An ordinary run times under a shared hold on one thread (`rustymath_timing::Hold::on_one_thread`), then retimes each case over its budget once under the exclusive hold where no build holds the lock (`try_exclusive`), and gives those still over it. Where builds hold the lock, it prints each case over its budget, marked `unconfirmed`, and gives none. A run that `calibrate`s holds the lock exclusively, prints the copy lines and every cost with its footprint, and retimes each case over its budget after each pause in `RETRY_PAUSES`. Every run prints each case's pending route obligations, then each case still over its budget, marked `over`, and last each case whose gap's hold has outlived the gap, as `rustymath_timing::Budget::stale` decides from its first cost, marked `stale`.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn measured(files: &[File], only: Option<Selection<'_>>, calibrate: bool, mut print: impl FnMut(&str)) -> Result<Vec<String>, String> {
    use rustymath_timing::{Hold, exclusive, retried, shared, try_exclusive, warm};
    let line = |c: &Case| c.line(NATIVE_COLUMN);
    let over = |c: &Case| c.budget(NATIVE_COLUMN).is_some_and(|b| b.over(c.cost));
    let interrupt = InterruptHandle::default();
    let retime = |cases: &[Case]| {
        let rows: Vec<JsonValue> = cases.iter().map(|c| json!([c.file, c.section, c.code])).collect();
        costs(files, Some(Selection::Rows(&rows)), "", true, &interrupt)
    };
    let hold = if calibrate { exclusive() } else { shared().map(Hold::on_one_thread) }.map_err(|e| e.to_string())?;
    let mut first = costs(files, only, "", calibrate, &interrupt)?;
    for c in &first { for obligation in &c.obligations { print(&format!("{}: route obligation: {obligation}", c.label)); } }
    if calibrate {
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
    let over_budget = if calibrate {
        let mut failed = Ok(());
        let over_budget = retried(first, |cases, pause| {
            for c in &cases { print(&format!("{}: over, timed again after {pause} s", line(c))); }
            match retime(&cases) {
                Ok(retimed) => { for c in &retimed { print(&line(c)); } retimed.into_iter().filter(over).collect() } Err(e) => { failed = Err(e); Vec::new() }
            }
        });
        failed?;
        over_budget
    } else {
        drop(hold);
        if first.is_empty() { first }
        else if let Some(_hold) = try_exclusive().map_err(|e| e.to_string())? {
            warm();
            for c in &first { print(&format!("{}: over, timed again", line(c))); }
            let retimed = retime(&first)?;
            for c in &retimed { print(&line(c)); }
            retimed.into_iter().filter(over).collect()
        } else { for c in &first { print(&format!("{}: unconfirmed", line(c))); } Vec::new() }
    };
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
            Ok((seconds, _, _, cold_seconds)) => costs.push(json!({"code":code, "seconds":seconds, "cold_seconds":cold_seconds, "copies":seconds / n as f64 / copy.seconds})),
            Err(failed) => return Ok(response(failed)),
        }
    }
    Ok(json!({"n":n, "costs":costs, "output":[], "error":null}))
}

/// Runs each case `steps` times as `time` runs it, for an instruction count, and gives no timing. Unlike `request`, it takes no timing lock, so counts can run side by side under one shared hold.
pub fn steps_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    repeated_request(spec, interrupt, None::<fn() -> Result<(), String>>)
}

/// Prepares one case, forces two warm calls, signals readiness, then repeats in that same Session.
pub fn profile_request(
    spec: &JsonValue, interrupt: &InterruptHandle, ready: impl FnOnce() -> Result<(), String>
) -> Result<JsonValue, String> {
    repeated_request(spec, interrupt, Some(ready))
}

fn repeated_request(
    spec: &JsonValue, interrupt: &InterruptHandle, mut ready: Option<impl FnOnce() -> Result<(), String>>
) -> Result<JsonValue, String> {
    let text = |key| spec[key].as_str().ok_or_else(|| format!("steps needs {key} as a string"));
    let steps = spec["steps"].as_u64().ok_or("steps needs steps as a count")?;
    let cases = spec["cases"].as_array().ok_or("steps needs cases as an array")?;
    if ready.is_some() && cases.len() != 1 { return Err("profiling needs exactly one case".into()); }
    let session = match session(text("prefs")?, text("setup")?) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        let mut failed = None;
        match runner(&session, code, interrupt) {
            Ok(mut run) => {
                if let Some(ready) = ready.take() {
                    for _ in 0..2 { run(&mut failed); if let Some(failed) = failed { return Ok(response(failed)); } }
                    ready()?;
                }
                for _ in 0..steps { run(&mut failed); if failed.is_some() { break; } }
            }
            Err(error) => failed = Some(error)
        }
        if let Some(failed) = failed { return Ok(response(failed)); }
    }
    Ok(json!({"output":[], "error":null}))
}

/// The worker's `costs` request: each selected case of `spec`'s `files` timed with its footprint (`costs`) under the exclusive timing hold, as JSON rows with their native budgets (`rows`). `bpltests.speed_costs` reads it.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    let files = reference::sources(&spec["files"])?;
    let _lock = timing_lock()?;
    let costs = costs(&files, only.as_deref().map(Selection::Codes), spec["prefs"].as_str().unwrap_or_default(), true, interrupt)?;
    Ok(json!({"costs":rows(&costs, NATIVE_COLUMN), "error":null}))
}
