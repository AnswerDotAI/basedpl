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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Domain,
    Length,
    Limit,
    Rank,
    Index,
    Value,
    Io,
    Unsupported,
    Interrupt,
    Timeout,
    /// A kind that a program names when it signals an error, in upper case.
    Custom(Arc<str>),
}

impl ErrorKind {
    const BUILTIN: [Self; 11] = [
        Self::Syntax,
        Self::Domain,
        Self::Length,
        Self::Limit,
        Self::Rank,
        Self::Index,
        Self::Value,
        Self::Io,
        Self::Unsupported,
        Self::Interrupt,
        Self::Timeout,
    ];
    /// The name that guards and `•signal` use, such as `DOMAIN`.
    pub fn name(&self) -> &str {
        match self {
            Self::Syntax => "SYNTAX",
            Self::Domain => "DOMAIN",
            Self::Length => "LENGTH",
            Self::Limit => "LIMIT",
            Self::Rank => "RANK",
            Self::Index => "INDEX",
            Self::Value => "VALUE",
            Self::Io => "IO",
            Self::Unsupported => "UNSUPPORTED",
            Self::Interrupt => "INTERRUPT",
            Self::Timeout => "TIMEOUT",
            Self::Custom(name) => name,
        }
    }
    /// The kind that `name` names, in any case. A name that isn't BPL's is a custom kind. `None` unless `name` is one word of letters,
    /// digits and underscores.
    pub(crate) fn named(name: &str) -> Option<Self> {
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') { return None; }
        let name = name.to_uppercase();
        Some(Self::BUILTIN.into_iter().find(|k| k.name() == name).unwrap_or_else(|| Self::Custom(name.into())))
    }
}

/// `DOMAIN ERROR` and so on. Interrupts, timeouts and unsupported features show their names alone.
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Self::Unsupported | Self::Interrupt | Self::Timeout => f.write_str(self.name()), _ => write!(f, "{} ERROR", self.name()) }
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Span,
    pub calls: Vec<Span>,
}

impl Span {
    /// The whole of `source`.
    pub(crate) fn whole(source: Arc<Source>) -> Self { Self { range: 0..source.text.len(), source } }
    pub(crate) fn error(&self, kind: ErrorKind, message: impl Into<String>) -> Error {
        Error { kind, message: message.into(), span: self.clone(), calls: Vec::new() }
    }
    pub(crate) fn domain_error(&self, message: impl Into<String>) -> Error { self.error(ErrorKind::Domain, message) }
    /// The file `path` names. A path that starts with `./` or `../` is relative to the file holding this code. Other relative paths,
    /// and every path in code not read from a file, are relative to the working directory.
    pub(crate) fn path(&self, path: &str) -> PathBuf {
        let relative = path.starts_with("./") || path.starts_with("../");
        match std::path::Path::new(&self.source.name).parent() { Some(dir) if relative && self.source.file => dir.join(path), _ => path.into() }
    }
    /// An IO error about `what`, such as a file's path, a URL or standard input.
    pub(crate) fn io_error(&self, what: &str, e: impl std::fmt::Display) -> Error { self.error(ErrorKind::Io, format!("{what}: {e}")) }
    /// The bytes of the file `path` names.
    pub(crate) fn read(&self, path: &str) -> Result<Vec<u8>, Error> { std::fs::read(self.path(path)).map_err(|e| self.io_error(path, e)) }
}

/// Places an error from array building, which carries only its kind, at a source span.
pub(crate) trait ErrorAt<T> { fn error_at(self, span: &Span, message: &str) -> Result<T, Error>; }
impl<T> ErrorAt<T> for Result<T, ErrorKind> { fn error_at(self, span: &Span, message: &str) -> Result<T, Error> { self.map_err(|k| span.error(k, message)) } }

/// Places an error from numeric code, which carries only its message, at a source span as a DOMAIN error.
pub(crate) trait DomainAt<T> { fn domain_at(self, span: &Span) -> Result<T, Error>; }
impl<T, M: Into<String>> DomainAt<T> for Result<T, M> { fn domain_at(self, span: &Span) -> Result<T, Error> { self.map_err(|m| span.domain_error(m)) } }

// Render tabs at four-column stops. Escape other controls rather than sending them
// to the terminal. Source spans themselves remain unchanged UTF-8 byte offsets.
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
            f,
            " {label} {}:{line}:{column}\n{}\n{}{}",
            display_text(&self.source.name),
            display_text(&text[line_start..line_end]),
            " ".repeat(column - 1),
            "^".repeat((end + 1).saturating_sub(column).max(1))
        )
    }
}

/// A traceback shows this many calls at each end, and counts the calls between them.
const CALLS_SHOWN: usize = 3;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.kind, self.message)?;
        self.span.render("-->", f)?;
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
