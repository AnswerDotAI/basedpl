use crate::{Error, ErrorKind, Span, cancel::{InterruptHandle, Poll, Stopping}};
use std::{cell::{Cell, RefCell}, io, sync::Arc, time::Duration};

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

/// One evaluation's stops and output, in force until the guard drops. Dropping it restores the evaluation it ran inside, if any, as when a Python callback evaluates inside an evaluation.
pub(crate) struct Running<'a> { execution: &'a Execution, outer: (Run, bool), _stops: Stopping }
impl Drop for Running<'_> {
    fn drop(&mut self) {
        let (run, echo) = std::mem::take(&mut self.outer);
        self.execution.run.replace(run);
        self.execution.echo.set(echo);
    }
}

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
    /// Starts an evaluation under `options`. Its stops (`cancel::begin`) and output are in force on this thread until the returned guard drops.
    pub(crate) fn begin(&self, options: EvalOptions) -> Running<'_> {
        let stops = crate::cancel::begin(options.interrupt, options.timeout, options.poll);
        let run = self.run.replace(Run { output: options.output, input: options.input, ..Run::default() });
        Running { execution: self, outer: (run, self.echo.replace(options.echo)), _stops: stops }
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
            if e.kind() == io::ErrorKind::Interrupted || crate::cancel::interrupted() { Error::new(ErrorKind::Interrupt, "").at(span) }
            else { Error::io("standard input", e).at(span) }
        })
    }
}

