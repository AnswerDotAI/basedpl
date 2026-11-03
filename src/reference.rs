//! Development-only checks, shared by the tests and the worker: reference cases, `.bpl` reference files and the examples in `.qmd` pages.
use crate::{EvalOptions, InterruptHandle, Session, Value, number::Numeric, settings::Compiles};
use serde_json::{Value as JsonValue, json};
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

/// The kinds of storage, as `•storage` names them under `numeric`, at the first place where `x` and `y` are held differently. It walks into the items of mixed arrays. With `compact_alike`, every compact kind counts as one, so only nested storage against compact storage differs.
fn kind_difference(x: &Value, y: &Value, numeric: Numeric, compact_alike: bool) -> Option<(&'static str, &'static str)> {
    let nested = |kind: &str| kind == "mixed";
    let (a, b) = (x.storage_kind(numeric), y.storage_kind(numeric));
    let differ = if compact_alike || nested(a) && nested(b) { nested(a) != nested(b) } else { a != b };
    if differ { return Some((a, b)); }
    if !nested(a) || x.is_atom() || y.is_atom() || x.len() != y.len() { return None; }
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

/// One scheduler under its compile policy, plus a second-call replay check.
#[derive(Clone, Copy)]
struct Run { compiles: Compiles, twice: bool, words: &'static str }
const RUNS: [Run; 4] = [
    Run { compiles: Compiles::Policy, twice: false, words: "with normal compilation" },
    Run { compiles: Compiles::Always, twice: false, words: "with compiled stages" },
    Run { compiles: Compiles::Never, twice: false, words: "with uncompiled stages" },
    Run { compiles: Compiles::Policy, twice: true, words: "on a dfn's second call" },
];

/// `mixed` permits another storage kind; `compiled` skips the uncompiled run when the result reports compilation.
const FLAGS: [&str; 2] = ["mixed", "compiled"];
fn runs(case: &JsonValue) -> Vec<Run> { RUNS.into_iter().filter(|r| !(r.compiles == Compiles::Never && case["compiled"] == true)).collect() }

/// Defaults and file preferences, with the run's compile setting.
fn enter(prefs: Option<&str>, run: Option<Run>) -> Result<(), JsonValue> {
    crate::settings::reset();
    crate::settings::set_verifies(true);
    if let Some(prefs) = prefs
        && let Some(error) = Session::current(|session| session.eval_apart(prefs, EvalOptions { timeout: Some(TIMEOUT), echo: false, ..EvalOptions::default() })).error {
            return Err(json!({"status":"invalid", "message":format!("file header: {error}")}));
        }
    if let Some(run) = run { crate::settings::set_compiles(run.compiles); }
    Ok(())
}

/// One run of a case: the value its code gave, the numeric settings in force after it, and the value its BPL expectation gave.
struct Outcome { value: Option<Value>, numeric: Numeric, expected: Option<Value> }

/// A case's relative tolerance for comparing numbers, where its header sets one, and its absolute tolerance, `TOLERANCE` unless its header sets it.
fn tolerances(case: &JsonValue) -> (Option<f64>, f64) { (case["relative_tolerance"].as_f64(), case["absolute_tolerance"].as_f64().unwrap_or(TOLERANCE)) }

/// `case`'s code checked against an independent captured value or BPL expectation, in `run`, under the settings in force. Each side receives a fresh session, and the expectation evaluates after the code, under the settings that the code leaves. A BPL expectation must match each number's exactness. A value captured from another interpreter carries none, so its numbers compare by value. A failure's message names its run.
fn outcome(
    case: &JsonValue, setup: &str, Run { words, .. }: Run, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<Outcome, JsonValue> {
    let Some(code) = case["code"].as_str() else { return Err(json!({"status":"invalid", "message":"missing code"})); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let (output, input) = (optional_text(case, "expected_output", "output expectation")?, optional_text(case, "input", "input")?);
    let result = evaluated(code, setup, input, timeout, interrupt)?;
    let (expected, no_result) = if let Some(source) = case["expected_code"].as_str() {
        let expected = evaluated(source, setup, None, timeout, interrupt)?;
        if expected.error.is_some() || expected.function.is_some() || expected.operator.is_some() {
            return Err(json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected).map_err(unreadable)?}));
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
        failure["message"] = json!(format!("{words}: {}", failure["message"].as_str().unwrap_or_default()));
        Err(failure)
    };
    if let Some(error) = &result.error {
        let (kind, message) = (error.kind.to_string(), error.message.clone());
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) {
            return failed(json!({"status": kind.to_lowercase(), "message":message}));
        }
        if error_kind != Some(kind.as_str()) {
            return failed(json!({"status":"error", "kind":kind, "message":message, "actual":crate::protocol::response(result).map_err(unreadable)?}));
        }
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
        Some(message) => failed(json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result).map_err(unreadable)?})),
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
            *session.generator.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = crate::distribution::Counter::seeded(0);
            calls.push(observed_result(session.eval_with(&format!("{TWICE} 0"), EvalOptions { input: lines(input), ..options() }), &session)?);
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
    let mismatch = if error(&first) != error(&second) {
        Some(format!("error {:?}, {:?} on the first call", error(&second), error(&first)))
    } else if written(&first) != written(&second) {
        Some(format!("output {:?}, {:?} on the first call", written(&second), written(&first)))
    } else if category(&first) != category(&second) { Some("a function or operator from one call only".into()) } else {
        match (&first.value, &second.value) {
            (Some(a), Some(b)) => difference(b, a, relative, absolute, true)
                .or_else(|| kind_difference(b, a, numeric, false).map(|(kind, first_kind)| format!("storage kind: {kind}, {first_kind} on the first call"))),
            (None, None) => None, _ => Some("a value from one call only".into())
        }
    };
    match mismatch {
        Some(message) => Err(json!({"status":"mismatch", "message":format!("{words}: {message}"), "actual":crate::protocol::response(second).map_err(unreadable)?})),
        None => Ok(Outcome { value: None, numeric, expected: None }),
    }
}

