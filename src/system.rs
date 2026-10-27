use crate::execution::check;
use rustymath::{buffer::{DType, FloatWidth, View}, error::Kind, layout::FRESH};
use crate::session;
use crate::{array::generated_len, Span, number::Numeric, primitive::{numeric, Extension::{self, Items, Whole}, Item::{Number, Text, Vector}, Primitive}, Described, Error, ErrorKind, Function, Value};
use std::borrow::Cow;

/// A system function implemented natively, taking an optional left argument and a right argument.
pub(crate) type Native = fn(Option<&Value>, &Value, &Span) -> Result<Value, Error>;
/// A native called for its effect. It returns its result, and whether the result is shy, as an assignment's is.
pub(crate) type Effect = fn(Option<&Value>, &Value, &Span) -> Result<(Value, bool), Error>;

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

/// A system function: its name, call, valence, extension and result prototype, and its inverse with the inverse's valence. In prototype mode, a function with a prototype returns it instead of running. An empty mapped argument gives an empty result with that prototype. A format function reads its format, and its inverse writes it. `entry` is the function's row in the system table, which `Entry::function` sets. It's `None` in the table itself, and for a function that a factory made, as `•element` makes one from a tag, even though such a function shares its factory's name.
#[derive(Clone, Debug)]
pub(crate) struct SystemFunction {
    pub name: &'static str, pub call: Call, pub valence: Valence, pub extension: Extension, pub prototype: Option<fn(&Span) -> Value>, pub inverse: Option<(Native, Valence)>, pub entry: Option<Entry>
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
    pub(crate) fn check(self, name: &str, left: Option<&Value>) -> Result<(), Error> {
        match (self, left) {
            (Monadic, Some(_)) => Err(Error::new(ErrorKind::Syntax, format!("{name} is monadic"))),
            (Dyadic, None) => Err(Error::new(ErrorKind::Syntax, format!("{name} needs a left argument"))), _ => Ok(())
        }
    }
}

impl SystemFunction {
    pub(crate) fn check(&self, left: Option<&Value>) -> Result<(), Error> { self.valence.check(self.name, left) }
    /// The name that a factory made this function from, as `•element` makes one from a tag and `•edge` from a mode. Display writes it after the system name.
    pub(crate) fn argument(&self) -> Option<&str> { match &self.call { Call::Element(tag) => Some(tag), Call::Edge(mode) => Some(mode.name()), _ => None } }
}

impl Call {
    /// Whether this call draws random numbers: a generator's roll or deal, or a distribution's sample. A batched run leaves a draw to its own code, which draws for every cell of the run (`distribution::drawn_cells`).
    pub(crate) fn draws(&self) -> bool { matches!(self, Call::Generator(..) | Call::Distribution(_, crate::distribution::Operation::Sample)) }
}

/// A function that calls `call` natively, taking its right argument whole. A generator's draw and a distribution's sample have a result prototype. In prototype mode they return it without drawing.
pub(crate) fn native(name: &'static str, call: Call, valence: Valence) -> Function {
    let draws = call.draws();
    Function::system(SystemFunction { name, call, valence, extension: Whole, prototype: draws.then_some(float), inverse: None, entry: None })
}

