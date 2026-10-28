use crate::execution::check;
use crate::session;
use crate::{Described, Error, ErrorKind, Number, Span, Value, array::Storage, keyed, number::Numeric, primitive::{numeric, real}};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{buffer::{Buffer, ComplexBuf, DType, Element, FloatBuf, FloatWidth, Ints, View, Width}, error::Kind};
use std::io::Read;

pub(crate) fn text(value: &Value) -> Result<String, Error> { keyed::name(value).map(|s| s.to_string()).ok_or_else(|| Error::domain("expected text")) }

/// Imported items, where `filled` marks missing-value fills. Where every other number is an ordinary integer, every one is a fixed integer of one width, or every one is a Boolean, a fill takes that type wherever it keeps its value there, as a fill under `⍠` does (`Value::fill_of`): a fill of `0` beside integers gives `0ₓ`. Approximate inputs, and a fill that doesn't keep its value in that type, such as NaN or `0.5` beside integers, select real or complex storage at the widest strong approximate width, or the widest weak width when none is strong. Weak approximate values may round; exact values must convert losslessly. Otherwise items keep their own kinds.
pub(crate) fn imported(shape: Vec<usize>, mut items: Vec<Value>, filled: impl Fn(usize) -> bool, empty_prototype: Value) -> Result<Value, ErrorKind> {
    fn number(v: &Value) -> Option<&Number> { if let Value::Number(n) = v { Some(n) } else { None } }
    let mut widened = false;
    if (0..items.len()).any(&filled) {
        let mut data = items.iter().enumerate().filter(|&(i, _)| !filled(i)).map(|(_, v)| number(v));
        let like = data.next().flatten().filter(|n| n.as_integer().is_some()).cloned().filter(|n| data.all(|d| d.is_some_and(|d| d.same_variant(n))));
        if let Some(like) = like {
            // An ordinary integer's view is at 64 bits, so the fill isn't limited to the narrowest width that holds `like`.
            let (dtype, like) = (like.view().map(|v| v.dtype()), Value::Number(like));
            for v in items.iter_mut().enumerate().filter_map(|(i, v)| filled(i).then_some(v)) {
                match Value::fill_of(v, dtype, like.fixed_width().is_some()) { Ok((_, fill)) => *v = like.fill_item(Some(&fill)), Err(_) => widened = true }
            }
        }
    }
    let numbers = || items.iter().filter_map(number);
    if widened || items.iter().enumerate()
            .any(|(i, v)| number(v).is_some_and(|n| n.as_float().is_some_and(|f| !f.is_finite() || !filled(i)) || n.as_complex().is_some())) {
                let (mut width, mut strong, mut complex) = (None, None, false);
                for n in numbers() {
                    let w = n.float_width();
                    width = width.max(w);
                    complex |= n.as_complex().is_some();
                    if !n.is_weak() { strong = strong.max(w); }
                }
                let width = strong.or(width).unwrap_or(FloatWidth::F64);
                let width = if complex { width.max(FloatWidth::F32) } else { width };
                let fits = |n: &Number, f: f64| n.is_weak() && !n.is_exact() || f.is_nan() || Number::float(f, width).as_float() == Some(f);
                let exactly = |n: &Number| n.lossless_float().filter(|&f| fits(n, f));
                if complex {
                    let values = items.iter().map(|v| {
                        let n = number(v)?;
                        let z = n.as_complex().or_else(|| exactly(n).map(|re| num_complex::Complex64::new(re, 0.0)))?;
                        (fits(n, z.re) && fits(n, z.im)).then_some(z)
                    }).collect::<Option<Vec<_>>>();
                    if let Some(values) = values { return Value::complex(shape, ComplexBuf::collect(width.into(), values.into_iter())); }
                } else if let Some(floats) = items.iter().map(|v| number(v).and_then(exactly)).collect::<Option<Vec<_>>>() {
                    return Value::from_storage(shape, Storage::Compact(Buffer::Floats(FloatBuf::collect(width, floats.into_iter()))));
                }
            }
    Value::from_parts(shape, items, empty_prototype)
}

