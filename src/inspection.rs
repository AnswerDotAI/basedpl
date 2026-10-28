use crate::{ParseStatus, Source, syntax::NodeKind};

struct Page { name: &'static str, title: &'static str, aliases: &'static [&'static str], text: &'static str }

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

/// A reference page or guide by its topic name or one of its aliases.
pub(crate) fn page(name: &str) -> Option<&'static str> {
    HELP.iter().find(|p| p.name.eq_ignore_ascii_case(name) || p.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))).map(|p| p.text)
}

pub(crate) fn topics() -> String { HELP.iter().map(|p| format!("- `]help {}`: {}", p.name, p.title)).collect::<Vec<_>>().join("\n") }

/// Matches a literal topic name/title, or a regex over names, headings and page text, in that order.
pub(crate) fn search(pattern: &str, regex: bool) -> Result<String, crate::Error> {
    let compiled = regex.then(|| crate::host::Regex::new(pattern, true)).transpose()?;
    let query = pattern.to_lowercase();
    let mut found = Vec::new();
    for page in HELP {
        let names = [page.name, page.title].into_iter().chain(page.aliases.iter().copied());
        let Some(regex) = &compiled else {
            if names.clone().any(|name| name.to_lowercase().contains(&query)) { found.push((0, page, String::new())); }
            continue;
        };
        let mut named = false;
        for name in names { if regex.find(name)?.is_some() { named = true; break; } }
        if named { found.push((0, page, String::new())); continue; }
        let (mut heading, mut fenced) = (None, false);
        for line in page.text.lines() {
            fenced ^= line.starts_with("```");
            if !fenced && line.starts_with('#') && regex.find(line)?.is_some() { heading = Some(line); break; }
        }
        if let Some(line) = heading { found.push((1, page, excerpt(line, 0))); }
        else if let Some(at) = regex.find(page.text)? { found.push((2, page, excerpt(page.text, at))); }
    }
    found.sort_by_key(|(rank, p, _)| (*rank, p.name));
    if found.is_empty() { return Ok(format!("No matching topics for {pattern:?}")); }
    Ok(found.iter().map(|(_, p, text)| {
        let title = format!("- `]help {}`: {}", p.name, p.title);
        if text.is_empty() { title } else { format!("{title}\n  {text}") }
    }).collect::<Vec<_>>().join("\n"))
}

fn excerpt(text: &str, at: usize) -> String {
    let start = text[..at].char_indices().rev().nth(40).map_or(0, |(i, _)| i);
    text[start..].chars().take(180).collect::<String>().replace('\n', " ")
}

pub(crate) fn item(text: &str, aliases: bool) -> Option<NodeKind> {
    let ParseStatus::Complete(parsed) = crate::syntax::parse_with_aliases(Source::new("<inspect>", text), aliases) else { return None; };
    let [statement] = parsed.statements.as_slice() else { return None; };
    let [node] = statement.nodes.as_slice() else { return None; };
    (node.span.range == (0..text.len())).then(|| node.kind.clone())
}

pub(crate) enum Help<'a> { Topics, Search(&'a str), Topic(&'a str, bool) }

pub(crate) fn help_command(args: &str, search: bool) -> Option<Help<'_>> {
    let args = args.trim();
    if search { return (!args.is_empty()).then_some(Help::Search(args)); }
    if args.is_empty() { return Some(Help::Topics); }
    let mut words = args.split_whitespace();
    let name = words.next()?;
    let detail = match words.next() { None => false, Some("-source") => true, _ => return None };
    words.next().is_none().then_some(Help::Topic(name, detail))
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
