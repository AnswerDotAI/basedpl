use crate::{
    array::{Ints, Items, Storage, Width},
    execution::Context,
    keyed,
    primitive::{numeric, real},
    DomainAt, Error, ErrorAt, ErrorKind, Number, Value,
};
use foldhash::{HashMap, HashMapExt};
use std::io::Read;

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

/// The bytes in an array of integers from 0 to 255, in ravel order. `None` for any other array.
pub(crate) fn byte_items(value: &Value) -> Option<Vec<u8>> {
    if let Items::Integers(Ints::U8(v)) = value.as_items() { return Some(v.to_vec()); }
    value.as_items().nonnegative_integers().ok()
}

/// The bytes in an array of integers from 0 to 255, in ravel order.
pub(crate) fn bytes(value: &Value, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    byte_items(value).ok_or_else(|| span.domain_error("bytes must be integral numbers in 0..255"))
}

/// `value` as bytes, taking integers from 0 to 255 as they are and text as UTF-8.
pub(crate) fn payload(value: &Value, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    if value.prototype().as_number().is_some() { bytes(value, span) } else { Ok(text(value, span)?.into_bytes()) }
}

/// The algorithm or format that the optional left argument names, in lower case, or `default` without one.
pub(crate) fn algorithm(left: Option<&Value>, default: &str, span: &Context<'_>) -> Result<String, Error> {
    left.map_or(Ok(default.into()), |name| text(name, span).map(|s| s.to_lowercase()))
}

