use crate::execution::check;
use crate::{Described, Error, Span, Value, display, keyed, system::{Call, Valence::Ambivalent, native}};
fn valid_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_alphabetic() || c == '_') && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

/// The function that builds elements tagged `tag`, as `•element` returns it.
pub(crate) fn element_function(tag: std::sync::Arc<str>) -> Value { Value::Function(native("•element", Call::Element(tag), Ambivalent)) }

pub(crate) fn factory(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let tag = keyed::name(right).filter(|s| valid_name(s)).ok_or_else(|| Error::domain("invalid XML element name"))?;
    Ok(element_function(tag))
}

pub(crate) fn element(tag: &str, attrs: Option<&Value>, children: &Value) -> Result<Value, Error> {
    let attrs = attrs.cloned().unwrap_or_else(keyed::empty_record);
    keyed::vector(vec!["tag".into(), "attrs".into(), "children".into()], vec![keyed::text(tag), attrs, children.clone()])
        .and_then(|e| e.with_renderer(display::renderer("html-renderer", render_html))).described("XML element exceeds array limits")
}

fn attribute(value: &Value) -> Result<String, Error> {
    if let Some(text) = keyed::name(value) { return Ok(text.to_string()); }
    if value.shape().len() > 1 { return Err(Error::domain("XML attributes must be text or numeric units or vectors")); }
    value.elements().map(|v| {
        let n = v.as_number().ok_or_else(|| Error::domain("XML attributes must be text or numbers"))?;
        if let Some(t) = n.integer_text() { return Ok(t); }
        let x = n.to_float()?;
        if !x.is_finite() { return Err(Error::domain("XML numeric attributes must be finite")); }
        Ok(x.to_string())
    }).collect::<Result<Vec<_>, _>>().map(|v| v.join(" "))
}

fn escape(text: &str, output: &mut String) {
    for c in text.chars() {
        match c {
            '&' => output.push_str("&amp;"), '<' => output.push_str("&lt;"), '>' => output.push_str("&gt;"), '"' => output.push_str("&quot;"),
            _ => output.push(c)
        }
    }
}

/// How `write` writes markup. `Xml` escapes all text, and any element without children closes itself. `Html` follows HTML's parsing rules: the text of a `script` or `style` element is `Raw`, the contents of an `svg` or `math` element are `Xml`, and only the void elements in `VOID` close themselves.
#[derive(Clone, Copy, PartialEq)]
enum Markup { Xml, Html, Raw }

const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

fn write(value: &Value, output: &mut String, mode: Markup, span: &Span) -> Result<(), Error> {
    check()?;
    if let Some(text) = keyed::name(value) {
        if mode == Markup::Raw { output.push_str(&text) }
        else { escape(&text, output) }
        return Ok(());
    }
    if value.keys(0).is_none() {
        if value.is_atom() { return Err(Error::domain("XML children must be text or elements")); }
        for child in value.elements() { write(&child, output, mode, span)?; }
        return Ok(());
    }
    let tag = keyed::field(value, "tag").and_then(|v| keyed::name(&v)).filter(|s| valid_name(s)).ok_or_else(|| Error::domain("invalid XML element"))?;
    let attrs = keyed::field(value, "attrs").ok_or_else(|| Error::domain("XML element needs attrs"))?;
    let children = keyed::field(value, "children").ok_or_else(|| Error::domain("XML element needs children"))?;
    output.push('<');
    output.push_str(&tag);
    for (name, value) in keyed::pairs(&attrs).described("XML attributes must be a keyed vector")? {
        if !valid_name(&name) { return Err(Error::domain("invalid XML attribute name")); }
        output.push(' ');
        output.push_str(&name);
        output.push_str("=\"");
        escape(&attribute(&value)?, output);
        output.push('"');
    }
    let html = (mode != Markup::Xml).then(|| tag.to_ascii_lowercase());
    let inner = match html.as_deref() { None | Some("svg" | "math") => Markup::Xml, Some("script" | "style") => Markup::Raw, Some(_) => Markup::Html };
    if children.is_empty() && html.as_deref().is_none_or(|t| VOID.contains(&t)) { output.push_str("/>"); return Ok(()); }
    output.push('>');
    write(&children, output, inner, span)?;
    output.push_str("</");
    output.push_str(&tag);
    output.push('>');
    Ok(())
}

