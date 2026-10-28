use basedpl::{EvalOptions, Input, InterruptHandle, OutputKind, ParseStatus, Session, Source, Turn};
use kernmini::{
    CompleteRequest, ExecuteOutcome, ExecuteRequest, ExecutionContext, InspectRequest, KernelInfo, LanguageError, LanguageEvent, LanguageSession, ThreadWorker,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct BplSession { worker: ThreadWorker<Turn> }

/// A request's cursor position, as an index into its code's characters.
fn cursor(pos: u64) -> usize { usize::try_from(pos).unwrap_or(usize::MAX) }
fn language_error(error: basedpl::Error) -> LanguageError {
    LanguageError { ename: if matches!(error.kind, basedpl::ErrorKind::Interrupt) { "KeyboardInterrupt".into() } else { error.kind.to_string() },
        evalue: error.message.clone(), traceback: vec![error.to_string()] }
}

/// Reader-facing expression results, shared by Jupyter user expressions and the gateway's eval operation.
fn expression(session: &Session, code: &str, interrupt: &InterruptHandle) -> Result<Value, basedpl::Error> {
    let result = session.eval_with(code,
        EvalOptions { interrupt: interrupt.clone(), echo: false, output: Some(Arc::new(|_| {})), ..EvalOptions::default() });
    if let Some(error) = result.error { return Err(error); }
    let (kind, data) = if let Some(value) = result.value {
        (Some("array"), json!(session.render(&value, interrupt.clone())?))
    } else if let Some(f) = result.function { (Some("function"), json!({"text/plain": f.bpl()})) }
    else if let Some(op) = result.operator { (Some("operator"), json!({"text/plain": op.to_string()})) }
    else { (None, json!({})) };
    Ok(json!({"status": "ok", "data": data, "metadata": kind.map_or(json!({}), |kind| json!({"type": kind}))}))
}

fn expressions<'a>(session: &Session, items: impl IntoIterator<Item = (&'a str, &'a str)>, interrupt: &InterruptHandle) -> (Value, Option<LanguageError>) {
    let mut values = serde_json::Map::new();
    for (name, code) in items {
        let value = match expression(session, code, interrupt) {
            Ok(value) => value,
            Err(error) => {
                let cancelled = matches!(error.kind, basedpl::ErrorKind::Interrupt);
                let error = language_error(error);
                if cancelled { return (Value::Object(values), Some(error)); }
                json!({"status": "error", "ename": error.ename, "evalue": error.evalue, "traceback": error.traceback})
            }
        };
        values.insert(name.into(), value);
    }
    (Value::Object(values), None)
}

/// Standard input in a notebook. Each line is an `input_request` to the frontend, which shows the prompt beside its input box. The input never ends, so `rest` is an error.
struct NotebookInput(ExecutionContext);
impl Input for NotebookInput {
    fn line(&self, prompt: &str) -> std::io::Result<Option<String>> {
        self.0.input(prompt, false).map(Some).map_err(|e| {
            let kind = if e.kind() == kernmini::ErrorKind::Interrupted { std::io::ErrorKind::Interrupted } else { std::io::ErrorKind::Other };
            std::io::Error::new(kind, e)
        })
    }
    fn rest(&self) -> std::io::Result<Vec<u8>> { Err(std::io::Error::other("a notebook's input has no end")) }
}

#[async_trait::async_trait]
impl LanguageSession for BplSession {
    fn supported_features(&self) -> Vec<String> { vec!["eval".into()] }

    fn kernel_info(&self) -> kernmini::Result<KernelInfo> {
        Ok(KernelInfo { implementation: "basedpl".into(), implementation_version: env!("CARGO_PKG_VERSION").into(),
            banner: "BasedPL — an APL-derived array language".into(),
            language_info: json!({"name": "bpl", "version": env!("CARGO_PKG_VERSION"), "mimetype": "text/apl", "file_extension": ".bpl", "codemirror_mode": "apl"})
        })
    }

