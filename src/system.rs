use rustymath::{buffer::{DType, FloatWidth, View}, error::Kind};
use crate::{array::{generated_len, Items}, execution::Context, number::Numeric, primitive::{numeric, Primitive}, At, Error, ErrorAt, ErrorKind, Function, Value};
use std::borrow::Cow;

/// A system function implemented natively, taking an optional left argument and a right argument.
pub(crate) type Native = fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>;
/// A native called for its effect. It returns its result, and whether the result is shy, as an assignment's is.
pub(crate) type Effect = fn(Option<&Value>, &Value, &Context<'_>) -> Result<(Value, bool), Error>;

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(Native),
    /// A native that takes values that `∂` is differentiating, which other natives refuse. It never reads numbers or forces a pending array.
    Active(Native),
    /// A native called for its effect.
    Effect(Effect),
    Session(fn(&crate::Session, Option<&Value>, &Value, &crate::Span) -> Result<Value, Error>),
    Regex(std::sync::Arc<crate::host::Regex>, crate::regex::Operation),
    Distribution(std::sync::Arc<crate::distribution::Distribution>, crate::distribution::Operation),
    Generator(crate::distribution::Generator, crate::distribution::Draw),
    Load,
    Element(std::sync::Arc<str>),
    /// An edge-mode function from `•edge`, holding its mode.
    Edge(rustymath::structure::Edge),
    /// A `•canvas` drawing function, holding its JavaScript source.
    Canvas(std::sync::Arc<str>),
    /// A JavaScript function, which only the browser build has.
    #[cfg(web)]
    Js(std::sync::Arc<crate::js::JsFunction>),
    Mime,
    Time,
    /// `•prefs`, whose result is shy when it sets something.
    Prefs,
}

/// How many arguments a system function takes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Valence { Monadic, Dyadic, Ambivalent }
use Valence::{Ambivalent, Dyadic, Monadic};

/// What a system function takes as one item on the right. A right argument that holds several items maps over them, as Each does, with the left argument whole for each.
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
        let depth = || crate::primitive::depth(right);
        match self {
            Whole => false, Number => depth() > 0, Vector => depth() > 1,
            Text => depth() > 1 || (right.is_empty() && !matches!(right.as_items(), Items::Compact(View::Characters(_))))
        }
    }
}

/// A system function. `prototype` gives the prototype of its result. In prototype mode, a function that has one returns it instead of running. An empty mapped argument gives an empty result with that prototype.
#[derive(Clone, Debug)]
pub(crate) struct SystemFunction {
    pub name: &'static str, pub call: Call, pub valence: Valence, pub item: Item, pub prototype: Option<fn(&Context<'_>) -> Value>
}
/// System functions match when they share a name and their data matches. A generator matches only its own stream.
impl PartialEq for SystemFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && match (&self.call, &other.call) {
            (Call::Regex(x, a), Call::Regex(y, b)) => (x.as_str(), a) == (y.as_str(), b),
            (Call::Distribution(x, a), Call::Distribution(y, b)) => (x, a) == (y, b),
            (Call::Generator(x, a), Call::Generator(y, b)) => std::sync::Arc::ptr_eq(x, y) && a == b, (Call::Element(x), Call::Element(y)) => x == y, (Call::Edge(x), Call::Edge(y)) => x == y,
            (Call::Canvas(x), Call::Canvas(y)) => x == y, #[cfg(web)] (Call::Js(x), Call::Js(y)) => std::sync::Arc::ptr_eq(x, y),
            (x, y) => std::mem::discriminant(x) == std::mem::discriminant(y)
        }
    }
}

