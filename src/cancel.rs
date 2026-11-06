//! Stopping the running evaluation: its interrupt, its deadline and its poll. Code checks them with `check` between steps, rustymath's worker threads through the interrupt check that `begin` installs (`stopped`), and waits with `pause`. A force of a pending array holds them off (`forcing`).
use crate::Error;
use rustymath::error::Kind;
use std::{sync::{Arc, Mutex, MutexGuard, PoisonError, atomic::{AtomicBool, AtomicU32, Ordering}}, time::Duration};

/// The platform's clock, which deadlines and timings read.
#[cfg(not(web))]
pub(crate) use std::time::Instant;
#[cfg(web)]
pub(crate) use web_time::Instant;

/// Cancellation for one evaluation; may be sent to another thread.
#[derive(Clone, Default)]
pub struct InterruptHandle(Arc<AtomicBool>);
impl InterruptHandle {
    pub fn interrupt(&self) { self.0.store(true, Ordering::Relaxed); }
    fn is_set(&self) -> bool { self.0.load(Ordering::Relaxed) }
}

/// A frontend's check of whether to interrupt the evaluation, which `stopped` calls at most every `POLL_INTERVAL`.
pub type Poll = Arc<dyn Fn() -> bool + Send + Sync>;

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

/// Forces of pending arrays running now, on any thread. A force can't fail, so [`stopped`] holds off while one runs: a deadline or interrupt that comes due mid-force stops the evaluation at the first check after it.
static FORCING: AtomicU32 = AtomicU32::new(0);
/// Holds off the running evaluation's stops while it lives. `Value::storage`'s force takes it; `Value::forced`, which can fail, doesn't.
pub(crate) struct Forcing(());
impl Drop for Forcing { fn drop(&mut self) { FORCING.fetch_sub(1, Ordering::Relaxed); } }
pub(crate) fn forcing() -> Forcing { FORCING.fetch_add(1, Ordering::Relaxed); Forcing(()) }

/// Why the running evaluation must stop: an interrupt, or a passed deadline. It first calls the poll, when `POLL_INTERVAL` has passed since the poll last ran, outside the lock. `Ok` when no evaluation runs. It's rustymath's interrupt check in this process, which its worker threads call too.
fn stopped() -> Result<(), rustymath::error::Error> {
    if FORCING.load(Ordering::Relaxed) > 0 { return Ok(()) }
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
pub(crate) fn interrupted() -> bool { stops().as_ref().is_some_and(|s| s.interrupt.is_set()) }
/// Whether an evaluation runs, whose stops are in force.
pub(crate) fn running() -> bool { stops().is_some() }

/// Ends an interrupt that a guard caught, so that its handler runs. Another interrupt stops the handler.
pub(crate) fn clear_interrupt() { if let Some(s) = &*stops() { s.interrupt.0.store(false, Ordering::Relaxed) } }

/// One evaluation's stops, in force until it drops. Dropping it restores the stops it replaced, as when a Python callback evaluates inside an evaluation.
pub(crate) struct Stopping { outer: (u32, Option<Stops>) }
impl Drop for Stopping {
    fn drop(&mut self) {
        let (countdown, outer) = std::mem::take(&mut self.outer);
        COUNTDOWN.store(countdown, Ordering::Relaxed);
        *stops() = outer;
    }
}

/// Puts `interrupt`, a deadline `timeout` from now and `poll` in force, and starts the countdown at zero. The first call installs [`stopped`] as rustymath's interrupt check.
pub(crate) fn begin(interrupt: InterruptHandle, timeout: Option<Duration>, poll: Option<Poll>) -> Stopping {
    rustymath::interrupt::install(stopped);
    let now = Instant::now();
    let outer = std::mem::replace(&mut *stops(), Some(Stops { interrupt, timeout: timeout.map(|d| (now, d)), poll: poll.map(|poll| (poll, now)) }));
    Stopping { outer: (COUNTDOWN.swap(0, Ordering::Relaxed), outer) }
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
