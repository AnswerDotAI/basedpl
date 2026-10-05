use crate::{Error, ErrorKind, Span};
use std::{
    cell::{Cell, RefCell},
    io,
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
    /// Standard input for `⎕` and `•nget "-"`. Without one, reading either is a VALUE error.
    pub input: Option<Arc<dyn Input>>,
    /// Called at most every 10 ms while the evaluation checks for interruption. Returning true interrupts the evaluation.
    pub poll: Option<Poll>,
}
/// `Explicit` output and `Display` results each end a line. `Text` is written as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputKind { Explicit, Display, Text }
impl OutputKind { pub(crate) fn name(self) -> &'static str { match self { Self::Explicit => "explicit", Self::Display => "display", Self::Text => "text" } } }
pub type MimeBundle = std::collections::BTreeMap<String, String>;
#[derive(Clone, Debug, PartialEq)]
pub struct Output { pub kind: OutputKind, pub data: MimeBundle }
impl Output {
    pub(crate) fn plain(kind: OutputKind, text: String) -> Self { Self { kind, data: [("text/plain".into(), text)].into_iter().collect() } }
    pub fn text(&self) -> &str { self.data.get("text/plain").map_or("", String::as_str) }
    /// The text as a terminal shows it, with a line ending after any output but `Text`.
    pub fn written(&self) -> String { if self.kind == OutputKind::Text { self.text().into() } else { format!("{}\n", self.text()) } }
    pub fn json(&self) -> serde_json::Value { serde_json::json!({"kind": self.kind.name(), "data": self.data}) }
}
pub type OutputSink = Arc<dyn Fn(&Output) + Send + Sync>;
pub type Poll = Arc<dyn Fn() -> bool + Send + Sync>;

/// A frontend's standard input. Reading `⎕` takes the next line. `•nget "-"` takes the rest.
pub trait Input: Send + Sync {
    /// The next line, without its line ending, read after showing `prompt`. `None` at the end of the input.
    fn line(&self, prompt: &str) -> io::Result<Option<String>>;
    /// Everything not read yet.
    fn rest(&self) -> io::Result<Vec<u8>>;
}

impl Default for EvalOptions {
    fn default() -> Self { Self { interrupt: InterruptHandle::default(), timeout: None, echo: true, output: None, input: None, poll: None } }
}

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
    /// Output that no sink takes, kept for the evaluation's result.
    captured: RefCell<Vec<Output>>,
    input: Option<Arc<dyn Input>>,
    /// Text written since the last line ending, which a read of `⎕` shows as its prompt.
    pending: RefCell<String>,
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
        self.input = options.input;
        self.poll = options.poll.map(|poll| (poll, Cell::new(Instant::now())));
        self.countdown.set(0);
    }
    pub(crate) fn output(&self, kind: OutputKind, text: String) { self.emit(Output::plain(kind, text)); }
    /// Sends `output`, after any pending text.
    pub(crate) fn emit(&self, output: Output) {
        self.flush();
        self.send(output);
    }
    /// Sends `output` to the sink, or keeps it for the evaluation's result when there is none.
    fn send(&self, output: Output) { if let Some(sink) = &self.output { sink(&output); } else { self.captured.borrow_mut().push(output); } }
    /// Writes `text` with no line ending added. Text after the last line ending waits, so that a read of `⎕` can show it as its prompt.
    pub(crate) fn write(&self, text: &str) {
        let mut pending = self.pending.borrow_mut();
        pending.push_str(text);
        let Some(end) = pending.rfind('\n') else { return };
        let lines = pending.drain(..=end).collect();
        drop(pending);
        self.send(Output::plain(OutputKind::Text, lines));
    }
    /// Sends the text that waits for a line ending.
    pub(crate) fn flush(&self) { let text = self.pending.take(); if !text.is_empty() { self.send(Output::plain(OutputKind::Text, text)); } }
    /// The output kept since the last call.
    pub(crate) fn take_output(&self) -> Vec<Output> { self.captured.take() }
    /// The next line of input, shown with the pending text as its prompt. `None` at the end of the input.
    pub(crate) fn line(&self, span: &Span) -> Result<Option<String>, Error> { self.read(span, |input| input.line(&self.pending.take())) }
    /// The rest of the input, read after the pending text is sent.
    pub(crate) fn rest(&self, span: &Span) -> Result<Vec<u8>, Error> { self.flush(); self.read(span, |input| input.rest()) }
    /// The result of `read` on the frontend's input. A frontend with no input, or a read that fails, is a VALUE error. A read that
    /// fails as interrupted, or during an interrupt, is an interrupt.
    fn read<T>(&self, span: &Span, read: impl FnOnce(&dyn Input) -> io::Result<T>) -> Result<T, Error> {
        let input = self.input.as_deref().ok_or_else(|| span.error(ErrorKind::Value, "standard input is unavailable here"))?;
        read(input).map_err(|e| {
            if e.kind() == io::ErrorKind::Interrupted || self.interrupt.0.load(Ordering::Relaxed) {
                span.error(ErrorKind::Interrupt, "evaluation interrupted")
            } else { span.error(ErrorKind::Value, format!("standard input: {e}")) }
        })
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
        if let Some((poll, last)) = &self.poll { if now - last.get() >= POLL_INTERVAL { last.set(now); if poll() { self.interrupt.interrupt(); } } }
        if self.interrupt.0.load(Ordering::Relaxed) { return Err(span.error(ErrorKind::Interrupt, "evaluation interrupted")); }
        if self.timeout.is_some_and(|(start, limit)| now - start >= limit) { return Err(span.error(ErrorKind::Timeout, "evaluation deadline exceeded")); }
        Ok(())
    }
    /// Ends an interrupt that a guard caught, so that its handler runs. Another interrupt stops the handler.
    pub(crate) fn clear_interrupt(&self) { self.interrupt.0.store(false, Ordering::Relaxed); }
}

/// Where an evaluation step runs: the span it reports errors at, and the session it reads and changes.
pub(crate) struct Context<'a> { pub span: &'a Span, pub session: &'a mut crate::Session }
impl Context<'_> {
    pub(crate) fn check(&self) -> Result<(), Error> { self.session.execution.check(self.span) }
    pub(crate) fn rest(&self) -> Result<Vec<u8>, Error> { self.session.execution.rest(self.span) }
    pub(crate) fn write(&self, text: &str) { self.session.execution.write(text) }
}
impl Deref for Context<'_> {
    type Target = Span;
    fn deref(&self) -> &Span { self.span }
}
