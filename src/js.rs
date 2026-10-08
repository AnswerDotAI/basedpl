//! How values cross between BPL and JavaScript. `export` turns a value into a `Js` tree, and `import` turns a tree into a value. The
//! browser build converts trees to and from JavaScript values. `•canvas` writes one as JSON, with its typed arrays as bytes.
//!
//! - A number is a JavaScript number, and a Boolean is `true` or `false`. A string is a JavaScript string, and a record is an object.
//! - Any other array is `{shape, data}`. `data` is a typed array when every item is a number: `Float64Array` for approximate and
//!   fractional numbers, and for integers the width BPL stores them at, `Uint8Array`, `Int16Array`, `Int32Array` or `BigInt64Array`,
//!   as Python's `.np` does. Complex numbers are their real and imaginary parts in turn, with a last axis of length 2. Otherwise `data`
//!   is an `Array` of items. Axis names and keys, when an array has them, are `axis_names` and `axis_keys`, as in `protocol.rs`.
//! - A JavaScript number becomes an approximate number, a `BigInt` an exact one, an `Array` a vector and a function a BPL function.
//!   An object with `shape` and `data` becomes an array, and any other object a record. `undefined` and `null` give no value.
use crate::{
    array::{Ints, Items},
    element::read_as,
    keyed, ErrorKind, Number, Value,
};
use serde_json::{json, Value as Json};

/// A JavaScript value.
#[derive(Debug)]
pub enum Js {
    Undefined,
    Bool(bool),
    Number(f64),
    BigInt(i64),
    Text(String),
    List(Vec<Js>),
    Object(Vec<(String, Js)>),
    Typed(Typed),
    #[cfg(web)]
    Function(std::sync::Arc<JsFunction>),
}

