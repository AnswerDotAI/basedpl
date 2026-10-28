//! Development-only checks, shared by the tests and the worker: reference cases, `.bpl` reference files and the examples in `.qmd` pages.
use crate::{number::Numeric, settings::Compiles, EvalOptions, InterruptHandle, Session, Value};
use serde_json::{json, Value as JsonValue};
use std::{collections::VecDeque, sync::{Arc, Mutex}};

/// A value captured from another interpreter. Its numbers carry no exactness. A capture encodes a simple atom as a scalar array, and BPL expectations use based values directly.
pub fn expected_array(value: &JsonValue) -> Option<Value> {
    let result = crate::protocol::import(value, true).ok()?;
    let prototype = crate::protocol::import(&value["prototype"], true).ok()?;
    let result = if result.shape().is_empty() && !result.is_atom() && result.at(0).is_atom() { result.at(0) } else { result };
    result.prototype().same(&prototype).then_some(result)
}

/// The default relative and absolute tolerances for comparing numbers.
const TOLERANCE: f64 = 1e-13;
/// The units in the last place of a value's float width that it may differ by from what it's compared with, where a case sets no relative tolerance. Two results that each lie within rustymath's 4 units of the true value lie within 8 units of each other.
const UNITS: f64 = 8.0;

/// Numbers match when they are equal: exact ones exactly, and others within the tolerances. Without a relative tolerance, they match within `TOLERANCE`, or within `UNITS` units in the last place of the float width of `x`, the value under test, where that's greater. With `exactness`, an exact number never matches an approximate one. Other atoms match when they are the same.
fn same_atom(x: &Value, y: &Value, relative: Option<f64>, absolute: f64, exactness: bool) -> bool {
    match (x, y) {
        (Value::Number(x), Value::Number(y)) if exactness && (x.is_exact() != y.is_exact() || x.as_bool().is_some() != y.as_bool().is_some()) => false,
        (Value::Number(x), Value::Number(y)) if x.is_nan() && y.is_nan() => true,
        (Value::Number(x), Value::Number(y)) if !x.is_exact() || !y.is_exact() => {
            let (Ok(a), Ok(b)) = (x.to_complex(), y.to_complex()) else { return false };
            let relative = relative.unwrap_or_else(|| TOLERANCE.max(UNITS * x.float_width().map_or(f64::EPSILON, |w| w.epsilon())));
            a == b || (a.is_finite() && b.is_finite() && (a - b).norm() <= absolute.max(relative * a.norm().max(b.norm())))
        }
        (Value::Number(x), Value::Number(y)) => x == y,
        _ => x.same(y),
    }
}

/// Locate a structural or numeric mismatch of the value `x` from `y`. Approximate numbers may differ by the tolerances that `same_atom` takes. With `exactness`, each number's exactness must match too. Items compare before prototypes, because a nested array's prototype comes from its first item. A mismatch inside an item that is an array adds its own message, as in `data[0]: shape: [0] != [1]`.
pub fn difference(x: &Value, y: &Value, relative: Option<f64>, absolute: f64, exactness: bool) -> Option<String> {
    let inner = |a: &Value, b: &Value| {
        if a.is_atom() && b.is_atom() { (!same_atom(a, b, relative, absolute, exactness)).then(String::new) }
        else { difference(a, b, relative, absolute, exactness).map(|m| format!(": {m}")) }
    };
    if x.is_atom() != y.is_atom() { return Some("atom versus array".into()); }
    if x.is_atom() { return inner(x, y).map(|_| "atom".into()); }
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if x.layout().all_keys() != y.layout().all_keys() || x.axis_names() != y.axis_names() { return Some("axis keys or names".into()); }
    let items = x.elements().zip(y.elements()).enumerate().find_map(|(i, (a, b))| inner(&a, &b).map(|m| format!("data[{i}]{m}")));
    items.or_else(|| inner(&x.prototype(), &y.prototype()).map(|m| format!("prototype{m}")))
}

/// The kinds of storage, as `•storage` names them under `numeric`, at the first place where `x` and `y` are held differently. It walks into the items of arrays that both hold as mixed storage. With `compact_alike`, every compact kind counts as one, so only mixed storage against compact storage differs.
fn kind_difference(x: &Value, y: &Value, numeric: Numeric, compact_alike: bool) -> Option<(&'static str, &'static str)> {
    let (a, b) = (x.storage_kind(numeric), y.storage_kind(numeric));
    let differ = if compact_alike { (a == "mixed") != (b == "mixed") } else { a != b };
    if differ { return Some((a, b)); }
    if a != "mixed" || x.is_atom() || y.is_atom() || x.len() != y.len() { return None; }
    x.elements().zip(y.elements()).find_map(|(p, q)| kind_difference(&p, &q, numeric, compact_alike))
}

