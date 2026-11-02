use crate::execution::check;
use crate::{Described, Error, ErrorKind, Number, Value, system::HostCall, data::{Options, imported, text}, keyed::{self, Form}};
use num_bigint::BigInt;
use rustymath::error::Kind;
use rustymath::hashing::{HashMap, HashMapExt};
use serde_json::Value as Json;

/// A JSON number, exact when written without a point or exponent and otherwise a float of `width`. A float too large for `f64` is a DOMAIN error.
pub(crate) fn number(n: &serde_json::Number, width: rustymath::buffer::FloatWidth) -> Result<Number, ErrorKind> {
    if let Some(i) = n.as_i64() { return Ok(Number::from_integer(i)); }
    if n.as_str().contains(['.', 'e', 'E']) {
        return n.as_str().parse::<f64>().ok().filter(|x| x.is_finite()).map(|x| Number::float(x, width)).ok_or_else(|| Kind::Domain.into());
    }
    Ok(Number::from(n.as_str().parse::<BigInt>().map_err(|_| Kind::Domain)?))
}

/// An exact integer as a JSON number, whatever its size.
pub(crate) fn integer(n: &Number) -> Option<Json> {
    if let Some(i) = n.as_integer() { return Some(i.into()); }
    n.integer_text().map(|t| Json::Number(t.parse().expect("decimal integer")))
}

/// `•json` reads JSON5, which includes all JSON. `fill` replaces `null`, and `types` gives the arrays that fields hold their types.
pub(crate) fn parse(HostCall { left, right, numeric, .. }: HostCall<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json", left, None, &["fill", "types"])?;
    let read = json5::from_str(&text(right)?).map_err(|e| Error::domain(format!("JSON {e}")))?;
    let types = match opts.types()? {
        crate::data::Types::Named(types) if types.values().all(|t| matches!(t, Value::Number(_))) => types,
        _ => return Err(Error::domain("•json types must be a record from field names to numbers")),
    };
    let mut reading = Reading { numeric, fill: opts.fill(numeric)?, texts: keyed::Texts::default(), types, matched: Vec::new() };
    let value = import(&read, "", &mut reading)?;
    if let Some(k) = reading.types.keys().find(|k| !reading.matched.contains(k)) {
        return Err(Error::domain(format!("•json types names no field {k:?} that holds an array")));
    }
    Ok(value)
}

/// A JSON5 value as the `json5` crate reads it. Unlike `serde_json::Value`, it holds NaN.
enum Read { Null, Bool(bool), Integer(BigInt), Float(f64), Text(String), List(Vec<Read>), Record(Vec<(String, Read)>) }

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
pub(crate) fn serialize(HostCall { left, right, numeric, .. }: HostCall<'_>) -> Result<Value, Error> {
    let opts = Options::new("•json⁻¹", left, None, &["fill"])?;
    let fill = opts.fill(numeric)?;
    let value = exportable(right).read(crate::ReadKind::Value)?;
    crate::session().raise()?;
    Ok(keyed::text(&export(&value, opts.values.contains_key("fill").then_some(&fill))?.to_string()))
}

/// Drops function-valued keyed entries without reading data.
pub(crate) fn exportable(value: &Value) -> Value {
    let Value::Array(array) = value else { return value.clone() };
    let crate::array::ArrayData::Mixed { items, .. } = &**array else { return value.clone() };
    let keys = value.keys(0).filter(|_| value.rank() == 1);
    let (mut names, mut data, mut changed) = (Vec::new(), Vec::new(), false);
    for (i, item) in items.iter().enumerate() {
        if keys.is_some() && matches!(item, Value::Function(_) | Value::Operator(_)) { changed = true; continue; }
        names.extend(keys.map(|keys| keys.names()[i].clone()));
        let kept = exportable(item);
        changed |= !kept.identical(item);
        data.push(kept);
    }
    if !changed { return value.clone(); }
    if keys.is_some() { keyed::partial_vector(names, data) } else { value.layout().collect(data, || value.prototype()) }.expect("subset of a valid array")
}

