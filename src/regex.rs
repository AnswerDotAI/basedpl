use crate::{
    array::generated_len,
    data::text,
    execution::Context,
    keyed,
    primitive::integer,
    system::{Call, SystemFunction},
    Error, ErrorKind, Function, Value,
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

pub(crate) fn compile(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•r is monadic")); }
    let pattern = text(right, span)?;
    let regex = Arc::new(Regex::new(&pattern).map_err(|e| span.error(ErrorKind::Domain, e.to_string()))?);
    let operations = [
        ("match", "•r.match", Operation::Match),
        ("position", "•r.position", Operation::Position),
        ("length", "•r.length", Operation::Length),
        ("groups", "•r.groups", Operation::Groups),
        ("replace", "•r.replace", Operation::Replace),
    ];
    let keys = operations.iter().map(|(key, _, _)| (*key).into()).collect();
    let functions =
        operations.into_iter().map(|(_, name, op)| Value::Function(Function::system(SystemFunction { name, call: Call::Regex(regex.clone(), op) }))).collect();
    keyed::vector(keys, functions).map_err(|k| span.error(k, "invalid regex functions"))
}

fn vector(values: Vec<Value>, empty_prototype: Value, span: &Context<'_>) -> Result<Value, Error> {
    generated_len(&[values.len()]).map_err(|k| span.error(k, "regex result too large"))?;
    Value::from_parts(vec![values.len()], values, empty_prototype).map_err(|k| span.error(k, "invalid regex result"))
}

pub(crate) fn call(regex: &Regex, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() != matches!(op, Operation::Replace) { return Err(span.error(ErrorKind::Syntax, "regex search is monadic; replacement is dyadic")); }
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
            if result.len() > 4 * crate::array::MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "regex result too large")); }
            end = m.end();
        }
        result.push_str(&source[end..]);
        generated_len(&[result.chars().count()]).map_err(|k| span.error(k, "regex result too large"))?;
        return Ok(keyed::text(&result));
    }
    let empty = keyed::text("");
    if let Operation::Groups = op {
        let prototype = vector(vec![empty.clone(); regex.captures_len() - 1], empty.clone(), span)?;
        let values = regex
            .captures_iter(&source)
            .map(|caps| {
                span.check()?;
                vector(caps.iter().skip(1).map(|m| keyed::text(m.map_or("", |m| m.as_str()))).collect(), empty.clone(), span)
            })
            .collect::<Result<_, _>>()?;
        return vector(values, prototype, span);
    }
    let mut previous = 0;
    let mut position = 1;
    let values = regex
        .find_iter(&source)
        .map(|m| {
            span.check()?;
            Ok(match op {
                Operation::Match => keyed::text(m.as_str()),
                Operation::Length => integer(m.as_str().chars().count() as i64),
                Operation::Position => {
                    position += source[previous..m.start()].chars().count();
                    previous = m.start();
                    integer(position as i64)
                }
                _ => unreachable!(),
            })
        })
        .collect::<Result<_, Error>>()?;
    vector(values, if matches!(op, Operation::Match) { empty } else { integer(0) }, span)
}
