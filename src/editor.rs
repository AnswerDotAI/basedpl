//! Terminal-only symbol entry. Neither the parser nor noninteractive input uses this module.
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
    ops::Range,
    sync::{Arc, Mutex},
};

// One row per glyph: ambiguity is between glyphs, not between aliases for the same glyph.
// This is an input catalogue, not a claim that every primitive is implemented yet.
const SYMBOLS: &[(&str, &str)] = &[
    ("←", "assign leftarrow"),
    ("⍳", "iota indexof"),
    ("⍴", "rho shape reshape"),
    ("≢", "tally notmatch"),
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
    ("○", "circle pi"),
    ("!", "factorial binomial"),
    ("∧", "and lcm"),
    ("∨", "or gcd"),
    ("⍲", "nand"),
    ("⍱", "nor"),
    ("~", "not without"),
    ("=", "equal"),
    ("≠", "notequal"),
    ("<", "less"),
    ("≤", "lessequal"),
    (">", "greater"),
    ("≥", "greaterequal"),
    ("⎕", "quad"),
    ("⍞", "quotequad"),
    ("⍺", "alpha"),
    ("⍵", "omega"),
    ("⍺⍺", "alphaalpha leftoperand"),
    ("⍵⍵", "omegaomega rightoperand"),
    ("∇", "del recursion"),
    ("∇∇", "operatorrecursion"),
    ("⍝", "comment"),
    ("⋄", "diamond"),
    ("¯", "overbar"),
    ("⍬", "zilde empty"),
    (",", "ravel catenate"),
    ("⍪", "table catenatefirst"),
    ("⊂", "enclose"),
    ("⊃", "disclose pick"),
    ("⊆", "nest partition"),
    ("∊", "epsilon enlist member"),
    ("∪", "unique union"),
    ("∩", "intersection"),
    ("⍋", "gradeup"),
    ("⍒", "gradedown"),
    ("↑", "take mix"),
    ("↓", "drop split"),
    ("⌽", "reverse rotate"),
    ("⊖", "reversefirst rotatefirst"),
    ("⍉", "transpose"),
    ("⊤", "encode"),
    ("⊥", "decode"),
    ("⍎", "execute"),
    ("⍕", "format"),
    ("⌷", "index squad"),
    ("⌹", "domino matrixdivide"),
    ("¨", "each dieresis"),
    ("/", "reduce replicate slash"),
    ("⌿", "reducefirst replicatefirst"),
    ("\\", "scan backslash"),
    ("⍀", "scanfirst"),
    ("⍤", "rank atop"),
    ("∘", "jot compose bind"),
    ("⍨", "commute selfie"),
    ("⍥", "over"),
    ("⍛", "behind"),
    ("⍣", "repeat iterate"),
    ("⍸", "where intervalindex"),
    ("⍷", "find"),
    ("⊢", "right same"),
    ("⊣", "left"),
    ("⌸", "key"),
    ("@", "at"),
    ("⌺", "stencil"),
    ("⍠", "variant"),
    ("?", "roll deal"),
];

fn matches(prefix: &str) -> Vec<(&'static str, &'static str)> {
    let prefix = prefix.to_ascii_lowercase();
    let mut found = Vec::new();
    for &(glyph, names) in SYMBOLS {
        if names.split_whitespace().any(|name| name == prefix) { return vec![(glyph, names.split_whitespace().find(|name| *name == prefix).unwrap())]; }
        if let Some(name) = names.split_whitespace().find(|name| name.starts_with(&prefix)) { found.push((glyph, name)); }
    }
    found
}

// Strings and comments are literal even before their language implementation is complete.
fn in_code(text: &str) -> bool {
    let mut quote = None;
    let mut comment = false;
    for c in text.chars() {
        if comment { if c == '\n' { comment = false; } } else if let Some(q) = quote { if c == q { quote = None; } } else if c == '\'' || c == '"' { quote = Some(c); } else if c == '⍝' { comment = true; }
    }
    quote.is_none() && !comment
}

fn entry(line: &str, pos: usize) -> Option<(usize, &str)> {
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
        // silently become the exact name `scan just because scanfirst shares that prefix.
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
    fn names_prefixes_and_literal_context() {
        for (name, glyph) in [("io", "⍳"), ("RHO", "⍴"), ("scan", "\\"), ("scanfirst", "⍀"), ("alpha", "⍺"), ("alphaalpha", "⍺⍺"), ("replicate", "/")]
        { assert_eq!(matches(name).iter().map(|(g, _)| *g).collect::<Vec<_>>(), [glyph]); }
        assert!(matches("sca").len() > 1);
        assert!(matches("nosuchsymbol").is_empty());
        for text in ["'`io", "'can''t `io", "\"`io", "⍝ `io"] { assert!(entry(text, text.len()).is_none()); }
        for text in ["界+`io", "'text' `io", "⍝ comment\n`io"] { assert_eq!(entry(text, text.len()).unwrap().1, "io"); }
        for &(glyph, names) in SYMBOLS { for name in names.split_whitespace() { assert_eq!(matches(name), [(glyph, name)]); } }
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
        assert_eq!(input.key(KeyEvent::from('\\'), "1", 1), None);
    }
}
