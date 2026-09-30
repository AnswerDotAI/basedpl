use crate::{
    array::generated_len,
    data::text,
    execution::Context,
    keyed,
    system::{
        natives, Call,
        Valence::{Dyadic, Monadic},
    },
    Error, ErrorAt, Value,
};
use ::regex::Regex;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Match,
    Position,
    Length,
    Groups,
    Replace,
}

pub(crate) fn compile(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let pattern = text(right, span)?;
    let regex = Arc::new(Regex::new(&pattern).map_err(|e| span.domain_error(e.to_string()))?);
    let operations = [
        ("match", Operation::Match),
        ("position", Operation::Position),
        ("length", Operation::Length),
        ("groups", Operation::Groups),
        ("replace", Operation::Replace),
    ];
    Ok(natives(operations.map(|(name, op)| (name, Call::Regex(regex.clone(), op), if let Operation::Replace = op { Dyadic } else { Monadic }))))
}

fn vector(values: Vec<Value>, empty_prototype: Value, span: &Context<'_>) -> Result<Value, Error> {
    generated_len(&[values.len()]).error_at(span, "regex result too large")?;
    Value::from_parts(vec![values.len()], values, empty_prototype).error_at(span, "invalid regex result")
}

pub(crate) fn call(regex: &Regex, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let source = text(right, span)?;
    if let Operation::Replace = op {
        let replacement = text(left.unwrap(), span)?;
        let mut result = String::new();
        let mut end = 0;
        for captures in regex.captures_iter(&source) {
            span.check()?;
            let m = captures.get(0).unwrap();
            result.push_str(&source[end..m.start()]);
            captures.expand(&replacement, &mut result);
            generated_len(&[result.len() / 4]).error_at(span, "regex result too large")?;
            end = m.end();
        }
        result.push_str(&source[end..]);
        generated_len(&[result.chars().count()]).error_at(span, "regex result too large")?;
        return Ok(keyed::text(&result));
    }
    let empty = keyed::text("");
    if let Operation::Groups = op {
        let prototype = vector(vec![empty.clone(); regex.captures_len() - 1], empty.clone(), span)?;
        let values = regex
            .captures_iter(&source)
            .map(|caps| { span.check()?; vector(caps.iter().skip(1).map(|m| keyed::text(m.map_or("", |m| m.as_str()))).collect(), empty.clone(), span) })
            .collect::<Result<_, _>>()?;
        return vector(values, prototype, span);
    }
    if let Operation::Match = op {
        let values = regex.find_iter(&source).map(|m| span.check().map(|_| keyed::text(m.as_str()))).collect::<Result<_, _>>()?;
        return vector(values, empty, span);
    }
    let (mut previous, mut position) = (0, 0);
    let numbers = regex
        .find_iter(&source)
        .map(|m| {
            span.check()?;
            if let Operation::Length = op { return Ok(m.as_str().chars().count() as i64); }
            position += source[previous..m.start()].chars().count();
            previous = m.start();
            Ok(position as i64)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    generated_len(&[numbers.len()]).and_then(|_| Value::integers(vec![numbers.len()], numbers)).error_at(span, "regex result too large")
}
