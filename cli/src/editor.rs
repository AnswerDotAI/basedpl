//! Glyph completion and terminal input. Source execution never rewrites aliases.
use basedpl::{
    in_code,
    symbols::{chord, entry, find, layout, matches, symbols, Action},
};
use rustyline::{
    completion::{Completer, Pair},
    highlight::{CmdKind, Highlighter},
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
            let keys: String = layout().display_keys(&state.keys).map(|(key, _)| key).collect();
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

/// The ANSI colour of each highlight class the REPL colours. Numbers, names, system names and syntax keep the terminal's colour.
fn colour(class: &str) -> Option<&'static str> {
    Some(match class {
        "function" => "32",
        "monadic-operator" => "35",
        "dyadic-operator" => "33",
        "argument" => "34",
        "string" => "36",
        "comment" => "2",
        _ => return None,
    })
}

impl Highlighter for Symbols {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        let Some(inner) = hint.strip_prefix("  [").and_then(|hint| hint.strip_suffix(']')) else { return hint.into() };
        format!("  [{}]", inner.split(", ").map(styled).collect::<Vec<_>>().join(", ")).into()
    }
    fn highlight_candidate<'c>(&self, candidate: &'c str, _: CompletionType) -> Cow<'c, str> { styled(candidate).into() }
    // Every edit repaints the line, so it always shows its classes, including the final repaint when Enter submits it.
    fn highlight<'l>(&self, line: &'l str, _: usize) -> Cow<'l, str> {
        let (mut out, mut at) = (String::new(), 0);
        for (range, colour) in basedpl::highlight(line).into_iter().filter_map(|(range, class)| Some((range, colour(class)?))) {
            out.push_str(&format!("{}\x1b[{colour}m{}\x1b[0m", &line[at..range.start], &line[range.start..range.end]));
            at = range.end;
        }
        out.push_str(&line[at..]);
        out.into()
    }
    fn highlight_char(&self, _: &str, _: usize, _: CmdKind) -> bool { true }
}
impl Helper for Symbols {}

pub(crate) struct LineEditor { editor: Editor<Symbols, DefaultHistory>, history: Option<std::path::PathBuf> }
impl LineEditor {
    pub fn new() -> rustyline::Result<Self> {
        let mut editor = Editor::with_config(Config::builder().max_history_size(1000)?.completion_type(CompletionType::List).build())?;
        let symbols = Symbols::default();
        editor.bind_sequence(Event::Any, EventHandler::Conditional(Box::new(symbols.clone())));
        editor.set_helper(Some(symbols));
        let config = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).filter(|dir| dir.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config")));
        let history = config.map(|dir| dir.join("basedpl/history"));
        if let Some(path) = &history {
            match editor.load_history(path) {
                Ok(()) => {},
                Err(rustyline::error::ReadlineError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {},
                Err(e) => eprintln!("History: {e}"),
            }
        }
        Ok(Self { editor, history })
    }

    pub fn readline(&mut self, prompt: &str) -> rustyline::Result<String> {
        *self.editor.helper().unwrap().0.lock().unwrap() = Input::default();
        let mut line = self.editor.readline(prompt)?;
        if let Some((range, glyph)) = self.editor.helper().unwrap().0.lock().unwrap().pending.take() { line.replace_range(range, &glyph); }
        Ok(line)
    }
    pub fn remember(&mut self, code: &str) -> rustyline::Result<()> { self.editor.add_history_entry(code.trim_end()).map(|_| ()) }
}

impl Drop for LineEditor {
    fn drop(&mut self) {
        if let Some(path) = &self.history {
            let result = std::fs::create_dir_all(path.parent().unwrap()).map_err(rustyline::error::ReadlineError::Io)
                .and_then(|_| self.editor.append_history(path));
            if let Err(e) = result { eprintln!("History: {e}"); }
        }
    }
}
