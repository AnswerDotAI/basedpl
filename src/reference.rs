//! Development-only reference checking, shared by tests and the worker frontend.
use crate::{Array, Element, EvalOptions, Session};
use serde_json::{json, Value};

fn element(value: &Value) -> Option<Element> {
    if let Some(n) = value.as_f64() { Some(Element::Number(n.try_into().ok()?)) } else if let Some(s) = value.as_str() {
        let mut chars = s.chars();
        let c = chars.next()?;
        if chars.next().is_some() { return None; }
        Some(Element::Character(c))
    } else if let Some(sign) = value.get("infinity").and_then(Value::as_i64).filter(|n| matches!(n, -1 | 1)) {
        Some(Element::Number((sign as f64 * f64::INFINITY).try_into().ok()?))
    } else if let Some(z) = value.get("complex").and_then(Value::as_array) {
        if z.len() != 2 { return None; }
        Some(Element::Number(num_complex::Complex64::new(z[0].as_f64()?, z[1].as_f64()?).try_into().ok()?))
    } else { Some(Element::Nested(expected_array(value)?)) }
}

fn expected_array(value: &Value) -> Option<Array> {
    let shape = value["shape"].as_array()?.iter().map(|n| usize::try_from(n.as_u64()?).ok()).collect::<Option<_>>()?;
    let data = value["data"].as_array()?.iter().map(element).collect::<Option<_>>()?;
    let prototype = element(&value["prototype"])?;
    let result = Array::from_parts(shape, data, prototype.clone()).ok()?;
    (result.prototype() == &prototype).then_some(result)
}

fn same_element(x: &Element, y: &Element, relative: f64, absolute: f64) -> bool {
    match (x, y) {
        (Element::Nested(x), Element::Nested(y)) => difference(x, y, relative, absolute).is_none(),
        (Element::Number(x), Element::Number(y)) if x.is_exact() && y.as_float().is_some() => {
            x.as_exact() == num_rational::BigRational::from_float(y.as_float().unwrap())
        }
        (Element::Number(x), Element::Number(y)) if relative != 0.0 || absolute != 0.0 => {
            let complex = |n: &crate::Number| n.as_complex().or_else(|| n.as_float().map(|x| num_complex::Complex64::new(x, 0.)));
            match (complex(x), complex(y)) {
                (Some(a), Some(b)) => a == b || (a.is_finite() && b.is_finite() && (a - b).norm() <= absolute.max(relative * a.norm().max(b.norm()))),
                _ => x == y,
            }
        }
        _ => x == y,
    }
}

fn difference(x: &Array, y: &Array, relative: f64, absolute: f64) -> Option<String> {
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if !same_element(x.prototype(), y.prototype(), relative, absolute) { return Some("prototype".into()); }
    x.elements().zip(y.elements()).position(|(a, b)| !same_element(&a, &b, relative, absolute)).map(|i| format!("data[{i}]"))
}

/// Check an independent captured value or APL expectation. Each side receives a fresh session.
pub fn check(case: &Value, options: EvalOptions) -> Value {
    let Some(code) = case["code"].as_str() else { return json!({"status":"invalid", "message":"missing code"}); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let output = match case.get("expected_output") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        _ => return json!({"status":"invalid", "message":"output expectation must be text"}),
    };
    let (expected, no_result) = if let Some(source) = case["expected_code"].as_str() {
        let expected = Session::new().eval_with(source, EvalOptions { timeout: options.timeout, interrupt: options.interrupt.clone(), echo: false });
        if expected.error.is_some() || expected.function.is_some() {
            return json!({"status":"invalid", "message":"expectation must produce a value or no result", "actual":crate::protocol::response(expected)});
        }
        let no_result = expected.value.is_none();
        (expected.value, no_result)
    } else { (expected_array(&case["expected"]), case.get("expected") == Some(&Value::Null)) };
    let relative = case["relative_tolerance"].as_f64().unwrap_or(0.0);
    let absolute = case["absolute_tolerance"].as_f64().unwrap_or(0.0);
    if [relative, absolute].iter().any(|t| !t.is_finite() || *t < 0.0) || (error_kind.is_none() && expected.is_none() && !no_result) {
        return json!({"status":"invalid", "message":"invalid or missing independent expectation"});
    }
    let result = Session::new().eval_with(code, EvalOptions { echo: false, ..options });
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
                (actual != &expected).then(|| format!("representation: {actual:?} != {expected:?}"))
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
