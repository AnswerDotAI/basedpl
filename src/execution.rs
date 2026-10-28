use crate::{Error, ErrorKind, Span, host::Instant};
use rustymath::error::Kind;
use std::{cell::{Cell, RefCell}, io, sync::{Arc, Mutex, MutexGuard, PoisonError, atomic::{AtomicBool, AtomicU32, Ordering}}, time::Duration};

/// Cancellation for one evaluation; may be sent to another thread.
#[derive(Clone, Default)]
pub struct InterruptHandle(Arc<AtomicBool>);
impl InterruptHandle {
    pub fn interrupt(&self) { self.0.store(true, Ordering::Relaxed); }
    fn is_set(&self) -> bool { self.0.load(Ordering::Relaxed) }
}

pub struct EvalOptions {
    pub interrupt: InterruptHandle,
    pub timeout: Option<Duration>,
    /// Include implicit expression display; explicit output is always retained.
    pub echo: bool,
    /// Stream output instead of collecting it in Evaluation.output.
    pub output: Option<OutputSink>,
    /// Standard input for `⎕` and `•nget "-"`. Without one, reading either is an IO error.
    pub input: Option<Arc<dyn Input>>,
    /// Called at most every 10 ms while the evaluation checks for interruption. Returning true interrupts the evaluation.
    pub poll: Option<Poll>,
}
/// `Explicit` output and `Display` results each end a line. `Text` is written as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputKind { Explicit, Display, Text }
impl OutputKind {
    pub fn name(self) -> &'static str { match self { Self::Explicit => "explicit", Self::Display => "display", Self::Text => "text" } }
}
/// One entry of a MIME bundle: text, or bytes for a binary type such as `image/png`.
#[derive(Clone, Debug, PartialEq)]
pub enum MimeData { Text(String), Bytes(Vec<u8>) }
pub type MimeBundle = std::collections::BTreeMap<String, MimeData>;
#[derive(Clone, Debug, PartialEq)]
pub struct Output { pub kind: OutputKind, pub data: MimeBundle }
impl Output {
    pub(crate) fn plain(kind: OutputKind, text: String) -> Self { Self { kind, data: [("text/plain".into(), MimeData::Text(text))].into_iter().collect() } }
    pub fn text(&self) -> &str { if let Some(MimeData::Text(text)) = self.data.get("text/plain") { text } else { "" } }
    /// The text as a terminal shows it, with a line ending after any output but `Text`.
    pub fn written(&self) -> String { if self.kind == OutputKind::Text { self.text().into() } else { format!("{}\n", self.text()) } }
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

/// Calls to [`check`] between clock reads. A loop whose steps each take a millisecond still stops within about 64 ms.
const CHECKS_PER_CLOCK: u32 = 64;
/// The shortest time between two calls of a poll, and the longest sleep in one `pause`.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// What stops the running evaluation: its interrupt handle, its deadline as a start and a limit, and its poll with when the poll last ran.
struct Stops { interrupt: InterruptHandle, timeout: Option<(Instant, Duration)>, poll: Option<(Poll, Instant)> }

/// The stops of the running evaluation. Every thread of the process checks them: the thread with the turn, rustymath's worker threads, and a thread without a turn, such as the one Python's `in` runs on. With one session per process (`meta/session.md`, design 3), an interrupt of the running evaluation stops all the process's work.
static STOPS: Mutex<Option<Stops>> = Mutex::new(None);
fn stops() -> MutexGuard<'static, Option<Stops>> { STOPS.lock().unwrap_or_else(PoisonError::into_inner) }

/// Checks left before [`check`] next reads the clock. Each evaluation starts it at zero, so an interrupt set before the evaluation stops it at the first check. Relaxed loads and stores are enough: a check on another thread at the same time, such as Python's `in` makes without a turn, only moves the next clock read.
static COUNTDOWN: AtomicU32 = AtomicU32::new(0);

/// One evaluation's stops and output, in force until the guard drops. Dropping it restores the evaluation it ran inside, if any, as when a Python callback evaluates inside an evaluation.
pub(crate) struct Running<'a> { execution: &'a Execution, outer: (Run, bool, u32, Option<Stops>) }
impl Drop for Running<'_> {
    fn drop(&mut self) {
        let (run, echo, countdown, outer) = std::mem::take(&mut self.outer);
        self.execution.run.replace(run);
        self.execution.echo.set(echo);
        COUNTDOWN.store(countdown, Ordering::Relaxed);
        *stops() = outer;
    }
}

/// Why the running evaluation must stop: an interrupt, or a passed deadline. It first calls the poll, when `POLL_INTERVAL` has passed since the poll last ran, outside the lock. `Ok` when no evaluation runs. It's rustymath's interrupt check in this process, which its worker threads call too.
fn stopped() -> Result<(), rustymath::error::Error> {
    let now = Instant::now();
    let (interrupt, poll, late) = {
        let mut stops = stops();
        let Some(Stops { interrupt, timeout, poll }) = &mut *stops else { return Ok(()) };
        let poll = poll.as_mut().filter(|(_, last)| now - *last >= POLL_INTERVAL).map(|(poll, last)| {
            *last = now;
            poll.clone()
        });
        (interrupt.clone(), poll, matches!(timeout, Some((start, limit)) if now - *start >= *limit))
    };
    if poll.is_some_and(|poll| poll()) { interrupt.interrupt(); }
    if interrupt.is_set() { return Err(rustymath::error::Error::new(Kind::Interrupt, "")); }
    if late { return Err(rustymath::error::Error::new(Kind::Timeout, "")); }
    Ok(())
}

