use rustymath::error::Kind;
use std::{fmt, ops::Range, path::PathBuf, sync::Arc};
use unicode_width::UnicodeWidthStr;

/// Shared source identity and text. Syntax and errors retain only the sources they use.
#[derive(Debug)]
/// `file` marks a source read from a file. Its `name` is then the file's path.
pub struct Source { pub name: String, pub text: String, pub file: bool }

impl Source {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Arc<Self> { Arc::new(Self { name: name.into(), text: text.into(), file: false }) }
    pub fn file(path: impl Into<String>, text: impl Into<String>) -> Arc<Self> { Arc::new(Self { name: path.into(), text: text.into(), file: true }) }
}

/// A UTF-8 byte range within an owned source, never an offset into a later input.
#[derive(Clone, Debug)]
pub struct Span { pub source: Arc<Source>, pub range: Range<usize> }

#[derive(Clone, Debug)]
pub enum ErrorKind {
    Syntax,
    Value,
    Io,
    Unsupported,
    Interrupt,
    Timeout,
    /// A batched run's stop, which holds the id of the level that owns the run and whether a later run of the same operand would stop again (`Stop`). Only that level catches it.
    Stop(Stop),
    /// An error of a kind that rustymath's functions on arrays share: Length, Rank, Index, Limit, Domain or Overflow.
    Array(rustymath::error::Error),
    /// A kind that a program names when it signals an error, in upper case.
    Custom(Arc<str>),
}

impl ErrorKind {
    /// The kinds that BPL names.
    fn builtin() -> [Self; 11] {
        [
            Self::Syntax, Kind::Domain.into(), Kind::Length.into(), Kind::Limit.into(), Kind::Rank.into(), Kind::Index.into(), Self::Value, Self::Io,
            Self::Unsupported, Self::Interrupt, Self::Timeout,
        ]
    }
    /// The name that guards and `•signal` use, such as `DOMAIN`. Rustymath's Overflow is LIMIT, because BPL's exact integers grow and BPL has no overflow kind.
    pub fn name(&self) -> &str {
        match self {
            Self::Syntax => "SYNTAX",
            Self::Value => "VALUE",
            Self::Io => "IO",
            Self::Unsupported => "UNSUPPORTED",
            Self::Interrupt => "INTERRUPT",
            Self::Timeout => "TIMEOUT",
            Self::Stop { .. } => "STOP",
            Self::Array(e) => match e.kind {
                Kind::Domain => "DOMAIN", Kind::Length => "LENGTH", Kind::Limit | Kind::Overflow => "LIMIT", Kind::Rank => "RANK", Kind::Index => "INDEX",
                Kind::Interrupt => "INTERRUPT", Kind::Timeout => "TIMEOUT"
            },
            Self::Custom(name) => name,
        }
    }
    /// The kind that `name` names, in any case. A name that isn't BPL's is a custom kind. `None` unless `name` is one word of letters, digits and underscores.
    pub(crate) fn named(name: &str) -> Option<Self> {
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') { return None; }
        let name = name.to_uppercase();
        Some(Self::builtin().into_iter().find(|k| k.name() == name).unwrap_or_else(|| Self::Custom(name.into())))
    }
    /// Whether an error of this kind controls evaluation rather than reporting a failure. An interrupt or a timeout ends the evaluation. A batched run's stop sends its level back to running cell by cell. A handler that catches every error lets these pass.
    pub(crate) fn is_control(&self) -> bool { matches!(self, Self::Interrupt | Self::Timeout | Self::Stop { .. }) }
}

impl From<rustymath::error::Error> for ErrorKind { fn from(e: rustymath::error::Error) -> Self { Self::Array(e) } }
/// A kind with no message, for `Error::new` or `described` to supply one.
impl From<Kind> for ErrorKind { fn from(kind: Kind) -> Self { Self::Array(rustymath::error::Error::new(kind, "")) } }

/// `DOMAIN ERROR` and so on. Interrupts, timeouts, stops and unsupported features show their names alone.
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported | Self::Interrupt | Self::Timeout | Self::Stop { .. } => f.write_str(self.name()), _ => write!(f, "{} ERROR", self.name())
        }
    }
}