/// The text at `key` in `case`, or `None` without one. Anything else makes the case invalid, as `what` describes.
fn optional_text<'a>(case: &'a JsonValue, key: &str, what: &str) -> Result<Option<&'a str>, JsonValue> {
    match case.get(key) {
        None => Ok(None), Some(JsonValue::String(s)) => Ok(Some(s)), _ => Err(json!({"status":"invalid", "message":format!("{what} must be text")}))
    }
}

/// The input lines a case supplies, which `⎕` reads in order.
struct Lines(Mutex<VecDeque<String>>);
impl crate::Input for Lines {
    fn line(&self, _prompt: &str) -> std::io::Result<Option<String>> { Ok(self.0.lock().unwrap().pop_front()) }
    fn rest(&self) -> std::io::Result<Vec<u8>> { Ok(self.0.lock().unwrap().drain(..).map(|line| line + "\n").collect::<String>().into_bytes()) }
}

/// A run of a case: whether the `batch` pref is on, whether fusion takes its small sizes (`fusion::Sizes::SMALL`), when block stages compile their parts (`settings::Compiles`), whether the code runs as the body of a dfn called twice (`twice`), and the words that name the run in a failure.
#[derive(Clone, Copy)]
struct Run { batch: bool, fused: bool, compiles: Compiles, twice: bool, words: &'static str }
/// The runs of a case without a `batch` field: the `batch` pref off, then on, then on with fusion's small sizes, under which every fusible call waits, a block stage splits even small arrays into blocks and every stage compiles its parts, then the same with no compiled parts, then on with the code as a dfn's body, whose second call runs the statements that its first call recorded. The run with no compiled parts takes place only where the run before it used one (`failures`).
const RUNS: [Run; 5] = [
    Run { batch: false, fused: false, compiles: Compiles::Policy, twice: false, words: "" },
    Run { batch: true, fused: false, compiles: Compiles::Policy, twice: false, words: "with the batch pref on" },
    Run { batch: true, fused: true, compiles: Compiles::Always, twice: false, words: "with fusion at any size" },
    Run { batch: true, fused: true, compiles: Compiles::Never, twice: false, words: "with fusion at any size and no compiled parts" },
    Run { batch: true, fused: false, compiles: Compiles::Policy, twice: true, words: "on a dfn's second call" },
];

/// The header options that set a flag on a case, each setting the field of its own name to `true`: `unfused` leaves out the runs with fusion's small sizes, `batched` leaves out the run with the `batch` pref off, `mixed` leaves out the comparison of the result's storage with the expectation's, and `compiled` leaves out the run with no compiled parts, for a case whose value says whether a stage compiled.
const FLAGS: [&str; 4] = ["unfused", "batched", "mixed", "compiled"];

/// The runs that `case` takes: the one of `RUNS` that its Boolean `batch` field picks, where it has one, and otherwise each of `RUNS` that its `unfused`, `batched` and `compiled` options leave in.
fn runs(case: &JsonValue) -> Vec<Run> {
    match case["batch"].as_bool() {
        Some(batch) => vec![RUNS[usize::from(batch)]],
        None => RUNS.into_iter().filter(|r| !(r.fused && case["unfused"] == true) && (r.batch || case["batched"] != true) && !(r.compiles == Compiles::Never && case["compiled"] == true)).collect(),
    }
}

/// Defaults, with each program checked before it runs, and file/section preferences, optionally overridden by a run's batch, fusion and compile settings.
fn enter(prefs: Option<&str>, run: Option<Run>) -> Result<(), JsonValue> {
    crate::settings::reset();
    crate::settings::set_verifies(true);
    if let Some(prefs) = prefs && let Some(error) = Session::current().eval_apart(prefs, EvalOptions { timeout: Some(TIMEOUT), echo: false, ..EvalOptions::default() }).error {
        return Err(json!({"status":"invalid", "message":format!("file header: {error}")}));
    }
    if let Some(Run { batch, fused, compiles, .. }) = run {
        crate::settings::set_batch(batch);
        crate::settings::set_small_sizes(fused);
        crate::settings::set_compiles(compiles);
    }
    Ok(())
}

/// One run of a case: the value its code gave, the numeric settings in force after it, and the value its BPL expectation gave.
struct Outcome { value: Option<Value>, numeric: Numeric, expected: Option<Value> }

/// A case's relative tolerance for comparing numbers, where its header sets one, and its absolute tolerance, `TOLERANCE` unless its header sets it.
fn tolerances(case: &JsonValue) -> (Option<f64>, f64) { (case["relative_tolerance"].as_f64(), case["absolute_tolerance"].as_f64().unwrap_or(TOLERANCE)) }

/// `case`'s code checked against an independent captured value or BPL expectation, in `run`, under the settings in force. Each side receives a fresh session, and the expectation evaluates after the code, under the settings that the code leaves. A BPL expectation must match each number's exactness. A value captured from another interpreter carries none, so its numbers compare by value. A failure's message names its run.
fn outcome(case: &JsonValue, setup: &str, Run { words, .. }: Run, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<Outcome, JsonValue> {
    let Some(code) = case["code"].as_str() else { return Err(json!({"status":"invalid", "message":"missing code"})); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let (output, input) = (optional_text(case, "expected_output", "output expectation")?, optional_text(case, "input", "input")?);
    let result = evaluated(code, setup, input, timeout, interrupt)?;
    let (expected, no_result) = if let Some(source) = case["expected_code"].as_str() {
        let expected = evaluated(source, setup, None, timeout, interrupt)?;
        if expected.error.is_some() || expected.function.is_some() || expected.operator.is_some() {
            return Err(json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected)}));
        }
        let no_result = expected.value.is_none();
        (expected.value, no_result)
    } else { (expected_array(&case["expected"]), case.get("expected") == Some(&JsonValue::Null)) };
    let exactness = case["expected_code"].is_string();
    let (relative, absolute) = tolerances(case);
    if relative.into_iter().chain([absolute]).any(|t| !t.is_finite() || t < 0.0) || (error_kind.is_none() && expected.is_none() && !no_result) {
        return Err(json!({"status":"invalid", "message":"invalid or missing independent expectation"}));
    }
    let failed = |mut failure: JsonValue| {
        if !words.is_empty() { failure["message"] = json!(format!("{words}: {}", failure["message"].as_str().unwrap_or_default())); }
        Err(failure)
    };
    if let Some(error) = &result.error {
        let (kind, message) = (error.kind.to_string(), error.message.clone());
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) { return failed(json!({"status": kind.to_lowercase(), "message":message})); }
        if error_kind != Some(kind.as_str()) { return failed(json!({"status":"error", "kind":kind, "message":message, "actual":crate::protocol::response(result)})); }
    }
    let mismatch = if result.error.is_some() { None }
    else if result.function.is_some() || result.operator.is_some() { Some("unexpected function or operator result".into()) } else {
        match (error_kind, &result.value, &expected) {
            (Some(kind), _, _) => Some(format!("expected {kind}")), (_, None, _) if no_result => None, (_, None, _) => Some("no result".into()),
            (_, Some(_), _) if no_result => Some("expected no result".into()),
            (_, Some(actual), Some(expected)) => difference(actual, expected, relative, absolute, exactness), _ => unreachable!()
        }
    };
    let mismatch = mismatch.or_else(|| {
        output.and_then(|expected| {
            let written: String = result.output.iter().map(crate::Output::written).collect();
            let actual = written.strip_suffix('\n').unwrap_or(&written);
            (actual != expected).then(|| format!("output: {actual:?} != {expected:?}"))
        })
    });
    match mismatch {
        Some(message) => failed(json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result)})),
        None => Ok(Outcome { value: result.value, numeric: crate::settings::numeric(), expected }),
    }
}

