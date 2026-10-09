//! `•canvas`: drawing functions written in JavaScript. `•canvas Y` makes a drawing function from the JavaScript source `Y`. `X draw Y`
//! gives a record of the source, the options `X` and the data `Y`, which displays as a canvas. Displaying it calls the JavaScript
//! function with the canvas's 2D context, the data and the options, which `js.rs` converts to JavaScript values.
use crate::{
    data, display,
    execution::Context,
    js, keyed,
    system::{native, Call, Valence::Ambivalent},
    Error, ErrorAt, Value,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

/// The script that draws a canvas output, a function of the canvas, the JSON and the bytes that `render` writes.
const SCRIPT: &str = include_str!("canvas.js");
/// The browser build's MIME types for a canvas: its JSON, and the bytes of the typed arrays in it.
const JSON_TYPE: &str = "application/x-bpl-canvas+json";
const BYTES_TYPE: &str = "application/x-bpl-canvas-data";

/// The drawing function for the JavaScript `source`, as `•canvas` returns it.
pub(crate) fn function(source: Arc<str>) -> Value { Value::Function(native("•canvas", Call::Canvas(source), Ambivalent)) }

pub(crate) fn factory(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> { Ok(function(data::text(right, span)?.into())) }

/// `X draw Y`: a record of the drawing function's source, the options `X` and the data `Y`, displayed as a canvas. Without options,
/// the JavaScript gets an empty object.
pub(crate) fn draw(source: &str, options: Option<&Value>, data: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let options = options.cloned().unwrap_or_else(keyed::empty_record);
    keyed::vector(vec!["source".into(), "options".into(), "data".into()], vec![keyed::text(source), options, data.clone()])
        .and_then(|canvas| canvas.with_renderer(display::renderer("canvas-renderer", render)))
        .error_at(span, "canvas exceeds array limits")
}

/// The browser build sends the canvas as JSON, with its typed arrays as bytes, for the page to draw with `canvas.js`. The native build
/// sends HTML that holds the same JSON and bytes, and runs `canvas.js` itself. Both send the text `canvas: N frames` when the options
/// give `fps` and the data is a vector of `N` frames, and `canvas` otherwise.
fn render(_: Option<&Value>, canvas: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (json, bytes) = js::serialize(&js::export(canvas).error_at(span, "canvas data must not hold functions")?);
    let json = json.to_string();
    // The default text form would show the whole data.
    let frames = keyed::field(canvas, "options").and_then(|o| keyed::field(&o, "fps")).and(keyed::field(canvas, "data")).filter(|d| d.shape().len() == 1);
    let text = keyed::text(&frames.map_or_else(|| "canvas".into(), |d| format!("canvas: {} frames", d.len())));
    let bundle = if cfg!(web) {
        let bytes = data::byte_vector(bytes).error_at(span, "canvas data exceeds array limits")?;
        keyed::vector(vec!["text/plain".into(), JSON_TYPE.into(), BYTES_TYPE.into()], vec![text, keyed::text(&json), bytes])
    } else {
        // A JSON string is a JavaScript string literal. Escaping `<` keeps `</script>` out of the script.
        let json = serde_json::to_string(&json).expect("a string serializes").replace('<', "\\u003c");
        let bytes = STANDARD.encode(bytes);
        let html = format!("<canvas></canvas><script>({SCRIPT})(document.currentScript.previousElementSibling, {json}, Uint8Array.from(atob('{bytes}'), c => c.charCodeAt(0)))</script>");
        keyed::vector(vec!["text/plain".into(), "text/html".into()], vec![text, keyed::text(&html)])
    };
    bundle.error_at(span, "invalid canvas MIME bundle")
}
