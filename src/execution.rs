use crate::{Error, ErrorKind, Span};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub(crate) fn thread() -> std::thread::Builder { std::thread::Builder::new().name("basedpl".into()).stack_size(256 * 1024 * 1024) }

/// Run a Rust interpreter workload on the same 256 MiB stack as the CLI and Python workers.
pub fn with_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| thread().spawn_scoped(scope, f).expect("start basedpl execution thread").join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
}

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
}
#[derive(Clone, Copy)]
pub enum OutputKind { Explicit, Display }
pub type OutputSink = Arc<dyn Fn(OutputKind, &str) + Send + Sync>;

impl Default for EvalOptions { fn default() -> Self { Self { interrupt: InterruptHandle::default(), timeout: None, echo: true, output: None } } }

#[derive(Default)]
pub(crate) struct Execution {
    interrupt: InterruptHandle,
    timeout: Option<(Instant, Duration)>,
    pub echo: bool,
    output: Option<OutputSink>,
}
impl Execution {
    pub(crate) fn begin(&mut self, options: EvalOptions) {
        self.interrupt = options.interrupt;
        self.timeout = options.timeout.map(|d| (Instant::now(), d));
        self.echo = options.echo;
        self.output = options.output;
    }
    pub(crate) fn output(&self, captured: &mut Vec<String>, kind: OutputKind, text: String) {
        if let Some(sink) = &self.output { sink(kind, &text); } else { captured.push(text); }
    }
    pub(crate) fn check(&self, span: &Span) -> Result<(), Error> {
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
