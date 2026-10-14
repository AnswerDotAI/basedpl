//! Development-only checks, shared by the tests and the worker: reference cases, `.bpl` reference files and the examples in `.qmd` pages.
use crate::{EvalOptions, InterruptHandle, Session, Value};
use serde_json::{json, Value as JsonValue};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// A value captured from another interpreter. Its numbers carry no exactness. A capture encodes a simple atom as a scalar array, and BPL
/// expectations use based values directly.
pub fn expected_array(value: &JsonValue) -> Option<Value> {
    let result = crate::protocol::import(value, true).ok()?;
    let prototype = crate::protocol::import(&value["prototype"], true).ok()?;
    let result = if result.shape().is_empty() && !result.is_atom() && result.at(0).is_atom() { result.at(0) } else { result };
    result.prototype().same(&prototype).then_some(result)
}

/// Numbers match when they are equal: exact ones exactly, and others within the tolerances. With `exactness`, an exact number never
/// matches an approximate one.
fn same_element(x: &Value, y: &Value, relative: f64, absolute: f64, exactness: bool) -> bool {
    match (x, y) {
        (x @ Value::Array(_), y @ Value::Array(_)) => difference(x, y, relative, absolute, exactness).is_none(),
        (Value::Number(x), Value::Number(y)) if exactness && (x.is_exact() != y.is_exact() || x.as_bool().is_some() != y.as_bool().is_some()) => false,
        (Value::Number(x), Value::Number(y)) if x.is_nan() && y.is_nan() => true,
        (Value::Number(x), Value::Number(y)) if !x.is_exact() || !y.is_exact() => {
            let (Ok(a), Ok(b)) = (x.to_complex(), y.to_complex()) else { return false };
            a == b || (a.is_finite() && b.is_finite() && (a - b).norm() <= absolute.max(relative * a.norm().max(b.norm())))
        }
        _ => x.same(y),
    }
}

/// Locate a structural or numeric mismatch. Approximate numbers may differ by the tolerances. With `exactness`, each number's
/// exactness must match too.
pub fn difference(x: &Value, y: &Value, relative: f64, absolute: f64, exactness: bool) -> Option<String> {
    let same = |a: &Value, b: &Value| same_element(a, b, relative, absolute, exactness);
    if x.is_atom() != y.is_atom() { return Some("atom versus array".into()); }
    if x.is_atom() { return (!same(x, y)).then(|| "atom".into()); }
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if x.layout().all_keys() != y.layout().all_keys() || x.axis_names() != y.axis_names() { return Some("axis keys or names".into()); }
    if !same(&x.prototype(), &y.prototype()) { return Some("prototype".into()); }
    x.elements().zip(y.elements()).position(|(a, b)| !same(&a, &b)).map(|i| format!("data[{i}]"))
}

/// The text at `key` in `case`, or `None` without one. Anything else makes the case invalid, as `what` describes.
fn optional_text<'a>(case: &'a JsonValue, key: &str, what: &str) -> Result<Option<&'a str>, JsonValue> {
    match case.get(key) {
        None => Ok(None),
        Some(JsonValue::String(s)) => Ok(Some(s)),
        _ => Err(json!({"status":"invalid", "message":format!("{what} must be text")})),
    }
}

/// The input lines a case supplies, which `⎕` reads in order.
struct Lines(Mutex<VecDeque<String>>);
impl crate::Input for Lines {
    fn line(&self, _prompt: &str) -> std::io::Result<Option<String>> { Ok(self.0.lock().unwrap().pop_front()) }
    fn rest(&self) -> std::io::Result<Vec<u8>> { Ok(self.0.lock().unwrap().drain(..).map(|line| line + "\n").collect::<String>().into_bytes()) }
}

