use crate::{
    display,
    execution::Context,
    keyed,
    system::{Call, SystemFunction},
    Error, ErrorKind, Function, Value,
};

fn invalid(span: &Context<'_>, message: &str) -> Error { span.error(ErrorKind::Domain, message) }

fn valid_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_alphabetic() || c == '_') && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

pub(crate) fn factory(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•element is monadic")); }
    let tag = keyed::name(right).filter(|s| valid_name(s)).ok_or_else(|| invalid(span, "invalid XML element name"))?;
    Ok(Value::Function(Function::system(SystemFunction { name: "•element", call: Call::Element(tag.into()) })))
}

pub(crate) fn element(tag: &str, attrs: Option<&Value>, children: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let attrs = attrs.cloned().unwrap_or_else(|| keyed::vector(vec![], vec![]).unwrap());
    keyed::vector(vec!["tag".into(), "attrs".into(), "children".into()], vec![keyed::text(tag), attrs, children.clone()])
        .map_err(|k| span.error(k, "XML element exceeds array limits"))
}

fn attribute(value: &Value, span: &Context<'_>) -> Result<String, Error> {
    if let Some(text) = keyed::name(value) { return Ok(text.to_string()); }
    if value.shape().len() > 1 { return Err(invalid(span, "XML attributes must be text or numeric scalars/vectors")); }
    value
        .elements()
        .map(|v| {
            let n = v.as_number().ok_or_else(|| invalid(span, "XML attributes must be text or numbers"))?;
            if let Some(i) = n.as_integer() { return Ok(i.to_string()); }
            if let Some(q) = n.as_exact().filter(|q| q.is_integer()) { return Ok(q.numer().to_string()); }
            let x = n.to_float().map_err(|e| invalid(span, e))?;
            if !x.is_finite() { return Err(invalid(span, "XML numeric attributes must be finite")); }
            Ok(x.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.join(" "))
}

fn escape(text: &str, output: &mut String) {
    for c in text.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(c),
        }
    }
}

fn write(value: &Value, output: &mut String, span: &Context<'_>) -> Result<(), Error> {
    span.check()?;
    if let Some(text) = keyed::name(value) {
        escape(&text, output);
        return Ok(());
    }
    if value.keys(0).is_none() {
        if value.is_atom() { return Err(invalid(span, "XML children must be text or elements")); }
        for child in value.elements() { write(&child, output, span)?; }
        return Ok(());
    }
    let tag = keyed::field(value, "tag").and_then(|v| keyed::name(&v)).filter(|s| valid_name(s)).ok_or_else(|| invalid(span, "invalid XML element"))?;
    let attrs = keyed::field(value, "attrs").ok_or_else(|| invalid(span, "XML element needs attrs"))?;
    let children = keyed::field(value, "children").ok_or_else(|| invalid(span, "XML element needs children"))?;
    output.push('<');
    output.push_str(&tag);
    for (name, value) in keyed::pairs(&attrs).map_err(|k| span.error(k, "XML attributes must be a keyed vector"))? {
        if !valid_name(&name) { return Err(invalid(span, "invalid XML attribute name")); }
        output.push(' ');
        output.push_str(&name);
        output.push_str("=\"");
        escape(&attribute(&value, span)?, output);
        output.push('"');
    }
    if children.is_empty() {
        output.push_str("/>");
        return Ok(());
    }
    output.push('>');
    write(&children, output, span)?;
    output.push_str("</");
    output.push_str(&tag);
    output.push('>');
    Ok(())
}

pub(crate) fn serialize(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•xml is monadic")); }
    let mut output = String::new();
    write(right, &mut output, span)?;
    Ok(keyed::text(&output))
}

pub(crate) fn svg(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let defaults = keyed::vector(vec!["xmlns".into(), "viewBox".into()], vec![keyed::text("http://www.w3.org/2000/svg"), keyed::text("0 0 100 100")]).unwrap();
    let attrs = left.map_or(Ok(defaults.clone()), |a| keyed::merge(&defaults, a)).map_err(|k| span.error(k, "invalid SVG attributes"))?;
    let tree = element("svg", Some(&attrs), right, span)?;
    display::with_renderer(&tree, "svg-renderer", render_svg).map_err(|k| span.error(k, "SVG exceeds array limits"))
}

fn render_svg(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    display::svg(serialize(left, right, span)?).map_err(|k| span.error(k, "invalid SVG MIME bundle"))
}