/// The name of the dfn that `twice` makes from a case's code.
const TWICE: &str = "reference_case";

/// `case`'s code as the body of a dfn that one session calls twice, under the settings in force. The first call runs each statement through the binder, which records it, and the second call runs the recordings. The second call must give the first's value, storage kind, error and output. Before each call, the session's random numbers restart from seed 0, each call reads its own `input` lines, and the second call finds the `testpath` fixture's directory emptied. The settings go back in force before the second call, because the first call can change them. Code that can't be a dfn's body, such as code with a syntax error, passes this run unchecked.
fn twice(case: &JsonValue, setup: &str, run: Run, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<Outcome, JsonValue> {
    let Some(code) = case["code"].as_str() else { return Err(json!({"status":"invalid", "message":"missing code"})); };
    let input = optional_text(case, "input", "input")?;
    let options = || EvalOptions { timeout, interrupt: interrupt.clone(), echo: false, ..EvalOptions::default() };
    let session = Session::new();
    initialized(&session, setup)?;
    let files = fixture(&session, code)?;
    let mut calls = Vec::new();
    if session.eval_with(&format!("{TWICE}←{{\n{code}\n}}"), options()).error.is_none() {
        for call in 0..2 {
            if call == 1 { enter(case["prefs"].as_str(), Some(run))?; }
            if call == 1 && let Some(dir) = &files { emptied(dir.path())?; }
            *session.generator.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = crate::distribution::Stream::seeded(0);
            calls.push(session.eval_with(&format!("{TWICE} 0"), EvalOptions { input: lines(input), ..options() }));
        }
    }
    let numeric = crate::settings::numeric();
    let Ok([first, second]) = <[crate::Evaluation; 2]>::try_from(calls) else { return Ok(Outcome { value: None, numeric, expected: None }) };
    let words = run.words;
    for error in [&first.error, &second.error].into_iter().flatten() {
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) {
            return Err(json!({"status":error.kind.to_string().to_lowercase(), "message":format!("{words}: {}", error.message)}));
        }
    }
    let error = |e: &crate::Evaluation| e.error.as_ref().map(|e| (e.kind.to_string(), e.message.clone()));
    let written = |e: &crate::Evaluation| e.output.iter().map(crate::Output::written).collect::<String>();
    let category = |e: &crate::Evaluation| (e.function.is_some(), e.operator.is_some());
    let (relative, absolute) = tolerances(case);
    let mismatch = if error(&first) != error(&second) { Some(format!("error {:?}, {:?} on the first call", error(&second), error(&first))) }
        else if written(&first) != written(&second) { Some(format!("output {:?}, {:?} on the first call", written(&second), written(&first))) }
        else if category(&first) != category(&second) { Some("a function or operator from one call only".into()) }
        else {
            match (&first.value, &second.value) {
                (Some(a), Some(b)) => difference(b, a, relative, absolute, true)
                    .or_else(|| kind_difference(b, a, numeric, false).map(|(kind, first_kind)| format!("storage kind: {kind}, {first_kind} on the first call"))),
                (None, None) => None,
                _ => Some("a value from one call only".into()),
            }
        };
    match mismatch {
        Some(message) => Err(json!({"status":"mismatch", "message":format!("{words}: {message}"), "actual":crate::protocol::response(second)})),
        None => Ok(Outcome { value: None, numeric, expected: None }),
    }
}