/// A keyed vector of native functions, each named by its key.
pub(crate) fn natives(entries: impl IntoIterator<Item = (&'static str, Call, Valence)>) -> Value {
    let (keys, functions) = entries.into_iter().map(|(name, call, valence)| (name.into(), Value::Function(native(name, call, valence)))).unzip();
    crate::keyed::vector(keys, functions).expect("distinct keys")
}

// Result prototypes for the system table.
fn number(_: &Span) -> Value { Value::integer(0) }
fn float(_: &Span) -> Value { Value::Number(Numeric::current().float(0.0)) }
/// Times are 64-bit, as `•date` and `•time` give them.
fn seconds(_: &Span) -> Value { Value::Number(crate::Number::float(0.0, FloatWidth::F64)) }
fn text(_: &Span) -> Value { crate::keyed::text("") }
fn texts(_: &Span) -> Value { crate::keyed::texts(&[0], Vec::new()).expect("an empty list of text") }
fn numbers(_: &Span) -> Value { Value::integers(vec![0], Vec::new()).expect("an empty vector") }
fn element(_: &Span) -> Value { crate::xml::element_function("".into()) }
fn canvas(_: &Span) -> Value { crate::canvas::function("".into()) }
fn valid_numbers(span: &Span) -> Value { Value::new(vec![2], vec![numbers(span), numbers(span)]).expect("a pair") }
fn record(_: &Span) -> Value { crate::keyed::empty_record() }

/// A row of the table. A system function that maps over several items has a result prototype, which an empty mapped argument gives.
const fn row(name: &'static str, call: Call, valence: Valence, extension: Extension, prototype: Option<fn(&Span) -> Value>, inverse: Option<(Native, Valence)>) -> SystemFunction {
    assert!(matches!(extension, Whole) || prototype.is_some(), "a system function that maps over items needs a result prototype");
    SystemFunction { name, call, valence, extension, prototype, inverse, entry: None }
}

/// Each system function's row.
const BUILTINS: &[SystemFunction] = &[
    row("•c", Call::Value(case_convert), Ambivalent, Whole, Some(text), None),
    row("•csv", Call::Value(crate::csv::parse), Ambivalent, Items(Text), Some(record), Some((crate::csv::serialize, Ambivalent))),
    row("•json", Call::Value(crate::json::parse), Ambivalent, Items(Text), Some(number), Some((crate::json::serialize, Ambivalent))),
    row("•mime", Call::Mime, Ambivalent, Whole, Some(record), None),
    row("•element", Call::Value(crate::xml::factory), Monadic, Items(Text), Some(element), None),
    row("•xml", Call::Value(crate::xml::parse), Monadic, Items(Text), Some(record), Some((crate::xml::serialize, Monadic))),
    row("•svg", Call::Value(crate::xml::svg), Ambivalent, Whole, Some(record), None),
    row("•plot", Call::Value(crate::plot::plot), Ambivalent, Whole, Some(record), None),
    row("•image", Call::Value(crate::image::image), Ambivalent, Items(Vector), Some(float), Some((crate::image::encode, Ambivalent))),
    row("•canvas", Call::Value(crate::canvas::factory), Monadic, Items(Text), Some(canvas), None),
    #[cfg(web)] row("•js", Call::Value(crate::js::function), Monadic, Items(Text), Some(crate::js::prototype), None),
    row("•vfi", Call::Value(crate::data::vfi), Ambivalent, Items(Text), Some(valid_numbers), None),
    row("•r", Call::Value(crate::regex::compile), Monadic, Items(Text), Some(record), None),
    row("•distribution", Call::Value(crate::distribution::distribution), Ambivalent, Items(Text), Some(record), None),
    row("•rand", Call::Value(crate::distribution::generator), Monadic, Whole, Some(record), None),
    row("•nget", Call::Value(crate::data::read), Ambivalent, Items(Text), Some(text), None),
    row("•nput", Call::Effect(crate::data::write), Dyadic, Whole, Some(number), None),
    row("•fetch", Call::Value(crate::data::fetch), Ambivalent, Items(Text), Some(record), None),
    row("•deflate", Call::Value(crate::data::inflate), Ambivalent, Items(Vector), Some(numbers), Some((crate::data::deflate, Ambivalent))),
    row("•hash", Call::Value(crate::data::hash), Ambivalent, Items(Vector), Some(text), None),
    row("•uuid", Call::Value(crate::data::uuid), Ambivalent, Items(Number), Some(text), None),
    row("•normalize", Call::Value(normalize), Ambivalent, Items(Text), Some(text), None),
    row("•decompose", Call::Value(crate::primitive::decompose), Dyadic, Whole, Some(record), Some((crate::primitive::recompose, Dyadic))),
    row("•ucs", Call::Value(unicode_convert), Ambivalent, Items(Vector), Some(numbers), Some((unicode_convert, Ambivalent))),
    row("•load", Call::Load, Monadic, Items(Text), Some(record), None),
    row("•signal", Call::Value(signal), Ambivalent, Whole, Some(number), None),
    row("•storage", Call::Active(storage), Monadic, Whole, Some(record), None),
    row("•float", Call::Active(to_float), Ambivalent, Whole, None, None),
    row("•time", Call::Time, Ambivalent, Whole, Some(seconds), None),
    row("•host", Call::Value(host), Monadic, Items(Text), Some(text), None),
    #[cfg(not(web))] row("•delay", Call::Effect(delay), Monadic, Whole, Some(seconds), None),
    row("•date", Call::Value(crate::date::read), Ambivalent, Whole, Some(seconds), Some((crate::date::write, Ambivalent))),
    #[cfg(not(web))] row("•path", Call::Value(crate::files::path), Ambivalent, Whole, Some(record), Some((crate::files::join, Monadic))),
    #[cfg(not(web))] row("•metadata", Call::Value(crate::files::metadata), Monadic, Whole, Some(record), None),
    #[cfg(not(web))] row("•readdir", Call::Value(crate::files::readdir), Ambivalent, Items(Text), Some(record), None),
    #[cfg(not(web))] row("•copy", Call::Effect(crate::files::copy), Dyadic, Whole, Some(text), None),
    #[cfg(not(web))] row("•rename", Call::Effect(crate::files::rename), Dyadic, Whole, Some(text), None),
    #[cfg(not(web))] row("•remove", Call::Effect(crate::files::remove), Ambivalent, Items(Text), Some(text), None),
    #[cfg(not(web))] row("•mkdir", Call::Effect(crate::files::mkdir), Ambivalent, Items(Text), Some(text), None),
    row("•prefs", Call::Prefs, Monadic, Whole, Some(record), None),
    row("•nc", Call::Session(crate::Session::system_nc), Monadic, Items(Text), Some(number), None),
    row("•nl", Call::Session(crate::Session::system_nl), Ambivalent, Whole, Some(texts), None),
    row("•src", Call::Session(crate::Session::system_src), Monadic, Items(Text), Some(text), None),
    row("•ex", Call::Session(crate::Session::system_ex), Monadic, Items(Text), Some(number), None),
    row("•literal", Call::Session(crate::Session::system_literal), Monadic, Items(Text), Some(number), Some((write_literal, Monadic))),
    row("•explain", Call::Session(crate::Session::system_explain), Monadic, Items(Text), Some(record), None),
    row("•memory", Call::Active(memory), Monadic, Whole, Some(record), None),
    row("•detach", Call::Active(detach), Monadic, Whole, None, None),
    row("•index", Call::Value(index), Ambivalent, Whole, None, None),
    row("•edge", Call::Value(edge), Ambivalent, Items(Text), Some(number), None),
];

/// A system function's entry in the table, which the parser resolves a system name to.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Entry(u8);

const _: () = assert!(BUILTINS.len() <= u8::MAX as usize, "an `Entry` holds a table position in a `u8`");

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.name()) }
}

