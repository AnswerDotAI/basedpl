use crate::{
    data::{text, Options},
    execution::Context,
    keyed, Error, ErrorKind, Number, Value,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::Value as Json;

/// `•json`: parse JSON text. `fill` replaces `null`.
pub(crate) fn parse(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json", left, None, &["fill"], span)?;
    let json = serde_json::from_str(&text(right, span)?).map_err(|e| span.error(ErrorKind::Domain, format!("JSON {e}")))?;
    import(&json, &opts.fill(span)?, span)
}

/// `•tojson`: JSON text for a value. With `fill`, that number exports as `null`.
pub(crate) fn serialize(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•tojson", left, None, &["fill"], span)?;
    let fill = opts.fill(span)?;
    Ok(keyed::text(&export(&exportable(right), opts.values.contains_key("fill").then_some(&fill), span)?.to_string()))
}

/// A copy of `value` without keyed-vector entries that hold functions, at any depth.
pub(crate) fn exportable(value: &Value) -> Value {
    if value.is_atom() || !value.has_functions() { return value.clone(); }
    let keys = value.keys(0).filter(|_| value.shape().len() == 1);
    let (mut names, mut data) = (Vec::new(), Vec::new());
    for (i, e) in value.elements().enumerate() {
        if keys.is_some() && matches!(e, Value::Function(_)) { continue; }
        names.extend(keys.map(|k| k.names()[i].clone()));
        data.push(exportable(&e));
    }
    if keys.is_some() { keyed::vector(names, data) } else { value.layout().collect(data, value.prototype()) }.expect("subset of a valid array")
}

fn import(value: &Json, fill: &Number, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    let value = match value {
        Json::Null => Value::Number(fill.clone()),
        Json::Bool(b) => Value::Number(Number::from_integer(i64::from(*b))),
        Json::String(s) => keyed::text(s),
        Json::Number(n) => {
            let s = n.as_str();
            let number = if let Some(i) = n.as_i64() { Ok(Number::from_integer(i)) } else if s.contains(['.', 'e', 'E']) { n.as_f64().filter(|n| n.is_finite()).ok_or(ErrorKind::Domain).and_then(Number::try_from) } else { s.parse::<BigInt>().map_err(|_| ErrorKind::Domain).and_then(|n| Number::try_from(BigRational::from_integer(n))) };
            Value::Number(number.map_err(|k| span.error(k, "JSON number is outside the supported range"))?)
        }
        Json::Array(items) => {
            let data = items.iter().map(|v| import(v, fill, span)).collect::<Result<Vec<_>, _>>()?;
            Value::from_parts(vec![data.len()], data, Value::Number(Number::from_integer(0))).map_err(|k| span.error(k, "invalid JSON array"))?
        }
        Json::Object(items) => {
            let keys = items.keys().map(|k| k.as_str().into()).collect();
            let data = items.values().map(|v| import(v, fill, span)).collect::<Result<_, _>>()?;
            keyed::vector(keys, data).map_err(|k| span.error(k, "invalid JSON object"))?
        }
    };
    Ok(value)
}

fn export(value: &Value, fill: Option<&Number>, span: &Context<'_>) -> Result<Json, Error> {
    span.check()?;
    match value {
        Value::Number(n) => {
            if fill.is_some_and(|f| n.grade_order(f).is_eq()) { return Ok(Json::Null); }
            if let Some(n) = n.as_integer() { return Ok(n.into()); }
            if let Some(n) = n.as_float().and_then(serde_json::Number::from_f64) { return Ok(Json::Number(n)); }
            if let Some(n) = n.as_exact().filter(|n| n.is_integer()) { return Ok(Json::Number(n.numer().to_string().parse().expect("decimal integer"))); }
            Err(span.error(ErrorKind::Domain, "JSON numbers must be finite floats or integers"))
        }
        Value::Function(_) => Err(span.error(ErrorKind::Domain, "JSON cannot encode functions")),
        _ if value.keys(0).is_none() && keyed::name(value).is_some() => Ok(Json::String(text(value, span)?)),
        _ if value.shape().is_empty() => export(&value.at(0), fill, span),
        _ => {
            let cells = value.cells(value.shape().len() - 1).map_err(|k| span.error(k, "invalid JSON array"))?;
            let values = (0..cells.len())
                .map(|i| {
                    let cell = cells.get(i).map_err(|k| span.error(k, "invalid JSON cell"))?;
                    export(&cell, fill, span)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(keys) = value.keys(0) { Ok(Json::Object(keys.names().iter().zip(values).map(|(k, v)| (k.to_string(), v)).collect())) } else { Ok(Json::Array(values)) }
        }
    }
}
