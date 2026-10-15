use basedpl::{EvalOptions, InterruptHandle, Session};
use foldhash::{HashMap, HashMapExt};
use serde_json::Value;
use std::{io::{self, BufRead, Write}, sync::{mpsc, Arc, Mutex}};

pub(crate) fn run(output: &mut impl Write) -> io::Result<()> {
    let active = Arc::new(Mutex::new(HashMap::<u64, InterruptHandle>::new()));
    let controls = active.clone();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let request = match line {
                Ok(line) => serde_json::from_str::<Value>(&line).map_err(|e| format!("malformed JSON: {e}")),
                Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(e.to_string()), Err(_) => break
            };
            if let Some(id) = request.as_ref().ok().and_then(|r| r.get("interrupt")).and_then(Value::as_u64) {
                if let Some(handle) = controls.lock().unwrap().get(&id) { handle.interrupt(); }
                continue;
            }
            let interrupt = InterruptHandle::default();
            if let Some(id) = request.as_ref().ok().and_then(|r| r["id"].as_u64()) { controls.lock().unwrap().insert(id, interrupt.clone()); }
            if send.send((request, interrupt)).is_err() { break; }
        }
    });
    let mut session = Session::new();
    for (request, interrupt) in receive {
        let id = request.as_ref().ok().and_then(|r| r["id"].as_u64());
        let result = request.and_then(|r| reply(&mut session, &r, interrupt)).unwrap_or_else(|message| Reply::Json(basedpl::protocol::request_error(&message)));
        if let Some(id) = id { active.lock().unwrap().remove(&id); }
        serde_json::to_writer(&mut *output, &Line { id, result })?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}

/// One request's result: an evaluation's response, which serializes as it's written, or the JSON that another kind of request gives.
enum Reply { Response(basedpl::protocol::Response), Json(Value) }

impl serde::Serialize for Reply {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self { Self::Response(r) => serde::Serialize::serialize(r, s), Self::Json(v) => serde::Serialize::serialize(v, s) }
    }
}

/// A reply line: the request's `id` and its `result`.
struct Line { id: Option<u64>, result: Reply }

impl serde::Serialize for Line {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("result", &self.result)?;
        map.end()
    }
}

/// The result of one request, or why it can't run.
fn reply(session: &mut Session, request: &Value, interrupt: InterruptHandle) -> Result<Reply, String> {
    if request.get("id").is_some_and(|id| !id.is_u64()) { return Err("id must be a nonnegative integer".into()); }
    let options = EvalOptions { interrupt, ..basedpl::protocol::options(request)? };
    if let Some(case) = request.get("case") { return Ok(Reply::Json(basedpl::reference::check(case, options))); }
    if let Some(spec) = request.get("timing") { return basedpl::timing::request(spec, &options.interrupt).map(Reply::Json); }
    if let Some(spec) = request.get("costs") { return basedpl::timing::file_request(spec, &options.interrupt).map(Reply::Json); }
    if let Some(spec) = request.get("reference") { return basedpl::reference::file_request(spec, &options.interrupt).map(Reply::Json); }
    if let Some(spec) = request.get("page") { return basedpl::reference::page_request(spec, &options.interrupt).map(Reply::Json); }
    basedpl::protocol::request(session, request, options).map(|e| Reply::Response(basedpl::protocol::Response(e)))
}
