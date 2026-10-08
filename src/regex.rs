//! `•r pattern` compiles a regular expression into a record of two functions: `matches`, which gives a table of the matches in a
//! text, and `replace`. The host compiles, matches and replaces, each with its own syntax for patterns and replacement templates:
//! natively the regex crate's, and in the browser JavaScript's.
use crate::{
    array::generated_len,
    data::text,
    execution::Context,
    host::Regex,
    keyed,
    system::{
        natives, Call,
        Valence::{Dyadic, Monadic},
    },
    Error, ErrorAt, Number, Value,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Operation { Matches, Replace }

/// One match: its text, its position in characters from 0, and each group's text, which is empty for a group that took no part.
pub(crate) struct Match { pub text: String, pub position: usize, pub groups: Vec<String> }

pub(crate) fn compile(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let pattern = text(right, span)?;
    let regex = Arc::new(Regex::new(&pattern).map_err(|e| span.domain_error(e))?);
    Ok(natives([("matches", Call::Regex(regex.clone(), Operation::Matches), Monadic), ("replace", Call::Regex(regex, Operation::Replace), Dyadic)]))
}

/// A vector of `texts`.
fn texts(texts: Vec<String>, span: &Context<'_>) -> Result<Value, Error> {
    keyed::texts(&[texts.len()], texts.iter().map(|t| keyed::text(t)).collect()).error_at(span, "invalid regex result")
}

pub(crate) fn call(regex: &Regex, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let source = text(right, span)?;
    if let Operation::Replace = op {
        let result = regex.replace(&text(left.unwrap(), span)?, &source, span)?;
        generated_len(&[result.chars().count()]).error_at(span, "regex result too large")?;
        return Ok(keyed::text(&result));
    }
    let mut rows = Vec::new();
    regex.each(&source, |m| {
        span.check()?;
        rows.push(vec![keyed::text(&m.text), Value::Number(Number::from_integer(m.position as i64)), texts(m.groups, span)?]);
        generated_len(&[rows.len()]).error_at(span, "regex result too large").map(drop)
    })?;
    let empty = vec![keyed::text(""), Value::Number(Number::from_integer(0)), texts(vec![String::new(); regex.groups()], span)?];
    keyed::table(&["text", "position", "groups"], &[rows.len()], rows, empty).error_at(span, "invalid regex result")
}
