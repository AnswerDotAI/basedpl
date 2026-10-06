use basedpl::{EvalOptions, InterruptHandle, Session};
use foldhash::{HashMap, HashMapExt};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    sync::{mpsc, Arc, Mutex},
};

pub(crate) fn run(output: &mut impl Write) -> io::Result<()> {
    let active = Arc::new(Mutex::new(HashMap::<u64, InterruptHandle>::new()));
    let controls = active.clone();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let request = match line {
                Ok(line) => serde_json::from_str::<Value>(&line).map_err(|e| format!("malformed JSON: {e}")),
                Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(e.to_string()),
                Err(_) => break,
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
        let result = request.and_then(|r| reply(&mut session, &r, interrupt)).unwrap_or_else(|message| basedpl::protocol::request_error(&message));
        if let Some(id) = id { active.lock().unwrap().remove(&id); }
        serde_json::to_writer(&mut *output, &json!({"id": id, "result": result}))?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}

/// The result of one request, or why it can't run.
fn reply(session: &mut Session, request: &Value, interrupt: InterruptHandle) -> Result<Value, String> {
    if request.get("id").is_some_and(|id| !id.is_u64()) { return Err("id must be a nonnegative integer".into()); }
    let options = EvalOptions { interrupt, ..basedpl::protocol::options(request)? };
    if let Some(case) = request.get("case") { return Ok(basedpl::reference::check(case, options)); }
    basedpl::protocol::request(session, request, options).map(basedpl::protocol::response)
}
