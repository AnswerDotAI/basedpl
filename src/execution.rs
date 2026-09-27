use crate::{Error, ErrorKind, Span};
use std::{
    cell::Cell,
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
    /// Called at most every 10 ms while the evaluation checks for interruption. Returning true interrupts the evaluation.
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

/// Calls to `Execution::check` between clock reads. A loop whose steps each take a millisecond still stops within about 64 ms.
const CHECKS_PER_CLOCK: u32 = 64;
/// The shortest time between two calls of a poll.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Default)]
pub(crate) struct Execution {
    interrupt: InterruptHandle,
    timeout: Option<(Instant, Duration)>,
    pub echo: bool,
    output: Option<OutputSink>,
    /// The poll, and when it last ran.
    poll: Option<(Poll, Cell<Instant>)>,
    /// Checks left before the next clock read. It starts at zero, so an interrupt set before evaluation stops it at the first check.
    countdown: Cell<u32>,
}
impl Execution {
    pub(crate) fn begin(&mut self, options: EvalOptions) {
        self.interrupt = options.interrupt;
        self.timeout = options.timeout.map(|d| (Instant::now(), d));
        self.echo = options.echo;
        self.output = options.output;
        self.poll = options.poll.map(|poll| (poll, Cell::new(Instant::now())));
        self.countdown.set(0);
    }
    pub(crate) fn output(&self, captured: &mut Vec<Output>, kind: OutputKind, text: String) {
        self.emit(captured, Output { kind, data: [("text/plain".into(), text.into())].into_iter().collect() });
    }
    pub(crate) fn emit(&self, captured: &mut Vec<Output>, output: Output) {
        if let Some(sink) = &self.output { sink(&output); } else { captured.push(output); }
    }
    /// Stops the evaluation when it's interrupted or past its deadline. Most calls only count down. Every `CHECKS_PER_CLOCK`th call reads
    /// the clock. It calls the poll when `POLL_INTERVAL` has passed since the poll last ran.
    pub(crate) fn check(&self, span: &Span) -> Result<(), Error> {
        if let Some(n) = self.countdown.get().checked_sub(1) {
            self.countdown.set(n);
            return Ok(());
        }
        self.countdown.set(CHECKS_PER_CLOCK - 1);
        let now = Instant::now();
        if let Some((poll, last)) = &self.poll {
            if now - last.get() >= POLL_INTERVAL {
                last.set(now);
                if poll() { self.interrupt.interrupt(); }
            }
        }
        if self.interrupt.0.load(Ordering::Relaxed) { return Err(span.error(ErrorKind::Interrupt, "evaluation interrupted")); }
        if self.timeout.is_some_and(|(start, limit)| now - start >= limit) { return Err(span.error(ErrorKind::Timeout, "evaluation deadline exceeded")); }
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
