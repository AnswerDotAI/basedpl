//! Glyph completion and terminal input. Source execution never rewrites aliases.
use rustyline::{
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::{Hint, Hinter},
    history::DefaultHistory,
    validate::{ValidationContext, ValidationResult, Validator},
    Cmd, CompletionType, ConditionalEventHandler, Config, Context, Editor, Event, EventContext, EventHandler, Helper, KeyCode, KeyEvent, Modifiers,
    RepeatCount,
};
use std::{
    collections::HashMap,
    ops::Range,
    sync::{Arc, Mutex, OnceLock},
};

// Keys are US characters after Shift but before Alt. Browser adapters share this resource.
fn alt_keys() -> &'static HashMap<char, char> {
    static KEYS: OnceLock<HashMap<char, char>> = OnceLock::new();
    KEYS.get_or_init(|| serde_json::from_str(include_str!("../python/basedpl/keyboard.json")).expect("valid glyph keyboard"))
}

// One row per glyph: ambiguity is between glyphs, not between aliases for the same glyph.
// This is an input catalogue, not a claim that every primitive is implemented yet.
pub(crate) const SYMBOLS: &[(&str, &str)] = &[
    ("←", "assign left-arrow"),
    ("→", "pipe right-arrow"),
    ("⍳", "iota index-of"),
    ("⍴", "rho shape reshape"),
    ("≢", "tally not-match"),
    ("≡", "match depth"),
    ("+", "plus conjugate"),
    ("-", "minus negate"),
    ("×", "times multiply sign direction"),
    ("÷", "divide reciprocal"),
    ("⌈", "ceiling max"),
    ("⌊", "floor min"),
    ("|", "magnitude abs residue"),
    ("*", "power exp"),
    ("⍟", "log"),
    ("○", "circle cis"),
    ("π", "pi"),
    ("√", "root sqrt"),
    ("!", "factorial binomial"),
    ("∧", "and lcm"),
    ("∨", "or gcd"),
    ("⍲", "nand"),
    ("⍱", "nor"),
    ("~", "not without"),
    ("=", "equal"),
    ("≠", "not-equal"),
    ("<", "less"),
    ("≤", "less-equal"),
    (">", "greater"),
    ("≥", "greater-equal"),
    ("⎕", "quad"),
    ("•", "bullet system"),
    ("⍞", "quote-quad"),
    ("⍺", "alpha"),
    ("⍵", "omega"),
    ("⍶", "alpha-underbar left-operand"),
    ("⍹", "omega-underbar right-operand"),
    ("∇", "del recursion"),
    ("∇∇", "operator-recursion"),
    ("⍝", "comment"),
    ("⋄", "diamond"),
    ("¯", "overbar"),
    ("∞", "infinity"),
    ("⍬", "zilde empty"),
    (",", "ravel catenate"),
    ("⍪", "table catenate-first"),
    ("⊂", "enclose"),
    ("⊃", "mix pick"),
    ("⊆", "nest partition"),
    ("∊", "epsilon enlist member"),
    ("∪", "unique union"),
    ("∩", "intersection"),
    ("⍋", "grade-up"),
    ("⍒", "grade-down"),
    ("↑", "take first disclose"),
    ("↓", "drop split"),
    ("⌽", "reverse rotate"),
    ("⊖", "reverse-first rotate-first"),
    ("⍉", "transpose"),
    ("⊤", "encode"),
    ("⊥", "decode"),
    ("⍎", "execute"),
    ("⍕", "format"),
    ("⌷", "index squad"),
    ("⌹", "domino matrix-divide"),
    ("¨", "each dieresis"),
    ("/", "reduce replicate slash"),
    ("⌿", "reduce-first replicate-first"),
    ("\\", "scan backslash"),
    ("⍀", "scan-first"),
    ("⍤", "rank atop"),
    ("∘", "jot compose bind"),
    ("⌝", "outer-product top-right-corner"),
    ("⍨", "commute selfie"),
    ("⍥", "over"),
    ("⍛", "behind"),
    ("⍣", "repeat iterate"),
    ("⍣\\", "history trajectory"),
    ("⇄", "inverse-pair"),
    ("⌾", "under"),
    ("↕", "windows"),
    ("ℙ", "prime"),
    ("Ⓠ", "factor"),
    ("Ⓟ", "polynomial"),
    ("∂", "derivative"),
    ("˘", "breve tie strand"),
    ("◶", "agenda choose"),
    ("⍸", "where interval-index"),
    ("⍷", "find"),
    ("⊢", "right same"),
    ("⊣", "left"),
    ("⌸", "key"),
    ("@", "at"),
    ("⌺", "stencil"),
    ("⍠", "variant"),
    ("?", "roll deal"),
];