/// Whether a case's runs agree: each later run's value must be held in the same kind of storage as the first's. Where the expectation is BPL, the first's storage must be mixed where the expectation's is, and compact where it's compact. A `mixed` case leaves the storage checks out. An in-place write keeps mixed storage mixed, and a record from JSON or Python keeps each entry's own kind.
fn verdict(case: &JsonValue, outcomes: &[(Run, Outcome)]) -> JsonValue {
    let Some((first_run, first)) = outcomes.first() else { return json!({"status":"pass"}) };
    let named = |run: &'static str| if run.is_empty() { "with the batch pref off" } else { run };
    for (run, outcome) in &outcomes[1..] {
        if let (Some(value), Some(first)) = (&outcome.value, &first.value) && let Some((kind, first_kind)) = kind_difference(value, first, outcome.numeric, false) {
            return json!({"status":"mismatch", "message":format!("storage kind: {kind} {}, {first_kind} {}", named(run.words), named(first_run.words))});
        }
    }
    if case["expected_code"].is_string() && case["mixed"] != true && let (Some(value), Some(expected)) = (&first.value, &first.expected)
        && let Some((kind, expected_kind)) = kind_difference(value, expected, first.numeric, true) {
        return json!({"status":"mismatch", "message":format!("storage kind: {kind}, {expected_kind} in the expectation")});
    }
    json!({"status":"pass"})
}

/// `case` checked as `failures` checks the cases of a file, then against its route directives (`route_check`): `{"status":"pass"}`, or the case with the `status`, `message` and `kind` of its failure, and its `actual` result where there is one. Python's `check_file` checks each case this way. Within `cargo test`, only the explain test checks route directives.
pub fn check(case: &JsonValue, interrupt: &InterruptHandle) -> JsonValue {
    failures([case], "", interrupt).pop().or_else(|| route_check(case, "", interrupt).err().map(|result| failed(case, &result))).unwrap_or_else(|| json!({"status":"pass"}))
}

