//! What the native build takes from its operating system and crates: the clock, files, the environment, Unicode normalization,
//! `•fetch`'s transport and `•r`'s regular expressions. `host_web.rs` gives the browser's versions of the same functions. Each passes
//! straight through to its host, and keeps the host's own behaviour and syntax.
use crate::{
    array::generated_len,
    data::{Request, Response},
    execution::Context,
    regex::Match,
    Error, ErrorAt,
};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) use std::time::Instant;

pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> { std::fs::read(path) }
/// The name that identifies the file at `path`, for `•load`'s cycle check.
pub(crate) fn canonical(path: &Path) -> PathBuf { std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned()) }
/// The environment variables, as names and values.
pub(crate) fn environment() -> Vec<(String, String)> {
    std::env::vars_os().map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned())).collect()
}
/// `text` in the Unicode normalization form `form`: `"NFC"`, `"NFD"`, `"NFKC"` or `"NFKD"`.
pub(crate) fn normalize(form: &str, text: &str) -> String {
    use icu_normalizer::{ComposingNormalizerBorrowed as Composing, DecomposingNormalizerBorrowed as Decomposing};
    match form {
        "NFC" => Composing::new_nfc().normalize(text).into_owned(),
        "NFD" => Decomposing::new_nfd().normalize(text).into_owned(),
        "NFKC" => Composing::new_nfkc().normalize(text).into_owned(),
        _ => Decomposing::new_nfkd().normalize(text).into_owned(),
    }
}
/// The response to `request`, from the system's `curl`, run with an argument list. An interrupt stops it.
pub(crate) fn fetch(request: Request, cx: &Context<'_>) -> Result<Response, Error> {
    let url = &request.url;
    let mut args: Vec<String> = ["--silent", "--show-error", "--location", "--include", "--write-out", "\\n%{size_header}"].map(String::from).into();
    if let Some(method) = &request.method { args.extend(["--request".into(), method.clone()]); }
    for (name, value) in &request.headers { args.extend(["--header".into(), format!("{name}: {value}")]); }
    if request.body.is_some() { args.extend(["--data-binary".into(), "@-".into()]); }
    args.extend(["--url".into(), url.clone()]);
    let out = curl(&args, request.body, url, cx)?;
    // The headers of every response come first, then the body, then the headers' total size.
    let unreadable = || cx.io_error(url, "curl gave a response BPL can't read");
    let end = out.iter().rposition(|&b| b == b'\n').ok_or_else(unreadable)?;
    let size = std::str::from_utf8(&out[end + 1..]).ok().and_then(|s| s.parse::<usize>().ok()).filter(|&n| n <= end).ok_or_else(unreadable)?;
    let head = String::from_utf8_lossy(&out[..size]);
    let mut lines = head.split("\r\n\r\n").filter(|block| !block.is_empty()).last().unwrap_or("").lines();
    let status = lines.next().and_then(|line| line.split_whitespace().nth(1)?.parse::<i64>().ok()).ok_or_else(unreadable)?;
    let mut headers: Vec<(String, String)> = Vec::new();
    for (name, value) in lines.filter_map(|line| line.split_once(':')) {
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
        match headers.iter_mut().find(|(n, _)| *n == name) {
            Some((_, values)) => *values = format!("{values}, {value}"),
            None => headers.push((name, value.into())),
        }
    }
    Ok(Response { status, headers, body: out[size..end].to_vec() })
}

/// What the system's `curl` writes when run with `args`, given `body` as its input. An interrupt stops it.
fn curl(args: &[String], body: Option<Vec<u8>>, url: &str, cx: &Context<'_>) -> Result<Vec<u8>, Error> {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
    };
    let mut child = Command::new("curl")
        .args(args)
        .stdin(if body.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| cx.io_error("•fetch needs curl", e))?;
    if let (Some(body), Some(mut input)) = (body, child.stdin.take()) { std::thread::spawn(move || input.write_all(&body)); }
    let mut output = child.stdout.take().expect("stdout is piped");
    let reader = std::thread::spawn(move || { let mut out = Vec::new(); output.read_to_end(&mut out).map(|_| out) });
    while !reader.is_finished() {
        if let Err(e) = cx.pause(Duration::MAX) {
            let _ = child.kill();
            return Err(e);
        }
    }
    let out = reader.join().expect("the reader doesn't panic").map_err(|e| cx.io_error(url, e))?;
    let status = child.wait().map_err(|e| cx.io_error(url, e))?;
    if status.success() { return Ok(out); }
    let mut message = String::new();
    child.stderr.take().expect("stderr is piped").read_to_string(&mut message).map_err(|e| cx.io_error(url, e))?;
    Err(cx.io_error(url, message.trim().trim_start_matches("curl: ")))
}

/// A compiled `•r` pattern, in the regex crate's syntax.
#[derive(Debug)]
pub(crate) struct Regex(regex::Regex);
impl Regex {
    pub(crate) fn new(pattern: &str) -> Result<Self, String> { regex::Regex::new(pattern).map(Self).map_err(|e| e.to_string()) }
    pub(crate) fn as_str(&self) -> &str { self.0.as_str() }
    /// The number of groups, not counting the whole match.
    pub(crate) fn groups(&self) -> usize { self.0.captures_len() - 1 }
    /// Calls `f` with each match in `text`, and stops at its first error.
    pub(crate) fn each<E>(&self, text: &str, mut f: impl FnMut(Match) -> Result<(), E>) -> Result<(), E> {
        let (mut previous, mut position) = (0, 0);
        for found in self.0.captures_iter(text) {
            let whole = found.get(0).expect("a match includes its whole match");
            position += text[previous..whole.start()].chars().count();
            previous = whole.start();
            let groups = found.iter().skip(1).map(|g| g.map_or("", |g| g.as_str()).to_owned()).collect();
            f(Match { text: whole.as_str().to_owned(), position, groups })?;
        }
        Ok(())
    }
    /// `text` with each match replaced by `template`, which the regex crate expands: `$1`, `${name}` and `$$`.
    pub(crate) fn replace(&self, template: &str, text: &str, cx: &Context<'_>) -> Result<String, Error> {
        let (mut result, mut end) = (String::new(), 0);
        for found in self.0.captures_iter(text) {
            cx.check()?;
            let whole = found.get(0).expect("a match includes its whole match");
            result.push_str(&text[end..whole.start()]);
            found.expand(template, &mut result);
            generated_len(&[result.len() / 4]).error_at(cx, "regex result too large")?;
            end = whole.end();
        }
        result.push_str(&text[end..]);
        Ok(result)
    }
}