impl Entry {
    /// `•ucs`, which `•literal` writes a control character with.
    pub(crate) const UCS: Self = Self::named("•ucs");
    /// `•prefs`, which heads a file when called with a record literal.
    pub(crate) const PREFS: Self = Self::named("•prefs");
    /// `•decompose`, which Rank stacks over matrices.
    pub(crate) const DECOMPOSE: Self = Self::named("•decompose");
    /// The entry that the table spells `name`, found when the code compiles.
    const fn named(name: &str) -> Self {
        let mut i = 0;
        while i < BUILTINS.len() {
            if same(BUILTINS[i].name.as_bytes(), name.as_bytes()) { return Self(i as u8); }
            i += 1;
        }
        panic!("the system table has no row with this name")
    }
    /// The entry for system name `name`, ignoring case.
    pub(crate) fn find(name: &str) -> Option<Self> { BUILTINS.iter().position(|row| row.name.eq_ignore_ascii_case(name)).map(|i| Self(i as u8)) }
    /// Every entry, in table order.
    pub(crate) fn all() -> impl Iterator<Item = Self> { (0..BUILTINS.len()).map(|i| Self(i as u8)) }
    fn row(self) -> &'static SystemFunction { &BUILTINS[self.0 as usize] }
    /// The system function's name, as the table spells it.
    pub(crate) fn name(self) -> &'static str { self.row().name }
    /// The system function, built anew for the table that `Builtin::function` reads.
    pub(crate) fn built(self) -> Function { Function::system(SystemFunction { entry: Some(self), ..self.row().clone() }) }
}

