//! Development-only checks, shared by the tests and the worker: reference cases, `.bpl` reference files and the examples in `.qmd` pages.
use crate::{number::Numeric, EvalOptions, InterruptHandle, Session, Value};
use serde_json::{json, Value as JsonValue};
use std::{collections::VecDeque, sync::{Arc, Mutex}};

/// A value captured from another interpreter. Its numbers carry no exactness. A capture encodes a simple atom as a scalar array, and BPL expectations use based values directly.
pub fn expected_array(value: &JsonValue) -> Option<Value> {
    let result = crate::protocol::import(value, true).ok()?;
    let prototype = crate::protocol::import(&value["prototype"], true).ok()?;
    let result = if result.shape().is_empty() && !result.is_atom() && result.at(0).is_atom() { result.at(0) } else { result };
    result.prototype().same(&prototype).then_some(result)
}

/// Numbers match when they are equal: exact ones exactly, and others within the tolerances. With `exactness`, an exact number never matches an approximate one. Other atoms match when they are the same.
fn same_atom(x: &Value, y: &Value, relative: f64, absolute: f64, exactness: bool) -> bool {
    match (x, y) {
        (Value::Number(x), Value::Number(y)) if exactness && (x.is_exact() != y.is_exact() || x.as_bool().is_some() != y.as_bool().is_some()) => false,
        (Value::Number(x), Value::Number(y)) if x.is_nan() && y.is_nan() => true,
        (Value::Number(x), Value::Number(y)) if !x.is_exact() || !y.is_exact() => {
            let (Ok(a), Ok(b)) = (x.to_complex(), y.to_complex()) else { return false };
            a == b || (a.is_finite() && b.is_finite() && (a - b).norm() <= absolute.max(relative * a.norm().max(b.norm())))
        }
        _ => x.same(y),
    }
}