pub(crate) struct Options { pub values: HashMap<String, Value>, name: &'static str }

impl Options {
    /// Options from a keyed vector, or from plain text given to `shorthand`. `None`, or `''` without a shorthand, gives no options.
    pub fn new(name: &'static str, value: Option<&Value>, shorthand: Option<&str>, allowed: &[&str]) -> Result<Self, Error> {
        let mut values = HashMap::new();
        let Some(value) = value else { return Ok(Self { values, name }) };
        if let Some(keys) = value.keys(0) {
            if value.shape().len() != 1 { return Err(Error::new(Kind::Rank, format!("{name} options must be a keyed vector"))); }
            for (k, v) in keys.names().iter().zip(value.elements()) {
                let k = k.as_ref().ok_or_else(|| Error::domain(format!("{name} options need a key for every entry")))?.to_lowercase();
                if !allowed.contains(&k.as_str()) { return Err(Error::domain(format!("unknown {name} option: {k}"))); }
                if values.insert(k.clone(), v).is_some() { return Err(Error::domain(format!("duplicate {name} option: {k}"))); }
            }
        } else if let Some(key) = shorthand { values.insert(key.into(), value.clone()); }
        else if !text(value).is_ok_and(|t| t.is_empty()) { return Err(Error::domain(format!("{name} expects '' or keyed options"))); }
        Ok(Self { values, name })
    }

    pub fn text(&self, key: &str, default: Option<&str>) -> Result<String, Error> {
        match (self.values.get(key), default) {
            (Some(v), _) => text(v), (None, Some(d)) => Ok(d.into()), (None, None) => Err(Error::domain(format!("{} needs {key}", self.name)))
        }
    }

    pub fn number(&self, key: &str) -> Result<Option<f64>, Error> {
        self.values.get(key).map(|v| real(v).map_err(|_| Error::domain(format!("{} {key} must be a real number", self.name)))).transpose()
    }

    pub fn boolean(&self, key: &str, default: bool) -> Result<bool, Error> {
        match self.values.get(key) {
            None => Ok(default), Some(Value::Number(n)) => n.boolean().map_err(Error::from),
            _ => Err(Error::domain(format!("{} {key} must be Boolean", self.name)))
        }
    }

    pub fn fill(&self) -> Result<Number, Error> {
        match self.values.get("fill") {
            None => Ok(Numeric::current().float(f64::NAN)), Some(Value::Number(n)) if n.as_complex().is_none() => Ok(n.clone()),
            _ => Err(Error::domain(format!("{} fill must be a real number", self.name)))
        }
    }

