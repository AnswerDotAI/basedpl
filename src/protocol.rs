use crate::{Array, Element, Error, Evaluation, Session};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(serde::Deserialize)]
struct Request { code: String }

fn element(e: &Element) -> Value {
    match e {
        Element::Number(n) => match n.as_exact() {
            Some(n) => json!({"rational": [n.numer().to_string(), n.denom().to_string()]}),
            None => json!(n.as_float().unwrap()),
        },
        Element::Character(c) => json!(c.to_string()),
        Element::Nested(a) => array(a),
    }
}

fn array(a: &Array) -> Value { json!({"shape": a.shape(), "data": a.data().iter().map(element).collect::<Vec<_>>(), "prototype": element(a.prototype())}) }

fn error(e: &Error) -> Value {
    json!({"kind": e.kind.to_string(), "message": e.message,
        "source": {"name": e.span.source.name, "text": e.span.source.text}, "span": [e.span.range.start, e.span.range.end],
        "calls": e.calls.iter().map(|s| json!({"source": {"name": s.source.name, "text": s.source.text}, "span": [s.range.start, s.range.end]})).collect::<Vec<_>>()})
}

fn response(result: Evaluation) -> Value {
    json!({"value": result.value.as_ref().map(array), "output": result.output, "error": result.error.as_ref().map(error)})
}

pub(crate) fn run(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<()> {
    let mut session = Session::new();
    for line in input.lines() {
        let line = line?;
        // Serde structs also accept positional JSON arrays; this protocol requires objects.
        let request = line.trim_start().starts_with('{').then(|| serde_json::from_str::<Request>(&line).ok()).flatten();
        let reply = match request {
            Some(request) => response(session.eval(&request.code)),
            None => json!({"value": null, "output": [], "error": {"kind": "REQUEST ERROR", "message": "expected a JSON object with a code string"}}),
        };
        serde_json::to_writer(&mut *output, &reply)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
