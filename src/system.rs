use crate::{
    array::{generated_len, Items},
    execution::Context,
    primitive::{integer, numeric},
    Error, ErrorAt, ErrorKind, Function, Value,
};
use std::borrow::Cow;

/// A system function implemented natively, taking an optional left argument and a right argument.
pub(crate) type Native = fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>;
/// A native called for its effect. It returns its result, and whether the result is shy, as an assignment's is.
pub(crate) type Effect = fn(Option<&Value>, &Value, &Context<'_>) -> Result<(Value, bool), Error>;

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(Native),
    /// A native called for its effect.
    Effect(Effect),
    Session(fn(&mut crate::Session, Option<&Value>, &Value, &crate::Span) -> Result<Value, Error>),
    Regex(std::sync::Arc<crate::host::Regex>, crate::regex::Operation),
    Distribution(std::sync::Arc<crate::distribution::Distribution>, crate::distribution::Operation),
    Generator(crate::distribution::Generator, crate::distribution::Draw),
    Load,
    Element(std::sync::Arc<str>),
    Mime,
    Time,
}

/// How many arguments a system function takes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Valence { Monadic, Dyadic, Ambivalent }
use Valence::{Ambivalent, Dyadic, Monadic};

/// What a system function takes as one item on the right. A right argument that holds several items maps over them, as Each does,
/// with the left argument whole for each.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Item {
    /// The whole argument.
    Whole,
    /// A number. An array maps.
    Number,
    /// A vector of numbers or characters, such as code points or bytes. A deeper array maps.
    Vector,
    /// A string. A deeper array maps, and so does an empty array without characters, as an empty list of strings.
    Text,
}
use Item::{Number, Text, Vector, Whole};

impl Item {
    /// Whether `right` holds several items, not one.
    pub(crate) fn maps(self, right: &Value) -> bool {
        let depth = crate::primitive::depth(right);
        match self {
            Whole => false,
            Number => depth > 0,
            Vector => depth > 1,
            Text => depth > 1 || (right.is_empty() && !matches!(right.as_items(), Items::Characters(_))),
        }
    }
}

/// A system function. `prototype` gives the prototype of its result. In prototype mode, a function that has one returns it instead of
/// running. An empty mapped argument gives an empty result with that prototype.
#[derive(Clone, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call, pub valence: Valence, pub item: Item, pub prototype: Option<fn() -> Value> }
/// System functions match when they share a name and their data matches. A generator matches only its own stream.
impl PartialEq for SystemFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && match (&self.call, &other.call) {
                (Call::Regex(x, a), Call::Regex(y, b)) => (x.as_str(), a) == (y.as_str(), b),
                (Call::Distribution(x, a), Call::Distribution(y, b)) => (x, a) == (y, b),
                (Call::Generator(x, a), Call::Generator(y, b)) => std::sync::Arc::ptr_eq(x, y) && a == b,
                (Call::Element(x), Call::Element(y)) => x == y,
                (x, y) => std::mem::discriminant(x) == std::mem::discriminant(y),
            }
    }
}