pub(crate) fn serialize(_: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> { text(right, Markup::Xml, span) }

fn text(tree: &Value, mode: Markup, span: &Span) -> Result<Value, Error> {
    let mut output = String::new();
    write(tree, &mut output, mode, span)?;
    Ok(keyed::text(&output))
}

/// `•xml text` reads XML into the elements that `•xml⁻¹` writes, with the same meaning. Comments, processing instructions and text that is only whitespace are dropped.
pub(crate) fn parse(_: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let text = crate::data::text(right)?;
    let document = roxmltree::Document::parse(&text).map_err(|e| Error::domain(format!("XML {e}")))?;
    read(document.root_element(), span)
}

/// The element `node`, with names written with their prefixes, and the namespaces it declares as `xmlns` attributes.
fn read(node: roxmltree::Node<'_, '_>, span: &Span) -> Result<Value, Error> {
    check()?;
    let named = |space: Option<&str>, name: &str| match space.and_then(|uri| node.lookup_prefix(uri)).filter(|p| !p.is_empty()) {
        Some(prefix) => format!("{prefix}:{name}"), None => name.to_string()
    };
    let inherited: Vec<_> = node.parent_element().map_or(vec![], |parent| parent.namespaces().collect());
    let declared = node.namespaces().filter(|ns| ns.uri() != roxmltree::NS_XML_URI && !inherited.contains(ns));
    let attrs = declared.map(|ns| (ns.name().map_or("xmlns".into(), |p| format!("xmlns:{p}")), ns.uri()))
        .chain(node.attributes().map(|a| (named(a.namespace(), a.name()), a.value())));
    let (names, values) = attrs.map(|(name, value)| (name.into(), keyed::text(value))).unzip();
    let children = node.children().filter_map(|child| match child.text() {
        _ if child.is_element() => Some(read(child, span)), Some(text) if child.is_text() && !text.trim().is_empty() => Some(Ok(keyed::text(text))), _ => None
    }).collect::<Result<Vec<_>, _>>()?;
    let children = Value::from_parts(vec![children.len()], children, keyed::text("")).described("XML element exceeds array limits")?;
    let attrs = keyed::vector(names, values).described("invalid XML attributes")?;
    element(&named(node.tag_name().namespace(), node.tag_name().name()), Some(&attrs), &children)
}

pub(crate) fn svg(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let defaults = keyed::vector(vec!["xmlns".into(), "viewBox".into()], vec![keyed::text("http://www.w3.org/2000/svg"), keyed::text("0 0 100 100")]).unwrap();
    let attrs = left.map_or(Ok(defaults.clone()), |a| keyed::merge(&defaults, a)).described("invalid SVG attributes")?;
    element("svg", Some(&attrs), right)?.with_renderer(display::renderer("svg-renderer", render_svg)).described("SVG exceeds array limits")
}

fn render_html(_: Option<&Value>, tree: &Value, span: &Span) -> Result<Value, Error> { markup("text/html", tree, Markup::Html, span) }
fn render_svg(_: Option<&Value>, tree: &Value, span: &Span) -> Result<Value, Error> { markup("image/svg+xml", tree, Markup::Xml, span) }
/// The markup of `tree` as a MIME bundle of type `kind`.
fn markup(kind: &str, tree: &Value, mode: Markup, span: &Span) -> Result<Value, Error> {
    display::mime(kind, text(tree, mode, span)?).described("invalid XML MIME bundle")
}