/// A timed record's expectation, checked once outside timing, with fresh setup for both expressions.
pub fn check_once(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<(), JsonValue> {
    enter(case["prefs"].as_str(), None)?;
    outcome(case, setup, RUNS[0], Some(TIMEOUT), interrupt).map(drop)
}

/// Warm and capture a case under its ordinary preferences and setup.
pub fn route(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<crate::timing::Capture, JsonValue> {
    enter(case["prefs"].as_str(), None)?;
    let session = Session::new();
    initialized(&session, setup)?;
    crate::timing::route(&session, case, interrupt).map_err(|message| json!({"status":"invalid", "message":message}))
}

pub fn routed(case: &JsonValue) -> bool { ["route", "allows", "gap"].iter().any(|key| case.get(key).is_some()) }

fn route_check(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<(), JsonValue> {
    if !routed(case) { return Ok(()); }
    let wrong = route(case, setup, interrupt)?.violations();
    if wrong.is_empty() { Ok(()) } else { Err(json!({"status":"route", "message":wrong.join("; ")})) }
}
/// `case` with the `status`, `message` and `kind` of the failure `result`, and its `actual` result where there is one.
fn failed(case: &JsonValue, result: &JsonValue) -> JsonValue {
    let mut failure = case.clone();
    for key in ["status", "message", "kind", "actual"] { if let Some(value) = result.get(key) { failure[key] = value.clone(); } }
    failure
}
fn initialized(session: &Session, setup: &str) -> Result<(), JsonValue> {
    if setup.is_empty() { return Ok(()); }
    if let Some(error) = session.eval_with(setup, EvalOptions { echo: false, timeout: Some(TIMEOUT), ..EvalOptions::default() }).error {
        return Err(json!({"status":"invalid", "message":format!("setup: {error}")}));
    }
    Ok(())
}

/// `code` run in a fresh session with the settings in force, a `testpath` fixture where the code names one, and `input` as its lines of input.
fn evaluated(code: &str, setup: &str, input: Option<&str>, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<crate::Evaluation, JsonValue> {
    let session = Session::new();
    initialized(&session, setup)?;
    let _files = fixture(&session, code)?;
    Ok(session.eval_with(code, EvalOptions { timeout, interrupt: interrupt.clone(), echo: false, input: lines(input), ..EvalOptions::default() }))
}

/// A new temporary directory for the `testpath` fixture, where `code` names one, with `testpath` in `session` naming a path inside it. The directory lasts as long as the returned guard.
fn fixture(session: &Session, code: &str) -> Result<Option<tempfile::TempDir>, JsonValue> {
    if !code.contains("testpath") { return Ok(None); }
    let dir = tempfile::tempdir().map_err(unusable)?;
    session.set("testpath", crate::keyed::text(&dir.path().join("data").to_string_lossy())).expect("valid fixture name");
    Ok(Some(dir))
}

/// Removes everything in `dir`, a fixture's directory.
fn emptied(dir: &std::path::Path) -> Result<(), JsonValue> {
    for entry in std::fs::read_dir(dir).map_err(unusable)? {
        let path = entry.map_err(unusable)?.path();
        if path.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) }.map_err(unusable)?;
    }
    Ok(())
}

/// The failure of a case whose fixture directory can't be made or emptied.
fn unusable(error: std::io::Error) -> JsonValue { json!({"status":"invalid", "message":format!("temporary fixture: {error}")}) }

/// `input` as a case's lines of input.
fn lines(input: Option<&str>) -> Option<Arc<dyn crate::Input>> { input.map(|text| Arc::new(Lines(Mutex::new(text.lines().map(String::from).collect()))) as Arc<dyn crate::Input>) }

/// How long each evaluation of a case or a page example may run.
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// The id and comment of a case header, `⍝ id — comment`, or `⍝ — comment` with no id.
fn header(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix("⍝ ")?;
    let (id, comment) = match line.strip_prefix('—') { Some(comment) => ("", comment), None => line.split_once(" —")? };
    if id.chars().any(char::is_whitespace) { return None; }
    Some((id, if comment.is_empty() { "" } else { comment.strip_prefix(' ')? }))
}

/// The code and expectation of `code ⍝ expectation`, split at the first `⍝` outside a string or character.
fn inline(line: &str) -> Option<(&str, &str)> {
    let mut chars = line.char_indices();
    let mut string = false;
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => string = !string, '\'' if !string => { chars.next(); if !line[i..].starts_with("''") || line[i..].starts_with("'''") { chars.next(); } }
            '⍝' if !string => {
                if line[i..].contains('\n') { for (_, c) in chars.by_ref() { if c == '\n' { break; } } }
                else { return Some((line[..i].trim_end(), line[i + c.len_utf8()..].trim_start())); }
            }
            _ => ()
        }
    }
    None
}

/// `text` with its `\n` and `\\` escapes replaced.
fn unescaped(text: &str, location: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        result.push(if c == '\\' {
            match chars.next() { Some('n') => '\n', Some('\\') => '\\', _ => return Err(format!("{location}: input and output escapes are \\n and \\\\")) }
        } else { c });
    }
    Ok(result)
}

/// One BPL test file: a shared setup and records which check a result, carry timing budgets, or both.
pub struct File { pub name: String, pub setup: String, pub cases: Vec<JsonValue> }

/// Whether a record supplies an independent result or error expectation.
pub fn checked(case: &JsonValue) -> bool { ["expected", "expected_code", "expected_error"].iter().any(|key| case.get(key).is_some()) }

/// A section heading, excluding the per-case route annotations.
fn section(line: &str) -> Option<&str> { line.strip_prefix("⍝⍝ ") }

/// The record and event in a route annotation. Doubled quotes stand for one quote.
fn event(rest: &str) -> JsonValue {
    let (record, quoted) = rest.split_once(' ').unwrap_or((rest, ""));
    let (mut event, mut chars) = (String::new(), quoted.trim_start().strip_prefix('"').unwrap_or_default().chars().peekable());
    while let Some(c) = chars.next() {
        if c == '"' && chars.next_if_eq(&'"').is_none() { break; }
        event.push(c);
    }
    json!([record, event])
}

