//! Development-only reference checking, shared by tests and the worker frontend.
use crate::{Array, Element, EvalOptions, Session};
use serde_json::{json, Value};

fn element(value: &Value) -> Option<Element> {
    if let Some(n) = value.as_f64() { Some(Element::Number(n.try_into().ok()?)) } else if let Some(s) = value.as_str() {
        let mut chars = s.chars();
        let c = chars.next()?;
        if chars.next().is_some() { return None; }
        Some(Element::Character(c))
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

fn same_element(x: &Element, y: &Element, tolerance: f64) -> bool {
    match (x, y) {
        (Element::Nested(x), Element::Nested(y)) => difference(x, y, tolerance).is_none(),
        (Element::Number(x), Element::Number(y)) if x.is_exact() && y.as_float().is_some() => {
            x.as_exact() == num_rational::BigRational::from_float(y.as_float().unwrap())
        }
        (Element::Number(x), Element::Number(y)) if tolerance != 0.0 => {
            let complex = |n: &crate::Number| n.as_complex().or_else(|| n.as_float().map(|x| num_complex::Complex64::new(x, 0.)));
            match (complex(x), complex(y)) { (Some(a), Some(b)) => a == b || (a - b).norm() <= tolerance * a.norm().max(b.norm()), _ => x == y }
        }
        _ => x == y,
    }
}

fn difference(x: &Array, y: &Array, tolerance: f64) -> Option<String> {
    if x.shape() != y.shape() { return Some(format!("shape: {:?} != {:?}", x.shape(), y.shape())); }
    if !same_element(x.prototype(), y.prototype(), tolerance) { return Some("prototype".into()); }
    x.elements().zip(y.elements()).position(|(a, b)| !same_element(&a, &b, tolerance)).map(|i| format!("data[{i}]"))
}

/// Evaluate one independent reference case in a fresh session. Never derive an expectation from miniapl.
pub fn check(case: &Value, options: EvalOptions) -> Value {
    let Some(code) = case["code"].as_str().filter(|s| !s.is_empty()) else { return json!({"status":"invalid", "message":"missing code"}); };
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    let expected = expected_array(&case["expected"]);
    let tolerance = case["relative_tolerance"].as_f64().unwrap_or(0.0);
    if (!tolerance.is_finite() || tolerance < 0.0) || (error_kind.is_none() && expected.is_none()) {
        return json!({"status":"invalid", "message":"invalid or missing independent expectation"});
    }
    let result = Session::new().eval_with(code, options);
    if let Some(error) = &result.error {
        let kind = error.kind.to_string();
        if matches!(error.kind, crate::ErrorKind::Timeout | crate::ErrorKind::Interrupt) {
            return json!({"status": kind.to_lowercase(), "message":error.message});
        }
        if error_kind == Some(kind.as_str()) { return json!({"status":"pass"}); }
        return json!({"status":"error", "kind":kind, "message":error.message, "actual":crate::protocol::response(result)});
    }
    let mismatch = match (error_kind, &result.value, expected) {
        (Some(kind), _, _) => Some(format!("expected {kind}")),
        (_, None, _) => Some("no result".into()),
        (_, Some(actual), Some(expected)) => difference(actual, &expected, tolerance),
        _ => unreachable!(),
    };
    match mismatch {
        Some(message) => json!({"status":"mismatch", "message":message, "actual":crate::protocol::response(result)}),
        None => json!({"status":"pass"}),
    }
}
