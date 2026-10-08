//! Every glyph's kind, names and ranks, shared by completion, help and Python.
use crate::primitive::{OperatorKind, Primitive, Rank, WHOLE};
use foldhash::HashMap;
use std::sync::OnceLock;

/// A glyph's kind and names. `name` is its page in `nbs/glyphs`. An empty operation name means that valence has no typed name.
/// Aliases are extra completion words, separated by spaces.
pub struct Symbol {
    pub glyph: &'static str,
    pub name: &'static str,
    /// `"function"`, `"monadic-operator"`, `"dyadic-operator"`, or a syntax kind: `"argument"`, `"literal"`, `"comment"`, `"system"` or `"syntax"`.
    pub kind: &'static str,
    pub monad: &'static str,
    pub dyad: &'static str,
    /// The monadic form's natural rank, and the dyadic form's left and right ranks, as BPL text such as `1` or `1 ∞`. `∞` is the whole
    /// argument. Each is empty when the glyph has no such form.
    pub monad_rank: String,
    pub dyad_ranks: String,
    pub aliases: &'static str,
}

// Glyphs that are syntax, not functions or operators: glyph, name, kind, aliases.
const SYNTAX: &[(&str, &str, &str, &str)] = &[
    ("←", "assign", "syntax", "left-arrow"),
    ("→", "pipe", "syntax", "right-arrow"),
    ("⎕", "quad", "syntax", ""),
    ("•", "bullet", "system", "system"),
    ("⍺", "alpha", "argument", ""),
    ("⍵", "omega", "argument", ""),
    ("⍶", "alpha-underbar", "argument", "left-operand"),
    ("⍹", "omega-underbar", "argument", "right-operand"),
    ("∇", "del", "argument", "recursion"),
    ("⍢", "del-diaeresis", "argument", "operator-recursion"),
    ("⍝", "comment", "comment", ""),
    ("⋄", "diamond", "syntax", ""),
    ("?", "question", "syntax", "predicate"),
    ("¯", "overbar", "literal", ""),
    ("∞", "infinity", "literal", ""),
    ("⍬", "zilde", "literal", "empty"),
];

/// Every glyph: syntax, then primitives and operators from their rows in `primitive.rs`.
pub fn symbols() -> &'static [Symbol] {
    static SYMBOLS: OnceLock<Vec<Symbol>> = OnceLock::new();
    SYMBOLS.get_or_init(|| {
        let syntax = SYNTAX.iter().map(|&(glyph, name, kind, aliases)| Symbol { glyph, name, kind, monad: "", dyad: "", monad_rank: String::new(), dyad_ranks: String::new(), aliases });
        let primitives = Primitive::all().map(|p| {
            let info = p.info();
            let (monad, dyad) = (info.monad.map_or("", |m| m.name), info.dyad.map_or("", |d| d.name));
            let rank = |r: Rank| if r == WHOLE { "∞".to_string() } else { r.to_string() };
            let (monad_rank, dyad_ranks) = (info.monad.map_or(String::new(), |m| rank(m.rank)), info.dyad.map_or(String::new(), |d| d.ranks.map(rank).join(" ")));
            Symbol { glyph: info.glyph, name: info.name, kind: "function", monad, dyad, monad_rank, dyad_ranks, aliases: info.aliases }
        });
        let operators = OperatorKind::all().map(|op| {
            let info = op.info();
            let (name, aliases) = info.names;
            let kind = if info.dyadic() { "dyadic-operator" } else { "monadic-operator" };
            Symbol { glyph: info.glyph, name, kind, monad: "", dyad: "", monad_rank: String::new(), dyad_ranks: String::new(), aliases }
        });
        syntax.chain(primitives).chain(operators).collect()
    })
}

/// What a key does: type text, or move to a dead-key state, which types nothing until the next key.
#[derive(PartialEq)]
pub enum Action { Text(String), State(String) }

/// A dead-key state: what each listed key does next, and the terminator an unlisted key types first.
pub struct DeadState { pub terminator: String, pub keys: Vec<(char, Action)> }

/// The shared key mapping in `python/basedpl/layout.json`, which the macOS layout and the browser also read. Keys are US characters
/// after Shift. `option` holds unshifted keys typed with Option.
pub struct KeyLayout {
    pub option: Vec<(char, Action)>, pub alt_aliases: Vec<(char, Action)>, pub states: HashMap<String, DeadState>,
    pub unshifted: HashMap<char, char>,
}

impl KeyLayout {
    pub fn state(&self, name: &str) -> &DeadState { &self.states[name] }
    pub fn alt(&self, c: char) -> Option<&Action> { find(&self.alt_aliases, c).or_else(|| find(&self.option, c)) }
    pub fn display_keys<'a>(&'a self, keys: &'a [(char, Action)]) -> impl Iterator<Item = &'a (char, Action)> {
        keys.iter().filter(move |(key, action)| self.unshifted.get(key).and_then(|key| find(keys, *key)).is_none_or(|a| a != action))
    }
}

