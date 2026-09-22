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

fn file_options(name: &'static str, right: &Value, allowed: &[&str], span: &Context<'_>) -> Result<(Options, String), Error> {
    let opts = Options::new(name, right, Some("path"), allowed, span)?;
    if let Some(encoding) = opts.values.get("encoding") {
        if !text(encoding, span)?.eq_ignore_ascii_case("UTF-8") { return Err(span.error(ErrorKind::Domain, "encoding must be UTF-8")); }
    }
    let path = opts.text("path", span)?;
    Ok((opts, path))
}

pub(crate) fn read(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•NGET is monadic")); }
    let (_, path) = file_options("•NGET", right, &["path", "encoding"], span)?;
    span.check()?;
    let data = std::fs::read_to_string(&path).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    span.check()?;
    Ok(keyed::text(&data))
}

pub(crate) fn write(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "•NPUT needs text on the left"))?;
    let data = text(left, span)?;
    let (opts, path) = file_options("•NPUT", right, &["path", "encoding", "overwrite"], span)?;
    let overwrite = opts.boolean("overwrite", false, span)?;
    span.check()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(&path)
        .map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    file.write_all(data.as_bytes()).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
    Ok(Value::Number(Number::from_integer(data.len() as i64)))
}