/// A typed array.
#[derive(Debug)]
pub enum Typed {
    Float64(Vec<f64>),
    Uint8(Vec<u8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    BigInt64(Vec<i64>),
}

impl Typed {
    /// The name of the array's JavaScript constructor.
    fn name(&self) -> &'static str {
        match self {
            Self::Float64(_) => "Float64Array",
            Self::Uint8(_) => "Uint8Array",
            Self::Int16(_) => "Int16Array",
            Self::Int32(_) => "Int32Array",
            Self::BigInt64(_) => "BigInt64Array",
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Float64(v) => v.len(),
            Self::Uint8(v) => v.len(),
            Self::Int16(v) => v.len(),
            Self::Int32(v) => v.len(),
            Self::BigInt64(v) => v.len(),
        }
    }
    /// The items' bytes, little-endian, as JavaScript's typed arrays hold them on every current platform.
    fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Float64(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            Self::Uint8(v) => v.clone(),
            Self::Int16(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            Self::Int32(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            Self::BigInt64(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
        }
    }
}

/// `value` as a JavaScript value. A function or operator, even inside an array, has none.
pub fn export(value: &Value) -> Result<Js, ErrorKind> {
    match value {
        Value::Number(n) => number(n),
        Value::Character(c) => Ok(Js::Text(c.to_string())),
        Value::Function(_) | Value::Operator(_) => Err(ErrorKind::Domain),
        Value::Array(_) => {
            if value.keys(0).is_none() { if let Some(text) = keyed::name(value) { return Ok(Js::Text(text.to_string())); } }
            if value.shape().len() == 1 && value.keys(0).is_some_and(|k| k.complete()) {
                return keyed::pairs(value)?.into_iter().map(|(k, v)| Ok((k.to_string(), export(&v)?))).collect::<Result<_, _>>().map(Js::Object);
            }
            array(value)
        }
    }
}

fn number(n: &Number) -> Result<Js, ErrorKind> {
    if let Some(b) = n.as_bool() { return Ok(Js::Bool(b)); }
    if let Some(z) = n.as_complex() { return Ok(shaped(vec![2], Js::Typed(Typed::Float64(vec![z.re, z.im])), Vec::new(), Vec::new())); }
    n.to_float().map(Js::Number).map_err(|_| ErrorKind::Domain)
}

/// An array as `{shape, data}`, with `axis_names` and `axis_keys` when it has them.
fn array(value: &Value) -> Result<Js, ErrorKind> {
    let mut shape = value.shape().to_vec();
    let mut names: Vec<Js> = value.axis_names().iter().map(|n| n.as_ref().map_or(Js::Undefined, |n| Js::Text(n.to_string()))).collect();
    let mut keys: Vec<Js> = if value.has_keys() {
        keyed::key_lists(value)
            .into_iter()
            .map(|k| k.map_or(Js::Undefined, |k| Js::List(k.into_iter().map(|k| k.map_or(Js::Undefined, |k| Js::Text(k.into()))).collect())))
            .collect()
    } else { Vec::new() };
    let complex = |shape: &mut Vec<usize>, names: &mut Vec<Js>, keys: &mut Vec<Js>, parts: Vec<f64>| {
        shape.push(2);
        if !names.is_empty() { names.push(Js::Undefined); }
        if !keys.is_empty() { keys.push(Js::Undefined); }
        Js::Typed(Typed::Float64(parts))
    };
    // Reading as a kernel does clears a flag on integer storage that holds no infinity. `Extended` then holds infinities.
    let data = match value.checked_items() {
        Items::Booleans(b) => Js::Typed(Typed::Uint8(b.iter().map(|&b| b.into()).collect())),
        Items::Integers(Ints::U8(v)) => Js::Typed(Typed::Uint8(v.to_vec())),
        Items::Integers(Ints::I16(v)) => Js::Typed(Typed::Int16(v.to_vec())),
        Items::Integers(Ints::I32(v)) => Js::Typed(Typed::Int32(v.to_vec())),
        Items::Integers(Ints::I64(v)) => Js::Typed(Typed::BigInt64(v.to_vec())),
        Items::Extended(_) | Items::Floats(_) => Js::Typed(Typed::Float64(read_as::<f64>(value).expect("real storage").into_owned())),
        Items::Complex(v) => complex(&mut shape, &mut names, &mut keys, v.iter().flat_map(|z| [z.re, z.im]).collect()),
        Items::Characters(c) => Js::List(c.iter().map(|c| Js::Text(c.to_string())).collect()),
        Items::Values(items) => match items.iter().map(|v| if let Value::Number(n) = v { Some(n) } else { None }).collect::<Option<Vec<_>>>() {
            Some(numbers) if !numbers.is_empty() || matches!(value.prototype(), Value::Number(_)) => {
                if numbers.iter().any(|n| n.as_complex().is_some()) {
                    let parts = numbers.iter().map(|n| n.to_complex().map(|z| [z.re, z.im])).collect::<Result<Vec<_>, _>>().map_err(|_| ErrorKind::Domain)?;
                    complex(&mut shape, &mut names, &mut keys, parts.concat())
                } else { Js::Typed(Typed::Float64(numbers.iter().map(|n| n.to_float()).collect::<Result<_, _>>().map_err(|_| ErrorKind::Domain)?)) }
            }
            _ => Js::List(items.iter().map(export).collect::<Result<_, _>>()?),
        },
    };
    Ok(shaped(shape, data, names, keys))
}

fn shaped(shape: Vec<usize>, data: Js, names: Vec<Js>, keys: Vec<Js>) -> Js {
    let mut fields = vec![("shape".into(), Js::List(shape.into_iter().map(|n| Js::Number(n as f64)).collect())), ("data".into(), data)];
    if !names.is_empty() { fields.push(("axis_names".into(), Js::List(names))); }
    if !keys.is_empty() { fields.push(("axis_keys".into(), Js::List(keys))); }
    Js::Object(fields)
}

/// The value that `js` describes, or `None` for `undefined` and `null`.
pub fn import(js: Js) -> Result<Option<Value>, ErrorKind> {
    Ok(Some(match js {
        Js::Undefined => return Ok(None),
        Js::Bool(b) => Value::Number(Number::from_bool(b)),
        Js::Number(x) => Value::Number(x.into()),
        Js::BigInt(n) => Value::Number(Number::from_integer(n)),
        Js::Text(s) => keyed::text(&s),
        Js::Typed(t) => typed(vec![t.len()], t)?,
        Js::List(items) => {
            let items = items.into_iter().map(item).collect::<Result<Vec<_>, _>>()?;
            Value::from_parts(vec![items.len()], items, Value::Number(0.0.into()))?
        }
        Js::Object(fields) if is_array(&fields) => shaped_array(fields)?,
        Js::Object(fields) => {
            let (keys, values): (Vec<_>, Vec<_>) =
                fields.into_iter().map(|(k, v)| Ok((k.into(), item(v)?))).collect::<Result<Vec<_>, ErrorKind>>()?.into_iter().unzip();
            keyed::record(keys, values)?
        }
        #[cfg(web)]
        Js::Function(f) => web::value(f),
    }))
}

/// An item of an array or record, which must have a value.
fn item(js: Js) -> Result<Value, ErrorKind> { import(js)?.ok_or(ErrorKind::Domain) }

fn typed(shape: Vec<usize>, t: Typed) -> Result<Value, ErrorKind> {
    match t {
        Typed::Float64(v) => Value::floats(shape, v),
        Typed::Uint8(v) => Value::integers(shape, v.into_iter().map(i64::from).collect()),
        Typed::Int16(v) => Value::integers(shape, v.into_iter().map(i64::from).collect()),
        Typed::Int32(v) => Value::integers(shape, v.into_iter().map(i64::from).collect()),
        Typed::BigInt64(v) => Value::integers(shape, v),
    }
}

const ARRAY_FIELDS: [&str; 4] = ["shape", "data", "axis_names", "axis_keys"];

/// Whether an object's fields describe an array: `shape` and `data`, and perhaps `axis_names` and `axis_keys`.
fn is_array(fields: &[(String, Js)]) -> bool {
    ["shape", "data"].iter().all(|f| fields.iter().any(|(k, _)| k == f)) && fields.iter().all(|(k, _)| ARRAY_FIELDS.contains(&k.as_str()))
}

fn shaped_array(fields: Vec<(String, Js)>) -> Result<Value, ErrorKind> {
    let [mut shape, mut data, mut names, mut keys] = [None, None, None, None];
    for (k, v) in fields {
        let slot = match k.as_str() {
            "shape" => &mut shape,
            "data" => &mut data,
            "axis_names" => &mut names,
            _ => &mut keys,
        };
        *slot = Some(v);
    }
    let Some(Js::List(shape)) = shape else { return Err(ErrorKind::Domain) };
    let shape = shape
        .into_iter()
        .map(|n| match n { Js::Number(n) if n >= 0. && n.fract() == 0. => Ok(n as usize), _ => Err(ErrorKind::Domain) })
        .collect::<Result<Vec<_>, _>>()?;
    let value = match data.expect("is_array checked for data") {
        Js::Typed(t) => typed(shape, t)?,
        Js::List(items) => Value::from_parts(shape, items.into_iter().map(item).collect::<Result<_, _>>()?, Value::Number(0.0.into()))?,
        _ => return Err(ErrorKind::Domain),
    };
    let texts = |list: Js| match list {
        Js::List(items) => items
            .into_iter()
            .map(|n| match n { Js::Undefined => Ok(None), Js::Text(t) => Ok(Some(t)), _ => Err(ErrorKind::Domain) })
            .collect::<Result<Vec<_>, _>>(),
        _ => Err(ErrorKind::Domain),
    };
    let value = match names { Some(names) => keyed::with_names(value, texts(names)?).map_err(|_| ErrorKind::Domain)?, None => value };
    match keys {
        Some(Js::List(lists)) => {
            let lists = lists.into_iter().map(|k| if let Js::Undefined = k { Ok(None) } else { texts(k).map(Some) }).collect::<Result<_, _>>()?;
            keyed::with_key_lists(value, lists).map_err(|_| ErrorKind::Domain)
        }
        Some(_) => Err(ErrorKind::Domain),
        None => Ok(value),
    }
}

/// `js` as JSON, with each typed array replaced by `{"$typed": constructor, "offset": …, "length": …}`, and the bytes those entries
/// refer to. Each array starts at a multiple of 8 bytes, so a typed array can view it in place.
pub(crate) fn serialize(js: &Js) -> (Json, Vec<u8>) {
    fn write(js: &Js, bytes: &mut Vec<u8>) -> Json {
        match js {
            Js::Undefined => Json::Null,
            #[cfg(web)]
            Js::Function(_) => Json::Null,
            Js::Bool(b) => json!(b),
            Js::Number(x) => json!(x),
            Js::BigInt(n) => json!(n),
            Js::Text(s) => json!(s),
            Js::List(items) => Json::Array(items.iter().map(|v| write(v, bytes)).collect()),
            Js::Object(fields) => Json::Object(fields.iter().map(|(k, v)| (k.clone(), write(v, bytes))).collect()),
            Js::Typed(t) => {
                bytes.resize(bytes.len().next_multiple_of(8), 0);
                let offset = bytes.len();
                bytes.extend(t.bytes());
                json!({"$typed": t.name(), "offset": offset, "length": t.len()})
            }
        }
    }
    let mut bytes = Vec::new();
    (write(js, &mut bytes), bytes)
}

#[cfg(web)]
pub(crate) use web::{call, function, message, prototype};
#[cfg(web)]
pub use web::{from_js, to_js, JsFunction};

/// The browser build's side: JavaScript values, and the JavaScript functions that BPL calls.
#[cfg(web)]
mod web {
    use super::{export, import, Js, Typed};
    use crate::{execution::Context, Error, Value};
    use js_sys::{
        Array, BigInt64Array, Float32Array, Float64Array, Function, Int16Array, Int32Array, Int8Array, Object, Reflect, Uint16Array, Uint32Array, Uint8Array,
        Uint8ClampedArray,
    };
    use std::{cell::RefCell, sync::Arc};
    use wasm_bindgen::{JsCast, JsValue};

    thread_local! {
        /// The JavaScript functions that BPL functions call, by slot. A BPL value must be `Send + Sync` and a JavaScript function isn't,
        /// so a BPL function holds its slot's number.
        static FUNCTIONS: RefCell<Vec<Option<Function>>> = const { RefCell::new(Vec::new()) };
    }

    /// A JavaScript function that a BPL function calls. Its slot is freed when the last BPL function that holds it is dropped.
    #[derive(Debug)]
    pub struct JsFunction(usize);
    impl JsFunction {
        fn new(f: Function) -> Self {
            FUNCTIONS.with_borrow_mut(|slots| {
                let slot = slots.iter().position(Option::is_none).unwrap_or_else(|| { slots.push(None); slots.len() - 1 });
                slots[slot] = Some(f);
                Self(slot)
            })
        }
        fn get(&self) -> Function { FUNCTIONS.with_borrow(|slots| slots[self.0].clone().expect("a held slot has its function")) }
    }
    impl Drop for JsFunction { fn drop(&mut self) { FUNCTIONS.with_borrow_mut(|slots| slots[self.0] = None) } }

    /// The BPL function that calls the JavaScript function `f`.
    pub(super) fn value(f: Arc<JsFunction>) -> Value {
        Value::Function(crate::system::native("•js", crate::system::Call::Js(f), crate::system::Valence::Ambivalent))
    }

    /// `js` as a JavaScript value.
    pub fn to_js(js: Js) -> JsValue {
        match js {
            Js::Undefined => JsValue::UNDEFINED,
            Js::Bool(b) => b.into(),
            Js::Number(x) => x.into(),
            Js::BigInt(n) => js_sys::BigInt::from(n).into(),
            Js::Text(s) => s.into(),
            Js::List(items) => items.into_iter().map(to_js).collect::<Array>().into(),
            Js::Object(fields) => {
                let object = Object::new();
                for (k, v) in fields { _ = Reflect::set(&object, &k.into(), &to_js(v)); }
                object.into()
            }
            Js::Typed(Typed::Float64(v)) => Float64Array::from(&v[..]).into(),
            Js::Typed(Typed::Uint8(v)) => Uint8Array::from(&v[..]).into(),
            Js::Typed(Typed::Int16(v)) => Int16Array::from(&v[..]).into(),
            Js::Typed(Typed::Int32(v)) => Int32Array::from(&v[..]).into(),
            Js::Typed(Typed::BigInt64(v)) => BigInt64Array::from(&v[..]).into(),
            Js::Function(f) => f.get().into(),
        }
    }

    /// `value` as a `Js` tree. A typed array of another type widens to the nearest one that `Typed` has. A symbol has no tree.
    pub fn from_js(value: &JsValue) -> Result<Js, String> {
        let typed = |t| Ok(Js::Typed(t));
        if value.is_undefined() || value.is_null() { return Ok(Js::Undefined); }
        if let Some(b) = value.as_bool() { return Ok(Js::Bool(b)); }
        if let Some(x) = value.as_f64() { return Ok(Js::Number(x)); }
        if let Some(s) = value.as_string() { return Ok(Js::Text(s)); }
        if value.is_bigint() { return i64::try_from(value.clone()).map(Js::BigInt).map_err(|_| "a BigInt must fit in 64 bits".into()); }
        if let Some(f) = value.dyn_ref::<Function>() { return Ok(Js::Function(Arc::new(JsFunction::new(f.clone())))); }
        if Array::is_array(value) { return Array::from(value).iter().map(|v| from_js(&v)).collect::<Result<_, _>>().map(Js::List); }
        if let Some(a) = value.dyn_ref::<Float64Array>() { return typed(Typed::Float64(a.to_vec())); }
        if let Some(a) = value.dyn_ref::<Float32Array>() { return typed(Typed::Float64(a.to_vec().into_iter().map(f64::from).collect())); }
        if let Some(a) = value.dyn_ref::<Uint8Array>() { return typed(Typed::Uint8(a.to_vec())); }
        if let Some(a) = value.dyn_ref::<Uint8ClampedArray>() { return typed(Typed::Uint8(a.to_vec())); }
        if let Some(a) = value.dyn_ref::<Int8Array>() { return typed(Typed::Int16(a.to_vec().into_iter().map(i16::from).collect())); }
        if let Some(a) = value.dyn_ref::<Int16Array>() { return typed(Typed::Int16(a.to_vec())); }
        if let Some(a) = value.dyn_ref::<Uint16Array>() { return typed(Typed::Int32(a.to_vec().into_iter().map(i32::from).collect())); }
        if let Some(a) = value.dyn_ref::<Int32Array>() { return typed(Typed::Int32(a.to_vec())); }
        if let Some(a) = value.dyn_ref::<Uint32Array>() { return typed(Typed::BigInt64(a.to_vec().into_iter().map(i64::from).collect())); }
        if let Some(a) = value.dyn_ref::<BigInt64Array>() { return typed(Typed::BigInt64(a.to_vec())); }
        if value.is_object() {
            return Object::entries(value.unchecked_ref())
                .iter()
                .map(|entry| { let entry: Array = entry.unchecked_into(); Ok((entry.get(0).as_string().unwrap_or_default(), from_js(&entry.get(1))?)) })
                .collect::<Result<_, _>>()
                .map(Js::Object);
        }
        Err("this JavaScript value has no BPL form".into())
    }

    /// The message of the JavaScript exception `e`.
    pub(crate) fn message(e: &JsValue) -> String {
        e.dyn_ref::<js_sys::Error>().map_or_else(|| e.as_string().unwrap_or_else(|| "JavaScript exception".into()), |e| e.message().into())
    }

    /// `•js Y`: the function that the JavaScript source `Y` evaluates to.
    pub(crate) fn function(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let source = crate::data::text(right, span)?;
        let f = js_sys::eval(&source).map_err(|e| span.domain_error(message(&e)))?;
        let f = f.dyn_into::<Function>().map_err(|_| span.domain_error("•js source must give a function"))?;
        Ok(value(Arc::new(JsFunction::new(f))))
    }

    /// `•js`'s result prototype: a function that does nothing.
    pub(crate) fn prototype() -> Value { value(Arc::new(JsFunction::new(Function::new_no_args("")))) }

    /// Calls `f` with `right`, or with `left` then `right`. `None` when it returns `undefined` or `null`.
    pub(crate) fn call(f: &JsFunction, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Option<Value>, Error> {
        let arg = |v: &Value| export(v).map(to_js).map_err(|_| cx.domain_error("a function or operator can't cross into JavaScript"));
        let f = f.get();
        let result = match left { Some(left) => f.call2(&JsValue::UNDEFINED, &arg(left)?, &arg(right)?), None => f.call1(&JsValue::UNDEFINED, &arg(right)?) };
        let result = from_js(&result.map_err(|e| cx.domain_error(message(&e)))?).map_err(|e| cx.domain_error(e))?;
        import(result).map_err(|_| cx.domain_error("the JavaScript result has no BPL form"))
    }
}