/// Reads reference and speed records. Footer spans are byte offsets into `text`, excluding the newline.
pub fn file(name: &str, text: &str) -> Result<File, String> {
    let lines: Vec<_> = text.split_terminator('\n').collect();
    let mut offsets = Vec::with_capacity(lines.len());
    let mut offset = 0;
    for line in &lines { offsets.push(offset); offset += line.len() + 1; }
    let prefs = lines.first().filter(|line| line.starts_with("•prefs")).copied().unwrap_or_default();
    let (mut i, mut title, mut local) = (usize::from(!prefs.is_empty()), "", "");
    let mut file = File { name: name.into(), setup: String::new(), cases: Vec::new() };
    let mut ids = rustymath::hashing::HashSet::default();
    while i < lines.len() {
        if lines[i].is_empty() { i += 1; continue; }
        if let Some(heading) = section(lines[i]) {
            let start = i;
            i += 1;
            if heading == "setup" {
                if !file.setup.is_empty() || !file.cases.is_empty() { return Err(format!("line {}: setup must be the first section", start + 1)); }
                while i < lines.len() && section(lines[i]).is_none() { i += 1; }
                file.setup = lines[start + 1..i].join("\n").trim_end().into();
            } else {
                (title, local) = heading.split_once(" •prefs").map_or((heading, ""), |(name, _)| (name, &heading[name.len() + 1..]));
            }
            continue;
        }
        let start = i;
        let heading = header(lines[start]);
        i += 1;
        while i < lines.len() && header(lines[i]).is_none() && section(lines[i]).is_none() && !lines[i - 1].starts_with("⍝⍝:") { i += 1; }
        let (id, comment) = heading.unwrap_or(("", ""));
        let location = format!("{}line {} ({id})", if name.is_empty() { String::new() } else { format!("{name}: ") }, start + 1);
        let fail = |message: String| format!("{location}: {message}");
        if !id.is_empty() && !ids.insert(id) { return Err(fail(format!("duplicate ID {id}"))); }
        let suffix = comment.strip_prefix('[').map(|s| ("", s)).or_else(|| comment.rsplit_once(" ["));
        let options = suffix.and_then(|(text, s)| Some((text, s.strip_suffix(']')?))).filter(|(_, s)| s.starts_with("rtol=") || s.starts_with("atol=") || FLAGS.iter().any(|f| s.starts_with(f)));
        let mut case = json!({"id":id, "line":start + 1, "section":title, "comment":options.map_or(comment, |(text, _)| text)});
        let prefs = [prefs, local].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
        if !prefs.is_empty() { case["prefs"] = json!(prefs); }
        if let Some((_, options)) = options {
            for option in options.split_whitespace() {
                if FLAGS.contains(&option) {
                    if case.get(option).is_some() { return Err(fail(format!("repeated option {option}"))); }
                    case[option] = json!(true);
                    continue;
                }
                let invalid = || fail(format!("invalid tolerance {option}"));
                let (key, value) = option.split_once('=').ok_or_else(invalid)?;
                let key = match key { "rtol" => "relative_tolerance", "atol" => "absolute_tolerance", _ => return Err(fail(format!("unknown tolerance {key}"))) };
                let value: f64 = value.parse().map_err(|_| invalid())?;
                if !value.is_finite() || value < 0. || case.get(key).is_some() { return Err(invalid()); }
                case[key] = json!(value);
            }
        }
        let mut end = i;
        while end > start && lines[end - 1].is_empty() { end -= 1; }
        let mut body = &lines[start + usize::from(heading.is_some())..end];
        if let Some(budgets) = body.last().and_then(|line| line.strip_prefix("⍝⍝:")) {
            let fields: Vec<_> = budgets.split_whitespace().collect();
            if fields.is_empty() || fields.iter().any(|s| s.parse::<f64>().map_or(true, |v| !v.is_finite() || v < 0.)) { return Err(fail("invalid timing budgets".into())); }
            case["budgets"] = json!(fields);
            case["budget_span"] = json!([offsets[end - 1], offsets[end - 1] + lines[end - 1].len()]);
            body = &body[..body.len() - 1];
        } else if heading.is_none() { return Err(fail("expected a case header or a timing footer".into())); }
        else if i < lines.len() && end == i { return Err(fail("missing blank record separator".into())); }
        let mut comments = Vec::new();
        let mut leading = 0;
        for line in body {
            if let Some(directive) = line.strip_prefix("⍝⍝⍝ ") {
                let (key, rest) = directive.split_once(' ').ok_or_else(|| fail("a route directive needs its name".into()))?;
                let value = match key {
                    "route" if !rest.trim().is_empty() => json!(rest.trim()),
                    "allows" | "gap" => event(rest),
                    _ => return Err(fail(format!("invalid route directive {directive}"))),
                };
                case.as_object_mut().unwrap().entry(key).or_insert_with(|| json!([])).as_array_mut().unwrap().push(value);
            }
            else if let Some(rest) = line.strip_prefix("⍝ ") { if heading.is_none() { comments.push(rest); } }
            else { break; }
            leading += 1;
        }
        if heading.is_none() { body = &body[leading..]; }
        if !comments.is_empty() { case["comment"] = json!(comments.join("\n")); }
        for (marker, key) in [("⍝ ⎕:", "expected_output"), ("⍝ input:", "input")] {
            if let Some(text) = body.last().and_then(|s| s.strip_prefix(marker)) {
                case[key] = json!(unescaped(text.strip_prefix(' ').unwrap_or(text), &location)?);
                body = &body[..body.len() - 1];
            }
        }
        if body.iter().any(|s| s.starts_with("⍝ ⎕:") || s.starts_with("⍝ input:")) { return Err(fail("input and output lines must end the case, input first".into())); }
        let splits: Vec<_> = body.iter().enumerate().filter_map(|(i, s)| (*s == "⍝ =>").then_some(i)).collect();
        let joined = body.join("\n");
        let pair = inline(&joined).filter(|(code, _)| body.len() == 1 || code.matches('\n').count() + 1 == body.len());
        let (code, expect) = match (splits.as_slice(), pair) {
            ([], Some((code, expect))) => (code.to_owned(), Some(expect.to_owned())),
            ([], None) if case["budgets"].is_array() => (joined, None),
            ([], None) if body.len() == 2 || heading.is_some() && body.len() == leading + 2 => (body[..body.len() - 1].join("\n"), Some(body[body.len() - 1].to_owned())),
            ([i], _) => (body[..*i].join("\n"), Some(body[i + 1..].join("\n"))),
            _ => return Err(fail("use one ⍝ => between multiline expressions, or an inline expectation on the last line".into()))
        };
        if code.is_empty() && expect.is_none() || expect.as_ref().is_some_and(String::is_empty) { return Err(fail("missing code or expectation".into())); }
        case["code"] = json!(code);
        if let Some(expect) = expect {
            if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); }
            else { case["expected_code"] = json!(expect); }
        }
        file.cases.push(case);
    }
    Ok(file)
}