pub(crate) fn matches(query: &str) -> Vec<(&'static str, &'static str)> {
    let query = query.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut best = 3;
    for &(glyph, names) in SYMBOLS {
        let candidate = names
            .split_whitespace()
            .filter_map(|name| {
                let letters = name.replace('-', "");
                let rank = if letters == query { 0 } else if letters.starts_with(&query) { 1 } else {
                    let mut chars = letters.bytes();
                    if chars.next() != query.bytes().next() || !query.bytes().skip(1).all(|c| chars.any(|n| n == c)) { return None; }
                    2
                };
                Some((rank, name))
            })
            .min_by_key(|&(rank, _)| rank);
        if let Some((rank, name)) = candidate {
            if rank < best {
                found.clear();
                best = rank;
            }
            if rank == best { found.push((glyph, name)); }
        }
    }
    found
}

// Strings and comments are literal even before their language implementation is complete.
pub(crate) fn in_code(text: &str) -> bool {
    let mut quote = None;
    let mut comment = false;
    for c in text.chars() {
        if comment { if c == '\n' { comment = false; } } else if let Some(q) = quote { if c == q { quote = None; } } else if c == '\'' || c == '"' { quote = Some(c); } else if c == '⍝' { comment = true; }
    }
    quote.is_none() && !comment
}

pub(crate) fn entry(line: &str, pos: usize) -> Option<(usize, &str)> {
    let start = line[..pos].rfind('`')?;
    let prefix = &line[start + 1..pos];
    (prefix.bytes().all(|c| c.is_ascii_alphabetic()) && in_code(&line[..start])).then_some((start, prefix))
}

#[derive(Default)]
struct Input {
    // Only a typed prefix auto-expands. Paste, history and cursor movement cancel this state.
    active: bool,
    pending: Option<(Range<usize>, String)>,
}

impl Input {
    fn key(&mut self, key: KeyEvent, line: &str, pos: usize) -> Option<Cmd> {
        if let KeyEvent(KeyCode::Char(c), Modifiers::ALT) = key {
            self.active = false;
            if let Some(&glyph) = alt_keys().get(&c) { return Some(Cmd::Insert(1, glyph.to_string())); }
        }
        let enter = key == KeyEvent::from('\r') || key == KeyEvent::from('\n');
        let tab = key == KeyEvent::from('\t');
        let plain = key.1.is_empty();
        let delimiter = plain && matches!(key.0, KeyCode::Char(c) if !c.is_alphabetic());
        if tab || self.active && (enter || delimiter) {
            if let Some((start, prefix)) = entry(line, pos) {
                let found = matches(prefix);
                if let [(glyph, _)] = found.as_slice() {
                    self.active = key == KeyEvent::from('`');
                    let mut replacement = glyph.to_string();
                    if let KeyCode::Char(c) = key.0 { if plain { replacement.push(c); } }
                    self.pending = Some((start..pos, replacement));
                    if enter {
                        // Rustyline commands cannot combine replacement and submission. Apply this
                        // one accepted edit after readline, before history/evaluation; never rewrite a paste.
                        return Some(Cmd::AcceptLine);
                    }
                    // Completion's update replaces the byte range and advances the cursor.
                    return Some(Cmd::Complete);
                }
                if tab {
                    self.active = true;
                    return Some(Cmd::Complete);
                }
            }
        }
        self.active = match key {
            KeyEvent(KeyCode::Char('`'), Modifiers::NONE) => in_code(&line[..pos]),
            KeyEvent(KeyCode::Char(c), Modifiers::NONE) if c.is_ascii_alphabetic() => self.active,
            KeyEvent(KeyCode::Backspace, Modifiers::NONE) => self.active,
            _ => false,
        };
        None
    }
}

#[derive(Clone, Default)]
struct Symbols(Arc<Mutex<Input>>);

impl ConditionalEventHandler for Symbols {
    fn handle(&self, event: &Event, count: RepeatCount, positive: bool, ctx: &EventContext) -> Option<Cmd> {
        let mut input = self.0.lock().unwrap();
        if count != 1 || !positive {
            input.active = false;
            return None;
        }
        input.key(*event.get(0)?, ctx.line(), ctx.pos())
    }
}

impl Completer for Symbols {
    type Candidate = Pair;
    fn complete(&self, line: &str, pos: usize, _: &Context) -> rustyline::Result<(usize, Vec<Pair>)> {
        if let Some((range, replacement)) = self.0.lock().unwrap().pending.take() {
            return Ok((range.start, vec![Pair { display: replacement.clone(), replacement }]));
        }
        let Some((start, prefix)) = entry(line, pos) else { return Ok((pos, vec![])); };
        // List ambiguous names without extending their common prefix: `sca must not
        // silently become the exact name `scan just because scan-first shares that prefix.
        let choices =
            matches(prefix).into_iter().map(|(glyph, name)| Pair { display: format!("{glyph} {name}"), replacement: line[start..pos].into() }).collect();
        Ok((start, choices))
    }
}

struct Suggestion(String);
impl Hint for Suggestion {
    fn display(&self) -> &str { &self.0 }
    fn completion(&self) -> Option<&str> { None }
}

impl Hinter for Symbols {
    type Hint = Suggestion;
    fn hint(&self, line: &str, pos: usize, _: &Context) -> Option<Suggestion> {
        let (_, prefix) = entry(line, pos)?;
        let found = matches(prefix);
        let message = if prefix.is_empty() { "Tab: symbol names".into() } else if found.is_empty() { "unknown symbol".into() } else {
            let mut names = found.iter().take(6).map(|(glyph, name)| format!("{glyph} {name}")).collect::<Vec<_>>().join(", ");
            if found.len() > 6 { names.push_str(", … (Tab)"); }
            names
        };
        Some(Suggestion(format!("  [{message}]")))
    }
}

