//! Shared names for Python functions and glyph completion.
use crate::primitive::{OperatorKind, Primitive};
use foldhash::HashMap;
use std::sync::OnceLock;

/// One row per glyph: glyph, canonical name (nbs/glyphs.qmd), monad, dyad, extra completion aliases.
/// An empty operation name means that valence has no typed name. Aliases are space-separated.
pub(crate) type Symbol = (&'static str, &'static str, &'static str, &'static str, &'static str);

// Glyphs that are syntax, not functions or operators: glyph, name, aliases.
const SYNTAX: &[(&str, &str, &str)] = &[
    ("←", "assign", "left-arrow"),
    ("→", "pipe", "right-arrow"),
    ("⎕", "quad", ""),
    ("•", "bullet", "system"),
    ("⍺", "alpha", ""),
    ("⍵", "omega", ""),
    ("⍶", "alpha-underbar", "left-operand"),
    ("⍹", "omega-underbar", "right-operand"),
    ("∇", "del", "recursion"),
    ("⍢", "del-diaeresis", "operator-recursion"),
    ("⍝", "comment", ""),
    ("⋄", "diamond", ""),
    ("¯", "overbar", ""),
    ("∞", "infinity", ""),
    ("⍬", "zilde", "empty"),
];

/// Every glyph's names: syntax, then primitives and operators from their rows in `primitive.rs`.
pub(crate) fn symbols() -> &'static [Symbol] {
    static SYMBOLS: OnceLock<Vec<Symbol>> = OnceLock::new();
    SYMBOLS.get_or_init(|| {
        let syntax = SYNTAX.iter().map(|&(glyph, name, aliases)| (glyph, name, "", "", aliases));
        let primitives = Primitive::all().map(|p| {
            let info = p.info();
            (info.glyph, info.name, info.monad.map_or("", |m| m.name), info.dyad.map_or("", |d| d.name), info.aliases)
        });
        let operators = OperatorKind::all().filter_map(|op| {
            let info = op.info();
            info.names.map(|(name, aliases)| (info.glyph, name, "", "", aliases))
        });
        syntax.chain(primitives).chain(operators).collect()
    })
}

// Keys are US characters after Shift but before Alt. Browser adapters share this resource.
pub(crate) fn alt_keys() -> &'static HashMap<char, char> {
    static KEYS: OnceLock<HashMap<char, char>> = OnceLock::new();
    KEYS.get_or_init(|| serde_json::from_str(include_str!("../python/basedpl/keyboard.json")).expect("valid glyph keyboard"))
}

// The Alt chord's key on a US layout, shown beside each listed name: ` a`, or ` Sa` with Shift.
pub(crate) fn chord(glyph: &str) -> String {
    const SHIFTED: &str = "~!@#$%^&*()_+{}|:\"<>?";
    const PLAIN: &str = "`1234567890-=[]\\;',./";
    let Some((&key, _)) = alt_keys().iter().find(|&(_, &g)| glyph.chars().eq([g])) else { return String::new() };
    let base = SHIFTED.find(key).map_or(key.to_ascii_lowercase(), |i| PLAIN.as_bytes()[i] as char);
    format!(" {}{base}", if base == key { "" } else { "S" })
}