/// Check an independent captured value or BPL expectation. Each side receives a fresh session. A BPL expectation must match each
/// number's exactness. A value captured from another interpreter carries none, so its numbers compare by value.
pub fn check(case: &JsonValue, options: EvalOptions) -> JsonValue {
    let Some(code) = case["code"].as_str() else { return json!({"status":"invalid", "message":"missing code"}); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let (output, input) = match (optional_text(case, "expected_output", "output expectation"), optional_text(case, "input", "input")) {
        (Ok(output), Ok(input)) => (output, input),
        (Err(invalid), _) | (_, Err(invalid)) => return invalid,
    };
    let (expected, no_result) = if let Some(source) = case["expected_code"].as_str() {
        let expected = Session::new()
            .eval_with(source, EvalOptions { timeout: options.timeout, interrupt: options.interrupt.clone(), echo: false, ..EvalOptions::default() });
        if expected.error.is_some() || expected.function.is_some() || expected.operator.is_some() {
            return json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected)});
        }
        let no_result = expected.value.is_none();
        (expected.value, no_result)
    } else { (expected_array(&case["expected"]), case.get("expected") == Some(&JsonValue::Null)) };
    let exactness = case["expected_code"].is_string();
    let relative = case["relative_tolerance"].as_f64().unwrap_or(1e-13);
    let absolute = case["absolute_tolerance"].as_f64().unwrap_or(1e-13);
    if [relative, absolute].iter().any(|t| !t.is_finite() || *t < 0.0) || (error_kind.is_none() && expected.is_none() && !no_result) {
        return json!({"status":"invalid", "message":"invalid or missing independent expectation"});
    }
    let mut session = Session::new();
    let _files = if code.contains("testpath") {
        let dir = match tempfile::tempdir() { Ok(dir) => dir, Err(e) => return json!({"status":"invalid", "message":format!("temporary fixture: {e}")}) };
        let path = dir.path().join("data");
        session.set("testpath", crate::keyed::text(&path.to_string_lossy())).expect("valid fixture name");
        Some(dir)
    } else { None };
    let input = input.map(|text| Arc::new(Lines(Mutex::new(text.lines().map(String::from).collect()))) as Arc<dyn crate::Input>);
    let result = session.eval_with(code, EvalOptions { echo: false, input, ..options });
    if let Some(error) = &result.error {
        let kind = error.kind.to_string();
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) {
            return json!({"status": kind.to_lowercase(), "message":error.message});
        }
        if error_kind != Some(kind.as_str()) {
            return json!({"status":"error", "kind":kind, "message":error.message, "actual":crate::protocol::response(result)});
        }
    }
    let mismatch = if result.error.is_some() { None } else if result.function.is_some() || result.operator.is_some() { Some("unexpected function or operator result".into()) } else {
        match (error_kind, &result.value, expected) {
            (Some(kind), _, _) => Some(format!("expected {kind}")),
            (_, None, _) if no_result => None,
            (_, None, _) => Some("no result".into()),
            (_, Some(_), _) if no_result => Some("expected no result".into()),
            (_, Some(actual), Some(expected)) => difference(actual, &expected, relative, absolute, exactness),
            _ => unreachable!(),
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
        Some(message) => json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result)}),
        None => json!({"status":"pass"}),
    }
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
            '"' => string = !string,
            '\'' if !string => {
                chars.next();
                if !line[i..].starts_with("''") || line[i..].starts_with("'''") { chars.next(); }
            }
            '⍝' if !string => return Some((line[..i].trim_end(), line[i + c.len_utf8()..].trim_start())),
            _ => (),
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

/// The cases of `text`, in the format of a `.bpl` reference file, as `check` takes them. Each case also gives its `id`, its header's
/// `line` and its `section`. `tests/reference/README.md` describes the format.
pub fn cases(text: &str) -> Result<Vec<JsonValue>, String> {
    if text.is_empty() { return Ok(Vec::new()); }
    let lines: Vec<_> = text.split_terminator('\n').collect();
    let mut starts: Vec<_> = lines.iter().enumerate().filter_map(|(i, s)| (header(s).is_some() || s.starts_with("⍝⍝ ")).then_some(i)).collect();
    if starts.first() != Some(&0) { return Err("expected a case or section header on line 1".into()); }
    starts.push(lines.len());
    let (mut section, mut cases) = ("", Vec::new());
    for range in starts.windows(2) {
        let (start, end) = (range[0], range[1]);
        if let Some(title) = lines[start].strip_prefix("⍝⍝ ") {
            if !lines[start + 1..end].iter().all(|s| s.is_empty()) { return Err(format!("line {}: expected a case after section heading", start + 1)); }
            section = title;
            continue;
        }
        let (id, comment) = header(lines[start]).expect("each start is a header");
        let location = format!("line {} ({id})", start + 1);
        let fail = |message: String| format!("{location}: {message}");
        let mut case = json!({"id":id, "line":start + 1, "section":section});
        let suffix = comment.strip_prefix('[').or_else(|| comment.rsplit_once(" [").map(|(_, s)| s));
        if let Some(options) = suffix.and_then(|s| s.strip_suffix(']')).filter(|s| s.starts_with("rtol=") || s.starts_with("atol=")) {
            for option in options.split_whitespace() {
                let invalid = || fail(format!("invalid tolerance {option}"));
                let (key, value) = option.split_once('=').ok_or_else(invalid)?;
                let key = match key { "rtol" => "relative_tolerance", "atol" => "absolute_tolerance", _ => return Err(fail(format!("unknown tolerance {key}"))) };
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
            ([], Some((code, expect))) => (code.to_owned(), expect.to_owned()),
            ([], None) if body.len() == 2 => (body[0].to_owned(), body[1].to_owned()),
            ([i], _) => (body[..*i].join("\n"), body[i + 1..].join("\n")),
            _ => return Err(fail("use one ⍝ => between multiline expressions, or an inline expectation on the last line".into())),
        };
        if expect.is_empty() { return Err(fail("missing expectation".into())); }
        case["code"] = json!(code);
        if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); }
        else { case["expected_code"] = json!(expect); }
        cases.push(case);
    }
    Ok(cases)
}