/// An error: its kind, message and span, and the calls it passed through. Boxing its fields keeps each `Result<T, Error>` to one pointer on the stack that deep recursion uses.
#[derive(Clone, Debug)]
pub struct Error(Box<ErrorData>);

/// The fields of an [`Error`], which it dereferences to. `span` is `None` until the error is placed. The evaluator places its own errors at the nodes it reports, and `Function::call` places each error that the function's code raised without a span at the call's span.
#[derive(Clone, Debug)]
pub struct ErrorData { pub kind: ErrorKind, pub message: String, pub span: Option<Span>, pub calls: Vec<Span> }

impl std::ops::Deref for Error {
    type Target = ErrorData;
    fn deref(&self) -> &ErrorData { &self.0 }
}
impl std::ops::DerefMut for Error { fn deref_mut(&mut self) -> &mut ErrorData { &mut self.0 } }

impl Error {
    /// An error with no span yet. A kind that carries its own message, as rustymath's errors do, gives the error that message in place of `message`. An interrupt or a timeout with no message gets its usual one.
    pub(crate) fn new(kind: impl Into<ErrorKind>, message: impl Into<String>) -> Self {
        let mut kind = kind.into();
        let message = match &mut kind { ErrorKind::Array(e) if !e.message.is_empty() => std::mem::take(&mut e.message).into_owned(), _ => message.into() };
        // A stop from rustymath's interrupt check is BPL's own interrupt or timeout.
        let kind = match kind {
            ErrorKind::Array(e) if e.kind == Kind::Interrupt => ErrorKind::Interrupt, ErrorKind::Array(e) if e.kind == Kind::Timeout => ErrorKind::Timeout,
            kind => kind
        };
        let message = match &kind {
            ErrorKind::Interrupt if message.is_empty() => "evaluation interrupted".into(),
            ErrorKind::Timeout if message.is_empty() => "evaluation deadline exceeded".into(), _ => message
        };
        Self(Box::new(ErrorData { kind, message, span: None, calls: Vec::new() }))
    }
    pub(crate) fn domain(message: impl Into<String>) -> Self { Self::new(Kind::Domain, message) }
    /// An IO error about `what`, such as a file's path, a URL or standard input.
    pub(crate) fn io(what: &str, e: impl fmt::Display) -> Self { Self::new(ErrorKind::Io, format!("{what}: {e}")) }
    /// This error, placed at `span` if it has no span yet.
    #[cold]
    #[inline(never)]
    pub(crate) fn at(mut self, span: &Span) -> Self { if self.span.is_none() { self.span = Some(span.clone()); } self }
}

impl From<rustymath::error::Error> for Error { fn from(e: rustymath::error::Error) -> Self { Self::new(e, "") } }
/// A batched run's stop. Only the level that owns the run catches it, so its span never shows.
impl From<Stop> for Error { fn from(stop: Stop) -> Self { Self::new(stop, STOPPED) } }
impl From<Stop> for ErrorKind { fn from(stop: Stop) -> Self { ErrorKind::Stop(stop) } }
/// A message from numeric code, which reports a DOMAIN error as its message alone.
impl From<&'static str> for Error { fn from(message: &'static str) -> Self { Self::domain(message) } }

/// A batched run's stop: a point where the run can't go on for all its cells at once, such as an observing read, an effect or a call with no rule for whole arrays. The level that owns the run then runs its cells one at a time. No other code catches a stop (`ErrorKind::Stop`). Its cause says how long a dfn operand remembers it (`Cause::kept`).
///
/// The level's id, the cause and a detail share one `u64`, which keeps `ErrorKind` at 16 bytes in wasm32. With a separate field it takes 24 bytes. Every frame that moves an evaluation result then holds a larger local on the browser's stack, which leaves room for less recursion (`tests/wasm.mjs`). The detail's meaning depends on the cause, and each cause that keeps one reads it through its own accessor: `glyph` for `Cause::Rule`, `chunk` for `Cause::Memory`.
#[derive(Clone, Copy, Debug)]
pub struct Stop(u64);