impl Valence {
    /// A call with the wrong number of arguments is a SYNTAX error.
    pub(crate) fn check(self, name: &str, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> {
        match (self, left) {
            (Monadic, Some(_)) => Err(span.error(ErrorKind::Syntax, format!("{name} is monadic"))),
            (Dyadic, None) => Err(span.error(ErrorKind::Syntax, format!("{name} needs a left argument"))),
            _ => Ok(()),
        }
    }
}

impl SystemFunction { pub(crate) fn check(&self, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> { self.valence.check(self.name, left, span) } }

/// A function that calls `call` natively, taking its right argument whole. A generator's draw and a distribution's sample have a
/// result prototype. In prototype mode they return it without drawing.
pub(crate) fn native(name: &'static str, call: Call, valence: Valence) -> Function {
    let draws = matches!(call, Call::Generator(..) | Call::Distribution(_, crate::distribution::Operation::Sample));
    Function::system(SystemFunction { name, call, valence, item: Whole, prototype: draws.then_some(float) })
}

/// A keyed vector of native functions, each named by its key.
pub(crate) fn natives(entries: impl IntoIterator<Item = (&'static str, Call, Valence)>) -> Value {
    let (keys, functions) = entries.into_iter().map(|(name, call, valence)| (name.into(), Value::Function(native(name, call, valence)))).unzip();
    crate::keyed::vector(keys, functions).expect("distinct keys")
}

// Result prototypes for the system table.
fn number() -> Value { integer(0) }
fn float() -> Value { Value::Number(0.0.into()) }
fn text() -> Value { crate::keyed::text("") }
fn texts() -> Value { crate::keyed::texts(&[0], Vec::new()).expect("an empty list of text") }
fn numbers() -> Value { Value::integers(vec![0], Vec::new()).expect("an empty vector") }
fn record() -> Value { crate::keyed::vector(Vec::new(), Vec::new()).expect("an empty record") }
fn element() -> Value { crate::xml::element_function("".into()) }
fn valid_numbers() -> Value { Value::new(vec![2], vec![numbers(), numbers()]).expect("a pair") }

/// Each system function's name, call, valence, right-argument item and result prototype.
const BUILTINS: &[(&str, Call, Valence, Item, fn() -> Value)] = &[
    ("•c", Call::Value(case_convert), Ambivalent, Whole, text),
    ("•csv", Call::Value(crate::csv::parse), Ambivalent, Text, record),
    ("•json", Call::Value(crate::json::parse), Ambivalent, Text, number),
    ("•mime", Call::Mime, Ambivalent, Whole, record),
    ("•element", Call::Value(crate::xml::factory), Monadic, Text, element),
    ("•xml", Call::Value(crate::xml::parse), Monadic, Text, record),
    ("•svg", Call::Value(crate::xml::svg), Ambivalent, Whole, record),
    ("•plot", Call::Value(crate::plot::plot), Ambivalent, Whole, record),
    ("•image", Call::Value(crate::image::image), Monadic, Vector, float),
    ("•vfi", Call::Value(crate::data::vfi), Ambivalent, Text, valid_numbers),
    ("•r", Call::Value(crate::regex::compile), Monadic, Text, record),
    ("•distribution", Call::Value(crate::distribution::distribution), Ambivalent, Text, record),
    ("•rand", Call::Value(crate::distribution::generator), Monadic, Whole, record),
    ("•nget", Call::Value(crate::data::read), Ambivalent, Text, text),
    ("•nput", Call::Effect(crate::data::write), Dyadic, Whole, number),
    ("•fetch", Call::Value(crate::data::fetch), Ambivalent, Text, record),
    ("•deflate", Call::Value(crate::data::inflate), Ambivalent, Vector, numbers),
    ("•hash", Call::Value(crate::data::hash), Ambivalent, Vector, text),
    ("•uuid", Call::Value(crate::data::uuid), Ambivalent, Number, text),
    ("•normalize", Call::Value(normalize), Ambivalent, Text, text),
    ("•decompose", Call::Value(crate::primitive::decompose), Dyadic, Whole, record),
    ("•ucs", Call::Value(unicode_convert), Ambivalent, Vector, numbers),
    ("•load", Call::Load, Monadic, Text, record),
    ("•signal", Call::Value(signal), Ambivalent, Whole, number),
    ("•storage", Call::Value(storage), Monadic, Whole, text),
    ("•time", Call::Time, Ambivalent, Whole, float),
    ("•host", Call::Value(host), Monadic, Text, text),
    #[cfg(not(web))]
    ("•delay", Call::Effect(delay), Monadic, Whole, float),
    ("•date", Call::Value(crate::date::read), Ambivalent, Whole, float),
    #[cfg(not(web))]
    ("•path", Call::Value(crate::files::path), Ambivalent, Whole, record),
    #[cfg(not(web))]
    ("•metadata", Call::Value(crate::files::metadata), Monadic, Whole, record),
    #[cfg(not(web))]
    ("•readdir", Call::Value(crate::files::readdir), Ambivalent, Text, record),
    #[cfg(not(web))]
    ("•copy", Call::Effect(crate::files::copy), Dyadic, Whole, text),
    #[cfg(not(web))]
    ("•rename", Call::Effect(crate::files::rename), Dyadic, Whole, text),
    #[cfg(not(web))]
    ("•remove", Call::Effect(crate::files::remove), Ambivalent, Text, text),
    #[cfg(not(web))]
    ("•mkdir", Call::Effect(crate::files::mkdir), Ambivalent, Text, text),
    ("•prefs", Call::Session(crate::display::prefs), Monadic, Whole, record),
    ("•nc", Call::Session(crate::Session::system_nc), Monadic, Text, number),
    ("•nl", Call::Session(crate::Session::system_nl), Ambivalent, Whole, texts),
    ("•src", Call::Session(crate::Session::system_src), Monadic, Text, text),
    ("•ex", Call::Session(crate::Session::system_ex), Monadic, Text, number),
    ("•literal", Call::Session(crate::Session::system_literal), Monadic, Text, number),
];

pub fn names() -> impl Iterator<Item = &'static str> { BUILTINS.iter().map(|(name, ..)| *name) }

/// The table entry for system name `name`, ignoring case.
fn builtin(name: &str) -> Option<&'static (&'static str, Call, Valence, Item, fn() -> Value)> { BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name)) }