/// Whether the running evaluation has been interrupted.
fn interrupted() -> bool { stops().as_ref().is_some_and(|s| s.interrupt.is_set()) }

/// Ends an interrupt that a guard caught, so that its handler runs. Another interrupt stops the handler.
pub(crate) fn clear_interrupt() { if let Some(s) = &*stops() { s.interrupt.0.store(false, Ordering::Relaxed) } }

#[derive(Default)]
pub(crate) struct Execution {
    pub echo: Cell<bool>,
    /// The running evaluation's output and input, which a nested evaluation puts aside until it ends.
    run: RefCell<Run>,
}
/// One evaluation's output sink, output, input and unfinished line.
#[derive(Default)]
struct Run {
    output: Option<OutputSink>,
    /// Output that no sink takes, kept for the evaluation's result.
    captured: Vec<Output>,
    input: Option<Arc<dyn Input>>,
    /// Text written since the last line ending, which a read of `⎕` shows as its prompt.
    pending: String,
}
impl Execution {
    /// Starts an evaluation under `options`. Its stops and output are in force on this thread until the returned guard drops. The first evaluation installs [`stopped`] as rustymath's interrupt check.
    pub(crate) fn begin(&self, options: EvalOptions) -> Running<'_> {
        rustymath::interrupt::install(stopped);
        let now = Instant::now();
        let outer = std::mem::replace(
            &mut *stops(), Some(Stops { interrupt: options.interrupt, timeout: options.timeout.map(|d| (now, d)), poll: options.poll.map(|poll| (poll, now)) })
        );
        let run = self.run.replace(Run { output: options.output, input: options.input, ..Run::default() });
        Running { execution: self, outer: (run, self.echo.replace(options.echo), COUNTDOWN.swap(0, Ordering::Relaxed), outer) }
    }
    pub(crate) fn output(&self, kind: OutputKind, text: String) { self.emit(Output::plain(kind, text)); }
    /// Sends `output`, after any pending text.
    pub(crate) fn emit(&self, output: Output) {
        self.flush();
        self.send(output);
    }
    /// Sends `output` to the sink, or keeps it for the evaluation's result when there is none.
    fn send(&self, output: Output) {
        let sink = self.run.borrow().output.clone();
        if let Some(sink) = sink { sink(&output); } else { self.run.borrow_mut().captured.push(output); }
    }
    /// Writes `text` with no line ending added. Text after the last line ending waits, so that a read of `⎕` can show it as its prompt.
    pub(crate) fn write(&self, text: &str) {
        let mut run = self.run.borrow_mut();
        run.pending.push_str(text);
        let Some(end) = run.pending.rfind('\n') else { return };
        let lines = run.pending.drain(..=end).collect();
        drop(run);
        self.send(Output::plain(OutputKind::Text, lines));
    }
    /// Sends the text that waits for a line ending.
    pub(crate) fn flush(&self) {
        let text = std::mem::take(&mut self.run.borrow_mut().pending);
        if !text.is_empty() { self.send(Output::plain(OutputKind::Text, text)); }
    }
    /// The output kept since the last call.
    pub(crate) fn take_output(&self) -> Vec<Output> { std::mem::take(&mut self.run.borrow_mut().captured) }
    /// The next line of input, shown with the pending text as its prompt. `None` at the end of the input.
    pub(crate) fn line(&self, span: &Span) -> Result<Option<String>, Error> {
        self.read(span, |input| {
            let prompt = std::mem::take(&mut self.run.borrow_mut().pending);
            input.line(&prompt)
        })
    }
    /// The rest of the input, read after the pending text is sent.
    pub(crate) fn rest(&self, span: &Span) -> Result<Vec<u8>, Error> { self.flush(); self.read(span, |input| input.rest()) }
    /// The result of `read` on the frontend's input. A frontend with no input, or a read that fails, is an IO error. A read that fails as interrupted, or during an interrupt, is an interrupt.
    fn read<T>(&self, span: &Span, read: impl FnOnce(&dyn Input) -> io::Result<T>) -> Result<T, Error> {
        let input = self.run.borrow().input.clone().ok_or_else(|| Error::new(ErrorKind::Io, "standard input is unavailable here").at(span))?;
        read(&*input).map_err(|e| {
            if e.kind() == io::ErrorKind::Interrupted || interrupted() { Error::new(ErrorKind::Interrupt, "").at(span) }
            else { Error::io("standard input", e).at(span) }
        })
    }
}

/// Stops the running evaluation when it's interrupted or past its deadline. Most calls only count down, inlined into the caller. Every `CHECKS_PER_CLOCK`th call runs [`check_now`].
#[inline]
pub(crate) fn check() -> Result<(), Error> {
    match COUNTDOWN.load(Ordering::Relaxed).checked_sub(1) { Some(n) => { COUNTDOWN.store(n, Ordering::Relaxed); Ok(()) } None => check_now() }
}
/// Runs [`stopped`] at once and restarts the countdown. Callers whose checks are too far apart for `check`'s countdown call this directly.
#[cold]
#[inline(never)]
pub(crate) fn check_now() -> Result<(), Error> { COUNTDOWN.store(CHECKS_PER_CLOCK - 1, Ordering::Relaxed); Ok(stopped()?) }
/// One step of waiting: a full check, then a sleep of `limit` or `POLL_INTERVAL`, whichever is shorter. A loop of these notices an interrupt or a passed deadline within one interval.
#[cfg(not(web))]
pub(crate) fn pause(limit: Duration) -> Result<(), Error> {
    check_now()?;
    std::thread::sleep(limit.min(POLL_INTERVAL));
    Ok(())
}