/// Whether a case's runs agree: each later run's value must be held in the same kind of storage as the first's. Where the expectation is BPL, the first's storage must be mixed where the expectation's is, and compact where it's compact. A `mixed` case leaves the storage checks out. An in-place write keeps mixed storage mixed, and a record from JSON or Python keeps each entry's own kind.
fn verdict(case: &JsonValue, outcomes: &[(Run, Outcome)]) -> JsonValue {
    let Some((first_run, first)) = outcomes.first() else { return json!({"status":"pass"}) };
    for (run, outcome) in &outcomes[1..] {
        if let (Some(value), Some(first)) = (&outcome.value, &first.value)
            && let Some((kind, first_kind)) = kind_difference(value, first, outcome.numeric, false) {
                return json!({"status":"mismatch", "message":format!("storage kind: {kind} {}, {first_kind} {}", run.words, first_run.words)});
            }
    }
    if case["expected_code"].is_string() && case["mixed"] != true && let (Some(value), Some(expected)) = (&first.value, &first.expected)
        && let Some((kind, expected_kind)) = kind_difference(value, expected, first.numeric, true) {
            return json!({"status":"mismatch", "message":format!("storage kind: {kind}, {expected_kind} in the expectation")});
        }
    json!({"status":"pass"})
}

/// `case` checked as `failures` checks it with its route directives, and no setup: `{"status":"pass"}`, or the case with the `status`, `message` and `kind` of its failure, and its `actual` result where there is one.
pub fn check(case: &JsonValue, interrupt: &InterruptHandle) -> JsonValue {
    failures([case], "", true, interrupt).pop().unwrap_or_else(|| json!({"status":"pass"}))
}

