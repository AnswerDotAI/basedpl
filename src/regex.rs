//! `•r pattern` compiles a regular expression into a record of two functions: `matches`, which gives a table of the matches in a text, and `replace`. The host compiles, matches and replaces, each with its own syntax for patterns and replacement templates: natively the regex crate's, and in the browser JavaScript's.
use crate::execution::check;
use crate::{
    array::generated_len, data::text, Span, host::Regex, keyed, system::{natives, Call, Valence::{Dyadic, Monadic}}, Described, Error, Value
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Operation { Matches, Replace }

/// One match: its text, its position in characters from 0, and each group's text, which is empty for a group that took no part.
pub(crate) struct Match { pub text: String, pub position: usize, pub groups: Vec<String> }

pub(crate) fn compile(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let pattern = text(right)?;
    let regex = Arc::new(Regex::new(&pattern).map_err(|e| Error::domain(e))?);
    Ok(natives([("matches", Call::Regex(regex.clone(), Operation::Matches), Monadic), ("replace", Call::Regex(regex, Operation::Replace), Dyadic)]))
}

/// A vector of `texts`.
fn texts(texts: Vec<String>) -> Result<Value, Error> {
    keyed::texts(&[texts.len()], texts.iter().map(|t| keyed::text(t)).collect()).described("invalid regex result")
}

pub(crate) fn call(regex: &Regex, op: Operation, left: Option<&Value>, right: &Value) -> Result<Value, Error> {
    let source = text(right)?;
    if let Operation::Replace = op {
        let result = regex.replace(&text(left.unwrap())?, &source)?;
        generated_len(&[result.chars().count()])?;
        return Ok(keyed::text(&result));
    }
    let mut rows = Vec::new();
    regex.each(&source, |m| {
        check()?;
        rows.push(vec![keyed::text(&m.text), Value::integer(m.position as i64), texts(m.groups)?]);
        generated_len(&[rows.len()]).map_err(Error::from).map(drop)
    })?;
    let empty = vec![keyed::text(""), Value::integer(0), texts(vec![String::new(); regex.groups()])?];
    keyed::table(&["text", "position", "groups"], &[rows.len()], rows, empty).described("invalid regex result")
}