impl Valence {
    /// A call with the wrong number of arguments is a SYNTAX error.
    pub(crate) fn check(self, name: &str, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> {
        match (self, left) {
            (Monadic, Some(_)) => Err(span.error(ErrorKind::Syntax, format!("{name} is monadic"))),
            (Dyadic, None) => Err(span.error(ErrorKind::Syntax, format!("{name} needs a left argument"))), _ => Ok(())
        }
    }
}

impl SystemFunction {
    pub(crate) fn check(&self, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> { self.valence.check(self.name, left, span) }
    /// Asks the running levels before this function or its inverse runs (`Levels::effect`). A random draw runs in a batched run, from each cell's stream. Any other system function stops the run, because it would act once for all the cells, or read a batch through code with no rule for one.
    pub(crate) fn acts(&self, cx: &Context<'_>) -> Result<(), Error> {
        if self.call.draws() { return Ok(()); }
        cx.session.effect(crate::transform::Effect::Acts).at(cx.span)
    }
    /// The name that a factory made this function from, as `•element` makes one from a tag and `•edge` from a mode. Display writes it after the system name.
    pub(crate) fn argument(&self) -> Option<&str> { match &self.call { Call::Element(tag) => Some(tag), Call::Edge(mode) => Some(mode.name()), _ => None } }
}

impl Call {
    /// Whether this call draws random numbers: a generator's roll or deal, or a distribution's sample. A batched run lets a draw through, because it draws for every cell of the run (`distribution::drawn_cells`).
    pub(crate) fn draws(&self) -> bool { matches!(self, Call::Generator(..) | Call::Distribution(_, crate::distribution::Operation::Sample)) }
}

/// A function that calls `call` natively, taking its right argument whole. A generator's draw and a distribution's sample have a result prototype. In prototype mode they return it without drawing.
pub(crate) fn native(name: &'static str, call: Call, valence: Valence) -> Function {
    let draws = call.draws();
    Function::system(SystemFunction { name, call, valence, item: Whole, prototype: draws.then_some(float) })
}

/// A keyed vector of native functions, each named by its key.
pub(crate) fn natives(entries: impl IntoIterator<Item = (&'static str, Call, Valence)>) -> Value {
    let (keys, functions) = entries.into_iter().map(|(name, call, valence)| (name.into(), Value::Function(native(name, call, valence)))).unzip();
    crate::keyed::vector(keys, functions).expect("distinct keys")
}

// Result prototypes for the system table.
fn number(_: &Context<'_>) -> Value { Value::integer(0) }
fn float(_: &Context<'_>) -> Value { Value::Number(Numeric::current().float(0.0)) }
/// Times are 64-bit, as `•date` and `•time` give them.
fn seconds(_: &Context<'_>) -> Value { Value::Number(crate::Number::float(0.0, FloatWidth::F64)) }
fn text(_: &Context<'_>) -> Value { crate::keyed::text("") }
fn texts(_: &Context<'_>) -> Value { crate::keyed::texts(&[0], Vec::new()).expect("an empty list of text") }
fn numbers(_: &Context<'_>) -> Value { Value::integers(vec![0], Vec::new()).expect("an empty vector") }
fn element(_: &Context<'_>) -> Value { crate::xml::element_function("".into()) }
fn canvas(_: &Context<'_>) -> Value { crate::canvas::function("".into()) }
fn valid_numbers(cx: &Context<'_>) -> Value { Value::new(vec![2], vec![numbers(cx), numbers(cx)]).expect("a pair") }
fn record(_: &Context<'_>) -> Value { crate::keyed::empty_record() }

/// A system table entry: a name, call, valence, right-argument item and result prototype. In prototype mode, a function with a prototype returns it instead of running.
type Builtin = (&'static str, Call, Valence, Item, Option<fn(&Context<'_>) -> Value>);
/// Each system function's entry.
const BUILTINS: &[Builtin] = &[
    ("•c", Call::Value(case_convert), Ambivalent, Whole, Some(text)), ("•csv", Call::Value(crate::csv::parse), Ambivalent, Text, Some(record)),
    ("•json", Call::Value(crate::json::parse), Ambivalent, Text, Some(number)), ("•mime", Call::Mime, Ambivalent, Whole, Some(record)),
    ("•element", Call::Value(crate::xml::factory), Monadic, Text, Some(element)), ("•xml", Call::Value(crate::xml::parse), Monadic, Text, Some(record)),
    ("•svg", Call::Value(crate::xml::svg), Ambivalent, Whole, Some(record)), ("•plot", Call::Value(crate::plot::plot), Ambivalent, Whole, Some(record)),
    ("•image", Call::Value(crate::image::image), Ambivalent, Vector, Some(float)), ("•canvas", Call::Value(crate::canvas::factory), Monadic, Text, Some(canvas)),
    #[cfg(web)] ("•js", Call::Value(crate::js::function), Monadic, Text, Some(crate::js::prototype)),
    ("•vfi", Call::Value(crate::data::vfi), Ambivalent, Text, Some(valid_numbers)), ("•r", Call::Value(crate::regex::compile), Monadic, Text, Some(record)),
    ("•distribution", Call::Value(crate::distribution::distribution), Ambivalent, Text, Some(record)),
    ("•rand", Call::Value(crate::distribution::generator), Monadic, Whole, Some(record)), ("•nget", Call::Value(crate::data::read), Ambivalent, Text, Some(text)),
    ("•nput", Call::Effect(crate::data::write), Dyadic, Whole, Some(number)), ("•fetch", Call::Value(crate::data::fetch), Ambivalent, Text, Some(record)),
    ("•deflate", Call::Value(crate::data::inflate), Ambivalent, Vector, Some(numbers)), ("•hash", Call::Value(crate::data::hash), Ambivalent, Vector, Some(text)),
    ("•uuid", Call::Value(crate::data::uuid), Ambivalent, Number, Some(text)), ("•normalize", Call::Value(normalize), Ambivalent, Text, Some(text)),
    ("•decompose", Call::Value(crate::primitive::decompose), Dyadic, Whole, Some(record)), ("•ucs", Call::Value(unicode_convert), Ambivalent, Vector, Some(numbers)),
    ("•load", Call::Load, Monadic, Text, Some(record)), ("•signal", Call::Value(signal), Ambivalent, Whole, Some(number)),
    ("•storage", Call::Active(storage), Monadic, Whole, Some(record)), ("•float", Call::Active(to_float), Ambivalent, Whole, None),
    ("•time", Call::Time, Ambivalent, Whole, Some(seconds)), ("•host", Call::Value(host), Monadic, Text, Some(text)), #[cfg(not(web))]
    ("•delay", Call::Effect(delay), Monadic, Whole, Some(seconds)), ("•date", Call::Value(crate::date::read), Ambivalent, Whole, Some(seconds)), #[cfg(not(web))]
    ("•path", Call::Value(crate::files::path), Ambivalent, Whole, Some(record)), #[cfg(not(web))]
    ("•metadata", Call::Value(crate::files::metadata), Monadic, Whole, Some(record)), #[cfg(not(web))]
    ("•readdir", Call::Value(crate::files::readdir), Ambivalent, Text, Some(record)), #[cfg(not(web))]
    ("•copy", Call::Effect(crate::files::copy), Dyadic, Whole, Some(text)), #[cfg(not(web))] ("•rename", Call::Effect(crate::files::rename), Dyadic, Whole, Some(text)),
    #[cfg(not(web))] ("•remove", Call::Effect(crate::files::remove), Ambivalent, Text, Some(text)), #[cfg(not(web))]
    ("•mkdir", Call::Effect(crate::files::mkdir), Ambivalent, Text, Some(text)), ("•prefs", Call::Prefs, Monadic, Whole, Some(record)),
    ("•nc", Call::Session(crate::Session::system_nc), Monadic, Text, Some(number)), ("•nl", Call::Session(crate::Session::system_nl), Ambivalent, Whole, Some(texts)),
    ("•src", Call::Session(crate::Session::system_src), Monadic, Text, Some(text)), ("•ex", Call::Session(crate::Session::system_ex), Monadic, Text, Some(number)),
    ("•literal", Call::Session(crate::Session::system_literal), Monadic, Text, Some(number)),
    ("•explain", Call::Session(crate::Session::system_explain), Monadic, Text, Some(record)), ("•memory", Call::Active(memory), Monadic, Whole, Some(record)),
    ("•detach", Call::Active(detach), Monadic, Whole, None), ("•index", Call::Value(index), Ambivalent, Number, Some(numbers)),
    ("•edge", Call::Value(edge), Ambivalent, Text, Some(number)),
];

/// Every system function's name.
pub fn names() -> impl Iterator<Item = &'static str> { BUILTINS.iter().map(|(name, ..)| *name) }

/// The table entry for system name `name`, ignoring case.
fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name))
}

/// The help page for system name `name`, ignoring case, from `nbs/system`.
pub(crate) fn help(name: &str) -> Option<&'static str> { crate::inspection::page(builtin(name)?.0) }

/// The system function named `name`, ignoring case.
pub(crate) fn lookup(name: &str) -> Option<Function> {
    let (name, call, valence, item, prototype) = builtin(name)?;
    Some(Function::system(SystemFunction { name, call: call.clone(), valence: *valence, item: *item, prototype: *prototype }))
}

/// The system functions with an inverse, each with its inverse and the inverse's valence. A format function reads its format, and its inverse writes it.
const INVERSES: &[(&str, Native, Valence)] = &[
    ("•csv", crate::csv::serialize, Ambivalent), ("•json", crate::json::serialize, Ambivalent), ("•xml", crate::xml::serialize, Monadic),
    ("•image", crate::image::encode, Ambivalent), ("•deflate", crate::data::deflate, Ambivalent), ("•date", crate::date::write, Ambivalent),
    #[cfg(not(web))] ("•path", crate::files::join, Monadic), ("•literal", write_literal, Monadic), ("•ucs", unicode_convert, Ambivalent),
    ("•decompose", crate::primitive::recompose, Dyadic),
];

/// `f⁻¹` for a system function `f` that has an inverse.
pub(crate) fn inverse(f: &SystemFunction, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    use crate::distribution::Operation::{Cdf, Quantile};
    f.acts(cx)?;
    if let Call::Distribution(d, op @ (Cdf | Quantile)) = &f.call {
        return crate::distribution::call(d, if matches!(op, Cdf) { Quantile } else { Cdf }, left, right, cx);
    }
    let (_, call, valence) = INVERSES.iter().find(|(name, ..)| *name == f.name).ok_or_else(|| cx.domain_error("this function has no known inverse"))?;
    valence.check(&format!("{}⁻¹", f.name), left, cx.span)?;
    call(left, right, cx)
}

const HOST_FACTS: &[&str] = &["args", "version", "env", "width", "height", #[cfg(not(web))] "cwd", #[cfg(not(web))] "temp",];

/// `•host name` gives the host fact called `name`: the command-line `"args"`, BPL's `"version"`, the `"env"` record, the terminal's `"width"` and `"height"`, the working directory `"cwd"`, and `"temp"`, the directory for temporary files.
fn host(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let fact = match crate::keyed::name(right).as_deref() {
        Some("args") => {
            let args = &span.session.args;
            crate::keyed::texts(&[args.len()], args.iter().map(|a| crate::keyed::text(a)).collect())
        }
        Some("version") => Ok(crate::keyed::text(env!("CARGO_PKG_VERSION"))),
        Some("env") => {
            let (names, values) = crate::host::environment().into_iter().map(|(k, v)| (k.into(), crate::keyed::text(&v))).unzip();
            crate::keyed::vector(names, values)
        }
        Some(fact @ ("width" | "height")) => match terminal_size() {
            Some((rows, columns)) => Ok(Value::integer((if fact == "height" { rows } else { columns }) as i64)), None => Value::integers(vec![0], vec![])
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
    Ok((Value::Number(crate::Number::float(start.elapsed().as_secs_f64(), FloatWidth::F64)), true))
}

/// `•storage Y`: how `Y`'s items are held, as a record. `kind` names the storage (`Value::storage_kind`). Under static kinds, an integer's kind names its width as well, `int32` or `int64`, because the width is part of the kind there. `pending` is true while the items wait in a pending call, and `view` while `Y` reads another array's items. `active` counts the `∂` levels whose tags `Y` carries. Reading no items, it never forces a pending array.
fn storage(_: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let flag = |b: bool| Value::Number(crate::Number::from_bool(b));
    let kind = right.storage_kind(Numeric::current());
    let values = vec![crate::keyed::text(kind), flag(right.template().pending().is_some()), flag(right.template().is_view()), Value::integer(crate::derivative::levels(right) as i64)];
    crate::keyed::vector(["kind", "pending", "view", "active"].map(Into::into).to_vec(), values).error_at(cx, "invalid storage record")
}

/// `•memory x`: what this thread holds, and what `x` keeps alive, as a record. `records` counts the live record nodes of `∂`, which each `∂` frees when it returns, and `pending` the live pending arrays. Each counts the nodes this thread made, wherever they are dropped. `bytes` counts the allocations that `x` keeps alive (`Value::footprint`). It reads no items and never forces a pending array. It takes active values under `∂`.
fn memory(_: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    use crate::explain::{live, Counted};
    let counts = [live(Counted::Record), live(Counted::Pending), right.footprint()].map(|n| Value::integer(n as i64)).to_vec();
    crate::keyed::vector(["records", "pending", "bytes"].map(Into::into).to_vec(), counts).error_at(cx, "invalid memory record")
}

/// `•detach Y`: `Y` with every `∂` tag stripped. No derivative flows back through the result. Outside `∂`, it gives `Y`.
fn detach(_: Option<&Value>, right: &Value, _: &Context<'_>) -> Result<Value, Error> { Ok(crate::derivative::detach(right)) }

/// The separator between a fresh name's stem and its random part. Display cuts a hidden name after it.
pub(crate) const FRESH: char = '·';
/// `•index n` is `⍳n` along a fresh hidden axis name. The name is `_`, the left argument's text as a readable stem, `·`, and 64 random bits as 13 base-32 digits.
fn index(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    const DIGITS: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let stem: std::sync::Arc<str> = match left {
        None => "".into(),
        Some(a) => crate::keyed::name(a).ok_or_else(|| span.domain_error("•index takes text as its stem"))?,
    };
    let mut bits: u64 = rand::random();
    let random: String = (0..13).map(|_| { let digit = DIGITS[(bits % 32) as usize] as char; bits /= 32; digit }).collect();
    Primitive::Iota.call(None, right, span)?.with_axis_names(vec![Some(format!("_{stem}{FRESH}{random}").into())]).error_at(span, "invalid index")
}

/// The edge mode that `name` names, one of rustymath's `structure::Edge::MODES`, as `•edge` and window options read it.
pub(crate) fn edge_mode(name: &Value, span: &Context<'_>) -> Result<rustymath::structure::Edge, Error> {
    use rustymath::structure::Edge;
    crate::keyed::name(name).and_then(|m| Edge::named(&m)).ok_or_else(|| {
        let names: Vec<_> = Edge::MODES.iter().map(|(name, _)| *name).collect();
        let (last, rest) = names.split_last().expect("modes");
        span.domain_error(format!("the edge modes are {} and {last}", rest.join(", ")))
    })
}
/// `•edge mode`: the function `n f p` that maps positions `p` on an axis of `n` items by the edge mode named `mode` (`edge_mode`). `n •edge p` maps them by the fill mode.
fn edge(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(n) = left else { return Ok(Value::Function(native("•edge", Call::Edge(edge_mode(right, span)?), Dyadic))) };
    edged(rustymath::structure::Edge::Fill, n, right, span)
}
/// The position on an axis of `n` items that each position in `p` reads under edge mode `mode`, as rustymath's `structure::Edge` gives it. Fill gives NaN for a position outside the axis, which reads the fill, so its result is 64-bit floats, whatever the float pref. The other modes give integers, unless the axis is empty. A NaN in `p` stays NaN.
pub(crate) fn edged(mode: rustymath::structure::Edge, n: &Value, p: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let n = n.as_number().and_then(|n| n.nonnegative_integer().ok()).ok_or_else(|| span.domain_error("•edge takes a nonnegative whole length"))?;
    let positions = p.as_items().positions().map_err(|_| span.domain_error("•edge takes whole positions"))?;
    let mapped: Vec<Option<usize>> = positions.iter().map(|q| q.and_then(|q| mode.position(q, n))).collect();
    let result = if mode == rustymath::structure::Edge::Fill || mapped.contains(&None) {
        let floats = mapped.iter().map(|q| q.map_or(f64::NAN, |q| q as f64));
        if p.is_atom() { return Ok(Value::Number(crate::Number::float(floats.last().unwrap_or(f64::NAN), FloatWidth::F64))); }
        p.layout().floats(FloatWidth::F64, floats)
    } else {
        if p.is_atom() { return Ok(Value::integer(mapped[0].unwrap_or(0) as i64)); }
        p.layout().integers(mapped.iter().map(|q| q.unwrap_or(0) as i64).collect())
    };
    result.error_at(span, "invalid positions")
}
/// `width •float Y`: the numbers of `Y` as floats of `width` bits, 16, 32 or 64, with the arrays nested in `Y` converted too. Under `∂`, a tangent converts with its primal. `•float Y`: the width of `Y`'s floats, as a number that `•float`'s left argument takes. It's the widest float width among `Y`'s numbers, nested arrays included, or the default width, which the `float` pref sets, when they're all exact.
fn to_float(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(w) = left else {
        let width = widest(right).error_at(span, "•float takes numbers")?.unwrap_or(Numeric::current().width);
        return Ok(Value::integer(width.bits()));
    };
    let width = w.as_number().and_then(|n| n.integer().ok()).and_then(FloatWidth::of_bits).ok_or(Kind::Domain).error_at(span, "•float takes a width of 16, 32 or 64")?;
    crate::derivative::to_float_width(right, width, span)
}

/// The widest float width among the numbers in `v`, nested arrays and a dual's primal included, or `None` when they're all exact. Characters and functions are DOMAIN.
fn widest(v: &Value) -> Result<Option<FloatWidth>, ErrorKind> {
    let v = v.template();
    match v.dtype() {
        Some(DType::Character) => Err(Kind::Domain.into()),
        Some(_) => Ok(v.float_width()),
        None if matches!(v, Value::Number(_)) => Ok(v.as_number().and_then(|n| n.float_width())),
        None if matches!(v, Value::Array(_)) => {
            let items = if v.is_empty() { vec![v.prototype()] } else { v.elements().collect() };
            items.iter().map(widest).try_fold(None, |w, e| Ok(w.max(e?)))
        }
        None => Err(Kind::Domain.into()),
    }
}

/// `•signal kind` raises an error of the kind that `kind` names, and `message •signal kind` gives it a message. `kind` can also be a caught error, such as `$e`, which raises its kind and message again.
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
        None => -3, Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().error_at(span, "•c mode must be 1, ¯1 or ¯3")?,
        _ => return Err(span.domain_error("•c needs one case mode"))
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.domain_error("•c mode must be 1, ¯1 or ¯3")); }
    let text: fn(&str) -> String = match mode { 1 => str::to_uppercase, -1 => str::to_lowercase, _ => |s| s.to_uppercase().to_lowercase() };
    cased(right, text, span)
}

/// `value` with each character vector mapped as a string by `text`, and each other character mapped on its own. A character whose mapping has more than one character stays as it is.
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
            None => Value::new(value.shape(), value.elements().map(|e| cased(&e, text, span)).collect::<Result<_, _>>()?).error_at(span, "invalid result")?,
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
    span.formats(None, right)?;
    if right.holds_function() { return Err(span.domain_error("•literal⁻¹ writes data, not functions")); }
    Ok(crate::keyed::text(&right.source(crate::display::Elide { plain: None, ..crate::display::Elide::NONE })))
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
        if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "encoded •ucs needs a vector")); }
        Some(text)
    } else { None };
    let shape = |len: usize| {
        let shape = if encoding.is_some() { vec![len] } else { right.shape().to_vec() };
        generated_len(&shape).map(|_| shape).at(span)
    };
    let result = if matches!(right.prototype(), Value::Character(_)) {
        let chars: Cow<[char]> = match right.as_items() {
            Items::Compact(View::Characters(cs)) => Cow::Borrowed(cs),
            Items::Values(vs) => Cow::Owned(vs.iter().map(|v| if let Value::Character(c) = v { Ok(*c) } else { Err(invalid()) }).collect::<Result<_, _>>()?),
            _ => return Err(invalid()),
        };
        let codes: Vec<i64> = match encoding.as_deref() {
            Some("UTF-8") => chars.iter().collect::<String>().bytes().map(i64::from).collect(),
            Some("UTF-16") => chars.iter().collect::<String>().encode_utf16().map(i64::from).collect(),
            _ => chars.iter().map(|&c| i64::from(u32::from(c))).collect(),
        };
        if encoding.is_none() && right.is_atom() { return Ok(Value::integer(codes[0])); }
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
