use crate::{EvalOptions, InterruptHandle, OutputKind, ParseStatus, Session, Source};
use kernmini::{
    CompleteRequest, ExecuteOutcome, ExecuteRequest, ExecutionContext, InspectRequest, KernelInfo, Language, LanguageError, LanguageEvent, LanguageSession,
    ThreadWorker,
};
use serde_json::{json, Value};
use std::future::Future;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone)]
struct AplSession { worker: ThreadWorker<Session>, count: Arc<AtomicU64> }

fn language_error(error: crate::Error) -> LanguageError {
    LanguageError {
        ename: if error.kind == crate::ErrorKind::Interrupt { "KeyboardInterrupt".into() } else { error.kind.to_string() },
        evalue: error.message.clone(),
        traceback: vec![error.to_string()],
    }
}

#[async_trait::async_trait]
impl LanguageSession for AplSession {
    fn kernel_info(&self) -> anyhow::Result<KernelInfo> {
        Ok(KernelInfo {
            implementation: "basedpl".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            banner: "bAsedPL — an APL-derived array language".into(),
            language_info: json!({"name": "apl", "version": env!("CARGO_PKG_VERSION"), "mimetype": "text/apl", "file_extension": ".apl", "codemirror_mode": "apl"}),
        })
    }

    fn execution_count(&self) -> u64 { self.count.load(Ordering::Acquire) }

    async fn execute(&self, request: ExecuteRequest, context: ExecutionContext) -> anyhow::Result<ExecuteOutcome> {
        let count = next_count(&self.count, &request);
        let interrupt = InterruptHandle::default();
        let cancel = interrupt.clone();
        context.set_interrupt_handler(Arc::new(move || {
            cancel.interrupt();
            Ok(())
        }))?;
        self.worker
            .call(move |session| {
                let cancel = interrupt.clone();
                let output = Arc::new(move |output: &crate::Output| {
                    let event = match output.kind {
                        OutputKind::Explicit => LanguageEvent::Stream { name: "stdout".into(), text: format!("{}\n", output.text()) },
                        OutputKind::Display => LanguageEvent::Message {
                            msg_type: "execute_result".into(),
                            content: json!({"execution_count": count, "data": output.data, "metadata": {}}),
                            metadata: json!({}),
                            identity: None,
                            buffers: vec![],
                        },
                    };
                    if context.emit_blocking(event).is_err() { cancel.interrupt(); }
                });
                let result = session.eval_with(
                    &request.code,
                    EvalOptions { interrupt: interrupt.clone(), echo: !request.silent, output: Some(output), ..EvalOptions::default() },
                );
                let mut error = result.error.map(language_error);
                let mut expressions = serde_json::Map::new();
                if error.is_none() {
                    if let Some(items) = request.user_expressions.as_object() {
                        for (name, code) in items {
                            let Some(code) = code.as_str() else { continue; };
                            let result = session.eval_with(
                                code,
                                EvalOptions { interrupt: interrupt.clone(), echo: false, output: Some(Arc::new(|_| {})), ..EvalOptions::default() },
                            );
                            let value = if let Some(e) = result.error {
                                let cancelled = e.kind == crate::ErrorKind::Interrupt;
                                let e = language_error(e);
                                let value = json!({"status": "error", "ename": e.ename, "evalue": e.evalue, "traceback": e.traceback});
                                if cancelled {
                                    error = Some(e);
                                    break;
                                }
                                value
                            } else {
                                let text = result.value.map(|a| session.display.array(&a, false)).or_else(|| result.function.map(|f| f.apl()));
                                json!({"status": "ok", "data": text.map_or(json!({}), |text| json!({"text/plain": text})), "metadata": {}})
                            };
                            expressions.insert(name.clone(), value);
                        }
                    }
                }
                ExecuteOutcome {
                    execution_count: count,
                    result: None,
                    result_metadata: json!({}),
                    error,
                    user_expressions: Value::Object(expressions),
                    payload: json!([]),
                }
            })
            .await
    }

    async fn complete(&self, request: CompleteRequest) -> anyhow::Result<Value> {
        self.worker.call(move |session| {
            let end = request.code.char_indices().nth(request.cursor_pos as usize).map_or(request.code.len(), |(i, _)| i);
            let before = &request.code[..end];
            let (start, mut matches) = if let Some((start, query)) = crate::editor::entry(before, end) {
                (start, crate::editor::matches(query).into_iter().map(|(glyph, _)| glyph.to_owned()).collect::<Vec<_>>())
            } else if crate::editor::in_code(before) {
                let start = before.char_indices().rev().find(|(_, c)| !crate::inspection::word_char(*c)).map_or(0, |(i, c)| i + c.len_utf8());
                (start, session.complete(&before[start..]))
            } else { (end, vec![]) };
            matches.sort();
            json!({"status": "ok", "matches": matches, "cursor_start": request.code[..start].chars().count(), "cursor_end": before.chars().count(), "metadata": {}})
        }).await
    }

    async fn inspect(&self, request: InspectRequest) -> anyhow::Result<Value> {
        self.worker
            .call(move |session| {
                let info = crate::inspection::at_cursor(&request.code, request.cursor_pos as usize).and_then(|name| session.inspect(name));
                let data = info.as_ref().map(|i| json!({"text/plain":i.text(request.detail_level>0), "text/markdown":i.markdown(request.detail_level>0)}));
                json!({"status":"ok", "found":info.is_some(), "data":data.unwrap_or(json!({})), "metadata":{}})
            })
            .await
    }

    async fn is_complete(&self, code: String) -> anyhow::Result<Value> {
        if code.trim_start().starts_with(']') { return Ok(json!({"status": "complete"})); }
        Ok(match crate::parse(Source::new("<cell>", code)) {
            ParseStatus::Complete(_) => json!({"status": "complete"}),
            ParseStatus::Incomplete(_) => json!({"status": "incomplete", "indent": "    "}),
            ParseStatus::Invalid(_) => json!({"status": "invalid"}),
        })
    }

    async fn shutdown(&self) -> anyhow::Result<()> { self.worker.shutdown().await }
}

/// A kernmini language with one session and no subshells.
struct Solo<S>(S);

#[async_trait::async_trait]
impl<S: LanguageSession> Language for Solo<S> {
    type Session = S;
    fn parent(&self) -> S { self.0.clone() }
    async fn create_child(&self) -> anyhow::Result<S> { anyhow::bail!("subshells are not supported") }
}

/// Serve the session that `start` builds on the kernel connection in `file`.
pub(crate) fn serve<S: LanguageSession>(file: &str, start: impl Future<Output = anyhow::Result<S>>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread().enable_all().build()?.block_on(async { kernmini::run_kernel(file, Solo(start.await?)).await })
}

/// The execution count for `request`, counting it when it is stored in history.
pub(crate) fn next_count(count: &AtomicU64, request: &ExecuteRequest) -> u64 {
    if request.store_history && !request.silent { count.fetch_add(1, Ordering::AcqRel) + 1 } else { count.load(Ordering::Acquire) }
}

pub(crate) fn run(file: &str) -> anyhow::Result<()> {
    serve(file, async {
        let worker = ThreadWorker::start(std::thread::Builder::new().name("basedpl".into()), || Ok(Session::interactive())).await?;
        Ok(AplSession { worker, count: Arc::new(AtomicU64::new(0)) })
    })
}
