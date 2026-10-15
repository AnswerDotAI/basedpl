//! The speed test's measurements, shared by `tests/speed.rs`, the browser build's `costs` and the worker's `timing` and `costs`
//! requests, which serve `basedpl.bpltests.time_cases` and `speed_costs`. A time is the fastest of five evaluations of a case, after
//! one to warm up. The case is parsed once, before them. Its unit is one copy, as `copy` times it, of the `n` items in the data of the
//! `[setup]` before the case's section.
//! `costs` and `request` hold the machine's timing lock while they run. `timing_lock` describes the lock. The reference test takes it
//! shared.
pub use crate::host::timing_lock;
use crate::{host::Instant, protocol::response, EvalOptions, Evaluation, InterruptHandle, Session, Source};
use serde_json::{json, Value as JsonValue};
use std::{collections::HashMap, hint::black_box};

/// One section of a file in `speed.txt`'s format: its name, the prefs its session starts with, and its lines.
pub type Section<'a> = (&'a str, &'a str, Vec<&'a str>);

/// The sections of `text`, in `speed.txt`'s format: a line `[name]` or `[name] prefs` starts each section. Blank lines and lines that
/// start with `#` are skipped.
pub fn sections(text: &str) -> Result<Vec<Section<'_>>, String> {
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let header = line.strip_prefix('[').and_then(|l| l.split_once(']'));
        match header.filter(|(name, prefs)| !name.contains(' ') && (prefs.is_empty() || prefs.starts_with(' '))) {
            Some((name, prefs)) => sections.push((name, prefs.trim(), Vec::new())),
            None => sections.last_mut().ok_or_else(|| format!("{line:?} comes before the first section"))?.2.push(line),
        }
    }
    Ok(sections)
}

/// The fastest of five calls of `f`, in seconds, after one call to warm up.
fn fastest(mut f: impl FnMut()) -> f64 {
    f();
    (0..5).map(|_| {
        let start = Instant::now();
        f();
        start.elapsed()
    })
    .min()
    .unwrap()
    .as_secs_f64()
}

/// The pauses, in seconds, before each retry in `retried`. All of a case's runs together take a fraction of a millisecond, and one
/// disturbance on a shared machine can last that long. `tests/wasm-speed.mjs` uses the same pauses.
pub const RETRY_PAUSES: [f64; 2] = [0.1, 1.0];

/// The failures of a timing test that still fail after every retry. After each pause in `RETRY_PAUSES`, `retime` gets the failures
/// and the pause, times them again, prints the retry and gives those that still fail. The speed and scaling tests call it after
/// timing every case.
#[cfg(not(web))]
pub fn retried<T>(mut failures: Vec<T>, mut retime: impl FnMut(Vec<T>, f64) -> Vec<T>) -> Vec<T> {
    for pause in RETRY_PAUSES {
        if failures.is_empty() { break; }
        std::thread::sleep(std::time::Duration::from_secs_f64(pause));
        failures = retime(failures, pause);
    }
    failures
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
        if result.error.is_some() { failed.get_or_insert(result); }
    });
    failed.map_or(Ok(seconds), Err)
}

/// The time of one copy of `n` items, in seconds: writing each of `n` floats negated into another buffer, as rustymath's speed test
/// times its unit. The loop reads and writes each item once. It calls no `memcpy` and allocates nothing, because their speed differs
/// between platforms far more than a loop's does.
pub fn copy(n: usize) -> f64 {
    let data: Vec<f64> = (1..=n).map(|i| i as f64).collect();
    let mut out = vec![0.0; n];
    fastest(|| {
        for (o, &v) in out.iter_mut().zip(black_box(&data)) { *o = -v }
        black_box(&mut out);
    })
}

/// One case of a file in `speed.txt`'s format, with its cost: the section it's in, its code, its budgets in the file's order, and
/// the cost those budgets limit.
pub struct Case<'a> {
    pub section: &'a str,
    pub code: &'a str,
    pub budgets: Vec<&'a str>,
    pub cost: f64,
}