/// A timed record's expectation, checked once outside timing, with fresh setup for both expressions.
pub fn check_once(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<(), JsonValue> {
    enter(case["prefs"].as_str(), None)?;
    outcome(case, setup, RUNS[0], Some(TIMEOUT), interrupt).map(drop)
}

/// Warm and capture a case under its ordinary preferences and setup, on one thread, as route checks run it.
fn capture(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<crate::timing::Capture, JsonValue> {
    enter(case["prefs"].as_str(), None)?;
    let session = Session::new();
    initialized(&session, setup)?;
    let threads = rustymath::parallel::set_threads(1);
    let captured = crate::timing::route(&session, case, interrupt);
    rustymath::parallel::set_threads(threads);
    captured.map_err(|message| json!({"status":"invalid", "message":message}))
}

/// `capture` of `file`'s `case` with the file's setup, named as the route test reports it (`timing::named`).
pub fn route(file: &File, case: &JsonValue, interrupt: &InterruptHandle) -> Result<crate::timing::Capture, String> {
    let message = |failed: JsonValue| failed["message"].as_str().map_or_else(|| failed.to_string(), str::to_owned);
    crate::timing::named(capture(case, &file.setup, interrupt).map_err(message), file, case)
}

pub fn routed(case: &JsonValue) -> bool { ["route", "allows", "gap"].iter().any(|key| case.get(key).is_some()) }

/// The data that `coverage`'s cases read, with a matrix for its inner products.
const COVERAGE_SETUP: &str = "x←0.25+¿4096⍴0 ⋄ y←0.25+¿4096⍴0 ⋄ m←0.25+¿[64 64]⍴0 ⋄ k←0.25+¿[64 64]⍴0";
/// `coverage`'s sections, with the preferences each runs under.
const COVERAGE_SECTIONS: [(&str, &str); 2] = [("dynamic", ""), ("static", r#"•prefs ["kinds":"static"]"#)];
/// The constants that `coverage` puts on the right of each dyadic form, and those it puts on the left.
const COVERAGE_RIGHT: [&str; 5] = ["0", "1", "2", "0.5", "¯1"];
const COVERAGE_LEFT: [&str; 2] = ["1", "2"];

/// The coverage matrix of the pervasive primitives against the executors, as the route test checks it. Each pervasive form in the facts table, apart from `ᵇ`'s, monadic `+`'s and those that no kind runs on carried values (`Form::Unsupported`), gives cases in a section under dynamic kinds and one under static kinds, all on floats. Each case that maps a form puts it between two other steps of one block stage: it reads `1+x` and its result is doubled. A lone call forced at once runs eagerly, outside any stage, and a form that `Pervasive::kept` removes leaves two steps, which compile. A form that doesn't lower leaves no compiled part, because a part of one step after it doesn't compile. A monadic form gives `2×f 1+x` and its gradient, `{+/f⍵}∂ x`. So does each monadic rule of the form rewriter whose function maps its argument, spelled as the rule writes it with `1+x` for `⍵`, as in `2×⍟1+1+x`, where the rewriter puts the rule's function. Each row of the mark `ₐ` gives its marked function's cases as code writes them: a monadic one `2×√ₐ1+x` and `{+/√ₐ⍵}∂ x`, a dyadic one `2×(1+x)*ₐy` and both gradients, `{+/⍵*ₐy}∂ x` and `{+/y*ₐ⍵}∂ x`. A form's own case that a rule would rewrite reads `2+x` in place of `1+x`, as `2×⍟2+x` does, and tests the form itself. A dyadic form gives `2×(1+x)g y`, `2×(1+x)g c` for each constant of `COVERAGE_RIGHT`, `2×c g 1+x` for each of `COVERAGE_LEFT`, the fold `g/2×x`, the inner product `m+.g k` and the gradients `{+/⍵ g y}∂ x`, `{g/⍵}∂ x` and `{+/,m+.g ⍵}∂ k`. A form whose left argument chooses a function, as `○`'s circle code does, takes the codes 0 to 8 on the left instead. Each case runs as the run with fusion at any size runs it, which compiles every stage (`RUNS[2]`), and must take the `compiled` route, or the `fused fold` route for a fold, or the `trace` route for a gradient. An inner product must only record no slow event. `gaps` lists the ways the matrix accepts a case falling short, one line each: the section, a tab, the case's code, a tab, then a `⍝⍝⍝ gap` or `⍝⍝⍝ allows` line as a reference case writes it. A case that `gaps` lists needn't take its route, because each of its lines names the event that stands in the route's way. The captures check those lines as a reference case's are checked, so a line whose event has gone fails. Gives the captures and, for each case whose evaluation fails, such as `~` on floats, its section, code and error. A line of `gaps` that names no case is an error.
pub fn coverage(gaps: &std::path::Path, interrupt: &InterruptHandle) -> Result<(Vec<crate::timing::Capture>, Vec<String>), String> {
    use crate::primitive::{Extension, Form, Primitive};
    let text = std::fs::read_to_string(gaps).map_err(|e| format!("{}: {e}", gaps.display()))?;
    let mut listed = Vec::new();
    for (n, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let fail = || format!("{} line {}: expected a section, a case's code and a gap or allows line, separated by tabs", gaps.display(), n + 1);
        let [section, code, directive] = line.splitn(3, '\t').collect::<Vec<_>>()[..] else { return Err(fail()) };
        let (key, rest) = directive.strip_prefix("⍝⍝⍝ ").and_then(|d| d.split_once(' ')).filter(|(k, _)| matches!(*k, "gap" | "allows")).ok_or_else(fail)?;
        listed.push((section, code, key, event(rest), false));
    }
    let pervasive = |p: Primitive, dyadic: bool| { let f = p.facts(dyadic); f.extension == Extension::Pervasive && f.form != Form::Unsupported };
    let mut cases: Vec<(String, &str)> = Vec::new();
    // A function's cases with one argument, and with each circle code as its left argument.
    let monadic = |g: &str| [(format!("2×{g}2+x"), "compiled"), (format!("{{+/{g}⍵}}∂ x"), "trace")];
    let coded = |g: &str| (0..=8).flat_map(|c| [(format!("2×{c}{g}2+x"), "compiled"), (format!("{{+/{c}{g}⍵}}∂ x"), "trace")]).collect::<Vec<_>>();
    for p in Primitive::all().filter(|p| !matches!(p, Primitive::Bitwise(_))) {
        let g = p.glyph();
        // Monadic `+` keeps a real operand (`Pervasive::kept`), so its case would compute nothing to compile.
        if pervasive(p, false) && p.facts(false).linear != Some(crate::primitive::LinearMap::Same) {
            cases.extend(monadic(&g));
        }
        if !pervasive(p, true) { continue; }
        if p.info().dyad.code {
            cases.extend(coded(&g));
            continue;
        }
        cases.push((format!("2×(2+x){g}y"), "compiled"));
        cases.extend(COVERAGE_RIGHT.iter().map(|c| (format!("2×(2+x){g}{c}"), "compiled")));
        cases.extend(COVERAGE_LEFT.iter().map(|c| (format!("2×{c}{g}2+x"), "compiled")));
        cases.extend([(format!("{g}/2×x"), "fused fold"), (format!("m+.{g}k"), ""), (format!("{{+/⍵{g}y}}∂ x"), "trace")]);
        cases.extend([(format!("{{{g}/⍵}}∂ x"), "trace"), (format!("{{+/,m+.{g}⍵}}∂ k"), "trace")]);
    }
    let (mut captures, mut failed) = (Vec::new(), Vec::new());
    for (section, prefs) in COVERAGE_SECTIONS {
        for (code, route) in &cases {
            let mut case = json!({"code":code, "section":section, "id":code});
            for entry in listed.iter_mut().filter(|e| e.0 == section && e.1 == code.as_str()) {
                case.as_object_mut().unwrap().entry(entry.2).or_insert_with(|| json!([])).as_array_mut().unwrap().push(entry.3.clone());
                entry.4 = true;
            }
            if !route.is_empty() && case.get("gap").is_none() && case.get("allows").is_none() { case["route"] = json!([route]); }
            enter(Some(prefs).filter(|p| !p.is_empty()), Some(RUNS[1])).map_err(|e| e.to_string())?;
            let session = Session::new();
            initialized(&session, COVERAGE_SETUP).map_err(|e| e.to_string())?;
            match crate::timing::route(&session, &case, interrupt) { Ok(capture) => captures.push(capture), Err(message) => failed.push(format!("[{section}] {code}: {message}")) }
        }
    }
    let unused: Vec<_> = listed.iter().filter(|e| !e.4).map(|e| format!("{}\t{}", e.0, e.1)).collect();
    if !unused.is_empty() { return Err(format!("{} names no case: {unused:?}", gaps.display())); }
    Ok((captures, failed))
}

fn route_check(case: &JsonValue, setup: &str, interrupt: &InterruptHandle) -> Result<(), JsonValue> {
    if !routed(case) { return Ok(()); }
    let wrong = capture(case, setup, interrupt)?.violations();
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
fn evaluated(
    code: &str, setup: &str, input: Option<&str>, timeout: Option<std::time::Duration>, interrupt: &InterruptHandle) -> Result<crate::Evaluation, JsonValue> {
    let session = Session::new();
    initialized(&session, setup)?;
    let _files = fixture(&session, code)?;
    observed_result(session.eval_with(code, EvalOptions { timeout, interrupt: interrupt.clone(), echo: false, input: lines(input), ..EvalOptions::default() }), &session)
}

fn observed_result(mut result: crate::Evaluation, session: &Session) -> Result<crate::Evaluation, JsonValue> {
    if result.error.is_none() && let Some(value) = result.value.take() {
        match value.read(crate::ReadKind::Value).and_then(|value| { session.raise()?; Ok(value) }) {
            Ok(value) => result.value = Some(value),
            Err(error) if error.is_control() => return Err(unreadable(error)),
            Err(error) => result.error = Some(error),
        }
    }
    Ok(result)
}
fn unreadable(error: crate::Error) -> JsonValue { json!({"status":"error", "message":error.to_string()}) }

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
fn lines(input: Option<&str>) -> Option<Arc<dyn crate::Input>> {
    input.map(|text| Arc::new(Lines(Mutex::new(text.lines().map(String::from).collect()))) as Arc<dyn crate::Input>)
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
            '\'' if !string => { chars.next(); if !line[i..].starts_with("''") || line[i..].starts_with("'''") { chars.next(); } }
            '⍝' if !string => {
                if line[i..].contains('\n') {
                    for (_, c) in chars.by_ref() { if c == '\n' { break; } }
                } else { return Some((line[..i].trim_end(), line[i + c.len_utf8()..].trim_start())); }
            }
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
    for line in &lines {
        offsets.push(offset);
        offset += line.len() + 1;
    }
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
        let options = suffix.and_then(|(text, s)| Some((text, s.strip_suffix(']')?)))
            .filter(|(_, s)| s.starts_with("rtol=") || s.starts_with("atol=") || FLAGS.iter().any(|f| s.starts_with(f)));
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
                let key = match key {
                    "rtol" => "relative_tolerance", "atol" => "absolute_tolerance", _ => return Err(fail(format!("unknown tolerance {key}")))
                };
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
            if fields.is_empty() || fields.iter().any(|s| s.parse::<f64>().map_or(true, |v| !v.is_finite() || v < 0.)) {
                return Err(fail("invalid timing budgets".into()));
            }
            case["budgets"] = json!(fields);
            case["budget_span"] = json!([offsets[end - 1], offsets[end - 1] + lines[end - 1].len()]);
            body = &body[..body.len() - 1];
        } else if heading.is_none() { return Err(fail("expected a case header or a timing footer".into())); }
        else if i < lines.len() && end == i { return Err(fail("missing blank record separator".into())); }
        let mut comments = Vec::new();
        let (mut leading, mut holds) = (0, Vec::new());
        for line in body {
            if let Some(directive) = line.strip_prefix("⍝⍝⍝ ") {
                let (key, rest) = directive.split_once(' ').ok_or_else(|| fail("a directive needs its name".into()))?;
                let (key, value) = match key {
                    "route" if !rest.trim().is_empty() => (key, json!(rest.trim())), "allows" | "gap" => (key, event(rest)),
                    _ if rustymath::timing::hold(directive, &mut holds).map_err(&fail)? => ("held", json!(directive)),
                    _ => return Err(fail(format!("invalid directive {directive}")))
                };
                case.as_object_mut().unwrap().entry(key).or_insert_with(|| json!([])).as_array_mut().unwrap().push(value);
            } else if let Some(rest) = line.strip_prefix("⍝ ") { if heading.is_none() { comments.push(rest); } }
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
        if body.iter().any(|s| s.starts_with("⍝ ⎕:") || s.starts_with("⍝ input:")) {
            return Err(fail("input and output lines must end the case, input first".into()));
        }
        let splits: Vec<_> = body.iter().enumerate().filter_map(|(i, s)| (*s == "⍝ =>").then_some(i)).collect();
        let joined = body.join("\n");
        let pair = inline(&joined).filter(|(code, _)| body.len() == 1 || code.matches('\n').count() + 1 == body.len());
        let (code, expect) = match (splits.as_slice(), pair) {
            ([], Some((code, expect))) => (code.to_owned(), Some(expect.to_owned())),
            ([], None) if case["budgets"].is_array() => (joined, None),
            ([], None) if body.len() == 2 || heading.is_some() && body.len() == leading + 2 => {
                (body[..body.len() - 1].join("\n"), Some(body[body.len() - 1].to_owned()))
            }
            ([i], _) => (body[..*i].join("\n"), Some(body[i + 1..].join("\n"))),
            _ => return Err(fail("use one ⍝ => between multiline expressions, or an inline expectation on the last line".into())),
        };
        if code.is_empty() && expect.is_none() || expect.as_ref().is_some_and(String::is_empty) { return Err(fail("missing code or expectation".into())); }
        case["code"] = json!(code);
        if let Some(expect) = expect {
            if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); } else { case["expected_code"] = json!(expect); }
        }
        file.cases.push(case);
    }
    Ok(file)
}

/// Reads named source texts supplied by the worker or browser caller.
pub fn sources(spec: &JsonValue) -> Result<Vec<File>, String> {
    spec.as_array().ok_or("files must be a list of named source texts")?.iter()
        .map(|s| file(s["name"].as_str().ok_or("file needs name")?, s["text"].as_str().ok_or("file needs text")?)).collect()
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

/// The cases of `cases` whose runs fail their expectations, in their order. Where `routes` is set, each case that meets its expectations is then checked against its route directives (`route_check`), as the route test checks them: `check` and Python's `check_file` set it. `tests/reference.rs` doesn't, because within `cargo test` only the explain test checks route directives. Each failure adds the case's `status` and `message`, and the `kind` of the error that the case gave. Each case takes its runs (`runs`) one after another, each under its file's leading `•prefs` header and the run's settings (`enter`), and each run has `TIMEOUT`. Callers that check cases in parallel split them across processes, as `tests/reference.rs` and Python's `check_file` do, because a process evaluates one case at a time. An interrupt, or the deadline of `interrupt` passing, fails the case that it stops as `unchecked`, and no case after it runs. The settings are reset at the end.
pub fn failures<'a>(cases: impl IntoIterator<Item = &'a JsonValue>, setup: &str, routes: bool, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let mut failures = Vec::new();
    for case in cases {
        let result = held_back(|| case_result(case, setup, routes, interrupt));
        if let Some(why) = interrupt.stop() {
            failures.push(failed(case, &json!({"status": "unchecked", "message": format!("the check stopped at {why} before this case finished, so it and the cases after it are unchecked")})));
            break;
        }
        if result["status"] == "pass" { continue; }
        failures.push(failed(case, &result));
    }
    crate::settings::reset();
    failures
}

/// Clears the program store before an independent case or page. Repeated calls within that case share the store.
fn forget_compiled() { crate::program::programs().entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear(); }

/// The verdict on `case` over each of its runs, and on its routes where `routes` asks. Each run's settings (`enter`), the run and the reset after it run under `unwound`. A panic in any of them, the reset's barrier over the pending work that the run left included, fails this case. A failing run doesn't stop the runs after it, except one that panicked, timed out or was interrupted: after a panic the case's settings can't be trusted, and more runs would only spend more time. The failure names each failing run (`combined`). Each case starts with no kept plans or compiled kernels (`forget_compiled`), so its result doesn't depend on the cases before it.
fn case_result(case: &JsonValue, setup: &str, routes: bool, interrupt: &InterruptHandle) -> JsonValue {
    forget_compiled();
    let (mut outcomes, mut failures, mut stopped) = (Vec::new(), Vec::new(), false);
    for run in runs(case) {
        let ran = unwound(run.words, || {
            enter(case["prefs"].as_str(), Some(run))?;
            let ran = if run.twice { twice(case, setup, run, Some(TIMEOUT), interrupt) } else { outcome(case, setup, run, Some(TIMEOUT), interrupt) };
            crate::settings::reset();
            ran
        });
        match ran {
            Ok(outcome) => outcomes.push((run, outcome)),
            Err(failure) => {
                stopped = matches!(failure["status"].as_str(), Some("panic" | "timeout" | "interrupt"));
                failures.push((run, failure));
                if stopped { break; }
            }
        }
    }
    let result = if failures.is_empty() { verdict(case, &outcomes) } else { combined(&failures, outcomes.is_empty() && !stopped) };
    if routes && result["status"] == "pass" { unwound("in the route check", || route_check(case, setup, interrupt)).err().unwrap_or(result) } else { result }
}

/// A case's failing runs as one failure. It takes the status, kind and result of the first, or of an interrupt, which stops its caller (`failures`). Its message names each failing run, since each run's message starts with the run's words (`outcome`, `unwound`), or says "every run" where `every` run failed with one message.
fn combined(failures: &[(Run, JsonValue)], every: bool) -> JsonValue {
    let message = |f: &JsonValue| f["message"].as_str().unwrap_or_default().to_owned();
    let body = |(run, f): &(Run, JsonValue)| { let m = message(f); m.strip_prefix(run.words).and_then(|m| m.strip_prefix(": ")).map_or(m.clone(), str::to_owned) };
    let first = body(&failures[0]);
    let lead = failures.iter().find(|(_, f)| f["status"] == "interrupt").unwrap_or(&failures[0]);
    let mut failure = lead.1.clone();
    failure["message"] = json!(if every && failures.len() > 1 && failures.iter().all(|f| body(f) == first) { format!("every run: {first}") }
        else { failures.iter().map(|(_, f)| message(f)).collect::<Vec<_>>().join("; ") });
    failure
}

/// What `check` gives, or, where it panics, a failure of status `panic` whose message names `words`, the run, and the panic's message. The cases after it still run, in Python's `check_file` and in `cargo test` alike.
fn unwound<T>(words: &str, check: impl FnOnce() -> Result<T, JsonValue>) -> Result<T, JsonValue> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)).unwrap_or_else(|panic| {
        let message = panic.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| panic.downcast_ref::<String>().cloned()).unwrap_or_default();
        Err(json!({"status":"panic", "message":format!("{words}: panicked: {message}")}))
    })
}

