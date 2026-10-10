use crate::{
    array::Items,
    data::{imported, text, Options},
    execution::Context,
    keyed, Error, ErrorAt, ErrorKind, Number, Value,
};
use foldhash::{HashMap, HashMapExt};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::Value as Json;

/// A JSON number, exact when written without a point or exponent and otherwise a float of `width`. A float too large for `f64` is a DOMAIN error.
pub(crate) fn number(n: &serde_json::Number, width: crate::array::FloatWidth) -> Result<Number, ErrorKind> {
    if let Some(i) = n.as_i64() { return Ok(Number::from_integer(i)); }
    if n.as_str().contains(['.', 'e', 'E']) { return n.as_str().parse::<f64>().ok().filter(|x| x.is_finite()).map(|x| Number::float(x, width)).ok_or(ErrorKind::Domain); }
    exact(n.as_str().parse::<BigInt>().map_err(|_| ErrorKind::Domain)?)
}

/// An exact integer of any size.
fn exact(n: BigInt) -> Result<Number, ErrorKind> {
    match i64::try_from(&n) { Ok(i) => Ok(Number::from_integer(i)), Err(_) => Number::try_from(BigRational::from_integer(n)) }
}

/// An exact integer as a JSON number, whatever its size.
pub(crate) fn integer(n: &Number) -> Option<Json> {
    if let Some(i) = n.as_integer() { return Some(i.into()); }
    n.as_exact().filter(|q| q.is_integer()).map(|q| Json::Number(q.numer().to_string().parse().expect("decimal integer")))
}

/// `•json` reads JSON5, which includes all JSON. `fill` replaces `null`.
pub(crate) fn parse(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json", left, None, &["fill"], span)?;
    let read = json5::from_str(&text(right, span)?).map_err(|e| span.domain_error(format!("JSON {e}")))?;
    import(&read, &opts.fill(span)?, span)
}

/// A JSON5 value as the `json5` crate reads it. Unlike `serde_json::Value`, it holds NaN.
enum Read {
    Null,
    Bool(bool),
    Integer(BigInt),
    Float(f64),
    Text(String),
    List(Vec<Read>),
    Record(Vec<(String, Read)>),
}

impl<'de> serde::Deserialize<'de> for Read { fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> { d.deserialize_any(ReadVisitor) } }

struct ReadVisitor;

impl<'de> serde::de::Visitor<'de> for ReadVisitor {
    type Value = Read;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("a JSON5 value") }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Read, E> { Ok(Read::Null) }
    fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<Read, E> { Ok(Read::Bool(b)) }
    fn visit_i64<E: serde::de::Error>(self, n: i64) -> Result<Read, E> { Ok(Read::Integer(n.into())) }
    fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<Read, E> { Ok(Read::Integer(n.into())) }
    fn visit_i128<E: serde::de::Error>(self, n: i128) -> Result<Read, E> { Ok(Read::Integer(n.into())) }
    fn visit_u128<E: serde::de::Error>(self, n: u128) -> Result<Read, E> { Ok(Read::Integer(n.into())) }
    fn visit_f64<E: serde::de::Error>(self, x: f64) -> Result<Read, E> { Ok(Read::Float(x)) }
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Read, E> { Ok(Read::Text(s.into())) }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Read, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? { items.push(item); }
        Ok(Read::List(items))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Read, A::Error> {
        // A repeated key keeps its first place and takes its last value, as serde_json reads it.
        let (mut entries, mut places): (Vec<(String, Read)>, HashMap<String, usize>) = (Vec::new(), HashMap::new());
        while let Some((key, value)) = map.next_entry::<String, Read>()? {
            match places.get(&key) {
                Some(&i) => entries[i].1 = value,
                None => {
                    places.insert(key.clone(), entries.len());
                    entries.push((key, value));
                }
            }
        }
        Ok(Read::Record(entries))
    }
}

/// `•json⁻¹`: JSON text for a value. NaN exports as `null`, and so does the number given as `fill`.
pub(crate) fn serialize(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json⁻¹", left, None, &["fill"], span)?;
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

fn import(value: &Read, fill: &Number, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    let value = match value {
        Read::Null => Value::Number(fill.clone()),
        Read::Bool(b) => Value::Number(Number::from_bool(*b)),
        Read::Text(s) => keyed::text(s),
        Read::Integer(n) => Value::Number(exact(n.clone()).error_at(span, "JSON number is outside the supported range")?),
        Read::Float(x) => Value::Number(Number::float(*x, span.numeric().width)),
        Read::List(items) => {
            let data = items.iter().map(|v| import(v, fill, span)).collect::<Result<Vec<_>, _>>()?;
            imported(vec![data.len()], data, |i| matches!(items[i], Read::Null), Value::Number(Number::from_integer(0))).error_at(span, "invalid JSON array")?
        }
        Read::Record(entries) => {
            let keys = entries.iter().map(|(k, _)| k.as_str().into()).collect();
            let data = entries.iter().map(|(_, v)| import(v, fill, span)).collect::<Result<_, _>>()?;
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