/// Reads named source texts supplied by the worker or browser caller.
pub fn sources(spec: &JsonValue) -> Result<Vec<File>, String> {
    spec.as_array().ok_or("files must be a list of named source texts")?.iter().map(|s| {
        file(s["name"].as_str().ok_or("file needs name")?, s["text"].as_str().ok_or("file needs text")?)
    }).collect()
}

/// Reads one BPL file, or the BPL files in a directory, in filename order.
#[cfg(not(web))]
pub fn load(path: &std::path::Path) -> Result<Vec<File>, String> {
    let mut paths = if path.is_dir() {
        std::fs::read_dir(path).map_err(|e| e.to_string())?.map(|p| p.map(|p| p.path())).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    } else { vec![path.to_owned()] };
    paths.retain(|p| p.extension().is_some_and(|s| s == "bpl"));
    if paths.is_empty() { return Err(format!("{} contains no BPL files", path.display())); }
    paths.sort();
    paths.into_iter().map(|p| file(&p.file_name().unwrap().to_string_lossy(), &std::fs::read_to_string(&p).map_err(|e| e.to_string())?)).collect()
}


/// The cases of `cases` whose runs fail their expectations, in their order. Route directives aren't checked here: `check` and the explain test check them. Each failure adds the case's `status` and `message`, and the `kind` of the error that the case gave. Each case takes its runs (`runs`) one after another, each under its file's leading `•prefs` header and the run's settings (`enter`), and each run has `TIMEOUT`. Callers that check cases in parallel split them across processes, as `tests/reference.rs` and Python's `check_file` do, because a process evaluates one case at a time. After an interrupt, no case starts. The settings are reset at the end.
pub fn failures<'a>(cases: impl IntoIterator<Item = &'a JsonValue>, setup: &str, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let mut failures = Vec::new();
    for case in cases {
        let mut outcomes = Vec::new();
        let mut compiled = false;
        let result = runs(case).into_iter().try_for_each(|run| {
            if run.compiles == Compiles::Never && !compiled { return Ok(()); }
            enter(case["prefs"].as_str(), Some(run))?;
            let before = crate::program::blocks::compiled_runs();
            outcomes.push((run, if run.twice { twice(case, setup, run, Some(TIMEOUT), interrupt) } else { outcome(case, setup, run, Some(TIMEOUT), interrupt) }?));
            compiled |= run.compiles == Compiles::Always && crate::program::blocks::compiled_runs() > before;
            Ok(())
        }).map_or_else(|failure| failure, |()| verdict(case, &outcomes));
        if result["status"] == "pass" { continue; }
        failures.push(failed(case, &result));
        if result["status"] == "interrupt" { break; }
    }
    crate::settings::reset();
    failures
}