/// Locate a structural or numeric mismatch. Approximate numbers may differ by the tolerances. With `exactness`, each number's exactness must match too. Items compare before prototypes, because a nested array's prototype comes from its first item. A mismatch inside an item that is an array adds its own message, as in `data[0]: shape: [0] != [1]`.
pub fn difference(x: &Value, y: &Value, relative: f64, absolute: f64, exactness: bool) -> Option<String> {
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

/// A run of a case: whether the `batch` pref is on, whether fusion takes its small sizes (`fusion::Sizes::SMALL`), and the words that name the run in a failure.
type Run = (bool, bool, &'static str);
/// The runs of a case without a `batch` field: the `batch` pref off, then on, then on with fusion's small sizes, under which every fusible call waits and a graph splits even small arrays into blocks.
const RUNS: [Run; 3] = [(false, false, ""), (true, false, "with the batch pref on"), (true, true, "with fusion at any size")];

/// The header options that set a flag on a case, each setting the field of its own name to `true`: `unfused` leaves out the run with fusion's small sizes, `batched` leaves out the run with the `batch` pref off, `mixed` leaves out the comparison of the result's storage with the expectation's, and `serial` runs the case alone, as a case that calls `•prefs` must run.
const FLAGS: [&str; 4] = ["unfused", "batched", "mixed", "serial"];

/// The runs that `case` takes: one with its Boolean `batch` field where it has one, and otherwise each of `RUNS` that its `unfused` and `batched` options leave in.
fn runs(case: &JsonValue) -> Vec<Run> {
    match case["batch"].as_bool() {
        Some(batch) => vec![(batch, false, "")],
        None => RUNS.into_iter().filter(|&(batch, fused, _)| !(fused && case["unfused"] == true) && (batch || case["batched"] != true)).collect(),
    }
}

/// Puts in force the settings that `run` of a case runs under, where the case's file has the leading `•prefs` header `prefs`: the defaults, then the header, then the run's `batch` pref and fusion sizes.
fn enter(prefs: Option<&str>, (batch, fused, _): Run) -> Result<(), JsonValue> {
    crate::settings::reset();
    if let Some(prefs) = prefs && let Some(error) = Session::new().eval_with(prefs, EvalOptions { timeout: Some(TIMEOUT), echo: false, ..EvalOptions::default() }).error {
        return Err(json!({"status":"invalid", "message":format!("file header: {error}")}));
    }
    crate::settings::set_batch(batch);
    crate::settings::set_small_sizes(fused);
    Ok(())
}

/// One run of a case: the value its code gave, the numeric settings in force after it, and the value its BPL expectation gave.
struct Outcome { value: Option<Value>, numeric: Numeric, expected: Option<Value> }

/// `case`'s code checked against an independent captured value or BPL expectation, in `run`, under the settings in force. Each side receives a fresh session, and the expectation evaluates after the code, under the settings that the code leaves. A BPL expectation must match each number's exactness. A value captured from another interpreter carries none, so its numbers compare by value. A failure's message names its run.
fn outcome(case: &JsonValue, (_, _, run): Run, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<Outcome, JsonValue> {
    let Some(code) = case["code"].as_str() else { return Err(json!({"status":"invalid", "message":"missing code"})); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let (output, input) = (optional_text(case, "expected_output", "output expectation")?, optional_text(case, "input", "input")?);
    let result = evaluated(code, input, timeout, interrupt)?;
    let (expected, no_result) = if let Some(source) = case["expected_code"].as_str() {
        let expected = Session::new().eval_with(source, EvalOptions { timeout, interrupt: interrupt.clone(), echo: false, ..EvalOptions::default() });
        if expected.error.is_some() || expected.function.is_some() || expected.operator.is_some() {
            return Err(json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected)}));
        }
        let no_result = expected.value.is_none();
        (expected.value, no_result)
    } else { (expected_array(&case["expected"]), case.get("expected") == Some(&JsonValue::Null)) };
    let exactness = case["expected_code"].is_string();
    let relative = case["relative_tolerance"].as_f64().unwrap_or(1e-13);
    let absolute = case["absolute_tolerance"].as_f64().unwrap_or(1e-13);
    if [relative, absolute].iter().any(|t| !t.is_finite() || *t < 0.0) || (error_kind.is_none() && expected.is_none() && !no_result) {
        return Err(json!({"status":"invalid", "message":"invalid or missing independent expectation"}));
    }
    let failed = |mut failure: JsonValue| {
        if !run.is_empty() { failure["message"] = json!(format!("{run}: {}", failure["message"].as_str().unwrap_or_default())); }
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

/// Whether a case's runs agree: each later run's value must be held in the same kind of storage as the first's. Where the expectation is BPL, the first's storage must be mixed where the expectation's is, and compact where it's compact. A `mixed` case leaves this out. An in-place write keeps mixed storage mixed, and a record from JSON or Python keeps each entry's own kind.
fn verdict(case: &JsonValue, outcomes: &[(Run, Outcome)]) -> JsonValue {
    let Some(((_, _, first_run), first)) = outcomes.first() else { return json!({"status":"pass"}) };
    let named = |run: &'static str| if run.is_empty() { "with the batch pref off" } else { run };
    for ((_, _, run), outcome) in &outcomes[1..] {
        if let (Some(value), Some(first)) = (&outcome.value, &first.value) && let Some((kind, first_kind)) = kind_difference(value, first, outcome.numeric, false) {
            return json!({"status":"mismatch", "message":format!("storage kind: {kind} {}, {first_kind} {}", named(run), named(first_run))});
        }
    }
    if case["expected_code"].is_string() && case["mixed"] != true && let (Some(value), Some(expected)) = (&first.value, &first.expected)
        && let Some((kind, expected_kind)) = kind_difference(value, expected, first.numeric, true) {
        return json!({"status":"mismatch", "message":format!("storage kind: {kind}, {expected_kind} in the expectation")});
    }
    json!({"status":"pass"})
}

/// `case` checked as `failures` checks the cases of a file: `{"status":"pass"}`, or the case with the `status`, `message` and `kind` of its failure.
pub fn check(case: &JsonValue, interrupt: &InterruptHandle) -> JsonValue { failures([case], interrupt).pop().unwrap_or_else(|| json!({"status":"pass"})) }

/// `code` run in a fresh session with the settings in force, a `testpath` fixture where the code names one, and `input` as its lines of input.
fn evaluated(code: &str, input: Option<&str>, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<crate::Evaluation, JsonValue> {
    let mut session = Session::new();
    let _files = if code.contains("testpath") {
        let dir = tempfile::tempdir().map_err(|e| json!({"status":"invalid", "message":format!("temporary fixture: {e}")}))?;
        let path = dir.path().join("data");
        session.set("testpath", crate::keyed::text(&path.to_string_lossy())).expect("valid fixture name");
        Some(dir)
    } else { None };
    let input = input.map(|text| Arc::new(Lines(Mutex::new(text.lines().map(String::from).collect()))) as Arc<dyn crate::Input>);
    let result = session.eval_with(code, EvalOptions { timeout, interrupt: interrupt.clone(), echo: false, input, ..EvalOptions::default() });
    Ok(result)
}

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
            '⍝' if !string => return Some((line[..i].trim_end(), line[i + c.len_utf8()..].trim_start())), _ => ()
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

/// The cases of `text`, in the format of a `.bpl` reference file, as `check` takes them. Each case also gives its `id`, its header's `line`, its `section`, its header's `comment` without the options, and the file's leading `•prefs` header as `prefs`, where the file has one. Two cases can't share an id. `tests/reference/README.md` describes the format.
pub fn cases(text: &str) -> Result<Vec<JsonValue>, String> {
    if text.is_empty() { return Ok(Vec::new()); }
    let lines: Vec<_> = text.split_terminator('\n').collect();
    let prefs = lines.first().filter(|line| line.starts_with("•prefs")).copied();
    let first = if prefs.is_some() { lines.iter().skip(1).position(|s| !s.is_empty()).map_or(lines.len(), |i| i + 1) } else { 0 };
    let mut starts: Vec<_> = lines.iter().enumerate().skip(first).filter_map(|(i, s)| (header(s).is_some() || s.starts_with("⍝⍝ ")).then_some(i)).collect();
    if starts.first() != Some(&first) { return Err(format!("expected a case or section header on line {}", first + 1)); }
    starts.push(lines.len());
    let (mut section, mut cases, mut ids) = ("", Vec::new(), rustymath::hashing::HashSet::default());
    for range in starts.windows(2) {
        let (start, end) = (range[0], range[1]);
        if let Some(title) = lines[start].strip_prefix("⍝⍝ ") {
            if !lines[start + 1..end].iter().all(|s| s.is_empty()) { return Err(format!("line {}: expected a case after section heading", start + 1)); }
            section = title;
            continue;
        }
        let (id, comment) = header(lines[start]).expect("each start is a header");
        let location = format!("line {} ({id})", start + 1);
        if !id.is_empty() && !ids.insert(id) { return Err(format!("line {}: duplicate ID {id}", start + 1)); }
        let fail = |message: String| format!("{location}: {message}");
        let suffix = comment.strip_prefix('[').map(|s| ("", s)).or_else(|| comment.rsplit_once(" ["));
        let options = suffix.and_then(|(text, s)| Some((text, s.strip_suffix(']')?))).filter(|(_, s)| s.starts_with("rtol=") || s.starts_with("atol=") || FLAGS.iter().any(|f| s.starts_with(f)));
        let mut case = json!({"id":id, "line":start + 1, "section":section, "comment":options.map_or(comment, |(text, _)| text)});
        if let Some(prefs) = prefs { case["prefs"] = json!(prefs); }
        if let Some((_, options)) = options {
            for option in options.split_whitespace() {
                if FLAGS.contains(&option) {
                    if case.get(option).is_some() { return Err(fail(format!("repeated option {option}"))); }
                    case[option] = json!(true);
                    continue;
                }
                let invalid = || fail(format!("invalid tolerance {option}"));
                let (key, value) = option.split_once('=').ok_or_else(invalid)?;
                let key = match key {
                    "rtol" => "relative_tolerance", "atol" => "absolute_tolerance", _ => return Err(fail(format!("unknown tolerance {key}")))
                };
                let value: f64 = value.parse().map_err(|_| invalid())?;
                if !value.is_finite() || value < 0. || case.get(key).is_some() { return Err(invalid()); }
                case[key] = json!(value);
            }
        }
        let mut body = &lines[start + 1..end];
        if body.last() == Some(&"") { body = &body[..body.len() - 1]; }
        else if end != lines.len() { return Err(fail("missing blank record separator".into())); }
        for (marker, key) in [("⍝ ⎕:", "expected_output"), ("⍝ input:", "input")] {
            if let Some(text) = body.last().and_then(|s| s.strip_prefix(marker)) {
                case[key] = json!(unescaped(text.strip_prefix(' ').unwrap_or(text), &location)?);
                body = &body[..body.len() - 1];
            }
        }
        if body.iter().any(|s| s.starts_with("⍝ ⎕:") || s.starts_with("⍝ input:")) {
            return Err(fail("input and output lines must end the case, input first".into()));
        }
        let splits: Vec<_> = body.iter().enumerate().filter_map(|(i, s)| (*s == "⍝ =>").then_some(i)).collect();
        let joined = body.join("\n");
        let pair = inline(&joined).filter(|(code, expect)| !expect.contains('\n') && (body.len() == 1 || code.matches('\n').count() + 1 == body.len()));
        let (code, expect) = match (splits.as_slice(), pair) {
            ([], Some((code, expect))) => (code.to_owned(), expect.to_owned()), ([], None) if body.len() == 2 => (body[0].to_owned(), body[1].to_owned()),
            ([i], _) => (body[..*i].join("\n"), body[i + 1..].join("\n")),
            _ => return Err(fail("use one ⍝ => between multiline expressions, or an inline expectation on the last line".into()))
        };
        if expect.is_empty() { return Err(fail("missing expectation".into())); }
        case["code"] = json!(code);
        if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); }
        else { case["expected_code"] = json!(expect); }
        cases.push(case);
    }
    Ok(cases)
}

/// The cases of `cases` that fail `check`, in their order. Each failure adds the case's `status` and `message`, and the `kind` of the error that the case gave. Cases from files with the same leading `•prefs` header (`prefs`) run together, one run at a time (`RUNS`). For each run, `enter` puts the run's settings in force, the cases without the `serial` option run in parallel, on one thread for each core, and then each `serial` case runs alone, with the settings put in force again before it. A case without `serial` that calls `•prefs` fails, because it would change the settings of the cases running beside it. Each case has `TIMEOUT` to run. After an interrupt, no case starts, and failures after the first interrupted case drop out. The settings are reset at the end.
pub fn failures<'a>(cases: impl IntoIterator<Item = &'a JsonValue>, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
    let cases: Vec<&JsonValue> = cases.into_iter().collect();
    let stop = AtomicBool::new(false);
    let step = |case: &JsonValue, run: Run| {
        let calls_prefs = case["code"].as_str().is_some_and(|code| code.to_lowercase().contains("•prefs"));
        let result = if calls_prefs && case["serial"] != true { Err(json!({"status":"invalid", "message":"a case that calls •prefs needs the serial option"})) }
            else { outcome(case, run, Some(TIMEOUT), interrupt) };
        if result.as_ref().is_err_and(|f| f["status"] == "interrupt") { stop.store(true, Relaxed); }
        result
    };
    let mut outcomes: Vec<Result<Vec<(Run, Outcome)>, JsonValue>> = cases.iter().map(|_| Ok(Vec::new())).collect();
    let mut headers: Vec<Option<&str>> = Vec::new();
    for case in &cases { if !headers.contains(&case["prefs"].as_str()) { headers.push(case["prefs"].as_str()); } }
    for header in headers {
        for run in RUNS {
            let takes = |i: usize| cases[i]["prefs"].as_str() == header && runs(cases[i]).iter().any(|r| (r.0, r.1) == (run.0, run.1));
            let wanted: Vec<usize> = (0..cases.len()).filter(|&i| outcomes[i].is_ok() && takes(i)).collect();
            let (serial, parallel): (Vec<usize>, Vec<usize>) = wanted.into_iter().partition(|&i| cases[i]["serial"] == true);
            let mut record = |i: usize, result: Result<Outcome, JsonValue>| match result {
                Ok(outcome) => if let Ok(list) = &mut outcomes[i] { list.push((run, outcome)) },
                Err(failure) => outcomes[i] = Err(failure),
            };
            if let Err(invalid) = enter(header, run) {
                for &i in serial.iter().chain(&parallel) { record(i, Err(invalid.clone())); }
                continue;
            }
            for (i, result) in in_parallel(&parallel, &stop, |i| step(cases[i], run)) { record(i, result); }
            for i in serial {
                if stop.load(Relaxed) { break; }
                record(i, enter(header, run).and_then(|()| step(cases[i], run)));
            }
        }
    }
    crate::settings::reset();
    let mut failures: Vec<JsonValue> = cases.iter().zip(outcomes).filter_map(|(case, outcome)| {
        let result = outcome.map_or_else(|failure| failure, |list| verdict(case, &list));
        (result["status"] != "pass").then(|| {
            let mut failure = (*case).clone();
            for key in ["status", "message", "kind"] { if let Some(value) = result.get(key) { failure[key] = value.clone(); } }
            failure
        })
    }).collect();
    if let Some(first) = failures.iter().position(|f| f["status"] == "interrupt") { failures.truncate(first + 1); }
    failures
}

/// `f` of each of `items`, with each item, run on one thread for each core. Each thread takes the next item that no thread has taken, until `stop` is set.
fn in_parallel<T: Send>(items: &[usize], stop: &std::sync::atomic::AtomicBool, f: impl Fn(usize) -> T + Sync) -> Vec<(usize, T)> {
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(items.len());
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads).map(|_| scope.spawn(|| {
            let mut done = Vec::new();
            while !stop.load(Relaxed) {
                let Some(&i) = items.get(next.fetch_add(1, Relaxed)) else { break };
                done.push((i, f(i)));
            }
            done
        })).collect();
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    })
}

