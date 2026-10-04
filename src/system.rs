use crate::{
    array::{generated_len, Items},
    execution::Context,
    primitive::{integer, numeric, pervade, EmptyFill},
    Error, ErrorAt, ErrorKind, Function, Value,
};
use std::borrow::Cow;

/// A system function implemented natively, taking an optional left argument and a right argument.
pub(crate) type Native = fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>;

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(Native),
    Session(fn(&mut crate::Session, Option<&Value>, &Value, &crate::Span) -> Result<Value, Error>),
    Regex(std::sync::Arc<::regex::Regex>, crate::regex::Operation),
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

#[derive(Clone, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call, pub valence: Valence }
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

impl SystemFunction {
    /// A call with the wrong number of arguments is a SYNTAX error.
    pub(crate) fn check(&self, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> {
        match (self.valence, left) {
            (Monadic, Some(_)) => Err(span.error(ErrorKind::Syntax, format!("{} is monadic", self.name))),
            (Dyadic, None) => Err(span.error(ErrorKind::Syntax, format!("{} needs a left argument", self.name))),
            _ => Ok(()),
        }
    }
}

/// A function that calls `call` natively.
pub(crate) fn native(name: &'static str, call: Call, valence: Valence) -> Function { Function::system(SystemFunction { name, call, valence }) }

/// A keyed vector of native functions, each named by its key.
pub(crate) fn natives(entries: impl IntoIterator<Item = (&'static str, Call, Valence)>) -> Value {
    let (keys, functions) = entries.into_iter().map(|(name, call, valence)| (name.into(), Value::Function(native(name, call, valence)))).unzip();
    crate::keyed::vector(keys, functions).expect("distinct keys")
}

const BUILTINS: &[(&str, Call, Valence)] = &[
    ("•c", Call::Value(case_convert), Ambivalent),
    ("•csv", Call::Value(crate::csv::parse), Ambivalent),
    ("•tocsv", Call::Value(crate::csv::serialize), Ambivalent),
    ("•json", Call::Value(crate::json::parse), Ambivalent),
    ("•tojson", Call::Value(crate::json::serialize), Ambivalent),
    ("•mime", Call::Mime, Ambivalent),
    ("•element", Call::Value(crate::xml::factory), Monadic),
    ("•xml", Call::Value(crate::xml::serialize), Monadic),
    ("•svg", Call::Value(crate::xml::svg), Ambivalent),
    ("•plot", Call::Value(crate::plot::plot), Ambivalent),
    ("•image", Call::Value(crate::image::image), Ambivalent),
    ("•vfi", Call::Value(crate::data::vfi), Ambivalent),
    ("•r", Call::Value(crate::regex::compile), Monadic),
    ("•distribution", Call::Value(crate::distribution::distribution), Ambivalent),
    ("•rand", Call::Value(crate::distribution::generator), Monadic),
    ("•nget", Call::Value(crate::data::read), Ambivalent),
    ("•nput", Call::Value(crate::data::write), Dyadic),
    ("•zip", Call::Value(crate::data::zip), Ambivalent),
    ("•hash", Call::Value(crate::data::hash), Ambivalent),
    ("•uuid", Call::Value(crate::data::uuid), Ambivalent),
    ("•ucs", Call::Value(unicode_convert), Ambivalent),
    ("•load", Call::Load, Monadic),
    ("•signal", Call::Value(signal), Monadic),
    ("•storage", Call::Value(storage), Monadic),
    ("•time", Call::Time, Ambivalent),
    ("•host", Call::Value(host), Monadic),
    ("•delay", Call::Value(delay), Monadic),
    ("•prefs", Call::Session(crate::display::prefs), Monadic),
    ("•nc", Call::Session(crate::Session::system_nc), Monadic),
    ("•nl", Call::Session(crate::Session::system_nl), Ambivalent),
    ("•src", Call::Session(crate::Session::system_src), Monadic),
    ("•ex", Call::Session(crate::Session::system_ex), Monadic),
];

pub(crate) fn names() -> impl Iterator<Item = &'static str> { BUILTINS.iter().map(|(name, ..)| *name) }

/// The table entry for system name `name`, ignoring case.
fn builtin(name: &str) -> Option<&'static (&'static str, Call, Valence)> { BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name)) }

/// The help block for system name `name`, ignoring case, from `nbs/system-functions.qmd`.
pub(crate) fn help(name: &str) -> Option<&'static str> { let (name, ..) = builtin(name)?; crate::inspection::page(name) }

pub(crate) fn lookup(name: &str) -> Option<Function> { let (name, call, valence) = builtin(name)?; Some(native(name, call.clone(), *valence)) }

/// The system functions that have inverses, each with its inverse. `•xml⁻¹` reads XML, and `•zip⁻¹` decompresses.
const INVERSES: &[(&str, Native)] = &[("•xml", crate::xml::parse), ("•zip", crate::data::unzip)];

/// `f⁻¹` for a system function `f` that has an inverse. It takes the arguments that `f` takes.
pub(crate) fn inverse(f: &SystemFunction, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let (_, call) = INVERSES.iter().find(|(name, _)| *name == f.name).ok_or_else(|| cx.domain_error("this function has no known inverse"))?;
    f.check(left, cx.span)?;
    call(left, right, cx)
}

