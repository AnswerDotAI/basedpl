use std::{fmt, ops::Range, sync::Arc};
use unicode_width::UnicodeWidthStr;

/// Shared source identity and text. Syntax and errors retain only the sources they use.
#[derive(Debug)]
pub struct Source { pub name: String, pub text: String }

impl Source { pub fn new(name: impl Into<String>, text: impl Into<String>) -> Arc<Self> { Arc::new(Self { name: name.into(), text: text.into() }) } }

/// A UTF-8 byte range within an owned source, never an offset into a later input.
#[derive(Clone, Debug)]
pub struct Span { pub source: Arc<Source>, pub range: Range<usize> }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Domain,
    Length,
    Limit,
    Rank,
    Index,
    Value,
    Unsupported,
    Interrupt,
    Timeout,
}

impl ErrorKind {
    pub(crate) fn number(self) -> Option<usize> {
        Some(match self {
            Self::Syntax => 2,
            Self::Index => 3,
            Self::Rank => 4,
            Self::Length => 5,
            Self::Value => 6,
            Self::Limit => 10,
            Self::Domain => 11,
            Self::Unsupported | Self::Interrupt | Self::Timeout => return None,
        })
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "SYNTAX ERROR",
            Self::Domain => "DOMAIN ERROR",
            Self::Length => "LENGTH ERROR",
            Self::Limit => "LIMIT ERROR",
            Self::Rank => "RANK ERROR",
            Self::Index => "INDEX ERROR",
            Self::Value => "VALUE ERROR",
            Self::Unsupported => "UNSUPPORTED",
            Self::Interrupt => "INTERRUPT",
            Self::Timeout => "TIMEOUT",
        })
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
    pub(crate) fn error(&self, kind: ErrorKind, message: impl Into<String>) -> Error {
        Error { kind, message: message.into(), span: self.clone(), calls: Vec::new() }
    }
}

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
    fn render(&self, label: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = &self.source.text;
        let start = self.range.start;
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let line = text[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        let prefix = display_text(&text[line_start..start]).width();
        let end = display_text(&text[line_start..self.range.end.min(line_end)]).width();
        write!(
            f,
            " {label} {}:{line}:{}\n{}\n{}{}",
            display_text(&self.source.name),
            prefix + 1,
            display_text(&text[line_start..line_end]),
            " ".repeat(prefix),
            "^".repeat(end.saturating_sub(prefix).max(1))
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.kind, self.message)?;
        self.span.render("-->", f)?;
        for call in &self.calls {
            writeln!(f)?;
            call.render("called from", f)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