impl Stop {
    /// The stop of batched run `level`, for `cause`.
    pub(crate) fn new(level: u64, cause: Cause) -> Self { Self::with_detail(level, cause, 0) }
    /// The stop of batched run `level`, for `cause`, with `detail` in the stop's 7 bits of detail.
    pub(crate) fn with_detail(level: u64, cause: Cause, detail: u64) -> Self { Self(level << 10 | detail << 3 | cause as u64) }
    /// The stop of batched run `level` at a call that would hold too many items (`Cause::Memory`), where runs on chunks of `chunk` cells, a power of two, wouldn't. A chunk of one cell gives no chunks, and the run's cells then run one at a time.
    pub(crate) fn oversized(level: u64, chunk: usize) -> Self { Self::with_detail(level, Cause::Memory, chunk.ilog2().into()) }
    /// The id of the level that owns the run.
    pub(crate) fn level(self) -> u64 { self.0 >> 10 }
    /// Why the run stopped.
    pub(crate) fn cause(self) -> Cause { Cause::from_bits(self.0 & 7) }
    pub(crate) fn detail(self) -> u64 { self.0 >> 3 & 127 }
    /// The cells of each chunk that the run can go on in, where the stop is for too many items and gives chunks (`oversized`).
    pub(crate) fn chunk(self) -> Option<usize> { (self.cause() == Cause::Memory && self.detail() > 0).then(|| 1 << self.detail()) }
}

/// Why a batched run stopped, which says how long a dfn operand remembers the stop (`kept`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// Output, input, `⍕` of a carried value, or a system function's act (`transform::acts`), which the operand does on every call (`Effect::Acts`).
    Effect = 0,
    /// A write to a binding outside the map's frames (`Effect::Writes`).
    Write = 1,
    /// An observing read of a batch, as a count or a position that comes from the cell makes (`Value::observed`).
    Observed = 2,
    /// A call with no rule for batches of its arguments: a primitive with an axis or with no rule for cells, a rule that declines these arguments, a kernel's read of a cell's number (`Value::computed`), or `∂` with respect to cells that are arrays.
    Rule = 3,
    /// A call that would repeat constants over so many cells that it holds far more items than its arguments (`settings::Sizes::batch`). The run goes on in chunks of cells where the stop gives them (`Stop::chunk`).
    Memory = 4,
    /// A guard or a call on prototypes that would catch an error (`Effect::Catches`).
    Caught = 5,
    /// A guard on a batched value in a dfn whose guard already splits the run's cells, as a recursive call makes (`Levels::case`).
    Recursion = 6,
}

/// How long a dfn operand remembers a stop of its batched run (`batch::tried`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kept {
    /// For every later call, because the stop depends only on what the operand does.
    Always,
    /// While later calls repeat the arguments' kinds and cell shapes, and the frame's shape too where `frame` is set. A branch can depend on a cell's shape, through an observing read of a plain value such as `≢⍵`, so a call with no rule in one branch may not recur for other shapes.
    Arguments { frame: bool },
    /// Never, because whether the stop recurs depends on the cells' values.
    Never,
}

impl Cause {
    /// How long a dfn operand remembers a stop for this cause.
    pub(crate) fn kept(self) -> Kept {
        match self {
            Cause::Effect | Cause::Write | Cause::Observed | Cause::Recursion => Kept::Always, Cause::Rule => Kept::Arguments { frame: false },
            Cause::Memory => Kept::Arguments { frame: true }, Cause::Caught => Kept::Never
        }
    }
    /// The cause that `bits` encode, as a stop and a dfn operand's memory of a stop hold it.
    pub(crate) fn from_bits(bits: u64) -> Cause {
        match bits {
            0 => Cause::Effect, 1 => Cause::Write, 2 => Cause::Observed, 3 => Cause::Rule, 4 => Cause::Memory, 5 => Cause::Caught, _ => Cause::Recursion
        }
    }
    /// What `•explain` reports in `fallbacks` for a stop for this cause.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Cause::Effect => "an effect", Cause::Write => "a write", Cause::Observed => "a cell's value", Cause::Rule => "no batch rule",
            Cause::Memory => "too many items", Cause::Caught => "a caught error", Cause::Recursion => "a recursive call"
        }
    }
}

