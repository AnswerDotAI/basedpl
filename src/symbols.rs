//! Every glyph's kind and names, shared by completion, help and Python.
use crate::primitive::{OperatorKind, Primitive};
use foldhash::HashMap;
use std::sync::OnceLock;

/// A glyph's kind and names. `name` is its page in `nbs/glyphs`. An empty operation name means that valence has no typed name.
/// Aliases are extra completion words, separated by spaces.
pub(crate) struct Symbol {
    pub glyph: &'static str,
    pub name: &'static str,
    /// `"function"`, `"operator"` or `"syntax"`. Only `basedpl.symbols` reads it.
    #[cfg_attr(not(feature = "python"), allow(dead_code))]
    pub kind: &'static str,
    pub monad: &'static str,
    pub dyad: &'static str,
    pub aliases: &'static str,
}

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

/// Every glyph: syntax, then primitives and operators from their rows in `primitive.rs`.
pub(crate) fn symbols() -> &'static [Symbol] {
    static SYMBOLS: OnceLock<Vec<Symbol>> = OnceLock::new();
    SYMBOLS.get_or_init(|| {
        let syntax = SYNTAX.iter().map(|&(glyph, name, aliases)| Symbol { glyph, name, kind: "syntax", monad: "", dyad: "", aliases });
        let primitives = Primitive::all().map(|p| {
            let info = p.info();
            let (monad, dyad) = (info.monad.map_or("", |m| m.name), info.dyad.map_or("", |d| d.name));
            Symbol { glyph: info.glyph, name: info.name, kind: "function", monad, dyad, aliases: info.aliases }
        });
        let operators = OperatorKind::all().map(|op| {
            let info = op.info();
            let (name, aliases) = info.names;
            Symbol { glyph: info.glyph, name, kind: "operator", monad: "", dyad: "", aliases }
        });
        syntax.chain(primitives).chain(operators).collect()
    })
}

/// What a key does: type text, or move to a dead-key state, which types nothing until the next key.
pub(crate) enum Action { Text(String), State(String) }

/// A dead-key state: what each listed key does next, and the terminator an unlisted key types first.
pub(crate) struct DeadState { pub terminator: String, pub keys: Vec<(char, Action)> }

/// The shared key mapping in `python/basedpl/layout.json`, which the macOS layout and the browser also read. Keys are US characters
/// after Shift. `option` holds unshifted keys typed with Option, and `plain` holds dead keys typed alone, such as `^`.
pub(crate) struct KeyLayout { pub option: Vec<(char, Action)>, pub plain: Vec<(char, Action)>, pub states: HashMap<String, DeadState> }

impl KeyLayout { pub(crate) fn state(&self, name: &str) -> &DeadState { &self.states[name] } }

pub(crate) fn find(keys: &[(char, Action)], key: char) -> Option<&Action> { keys.iter().find(|(k, _)| *k == key).map(|(_, a)| a) }

pub(crate) fn layout() -> &'static KeyLayout {
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
        KeyLayout { option: keys(&json["option"]), plain: keys(&json["plain"]), states: states.collect() }
    })
}

/// The keys that type `glyph`, shown beside each listed name: ` a` for Option-A, or ` c t` for Option-C then T. The shortest
/// sequence wins. A dead key's terminator, such as `○` for Option-O, shows the key alone. An ASCII glyph, such as `+` or `|`, gives
/// nothing, because it has a key of its own.
pub(crate) fn chord(glyph: &str) -> String {
    if glyph.is_ascii() { return String::new(); }
    let layout = layout();
    let mut queue: std::collections::VecDeque<(String, &[(char, Action)])> = [(String::new(), layout.option.as_slice())].into();
    while let Some((typed, keys)) = queue.pop_front() {
        for (key, action) in keys {
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