/// `•deflate` decompresses bytes in the format `"gzip"`, `"zlib"` or `"deflate"`, gzip by default. `•deflate⁻¹` compresses bytes or
/// text. Both give bytes.
fn flate(left: Option<&Value>, right: &Value, span: &Context<'_>, compress: bool) -> Result<Value, Error> {
    use flate2::{read, Compression};
    let (data, level) = (payload(right, span)?, Compression::default());
    let mut reader: Box<dyn Read + '_> = match (algorithm(left, "gzip", span)?.as_str(), compress) {
        ("gzip", true) => Box::new(read::GzEncoder::new(&data[..], level)),
        ("gzip", false) => Box::new(read::GzDecoder::new(&data[..])),
        ("zlib", true) => Box::new(read::ZlibEncoder::new(&data[..], level)),
        ("zlib", false) => Box::new(read::ZlibDecoder::new(&data[..])),
        ("deflate", true) => Box::new(read::DeflateEncoder::new(&data[..], level)),
        ("deflate", false) => Box::new(read::DeflateDecoder::new(&data[..])),
        _ => return Err(span.domain_error("•deflate takes \"gzip\", \"zlib\" or \"deflate\"")),
    };
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(|e| span.domain_error(format!("invalid compressed data: {e}")))?;
    byte_vector(out).error_at(span, "invalid byte vector")
}
pub(crate) fn inflate(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> { flate(left, right, span, false) }
pub(crate) fn deflate(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> { flate(left, right, span, true) }

/// `•hash` gives the SHA-2 digest of bytes or text in hex, by `"sha224"`, `"sha256"`, `"sha384"` or `"sha512"`, SHA-256 by default.
pub(crate) fn hash(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
    let data = payload(right, span)?;
    let digest = match algorithm(left, "sha256", span)?.as_str() {
        "sha224" => Sha224::digest(&data).to_vec(),
        "sha256" => Sha256::digest(&data).to_vec(),
        "sha384" => Sha384::digest(&data).to_vec(),
        "sha512" => Sha512::digest(&data).to_vec(),
        _ => return Err(span.domain_error("•hash takes \"sha224\", \"sha256\", \"sha384\" or \"sha512\"")),
    };
    Ok(keyed::text(&digest.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

/// `•uuid v` gives a new UUID of version `v` as text. Versions 1, 4, 6 and 7 take no left argument. Versions 3 and 5 take a namespace and
/// a name on the left, and version 8 takes 16 bytes.
pub(crate) fn uuid(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use uuid::Uuid;
    // The standard allows a random node in place of a MAC address, with the multicast bit set.
    let node = || {
        let mut node: [u8; 6] = rand::random();
        node[0] |= 1;
        node
    };
    let version = numeric(right, span)?.integer().error_at(span, "•uuid needs a version number")?;
    let id = match (version, left) {
        (0, None) => Uuid::nil(),
        (1, None) => Uuid::now_v1(&node()),
        (4, None) => Uuid::new_v4(),
        (6, None) => Uuid::now_v6(&node()),
        (7, None) => Uuid::now_v7(),
        (3 | 5, Some(names)) => {
            let names: Vec<_> = names.elements().map(|v| text(&v, span)).collect::<Result<_, _>>()?;
            let [space, name] = names.as_slice() else { return Err(span.domain_error("UUID versions 3 and 5 take a namespace and a name")) };
            let space = match space.to_lowercase().as_str() {
                "dns" => Uuid::NAMESPACE_DNS,
                "url" => Uuid::NAMESPACE_URL,
                "oid" => Uuid::NAMESPACE_OID,
                "x500" => Uuid::NAMESPACE_X500,
                _ => Uuid::parse_str(space).map_err(|_| span.domain_error("unknown UUID namespace"))?,
            };
            if version == 3 { Uuid::new_v3(&space, name.as_bytes()) } else { Uuid::new_v5(&space, name.as_bytes()) }
        }
        (8, Some(data)) => Uuid::new_v8(bytes(data, span)?.try_into().map_err(|_| span.domain_error("UUID version 8 takes 16 bytes"))?),
        _ => return Err(span.domain_error("•uuid takes version 0, 1, 4, 6 or 7 alone, 3 or 5 with a namespace and a name, or 8 with 16 bytes")),
    };
    Ok(keyed::text(&id.hyphenated().to_string()))
}

pub(crate) fn read(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = file_options("•nget", left, None, &["encoding", "binary"], span)?;
    let binary = opts.boolean("binary", false, span)?;
    if binary && opts.values.contains_key("encoding") { return Err(span.domain_error("binary files do not take an encoding")); }
    let path = text(right, span)?;
    span.check()?;
    let data = if path == "-" { span.rest()? } else { span.read(&path)? };
    span.check()?;
    if binary { return byte_vector(data).error_at(span, "invalid byte vector"); }
    let data = String::from_utf8(data).map_err(|e| span.io_error(&path, e))?;
    Ok(keyed::text(&data))
}

/// `path •nput data` writes `data` to a new file, or to standard output with no line ending added when `path` is `"-"`. With the
/// `unique` option, `path` names a directory, and `•nput` writes a new file with a unique name inside it and gives its path, which is
/// displayed. Otherwise the byte count it gives isn't displayed.
pub(crate) fn write(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    let opts = file_options("•nput", left, Some("path"), &["path", "encoding", "overwrite", "unique", "prefix"], span)?;
    let path = opts.text("path", None, span)?;
    let data = payload(right, span)?;
    let count = Value::Number(Number::from_integer(data.len() as i64));
    if path != "-" { return write_file(&opts, &path, &data, count, span); }
    if opts.values.contains_key("unique") || opts.values.contains_key("prefix") {
        return Err(span.domain_error("standard output can't take the unique or prefix options"));
    }
    span.write(&String::from_utf8(data).map_err(|_| span.domain_error("standard output takes UTF-8 text"))?);
    Ok((count, true))
}

/// Writes `data` to the file `path` names, as `•nput` does for any path but `"-"`. `count` is the result.
#[cfg(not(web))]
fn write_file(opts: &Options, path: &str, data: &[u8], count: Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    use std::io::Write;
    if let Some(unique) = crate::files::Unique::new(opts, "•nput", span)? {
        span.check()?;
        let path = unique.make(path, span, |builder, dir| {
            let mut file = builder.tempfile_in(dir)?;
            file.write_all(data)?;
            file.keep().map(|(_, made)| made).map_err(|e| e.error)
        })?;
        return Ok((path, false));
    }
    let overwrite = opts.boolean("overwrite", false, span)?;
    span.check()?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(span.path(path))
        .map_err(|e| span.io_error(path, e))?;
    file.write_all(data).map_err(|e| span.io_error(path, e))?;
    Ok((count, true))
}

/// The browser can't write files.
#[cfg(web)]
fn write_file(_: &Options, path: &str, _: &[u8], _: Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    Err(span.io_error(path, "the browser can't write files"))
}

/// A request for `•fetch`'s host: no `method` means GET, or POST with a `body`.
pub(crate) struct Request {
    pub url: String,
    pub method: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}
/// The final response to a `Request`, after any redirects. Header names are in lower case, and a repeated header's values are joined
/// with `, `, as browsers give them.
pub(crate) struct Response { pub status: i64, pub headers: Vec<(String, String)>, pub body: Vec<u8> }

/// `X •fetch url` requests `url` and returns a record of the response's `status`, `headers` and `body`. Any response is a result,
/// whatever its status. Getting no response is an IO error.
pub(crate) fn fetch(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•fetch", left, None, &["method", "headers", "body", "binary"], span)?;
    let url = text(right, span)?;
    let method = opts.values.get("method").map(|m| text(m, span)).transpose()?;
    let headers = match opts.values.get("headers") {
        Some(h) => keyed::pairs(h)
            .map_err(|_| span.domain_error("•fetch headers must be a record"))?
            .into_iter()
            .map(|(n, v)| Ok((n.to_string(), text(&v, span)?)))
            .collect::<Result<_, Error>>()?,
        None => Vec::new(),
    };
    let body = opts.values.get("body").map(|b| payload(b, span)).transpose()?;
    let response = crate::host::fetch(Request { url: url.clone(), method, headers, body }, span)?;
    let body = response.body;
    let body = if opts.boolean("binary", false, span)? { byte_vector(body).error_at(span, "invalid byte vector")? } else { keyed::text(&String::from_utf8(body).map_err(|e| span.io_error(&url, e))?) };
    let (names, values) = response.headers.into_iter().map(|(n, v)| (n.into(), keyed::text(&v))).unzip();
    let headers = keyed::record(names, values).error_at(span, "invalid headers")?;
    keyed::record(vec!["status".into(), "headers".into(), "body".into()], vec![Value::Number(Number::from_integer(response.status)), headers, body])
        .error_at(span, "invalid response")
}