/// Whether two byte strings are equal, as a constant can test it.
const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}

/// `f⁻¹` for a system function `f` that has an inverse.
pub(crate) fn inverse(f: &SystemFunction, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use crate::distribution::Operation::{Cdf, Quantile};
    if let Call::Distribution(d, op @ (Cdf | Quantile)) = &f.call {
        return crate::distribution::call(d, if matches!(op, Cdf) { Quantile } else { Cdf }, left, right, span);
    }
    let (call, valence) = f.inverse.ok_or_else(|| Error::domain("this function has no known inverse"))?;
    valence.check(&format!("{}⁻¹", f.name), left)?;
    call(left, right, span)
}

const HOST_FACTS: &[&str] = &["args", "version", "env", "width", "height", #[cfg(not(web))] "cwd", #[cfg(not(web))] "temp",];

/// `•host name` gives the host fact called `name`: the command-line `"args"`, BPL's `"version"`, the `"env"` record, the terminal's `"width"` and `"height"`, the working directory `"cwd"`, and `"temp"`, the directory for temporary files.
fn host(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    crate::transform::acts()?;
    let fact = match crate::keyed::name(right).as_deref() {
        Some("args") => {
            let args = session().args.borrow();
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
        Some("cwd") => Ok(crate::keyed::text(&std::env::current_dir().map_err(|e| Error::domain(format!("working directory: {e}")))?.to_string_lossy())),
        #[cfg(not(web))]
        Some("temp") => Ok(crate::keyed::text(&std::env::temp_dir().to_string_lossy())),
        _ => return Err(Error::domain(format!("•host takes {}", HOST_FACTS.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ")))),
    };
    fact.described("invalid host fact")
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
fn delay(_: Option<&Value>, right: &Value, _: &Span) -> Result<(Value, bool), Error> {
    use {crate::host::Instant, std::time::Duration};
    let seconds = crate::primitive::real(right)?;
    if seconds.is_nan() || seconds < 0.0 { return Err(Error::domain("•delay needs a nonnegative number of seconds")); }
    let start = Instant::now();
    let end = Duration::try_from_secs_f64(seconds).ok().and_then(|d| start.checked_add(d));
    loop {
        let now = Instant::now();
        if end.is_some_and(|end| now >= end) { break; }
        crate::execution::pause(end.map_or(Duration::MAX, |end| end - now))?;
    }
    Ok((Value::Number(crate::Number::float(start.elapsed().as_secs_f64(), FloatWidth::F64)), true))
}

/// `•storage Y`: how `Y`'s items are held, as a record. `kind` names the storage (`Value::storage_kind`). Under static kinds, an integer's kind names its width as well, `int32` or `int64`, because the width is part of the kind there. `pending` is true while the items wait in a pending call, and `view` while `Y` reads another array's items. `active` counts the `∂` levels whose tags `Y` carries. Reading no items, it never forces a pending array.
fn storage(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    // A batched run's cells aren't stored as the cells of a run cell by cell are.
    crate::transform::acts()?;
    let flag = |b: bool| Value::Number(crate::Number::from_bool(b));
    let kind = right.storage_kind(Numeric::current());
    let values = vec![crate::keyed::text(kind), flag(right.template().pending().is_some()), flag(right.template().is_view()), Value::integer(crate::derivative::levels(right) as i64)];
    crate::keyed::vector(["kind", "pending", "view", "active"].map(Into::into).to_vec(), values).described("invalid storage record")
}

/// `•memory x`: what this thread holds, and what `x` keeps alive, as a record. `records` counts the live record nodes of `∂`, which each `∂` frees when it returns, and `pending` the live pending arrays. Each counts the nodes this thread made, wherever they are dropped. `bytes` counts the allocations that `x` keeps alive (`Value::footprint`). It reads no items and never forces a pending array. It takes active values under `∂`.
fn memory(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    // A batched run keeps alive what a run cell by cell doesn't.
    crate::transform::acts()?;
    use crate::explain::{live, Counted};
    let counts = [live(Counted::Record), live(Counted::Pending), right.footprint()].map(|n| Value::integer(n as i64)).to_vec();
    crate::keyed::vector(["records", "pending", "bytes"].map(Into::into).to_vec(), counts).described("invalid memory record")
}

/// `•detach Y`: `Y` with every `∂` tag stripped. No derivative flows back through the result. Outside `∂`, it gives `Y`.
fn detach(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> { Ok(crate::derivative::detach(right)) }

/// `•index n` is `⍳n` along a fresh hidden axis name whose stem is the left argument's text (`fresh`). An array of counts gives an index for each (`indices`).
fn index(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let stem: std::sync::Arc<str> = match left {
        None => "".into(),
        Some(a) => crate::keyed::name(a).ok_or_else(|| Error::domain("•index takes text as its stem"))?,
    };
    indices(&stem, right, session().prototype.get(), span)
}

/// An index with stem `stem` for each count in `right`, nested as `right` is, in its layout with its keys: what Each would give, in one call. An empty array's fill is the result on its prototype. A call on prototypes gives `⍬` for each count and draws no name.
fn indices(stem: &str, right: &Value, prototype: bool, span: &Span) -> Result<Value, Error> {
    if right.is_atom() {
        if prototype { return Ok(numbers(span)); }
        return Primitive::Iota.call(None, right, span)?.with_axis_names(vec![Some(fresh(stem))]).described("invalid index");
    }
    let items = right.elements().map(|count| indices(stem, &count, prototype, span)).collect::<Result<Vec<_>, _>>()?;
    let fill = || indices(stem, &right.prototype(), true, span).expect("a call on prototypes draws nothing").zeroed();
    right.layout().collect(items, fill).described("invalid index")
}

/// A fresh hidden name: `_`, `stem`, `·`, and 64 random bits from the operating system as 13 base-32 digits, anew on each call. A running trace or batched run keeps it (`transform::Levels::keep_fresh`).
pub(crate) fn fresh(stem: &str) -> std::sync::Arc<str> {
    const DIGITS: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let mut bits: u64 = rand::random();
    let random: String = (0..13).map(|_| { let digit = DIGITS[(bits % 32) as usize] as char; bits /= 32; digit }).collect();
    let name: std::sync::Arc<str> = format!("_{stem}{FRESH}{random}").into();
    session().keep_fresh(name.clone());
    name
}

/// A fresh name with the stem of `name`, which `fresh` drew.
pub(crate) fn renamed(name: &str) -> std::sync::Arc<str> { fresh(&name[1..name.rfind(FRESH).expect("a fresh name holds its separator")]) }

/// The edge mode that `name` names, one of rustymath's `structure::Edge::MODES`, as `•edge` and window options read it.
pub(crate) fn edge_mode(name: &Value) -> Result<rustymath::structure::Edge, Error> {
    use rustymath::structure::Edge;
    crate::keyed::name(name).and_then(|m| Edge::named(&m)).ok_or_else(|| {
        let names: Vec<_> = Edge::MODES.iter().map(|(name, _)| *name).collect();
        let (last, rest) = names.split_last().expect("modes");
        Error::domain(format!("the edge modes are {} and {last}", rest.join(", ")))
    })
}
/// `•edge mode`: the function `n f p` that maps positions `p` on an axis of `n` items by the edge mode named `mode` (`edge_mode`). `n •edge p` maps them by the fill mode.
fn edge(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let Some(n) = left else { return Ok(Value::Function(native("•edge", Call::Edge(edge_mode(right)?), Dyadic))) };
    edged(rustymath::structure::Edge::Fill, n, right)
}
/// The position on an axis of `n` items that each position in `p` reads under edge mode `mode`, as rustymath's `structure::Edge::positions` gives it, in one pass over compact storage. Fill gives NaN for a position outside the axis, which reads the fill, so its result is 64-bit floats, whatever the float pref. The other modes give integers, unless the axis is empty. A NaN in `p` stays NaN.
pub(crate) fn edged(mode: rustymath::structure::Edge, n: &Value, p: &Value) -> Result<Value, Error> {
    use rustymath::structure::Edged;
    let n = n.as_number().and_then(|n| n.nonnegative_integer().ok()).ok_or_else(|| Error::domain("•edge takes a nonnegative whole length"))?;
    let edged = match p.as_items().view() {
        Some(view @ (View::Booleans(_) | View::Integers(_) | View::Floats(_))) => mode.positions(view, n).ok(),
        _ => p.as_items().positions().ok().map(|signed| mode.mapped(n, signed.contains(&None), signed.into_iter())),
    };
    let result = match edged.ok_or_else(|| Error::domain("•edge takes whole positions"))? {
        Edged::Floats(y) if p.is_atom() => return Ok(Value::Number(crate::Number::float(y[0], FloatWidth::F64))),
        Edged::Positions(q) if p.is_atom() => return Ok(Value::integer(q[0] as i64)),
        Edged::Floats(y) => p.layout().floats(FloatWidth::F64, y),
        Edged::Positions(q) => Value::positions(p.layout().shape(), n, q.into_iter()).and_then(|v| v.with_layout(p.layout().clone())),
    };
    result.described("invalid positions")
}
/// `width •float Y`: the numbers of `Y` as floats of `width` bits, 16, 32 or 64, with the arrays nested in `Y` converted too. Under `∂`, a tangent converts with its primal. `•float Y`: the width of `Y`'s floats, as a number that `•float`'s left argument takes. It's the widest float width among `Y`'s numbers, nested arrays included, or the default width, which the `float` pref sets, when they're all exact.
fn to_float(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let Some(w) = left else {
        let width = widest(right).described("•float takes numbers")?.unwrap_or(Numeric::current().width);
        return Ok(Value::integer(width.bits()));
    };
    let width = w.as_number().and_then(|n| n.integer().ok()).and_then(FloatWidth::of_bits).ok_or(Kind::Domain).described("•float takes a width of 16, 32 or 64")?;
    crate::derivative::converted(right, rustymath::buffer::DType::Float(width), span)
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
fn signal(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let invalid = || Error::domain("•signal needs an error kind such as \"DOMAIN\", or a caught error");
    let (name, message) = match crate::keyed::name(right) {
        Some(name) => (name, "explicitly signalled".into()),
        None => {
            let fields = crate::keyed::pairs(right).map_err(|_| invalid())?;
            let field = |key: &str| fields.iter().find(|(k, _)| &**k == key).and_then(|(_, v)| crate::keyed::name(v)).ok_or_else(invalid);
            (field("kind")?, field("message")?.to_string())
        }
    };
    let kind = ErrorKind::named(&name).ok_or_else(invalid)?;
    let message = match left { Some(m) => crate::data::text(m)?, None => message };
    Err(Error::new(kind, message))
}

fn case_convert(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let mode = match left {
        None => -3, Some(a) if a.is_singleton() => numeric(&a.at(0))?.integer().described("•c mode must be 1, ¯1 or ¯3")?,
        _ => return Err(Error::domain("•c needs one case mode"))
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(Error::domain("•c mode must be 1, ¯1 or ¯3")); }
    let text: fn(&str) -> String = match mode { 1 => str::to_uppercase, -1 => str::to_lowercase, _ => |s| s.to_uppercase().to_lowercase() };
    cased(right, text, span)
}

/// `value` with each character vector mapped as a string by `text`, and each other character mapped on its own. A character whose mapping has more than one character stays as it is.
fn cased(value: &Value, text: fn(&str) -> String, span: &Span) -> Result<Value, Error> {
    check()?;
    let mapped = match value {
        Value::Character(c) => {
            let s = text(&c.to_string());
            let mut chars = s.chars();
            return Ok(Value::Character(match (chars.next(), chars.next()) { (Some(m), None) => m, _ => *c }));
        }
        Value::Array(_) if !value.is_empty() => match crate::keyed::name(value) {
            Some(s) => crate::keyed::text(&text(&s)),
            None => Value::new(value.shape(), value.elements().map(|e| cased(&e, text, span)).collect::<Result<_, _>>()?).described("invalid result")?,
        },
        _ => return Ok(value.clone()),
    };
    if mapped.shape() != value.shape() { return Ok(mapped); }
    mapped.with_layout(value.layout().clone()).described("invalid result")
}

/// `form •normalize text` puts `text` into a Unicode normalization form: `"NFC"`, the default, `"NFD"`, `"NFKC"` or `"NFKD"`.
fn normalize(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let form = crate::data::algorithm(left, "nfc")?.to_uppercase();
    if !["NFC", "NFD", "NFKC", "NFKD"].contains(&form.as_str()) { return Err(Error::domain("•normalize takes \"NFC\", \"NFD\", \"NFKC\" or \"NFKD\"")); }
    let text = crate::keyed::name(right).ok_or_else(|| Error::domain("•normalize needs text"))?;
    Ok(crate::keyed::text(&crate::host::normalize(&form, &text)))
}

/// `•literal⁻¹ Y` writes `Y` as BPL source, which `•literal` reads back. A value that holds a function or an operator is a DOMAIN error.
pub(crate) fn write_literal(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    crate::transform::formats(None, right)?;
    if right.holds_function() { return Err(Error::domain("•literal⁻¹ writes data, not functions")); }
    Ok(crate::keyed::text(&right.source(crate::display::Elide { plain: None, ..crate::display::Elide::NONE })))
}

fn unicode_convert(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let invalid = || Error::domain("invalid Unicode conversion");
    let encoding = if let Some(spec) = left {
        let name = if matches!(spec.elements().next(), Some(Value::Array(_))) {
            if spec.shape().len() > 1 || !(1..=2).contains(&spec.len()) { return Err(invalid()); }
            if spec.len() == 2 {
                let mode = numeric(&spec.at(1))?.integer().map_err(|_| invalid())?;
                if mode == 83 { return Err(Error::new(ErrorKind::Unsupported, "•ucs signed bytes are out of scope")); }
                if mode != 0 { return Err(invalid()); }
            }
            spec.at(0).clone()
        } else { spec.clone() };
        let text = crate::keyed::name(&name).filter(|_| !name.is_atom()).ok_or_else(invalid)?;
        if !matches!(&*text, "UTF-8" | "UTF-16" | "UTF-32") { return Err(invalid()); }
        if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "encoded •ucs needs a vector")); }
        Some(text)
    } else { None };
    let shape = |len: usize| {
        let shape = if encoding.is_some() { vec![len] } else { right.shape().to_vec() };
        generated_len(&shape).map(|_| shape).map_err(Error::from)
    };
    let result = if matches!(right.prototype(), Value::Character(_)) {
        use crate::array::Items;
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
    result.and_then(|a| if encoding.is_none() { a.with_layout(right.layout().clone()) } else { Ok(a) }).described("invalid Unicode result")
}
