//! `text •r pattern` gives a table of the matches of a regular expression in a text, and `text •replace [pattern template]` replaces them. Each thread keeps the patterns it compiled last, so a pattern that a loop uses compiles once (`compiled`). The host owns pattern syntax and matching for this API and help search: fancy-regex natively, JavaScript in the browser.
use crate::execution::check;
use crate::system::HostCall;
use crate::{Described, Error, Value, array::generated_len, data::text, host::Regex, keyed};
use std::{cell::RefCell, sync::Arc};

/// One match: its text, its position in characters from 0, and each group's text, which is empty for a group that took no part.
pub(crate) struct Match { pub text: String, pub position: usize, pub groups: Vec<String> }

/// How many compiled patterns each thread keeps.
const KEPT: usize = 16;

thread_local! {
    /// The patterns that this thread compiled last, with their text, the most recent first.
    static COMPILED: RefCell<Vec<(String, Arc<Regex>)>> = const { RefCell::new(Vec::new()) };
}

/// The pattern whose text is `pattern`, compiled, or kept from an earlier call with the same text.
fn compiled(pattern: &str) -> Result<Arc<Regex>, Error> {
    COMPILED.with_borrow_mut(|kept| {
        let entry = match kept.iter().position(|(text, _)| text == pattern) {
            Some(at) => kept.remove(at), None => (pattern.to_owned(), Arc::new(Regex::new(pattern, false)?))
        };
        let regex = entry.1.clone();
        kept.insert(0, entry);
        kept.truncate(KEPT);
        Ok(regex)
    })
}

/// A vector of `texts`.
fn texts(texts: Vec<String>) -> Result<Value, Error> {
    keyed::texts(&[texts.len()], texts.iter().map(|t| keyed::text(t)).collect()).described("invalid regex result")
}

/// `text •r pattern`: a table of the matches of `pattern` in `text`, with each one's `text`, its `position` in characters from 0, and its `groups`.
pub(crate) fn matches(HostCall { left, right, .. }: HostCall<'_>) -> Result<Value, Error> {
    let regex = compiled(&text(right)?)?;
    let source = text(left.expect("a dyadic system function has a left argument"))?;
    let mut rows = Vec::new();
    regex.each(&source, |m| {
        check()?;
        rows.push(vec![keyed::text(&m.text), Value::integer(m.position as i64), texts(m.groups)?]);
        generated_len(&[rows.len()]).map_err(Error::from).map(drop)
    })?;
    let empty = vec![keyed::text(""), Value::integer(0), texts(vec![String::new(); regex.groups()])?];
    keyed::table(&["text", "position", "groups"], &[rows.len()], rows, empty).described("invalid regex result")
}

/// `text •replace [pattern template]`: `text` with each match of `pattern` replaced by `template`, where `$1` or `${name}` stands for a group and `$$` for `$`.
pub(crate) fn replace(HostCall { left, right, .. }: HostCall<'_>) -> Result<Value, Error> {
    let Some((_, parts)) = keyed::text_items(right).filter(|(shape, _)| shape[..] == [2]) else {
        return Err(Error::domain("•replace takes a pattern and a template"));
    };
    let result = compiled(&parts[0])?.replace(&parts[1], &text(left.expect("a dyadic system function has a left argument"))?)?;
    generated_len(&[result.chars().count()])?;
    Ok(keyed::text(&result))
}
