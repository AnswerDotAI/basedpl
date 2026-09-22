use crate::{
    data::{text, Options},
    execution::Context,
    keyed, Error, ErrorKind, Number, Value,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::Value as Json;

pub(crate) fn call(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let allowed = if left.is_some() { &["fill"][..] } else { &["source", "fill"][..] };
    let opts = Options::new("•json", right, left.is_none().then_some("source"), allowed, span)?;
    let fill = opts.fill(span)?;
    match left {
        None => {
            let source = opts.text("source", span)?;
            let json = serde_json::from_str(&source).map_err(|e| span.error(ErrorKind::Domain, format!("JSON {e}")))?;
            import(&json, &fill, span)
        }
        Some(value) => {
            let json = export(value, opts.values.contains_key("fill").then_some(&fill), span)?;
            Ok(keyed::text(&json.to_string()))
        }
    }
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