    /// The `types` option, as in `["types":["id":0ₓ₈ "zip":""]]`: a value of each column's type, from a record that names columns, or from a vector that holds one for each column in order. A number gives its type, and text, such as `""`, makes a column text.
    pub fn types(&self) -> Result<Types, Error> {
        let Some(types) = self.values.get("types") else { return Ok(Types::Named(HashMap::new())) };
        let invalid = || Error::domain(format!("{} types must be a record or a vector of numbers and texts", self.name));
        let items: Vec<Value> = types.elements().collect();
        if types.shape().len() != 1 || keyed::name(types).is_some() || !items.iter().all(|t| matches!(t, Value::Number(_)) || keyed::name(t).is_some()) {
            return Err(invalid());
        }
        Ok(match types.keys(0) {
            Some(keys) => {
                Types::Named(keys.names().iter().zip(items).map(|(k, t)| Ok((k.as_ref().ok_or_else(invalid)?.to_string(), t))).collect::<Result<_, Error>>()?)
            }
            None => Types::Ordered(items),
        })
    }
}

/// The columns that the `types` option gives types: by name, or one for each column in order.
pub(crate) enum Types { Named(HashMap<String, Value>), Ordered(Vec<Value>) }

/// `column`, a column of numbers that `reader` read, named `name`, at the type it takes. A column that the `types` option names takes the type of `typed`, its entry there: an integer of the `int` width under static kinds where `typed` is an ordinary integer. Under static kinds, a column of integers that the option doesn't name takes the `int` width, as every new integer does, so that its type never depends on its values. A value that doesn't keep itself in that type is a DOMAIN error that names the column and the `types` option, and the fill where `missing` says that the column had missing cells. Any other column keeps the type that its numbers gave it.
pub(crate) fn typed_column(reader: &str, name: &str, column: Value, typed: Option<&Value>, missing: bool) -> Result<Value, Error> {
    let numeric = Numeric::current();
    let int = if numeric.is_static() { numeric.int } else { Width::I64 };
    let integers = || match column.dtype() {
        Some(DType::Integer(_)) => true,
        None => column.len() > 0 && column.elements().all(|v| matches!(&v, Value::Number(n) if n.is_integral() && n.as_bool().is_none())), _ => false
    };
    let target = match typed {
        Some(Value::Number(t)) => match (t.dtype(), t.fixed_width()) {
            (Some(DType::Integer(w)), Some(_)) => (DType::Integer(w), true), (Some(DType::Integer(_)), None) => (DType::Integer(int), false),
            (Some(dtype), _) => (dtype, false),
            (None, _) => return Err(Error::domain(format!("{reader} types needs a number of each column's type, such as 0ₓ₈, 0ₛ or $f")))
        },
        _ if numeric.is_static() && integers() => (DType::Integer(int), false),
        _ => return Ok(column),
    };
    let (dtype, fixed) = target;
    let place = if name.is_empty() { format!("{reader} array") } else { format!("{reader} column {name:?}") };
    let value = if missing { "a value, or the fill at its missing cells," } else { "a value" };
    let lost = || {
        Error::domain(match typed {
            Some(_) => format!("{place}: {value} doesn't keep itself as {}, the type that types gives it", dtype.name()),
            None => format!("{place}: {value} doesn't fit {}. Give the column a type with the types option", dtype.name()),
        })
    };
    let buffer = match column.view() {
        Some(view) => Buffer::kept(view, dtype).ok(),
        // Items that keep their own kinds, as a JSON array of Booleans and integers does, convert one at a time.
        None => column.elements().try_fold(Buffer::with_capacity(dtype, column.len()), |mut b, v| {
            let Value::Number(n) = &v else { return None };
            b.extend_from(Buffer::kept(n.view()?, dtype).ok()?.view(), 0..1).then_some(b)
        }),
    }.ok_or_else(lost)?;
    let storage = if fixed || numeric.is_static() || !matches!(dtype, DType::Integer(_)) { Storage::Compact(buffer).fixed(fixed) }
    else { Storage::narrowed(i64::of(buffer.view()).expect("integers at 64 bits").to_vec()) };
    Value::from_storage([column.len()], storage).described("invalid column")
}

pub(crate) fn vfi(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let input = text(right)?;
    let separators = left.map(|a| text(a)).transpose()?;
    let mut valid = Vec::new();
    let mut numbers = Vec::new();
    let numeric = Numeric::current();
    let zero = numeric.float(0.0);
    let mut field = |s: &str| -> Result<(), Error> {
        check()?;
        let s = s.trim();
        let n = if s.is_empty() { Ok(zero.clone()) } else { Number::parse(s, numeric) };
        valid.push(i64::from(n.is_ok()));
        numbers.push(Value::Number(n.unwrap_or_else(|_| zero.clone())));
        Ok(())
    };
    if let Some(separators) = separators {
        if !input.is_empty() { for s in input.split(|c| separators.contains(c)) { field(s)?; } }
    } else { for s in input.split_whitespace() { field(s)?; } }
    let result = || {
        let numbers = imported(vec![numbers.len()], numbers, |i| valid[i] == 0, Value::Number(zero))?;
        Value::new(vec![2], vec![Value::integers(vec![valid.len()], valid)?, numbers])
    };
    result().described("invalid numeric input result")
}

/// File options. `encoding` must be UTF-8.
fn file_options(name: &'static str, left: Option<&Value>, shorthand: Option<&str>, allowed: &[&str]) -> Result<Options, Error> {
    let opts = Options::new(name, left, shorthand, allowed)?;
    if !opts.text("encoding", Some("UTF-8"))?.eq_ignore_ascii_case("UTF-8") { return Err(Error::domain("encoding must be UTF-8")); }
    Ok(opts)
}

/// `data` as a vector of byte values, one byte each.
pub(crate) fn byte_vector(data: Vec<u8>) -> Result<Value, ErrorKind> {
    Value::from_storage([data.len()], Storage::within(data, crate::array::ordinary_width(Width::U8)))
}

/// The bytes in an array of integers from 0 to 255, in ravel order. `None` for any other array.
pub(crate) fn byte_items(value: &Value) -> Option<Vec<u8>> {
    if let Some(View::Integers(Ints::U8(v))) = value.view() { return Some(v.to_vec()); }
    value.as_items().nonnegative_integers().ok()
}

/// The bytes in an array of integers from 0 to 255, in ravel order.
pub(crate) fn bytes(value: &Value) -> Result<Vec<u8>, Error> { byte_items(value).ok_or_else(|| Error::domain("bytes must be whole numbers in 0..255")) }

/// `value` as bytes, taking integers from 0 to 255 as they are and text as UTF-8.
pub(crate) fn payload(value: &Value) -> Result<Vec<u8>, Error> {
    if value.prototype().as_number().is_some() { bytes(value) } else { Ok(text(value)?.into_bytes()) }
}

/// The algorithm or format that the optional left argument names, in lower case, or `default` without one.
pub(crate) fn algorithm(left: Option<&Value>, default: &str) -> Result<String, Error> {
    left.map_or(Ok(default.into()), |name| text(name).map(|s| s.to_lowercase()))
}

/// `•deflate` decompresses bytes in the format `"gzip"`, `"zlib"` or `"deflate"`, gzip by default. `•deflate⁻¹` compresses bytes or text. Both give bytes.
fn flate(left: Option<&Value>, right: &Value, compress: bool) -> Result<Value, Error> {
    use flate2::{Compression, read};
    let (data, level) = (payload(right)?, Compression::default());
    let mut reader: Box<dyn Read + '_> = match (algorithm(left, "gzip")?.as_str(), compress) {
        ("gzip", true) => Box::new(read::GzEncoder::new(&data[..], level)), ("gzip", false) => Box::new(read::GzDecoder::new(&data[..])),
        ("zlib", true) => Box::new(read::ZlibEncoder::new(&data[..], level)), ("zlib", false) => Box::new(read::ZlibDecoder::new(&data[..])),
        ("deflate", true) => Box::new(read::DeflateEncoder::new(&data[..], level)), ("deflate", false) => Box::new(read::DeflateDecoder::new(&data[..])),
        _ => return Err(Error::domain("•deflate takes \"gzip\", \"zlib\" or \"deflate\""))
    };
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(|e| Error::domain(format!("invalid compressed data: {e}")))?;
    byte_vector(out).described("invalid byte vector")
}
pub(crate) fn inflate(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> { flate(left, right, false) }
pub(crate) fn deflate(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> { flate(left, right, true) }

/// `•hash` gives the SHA-2 digest of bytes or text in hex, by `"sha224"`, `"sha256"`, `"sha384"` or `"sha512"`, SHA-256 by default.
pub(crate) fn hash(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
    let data = payload(right)?;
    let digest = match algorithm(left, "sha256")?.as_str() {
        "sha224" => Sha224::digest(&data).to_vec(), "sha256" => Sha256::digest(&data).to_vec(), "sha384" => Sha384::digest(&data).to_vec(),
        "sha512" => Sha512::digest(&data).to_vec(), _ => return Err(Error::domain("•hash takes \"sha224\", \"sha256\", \"sha384\" or \"sha512\""))
    };
    Ok(keyed::text(&digest.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

/// `•uuid v` gives a new UUID of version `v` as text. Versions 1, 4, 6 and 7 take no left argument. Versions 3 and 5 take a namespace and a name on the left, and version 8 takes 16 bytes.
pub(crate) fn uuid(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    use uuid::Uuid;
    // The standard allows a random node in place of a MAC address, with the multicast bit set.
    let node = || {
        let mut node: [u8; 6] = rand::random();
        node[0] |= 1;
        node
    };
    let version = numeric(right)?.integer().described("•uuid needs a version number")?;
    // These versions read the clock or random bits.
    if matches!((version, left), (1 | 4 | 6 | 7, None)) { crate::transform::acts()?; }
    let id = match (version, left) {
        (0, None) => Uuid::nil(),
        (1, None) => Uuid::now_v1(&node()),
        (4, None) => Uuid::new_v4(),
        (6, None) => Uuid::now_v6(&node()),
        (7, None) => Uuid::now_v7(),
        (3 | 5, Some(names)) => {
            let names: Vec<_> = names.elements().map(|v| text(&v)).collect::<Result<_, _>>()?;
            let [space, name] = names.as_slice() else { return Err(Error::domain("UUID versions 3 and 5 take a namespace and a name")) };
            let space = match space.to_lowercase().as_str() {
                "dns" => Uuid::NAMESPACE_DNS, "url" => Uuid::NAMESPACE_URL, "oid" => Uuid::NAMESPACE_OID, "x500" => Uuid::NAMESPACE_X500,
                _ => Uuid::parse_str(space).map_err(|_| Error::domain("unknown UUID namespace"))?
            };
            if version == 3 { Uuid::new_v3(&space, name.as_bytes()) } else { Uuid::new_v5(&space, name.as_bytes()) }
        }
        (8, Some(data)) => Uuid::new_v8(bytes(data)?.try_into().map_err(|_| Error::domain("UUID version 8 takes 16 bytes"))?),
        _ => return Err(Error::domain("•uuid takes version 0, 1, 4, 6 or 7 alone, 3 or 5 with a namespace and a name, or 8 with 16 bytes")),
    };
    Ok(keyed::text(&id.hyphenated().to_string()))
}

pub(crate) fn read(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let opts = file_options("•nget", left, None, &["encoding", "binary"])?;
    let binary = opts.boolean("binary", false)?;
    if binary && opts.values.contains_key("encoding") { return Err(Error::domain("binary files do not take an encoding")); }
    let path = text(right)?;
    check()?;
    crate::transform::acts()?;
    let data = if path == "-" { session().execution.rest(span)? } else { span.read(&path)? };
    check()?;
    if binary { return byte_vector(data).described("invalid byte vector"); }
    let data = String::from_utf8(data).map_err(|e| Error::io(&path, e))?;
    Ok(keyed::text(&data))
}

/// `path •nput data` writes `data` to a new file, or to standard output with no line ending added when `path` is `"-"`. With the `unique` option, `path` names a directory, and `•nput` writes a new file with a unique name inside it and gives its path, which is displayed. Otherwise the byte count it gives isn't displayed.
pub(crate) fn write(left: Option<&Value>, right: &Value, span: &Span) -> Result<(Value, bool), Error> {
    let opts = file_options("•nput", left, Some("path"), &["path", "encoding", "overwrite", "unique", "prefix"])?;
    let path = opts.text("path", None)?;
    let data = payload(right)?;
    let count = Value::integer(data.len() as i64);
    if path != "-" { return write_file(&opts, &path, &data, count, span); }
    if opts.values.contains_key("unique") || opts.values.contains_key("prefix") {
        return Err(Error::domain("standard output can't take the unique or prefix options"));
    }
    session().execution.write(&String::from_utf8(data).map_err(|_| Error::domain("standard output takes UTF-8 text"))?);
    Ok((count, true))
}

/// Writes `data` to the file `path` names, as `•nput` does for any path but `"-"`. `count` is the result.
#[cfg(not(web))]
fn write_file(opts: &Options, path: &str, data: &[u8], count: Value, span: &Span) -> Result<(Value, bool), Error> {
    use std::io::Write;
    if let Some(unique) = crate::files::Unique::new(opts, "•nput")? {
        check()?;
        let path = unique.make(path, span, |builder, dir| {
            let mut file = builder.tempfile_in(dir)?;
            file.write_all(data)?;
            file.keep().map(|(_, made)| made).map_err(|e| e.error)
        })?;
        return Ok((path, false));
    }
    let overwrite = opts.boolean("overwrite", false)?;
    check()?;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(!overwrite).create(overwrite).truncate(overwrite).open(span.path(path))
        .map_err(|e| Error::io(path, e))?;
    file.write_all(data).map_err(|e| Error::io(path, e))?;
    Ok((count, true))
}

/// The browser can't write files.
#[cfg(web)]
fn write_file(_: &Options, path: &str, _: &[u8], _: Value, _: &Span) -> Result<(Value, bool), Error> { Err(Error::io(path, "the browser can't write files")) }

/// A request for `•fetch`'s host: no `method` means GET, or POST with a `body`.
pub(crate) struct Request { pub url: String, pub method: Option<String>, pub headers: Vec<(String, String)>, pub body: Option<Vec<u8>> }
/// The final response to a `Request`, after any redirects. Header names are in lower case, and a repeated header's values are joined with `, `, as browsers give them.
pub(crate) struct Response { pub status: i64, pub headers: Vec<(String, String)>, pub body: Vec<u8> }

/// `X •fetch url` requests `url` and returns a record of the response's `status`, `headers` and `body`. Any response is a result, whatever its status. Getting no response is an IO error.
pub(crate) fn fetch(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let opts = Options::new("•fetch", left, None, &["method", "headers", "body", "binary"])?;
    let url = text(right)?;
    let method = opts.values.get("method").map(|m| text(m)).transpose()?;
    let headers = match opts.values.get("headers") {
        Some(h) => keyed::pairs(h).map_err(|_| Error::domain("•fetch headers must be a record"))?.into_iter().map(|(n, v)| Ok((n.to_string(), text(&v)?)))
            .collect::<Result<_, Error>>()?, None => Vec::new()
    };
    let body = opts.values.get("body").map(|b| payload(b)).transpose()?;
    crate::transform::acts()?;
    let response = crate::host::fetch(Request { url: url.clone(), method, headers, body })?;
    let body = response.body;
    let body = if opts.boolean("binary", false)? { byte_vector(body).described("invalid byte vector")? }
    else { keyed::text(&String::from_utf8(body).map_err(|e| Error::io(&url, e))?) };
    let (names, values) = response.headers.into_iter().map(|(n, v)| (n.into(), keyed::text(&v))).unzip();
    let headers = keyed::vector(names, values).described("invalid headers")?;
    keyed::vector(vec!["status".into(), "headers".into(), "body".into()], vec![Value::integer(response.status), headers, body]).described("invalid response")
}
