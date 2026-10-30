//! The browser's interface to BPL. A `Session` runs code and passes each output to a JavaScript function as the program produces it, gives the value of code as a JavaScript value, and binds JavaScript values and functions as BPL names. `nbs/playground/worker.js` loads the module and runs it in a Web Worker.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use basedpl::{EvalOptions, MimeData, Output, OutputSink, js, protocol};
use js_sys::{Function, Object, Reflect, Uint8Array};
use std::{cell::RefCell, sync::Arc};
use wasm_bindgen::prelude::*;

thread_local! {
    /// The function that receives each output of the running code. It lives here, not in the sink, because an `OutputSink` must be `Send + Sync` and a JavaScript function isn't.
    static OUTPUT: RefCell<Option<Function>> = const { RefCell::new(None) };
    /// The function that receives a panic's message, just before the module traps.
    static PANIC: RefCell<Option<Function>> = const { RefCell::new(None) };
}

/// The global allocator: rustymath's memory guard, which counts large allocations for the speed tests' footprints, over `talc`. `talc` aligns every block to 16 bytes on wasm32, where std's `dlmalloc` aligns only to 8. Elementwise passes that read a large complex array and write complex items ran two to four times slower when the result started at 8 mod 16 and the argument at 0 mod 16.
#[global_allocator]
static GUARD: rustymath::memory::Guard<talc::wasm::WasmDynamicTalc> = rustymath::memory::Guard::over(talc::wasm::new_wasm_dynamic_allocator());

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

/// Names this machine's CPU for the timing code, which can't read it in a browser, as `rustymath::timing::name_cpu` describes.
#[wasm_bindgen]
pub fn name_cpu(name: &str) { basedpl::timing::name_cpu(name) }

/// The pauses before each retry of a timing that fails, in seconds: `rustymath::timing::RETRY_PAUSES`.
#[wasm_bindgen]
pub fn retry_pauses() -> Vec<f64> { basedpl::timing::RETRY_PAUSES.to_vec() }

/// Keeps the calling thread busy for 0.1 s after a retry's pause: `rustymath::timing::warm`.
#[wasm_bindgen]
pub fn warm() { rustymath::timing::warm() }

/// Whether the speed test fails on a case still over its wasm budget, as `rustymath::timing::checks_budgets` decides from the CPU that `name_cpu` named.
#[wasm_bindgen]
pub fn checks_budgets() -> bool { basedpl::timing::checks_budgets() }

/// Time named BPL source files, supplied as `[{name, text}]`. Results retain full code and single-line log labels.
#[wasm_bindgen]
pub fn costs(files: JsValue, cases: Option<Vec<String>>, prefs: Option<String>) -> Result<JsValue, JsValue> {
    let only: Option<Vec<&str>> = cases.as_ref().map(|c| c.iter().map(String::as_str).collect());
    let fail = |e: String| JsValue::from(js_sys::Error::new(&e));
    let files = serde_json::from_str(&js_sys::JSON::stringify(&files)?.as_string().unwrap()).map_err(|e| fail(e.to_string()))?;
    let files = basedpl::reference::sources(&files).map_err(fail)?;
    let cases = basedpl::timing::costs(&files, only.as_deref(), prefs.as_deref().unwrap_or_default(), &Default::default()).map_err(fail)?;
    Ok(js_sys::JSON::parse(&basedpl::timing::rows(&cases, Some(basedpl::timing::WASM_COLUMN)).to_string())?)
}

#[wasm_bindgen]
/// A turn at the page's one BPL session. Each `new Session()` clears it, as a new session starts.
pub struct Session(basedpl::Turn);

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self(basedpl::Session::new()) }
    /// Runs `code`, passing each output to `output` as the program produces it, including any result it displays. Returns the error as an object, or `null`.
    pub fn run(&mut self, code: &str, output: Function) -> JsValue {
        OUTPUT.set(Some(output));
        let sink: OutputSink = Arc::new(|o: &Output| OUTPUT.with(|f| send(f, &object(o))));
        let error = self.0.eval_with(code, EvalOptions { output: Some(sink), ..EvalOptions::default() }).error;
        OUTPUT.set(None);
        error.as_ref().map_or(JsValue::NULL, error_object)
    }
    /// The value of `code` as a JavaScript value, which `basedpl::js` describes, or `undefined` when the code gives no value. A BPL error throws its error object. The code's outputs are dropped, and `run` shows them.
    pub fn eval(&mut self, code: &str) -> Result<JsValue, JsValue> {
        let result = self.0.eval_with(code, EvalOptions { echo: false, ..EvalOptions::default() });
        if let Some(e) = &result.error { return Err(error_object(e)); }
        let crossing = || JsValue::from(js_sys::Error::new("a function or operator can't cross into JavaScript"));
        if result.function.is_some() || result.operator.is_some() { return Err(crossing()); }
        result.value.map_or(Ok(JsValue::UNDEFINED), |v| js::export(&v).map(js::to_js).map_err(|_| crossing()))
    }
    /// Binds each property of `names` as a BPL name, to the BPL value that `basedpl::js` converts it to. A JavaScript function becomes a BPL function that calls it. Throws when a property's name isn't a BPL name, or its value has no BPL form.
    pub fn bind(&mut self, names: &JsValue) -> Result<(), JsValue> {
        let Ok(js::Js::Object(fields)) = js::from_js(names) else { return Err(js_sys::Error::new("bind needs an object").into()) };
        for (name, value) in fields {
            let fail = |why: &str| JsValue::from(js_sys::Error::new(&format!("can't bind {name}: {why}")));
            let value = js::import(value).map_err(|_| fail("its value has no BPL form"))?.ok_or_else(|| fail("it has no value"))?;
            self.0.set(&name, value).map_err(|_| fail("it isn't a BPL name"))?;
        }
        Ok(())
    }
}

/// A BPL error as a JavaScript object, with the fields of `protocol::error`.
fn error_object(e: &basedpl::Error) -> JsValue { js_sys::JSON::parse(&protocol::error(e).to_string()).expect("serde_json writes valid JSON") }
