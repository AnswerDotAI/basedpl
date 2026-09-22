use crate::{syntax::NodeKind, ParseStatus, Source};

include!(concat!(env!("OUT_DIR"), "/help.rs"));

#[derive(Clone, Debug)]
pub struct Inspection { pub kind: &'static str, pub source: String, pub help: String }

impl Inspection {
    pub(crate) fn new(kind: &'static str, source: String) -> Self {
        let comments: Vec<_> = source
            .strip_prefix('{')
            .unwrap_or("")
            .trim_start()
            .lines()
            .map(str::trim)
            .take_while(|line| line.starts_with('⍝'))
            .map(|line| line.trim_start_matches('⍝').trim())
            .collect();
        let help = if comments.is_empty() { documentation(&source).unwrap_or(&source).to_owned() } else { comments.join("\n") };
        Self { kind, source, help }
    }
    pub fn text(&self, detail: bool) -> String { format!("{}\n\n{}", self.kind, self.markdown(detail)) }
    pub fn markdown(&self, detail: bool) -> String {
        if detail { format!("```apl\n{}\n```", self.source) } else if self.kind == "function" { format!("Calls: `f Y` or `X f Y`\n\n{}", self.help) } else { self.help.clone() }
    }
}

pub(crate) fn documentation(symbol: &str) -> Option<&'static str> {
    let page = match symbol.to_lowercase().as_str() {
        "•nc" | "•nl" | "•src" | "•ex" => "introspection",
        "•csv" | "•json" | "•vfi" | "•nget" | "•nput" => "data",
        "•c" => "case",
        "•ucs" => "unicode",
        "•load" => "load",
        "•a" => "alphabet",
        "•d" => "digits",
        "•r" => "regex",
        "•signal" | "::" => "error-guard",
        "{" | "}" => "braces",
        "[" | "]" => "brackets",
        "(" | ")" => "parentheses",
        ";" => "semicolon",
        "." => "dot",
        "'" => "quote",
        _ => crate::symbols::SYMBOLS.iter().find(|row| row.0 == symbol)?.1,
    };
    HELP.iter().find(|(name, _)| *name == page).map(|(_, text)| *text)
}

pub(crate) fn item(text: &str) -> Option<NodeKind> {
    let ParseStatus::Complete(parsed) = crate::parse(Source::new("<inspect>", text)) else { return None; };
    let [statement] = parsed.statements.as_slice() else { return None; };
    let [node] = statement.nodes.as_slice() else { return None; };
    (node.span.range == (0..text.len())).then(|| node.kind.clone())
}

pub(crate) fn help_command(code: &str) -> Option<(&str, bool)> {
    let mut words = code.split_whitespace();
    if !words.next()?.eq_ignore_ascii_case("]help") { return None; }
    let name = words.next()?;
    let detail = match words.next() { None => false, Some("-source") => true, _ => return None };
    words.next().is_none().then_some((name, detail))
}

pub(crate) fn word_char(c: char) -> bool {
    (c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙' | '•')) && crate::primitive::Primitive::from_glyph(c).is_none()
}

pub(crate) fn at_cursor(code: &str, cursor: usize) -> Option<&str> {
    if documentation(code.trim()).is_some() { return Some(code.trim()); }
    let cursor = code.char_indices().nth(cursor).map_or(code.len(), |(i, _)| i);
    if !crate::editor::in_code(&code[..cursor]) { return None; }
    let (mut start, mut end) = (cursor, cursor);
    while let Some(c) = code[..start].chars().next_back().filter(|c| word_char(*c)) { start -= c.len_utf8(); }
    while let Some(c) = code[end..].chars().next().filter(|c| word_char(*c)) { end += c.len_utf8(); }
    if start != end { return Some(&code[start..end]); }
    if let Some(c) = code[cursor..].chars().next().filter(|c| !c.is_whitespace()) { return Some(&code[cursor..cursor + c.len_utf8()]); }
    let c = code[..cursor].chars().next_back()?;
    Some(&code[cursor - c.len_utf8()..cursor])
}