pub fn find(keys: &[(char, Action)], key: char) -> Option<&Action> { keys.iter().find(|(k, _)| *k == key).map(|(_, a)| a) }

pub fn layout() -> &'static KeyLayout {
    static LAYOUT: OnceLock<KeyLayout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let json: serde_json::Value = serde_json::from_str(include_str!("../python/basedpl/layout.json")).expect("valid key layout");
        let action = |a: &serde_json::Value| match a.as_str() {
            Some(text) => Action::Text(text.into()),
            None => Action::State(a["state"].as_str().expect("a state name").into()),
        };
        let keys =
            |m: &serde_json::Value| m.as_object().expect("a key map").iter().map(|(k, a)| (k.chars().next().expect("a key"), action(a))).collect::<Vec<_>>();
        let states = json["states"]
            .as_object()
            .expect("states")
            .iter()
            .map(|(name, s)| (name.clone(), DeadState { terminator: s["terminator"].as_str().expect("a terminator").into(), keys: keys(&s["keys"]) }));
        let unshifted = json["unshifted"].as_object().expect("unshifted keys").iter()
            .map(|(shifted, plain)| (shifted.chars().next().unwrap(), plain.as_str().unwrap().chars().next().unwrap())).collect();
        KeyLayout { option: keys(&json["option"]), alt_aliases: keys(&json["alt_aliases"]), states: states.collect(), unshifted }
    })
}

/// The keys that type `glyph`, shown beside each listed name: ` a` for Option-A, or ` c t` for Option-C then T. The shortest
/// sequence wins. A dead key's terminator, such as `○` for Option-O, shows the key alone. An ASCII glyph, such as `+` or `|`, gives
/// nothing, because it has a key of its own.
pub fn chord(glyph: &str) -> String {
    if glyph.is_ascii() { return String::new(); }
    let layout = layout();
    let mut queue: std::collections::VecDeque<(String, &[(char, Action)])> = [(String::new(), layout.option.as_slice())].into();
    while let Some((typed, keys)) = queue.pop_front() {
        for (key, action) in layout.display_keys(keys) {
            let sequence = format!("{typed} {key}");
            match action {
                Action::Text(text) if text == glyph => return sequence,
                Action::State(name) if layout.state(name).terminator == glyph => return sequence,
                Action::State(name) => queue.push_back((sequence, &layout.state(name).keys)),
                Action::Text(_) => {}
            }
        }
    }
    String::new()
}

/// Every glyph's row, as `basedpl.symbols` and the browser's language bar read it: the fields of `Symbol`, and `shortcut`, the
/// glyph's `chord`.
pub fn rows() -> serde_json::Value {
    let row = |s: &Symbol| {
        serde_json::json!({"glyph": s.glyph, "name": s.name, "kind": s.kind, "monad": s.monad, "dyad": s.dyad, "monad_rank": s.monad_rank,
            "dyad_ranks": s.dyad_ranks, "aliases": s.aliases, "shortcut": chord(s.glyph)})
    };
    symbols().iter().map(row).collect()
}

// At each level (exact, prefix, prefixes of hyphen-separated parts) a name outranks a search word.
pub fn matches(query: &str) -> Vec<(&'static str, &'static str)> {
    let query = query.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut best = usize::MAX;
    for &Symbol { glyph, name, monad, dyad, aliases: words, .. } in symbols() {
        let rank = std::iter::once(name)
            .chain([monad, dyad])
            .chain(words.split_whitespace())
            .filter(|word| !word.is_empty())
            .enumerate()
            .filter_map(|(i, word)| {
                let letters = word.replace('-', "");
                let rank = if letters == query { 0 } else if letters.starts_with(&query) { 1 } else {
                    let mut rest = query.as_str();
                    for part in word.split('-') {
                        let n = part.bytes().zip(rest.bytes()).take_while(|(a, b)| a == b).count();
                        if n == 0 { break; }
                        rest = &rest[n..];
                        if rest.is_empty() { break; }
                    }
                    if !rest.is_empty() { return None; }
                    2
                };
                Some(2 * rank + usize::from(i > 0))
            })
            .min();
        if let Some(rank) = rank {
            if rank < best {
                found.clear();
                best = rank;
            }
            if rank == best { found.push((glyph, name)); }
        }
    }
    // A name that is a prefix of every other match wins: `om gives omega, `omu gives omega-underbar.
    let letters = |name: &str| name.replace('-', "");
    let shortest = found.iter().copied().find(|a| found.iter().all(|b| letters(b.1).starts_with(&letters(a.1))));
    match shortest { Some(shortest) if found.len() > 1 => vec![shortest], _ => found }
}

pub fn entry(line: &str, pos: usize) -> Option<(usize, &str)> {
    let start = line[..pos].rfind('`')?;
    let prefix = &line[start + 1..pos];
    (prefix.bytes().all(|c| c.is_ascii_alphabetic()) && crate::syntax::in_code(&line[..start])).then_some((start, prefix))
}
