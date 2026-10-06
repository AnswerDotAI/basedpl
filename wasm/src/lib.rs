//! The browser's interface to BPL. A `Session` answers the JSON requests that `bpl --worker` answers, and passes each output to a
//! JavaScript function as the program produces it. `nbs/playground/worker.js` loads the module and runs it in a Web Worker.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use basedpl::{protocol, EvalOptions, Output, OutputSink};
use js_sys::Function;
use std::{cell::RefCell, sync::Arc};
use wasm_bindgen::prelude::*;

thread_local! {
    /// The function that receives each output of the running request. It lives here, not in the sink, because an `OutputSink` must be
    /// `Send + Sync` and a JavaScript function isn't.
    static OUTPUT: RefCell<Option<Function>> = const { RefCell::new(None) };
    /// The function that receives a panic's message, just before the module traps.
    static PANIC: RefCell<Option<Function>> = const { RefCell::new(None) };
}

/// Calls `f`, if there is one, with `text`.
fn send(f: &RefCell<Option<Function>>, text: &str) { if let Some(f) = &*f.borrow() { _ = f.call1(&JsValue::NULL, &text.into()); } }

/// Sets the URL that relative paths resolve against, and the function that receives a panic's message.
#[wasm_bindgen]
pub fn configure(base: String, panic: Function) {
    basedpl::configure_browser(base);
    PANIC.set(Some(panic));
    std::panic::set_hook(Box::new(|info| PANIC.with(|f| send(f, &info.to_string()))));
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
    /// The JSON reply to a JSON request: `{"error": …}`, with `null` when the request succeeds. Each output goes to `output`, as JSON,
    /// while the program runs. The reply has no value: a page shows the output, and a value that holds functions has no JSON form.
    pub fn request(&mut self, request: &str, output: Function) -> String {
        OUTPUT.set(Some(output));
        let sink: OutputSink = Arc::new(|o: &Output| OUTPUT.with(|f| send(f, &o.json().to_string())));
        let reply = serde_json::from_str(request).map_err(|e| format!("malformed JSON: {e}")).and_then(|request| {
            let options = EvalOptions { output: Some(sink), ..protocol::options(&request)? };
            protocol::request(&mut self.0, &request, options)
        });
        OUTPUT.set(None);
        match reply {
            Ok(result) => serde_json::json!({"error": result.error.as_ref().map(protocol::error)}),
            Err(message) => protocol::request_error(&message),
        }
        .to_string()
    }
}
