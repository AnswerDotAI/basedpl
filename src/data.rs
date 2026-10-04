use crate::{
    array::{Ints, Items, Storage, Width},
    execution::Context,
    keyed,
    primitive::real,
    DomainAt, Error, ErrorAt, ErrorKind, Number, Value,
};
use foldhash::{HashMap, HashMapExt};
use std::{fs::OpenOptions, io::Write};

pub(crate) fn text(value: &Value, span: &Context<'_>) -> Result<String, Error> {
    keyed::name(value).map(|s| s.to_string()).ok_or_else(|| span.domain_error("expected text"))
}

/// Imported items, where `filled` marks the fills for missing values. When any other item is a finite float, numbers become floats if
/// every one converts exactly. Otherwise, or when one wouldn't, each item keeps its own kind. A fill or an infinity never makes exact
/// numbers approximate.
pub(crate) fn imported(shape: Vec<usize>, items: Vec<Value>, filled: impl Fn(usize) -> bool, empty_prototype: Value) -> Result<Value, ErrorKind> {
    if items.iter().enumerate().any(|(i, v)| !filled(i) && matches!(v, Value::Number(n) if n.as_float().is_some_and(f64::is_finite))) {
        if let Some(floats) = items.iter().map(|v| if let Value::Number(n) = v { n.lossless_float() } else { None }).collect::<Option<Vec<_>>>() {
            return Value::floats(shape, floats);
        }
    }
    Value::keeping_kinds(shape, items, empty_prototype)
}

pub(crate) struct Options { pub values: HashMap<String, Value>, name: &'static str }

impl Options {
    /// Options from a keyed vector, or from plain text given to `shorthand`. `None`, or `''` without a shorthand, gives no options.
    pub fn new(name: &'static str, value: Option<&Value>, shorthand: Option<&str>, allowed: &[&str], span: &Context<'_>) -> Result<Self, Error> {
        let mut values = HashMap::new();
        let Some(value) = value else { return Ok(Self { values, name }) };
        if let Some(keys) = value.keys(0) {
            if value.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, format!("{name} options must be a keyed vector"))); }
            for (k, v) in keys.names().iter().zip(value.elements()) {
                let k = k.as_ref().ok_or_else(|| span.domain_error(format!("{name} options need a key for every entry")))?.to_lowercase();
                if !allowed.contains(&k.as_str()) { return Err(span.domain_error(format!("unknown {name} option: {k}"))); }
                if values.insert(k.clone(), v).is_some() { return Err(span.domain_error(format!("duplicate {name} option: {k}"))); }
            }
        }
        else if let Some(key) = shorthand { values.insert(key.into(), value.clone()); }
        else if !text(value, span)?.is_empty() { return Err(span.domain_error(format!("{name} expects '' or keyed options"))); }
        Ok(Self { values, name })
    }

    pub fn text(&self, key: &str, default: Option<&str>, span: &Context<'_>) -> Result<String, Error> {
        match (self.values.get(key), default) {
            (Some(v), _) => text(v, span),
            (None, Some(d)) => Ok(d.into()),
            (None, None) => Err(span.domain_error(format!("{} needs {key}", self.name))),
        }
    }

    pub fn number(&self, key: &str, default: f64, span: &Context<'_>) -> Result<f64, Error> {
        self.values.get(key).map_or(Ok(default), |v| real(v, span).map_err(|_| span.domain_error(format!("{} {key} must be a real number", self.name))))
    }

    pub fn boolean(&self, key: &str, default: bool, span: &Context<'_>) -> Result<bool, Error> {
        match self.values.get(key) {
            None => Ok(default),
            Some(Value::Number(n)) => n.boolean().domain_at(span),
            _ => Err(span.domain_error(format!("{} {key} must be Boolean", self.name))),
        }
    }

    pub fn fill(&self, span: &Context<'_>) -> Result<Number, Error> {
        match self.values.get("fill") {
            None => Ok(Number::from(f64::NAN)),
            Some(Value::Number(n)) if n.as_complex().is_none() => Ok(n.clone()),
            _ => Err(span.domain_error(format!("{} fill must be a real number", self.name))),
        }
    }
}

pub(crate) fn vfi(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let input = text(right, span)?;
    let separators = left.map(|a| text(a, span)).transpose()?;
    let mut valid = Vec::new();
    let mut numbers = Vec::new();
    let zero = Number::from(0.0);
    let mut field = |s: &str| -> Result<(), Error> {
        span.check()?;
        let s = s.trim();
        let n = if s.is_empty() { Ok(zero.clone()) } else { Number::parse(s) };
        valid.push(i64::from(n.is_ok()));
        numbers.push(Value::Number(n.unwrap_or_else(|_| zero.clone())));
        Ok(())
    };
    if let Some(separators) = separators {
        if !input.is_empty() { for s in input.split(|c| separators.contains(c)) { field(s)?; } }
    }
    else { for s in input.split_whitespace() { field(s)?; } }
    let result = || {
        let numbers = imported(vec![numbers.len()], numbers, |i| valid[i] == 0, Value::Number(zero))?;
        Value::new(vec![2], vec![Value::integers(vec![valid.len()], valid)?, numbers])
    };
    result().error_at(span, "invalid numeric input result")
}

/// File options. `encoding` must be UTF-8.
fn file_options(name: &'static str, left: Option<&Value>, shorthand: Option<&str>, allowed: &[&str], span: &Context<'_>) -> Result<Options, Error> {
    let opts = Options::new(name, left, shorthand, allowed, span)?;
    if !opts.text("encoding", Some("UTF-8"), span)?.eq_ignore_ascii_case("UTF-8") { return Err(span.domain_error("encoding must be UTF-8")); }
    Ok(opts)
}

/// `data` as a vector of byte values, one byte each.
pub(crate) fn byte_vector(data: Vec<u8>) -> Result<Value, ErrorKind> { Value::from_storage(vec![data.len()], Storage::within(data, Width::U8)) }

/// The bytes in an array of integers from 0 to 255, in ravel order.
pub(crate) fn bytes(value: &Value, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    if let Items::Integers(Ints::U8(v)) = value.as_items() { return Ok(v.to_vec()); }
    let invalid = || span.domain_error("bytes must be integral numbers in 0..255");
    value.as_items().nonnegative_integers().map_err(|_| invalid())?.into_iter().map(|n| u8::try_from(n).map_err(|_| invalid())).collect()
}

pub(crate) fn read(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = file_options("•nget", left, None, &["encoding", "binary"], span)?;
    let binary = opts.boolean("binary", false, span)?;
    if binary && opts.values.contains_key("encoding") { return Err(span.domain_error("binary files do not take an encoding")); }
    let path = text(right, span)?;
    span.check()?;
    let data = if path == "-" { span.input(|input| input.rest())? } else { span.read(&path)? };
    span.check()?;
    if binary { return byte_vector(data).error_at(span, "invalid byte vector"); }
    let data = String::from_utf8(data).map_err(|e| span.file_error(&path, e))?;
    Ok(keyed::text(&data))
}

pub(crate) fn write(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = file_options("•nput", left, Some("path"), &["path", "encoding", "overwrite"], span)?;
    let path = opts.text("path", None, span)?;
    let data = if right.prototype().as_number().is_some() { bytes(right, span)? } else { text(right, span)?.into_bytes() };
    let overwrite = opts.boolean("overwrite", false, span)?;
    span.check()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(span.path(&path))
        .map_err(|e| span.file_error(&path, e))?;
    file.write_all(&data).map_err(|e| span.file_error(&path, e))?;
    Ok(Value::Number(Number::from_integer(data.len() as i64)))
}