/// The cases of `cases` that fail `check`, in their order. Each failure adds `check`'s `status` and `message`, and the `kind` of the
/// error that the case gave. Each case has `TIMEOUT` to run. The cases run on one thread for each core. Each thread takes the next case
/// that no thread has taken. After an interrupt, no thread takes another case, and failures after the first interrupted case drop out.
pub fn failures<'a>(cases: impl IntoIterator<Item = &'a JsonValue>, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
    let cases: Vec<&JsonValue> = cases.into_iter().collect();
    let (next, stop) = (AtomicUsize::new(0), AtomicBool::new(false));
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(cases.len());
    let mut failures: Vec<(usize, JsonValue)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut found = Vec::new();
                    while !stop.load(Relaxed) {
                        let i = next.fetch_add(1, Relaxed);
                        let Some(&case) = cases.get(i) else { break };
                        let result = check(case, EvalOptions { timeout: Some(TIMEOUT), interrupt: interrupt.clone(), ..EvalOptions::default() });
                        if result["status"] == "pass" { continue; }
                        if result["status"] == "interrupt" { stop.store(true, Relaxed); }
                        let mut failure = case.clone();
                        for key in ["status", "message", "kind"] {
                            if let Some(value) = result.get(key) { failure[key] = value.clone(); }
                        }
                        found.push((i, failure));
                    }
                    found
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    });
    failures.sort_by_key(|&(i, _)| i);
    if let Some(first) = failures.iter().position(|(_, f)| f["status"] == "interrupt") { failures.truncate(first + 1); }
    failures.into_iter().map(|(_, f)| f).collect()
}

/// The examples in the `bpl` blocks of `text`, a `.qmd` page, that fail. Each block runs in a session of its own. The lines up to each
/// line with a ` ⍝ ` comment, and up to the block's end, run as one evaluation, with `TIMEOUT` to run. Its value must show the same
/// text as the comment's value, in the session's display with every item, or match it within 1e-13 with the same exactness. `•prefs`
/// option `prec` sets how many digits the display shows. Lines with no comment must run without an error. Each failure gives the
/// `line` of its comment or of the block's end, the `code` that ran, its `expected` comment and a `message`.
pub fn page(text: &str, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let options = || EvalOptions { timeout: Some(TIMEOUT), echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    let (mut failures, mut session, mut code) = (Vec::new(), None, String::new());
    for (i, line) in text.lines().enumerate() {
        if line == "```bpl" {
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
        let message = match (result.error, result.value, expected) {
            (Some(error), _, _) => Some(error.to_string()),
            (None, _, None) => None,
            (None, actual, Some(expected)) => {
                let wanted = Session::new().eval_with(expected, options());
                let shown = bpl.display_settings();
                match (actual, wanted.value, wanted.error) {
                    (_, _, Some(error)) => Some(format!("expectation: {error}")),
                    (Some(actual), Some(wanted), None) if shown.explicit(&actual, false) == shown.explicit(&wanted, false) => None,
                    (Some(actual), Some(wanted), None) => difference(&actual, &wanted, 1e-13, 1e-13, true),
                    _ => Some("no result".into()),
                }
            }
        };
        if let Some(message) = message { failures.push(json!({"line": i + 1, "code": code.trim_end(), "expected": expected, "message": message})); }
        code.clear();
        if closing { session = None; }
    }
    failures
}

/// The reply to the worker's `reference` request. It gives `text` in a `.bpl` reference file's format and, optionally, a list of the
/// `ids` to check, each a case's id or its header's line number. An id that matches no case is an error. The reply gives the cases
/// that fail, as `failures` gives them.
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let cases = cases(spec["text"].as_str().ok_or("reference needs text as a string")?)?;
    let ids = match spec.get("ids") {
        None | Some(JsonValue::Null) => None,
        Some(ids) => Some(ids.as_array().ok_or("ids must be a list of case ids and header line numbers")?),
    };
    let matches = |id: &JsonValue, case: &JsonValue| *id == case["id"] || *id == case["line"];
    let unknown: Vec<_> = ids.into_iter().flatten().filter(|id| !cases.iter().any(|case| matches(id, case))).map(JsonValue::to_string).collect();
    if !unknown.is_empty() { return Err(format!("{} match no case. Each id is a case's id or its header's line number.", unknown.join(", "))); }
    let selected = cases.iter().filter(|case| ids.is_none_or(|ids| ids.iter().any(|id| matches(id, case))));
    Ok(json!({"failures": failures(selected, interrupt), "error": null}))
}

/// The reply to the worker's `page` request, which gives the `text` of a `.qmd` page. The reply gives the examples that fail, as `page`
/// gives them.
pub fn page_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    Ok(json!({"failures": page(spec["text"].as_str().ok_or("page needs text as a string")?, interrupt), "error": null}))
}