/// What `check` gives, or a leak where `rustymath::memory::set_checks` is on and a case that passes leaves the guard's count higher than it found it on each of two runs. One run that leaves it higher can be filling a cache that lasts.
fn held_back(check: impl Fn() -> JsonValue) -> JsonValue {
    use rustymath::memory::{checks, held};
    let start = held();
    let result = check();
    if !checks() || result["status"] != "pass" || held() <= start { return result; }
    let again = held();
    let result = check();
    let left = held().saturating_sub(again);
    if result["status"] != "pass" || left == 0 { return result; }
    json!({"status":"leak", "message":format!("each of two runs left {left} or more bytes held in large allocations")})
}

/// The examples in the `bpl` blocks of `text`, a `.qmd` page, that fail. Each block runs in a fresh session of its own. Each expectation runs in a module of its own after its lines (`Session::eval_apart`), under the settings they leave. The lines up to each line with a ` ⍝ ` comment, and up to the block's end, run as one evaluation, with `TIMEOUT` to run. Its value must show the same text as the comment's value, in the session's display with every item, or match it within the default tolerances that `same_atom` describes, with the same exactness. `•prefs` option `prec` sets how many digits the display shows. Lines with no comment must run without an error. Each page starts with no kept plans or compiled kernels (`forget_compiled`). Each evaluation notes its line on stderr as it starts (`example LINE`), which a check reads as progress (`bpltests._checked`). An interrupt, or the deadline of `interrupt` passing, fails the example that it stops as `unchecked`, and no example after it runs. Each failure gives the `line` of its comment or of the block's end, the `code` that ran, its `expected` comment and a `message`.
pub fn page(text: &str, interrupt: &InterruptHandle) -> Vec<JsonValue> {
    let options = || EvalOptions { timeout: Some(TIMEOUT), echo: false, interrupt: interrupt.clone(), ..EvalOptions::default() };
    let (mut failures, mut session, mut code) = (Vec::new(), None, String::new());
    forget_compiled();
    for (i, line) in text.lines().enumerate() {
        if line == "```bpl" {
            crate::settings::reset();
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
        eprintln!("example {}", i + 1);
        let result = bpl.eval_with(&code, options());
        // The expectation evaluates after the code, under the settings that the code leaves.
        let wanted = expected.map(|e| bpl.eval_apart(e, options()));
        if let Some(why) = interrupt.stop() {
            failures.push(json!({"line": i + 1, "status": "unchecked", "code": code.trim_end(), "expected": expected, "message": format!("the check stopped at {why} before this example finished, so it and the examples after it are unchecked")}));
            break;
        }
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

/// The reply to the worker's `reference` request. It gives `text` in a `.bpl` reference file's format and, optionally, a list of the `ids` to check, each a case's id or its header's line number. An id that matches no case is an error. The reply gives the cases that fail, with their route directives, as `failures` gives them. As it starts each case, the worker notes `case LINE` on stderr, which names the case where an abort ends it (`bpltests._checked`).
pub fn file_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    let file = file(spec["name"].as_str().unwrap_or_default(), spec["text"].as_str().ok_or("reference needs text as a string")?)?;
    let cases = &file.cases;
    let ids = match spec.get("ids") {
        None | Some(JsonValue::Null) => None, Some(ids) => Some(ids.as_array().ok_or("ids must be a list of case ids and header line numbers")?)
    };
    let matches = |id: &JsonValue, case: &JsonValue| *id == case["id"] || *id == case["line"];
    let unknown: Vec<_> = ids.into_iter().flatten().filter(|id| !cases.iter().any(|case| matches(id, case))).map(JsonValue::to_string).collect();
    if !unknown.is_empty() { return Err(format!("{} match no case. Each id is a case's id or its header's line number.", unknown.join(", "))); }
    let selected = cases.iter().filter(|case| checked(case) && ids.is_none_or(|ids| ids.iter().any(|id| matches(id, case)))).inspect(|case| eprintln!("case {}", case["line"]));
    Ok(json!({"failures": failures(selected, &file.setup, true, interrupt), "error": null}))
}

/// The reply to the worker's `page` request, which gives the `text` of a `.qmd` page. The reply gives the examples that fail, as `page` gives them.
pub fn page_request(spec: &JsonValue, interrupt: &InterruptHandle) -> Result<JsonValue, String> {
    Ok(json!({"failures": page(spec["text"].as_str().ok_or("page needs text as a string")?, interrupt), "error": null}))
}
