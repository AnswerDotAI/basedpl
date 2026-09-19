use crate::{Array, Element, Error, EvalOptions, Evaluation, Number, Session};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn element(e: &Element) -> Value {
    match e {
        Element::Number(n) => {
            if let Some(n) = n.as_integer() { json!(n) } else if let Some(n) = n.as_exact() {
                if n.is_integer() { Value::Number(n.numer().to_string().parse().expect("decimal integer")) } else { json!({"rational": [n.numer().to_string(), n.denom().to_string()]}) }
            } else if let Some(n) = n.as_complex() { json!({"complex": [n.re, n.im]}) } else {
                let n = n.as_float().unwrap();
                if n.is_infinite() { json!({"infinity": if n.is_sign_positive() { 1 } else { -1 }}) } else { json!(n) }
            }
        }
        Element::Character(c) => json!(c.to_string()),
        Element::Nested(a) => array(a),
        Element::Function(_) => unreachable!("JSON response rejects function arrays"),
    }
}

fn array(a: &Array) -> Value { json!({"shape": a.shape(), "data": a.elements().map(|e| element(&e)).collect::<Vec<_>>(), "prototype": element(a.prototype())}) }

fn import_element(value: &Value, depth: usize) -> Result<Element, String> {
    let number = match value {
        Value::Number(n) => {
            let text = n.to_string();
            if text.contains(['.', 'e', 'E']) { Number::try_from(n.as_f64().ok_or("invalid float")?) } else { Number::try_from(BigRational::from_integer(text.parse::<BigInt>().map_err(|_| "invalid integer")?)) }
        }
        Value::String(s) => {
            let mut chars = s.chars();
            let c = chars.next().ok_or("expected one character")?;
            if chars.next().is_some() { return Err("expected one character".into()); }
            return Ok(Element::Character(c));
        }
        Value::Object(o) if o.contains_key("infinity") => {
            let sign = o["infinity"].as_i64().filter(|n| matches!(n, -1 | 1)).ok_or("infinity sign must be 1 or -1")?;
            Number::try_from(sign as f64 * f64::INFINITY)
        }
        Value::Object(o) if o.contains_key("rational") => {
            let parts = o["rational"].as_array().filter(|a| a.len() == 2).ok_or("expected rational numerator and denominator")?;
            let integer = |i: usize| parts[i].as_str().ok_or("expected decimal integer string")?.parse::<BigInt>().map_err(|_| "invalid integer");
            Number::try_from(BigRational::new_raw(integer(0)?, integer(1)?))
        }
        Value::Object(o) if o.contains_key("complex") => {
            let parts = o["complex"].as_array().filter(|a| a.len() == 2).ok_or("expected real and imaginary components")?;
            Number::try_from(num_complex::Complex64::new(
                parts[0].as_f64().ok_or("invalid real component")?,
                parts[1].as_f64().ok_or("invalid imaginary component")?,
            ))
        }
        Value::Object(_) => return import_array(value, depth + 1).map(Element::Nested),
        _ => return Err("invalid APL element".into()),
    };
    number.map(Element::Number).map_err(|k| k.to_string())
}

fn import_array(value: &Value, depth: usize) -> Result<Array, String> {
    if depth > 128 { return Err("array nesting exceeds 128 levels".into()); }
    let shape = value["shape"]
        .as_array()
        .ok_or("expected array shape")?
        .iter()
        .map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid array dimension"))
        .collect::<Result<Vec<_>, _>>()?;
    let data = value["data"].as_array().ok_or("expected array data")?.iter().map(|v| import_element(v, depth)).collect::<Result<Vec<_>, _>>()?;
    let prototype = import_element(&value["prototype"], depth)?;
    Array::from_parts(shape, data, prototype).map_err(|k| k.to_string())
}

/// Worker operations use the same array encoding in both directions.
pub(crate) fn request(session: &mut Session, request: &Value, options: EvalOptions) -> Result<Evaluation, String> {
    let code = request.get("code").map(|v| v.as_str().ok_or("code must be a string")).transpose()?;
    let function = request.get("call").map(|v| v.as_str().ok_or("call must be a function expression string")).transpose()?;
    if code.is_some() && function.is_some() { return Err("choose code or call, not both".into()); }
    if request.get("args").is_some() && function.is_none() { return Err("args requires call".into()); }
    if code.is_none() && function.is_none() && request.get("bindings").is_none() { return Err("expected code, call or bindings".into()); }
    let args = if function.is_some() {
        request["args"].as_array().ok_or("call requires an args array")?.iter().map(|v| import_array(v, 0)).collect::<Result<Vec<_>, _>>()?
    } else { Vec::new() };
    if let Some(bindings) = request.get("bindings") {
        let bindings = bindings
            .as_object()
            .ok_or("bindings must be an object")?
            .iter()
            .map(|(name, value)| Ok((name, import_array(value, 0)?)))
            .collect::<Result<Vec<_>, String>>()?;
        for (name, value) in bindings { session.set(name, value).map_err(|_| "binding requires an ordinary APL name")?; }
    }
    Ok(match (code, function) {
        (Some(code), _) => session.eval_with(code, options),
        (_, Some(function)) => session.call_with(function, &args, options),
        _ => Evaluation::default(),
    })
}

fn error(e: &Error) -> Value {
    json!({"kind": e.kind.to_string(), "message": e.message, "display": e.to_string(),
        "source": {"name": e.span.source.name, "text": e.span.source.text}, "span": [e.span.range.start, e.span.range.end],
        "calls": e.calls.iter().map(|s| json!({"source": {"name": s.source.name, "text": s.source.text}, "span": [s.range.start, s.range.end]})).collect::<Vec<_>>()})
}

pub(crate) fn response(mut result: Evaluation) -> Value {
    if result.function.is_some() || result.value.as_ref().is_some_and(Array::has_functions) {
        let span = crate::Span { source: crate::Source::new("<json>", ""), range: 0..0 };
        result.value = None;
        if result.error.is_none() { result.error = Some(span.error(crate::ErrorKind::Domain, "functions cannot be exported through JSON")); }
    }
    json!({"value": result.value.as_ref().map(array), "output": result.output, "error": result.error.as_ref().map(error)})
}

pub(crate) fn run(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<()> {
    let mut session = Session::new();
    for line in input.lines() {
        let line = line?;
        let reply = match serde_json::from_str::<String>(&line) {
            Ok(code) => response(session.eval(&code)),
            Err(_) => json!({"value": null, "output": [], "error": {"kind": "REQUEST ERROR", "message": "expected a JSON string containing APL source"}}),
        };
        serde_json::to_writer(&mut *output, &reply)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
