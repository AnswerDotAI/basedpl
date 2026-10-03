//! Development-only reference checking, shared by tests and the worker frontend.
use crate::{EvalOptions, Session, Value};
use serde_json::{json, Value as JsonValue};

/// A value captured from another interpreter. Its numbers carry no exactness. A capture encodes a simple atom as a scalar array, and BPL
/// expectations use based values directly.
pub(crate) fn expected_array(value: &JsonValue) -> Option<Value> {
    let result = crate::protocol::import(value, true).ok()?;
    let prototype = crate::protocol::import(&value["prototype"], true).ok()?;
    let result = if result.shape().is_empty() && !result.is_atom() && result.at(0).is_atom() { result.at(0) } else { result };
    (result.prototype() == prototype).then_some(result)
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
        _ => x == y,
    }
}

/// Locate a structural or numeric mismatch. Approximate numbers may differ by the tolerances. With `exactness`, each number's
/// exactness must match too.
pub fn difference(x: &Value, y: &Value, relative: f64, absolute: f64, exactness: bool) -> Option<String> {
    let same = |a: &Value, b: &Value| same_element(a, b, relative, absolute, exactness);
    if x.is_atom() != y.is_atom() { return Some("atom versus array".into()); }
    if x.is_atom() { return (!same(x, y)).then(|| "atom".into()); }
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if x.layout() != y.layout() { return Some("axis keys or names".into()); }
    if !same(&x.prototype(), &y.prototype()) { return Some("prototype".into()); }
    x.elements().zip(y.elements()).position(|(a, b)| !same(&a, &b)).map(|i| format!("data[{i}]"))
}

/// Check an independent captured value or BPL expectation. Each side receives a fresh session. A BPL expectation must match each
/// number's exactness. A value captured from another interpreter carries none, so its numbers compare by value.
pub fn check(case: &JsonValue, options: EvalOptions) -> JsonValue {
    let Some(code) = case["code"].as_str() else { return json!({"status":"invalid", "message":"missing code"}); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let output = match case.get("expected_output") {
        None => None,
        Some(JsonValue::String(s)) => Some(s.as_str()),
        _ => return json!({"status":"invalid", "message":"output expectation must be text"}),
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
    let result = session.eval_with(code, EvalOptions { echo: false, ..options });
    if let Some(error) = &result.error {
        let kind = error.kind.to_string();
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) {
            return json!({"status": kind.to_lowercase(), "message":error.message});
        }
        if error_kind != Some(kind.as_str()) {
            return json!({"status":"error", "kind":kind, "message":error.message, "actual":crate::protocol::response(result)});
        }
    }
    let mismatch = if result.error.is_some() {
        None
    } else if result.function.is_some() || result.operator.is_some() {
        Some("unexpected function or operator result".into())
    } else {
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
            let actual = result.output.iter().map(crate::Output::text).collect::<Vec<_>>().join("\n");
            (actual != expected).then(|| format!("output: {actual:?} != {expected:?}"))
        })
    });
    match mismatch {
        Some(message) => json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result)}),
        None => json!({"status":"pass"}),
    }
}
