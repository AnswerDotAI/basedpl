use crate::{Array, Element, Error, Evaluation, Session};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn element(e: &Element) -> Value {
    match e {
        Element::Number(n) => {
            if let Some(n) = n.as_integer() { json!(n) } else if let Some(n) = n.as_exact() {
                if n.is_integer() { Value::Number(n.numer().to_string().parse().expect("decimal integer")) } else { json!({"rational": [n.numer().to_string(), n.denom().to_string()]}) }
            } else if let Some(n) = n.as_complex() { json!({"complex": [n.re, n.im]}) } else { json!(n.as_float().unwrap()) }
        }
        Element::Character(c) => json!(c.to_string()),
        Element::Nested(a) => array(a),
    }
}

fn array(a: &Array) -> Value { json!({"shape": a.shape(), "data": a.elements().map(|e| element(&e)).collect::<Vec<_>>(), "prototype": element(a.prototype())}) }

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
        let reply = match serde_json::from_str::<String>(&line) {
            Ok(code) => response(session.eval(&code)),
            Err(_) => json!({"value": null, "output": [], "error": {"kind": "REQUEST ERROR", "message": "expected a JSON string containing APL source"}}),
        };
        serde_json::to_writer(&mut *output, &reply)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