/// The arguments that follow the program on the command line.
pub(crate) static ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// `•host name` gives the host fact called `name`, one of `"args"`, `"version"`, `"env"` and `"width"`.
fn host(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let fact = match crate::keyed::name(right).as_deref() {
        Some("args") => {
            let args = ARGS.get().map_or(&[][..], Vec::as_slice);
            Value::from_parts(vec![args.len()], args.iter().map(|a| crate::keyed::text(a)).collect(), crate::keyed::text(""))
        }
        Some("version") => Ok(crate::keyed::text(env!("CARGO_PKG_VERSION"))),
        Some("env") => {
            let (names, values) = std::env::vars_os().map(|(k, v)| (k.to_string_lossy().into(), crate::keyed::text(&v.to_string_lossy()))).unzip();
            crate::keyed::record(names, values)
        }
        Some("width") => terminal_width().map_or_else(|| Value::integers(vec![0], vec![]), |w| Value::number(w as f64)),
        _ => return Err(span.domain_error("•host takes \"args\", \"version\", \"env\" or \"width\"")),
    };
    fact.error_at(span, "invalid host fact")
}

/// The width of the terminal that standard output writes to. `None` without one.
#[cfg(unix)]
fn terminal_width() -> Option<usize> { rustix::termios::tcgetwinsize(std::io::stdout()).ok().map(|size| usize::from(size.ws_col)).filter(|&w| w > 0) }
#[cfg(not(unix))]
fn terminal_width() -> Option<usize> { None }

/// `•delay s` pauses for `s` seconds and gives the seconds it waited. `•delay ∞` waits until interrupted. Interrupts stop any delay.
fn delay(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use std::time::{Duration, Instant};
    let seconds = crate::primitive::real(right, span)?;
    if seconds.is_nan() || seconds < 0.0 { return Err(span.domain_error("•delay needs a nonnegative number of seconds")); }
    let start = Instant::now();
    let end = Duration::try_from_secs_f64(seconds).ok().and_then(|d| start.checked_add(d));
    loop {
        span.check()?;
        let now = Instant::now();
        if end.is_some_and(|end| now >= end) { break; }
        std::thread::sleep(end.map_or(Duration::MAX, |end| end - now).min(Duration::from_millis(10)));
    }
    Ok(Value::Number(start.elapsed().as_secs_f64().into()))
}

fn storage(_: Option<&Value>, right: &Value, _: &Context<'_>) -> Result<Value, Error> { Ok(crate::keyed::text(right.storage_name())) }

fn signal(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•signal needs an error name vector")); }
    let invalid = || span.domain_error("•signal needs an ordinary error name such as \"DOMAIN ERROR\"");
    let kind = match crate::keyed::name(right).ok_or_else(invalid)?.as_ref() {
        "SYNTAX ERROR" => ErrorKind::Syntax,
        "INDEX ERROR" => ErrorKind::Index,
        "RANK ERROR" => ErrorKind::Rank,
        "LENGTH ERROR" => ErrorKind::Length,
        "VALUE ERROR" => ErrorKind::Value,
        "LIMIT ERROR" => ErrorKind::Limit,
        "DOMAIN ERROR" => ErrorKind::Domain,
        _ => return Err(invalid()),
    };
    Err(span.error(kind, "explicitly signalled"))
}

fn case_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mode = match left {
        None => -3,
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().error_at(span, "•c mode must be 1, ¯1 or ¯3")?,
        _ => return Err(span.domain_error("•c needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.domain_error("•c mode must be 1, ¯1 or ¯3")); }
    let mapper = icu_casemap::CaseMapper::new();
    let case = |e: Value| {
        Ok(match e {
            Value::Character(c) => Value::Character(match mode {
                1 => mapper.simple_uppercase(c),
                -1 => mapper.simple_lowercase(c),
                _ => mapper.simple_fold(c),
            }),
            e => e,
        })
    };
    pervade(right, &case, &EmptyFill::Mapped, span)
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
        let codes = right.as_items().nonnegative_integers().map_err(|_| invalid())?;
        let chars: Vec<char> = match encoding.as_deref() {
            Some("UTF-8") => {
                let bytes: Vec<_> = codes.into_iter().map(u8::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                std::str::from_utf8(&bytes).map_err(|_| invalid())?.chars().collect()
            }
            Some("UTF-16") => {
                let units: Vec<_> = codes.into_iter().map(u16::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                char::decode_utf16(units).collect::<Result<_, _>>().map_err(|_| invalid())?
            }
            _ => codes.into_iter().map(|n| u32::try_from(n).ok().and_then(char::from_u32)).collect::<Option<_>>().ok_or_else(invalid)?,
        };
        if encoding.is_none() && right.is_atom() { return Ok(Value::Character(chars[0])); }
        Value::characters(shape(chars.len())?, chars)
    };
    result.and_then(|a| if encoding.is_none() { a.with_layout(right.layout().clone()) } else { Ok(a) }).error_at(span, "invalid Unicode result")
}
