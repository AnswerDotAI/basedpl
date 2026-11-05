use rustymath::error::Kind;
use std::{fmt, ops::Range, path::PathBuf, sync::Arc};
use unicode_width::UnicodeWidthStr;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

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

/// Two spans are equal where they cover the same bytes of one source.
impl PartialEq for Span { fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.source, &other.source) && self.range == other.range } }
impl Eq for Span {}

#[derive(Clone, Debug)]
pub enum ErrorKind {
    Syntax,
    Value,
    Io,
    Unsupported,
    Interrupt,
    Timeout,
    /// Only the matching map boundary consumes this tag.
    BatchStop(usize),
    /// A planning run stopped where it can't plan through. Only `trace::plan_call` consumes it, so it never shows.
    Planning,
    /// A broken internal assumption, which is a bug. Its message names the source line. No guard catches it.
    Internal,
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
    /// The name that guards and `•signal` use, such as `DOMAIN`. Rustymath's Overflow is LIMIT.
    pub fn name(&self) -> &str {
        match self {
            Self::Syntax => "SYNTAX",
            Self::Value => "VALUE",
            Self::Io => "IO",
            Self::Unsupported => "UNSUPPORTED",
            Self::Interrupt => "INTERRUPT",
            Self::Timeout => "TIMEOUT",
            Self::BatchStop(_) => "STOP",
            Self::Planning => "PLANNING",
            Self::Internal => "INTERNAL",
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
    /// Whether an error of this kind controls evaluation rather than reporting a failure. An interrupt or a timeout ends the evaluation. A batched run's stop sends its level back to running cell by cell. A planning stop ends a planning run. An internal error is a bug. A handler that catches every error lets these pass.
    pub(crate) fn is_control(&self) -> bool { matches!(self, Self::Interrupt | Self::Timeout | Self::BatchStop(_) | Self::Planning | Self::Internal) }
}

impl From<rustymath::error::Error> for ErrorKind { fn from(e: rustymath::error::Error) -> Self { Self::Array(e) } }
/// A kind with no message, for `Error::new` or `described` to supply one.
impl From<Kind> for ErrorKind { fn from(kind: Kind) -> Self { Self::Array(rustymath::error::Error::new(kind, "")) } }

/// `DOMAIN ERROR` and so on. Interrupts, timeouts, stops, planning stops and unsupported features show their names alone.
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported | Self::Interrupt | Self::Timeout | Self::BatchStop(_) | Self::Planning => f.write_str(self.name()),
            _ => write!(f, "{} ERROR", self.name()),
        }
    }
}

/// An error: its kind, message and span, and the calls it passed through. Boxing its fields keeps each `std::result::Result<T, Error>` to one pointer on the stack that deep recursion uses.
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
    pub(crate) fn batch_stop(level: usize) -> Self { Self::new(ErrorKind::BatchStop(level), "host read of a batched value") }
    pub(crate) fn batch_level(&self) -> Option<usize> { match &self.kind { ErrorKind::BatchStop(level) => Some(*level), _ => None } }
    /// A broken internal assumption, which is a bug: its message names the calling line. Call it directly: passed as a function item it loses that line.
    #[cold]
    #[inline(never)]
    #[track_caller]
    pub(crate) fn internal() -> Self { Self::new(ErrorKind::Internal, format!("at {}", std::panic::Location::caller())) }
    pub(crate) fn is_planning(&self) -> bool { matches!(self.kind, ErrorKind::Planning) }
    pub(crate) fn is_internal(&self) -> bool { matches!(self.kind, ErrorKind::Internal) }
}

/// An absent value that is a bug: `None` becomes an internal error naming the calling line.
pub(crate) trait OrInternal<T> { fn or_internal(self) -> Result<T>; }
impl<T> OrInternal<T> for Option<T> {
    #[track_caller]
    fn or_internal(self) -> Result<T> { match self { Some(value) => Ok(value), None => Err(Error::internal()) } }
}

impl From<rustymath::error::Error> for Error { fn from(e: rustymath::error::Error) -> Self { Self::new(e, "") } }
impl From<ErrorKind> for Error { fn from(kind: ErrorKind) -> Self { Self::new(kind, "") } }
impl From<Kind> for Error { fn from(kind: Kind) -> Self { Self::new(kind, "") } }
/// A batched run's stop. Only the level that owns the run catches it, so its span never shows. A message from numeric code, which reports a DOMAIN error as its message alone.
impl From<&'static str> for Error { fn from(message: &'static str) -> Self { Self::domain(message) } }

impl Span {
    /// The whole of `source`.
    pub(crate) fn whole(source: Arc<Source>) -> Self { Self { range: 0..source.text.len(), source } }
    /// The source text the span covers.
    pub(crate) fn text(&self) -> &str { &self.source.text[self.range.clone()] }
    /// The file `path` names. A path that starts with `./` or `../` is relative to the file holding this code. Other relative paths, and every path in code not read from a file, are relative to the working directory.
    pub(crate) fn path(&self, path: &str) -> PathBuf {
        let relative = path.starts_with("./") || path.starts_with("../");
        match std::path::Path::new(&self.source.name).parent() { Some(dir) if relative && self.source.file => dir.join(path), _ => path.into() }
    }
    /// The bytes of the file `path` names.
    pub(crate) fn read(&self, path: &str) -> std::result::Result<Vec<u8>, Error> { crate::host::read(&self.path(path)).map_err(|e| Error::io(path, e)) }
}

/// Gives an error kind its message, where the kind carries none. An `ErrorKind` that wraps a rustymath error keeps that error's message.
pub(crate) trait Described<T> { fn described(self, message: &str) -> std::result::Result<T, Error>; }
impl<T, E: Into<Error>> Described<T> for std::result::Result<T, E> {
    fn described(self, message: &str) -> std::result::Result<T, Error> {
        self.map_err(|error| {
            let mut error = error.into();
            if error.message.is_empty() { error.message = message.into(); }
            error
        })
    }
}

/// Places an error at a source span, where it has no span yet.
pub(crate) trait At<T> { fn at(self, span: &Span) -> std::result::Result<T, Error>; }
impl<T, E: Into<Error>> At<T> for std::result::Result<T, E> {
    fn at(self, span: &Span) -> std::result::Result<T, Error> { self.map_err(|e| e.into().at(span)) }
}

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