/// The help page for system name `name`, ignoring case, from `nbs/system`.
pub(crate) fn help(name: &str) -> Option<&'static str> { let (name, ..) = builtin(name)?; crate::inspection::page(name) }

pub(crate) fn lookup(name: &str) -> Option<Function> {
    let (name, call, valence, item, prototype) = builtin(name)?;
    Some(Function::system(SystemFunction { name, call: call.clone(), valence: *valence, item: *item, prototype: Some(*prototype) }))
}

/// The system functions with an inverse, each with its inverse and the inverse's valence. A format function reads its format, and
/// its inverse writes it.
const INVERSES: &[(&str, Native, Valence)] = &[
    ("•csv", crate::csv::serialize, Ambivalent),
    ("•json", crate::json::serialize, Ambivalent),
    ("•xml", crate::xml::serialize, Monadic),
    ("•image", crate::image::encode, Ambivalent),
    ("•deflate", crate::data::deflate, Ambivalent),
    ("•date", crate::date::write, Ambivalent),
    #[cfg(not(web))]
    ("•path", crate::files::join, Monadic),
    ("•literal", write_literal, Monadic),
    ("•ucs", unicode_convert, Ambivalent),
    ("•decompose", crate::primitive::recompose, Dyadic),
];

/// `f⁻¹` for a system function `f` that has an inverse.
pub(crate) fn inverse(f: &SystemFunction, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    use crate::distribution::Operation::{Cdf, Quantile};
    if let Call::Distribution(d, op @ (Cdf | Quantile)) = &f.call {
        return crate::distribution::call(d, if matches!(op, Cdf) { Quantile } else { Cdf }, left, right, cx);
    }
    let (_, call, valence) = INVERSES.iter().find(|(name, ..)| *name == f.name).ok_or_else(|| cx.domain_error("this function has no known inverse"))?;
    valence.check(&format!("{}⁻¹", f.name), left, cx.span)?;
    call(left, right, cx)
}

const HOST_FACTS: &[&str] = &[
    "args",
    "version",
    "env",
    "width",
    "height",
    #[cfg(not(web))]
    "cwd",
    #[cfg(not(web))]
    "temp",
];

