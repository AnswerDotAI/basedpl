//! What the browser build takes from the page: the clock, file reads, Unicode normalization, `•fetch`'s transport and `•r`'s regular
//! expressions. `host.rs` gives the native versions.
use crate::{
    data::{Request, Response},
    execution::Context,
    regex::Match,
    Error,
};
use js_sys::Uint8Array;
use std::{
    cell::RefCell,
    io,
    path::{Path, PathBuf},
};
use web_sys::{Url, XmlHttpRequest, XmlHttpRequestResponseType};

pub(crate) use web_time::Instant;

thread_local! {
    /// The URL that relative paths resolve against: the page's.
    static BASE: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Sets the URL that relative paths resolve against.
pub fn configure_browser(base: String) { BASE.set(base); }
/// The absolute URL of `path`.
fn url(path: &Path) -> io::Result<String> {
    let path = path.to_string_lossy();
    BASE.with_borrow(|base| Url::new_with_base(&path, base)).map(|url| url.href()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid URL"))
}
/// Sends a synchronous `XMLHttpRequest`, which a worker allows, and gives the response's status, headers and body. The request
/// blocks the worker, and only ending the worker stops it.
fn send(method: &str, url: &str, headers: &[(String, String)], body: Option<&[u8]>) -> Result<(u16, String, Vec<u8>), &'static str> {
    let request = XmlHttpRequest::new().map_err(|_| "this host has no XMLHttpRequest")?;
    request.open_with_async(method, url, false).map_err(|_| "invalid request")?;
    request.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
    for (name, value) in headers { request.set_request_header(name, value).map_err(|_| "invalid header")?; }
    match body { Some(body) => request.send_with_opt_u8_array(Some(body)), None => request.send() }
    .map_err(|_| "the request failed")?;
    let status = request.status().ok().filter(|&s| s != 0).ok_or("no response")?;
    let body = Uint8Array::new(&request.response().map_err(|_| "no response")?).to_vec();
    Ok((status, request.get_all_response_headers().unwrap_or_default(), body))
}
/// The bytes at `path`.
pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
    match send("GET", &url(path)?, &[], None).map_err(io::Error::other)? {
        (200..=299, _, body) => Ok(body),
        (404, ..) => Err(io::ErrorKind::NotFound.into()),
        (status, ..) => Err(io::Error::other(format!("HTTP status {status}"))),
    }
}
/// The response to `request`. CORS applies, and a relative URL resolves against the page's.
pub(crate) fn fetch(request: Request, cx: &Context<'_>) -> Result<Response, Error> {
    let method = request.method.as_deref().unwrap_or(if request.body.is_some() { "POST" } else { "GET" });
    let url = url(Path::new(&request.url)).map_err(|e| cx.io_error(&request.url, e))?;
    let (status, headers, body) = send(method, &url, &request.headers, request.body.as_deref()).map_err(|e| cx.io_error(&request.url, e))?;
    let headers = headers.lines().filter_map(|line| line.split_once(": ")).map(|(name, value)| (name.into(), value.into())).collect();
    Ok(Response { status: status.into(), headers, body })
}
/// The absolute URL of `path`, for `•load`'s cycle check.
pub(crate) fn canonical(path: &Path) -> PathBuf { url(path).map_or_else(|_| path.to_owned(), PathBuf::from) }
/// A page has no environment variables.
pub(crate) fn environment() -> Vec<(String, String)> { Vec::new() }
/// `text` in the Unicode normalization form `form`, through `String.prototype.normalize`.
pub(crate) fn normalize(form: &str, text: &str) -> String { js_sys::JsString::from(text).normalize(form).into() }

/// A compiled `•r` pattern, in JavaScript's `RegExp` syntax. It holds text, because a BPL value must be `Send + Sync` and a `RegExp`
/// isn't: each call compiles the pattern again.
#[derive(Debug)]
pub(crate) struct Regex {
    pattern: String,
    groups: usize,
}
impl Regex {
    pub(crate) fn new(pattern: &str) -> Result<Self, String> {
        // An empty alternative matches at once, and the match has one item for each group.
        let groups = regexp(&format!("(?:{pattern})|"))?.exec("").map_or(0, |m| m.length() as usize - 1);
        Ok(Self { pattern: pattern.into(), groups })
    }
    pub(crate) fn as_str(&self) -> &str { &self.pattern }
    /// The number of groups, not counting the whole match.
    pub(crate) fn groups(&self) -> usize { self.groups }
    /// Calls `f` with each match in `text`, and stops at its first error.
    pub(crate) fn each<E>(&self, text: &str, mut f: impl FnMut(Match) -> Result<(), E>) -> Result<(), E> {
        use js_sys::{Array, Reflect};
        use wasm_bindgen::JsCast;
        let regexp = regexp(&self.pattern).expect("the pattern compiled when •r made it");
        // JavaScript gives each match's position in UTF-16 units, which become characters.
        let (mut chars, mut units, mut position) = (text.chars(), 0, 0);
        for found in js_sys::JsString::from(text).match_all(&regexp).into_iter().flatten() {
            let found: Array = found.unchecked_into();
            let index = Reflect::get(&found, &"index".into()).ok().and_then(|i| i.as_f64()).unwrap_or_default() as usize;
            while units < index {
                units += chars.next().map_or(1, char::len_utf16);
                position += 1;
            }
            let groups = found.iter().skip(1).map(|g| g.as_string().unwrap_or_default()).collect();
            f(Match { text: found.get(0).as_string().unwrap_or_default(), position, groups })?;
        }
        Ok(())
    }
    /// `text` with each match replaced by `template`, which `String.prototype.replace` expands: `$1`, `$<name>` and `$$`.
    pub(crate) fn replace(&self, template: &str, text: &str, _: &Context<'_>) -> Result<String, Error> {
        let regexp = regexp(&self.pattern).expect("the pattern compiled when •r made it");
        Ok(js_sys::JsString::from(text).replace_by_pattern(&regexp, template).into())
    }
}
/// `new RegExp(source, "gu")`, or the message of the error it throws.
fn regexp(source: &str) -> Result<js_sys::RegExp, String> {
    use js_sys::{Array, Function, Reflect};
    use wasm_bindgen::JsCast;
    let constructor: Function = Reflect::get(&js_sys::global(), &"RegExp".into()).ok().and_then(|c| c.dyn_into().ok()).ok_or("this host has no RegExp")?;
    Reflect::construct(&constructor, &Array::of2(&source.into(), &"gu".into()))
        .map(JsCast::unchecked_into)
        .map_err(|e| e.dyn_into::<js_sys::Error>().map_or_else(|_| "invalid pattern".into(), |e| e.message().into()))
}
