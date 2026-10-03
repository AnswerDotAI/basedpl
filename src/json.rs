use crate::{
    array::Items,
    data::{imported, text, Options},
    execution::Context,
    keyed, Error, ErrorAt, ErrorKind, Number, Value,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::Value as Json;

/// A JSON number: exact when written without a point or exponent, and otherwise a float.
pub(crate) fn number(n: &serde_json::Number) -> Result<Number, ErrorKind> {
    if let Some(i) = n.as_i64() { return Ok(Number::from_integer(i)); }
    if n.as_str().contains(['.', 'e', 'E']) { return n.as_f64().map(Number::from).ok_or(ErrorKind::Domain); }
    n.as_str().parse::<BigInt>().map_err(|_| ErrorKind::Domain).and_then(|i| Number::try_from(BigRational::from_integer(i)))
}

/// An exact integer as a JSON number, whatever its size.
pub(crate) fn integer(n: &Number) -> Option<Json> {
    if let Some(i) = n.as_integer() { return Some(i.into()); }
    n.as_exact().filter(|q| q.is_integer()).map(|q| Json::Number(q.numer().to_string().parse().expect("decimal integer")))
}

/// `•json`: parse JSON text. `fill` replaces `null`.
pub(crate) fn parse(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json", left, None, &["fill"], span)?;
    let json = serde_json::from_str(&text(right, span)?).map_err(|e| span.domain_error(format!("JSON {e}")))?;
    import(&json, &opts.fill(span)?, span)
}

/// `•tojson`: JSON text for a value. NaN exports as `null`, and so does the number given as `fill`.
pub(crate) fn serialize(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•tojson", left, None, &["fill"], span)?;
    let fill = opts.fill(span)?;
    Ok(keyed::text(&export(&exportable(right), opts.values.contains_key("fill").then_some(&fill), span)?.to_string()))
}

/// A copy of `value` without keyed-vector entries that hold functions, at any depth. A value with nothing to drop comes back as it is.
pub(crate) fn exportable(value: &Value) -> Value {
    let Items::Values(items) = value.as_items() else { return value.clone() };
    if value.is_atom() { return value.clone(); }
    let keys = value.keys(0).filter(|_| value.shape().len() == 1);
    let (mut names, mut data, mut changed) = (Vec::new(), Vec::new(), false);
    for (i, e) in items.iter().enumerate() {
        if keys.is_some() && matches!(e, Value::Function(_) | Value::Operator(_)) {
            changed = true;
            continue;
        }
        names.extend(keys.map(|k| k.names()[i].clone()));
        let item = exportable(e);
        changed |= item.storage_id() != e.storage_id();
        data.push(item);
    }
    if !changed { return value.clone(); }
    if keys.is_some() { keyed::partial_vector(names, data) } else { value.layout().collect(data, || value.prototype()) }.expect("subset of a valid array")
}

fn import(value: &Json, fill: &Number, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    let value = match value {
        Json::Null => Value::Number(fill.clone()),
        Json::Bool(b) => Value::Number(Number::from_bool(*b)),
        Json::String(s) => keyed::text(s),
        Json::Number(n) => Value::Number(number(n).error_at(span, "JSON number is outside the supported range")?),
        Json::Array(items) => {
            let data = items.iter().map(|v| import(v, fill, span)).collect::<Result<Vec<_>, _>>()?;
            imported(vec![data.len()], data, |i| items[i].is_null(), Value::Number(Number::from_integer(0))).error_at(span, "invalid JSON array")?
        }
        Json::Object(items) => {
            let keys = items.keys().map(|k| k.as_str().into()).collect();
            let data = items.values().map(|v| import(v, fill, span)).collect::<Result<_, _>>()?;
            keyed::record(keys, data).error_at(span, "invalid JSON object")?
        }
    };
    Ok(value)
}

fn export(value: &Value, fill: Option<&Number>, span: &Context<'_>) -> Result<Json, Error> {
    span.check()?;
    match value {
        Value::Number(n) => {
            if let Some(b) = n.as_bool() { return Ok(Json::Bool(b)); }
            if n.is_nan() || fill.is_some_and(|f| n.grade_order(f).is_eq()) { return Ok(Json::Null); }
            if let Some(n) = integer(n) { return Ok(n); }
            if let Some(n) = n.as_float().and_then(serde_json::Number::from_f64) { return Ok(Json::Number(n)); }
            Err(span.domain_error("JSON numbers must be finite floats or integers"))
        }
        Value::Function(_) | Value::Operator(_) => Err(span.domain_error("JSON cannot encode functions or operators")),
        _ if value.keys(0).is_none() && keyed::name(value).is_some() => Ok(Json::String(text(value, span)?)),
        _ if value.shape().is_empty() => export(&value.at(0), fill, span),
        _ => {
            let cells = value.cells(value.shape().len() - 1).error_at(span, "invalid JSON array")?;
            let values = (0..cells.len())
                .map(|i| { let cell = cells.get(i).error_at(span, "invalid JSON cell")?; export(&cell, fill, span) })
                .collect::<Result<Vec<_>, _>>()?;
            let Some(keys) = value.keys(0) else { return Ok(Json::Array(values)) };
            if !keys.complete() { return Err(span.domain_error("JSON objects need a key for every entry")); }
            Ok(Json::Object(keys.names().iter().flatten().zip(values).map(|(k, v)| (k.to_string(), v)).collect()))
        }
    }
}