/// `•host name` gives the host fact called `name`: the command-line `"args"`, BPL's `"version"`, the `"env"` record, the terminal's
/// `"width"` and `"height"`, the working directory `"cwd"`, and `"temp"`, the directory for temporary files.
fn host(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let fact = match crate::keyed::name(right).as_deref() {
        Some("args") => {
            let args = &span.session.args;
            crate::keyed::texts(&[args.len()], args.iter().map(|a| crate::keyed::text(a)).collect())
        }
        Some("version") => Ok(crate::keyed::text(env!("CARGO_PKG_VERSION"))),
        Some("env") => {
            let (names, values) = crate::host::environment().into_iter().map(|(k, v)| (k.into(), crate::keyed::text(&v))).unzip();
            crate::keyed::record(names, values)
        }
        Some(fact @ ("width" | "height")) => match terminal_size() {
            Some((rows, columns)) => Value::number((if fact == "height" { rows } else { columns }) as f64),
            None => Value::integers(vec![0], vec![]),
        },
        #[cfg(not(web))]
        Some("cwd") => Ok(crate::keyed::text(&std::env::current_dir().map_err(|e| span.domain_error(format!("working directory: {e}")))?.to_string_lossy())),
        #[cfg(not(web))]
        Some("temp") => Ok(crate::keyed::text(&std::env::temp_dir().to_string_lossy())),
        _ => return Err(span.domain_error(format!("•host takes {}", HOST_FACTS.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ")))),
    };
    fact.error_at(span, "invalid host fact")
}

/// The rows and columns of the terminal that standard output writes to. `None` without one.
#[cfg(unix)]
pub(crate) fn terminal_size() -> Option<(usize, usize)> {
    let size = rustix::termios::tcgetwinsize(std::io::stdout()).ok()?;
    (size.ws_row > 0 && size.ws_col > 0).then(|| (usize::from(size.ws_row), usize::from(size.ws_col)))
}
#[cfg(not(unix))]
pub(crate) fn terminal_size() -> Option<(usize, usize)> { None }

/// `•delay s` pauses for `s` seconds and gives the seconds it waited. `•delay ∞` waits until interrupted. Interrupts stop any delay.
#[cfg(not(web))]
fn delay(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    use {crate::host::Instant, std::time::Duration};
    let seconds = crate::primitive::real(right, span)?;
    if seconds.is_nan() || seconds < 0.0 { return Err(span.domain_error("•delay needs a nonnegative number of seconds")); }
    let start = Instant::now();
    let end = Duration::try_from_secs_f64(seconds).ok().and_then(|d| start.checked_add(d));
    loop {
        let now = Instant::now();
        if end.is_some_and(|end| now >= end) { break; }
        span.pause(end.map_or(Duration::MAX, |end| end - now))?;
    }
    Ok((Value::Number(start.elapsed().as_secs_f64().into()), true))
}

fn storage(_: Option<&Value>, right: &Value, _: &Context<'_>) -> Result<Value, Error> { Ok(crate::keyed::text(right.storage_name())) }

/// `•signal kind` raises an error of the kind that `kind` names, and `message •signal kind` gives it a message. `kind` can also be a
/// caught error, such as `$e`, which raises its kind and message again.
fn signal(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let invalid = || span.domain_error("•signal needs an error kind such as \"DOMAIN\", or a caught error");
    let (name, message) = match crate::keyed::name(right) {
        Some(name) => (name, "explicitly signalled".into()),
        None => {
            let fields = crate::keyed::pairs(right).map_err(|_| invalid())?;
            let field = |key: &str| fields.iter().find(|(k, _)| &**k == key).and_then(|(_, v)| crate::keyed::name(v)).ok_or_else(invalid);
            (field("kind")?, field("message")?.to_string())
        }
    };
    let kind = ErrorKind::named(&name).ok_or_else(invalid)?;
    let message = match left { Some(m) => crate::data::text(m, span)?, None => message };
    Err(span.error(kind, message))
}

fn case_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mode = match left {
        None => -3,
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().error_at(span, "•c mode must be 1, ¯1 or ¯3")?,
        _ => return Err(span.domain_error("•c needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.domain_error("•c mode must be 1, ¯1 or ¯3")); }
    let text: fn(&str) -> String = match mode { 1 => str::to_uppercase, -1 => str::to_lowercase, _ => |s| s.to_uppercase().to_lowercase() };
    cased(right, text, span)
}

/// `value` with each character vector mapped as a string by `text`, and each other character mapped on its own. A character whose
/// mapping has more than one character stays as it is.
fn cased(value: &Value, text: fn(&str) -> String, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    let mapped = match value {
        Value::Character(c) => {
            let s = text(&c.to_string());
            let mut chars = s.chars();
            return Ok(Value::Character(match (chars.next(), chars.next()) { (Some(m), None) => m, _ => *c }));
        }
        Value::Array(_) if !value.is_empty() => match crate::keyed::name(value) {
            Some(s) => crate::keyed::text(&text(&s)),
            None => Value::new(value.shape().to_vec(), value.elements().map(|e| cased(&e, text, span)).collect::<Result<_, _>>()?)
                .error_at(span, "invalid result")?,
        },
        _ => return Ok(value.clone()),
    };
    if mapped.shape() != value.shape() { return Ok(mapped); }
    mapped.with_layout(value.layout().clone()).error_at(span, "invalid result")
}

/// `form •normalize text` puts `text` into a Unicode normalization form: `"NFC"`, the default, `"NFD"`, `"NFKC"` or `"NFKD"`.
fn normalize(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let form = crate::data::algorithm(left, "nfc", span)?.to_uppercase();
    if !["NFC", "NFD", "NFKC", "NFKD"].contains(&form.as_str()) { return Err(span.domain_error("•normalize takes \"NFC\", \"NFD\", \"NFKC\" or \"NFKD\"")); }
    let text = crate::keyed::name(right).ok_or_else(|| span.domain_error("•normalize needs text"))?;
    Ok(crate::keyed::text(&crate::host::normalize(&form, &text)))
}

/// `•literal⁻¹ Y` writes `Y` as BPL source, which `•literal` reads back. A value that holds a function or an operator is a DOMAIN error.
pub(crate) fn write_literal(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.holds_function() { return Err(span.domain_error("•literal⁻¹ writes data, not functions")); }
    Ok(crate::keyed::text(&right.literal()))
}

fn unicode_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let invalid = || span.domain_error("invalid Unicode conversion");
    let encoding = if let Some(spec) = left {
        let name = if matches!(spec.elements().next(), Some(Value::Array(_))) {
            if spec.shape().len() > 1 || !(1..=2).contains(&spec.len()) { return Err(invalid()); }
            if spec.len() == 2 {
                let mode = numeric(&spec.at(1), span)?.integer().map_err(|_| invalid())?;
                if mode == 83 { return Err(span.error(ErrorKind::Unsupported, "•ucs signed bytes are out of scope")); }
                if mode != 0 { return Err(invalid()); }
            }
            spec.at(0).clone()
        } else { spec.clone() };
        let text = crate::keyed::name(&name).filter(|_| !name.is_atom()).ok_or_else(invalid)?;
        if !matches!(&*text, "UTF-8" | "UTF-16" | "UTF-32") { return Err(invalid()); }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "encoded •ucs needs a vector")); }
        Some(text)
    } else { None };
    let shape = |len: usize| {
        let shape = if encoding.is_some() { vec![len] } else { right.shape().to_vec() };
        generated_len(&shape).map(|_| shape).error_at(span, "Unicode result exceeds element limit")
    };
    let result = if matches!(right.prototype(), Value::Character(_)) {
        let chars: Cow<[char]> = match right.as_items() {
            Items::Characters(cs) => Cow::Borrowed(cs),
            Items::Values(vs) => Cow::Owned(vs.iter().map(|v| if let Value::Character(c) = v { Ok(*c) } else { Err(invalid()) }).collect::<Result<_, _>>()?),
            _ => return Err(invalid()),
        };
        let codes: Vec<i64> = match encoding.as_deref() {
            Some("UTF-8") => chars.iter().collect::<String>().bytes().map(i64::from).collect(),
            Some("UTF-16") => chars.iter().collect::<String>().encode_utf16().map(i64::from).collect(),
            _ => chars.iter().map(|&c| i64::from(u32::from(c))).collect(),
        };
        if encoding.is_none() && right.is_atom() { return Ok(integer(codes[0])); }
        Value::integers(shape(codes.len())?, codes)
    } else {
        if !matches!(right.prototype(), Value::Number(_)) { return Err(invalid()); }
        let codes = right.as_items().nonnegative_integers::<u32>().map_err(|_| invalid())?;
        let chars: Vec<char> = match encoding.as_deref() {
            Some("UTF-8") => {
                let bytes: Vec<_> = codes.into_iter().map(u8::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                std::str::from_utf8(&bytes).map_err(|_| invalid())?.chars().collect()
            }
            Some("UTF-16") => {
                let units: Vec<_> = codes.into_iter().map(u16::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                char::decode_utf16(units).collect::<Result<_, _>>().map_err(|_| invalid())?
            }
            _ => codes.into_iter().map(char::from_u32).collect::<Option<_>>().ok_or_else(invalid)?,
        };
        if encoding.is_none() && right.is_atom() { return Ok(Value::Character(chars[0])); }
        Value::characters(shape(chars.len())?, chars)
    };
    result.and_then(|a| if encoding.is_none() { a.with_layout(right.layout().clone()) } else { Ok(a) }).error_at(span, "invalid Unicode result")
}
