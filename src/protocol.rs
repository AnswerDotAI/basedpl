use crate::{Error, EvalOptions, Evaluation, Number, Session, Value, keyed};
use num_bigint::BigInt;
use num_rational::BigRational;
use rustymath::{buffer::FloatWidth, error::Kind};
use serde_json::{Value as JsonValue, json};

/// A value as the process interface's JSON, serialized as it's written. An item is a JSON scalar, or an object for a number JSON can't hold. An array is an object of its shape, items and prototype, with its axis names and keys where it has them. The value holds no function, as `Value::holds_function` checks.
struct Json<'a>(&'a Value);

impl serde::Serialize for Json<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error as _, SerializeMap};
        fn tagged<S: serde::Serializer>(s: S, key: &str, value: impl serde::Serialize) -> Result<S::Ok, S::Error> {
            let mut map = s.serialize_map(Some(1))?;
            map.serialize_entry(key, &value)?;
            map.end()
        }
        match self.0 {
            Value::Number(n) => {
                if let Some(b) = n.as_bool() { return s.serialize_bool(b); }
                if let Some(i) = n.as_integer() { return s.serialize_i64(i); }
                if let Some(i) = crate::json::integer(n) { return i.serialize(s); }
                if let Some(q) = n.exact_ref() { return tagged(s, "rational", [q.numer().to_string(), q.denom().to_string()]); }
                if let Some(z) = n.as_complex() { return tagged(s, "complex", [z.re, z.im]); }
                let x = n.as_float().unwrap();
                if x.is_nan() { tagged(s, "nan", 1) }
                else if x.is_infinite() { tagged(s, "infinity", if x.is_sign_positive() { 1 }
                else { -1 }) }
                else { s.serialize_f64(x) }
            }
            Value::Character(c) => s.serialize_char(*c),
            a @ Value::Array(_) => {
                let mut map = s.serialize_map(None)?;
                map.serialize_entry("shape", a.shape())?;
                map.serialize_entry("data", &Data(a))?;
                map.serialize_entry("prototype", &Json(&a.prototype()))?;
                if a.layout().has_names() { map.serialize_entry("axis_names", &a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>())?; }
                if a.has_keys() { map.serialize_entry("axis_keys", &keyed::key_lists(a))?; }
                map.end()
            }
            Value::Carried(carrier) => Json(carrier.template()).serialize(s),
            Value::Function(_) | Value::Operator(_) => Err(S::Error::custom("a function has no JSON value")),
        }
    }
}

/// An array's items, as a JSON list.
struct Data<'a>(&'a Value);

impl serde::Serialize for Data<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for e in self.0.elements() { seq.serialize_element(&Json(&e))?; }
        seq.end()
    }
}

/// A one-character string is a character.
pub fn character(s: &str) -> Option<char> { let mut chars = s.chars(); chars.next().filter(|_| chars.next().is_none()) }

/// An element of the JSON protocol. Its floats are 64-bit, because the protocol carries no width. A complex number follows the current kinds, as `Number::complex_in_session` says.
fn import_element(value: &JsonValue, approximate: bool) -> Result<Value, String> {
    let number = match value {
        JsonValue::Number(n) if approximate => n.as_f64().map(|x| Number::float(x, FloatWidth::F64)).ok_or_else(|| Kind::Domain.into()),
        JsonValue::Number(n) => crate::json::number(n, FloatWidth::F64),
        JsonValue::Bool(b) => Ok(Number::from_bool(*b)),
        JsonValue::String(s) => return character(s).map(Value::Character).ok_or_else(|| "expected one character".into()),
        JsonValue::Object(o) if o.contains_key("infinity") => {
            let sign = o["infinity"].as_i64().filter(|n| matches!(n, -1 | 1)).ok_or("infinity sign must be 1 or -1")?;
            Ok(Number::float(sign as f64 * f64::INFINITY, FloatWidth::F64))
        }
        JsonValue::Object(o) if o.contains_key("nan") => Ok(Number::float(f64::NAN, FloatWidth::F64)),
        JsonValue::Object(o) if o.contains_key("rational") => {
            let parts = o["rational"].as_array().filter(|a| a.len() == 2).ok_or("expected rational numerator and denominator")?;
            let integer = |i: usize| parts[i].as_str().ok_or("expected decimal integer string")?.parse::<BigInt>().map_err(|_| "invalid integer");
            Number::try_from(BigRational::new_raw(integer(0)?, integer(1)?))
        }
        JsonValue::Object(o) if o.contains_key("complex") => {
            let parts = o["complex"].as_array().filter(|a| a.len() == 2).ok_or("expected real and imaginary components")?;
            Ok(Number::complex_in_session(
                num_complex::Complex64::new(parts[0].as_f64().ok_or("invalid real component")?, parts[1].as_f64().ok_or("invalid imaginary component")?),
                FloatWidth::F64))
        }
        JsonValue::Object(o) if o.contains_key("shape") => return import_array(value, approximate),
        _ => return Err("invalid BPL element".into()),
    };
    number.map(Value::Number).map_err(|k| k.to_string())
}