/// The message of a batched run's stop (`Stop`).
pub(crate) const STOPPED: &str = "a batched run can't do this for all its cells at once";

impl Span {
    /// The whole of `source`.
    pub fn whole(source: Arc<Source>) -> Self { Self { range: 0..source.text.len(), source } }
    /// The source text the span covers.
    pub(crate) fn text(&self) -> &str { &self.source.text[self.range.clone()] }
    /// The file `path` names. A path that starts with `./` or `../` is relative to the file holding this code. Other relative paths, and every path in code not read from a file, are relative to the working directory.
    pub(crate) fn path(&self, path: &str) -> PathBuf {
        let relative = path.starts_with("./") || path.starts_with("../");
        match std::path::Path::new(&self.source.name).parent() { Some(dir) if relative && self.source.file => dir.join(path), _ => path.into() }
    }
    /// The bytes of the file `path` names.
    pub(crate) fn read(&self, path: &str) -> Result<Vec<u8>, Error> { crate::host::read(&self.path(path)).map_err(|e| Error::io(path, e)) }
}

/// Gives an error kind its message, where the kind carries none. An `ErrorKind` that wraps a rustymath error keeps that error's message.
pub(crate) trait Described<T> { fn described(self, message: &str) -> Result<T, Error>; }
impl<T, K: Into<ErrorKind>> Described<T> for Result<T, K> { fn described(self, message: &str) -> Result<T, Error> { self.map_err(|k| Error::new(k, message)) } }

/// Places an error at a source span, where it has no span yet.
pub(crate) trait At<T> { fn at(self, span: &Span) -> Result<T, Error>; }
impl<T, E: Into<Error>> At<T> for Result<T, E> { fn at(self, span: &Span) -> Result<T, Error> { self.map_err(|e| e.into().at(span)) } }

// Render tabs at four-column stops. Escape other controls rather than sending them to the terminal. Source spans themselves remain unchanged UTF-8 byte offsets.
fn display_text(text: &str) -> String {
    let mut rendered = String::new();
    for part in text.split_inclusive('\t') {
        let plain = part.strip_suffix('\t').unwrap_or(part);
        for c in plain.chars() { if c.is_control() { rendered.extend(c.escape_default()); } else { rendered.push(c); } }
        if part.ends_with('\t') { rendered.push_str(&" ".repeat(4 - rendered.width() % 4)); }
    }
    rendered
}

impl Span {
    /// The line and column where the span starts, counting from 1. The column counts display width, as a terminal shows it.
    pub(crate) fn position(&self) -> (usize, usize) {
        let text = &self.source.text[..self.range.start];
        let line_start = text.rfind('\n').map_or(0, |i| i + 1);
        (text.bytes().filter(|&b| b == b'\n').count() + 1, display_text(&text[line_start..]).width() + 1)
    }
    fn render(&self, label: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = &self.source.text;
        let start = self.range.start;
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let (line, column) = self.position();
        let end = display_text(&text[line_start..self.range.end.min(line_end)]).width();
        write!(
            f, " {label} {}:{line}:{column}\n{}\n{}{}", display_text(&self.source.name), display_text(&text[line_start..line_end]), " ".repeat(column - 1),
            "^".repeat((end + 1).saturating_sub(column).max(1))
        )
    }
}

/// A traceback shows this many calls at each end, and counts the calls between them.
const CALLS_SHOWN: usize = 3;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.kind, self.message)?;
        if let Some(span) = &self.span { span.render("-->", f)?; }
        let hidden = self.calls.len().saturating_sub(2 * CALLS_SHOWN);
        let (first, rest) = self.calls.split_at(self.calls.len().min(CALLS_SHOWN));
        for call in first {
            writeln!(f)?;
            call.render("called from", f)?;
        }
        if hidden > 0 { write!(f, "\n ... {hidden} more calls")?; }
        for call in &rest[hidden..] {
            writeln!(f)?;
            call.render("called from", f)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