impl Validator for Symbols {
    fn validate(&self, _: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let input = self.0.lock().unwrap();
        Ok(ValidationResult::Valid(input.pending.as_ref().map(|(_, glyph)| format!("  → {glyph}"))))
    }
}
impl Highlighter for Symbols {}
impl Helper for Symbols {}

pub(crate) struct LineEditor(Editor<Symbols, DefaultHistory>);
impl LineEditor {
    pub fn new() -> rustyline::Result<Self> {
        let mut editor = Editor::with_config(Config::builder().completion_type(CompletionType::List).build())?;
        let symbols = Symbols::default();
        editor.bind_sequence(Event::Any, EventHandler::Conditional(Box::new(symbols.clone())));
        editor.set_helper(Some(symbols));
        Ok(Self(editor))
    }

    pub fn readline(&mut self, prompt: &str) -> rustyline::Result<String> {
        *self.0.helper().unwrap().0.lock().unwrap() = Input::default();
        let mut line = self.0.readline(prompt)?;
        if let Some((range, glyph)) = self.0.helper().unwrap().0.lock().unwrap().pending.take() { line.replace_range(range, &glyph); }
        self.0.add_history_entry(line.as_str())?;
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alt_layout_covers_glyphs_and_inserts_literal_characters() {
        let mut input = Input::default();
        for (&key, &glyph) in alt_keys() {
            assert!(key.is_ascii() && !glyph.is_ascii());
            for line in ["", "'", "⍝ "] {
                assert_eq!(input.key(KeyEvent::new(key, Modifiers::ALT), line, line.len()), Some(Cmd::Insert(1, glyph.to_string())));
            }
        }
        for &(glyph, _) in SYMBOLS {
            if glyph.chars().count() == 1 && !glyph.is_ascii() { assert!(alt_keys().values().any(|&c| glyph.starts_with(c)), "{glyph}"); }
        }
    }

    #[test]
    fn names_prefixes_and_literal_context() {
        for (name, glyph) in
            [("io", "⍳"), ("RHO", "⍴"), ("scan", "\\"), ("scanfirst", "⍀"), ("alpha", "⍺"), ("alphaunderbar", "⍶"), ("omegaunderbar", "⍹"), ("replicate", "/")]
        { assert_eq!(matches(name).iter().map(|(g, _)| *g).collect::<Vec<_>>(), [glyph]); }
        assert!(matches("sca").len() > 1);
        for name in ["lar", "larr", "leftar"] { assert_eq!(matches(name), [("←", "left-arrow")]); }
        assert_eq!(matches("grup"), [("⍋", "grade-up")]);
        assert!(matches("nosuchsymbol").is_empty());
        for text in ["'`io", "'can''t `io", "\"`io", "⍝ `io"] { assert!(entry(text, text.len()).is_none()); }
        for text in ["界+`io", "'text' `io", "⍝ comment\n`io"] { assert_eq!(entry(text, text.len()).unwrap().1, "io"); }
        for &(glyph, names) in SYMBOLS { for name in names.split_whitespace() { assert_eq!(matches(&name.replace('-', "")), [(glyph, name)]); } }
    }

    #[test]
    fn accepting_keys_do_not_rewrite_other_input() {
        let line = "界+`io";
        let mut input = Input { active: true, pending: None };
        assert_eq!(input.key(KeyEvent::from('3'), line, line.len()), Some(Cmd::Complete));
        assert_eq!(input.pending.take(), Some((4..7, "⍳3".into())));
        input.active = true;
        assert_eq!(input.key(KeyEvent::from('\t'), line, line.len()), Some(Cmd::Complete));
        assert_eq!(input.pending.take(), Some((4..7, "⍳".into())));
        input.active = true;
        assert_eq!(input.key(KeyEvent::from('\r'), line, line.len()), Some(Cmd::AcceptLine));
        assert_eq!(input.pending.take(), Some((4..7, "⍳".into())));
        for key in [KeyCode::BracketedPasteStart, KeyCode::Left, KeyCode::Up, KeyCode::Esc] {
            input.active = true;
            assert_eq!(input.key(KeyEvent(key, Modifiers::NONE), line, line.len()), None);
            assert_eq!(input.key(KeyEvent::from('\r'), line, line.len()), None);
            assert!(input.pending.is_none());
        }
        input.active = true;
        assert_eq!(input.key(KeyEvent::from(' '), "`sca", 4), None);
        input.active = true;
        assert_eq!(input.key(KeyEvent::from('-'), "x`lar", 5), Some(Cmd::Complete));
        assert_eq!(input.pending.take(), Some((1..5, "←-".into())));
        input.active = true;
        assert_eq!(input.key(KeyEvent::from('-'), "`sca", 4), None);
        assert!(input.pending.is_none());
        assert_eq!(input.key(KeyEvent::from('\\'), "1", 1), None);
    }
}
