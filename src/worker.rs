use crate::{EvalOptions, InterruptHandle, Session};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

pub(crate) fn run(output: &mut impl Write) -> io::Result<()> {
    let active = Arc::new(Mutex::new(HashMap::<u64, InterruptHandle>::new()));
    let controls = active.clone();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let request = line.and_then(|s| serde_json::from_str::<Value>(&s).map_err(io::Error::other));
            if let Ok(request) = &request {
                if let Some(id) = request.get("interrupt").and_then(Value::as_u64) {
                    if let Some(handle) = controls.lock().unwrap().get(&id) { handle.interrupt(); }
                    continue;
                }
            }
            let interrupt = InterruptHandle::default();
            if let Ok(request) = &request { if let Some(id) = request["id"].as_u64() { controls.lock().unwrap().insert(id, interrupt.clone()); } }
            if send.send((request, interrupt)).is_err() { break; }
        }
        for handle in controls.lock().unwrap().values() { handle.interrupt(); }
    });
    let mut session = Session::new();
    for (request, interrupt) in receive {
        let request = request?;
        let id = request["id"].as_u64().ok_or_else(|| io::Error::other("worker request needs an integer id"))?;
        let timeout = match request.get("timeout_ms") {
            Some(value) => Some(Duration::from_millis(value.as_u64().ok_or_else(|| io::Error::other("timeout_ms must be a nonnegative integer"))?)),
            None => None,
        };
        let options = EvalOptions { interrupt, timeout };
        let result = if let Some(case) = request.get("case") { crate::reference::check(case, options) } else if let Some(code) = request["code"].as_str() { crate::protocol::response(session.eval_with(code, options)) } else { json!({"status":"invalid", "message":"expected code or case"}) };
        active.lock().unwrap().remove(&id);
        serde_json::to_writer(&mut *output, &json!({"id":id, "result":result}))?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
