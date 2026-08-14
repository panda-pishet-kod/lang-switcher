//! The one place in the bench that is allowed to sleep.
//!
//! Requirement 1 of §11.5 of SPEC forbids deciding that a window is ready by waiting a fixed
//! amount of time. The distinction this module exists to make plain is between two things a
//! grep for `sleep` cannot tell apart on its own:
//!
//! * a **fixed delay** — "wait three seconds, then assume Word is up". Forbidden. It is either
//!   slower than it needs to be or wrong, and which of the two it is changes between runs.
//! * a **poll interval** — how often a condition that has already been asked about is asked
//!   again. Unavoidable in any wait that is not event-driven, and invisible in the result: the
//!   wait ends at the first poll that succeeds, so the observable behaviour is "as soon as the
//!   condition holds", never "after N milliseconds".
//!
//! Every wait in the bench is expressed as [`until`] over a condition. `std::thread::sleep`
//! appears exactly once in `tests\e2e\`, on the single line below, and it is the poll interval
//! of this function. A search of the bench for a fixed delay therefore has exactly one hit to
//! examine, and examining it shows a poll interval.

use std::time::{Duration, Instant};

/// How often a pending condition is re-examined.
///
/// Small enough that the granularity it adds to a measurement is below the reaction time of
/// every application the matrix drives, and large enough that a minute-long wait costs a
/// negligible number of cross-process UI Automation round trips.
pub const POLL: Duration = Duration::from_millis(50);

/// Waits until `probe` produces a value, and answers with the first one it produced.
///
/// `None` means the timeout elapsed with `probe` never succeeding — a fact the caller turns
/// into a verdict, never into an assumption that the thing happened anyway.
///
/// `probe` is asked **before** the first sleep, so a condition that already holds costs
/// nothing at all.
pub fn until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(value) = probe() {
            return Some(value);
        }

        if Instant::now() >= deadline {
            return None;
        }

        // The single poll interval of the bench. Not a fixed delay: see the module comment.
        std::thread::sleep(POLL);
    }
}

/// [`until`] for a condition that yields no value, only a yes or a no.
pub fn until_true(timeout: Duration, mut probe: impl FnMut() -> bool) -> bool {
    until(timeout, || probe().then_some(())).is_some()
}

/// How long to wait for an application window to come up.
///
/// Word is the slow one; the rest are up in a fraction of this. Because the wait ends at the
/// first successful poll, a generous bound costs nothing on the applications that are quick.
pub const WINDOW_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait for typed text to appear in the element that was read back.
pub const TEXT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for an application to disappear after it was asked to close.
pub const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);