    async fn execute(&self, request: ExecuteRequest, context: ExecutionContext) -> kernmini::Result<ExecuteOutcome> {
        let count = context.execution_count();
        let eval = context.parent()["metadata"]["eval"].as_array().cloned();
        let interrupt = InterruptHandle::default();
        let cancel = interrupt.clone();
        context.set_interrupt_handler(Arc::new(move || { cancel.interrupt(); Ok(()) }))?;
        self.worker.call(move |session| {
            if let Some(items) = eval {
                let (values, error) = expressions(session, items.iter().filter_map(|v| v.as_str().map(|s| (s, s))), &interrupt);
                return Ok(ExecuteOutcome { result: Some(json!({"application/json": values})), error, ..Default::default() });
            }
            let cancel = interrupt.clone();
            let failure = Arc::new(Mutex::new(None));
            let output_failure = failure.clone();
            let input = request.allow_stdin.then(|| Arc::new(NotebookInput(context.clone())) as Arc<dyn Input>);
            let output = Arc::new(move |output: &basedpl::Output| {
                let event = match output.kind {
                    OutputKind::Explicit | OutputKind::Text => LanguageEvent::Stream { name: "stdout".into(), text: output.written() },
                    OutputKind::Display => LanguageEvent::Message { msg_type: "execute_result".into(),
                        content: json!({"execution_count": count, "data": output.data, "metadata": {}}), metadata: json!({}), identity: None, buffers: vec![] }
                };
                if let Err(error) = context.emit_blocking(event) {
                    *output_failure.lock().unwrap() = Some(error);
                    cancel.interrupt();
                }
            });
            let result = session.eval_with(
                &request.code, EvalOptions { interrupt: interrupt.clone(), echo: !request.silent, output: Some(output), input, ..EvalOptions::default() });
            let mut error = result.error.map(language_error);
            let user_expressions = if error.is_none() {
                let (values, failure) = expressions(session,
                    request.user_expressions.as_object().into_iter().flatten().filter_map(|(name, code)| Some((name.as_str(), code.as_str()?))), &interrupt);
                error = failure;
                values
            } else { json!({}) };
            if let Some(error) = failure.lock().unwrap().take() { return Err(error); }
            Ok(ExecuteOutcome { error, user_expressions, ..Default::default() })
        }).await?
    }

    async fn complete(&self, request: CompleteRequest) -> kernmini::Result<Value> {
        self.worker.call(move |session| {
            let end = request.code.char_indices().nth(cursor(request.cursor_pos)).map_or(request.code.len(), |(i, _)| i);
            let before = &request.code[..end];
            let (start, mut matches) = if let Some((start, query)) = basedpl::symbols::entry(before, end, basedpl::settings::hanzi_aliases()) {
                (start, basedpl::symbols::matches(query).into_iter().map(|(glyph, _)| glyph.to_owned()).collect::<Vec<_>>())
            } else if basedpl::in_code(before, basedpl::settings::hanzi_aliases()) {
                let start = before.char_indices().rev().find(|(_, c)| !basedpl::inspection::word_char(*c, basedpl::settings::hanzi_aliases())).map_or(0, |(i, c)| i + c.len_utf8());
                (start, session.complete(&before[start..]))
            } else { (end, vec![]) };
            matches.sort();
            json!({"status": "ok", "matches": matches, "cursor_start": request.code[..start].chars().count(), "cursor_end": before.chars().count(), "metadata": {}})
        }).await
    }

    async fn inspect(&self, request: InspectRequest) -> kernmini::Result<Value> {
        self.worker.call(move |session| {
            let info = basedpl::inspection::at_cursor(&request.code, cursor(request.cursor_pos), basedpl::settings::hanzi_aliases()).and_then(|name| session.inspect(name));
            let data = info.as_ref().map(|i| json!({"text/plain":i.text(request.detail_level>0), "text/markdown":i.markdown(request.detail_level>0)}));
            json!({"status":"ok", "found":info.is_some(), "data":data.unwrap_or(json!({})), "metadata":{}})
        }).await
    }

    async fn debug(&self, request: Value) -> kernmini::Result<Value> {
        if request["command"] != "inspectVariables" { return Ok(kernmini::reply_defaults::debug()); }
        self.worker.call(move |session| {
            let variables: Vec<_> = session.names().into_iter().filter_map(|name| {
                let (kind, value) = session.preview(&name)?;
                let mut record = json!({"name": name, "value": value, "type": kind, "evaluateName": name, "variablesReference": 0});
                if matches!(kind, "function" | "operator") { record["presentationHint"] = json!({"kind": "method"}); }
                Some(record)
            }).collect();
            json!({"response": {"type": "response", "request_seq": request["seq"], "command": "inspectVariables", "success": true,
                "body": {"variables": variables}}, "events": []})
        }).await
    }

    async fn is_complete(&self, code: String) -> kernmini::Result<Value> {
        if code.trim_start().starts_with(']') { return Ok(json!({"status": "complete"})); }
        self.worker.call(move |session| match session.parse(Source::new("<cell>", code)) {
            ParseStatus::Complete(_) => json!({"status": "complete"}), ParseStatus::Incomplete(_) => json!({"status": "incomplete", "indent": "    "}),
            ParseStatus::Invalid(_) => json!({"status": "invalid"})
        }).await
    }

    async fn shutdown(&self) -> kernmini::Result<()> { self.worker.shutdown().await }
}

pub(crate) fn run(file: &str) -> kernmini::Result<()> {
    kernmini::run_kernel_blocking(file, async {
        Ok(BplSession { worker: ThreadWorker::start(std::thread::Builder::new().name("basedpl".into()), || Ok(Session::interactive())).await? })
    })
}
