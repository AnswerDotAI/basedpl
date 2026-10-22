use crate::{syntax::NodeKind, ParseStatus, Source};

include!(concat!(env!("OUT_DIR"), "/help.rs"));

/// The index of BPL's documentation for LLMs, `nbs/llms.txt`, which `]help llms` shows.
pub(crate) const LLMS: &str = include_str!("../nbs/llms.txt");

#[derive(Clone, Debug)]
pub struct Inspection { pub kind: &'static str, pub source: String, pub help: String }

impl Inspection {
    pub(crate) fn new(kind: &'static str, source: String) -> Self {
        let comments: Vec<_> = source.strip_prefix('{').unwrap_or("").trim_start().lines().map(str::trim)
            .take_while(|line| line.chars().next().is_some_and(|c| crate::symbols::canonical(c) == '⍝'))
            .map(|line| line.trim_start_matches(|c| crate::symbols::canonical(c) == '⍝').trim()).collect();
        let help = if comments.is_empty() { documentation(&source).unwrap_or(&source).to_owned() } else { comments.join("\n") };
        Self { kind, source, help }
    }
    pub fn text(&self, detail: bool) -> String { format!("{}\n\n{}", self.kind, self.markdown(detail)) }
    pub fn markdown(&self, detail: bool) -> String {
        if detail { format!("```apl\n{}\n```", self.source) }
        else if self.kind == "function" && documentation(&self.source).is_none() { format!("Calls: `f Y` or `X f Y`\n\n{}", self.help) }
        else { self.help.clone() }
    }
}

/// Help for a system name, syntax token or glyph.
pub(crate) fn documentation(symbol: &str) -> Option<&'static str> {
    let canonical: String = symbol.chars().map(crate::symbols::canonical).collect();
    let symbol = canonical.as_str();
    if symbol.starts_with('$') { return page(symbol); }
    if let Some(crate::primitive::Builtin::System(entry)) = crate::primitive::Builtin::find(symbol) { return page(entry.name()); }
    page(match symbol {
        "::" => "error-guard", "{" | "}" => "braces", "[" | "]" => "brackets", "(" | ")" => "parentheses", "'" | "\"" => "quote",
        _ => crate::symbols::symbols().iter().find(|s| s.glyph == symbol)?.name
    })
}

/// The glyph page `name` from `nbs/glyphs`, or the system page for name `name` from `nbs/system`.
pub(crate) fn page(name: &str) -> Option<&'static str> { HELP.iter().find(|(page, _)| *page == name).map(|(_, text)| *text) }

pub(crate) fn item(text: &str, aliases: bool) -> Option<NodeKind> {
    let ParseStatus::Complete(parsed) = crate::syntax::parse_with_aliases(Source::new("<inspect>", text), aliases) else { return None; };
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

pub fn word_char(c: char, aliases: bool) -> bool {
    crate::syntax::name_char_in(c, aliases) || matches!(if aliases { crate::symbols::canonical(c) } else { c }, '•' | '$') || c.is_ascii_digit()
}

pub fn at_cursor(code: &str, cursor: usize, aliases: bool) -> Option<&str> {
    if documentation(code.trim()).is_some() { return Some(code.trim()); }
    let cursor = code.char_indices().nth(cursor).map_or(code.len(), |(i, _)| i);
    if !crate::syntax::in_code(&code[..cursor], aliases) { return None; }
    let (mut start, mut end) = (cursor, cursor);
    while let Some(c) = code[..start].chars().next_back().filter(|c| word_char(*c, aliases)) { start -= c.len_utf8(); }
    while let Some(c) = code[end..].chars().next().filter(|c| word_char(*c, aliases)) { end += c.len_utf8(); }
    if start != end { return Some(&code[start..end]); }
    if let Some(c) = code[cursor..].chars().next().filter(|c| !c.is_whitespace()) { return Some(&code[cursor..cursor + c.len_utf8()]); }
    let c = code[..cursor].chars().next_back()?;
    Some(&code[cursor - c.len_utf8()..cursor])
}