fn import_array(value: &JsonValue, approximate: bool) -> Result<Value, String> {
    if value.get("shape").is_none() { return import_element(value, approximate); }
    let shape = value["shape"].as_array().ok_or("expected array shape")?.iter()
        .map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid array dimension")).collect::<Result<Vec<_>, _>>()?;
    let data = value["data"].as_array().ok_or("expected array data")?.iter().map(|v| import_element(v, approximate)).collect::<Result<Vec<_>, _>>()?;
    let prototype = import_element(&value["prototype"], approximate)?;
    let mut result = Value::from_parts(shape, data, prototype).map_err(|k| k.to_string())?;
    if let Some(names) = value.get("axis_names") { result = keyed::with_names(result, serde_json::from_value(names.clone()).map_err(|e| e.to_string())?)?; }
    let Some(keys) = value.get("axis_keys") else { return Ok(result); };
    keyed::with_key_lists(result, serde_json::from_value(keys.clone()).map_err(|e| e.to_string())?)
}

/// An array from its JSON encoding. Numbers captured from another interpreter carry no exactness, so `approximate` reads them all as floats.
pub(crate) fn import(value: &JsonValue, approximate: bool) -> Result<Value, String> { import_array(value, approximate) }

/// The evaluation options a request sets: `timeout_ms`, and `echo`, which defaults to true.
pub fn options(request: &JsonValue) -> Result<EvalOptions, String> {
    let timeout =
        request.get("timeout_ms").map(|v| v.as_u64().map(std::time::Duration::from_millis).ok_or("timeout_ms must be a nonnegative integer")).transpose()?;
    let echo = request.get("echo").map(|v| v.as_bool().ok_or("echo must be a boolean")).transpose()?.unwrap_or(true);
    Ok(EvalOptions { timeout, echo, ..EvalOptions::default() })
}
/// Worker operations use the same array encoding in both directions.
pub fn request(session: &Session, request: &JsonValue, options: EvalOptions) -> Result<Evaluation, String> {
    let code = request.get("code").map(|v| v.as_str().ok_or("code must be a string")).transpose()?;
    let function = request.get("call").map(|v| v.as_str().ok_or("call must be a function expression string")).transpose()?;
    if code.is_some() && function.is_some() { return Err("choose code or call, not both".into()); }
    if request.get("args").is_some() && function.is_none() { return Err("args requires call".into()); }
    if code.is_none() && function.is_none() && request.get("bindings").is_none() { return Err("expected code, call or bindings".into()); }
    let args = if function.is_some() {
        request["args"].as_array().ok_or("call requires an args array")?.iter().map(|v| import(v, false)).collect::<Result<Vec<_>, _>>()?
    } else { Vec::new() };
    if let Some(bindings) = request.get("bindings") {
        let bindings = bindings.as_object().ok_or("bindings must be an object")?.iter().map(|(name, value)| Ok((name, import(value, false)?)))
            .collect::<Result<Vec<_>, String>>()?;
        for (name, value) in bindings { session.set(name, value).map_err(|_| "binding requires an ordinary BPL name")?; }
    }
    Ok(match (code, function) {
        (Some(code), _) => session.eval_with(code, options), (_, Some(function)) => session.call_with(function, &args, options), _ => Evaluation::default()
    })
}

/// A span as its source and byte range.
fn location(s: &crate::Span) -> JsonValue { json!({"source": {"name": s.source.name, "text": s.source.text}, "span": [s.range.start, s.range.end]}) }

pub fn error(e: &Error) -> JsonValue {
    let mut encoded = e.span.as_ref().map_or_else(|| json!({}), location);
    encoded["kind"] = json!(e.kind.name());
    encoded["message"] = json!(e.message);
    encoded["display"] = json!(e.to_string());
    encoded["calls"] = e.calls.iter().map(location).collect();
    encoded
}

/// JSON carries bytes as base64 text, as Jupyter messages do.
impl serde::Serialize for crate::MimeData {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        match self { Self::Text(text) => s.serialize_str(text), Self::Bytes(bytes) => s.serialize_str(&STANDARD.encode(bytes)) }
    }
}

/// An output as JSON: its kind, and its MIME bundle.
pub fn output(o: &crate::Output) -> JsonValue { json!({"kind": o.kind.name(), "data": o.data}) }

/// The reply to a request, serialized as it's written: its `value`, `output` and `error`. A function, or an array that holds one, has no JSON value: `value` is `null`, and `output` holds its display.
pub struct Response(pub Evaluation);

impl serde::Serialize for Response {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let value = self.0.value.as_ref().map(crate::json::exportable).filter(|v| !v.holds_function());
        let mut map = s.serialize_map(Some(3))?;
        map.serialize_entry("value", &value.as_ref().map(Json))?;
        map.serialize_entry("output", &self.0.output.iter().map(output).collect::<Vec<_>>())?;
        map.serialize_entry("error", &self.0.error.as_ref().map(error))?;
        map.end()
    }
}

/// The reply to a request, as a JSON value.
pub fn response(result: Evaluation) -> JsonValue { serde_json::to_value(Response(result)).expect("a response serializes") }

/// The reply to a request that can't run.
pub fn request_error(message: &str) -> JsonValue { json!({"value": null, "output": [], "error": {"kind": "REQUEST", "message": message}}) }
