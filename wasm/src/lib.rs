//! The browser's interface to BPL. A `Session` runs code and passes each output to a JavaScript function as the program produces it.
//! `nbs/playground/worker.js` loads the module and runs it in a Web Worker.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use basedpl::{protocol, EvalOptions, MimeData, Output, OutputSink};
use js_sys::{Function, Object, Reflect, Uint8Array};
use std::{cell::RefCell, sync::Arc};
use wasm_bindgen::prelude::*;

thread_local! {
    /// The function that receives each output of the running code. It lives here, not in the sink, because an `OutputSink` must be
    /// `Send + Sync` and a JavaScript function isn't.
    static OUTPUT: RefCell<Option<Function>> = const { RefCell::new(None) };
    /// The function that receives a panic's message, just before the module traps.
    static PANIC: RefCell<Option<Function>> = const { RefCell::new(None) };
}

/// Calls `f`, if there is one, with `value`.
fn send(f: &RefCell<Option<Function>>, value: &JsValue) { if let Some(f) = &*f.borrow() { _ = f.call1(&JsValue::NULL, value); } }

/// The output `o` as `{kind, data}`. `data` maps each MIME type to text, or to a `Uint8Array` for a binary type.
fn object(o: &Output) -> JsValue {
    let data = Object::new();
    for (kind, value) in &o.data {
        let value: JsValue = match value { MimeData::Text(text) => text.into(), MimeData::Bytes(bytes) => Uint8Array::from(&bytes[..]).into() };
        _ = Reflect::set(&data, &kind.into(), &value);
    }
    let output = Object::new();
    _ = Reflect::set(&output, &"kind".into(), &o.kind.name().into());
    _ = Reflect::set(&output, &"data".into(), &data);
    output.into()
}

/// Sets the URL that relative paths resolve against, and the function that receives a panic's message.
#[wasm_bindgen]
pub fn configure(base: String, panic: Function) {
    basedpl::configure_browser(base);
    PANIC.set(Some(panic));
    std::panic::set_hook(Box::new(|info| PANIC.with(|f| send(f, &info.to_string().into()))));
}

/// Every glyph's row, as JSON, for the language bar.
#[wasm_bindgen]
pub fn symbols() -> String { basedpl::symbols::rows().to_string() }

#[wasm_bindgen]
#[derive(Default)]
pub struct Session(basedpl::Session);

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self::default() }
    /// Runs `code`, passing each output to `output` as the program produces it, including any result it displays. Returns the error as
    /// an object, or `null`.
    pub fn run(&mut self, code: &str, output: Function) -> JsValue {
        OUTPUT.set(Some(output));
        let sink: OutputSink = Arc::new(|o: &Output| OUTPUT.with(|f| send(f, &object(o))));
        let error = self.0.eval_with(code, EvalOptions { output: Some(sink), ..EvalOptions::default() }).error;
        OUTPUT.set(None);
        error.map_or(JsValue::NULL, |e| js_sys::JSON::parse(&protocol::error(&e).to_string()).expect("serde_json writes valid JSON"))
    }
}