/// The examples in the `bpl` blocks of `text`, a `.qmd` page, that fail. Each block runs in a fresh session of its own, with the `batch` pref set to `batch` unless that is `None`. Each expectation runs in a module of its own after its lines (`Session::eval_apart`), under the settings they leave. The lines up to each line with a ` ⍝ ` comment, and up to the block's end, run as one evaluation, with `TIMEOUT` to run. Its value must show the same text as the comment's value, in the session's display with every item, or match it within the default tolerances that `same_atom` describes, with the same exactness. `•prefs` option `prec` sets how many digits the display shows. Lines with no comment must run without an error. Each page starts with no kept plans or compiled kernels, as a new process does (`program::forget_plans`, `jit::forget_kernels`), because examples that show block counts and compiles assume one. Each failure gives the `line` of its comment or of the block's end, the `code` that ran, its `expected` comment and a `message`.
pub fn page(text: &str, batch: Option<bool>, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let options = || EvalOptions { timeout: Some(TIMEOUT), echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    let (mut failures, mut session, mut code) = (Vec::new(), None, String::new());
    crate::program::forget_plans();
    #[cfg(not(web))]
    crate::jit::forget_kernels();
    for (i, line) in text.lines().enumerate() {
        if line == "```bpl" {
            crate::settings::reset();
            if let Some(batch) = batch { crate::settings::set_batch(batch); }
            session = Some(Session::new());
            continue;
        }
        let Some(bpl) = session.as_mut() else { continue };
        let closing = line.starts_with("```");
        let (source, expected) = line.split_once(" ⍝ ").map_or((line, None), |(c, e)| (c, Some(e)));
        if !closing {
            code.push_str(source);
            code.push('\n');
        }
        if expected.is_none() && !closing { continue; }
        let result = bpl.eval_with(&code, options());
        // The expectation evaluates after the code, under the settings that the code leaves.
        let wanted = expected.map(|e| bpl.eval_apart(e, options()));
        let message = match (result.error, result.value, wanted) {
            (Some(error), _, _) => Some(error.to_string()),
            (None, _, None) => None,
            (None, actual, Some(wanted)) => {
                let shown = bpl.display_settings();
                match (actual, wanted.value, wanted.error) {
                    (_, _, Some(error)) => Some(format!("expectation: {error}")),
                    (Some(actual), Some(wanted), None) if shown.explicit(&actual, false) == shown.explicit(&wanted, false) => None,
                    (Some(actual), Some(wanted), None) => difference(&actual, &wanted, None, TOLERANCE, true), _ => Some("no result".into())
                }
            }
        };
        if let Some(message) = message { failures.push(json!({"line": i + 1, "code": code.trim_end(), "expected": expected, "message": message})); }
        code.clear();
        if closing { session = None; }
    }
    failures
}

/// The reply to the worker's `reference` request. It gives `text` in a `.bpl` reference file's format and, optionally, a list of the `ids` to check, each a case's id or its header's line number. An id that matches no case is an error. The reply gives the cases that fail, as `failures` gives them.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let file = file(spec["name"].as_str().unwrap_or_default(), spec["text"].as_str().ok_or("reference needs text as a string")?)?;
    let cases = &file.cases;
    let ids = match spec.get("ids") {
        None | Some(JsonValue::Null) => None, Some(ids) => Some(ids.as_array().ok_or("ids must be a list of case ids and header line numbers")?)
    };
    let matches = |id: &JsonValue, case: &JsonValue| *id == case["id"] || *id == case["line"];
    let unknown: Vec<_> = ids.into_iter().flatten().filter(|id| !cases.iter().any(|case| matches(id, case))).map(JsonValue::to_string).collect();
    if !unknown.is_empty() { return Err(format!("{} match no case. Each id is a case's id or its header's line number.", unknown.join(", "))); }
    let selected = cases.iter().filter(|case| checked(case) && ids.is_none_or(|ids| ids.iter().any(|id| matches(id, case))));
    Ok(json!({"failures": failures(selected, &file.setup, interrupt), "error": null}))
}

/// The reply to the worker's `page` request, which gives the `text` of a `.qmd` page and, optionally, a Boolean `batch` that sets the `batch` pref for its examples. The reply gives the examples that fail, as `page` gives them.
pub fn page_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    Ok(json!({"failures": page(spec["text"].as_str().ok_or("page needs text as a string")?, spec["batch"].as_bool(), interrupt), "error": null}))
}