/// The examples in the `bpl` blocks of `text`, a `.qmd` page, that fail. Each block runs in a fresh session of its own, with the `batch` pref set to `batch` unless that is `None`. Each expectation runs in a fresh session after its lines, under the settings they leave. The lines up to each line with a ` ⍝ ` comment, and up to the block's end, run as one evaluation, with `TIMEOUT` to run. Its value must show the same text as the comment's value, in the session's display with every item, or match it within 1e-13 with the same exactness. `•prefs` option `prec` sets how many digits the display shows. Lines with no comment must run without an error. Each failure gives the `line` of its comment or of the block's end, the `code` that ran, its `expected` comment and a `message`.
pub fn page(text: &str, batch: Option<bool>, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let options = || EvalOptions { timeout: Some(TIMEOUT), echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    let (mut failures, mut session, mut code) = (Vec::new(), None, String::new());
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
        let wanted = expected.map(|e| Session::new().eval_with(e, options()));
        let message = match (result.error, result.value, wanted) {
            (Some(error), _, _) => Some(error.to_string()),
            (None, _, None) => None,
            (None, actual, Some(wanted)) => {
                let shown = bpl.display_settings();
                match (actual, wanted.value, wanted.error) {
                    (_, _, Some(error)) => Some(format!("expectation: {error}")),
                    (Some(actual), Some(wanted), None) if shown.explicit(&actual, false) == shown.explicit(&wanted, false) => None,
                    (Some(actual), Some(wanted), None) => difference(&actual, &wanted, 1e-13, 1e-13, true), _ => Some("no result".into())
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
    let cases = cases(spec["text"].as_str().ok_or("reference needs text as a string")?)?;
    let ids = match spec.get("ids") {
        None | Some(JsonValue::Null) => None, Some(ids) => Some(ids.as_array().ok_or("ids must be a list of case ids and header line numbers")?)
    };
    let matches = |id: &JsonValue, case: &JsonValue| *id == case["id"] || *id == case["line"];
    let unknown: Vec<_> = ids.into_iter().flatten().filter(|id| !cases.iter().any(|case| matches(id, case))).map(JsonValue::to_string).collect();
    if !unknown.is_empty() { return Err(format!("{} match no case. Each id is a case's id or its header's line number.", unknown.join(", "))); }
    let selected = cases.iter().filter(|case| ids.is_none_or(|ids| ids.iter().any(|id| matches(id, case))));
    Ok(json!({"failures": failures(selected, interrupt), "error": null}))
}

/// The reply to the worker's `page` request, which gives the `text` of a `.qmd` page and, optionally, a Boolean `batch` that sets the `batch` pref for its examples. The reply gives the examples that fail, as `page` gives them.
pub fn page_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    Ok(json!({"failures": page(spec["text"].as_str().ok_or("page needs text as a string")?, spec["batch"].as_bool(), interrupt), "error": null}))
}