/// Every case in `text`, in `speed.txt`'s format, with its cost. The first section is `[setup]`. Each later `[setup]` starts a new
/// group of sections. Each section's cases run in a session that has run the section's prefs and then the `[setup]` before it. Each
/// group makes its sessions when its turn comes and drops them before the next group starts. A cost is in copies of the group's `n`
/// items, timed just before the case, except in `[widths]`. There it is the case's time as a fraction of its time in `[exact]`.
/// `only`, when given, lists the codes of the cases to run. A group then makes sessions only for the sections that hold those cases,
/// plus `[exact]` for a `[widths]` case.
pub fn costs<'a>(text: &'a str, only: Option<&[&str]>, interrupt: &InterruptHandle) -> Result<Vec<Case<'a>>, String> {
    let _lock = timing_lock(false)?;
    let failure = |code: &str, failed: Evaluation| format!("{code}: {}", failed.error.map(|e| e.to_string()).unwrap_or_default());
    let code_of = |line: &'a str| line.split('\t').next().unwrap_or_default();
    let wanted = |code: &str| only.is_none_or(|only| only.contains(&code));
    let sections = sections(text)?;
    let mut result = Vec::new();
    for group in sections.chunk_by(|_, next| next.0 != "setup") {
        let [("setup", _, setup), cases @ ..] = group else { return Err("the first section must be [setup]".into()) };
        let mut used: Vec<&str> = cases.iter().filter(|(_, _, lines)| lines.iter().any(|l| wanted(code_of(l)))).map(|s| s.0).collect();
        if used.contains(&"widths") { used.push("exact"); }
        if used.is_empty() { continue; }
        let setup = setup.join("\n");
        let (mut sessions, mut sizes) = (HashMap::new(), HashMap::new());
        for &(name, prefs, _) in cases.iter().filter(|s| used.contains(&s.0)) {
            let mut session = session(prefs, &setup).map_err(|failed| failure(prefs, failed))?;
            sizes.insert(name, items(&mut session).ok_or("each setup must define n, its number of items")?);
            sessions.insert(name, session);
        }
        let mut timed = |name: &str, code: &str| {
            let session = sessions.get_mut(name).ok_or_else(|| format!("speed's {name} section is missing"))?;
            time(session, code, interrupt).map_err(|failed| failure(code, failed))
        };
        for &(section, _, ref lines) in cases {
            for line in lines {
                let mut fields = line.split('\t');
                let code = fields.next().unwrap_or_default();
                if !wanted(code) { continue; }
                let cost = if section == "widths" { timed(section, code)? / timed("exact", code)? } else {
                    let unit = copy(sizes[section]);
                    timed(section, code)? / unit
                };
                result.push(Case { section, code, budgets: fields.collect(), cost });
            }
        }
    }
    Ok(result)
}

/// The reply to the worker's `timing` request, which gives `speed`, text in `speed.txt`'s format whose first section is `[setup]`, the
/// `prefs` its session starts with, and the `cases` to time. The reply gives the setup's `n` and each case's time in seconds and in
/// copies, or the evaluation that failed. Each case's copy is timed just before the case. An optional `threads` sets how many threads
/// each case's rustymath calls split across, for a scaling check. Without it they run on the timing thread alone, as in the speed test.
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let _lock = timing_lock(false)?;
    if let Some(threads) = spec["threads"].as_u64() { rustymath::parallel::set_threads(threads as usize); }
    let text = |key| spec[key].as_str().ok_or_else(|| format!("timing needs {key} as a string"));
    let cases = spec["cases"].as_array().ok_or("timing needs cases as an array")?;
    let sections = sections(text("speed")?)?;
    let setup = match sections.first() {
        Some(("setup", _, lines)) => lines.join("\n"),
        _ => return Err("speed's first section must be [setup]".into()),
    };
    let mut session = match session(text("prefs")?, &setup) { Ok(session) => session, Err(failed) => return Ok(response(failed)) };
    let n = items(&mut session).ok_or("the setup must define n, its number of items")?;
    let mut costs = Vec::new();
    for code in cases {
        let code = code.as_str().ok_or("each case must be a string")?;
        let unit = copy(n);
        match time(&mut session, code, interrupt) {
            Ok(seconds) => costs.push(json!({"code": code, "seconds": seconds, "copies": seconds / unit})),
            Err(failed) => return Ok(response(failed)),
        }
    }
    Ok(json!({"n": n, "costs": costs, "output": [], "error": null}))
}

/// The reply to the worker's `costs` request, which gives `speed`, text in `speed.txt`'s format, and optionally `cases`, a list of the
/// codes to time. The reply gives each case with its cost, as `costs` times them: its section, code, budgets and cost.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let only: Option<Vec<&str>> = match spec.get("cases") {
        None | Some(JsonValue::Null) => None,
        Some(cases) => Some(cases.as_array().and_then(|c| c.iter().map(JsonValue::as_str).collect()).ok_or("costs needs cases as a list of strings")?),
    };
    let cases = costs(spec["speed"].as_str().ok_or("costs needs speed as a string")?, only.as_deref(), interrupt)?;
    let cases: Vec<_> = cases.iter().map(|c| json!({"section": c.section, "code": c.code, "budgets": c.budgets, "cost": c.cost})).collect();
    Ok(json!({"costs": cases, "error": null}))
}
