//! The speed test's measurements, shared by `tests/speed.rs` and the worker's `timing` request, which serves
//! `basedpl.bpltests.time_cases`. A time is the fastest of five runs, after one run to warm up. Its unit is one copy: allocating a
//! vector of `n` 8-byte numbers and copying `n` numbers into it, where `n` is the number of items in the setup's data.
use crate::{protocol::response, EvalOptions, Evaluation, InterruptHandle, Session};
use serde_json::{json, Value as JsonValue};
use std::{hint::black_box, time::Instant};

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
    let mut failed = None;
    let seconds = fastest(|| {
        let result = session.eval_with(code, EvalOptions { echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() });
        if result.error.is_some() { failed.get_or_insert(result); }
    });
    failed.map_or(Ok(seconds), Err)
}

/// The time of one copy of `n` items, in seconds.
pub fn copy(n: usize) -> f64 {
    let data: Vec<f64> = (0..n).map(|i| i as f64).collect();
    fastest(|| drop(black_box(black_box(&data).clone())))
}

/// The reply to the worker's `timing` request, which gives `speed`, text in `speed.txt`'s format whose first section is `[setup]`, the
/// `prefs` its session starts with, and the `cases` to time. The reply gives the setup's `n` and each case's time in seconds and in
/// copies, or the evaluation that failed. Each case's copy is timed just before the case.
pub fn request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
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
