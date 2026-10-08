use crate::{
    display,
    execution::Context,
    keyed,
    system::{native, Call, Valence::Ambivalent},
    DomainAt, Error, ErrorAt, Value,
};
fn valid_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_alphabetic() || c == '_') && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

/// The function that builds elements tagged `tag`, as `•element` returns it.
pub(crate) fn element_function(tag: std::sync::Arc<str>) -> Value { Value::Function(native("•element", Call::Element(tag), Ambivalent)) }

pub(crate) fn factory(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let tag = keyed::name(right).filter(|s| valid_name(s)).ok_or_else(|| span.domain_error("invalid XML element name"))?;
    Ok(element_function(tag))
}

pub(crate) fn element(tag: &str, attrs: Option<&Value>, children: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let attrs = attrs.cloned().unwrap_or_else(keyed::empty_record);
    keyed::vector(vec!["tag".into(), "attrs".into(), "children".into()], vec![keyed::text(tag), attrs, children.clone()])
        .and_then(|e| e.with_renderer(display::renderer("html-renderer", render_html)))
        .error_at(span, "XML element exceeds array limits")
}

fn attribute(value: &Value, span: &Context<'_>) -> Result<String, Error> {
    if let Some(text) = keyed::name(value) { return Ok(text.to_string()); }
    if value.shape().len() > 1 { return Err(span.domain_error("XML attributes must be text or numeric units or vectors")); }
    value
        .elements()
        .map(|v| {
            let n = v.as_number().ok_or_else(|| span.domain_error("XML attributes must be text or numbers"))?;
            if let Some(i) = n.as_integer() { return Ok(i.to_string()); }
            if let Some(q) = n.as_exact().filter(|q| q.is_integer()) { return Ok(q.numer().to_string()); }
            let x = n.to_float().domain_at(span)?;
            if !x.is_finite() { return Err(span.domain_error("XML numeric attributes must be finite")); }
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
        if value.is_atom() { return Err(span.domain_error("XML children must be text or elements")); }
        for child in value.elements() { write(&child, output, span)?; }
        return Ok(());
    }
    let tag = keyed::field(value, "tag").and_then(|v| keyed::name(&v)).filter(|s| valid_name(s)).ok_or_else(|| span.domain_error("invalid XML element"))?;
    let attrs = keyed::field(value, "attrs").ok_or_else(|| span.domain_error("XML element needs attrs"))?;
    let children = keyed::field(value, "children").ok_or_else(|| span.domain_error("XML element needs children"))?;
    output.push('<');
    output.push_str(&tag);
    for (name, value) in keyed::pairs(&attrs).error_at(span, "XML attributes must be a keyed vector")? {
        if !valid_name(&name) { return Err(span.domain_error("invalid XML attribute name")); }
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

pub(crate) fn serialize(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut output = String::new();
    write(right, &mut output, span)?;
    Ok(keyed::text(&output))
}

/// `•xml text` reads XML into the elements that `•xml⁻¹` writes, with the same meaning. Comments, processing instructions and text that
/// is only whitespace are dropped.
pub(crate) fn parse(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let text = crate::data::text(right, span)?;
    let document = roxmltree::Document::parse(&text).map_err(|e| span.domain_error(format!("XML {e}")))?;
    read(document.root_element(), span)
}

/// The element `node`, with names written with their prefixes, and the namespaces it declares as `xmlns` attributes.
fn read(node: roxmltree::Node<'_, '_>, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    let named = |space: Option<&str>, name: &str| match space.and_then(|uri| node.lookup_prefix(uri)).filter(|p| !p.is_empty()) {
        Some(prefix) => format!("{prefix}:{name}"),
        None => name.to_string(),
    };
    let inherited: Vec<_> = node.parent_element().map_or(vec![], |parent| parent.namespaces().collect());
    let declared = node.namespaces().filter(|ns| ns.uri() != roxmltree::NS_XML_URI && !inherited.contains(ns));
    let attrs = declared
        .map(|ns| (ns.name().map_or("xmlns".into(), |p| format!("xmlns:{p}")), ns.uri()))
        .chain(node.attributes().map(|a| (named(a.namespace(), a.name()), a.value())));
    let (names, values) = attrs.map(|(name, value)| (name.into(), keyed::text(value))).unzip();
    let children = node
        .children()
        .filter_map(|child| match child.text() {
            _ if child.is_element() => Some(read(child, span)),
            Some(text) if child.is_text() && !text.trim().is_empty() => Some(Ok(keyed::text(text))),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    let children = Value::from_parts(vec![children.len()], children, keyed::text("")).error_at(span, "XML element exceeds array limits")?;
    let attrs = keyed::vector(names, values).error_at(span, "invalid XML attributes")?;
    element(&named(node.tag_name().namespace(), node.tag_name().name()), Some(&attrs), &children, span)
}

pub(crate) fn svg(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let defaults = keyed::vector(vec!["xmlns".into(), "viewBox".into()], vec![keyed::text("http://www.w3.org/2000/svg"), keyed::text("0 0 100 100")]).unwrap();
    let attrs = left.map_or(Ok(defaults.clone()), |a| keyed::merge(&defaults, a)).error_at(span, "invalid SVG attributes")?;
    element("svg", Some(&attrs), right, span)?.with_renderer(display::renderer("svg-renderer", render_svg)).error_at(span, "SVG exceeds array limits")
}

fn render_html(_: Option<&Value>, tree: &Value, span: &Context<'_>) -> Result<Value, Error> { markup("text/html", tree, span) }
fn render_svg(_: Option<&Value>, tree: &Value, span: &Context<'_>) -> Result<Value, Error> { markup("image/svg+xml", tree, span) }
/// The XML text of `tree` as a MIME bundle of type `kind`.
fn markup(kind: &str, tree: &Value, span: &Context<'_>) -> Result<Value, Error> {
    display::mime(kind, serialize(None, tree, span)?).error_at(span, "invalid XML MIME bundle")
}
