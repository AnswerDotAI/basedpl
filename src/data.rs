use crate::{execution::Context, keyed, Error, ErrorKind, Number, Value};
use std::{collections::HashMap, fs::OpenOptions, io::Write};

pub(crate) fn text(value: &Value, span: &Context<'_>) -> Result<String, Error> {
    keyed::name(value).map(|s| s.to_string()).ok_or_else(|| span.error(ErrorKind::Domain, "expected text"))
}

pub(crate) struct Options { pub values: HashMap<String, Value>, name: &'static str }

impl Options {
    pub fn new(name: &'static str, right: &Value, shorthand: Option<&str>, allowed: &[&str], span: &Context<'_>) -> Result<Self, Error> {
        let mut values = HashMap::new();
        if let Some(keys) = right.keys(0) {
            if right.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, format!("{name} options must be a keyed vector"))); }
            for (k, v) in keys.names().iter().zip(right.elements()) {
                let k = k.to_lowercase();
                if !allowed.contains(&k.as_str()) { return Err(span.error(ErrorKind::Domain, format!("unknown {name} option: {k}"))); }
                if values.insert(k.clone(), v).is_some() { return Err(span.error(ErrorKind::Domain, format!("duplicate {name} option: {k}"))); }
            }
        }
        else if let Some(key) = shorthand { values.insert(key.into(), right.clone()); }
        else if !text(right, span)?.is_empty() { return Err(span.error(ErrorKind::Domain, format!("{name} expects '' or keyed options"))); }
        Ok(Self { values, name })
    }

    pub fn text(&self, key: &str, span: &Context<'_>) -> Result<String, Error> {
        text(self.values.get(key).ok_or_else(|| span.error(ErrorKind::Domain, format!("{} needs {key}", self.name)))?, span)
    }

    pub fn boolean(&self, key: &str, default: bool, span: &Context<'_>) -> Result<bool, Error> {
        match self.values.get(key) {
            None => Ok(default),
            Some(Value::Number(n)) => n.boolean().map_err(|s| span.error(ErrorKind::Domain, s)),
            _ => Err(span.error(ErrorKind::Domain, format!("{} {key} must be Boolean", self.name))),
        }
    }

    pub fn fill(&self, span: &Context<'_>) -> Result<Number, Error> {
        match self.values.get("fill") {
            None => Ok(Number::try_from(f64::INFINITY).unwrap()),
            Some(Value::Number(n)) if n.as_complex().is_none() => Ok(n.clone()),
            _ => Err(span.error(ErrorKind::Domain, format!("{} fill must be a real number", self.name))),
        }
    }
}

pub(crate) fn vfi(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let input = text(right, span)?;
    let separators = left.map(|a| text(a, span)).transpose()?;
    let mut valid = Vec::new();
    let mut numbers = Vec::new();
    let zero = Number::try_from(0.0).unwrap();
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
        let valid = Value::integers(vec![valid.len()], valid)?;
        let numbers = Value::from_parts(vec![numbers.len()], numbers, Value::Number(zero))?;
        Value::new(vec![2], vec![valid, numbers])
    };
    result().map_err(|k| span.error(k, "invalid numeric input result"))
}

fn file_options(name: &'static str, right: &Value, allowed: &[&str], span: &Context<'_>) -> Result<(Options, String, bool), Error> {
    let opts = Options::new(name, right, Some("path"), allowed, span)?;
    let binary = opts.boolean("binary", false, span)?;
    if let Some(encoding) = opts.values.get("encoding") {
        if binary { return Err(span.error(ErrorKind::Domain, "binary files do not take an encoding")); }
        if !text(encoding, span)?.eq_ignore_ascii_case("UTF-8") { return Err(span.error(ErrorKind::Domain, "encoding must be UTF-8")); }
    }
    let path = opts.text("path", span)?;
    Ok((opts, path, binary))
}

pub(crate) fn read(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•nget is monadic")); }
    let (_, path, binary) = file_options("•nget", right, &["path", "encoding", "binary"], span)?;
    span.check()?;
    let data = std::fs::read(&path).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    span.check()?;
    if binary { return Value::integers(vec![data.len()], data.into_iter().map(i64::from).collect()).map_err(|k| span.error(k, "invalid byte vector")); }
    let data = String::from_utf8(data).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    Ok(keyed::text(&data))
}

pub(crate) fn write(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "•nput needs data on the left"))?;
    let (opts, path, binary) = file_options("•nput", right, &["path", "encoding", "binary", "overwrite"], span)?;
    let data = if binary {
        if left.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "binary output needs a vector")); }
        let invalid = || span.error(ErrorKind::Domain, "bytes must be integral numbers in 0..255");
        if left.prototype().as_number().is_none() { return Err(invalid()); }
        left.elements()
            .map(|v| {
                span.check()?;
                let n = v.as_number().and_then(|n| n.nonnegative_integer().ok()).and_then(|n| u8::try_from(n).ok());
                n.ok_or_else(invalid)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else { text(left, span)?.into_bytes() };
    let overwrite = opts.boolean("overwrite", false, span)?;
    span.check()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(&path)
        .map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    file.write_all(&data).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    Ok(Value::Number(Number::from_integer(data.len() as i64)))
}
