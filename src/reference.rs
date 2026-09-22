//! Development-only reference checking, shared by tests and the worker frontend.
use crate::{EvalOptions, Session, Value};
use serde_json::{json, Value as JsonValue};

fn element(value: &JsonValue) -> Option<Value> {
    if let Some(n) = value.as_f64() { Some(Value::Number(n.try_into().ok()?)) } else if let Some(s) = value.as_str() {
        let mut chars = s.chars();
        let c = chars.next()?;
        if chars.next().is_some() { return None; }
        Some(Value::Character(c))
    } else if let Some(sign) = value.get("infinity").and_then(JsonValue::as_i64).filter(|n| matches!(n, -1 | 1)) {
        Some(Value::Number((sign as f64 * f64::INFINITY).try_into().ok()?))
    } else if let Some(z) = value.get("complex").and_then(JsonValue::as_array) {
        if z.len() != 2 { return None; }
        Some(Value::Number(num_complex::Complex64::new(z[0].as_f64()?, z[1].as_f64()?).try_into().ok()?))
    } else { Some(expected_array(value)?) }
}

fn expected_array(value: &JsonValue) -> Option<Value> {
    let shape: Vec<_> = value["shape"].as_array()?.iter().map(|n| usize::try_from(n.as_u64()?).ok()).collect::<Option<_>>()?;
    let data: Vec<_> = value["data"].as_array()?.iter().map(element).collect::<Option<_>>()?;
    let prototype = element(&value["prototype"])?;
    // Dyalog oracle captures encode simple atoms as scalar arrays. APL expectations use based values directly.
    let result =
        if shape.is_empty() && data.len() == 1 && data[0].is_atom() { data[0].clone() } else { Value::from_parts(shape, data, prototype.clone()).ok()? };
    (result.prototype() == prototype).then_some(result)
}

fn same_element(x: &Value, y: &Value, relative: f64, absolute: f64) -> bool {
    match (x, y) {
        (x @ Value::Array(_), y @ Value::Array(_)) => difference(x, y, relative, absolute).is_none(),
        (Value::Number(x), Value::Number(y)) if x.is_exact() && y.as_float().is_some() => {
            x.as_exact() == num_rational::BigRational::from_float(y.as_float().unwrap())
        }
        (Value::Number(x), Value::Number(y)) if relative != 0.0 || absolute != 0.0 => {
            let complex = |n: &crate::Number| n.as_complex().or_else(|| n.as_float().map(|x| num_complex::Complex64::new(x, 0.)));
            match (complex(x), complex(y)) {
                (Some(a), Some(b)) => a == b || (a.is_finite() && b.is_finite() && (a - b).norm() <= absolute.max(relative * a.norm().max(b.norm()))),
                _ => x == y,
            }
        }
        _ => x == y,
    }
}

/// Locate a structural or numeric mismatch, allowing the supplied floating-point tolerances.
pub fn difference(x: &Value, y: &Value, relative: f64, absolute: f64) -> Option<String> {
    if x.is_atom() != y.is_atom() { return Some("atom versus array".into()); }
    if x.is_atom() { return (!same_element(x, y, relative, absolute)).then(|| "atom".into()); }
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if x.layout() != y.layout() { return Some("axis keys or names".into()); }
    if !same_element(&x.prototype(), &y.prototype(), relative, absolute) { return Some("prototype".into()); }
    x.elements().zip(y.elements()).position(|(a, b)| !same_element(&a, &b, relative, absolute)).map(|i| format!("data[{i}]"))
}

/// Check an independent captured value or APL expectation. Each side receives a fresh session.
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
        if expected.error.is_some() || expected.function.is_some() {
            return json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected)});
        }
        let no_result = expected.value.is_none();
        (expected.value, no_result)
    } else { (expected_array(&case["expected"]), case.get("expected") == Some(&JsonValue::Null)) };
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
    let mismatch = if result.error.is_some() { None } else if result.function.is_some() { Some("unexpected function result".into()) } else {
        match (error_kind, &result.value, expected) {
            (Some(kind), _, _) => Some(format!("expected {kind}")),
            (_, None, _) if no_result => None,
            (_, None, _) => Some("no result".into()),
            (_, Some(_), _) if no_result => Some("expected no result".into()),
            (_, Some(actual), Some(expected)) if case["exact_representation"] == true => {
                (actual != &expected).then(|| format!("representation: {actual} != {expected}"))
            }
            (_, Some(actual), Some(expected)) => difference(actual, &expected, relative, absolute),
            _ => unreachable!(),
        }
    };
    let mismatch = mismatch.or_else(|| {
        output.and_then(|expected| {
            let actual = result.output.join("\n");
            (actual != expected).then(|| format!("output: {actual:?} != {expected:?}"))
        })
    });
    match mismatch {
        Some(message) => json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result)}),
        None => json!({"status":"pass"}),
    }
}