/// What `import` reads with: the number that replaces `null`, the texts read so far, the `types` option, and the names in `types` that an array has matched.
struct Reading { numeric: crate::number::Numeric, fill: Number, texts: keyed::Texts, types: HashMap<String, Value>, matched: Vec<String> }

/// `value` as BPL reads it. An array is a column named `name`, the key of the field that holds it, or `""` where no field does (`data::typed_column`).
fn import(value: &Read, name: &str, reading: &mut Reading) -> Result<Value, Error> {
    check()?;
    let value = match value {
        Read::Null => Value::Number(reading.fill.clone()),
        Read::Bool(b) => Value::Number(Number::from_bool(*b)),
        Read::Text(s) => reading.texts.text(s),
        Read::Integer(n) => Value::Number(Number::from(n.clone())),
        Read::Float(x) => Value::Number(Number::float(*x, reading.numeric.width)),
        Read::List(items) => {
            let texts: Option<Vec<&str>> = items.iter().map(|v| match v { Read::Text(s) => Some(s.as_str()), _ => None }).collect();
            let column = match texts {
                Some(texts) if !texts.is_empty() => keyed::text_column(&[texts.len()], &texts),
                _ => {
                    let mut data = Vec::with_capacity(items.len());
                    for v in items { data.push(import(v, "", reading)?); }
                    imported(vec![data.len()], data, |i| matches!(items[i], Read::Null), Value::integer(0))
                }
            }.described("invalid JSON array")?;
            let typed = reading.types.get(name).cloned();
            if typed.is_some() { reading.matched.push(name.to_string()); }
            crate::data::typed_column("•json", name, column, typed.as_ref(), items.iter().any(|v| matches!(v, Read::Null)), reading.numeric)?
        }
        Read::Record(entries) => {
            let keys = entries.iter().map(|(k, _)| k.as_str().into()).collect();
            let data = entries.iter().map(|(k, v)| import(v, k, reading)).collect::<Result<_, _>>()?;
            keyed::vector(keys, data).described("invalid JSON object")?
        }
    };
    Ok(value)
}

fn export(value: &Value, fill: Option<&Number>) -> Result<Json, Error> {
    check()?;
    match value {
        Value::Number(n) => {
            if let Some(b) = n.as_bool() { return Ok(Json::Bool(b)); }
            if n.is_nan() || fill.is_some_and(|fill| n.grade_order(fill).is_eq()) { return Ok(Json::Null); }
            if let Some(n) = integer(n) { return Ok(n); }
            if let Some(n) = n.as_float().and_then(serde_json::Number::from_f64) { return Ok(Json::Number(n)); }
            return Err(Error::domain("JSON numbers must be finite floats or integers"));
        }
        Value::Character(c) => return Ok(Json::String(c.to_string())),
        Value::Function(_) | Value::Operator(_) => return Err(Error::domain("JSON cannot encode functions or operators")),
        Value::Array(_) => (),
        Value::Pending(_) | Value::Carried { .. } => unreachable!("JSON exports observed values"),
    }
    let keys = match keyed::form(value) {
        Form::Text(text) => return Ok(Json::String(text.to_string())),
        Form::Array if value.is_unit() => return export(&value.at(0), fill),
        Form::Array => None,
        Form::Object(keys) => Some(keys),
    };
    let cells = value.cells(value.rank() - 1).described("invalid JSON array")?;
    let values = (0..cells.len()).map(|i| export(&cells.get(i).described("invalid JSON cell")?, fill)).collect::<Result<Vec<_>, _>>()?;
    let Some(keys) = keys else { return Ok(Json::Array(values)) };
    if !keys.complete() { return Err(Error::domain("JSON objects need a key for every entry")); }
    Ok(Json::Object(keys.names().iter().flatten().zip(values).map(|(key, value)| (key.to_string(), value)).collect()))
}
