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

#[derive(Default)]
pub struct EvalOptions { pub interrupt: InterruptHandle, pub timeout: Option<Duration> }

#[derive(Default)]
pub(crate) struct Execution { interrupt: InterruptHandle, timeout: Option<(Instant, Duration)> }
impl Execution {
    pub(crate) fn begin(&mut self, options: EvalOptions) {
        self.interrupt = options.interrupt;
        self.timeout = options.timeout.map(|d| (Instant::now(), d));
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
