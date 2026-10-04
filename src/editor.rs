//! Glyph completion and terminal input. Source execution never rewrites aliases.
use crate::symbols::{chord, find, layout, symbols, Action, Symbol};
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
    borrow::Cow,
    ops::Range,
    sync::{Arc, Mutex},
};

fn label((glyph, name): &(&str, &str)) -> String { format!("{glyph} {name}{}", chord(glyph)) }

// At each level (exact, prefix, prefixes of hyphen-separated parts) a name outranks a search word.
pub(crate) fn matches(query: &str) -> Vec<(&'static str, &'static str)> {
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

// Strings and comments are literal even before their language implementation is complete.
pub(crate) fn in_code(text: &str) -> bool {
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        let closed = match c {
            '⍝' => chars.by_ref().any(|c| c == '\n'),
            '"' => chars.by_ref().any(|c| c == '"'),
            '\'' => chars.nth(1).is_some(),
            _ => true,
        };
        if !closed { return false; }
    }
    true
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
    // The dead-key state that the next key completes.
    dead: Option<String>,
}

impl Input {
    /// Types an action's text, or waits in its dead-key state for the next key.
    fn act(&mut self, action: &Action) -> Cmd {
        match action {
            Action::Text(text) => Cmd::Insert(1, text.clone()),
            Action::State(name) => {
                self.dead = Some(name.clone());
                Cmd::Repaint
            }
        }
    }

    /// The next key after a dead key. A listed key types its text or moves on. Repeating the chord or Space types the terminator. Backspace and
    /// Escape cancel and type nothing. Any other key types the terminator, then acts as if nothing were pending. Enter submits
    /// the line with the terminator added.
    fn pending_key(&mut self, name: &str, key: KeyEvent, line: &str, pos: usize) -> Cmd {
        let state = layout().state(name);
        match key {
            KeyEvent(KeyCode::Char(c), Modifiers::ALT) if matches!(layout().alt(c), Some(Action::State(next)) if next == name) => {
                return Cmd::Insert(1, state.terminator.clone())
            }
            KeyEvent(KeyCode::Char(' '), Modifiers::NONE) => return Cmd::Insert(1, state.terminator.clone()),
            KeyEvent(KeyCode::Backspace | KeyCode::Esc, _) => return Cmd::Repaint,
            KeyEvent(KeyCode::Char(c), m) if m.difference(Modifiers::SHIFT).is_empty() => {
                if let Some(action) = find(&state.keys, c) { return self.act(action); }
            }
            _ => {}
        }
        if key == KeyEvent::from('\r') || key == KeyEvent::from('\n') {
            self.pending = Some((pos..pos, state.terminator.clone()));
            return Cmd::AcceptLine;
        }
        match self.key(key, line, pos) {
            Some(Cmd::Insert(n, text)) => Cmd::Insert(n, format!("{}{text}", state.terminator)),
            _ => match key {
                KeyEvent(KeyCode::Char(c), m) if m.difference(Modifiers::SHIFT).is_empty() => Cmd::Insert(1, format!("{}{c}", state.terminator)),
                _ => Cmd::Insert(1, state.terminator.clone()),
            },
        }
    }
    fn key(&mut self, key: KeyEvent, line: &str, pos: usize) -> Option<Cmd> {
        if let Some(name) = self.dead.take() { return Some(self.pending_key(&name, key, line, pos)); }
        if let KeyEvent(KeyCode::Char(c), Modifiers::ALT) = key {
            self.active = false;
            if let Some(action) = layout().alt(c) { return Some(self.act(action)); }
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
        // List ambiguous names without extending their common prefix: the typed text stays as it is.
        let choices = matches(prefix).into_iter().map(|found| Pair { display: label(&found), replacement: line[start..pos].into() }).collect();
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
        if let Some(name) = &self.0.lock().unwrap().dead {
            let state = layout().state(name);
            let keys: String = state.keys.iter().map(|(key, _)| key).collect();
            return Some(Suggestion(format!("  {}:{keys}", state.terminator)));
        }
        let (_, prefix) = entry(line, pos)?;
        let found = matches(prefix);
        let message = if prefix.is_empty() { "Tab: symbol names".into() } else if found.is_empty() { "unknown symbol".into() } else {
            let mut names = found.iter().take(6).map(label).collect::<Vec<_>>().join(", ");
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
// Bold cyan glyph and dim key, so each listed entry reads as glyph, name, key.
fn styled(entry: &str) -> String {
    let mut parts = entry.splitn(3, ' ');
    let (glyph, name, key) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next());
    if name.is_empty() || !symbols().iter().any(|s| s.glyph == glyph) { return entry.into(); }
    format!("\x1b[1;36m{glyph}\x1b[0m {name}{}", key.map_or(String::new(), |key| format!(" \x1b[2m{key}\x1b[0m")))
}

impl Highlighter for Symbols {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        let Some(inner) = hint.strip_prefix("  [").and_then(|hint| hint.strip_suffix(']')) else { return hint.into() };
        format!("  [{}]", inner.split(", ").map(styled).collect::<Vec<_>>().join(", ")).into()
    }
    fn highlight_candidate<'c>(&self, candidate: &'c str, _: CompletionType) -> Cow<'c, str> { styled(candidate).into() }
}
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
