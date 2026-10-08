use crate::{keyed, Error, ErrorKind, EvalOptions, Evaluation, Number, Session, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::{json, Value as JsonValue};

/// An item as JSON. `None` for a function, which JSON can't hold.
fn element(e: &Value) -> Option<JsonValue> {
    Some(match e {
        Value::Number(n) => {
            if let Some(b) = n.as_bool() { json!(b) } else if let Some(n) = crate::json::integer(n) { n } else if let Some(n) = n.as_exact() { json!({"rational": [n.numer().to_string(), n.denom().to_string()]}) } else if let Some(n) = n.as_complex() { json!({"complex": [n.re, n.im]}) } else {
                let n = n.as_float().unwrap();
                if n.is_nan() { json!({"nan": 1}) } else if n.is_infinite() { json!({"infinity": if n.is_sign_positive() { 1 } else { -1 }}) } else { json!(n) }
            }
        }
        Value::Character(c) => json!(c.to_string()),
        a @ Value::Array(_) => return array(a),
        Value::Function(_) | Value::Operator(_) => return None,
    })
}

/// An array as JSON. `None` when it holds a function.
fn array(a: &Value) -> Option<JsonValue> {
    if a.is_atom() { return element(a); }
    let data = a.elements().map(|e| element(&e)).collect::<Option<Vec<_>>>()?;
    let mut encoded = json!({"shape": a.shape(), "data": data, "prototype": element(&a.prototype())?});
    if !a.axis_names().is_empty() { encoded["axis_names"] = json!(a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>()); }
    if a.has_keys() { encoded["axis_keys"] = json!(keyed::key_lists(a)); }
    Some(encoded)
}

/// A one-character string is a character.
pub fn character(s: &str) -> Option<char> { let mut chars = s.chars(); chars.next().filter(|_| chars.next().is_none()) }

fn import_element(value: &JsonValue, approximate: bool) -> Result<Value, String> {
    let number = match value {
        JsonValue::Number(n) if approximate => n.as_f64().map(Number::from).ok_or(ErrorKind::Domain),
        JsonValue::Number(n) => crate::json::number(n),
        JsonValue::Bool(b) => Ok(Number::from_bool(*b)),
        JsonValue::String(s) => return character(s).map(Value::Character).ok_or_else(|| "expected one character".into()),
        JsonValue::Object(o) if o.contains_key("infinity") => {
            let sign = o["infinity"].as_i64().filter(|n| matches!(n, -1 | 1)).ok_or("infinity sign must be 1 or -1")?;
            Ok(Number::from(sign as f64 * f64::INFINITY))
        }
        JsonValue::Object(o) if o.contains_key("nan") => Ok(Number::from(f64::NAN)),
        JsonValue::Object(o) if o.contains_key("rational") => {
            let parts = o["rational"].as_array().filter(|a| a.len() == 2).ok_or("expected rational numerator and denominator")?;
            let integer = |i: usize| parts[i].as_str().ok_or("expected decimal integer string")?.parse::<BigInt>().map_err(|_| "invalid integer");
            Number::try_from(BigRational::new_raw(integer(0)?, integer(1)?))
        }
        JsonValue::Object(o) if o.contains_key("complex") => {
            let parts = o["complex"].as_array().filter(|a| a.len() == 2).ok_or("expected real and imaginary components")?;
            Ok(Number::from(num_complex::Complex64::new(
                parts[0].as_f64().ok_or("invalid real component")?,
                parts[1].as_f64().ok_or("invalid imaginary component")?,
            )))
        }
        JsonValue::Object(o) if o.contains_key("shape") => return import_array(value, approximate),
        _ => return Err("invalid BPL element".into()),
    };
    number.map(Value::Number).map_err(|k| k.to_string())
}

fn import_array(value: &JsonValue, approximate: bool) -> Result<Value, String> {
    if value.get("shape").is_none() { return import_element(value, approximate); }
    let shape = value["shape"]
        .as_array()
        .ok_or("expected array shape")?
        .iter()
        .map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid array dimension"))
        .collect::<Result<Vec<_>, _>>()?;
    let data = value["data"].as_array().ok_or("expected array data")?.iter().map(|v| import_element(v, approximate)).collect::<Result<Vec<_>, _>>()?;
    let prototype = import_element(&value["prototype"], approximate)?;
    let mut result = Value::from_parts(shape, data, prototype).map_err(|k| k.to_string())?;
    if let Some(names) = value.get("axis_names") { result = keyed::with_names(result, serde_json::from_value(names.clone()).map_err(|e| e.to_string())?)?; }
    let Some(keys) = value.get("axis_keys") else { return Ok(result); };
    keyed::with_key_lists(result, serde_json::from_value(keys.clone()).map_err(|e| e.to_string())?)
}

/// An array from its JSON encoding. Numbers captured from another interpreter carry no exactness, so `approximate` reads them all as
/// floats.
pub(crate) fn import(value: &JsonValue, approximate: bool) -> Result<Value, String> { import_array(value, approximate) }

/// The evaluation options a request sets: `timeout_ms`, and `echo`, which defaults to true.
pub fn options(request: &JsonValue) -> Result<EvalOptions, String> {
    let timeout =
        request.get("timeout_ms").map(|v| v.as_u64().map(std::time::Duration::from_millis).ok_or("timeout_ms must be a nonnegative integer")).transpose()?;
    let echo = request.get("echo").map(|v| v.as_bool().ok_or("echo must be a boolean")).transpose()?.unwrap_or(true);
    Ok(EvalOptions { timeout, echo, ..EvalOptions::default() })
}
/// Worker operations use the same array encoding in both directions.
pub fn request(session: &mut Session, request: &JsonValue, options: EvalOptions) -> Result<Evaluation, String> {
    let code = request.get("code").map(|v| v.as_str().ok_or("code must be a string")).transpose()?;
    let function = request.get("call").map(|v| v.as_str().ok_or("call must be a function expression string")).transpose()?;
    if code.is_some() && function.is_some() { return Err("choose code or call, not both".into()); }
    if request.get("args").is_some() && function.is_none() { return Err("args requires call".into()); }
    if code.is_none() && function.is_none() && request.get("bindings").is_none() { return Err("expected code, call or bindings".into()); }
    let args = if function.is_some() {
        request["args"].as_array().ok_or("call requires an args array")?.iter().map(|v| import(v, false)).collect::<Result<Vec<_>, _>>()?
    } else { Vec::new() };
    if let Some(bindings) = request.get("bindings") {
        let bindings = bindings
            .as_object()
            .ok_or("bindings must be an object")?
            .iter()
            .map(|(name, value)| Ok((name, import(value, false)?)))
            .collect::<Result<Vec<_>, String>>()?;
        for (name, value) in bindings { session.set(name, value).map_err(|_| "binding requires an ordinary BPL name")?; }
    }
    Ok(match (code, function) {
        (Some(code), _) => session.eval_with(code, options),
        (_, Some(function)) => session.call_with(function, &args, options),
        _ => Evaluation::default(),
    })
}

/// A span as its source and byte range.
fn location(s: &crate::Span) -> JsonValue { json!({"source": {"name": s.source.name, "text": s.source.text}, "span": [s.range.start, s.range.end]}) }

pub fn error(e: &Error) -> JsonValue {
    let mut encoded = location(&e.span);
    encoded["kind"] = json!(e.kind.name());
    encoded["message"] = json!(e.message);
    encoded["display"] = json!(e.to_string());
    encoded["calls"] = e.calls.iter().map(location).collect();
    encoded
}

/// JSON carries bytes as base64 text, as Jupyter messages do.
impl serde::Serialize for crate::MimeData {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        match self { Self::Text(text) => s.serialize_str(text), Self::Bytes(bytes) => s.serialize_str(&STANDARD.encode(bytes)) }
    }
}

/// An output as JSON: its kind, and its MIME bundle.
pub fn output(o: &crate::Output) -> JsonValue { json!({"kind": o.kind.name(), "data": o.data}) }

pub fn response(mut result: Evaluation) -> JsonValue {
    let value = result.value.as_ref().map(|v| array(&crate::json::exportable(v)));
    if result.function.is_some() || matches!(value, Some(None)) {
        let span = crate::Span { source: crate::Source::new("<json>", ""), range: 0..0 };
        if result.error.is_none() { result.error = Some(span.error(crate::ErrorKind::Domain, "functions cannot be exported through JSON")); }
    }
    json!({"value": value.flatten(), "output": result.output.iter().map(output).collect::<Vec<_>>(), "error": result.error.as_ref().map(error)})
}

/// The reply to a request that can't run.
pub fn request_error(message: &str) -> JsonValue { json!({"value": null, "output": [], "error": {"kind": "REQUEST", "message": message}}) }
