use crate::{
    array::{generated_len, Items},
    eval::Operand,
    execution::Context,
    primitive::{integer, numeric, pervade, EmptyFill},
    Error, ErrorAt, ErrorKind, Function, Value,
};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>),
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

enum Builtin { Text(&'static str), Function(Call, Valence) }

const BUILTINS: &[(&str, Builtin)] = &[
    ("•a", Builtin::Text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
    ("•d", Builtin::Text("0123456789")),
    ("•c", Builtin::Function(Call::Value(case_convert), Ambivalent)),
    ("•csv", Builtin::Function(Call::Value(crate::csv::parse), Ambivalent)),
    ("•tocsv", Builtin::Function(Call::Value(crate::csv::serialize), Ambivalent)),
    ("•json", Builtin::Function(Call::Value(crate::json::parse), Ambivalent)),
    ("•tojson", Builtin::Function(Call::Value(crate::json::serialize), Ambivalent)),
    ("•mime", Builtin::Function(Call::Mime, Ambivalent)),
    ("•element", Builtin::Function(Call::Value(crate::xml::factory), Monadic)),
    ("•xml", Builtin::Function(Call::Value(crate::xml::serialize), Monadic)),
    ("•svg", Builtin::Function(Call::Value(crate::xml::svg), Ambivalent)),
    ("•plot", Builtin::Function(Call::Value(crate::plot::plot), Ambivalent)),
    ("•image", Builtin::Function(Call::Value(crate::image::image), Ambivalent)),
    ("•vfi", Builtin::Function(Call::Value(crate::data::vfi), Ambivalent)),
    ("•r", Builtin::Function(Call::Value(crate::regex::compile), Monadic)),
    ("•normal", Builtin::Function(Call::Value(crate::distribution::normal), Monadic)),
    ("•uniform", Builtin::Function(Call::Value(crate::distribution::uniform), Monadic)),
    ("•beta", Builtin::Function(Call::Value(crate::distribution::beta), Monadic)),
    ("•bernoulli", Builtin::Function(Call::Value(crate::distribution::bernoulli), Monadic)),
    ("•binomial", Builtin::Function(Call::Value(crate::distribution::binomial), Monadic)),
    ("•cauchy", Builtin::Function(Call::Value(crate::distribution::cauchy), Monadic)),
    ("•chisquared", Builtin::Function(Call::Value(crate::distribution::chisquared), Monadic)),
    ("•exponential", Builtin::Function(Call::Value(crate::distribution::exponential), Monadic)),
    ("•fisher", Builtin::Function(Call::Value(crate::distribution::fisher), Monadic)),
    ("•gamma", Builtin::Function(Call::Value(crate::distribution::gamma), Monadic)),
    ("•inversegamma", Builtin::Function(Call::Value(crate::distribution::inversegamma), Monadic)),
    ("•laplace", Builtin::Function(Call::Value(crate::distribution::laplace), Monadic)),
    ("•lognormal", Builtin::Function(Call::Value(crate::distribution::lognormal), Monadic)),
    ("•logistic", Builtin::Function(Call::Value(crate::distribution::logistic_distribution), Monadic)),
    ("•poisson", Builtin::Function(Call::Value(crate::distribution::poisson), Monadic)),
    ("•student", Builtin::Function(Call::Value(crate::distribution::student), Monadic)),
    ("•weibull", Builtin::Function(Call::Value(crate::distribution::weibull), Monadic)),
    ("•rand", Builtin::Function(Call::Value(crate::distribution::generator), Monadic)),
    ("•nget", Builtin::Function(Call::Value(crate::data::read), Ambivalent)),
    ("•nput", Builtin::Function(Call::Value(crate::data::write), Dyadic)),
    ("•ucs", Builtin::Function(Call::Value(unicode_convert), Ambivalent)),
    ("•load", Builtin::Function(Call::Load, Monadic)),
    ("•signal", Builtin::Function(Call::Value(signal), Monadic)),
    ("•storage", Builtin::Function(Call::Value(storage), Monadic)),
    ("•time", Builtin::Function(Call::Time, Ambivalent)),
    ("•prefs", Builtin::Function(Call::Session(crate::display::prefs), Monadic)),
    ("•nc", Builtin::Function(Call::Session(crate::Session::system_nc), Monadic)),
    ("•nl", Builtin::Function(Call::Session(crate::Session::system_nl), Ambivalent)),
    ("•src", Builtin::Function(Call::Session(crate::Session::system_src), Monadic)),
    ("•ex", Builtin::Function(Call::Session(crate::Session::system_ex), Monadic)),
];

pub(crate) fn names() -> impl Iterator<Item = &'static str> { BUILTINS.iter().map(|(name, ..)| *name) }

/// The help for system name `name`, ignoring case: its block of `nbs/system-functions.qmd`.
pub(crate) fn help(name: &str) -> Option<&'static str> {
    let (name, _) = BUILTINS.iter().find(|(key, _)| key.eq_ignore_ascii_case(name))?;
    crate::inspection::page(name)
}

pub(crate) fn lookup(name: &str) -> Option<Operand> {
    let &(name, ref builtin) = BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name))?;
    Some(match builtin {
        Builtin::Text(text) => Operand::Value(crate::keyed::text(text)),
        Builtin::Function(call, valence) => Operand::Function(Function::system(SystemFunction { name, call: call.clone(), valence: *valence })),
    })
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
