use crate::{Error, ErrorKind, Span};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// Cancellation for one evaluation; may be sent to another thread.
#[derive(Clone, Default)]
pub struct InterruptHandle(Arc<AtomicBool>);
impl InterruptHandle { pub fn interrupt(&self) { self.0.store(true, Ordering::Relaxed); } }

pub struct EvalOptions {
    pub interrupt: InterruptHandle,
    pub timeout: Option<Duration>,
    /// Include implicit expression display; explicit output is always retained.
    pub echo: bool,
    /// Stream output instead of collecting it in Evaluation.output.
    pub output: Option<OutputSink>,
    /// Called at each interruption check. Returning true interrupts the evaluation.
    pub poll: Option<Poll>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputKind { Explicit, Display }
impl OutputKind { pub(crate) fn name(self) -> &'static str { if self == Self::Explicit { "explicit" } else { "display" } } }
pub type MimeBundle = std::collections::BTreeMap<String, String>;
#[derive(Clone, Debug, PartialEq)]
pub struct Output { pub kind: OutputKind, pub data: MimeBundle }
impl Output {
    pub fn text(&self) -> &str { self.data.get("text/plain").map_or("", String::as_str) }
    pub fn json(&self) -> serde_json::Value { serde_json::json!({"kind": self.kind.name(), "data": self.data}) }
}
pub type OutputSink = Arc<dyn Fn(&Output) + Send + Sync>;
pub type Poll = Arc<dyn Fn() -> bool + Send + Sync>;

impl Default for EvalOptions { fn default() -> Self { Self { interrupt: InterruptHandle::default(), timeout: None, echo: true, output: None, poll: None } } }

#[derive(Default)]
pub(crate) struct Execution {
    interrupt: InterruptHandle,
    timeout: Option<(Instant, Duration)>,
    pub echo: bool,
    output: Option<OutputSink>,
    poll: Option<Poll>,
}
impl Execution {
    pub(crate) fn begin(&mut self, options: EvalOptions) {
        self.interrupt = options.interrupt;
        self.timeout = options.timeout.map(|d| (Instant::now(), d));
        self.echo = options.echo;
        self.output = options.output;
        self.poll = options.poll;
    }
    pub(crate) fn output(&self, captured: &mut Vec<Output>, kind: OutputKind, text: String) {
        self.emit(captured, Output { kind, data: [("text/plain".into(), text.into())].into_iter().collect() });
    }
    pub(crate) fn emit(&self, captured: &mut Vec<Output>, output: Output) {
        if let Some(sink) = &self.output { sink(&output); } else { captured.push(output); }
    }
    pub(crate) fn check(&self, span: &Span) -> Result<(), Error> {
        if self.poll.as_ref().is_some_and(|poll| poll()) { self.interrupt.interrupt(); }
        if self.interrupt.0.load(Ordering::Relaxed) { return Err(span.error(ErrorKind::Interrupt, "evaluation interrupted")); }
        if self.timeout.is_some_and(|(start, limit)| start.elapsed() >= limit) { return Err(span.error(ErrorKind::Timeout, "evaluation deadline exceeded")); }
        Ok(())
    }
    pub(crate) fn at<'a>(&'a self, span: &'a Span) -> Context<'a> { Context { span, execution: self } }
}

pub(crate) struct Context<'a> { span: &'a Span, execution: &'a Execution }
impl Context<'_> { pub(crate) fn check(&self) -> Result<(), Error> { self.execution.check(self.span) } }
impl Deref for Context<'_> {
    type Target = Span;
    fn deref(&self) -> &Span { self.span }
}
