//! Switching the keyboard layout, the fallback chain.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-50 (the chain of three methods with a move to the next
//! one **on failure**), FR-51 (the Windows setting that decides the scope of a switch), FR-52
//! (how the layout of the foreground window is read), and the half of FR-35 that says a
//! TSF/IME layout can never be a **target** — the backlog row of task T-05-1 names exactly
//! these four, and the backlog is the source of truth for that distribution (decision R-17).
//!
//! Implemented by backlog tasks: T-05-1.
//!
//! # The one thing that makes this module correct — decision R-32
//!
//! FR-50 says "с переходом к следующему **при неуспехе**", and the whole module turns on what
//! *неуспех* is allowed to mean.
//!
//! `PostMessage` returns success when the message has been **queued**, not when the layout has
//! changed. An application is free to ignore `WM_INPUTLANGCHANGEREQUEST` — a console host, a
//! window that has no message loop running at that instant, a control that swallows it — and
//! method 1 then "succeeds" formally without doing anything at all. A chain that trusted that
//! return value would never reach methods 2 and 3, and the whole fallback mechanism would be
//! dead code **silently**, with no error anywhere to notice it by.
//!
//! So the verdict on every method is taken from **re-reading the layout by FR-52** and
//! comparing it with the target. The return values of the Win32 calls are still examined —
//! NFR-13 requires it and this module does it — but they are counted, never branched on. See
//! [`settled`] for the wait between the attempt and the check and for the number it is bounded
//! by.
//!
//! # The second thing — decision R-31
//!
//! Method 3 of FR-50 is `ITfInputProcessorProfileMgr::ActivateProfile`, which is COM, and
//! section 6.1 writes of the input thread: "Не выполняет: UI, файловый ввод-вывод, **COM**".
//! The two cannot both be obeyed literally in one thread.
//!
//! Methods 1 and 2 run on the input thread. Method 3 is **handed to the watcher thread**, which
//! already lives in a COM STA by the same table of section 6.1, and the input thread does not
//! block on it: [`hand_over_to_watcher`] publishes the target into an atomic and posts
//! [`WM_APP_SWITCH`], which is one `PostMessageW` and a return. Method 3 is reached only after
//! the first two have failed, that is, rarely, and asynchrony costs nothing there: FR-43 puts
//! the switch **after** the replacement and NFR-09 budgets the replacement.
//!
//! # What is not here
//!
//! Choosing *which* layout to switch to — the "Пара" and "Цикл" modes of FR-30 to FR-34 and the
//! cycle position — is task **T-05-2**'s. The target arrives as a parameter of [`to`] and this
//! module has no opinion about it beyond refusing an IME (FR-35).
//!
//! # SEC-01 and SEC-07
//!
//! Nothing in this module ever sees a character, a scan code or the contents of the typing
//! buffer, and nothing that could is allowed into its counters, its arguments or its panic
//! messages. An `HKL` is an identifier of a layout, not a keystroke, and is fair game.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ACTIVATE_KEYBOARD_LAYOUT_FLAGS, ActivateKeyboardLayout, GetKeyboardLayout, HKL,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_InputProcessorProfiles, ITfInputProcessorProfileMgr, TF_IPPMF_FORSESSION,
    TF_PROFILETYPE_KEYBOARDLAYOUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, SPI_GETTHREADLOCALINPUTSETTINGS,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW, WM_APP, WM_INPUTLANGCHANGEREQUEST,
};
use windows::core::{BOOL, GUID};

use crate::layouts::LayoutId;

// ---------------------------------------------------------------------------------------
// The bounded wait — section 6.1, FR-80
// ---------------------------------------------------------------------------------------

/// How long one method is given to take effect before it is declared a failure, in
/// milliseconds.
///
/// # Why there has to be a wait at all
///
/// `PostMessage` is asynchronous. The message is put on the foreground window's queue and that
/// window processes it on its own next turn round its own message loop, so a check made in the
/// instruction after the post reads the **old** layout even when method 1 is working perfectly.
/// A chain with no wait would declare method 1 failed every single time and would run all three
/// methods on every hotkey press.
///
/// # Why it is bounded, and by this number
///
/// Section 6.1 gives the input thread the low-level hook, and FR-80 records what happens to a
/// thread that stops being available: the system removes the hook **silently** once
/// `LowLevelHooksTimeout` is exceeded — `HKCU\Control Panel\Desktop`, about **5000 ms** by
/// default. Every millisecond spent here is a millisecond the input thread is not pumping
/// messages, so the wait is a hard ceiling and not a "usually short" hope.
///
/// `20` was chosen against a measurement rather than a feeling. On the machine this was
/// developed on, a `WM_INPUTLANGCHANGEREQUEST` posted to a window of this very process takes a
/// single message-loop turn to take effect — the number is in the report of task T-05-1 — so
/// 20 ms leaves roughly an order of magnitude of headroom for an application that is busy
/// when the message arrives, while the whole chain's worst case on the input thread is
///
/// ```text
/// method 1: 20 ms  +  method 2: 20 ms  =  40 ms
/// ```
///
/// which is **one part in 125** of `LowLevelHooksTimeout`. Method 3 adds nothing to that: it is
/// handed to the watcher thread (decision R-31) and the input thread returns at once.
///
/// The ceiling is enforced on the time actually slept, not on the number of slices: see
/// [`Machine::wait`], which answers how long it really waited, because `Sleep(1)` on Windows
/// rounds up to the timer resolution and a loop counting slices would silently overshoot.
pub const VERIFY_BUDGET_MS: u32 = 20;

/// Length of one slice of the wait of [`VERIFY_BUDGET_MS`], in milliseconds.
///
/// The layout is re-read after every slice, so a switch that took effect in 2 ms is noticed in
/// 2 ms and the remaining budget is not spent. `1` is the smallest value that means anything to
/// `Sleep`; going finer would need `timeBeginPeriod`, which changes the timer resolution of the
/// **whole system** and is not this program's to change.
pub const VERIFY_POLL_MS: u32 = 1;

// ---------------------------------------------------------------------------------------
// FR-51 — the scope of a switch
// ---------------------------------------------------------------------------------------

/// What the Windows setting of FR-51 says the scope of a layout switch is.
///
/// The setting is «Позволить выбрать метод ввода для каждого окна приложения» (Settings → Time
/// & language → Typing → Advanced keyboard settings, "Let me use a different input method for
/// each app window"), and it is **on by default**.
///
/// It is read through [`scope`], which asks the OS rather than the registry; the source is
/// named in the report of task T-05-1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Scope {
    /// The setting is **on**, which is the Windows default: the layout is a property of the
    /// window, every window keeps its own, and switching one switches only that one.
    #[default]
    PerWindow,
    /// The setting is **off**: the input language is one value for the whole session and a
    /// switch anywhere is a switch everywhere.
    Session,
}

/// Reads the setting of FR-51 — `SPI_GETTHREADLOCALINPUTSETTINGS`.
///
/// `TRUE` means input settings are **thread-local**, which is the same statement as "each app
/// window may have its own input method", so it maps to [`Scope::PerWindow`].
///
/// NFR-13: a failed call is not read as `FALSE`. The setting is on by default, the default is
/// what the overwhelming majority of installations run, and answering [`Scope::PerWindow`] is
/// therefore both the safer and the more likely answer; the failure is counted in
/// [`Failures::scope_unreadable`] so that it is never silent.
pub fn scope() -> Scope {
    let mut thread_local = BOOL(0);

    // SAFETY: `SPI_GETTHREADLOCALINPUTSETTINGS` is a query, and its `pvParam` is documented to
    // be a pointer to a single `BOOL` the OS writes. `thread_local` is exactly that `BOOL`,
    // lives on this frame for the whole call, and the pointer is derived from a live mutable
    // borrow of it, so it is aligned, non-null and writable for the four bytes the OS may
    // touch. `uiParam` is zero and `fWinIni` is zero, which is what a query with no broadcast
    // takes. Nothing else of ours is reachable from the call.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETTHREADLOCALINPUTSETTINGS,
            0,
            Some(core::ptr::from_mut(&mut thread_local).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };

    match read {
        Ok(()) if thread_local.as_bool() => Scope::PerWindow,
        Ok(()) => Scope::Session,
        Err(error) => {
            // NFR-13: examined, counted and reported, never silently turned into `FALSE`.
            SCOPE_UNREADABLE.fetch_add(1, Ordering::Relaxed);
            crate::app::report_non_critical("SystemParametersInfoW", &error);
            Scope::default()
        }
    }
}

// ---------------------------------------------------------------------------------------
// FR-52 — the layout of the foreground window
// ---------------------------------------------------------------------------------------

/// **FR-52.** The keyboard layout of the window the user is typing into.
///
/// The requirement gives the expression word for word:
///
/// ```text
/// GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), null))
/// ```
///
/// and that is what this is, split across three statements only so that each of the three
/// return values can be examined as NFR-13 demands. Nothing is substituted and nothing is
/// added: in particular this is **not** `GetKeyboardLayout(0)`, which answers for the *calling*
/// thread — the input thread of this program never holds the keyboard focus, so that form would
/// report a layout of ours and never change.
///
/// A zero answer is not an error. It means there is no foreground window — which happens while
/// the desktop is switching and on the secure desktop — or that the window went away between
/// two of the three calls. It is returned as [`LayoutId::default`], and [`to`] treats that as a
/// reason to refuse rather than as a layout.
pub fn current() -> LayoutId {
    // SAFETY: `GetForegroundWindow` takes no arguments, returns a handle by value and touches
    // no memory of ours. A null result is documented and is checked immediately below.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        // NFR-13: examined. Passing a null window on would yield a thread id of zero, and zero
        // means "the calling thread" to `GetKeyboardLayout` — the one answer that would be
        // wrong rather than merely unknown.
        return LayoutId::default();
    }

    // SAFETY: `foreground` is the handle the call above returned and was checked non-null.
    // `None` for the process id is the documented way to ask for the thread id alone, and it is
    // what keeps this call from writing back through a pointer of ours.
    let thread = unsafe { GetWindowThreadProcessId(foreground, None) };

    if thread == 0 {
        // NFR-13: zero is the documented failure — the window was destroyed between the two
        // calls — and must not be forwarded as "the calling thread".
        return LayoutId::default();
    }

    // SAFETY: takes a thread id by value, returns a layout handle by value, dereferences
    // nothing. The handle is not dereferenced here either: `LayoutId` keeps the numeric value,
    // which is what module `layouts` identifies a layout by.
    let layout = unsafe { GetKeyboardLayout(thread) };

    LayoutId::from_raw(layout.0 as usize)
}

// ---------------------------------------------------------------------------------------
// FR-50 — the outcome of the chain
// ---------------------------------------------------------------------------------------

/// Which method of FR-50 did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    /// Method 1: `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`.
    PostMessage,
    /// Method 2: `AttachThreadInput` + `ActivateKeyboardLayout` + `DetachThreadInput`.
    AttachActivate,
    /// Method 3: `ITfInputProcessorProfileMgr::ActivateProfile`, on the watcher thread.
    TextServices,
}

/// What one call of [`to`] achieved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The foreground window was already on the target layout and nothing was sent.
    ///
    /// Not an error and not a failure of any method: it is the ordinary answer when the user
    /// presses the hotkey twice, and no counter moves for it.
    AlreadyActive,
    /// The layout of the foreground window is the target, and this method is the one that
    /// changed it — verified by re-reading FR-52, never by a return value (decision R-32).
    Switched(Method),
    /// Methods 1 and 2 failed and method 3 has been handed to the watcher thread (decision
    /// R-31). Whether it worked is known to the watcher thread and shows up in
    /// [`Failures::text_services`]; the input thread does not wait to find out.
    HandedOver,
}

/// Why [`to`] refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwitchError {
    /// **FR-35.** The target is served by a TSF/IME text service. Module `layouts` filters
    /// those out of the participating set, so a caller that asks for one has a defect; it is
    /// refused and counted, and no layout is touched.
    ImeTarget,
    /// The target is the zero handle, which names no layout. Refused for the same reason as
    /// [`SwitchError::ImeTarget`]: a caller defect must not turn into a Win32 call with a
    /// meaningless argument.
    NoTarget,
    /// FR-52 answered zero: there is no foreground window to switch. Happens on the secure
    /// desktop and while the desktop is being switched.
    NoForeground,
    /// Methods 1 and 2 failed and method 3 could not even be handed over — the watcher thread
    /// has no window, which means the program is starting up or shutting down.
    Exhausted,
}

// ---------------------------------------------------------------------------------------
// The seam — what the chain needs from the machine
// ---------------------------------------------------------------------------------------

/// Everything the chain of FR-50 needs from outside this program.
///
/// # Why it is a trait
///
/// The requirement of this task is a **rule about failure**: the verdict on a method comes from
/// re-reading the layout and never from the return value of the call that attempted it
/// (decision R-32). That rule is invisible from outside a function that obeys it — a chain that
/// trusted `PostMessage` behaves identically to a correct one on every machine where method 1
/// happens to work, which is most of them.
///
/// With the machine behind this trait, `tests\switch.rs` can build the one case that tells them
/// apart: a machine whose [`post_request`] answers `true` and whose [`current`] never moves. A
/// chain that reads return values stops there and reports success; a chain that obeys R-32 goes
/// on to method 2 and then to method 3. That is the only honest way to check the rule, and it
/// needs no keyboard, no foreground window and no layout of the user's touched.
///
/// It is the same seam, for the same reason, that `inject::Environment` is for the order of
/// FR-40.
///
/// [`post_request`]: Machine::post_request
/// [`current`]: Machine::current
pub trait Machine {
    /// **FR-52.** The layout of the foreground window, right now.
    ///
    /// Asked before the first method and again after every one of them: this is the verdict.
    fn current(&mut self) -> LayoutId;

    /// **Method 1.** `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`.
    ///
    /// Answers whether the message was **queued**, which NFR-13 requires to be examined and
    /// which decision R-32 forbids to be believed. The chain counts a `false` and moves on to
    /// the verification either way.
    fn post_request(&mut self, target: LayoutId) -> bool;

    /// **Method 2.** `AttachThreadInput` + `ActivateKeyboardLayout` + `DetachThreadInput`.
    ///
    /// `scope` is FR-51 and it changes what this does — see [`System::activate`].
    fn activate(&mut self, target: LayoutId, scope: Scope) -> bool;

    /// **Method 3, the handover.** Gives the target to the watcher thread (decision R-31) and
    /// returns at once, without waiting for it and without touching COM.
    ///
    /// Answers whether the watcher thread was there to take it.
    fn hand_over(&mut self, target: LayoutId, scope: Scope) -> bool;

    /// Waits up to `ms` milliseconds and answers **how long it really waited**.
    ///
    /// The answer is what bounds the wait in wall-clock time rather than in slices: `Sleep(1)`
    /// on Windows rounds up to the system timer resolution, which is about 15.6 ms by default,
    /// so a loop that counted fifteen one-millisecond slices could sit on the input thread for
    /// a quarter of a second. See [`VERIFY_BUDGET_MS`].
    fn wait(&mut self, ms: u32) -> u32;
}

/// The real machine — the [`Machine`] the program runs on.
///
/// A unit type: everything it does is a system call, so there is no state to carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct System;

impl Machine for System {
    fn current(&mut self) -> LayoutId {
        current()
    }

    fn post_request(&mut self, target: LayoutId) -> bool {
        post_request(target)
    }

    fn activate(&mut self, target: LayoutId, scope: Scope) -> bool {
        activate(target, scope)
    }

    fn hand_over(&mut self, target: LayoutId, scope: Scope) -> bool {
        hand_over_to_watcher(target, scope)
    }

    fn wait(&mut self, ms: u32) -> u32 {
        let started = Instant::now();
        thread::sleep(Duration::from_millis(u64::from(ms)));

        // Clamped into `u32` because the budget is a `u32` and a machine that suspended in the
        // middle of the sleep must end the wait, not wrap it round to zero.
        u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
    }
}

// ---------------------------------------------------------------------------------------
// FR-50 — the chain
// ---------------------------------------------------------------------------------------

/// **FR-50 and FR-35, on the real machine.** Switches the foreground window to `target`.
///
/// This is what step 5 of FR-40 calls, on the input thread, with the hook callback long
/// returned — `switch` is never reached from inside the callback, for the same reason
/// `SendInput` is not: the callback is called synchronously by the system and everything here
/// is a Win32 call, a wait or a message.
///
/// `target` is a **parameter**: choosing it is task T-05-2's (FR-30 to FR-34) and this module
/// has no opinion about it beyond the refusals of [`SwitchError`].
///
/// The scope of FR-51 is read once per call, here, so that the chain below is a pure function
/// of its arguments.
pub fn to(target: LayoutId) -> Result<Outcome, SwitchError> {
    to_in(&mut System, scope(), target)
}

/// [`to`] against any [`Machine`] — this is where the chain of FR-50 actually lives.
///
/// # The order, and what ends it
///
/// 1. the refusals: an IME target (FR-35), the zero handle, no foreground window;
/// 2. **the layout is read once before anything is attempted** — if it is already the target
///    there is nothing to do, and no method runs;
/// 3. method 1, then the verification of [`settled`];
/// 4. method 2, then the same verification;
/// 5. method 3, handed to the watcher thread and not verified here.
///
/// ⚠ **The verification is the verdict — decision R-32.** The return value of each attempt is
/// bound to a name and counted, because NFR-13 forbids discarding it, and it is never what
/// decides whether the chain moves on. Steps 3 and 4 look identical from the outside for
/// exactly that reason.
pub fn to_in(
    machine: &mut impl Machine,
    scope: Scope,
    target: LayoutId,
) -> Result<Outcome, SwitchError> {
    // ---- the refusals ------------------------------------------------------------------
    if target == LayoutId::default() {
        NO_TARGET.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::NoTarget);
    }

    // **FR-35.** Module `layouts` already keeps IME-based layouts out of the participating set,
    // so a target that is one is a defect of the caller. It is refused and counted rather than
    // switched to: a text service that composes characters breaks the correspondence between
    // keystrokes and the field's contents, which is the whole reason FR-35 exists.
    if target.is_ime() {
        IME_TARGET.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::ImeTarget);
    }

    let before = machine.current();

    if before == LayoutId::default() {
        NO_FOREGROUND.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::NoForeground);
    }

    if before == target {
        // Already there. Not a failure of anything, so no counter moves, and — the point of
        // this branch — no method runs: pressing the hotkey twice must not post a redundant
        // `WM_INPUTLANGCHANGEREQUEST` to a window that is already on the layout it asks for.
        return Ok(Outcome::AlreadyActive);
    }

    // ---- method 1 ----------------------------------------------------------------------
    if !machine.post_request(target) {
        // NFR-13: the return value is examined and its failure is counted. R-32: it is not, and
        // must never become, the thing the next line branches on.
        POST_REJECTED.fetch_add(1, Ordering::Relaxed);
    }

    if settled(machine, target) {
        return Ok(Outcome::Switched(Method::PostMessage));
    }

    POST_MESSAGE.fetch_add(1, Ordering::Relaxed);

    // ---- method 2 ----------------------------------------------------------------------
    if !machine.activate(target, scope) {
        ACTIVATE_REJECTED.fetch_add(1, Ordering::Relaxed);
    }

    if settled(machine, target) {
        return Ok(Outcome::Switched(Method::AttachActivate));
    }

    ATTACH_ACTIVATE.fetch_add(1, Ordering::Relaxed);

    // ---- method 3 — decision R-31 ------------------------------------------------------
    //
    // Not performed here. `ActivateProfile` is COM and section 6.1 forbids COM on the input
    // thread; the watcher thread already holds an STA. The handover is one atomic store and one
    // `PostMessageW`, so this thread does not block on method 3 and does not learn its verdict
    // — the watcher thread counts it in `Failures::text_services`.
    if machine.hand_over(target, scope) {
        return Ok(Outcome::HandedOver);
    }

    EXHAUSTED.fetch_add(1, Ordering::Relaxed);
    Err(SwitchError::Exhausted)
}

/// **Decision R-32 in one function:** did the layout of the foreground window actually become
/// `target`?
///
/// Re-reads FR-52 and compares. The first read happens **after** one slice of the wait and not
/// before it, because the caller has just made an asynchronous attempt and a read taken in the
/// same instruction stream is guaranteed to see the old value; a caller that wants the
/// unattempted state reads [`Machine::current`] itself, as [`to_in`] does.
///
/// The loop is bounded by [`VERIFY_BUDGET_MS`] of time actually waited, which is what
/// [`Machine::wait`] answers. Time, and not a count of slices: see the constant.
fn settled(machine: &mut impl Machine, target: LayoutId) -> bool {
    let mut waited: u32 = 0;

    while waited < VERIFY_BUDGET_MS {
        waited = waited.saturating_add(machine.wait(VERIFY_POLL_MS));

        if machine.current() == target {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------------------
// Method 1 — PostMessage
// ---------------------------------------------------------------------------------------

/// **FR-50 method 1**, exactly as the requirement writes it:
/// `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`.
///
/// Answers whether the message was queued. ⚠ That answer is not the answer to "did the layout
/// change" — see decision R-32 and the module documentation. It exists so that NFR-13 has
/// something to examine.
fn post_request(target: LayoutId) -> bool {
    // SAFETY: takes no arguments, returns a handle by value, touches no memory of ours.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        // NFR-13: examined. Posting to a null window would be posting to a thread queue, which
        // is a different operation with a different meaning.
        return false;
    }

    // `wparam` is zero, as FR-50 writes it; `lparam` carries the `HKL`. Nothing here is a
    // pointer into memory of ours, so the receiving process cannot be handed anything.
    //
    // SAFETY: `foreground` is the handle the call above returned and was checked non-null.
    // `PostMessageW` only queues the message and returns; it dereferences neither `wparam` nor
    // `lparam`, and the `HKL` in `lparam` is a handle the OS owns, not a pointer of ours. It
    // does not block, which is what keeps the input thread off the FR-80 timeout.
    let posted = unsafe {
        PostMessageW(
            Some(foreground),
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(target.raw() as isize),
        )
    };

    match posted {
        Ok(()) => true,
        Err(error) => {
            // NFR-13: examined and reported. The window went away between the two calls, or it
            // belongs to a process this one may not post to.
            crate::app::report_non_critical("PostMessageW", &error);
            false
        }
    }
}

// ---------------------------------------------------------------------------------------
// Method 2 — AttachThreadInput + ActivateKeyboardLayout + DetachThreadInput
// ---------------------------------------------------------------------------------------

/// **FR-50 method 2**, exactly as the requirement writes it: `AttachThreadInput` to the thread of
/// the active window, `ActivateKeyboardLayout`, `DetachThreadInput`.
///
/// `scope` is accepted and deliberately **not** branched on. That is a measured decision and not
/// an oversight, and the measurement is in the report of task T-05-1: with the setting of FR-51
/// off — [`Scope::Session`], which is what this machine reports — an `ActivateKeyboardLayout`
/// issued on a thread that is *not* attached to the foreground window's thread does **not** reach
/// that window. So the attach is required in both scopes, and a branch that skipped it under
/// `Scope::Session` would be a method 2 that never worked. Where FR-51 does change what the
/// program does is method 3 — see [`activate_profile`], where the scope is the documented
/// argument of the call.
///
/// Answers whether `ActivateKeyboardLayout` reported success. As everywhere in this module that
/// is examined (NFR-13) and is not the verdict (R-32).
fn activate(target: LayoutId, scope: Scope) -> bool {
    let _ = scope;

    with_foreground_attached(|| activate_here(target)).unwrap_or(false)
}

/// Runs `action` with this thread's input queue attached to the thread that owns the foreground
/// window, and detaches again however `action` ends.
///
/// `None` when there was no foreground window, no thread behind it, or the attach was refused —
/// the caller then has no way to reach that window's input state and says so.
///
/// This is the mechanism FR-50 method 2 is built out of, and it is shared with method 3 because
/// there is no second way in Windows to reach another thread's input state: both
/// `ActivateKeyboardLayout` and a thread-scoped `ITfInputProcessorProfileMgr::ActivateProfile`
/// act on the **calling** thread's queue.
///
/// ⚠ The detach is not optional. `AttachThreadInput` couples this thread's responsiveness to a
/// foreign thread's, and leaving that in place is the FR-80 failure section 6.1 exists to
/// prevent, so it runs on every path out of the attached region.
fn with_foreground_attached<T>(action: impl FnOnce() -> T) -> Option<T> {
    // SAFETY: takes no arguments, returns a handle by value, touches no memory of ours.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        return None;
    }

    // SAFETY: `foreground` was checked non-null; `None` asks for the thread id alone and makes
    // the call write nothing back through a pointer of ours.
    let theirs = unsafe { GetWindowThreadProcessId(foreground, None) };

    if theirs == 0 {
        // NFR-13: the documented failure. Attaching to thread zero is not a thing.
        return None;
    }

    // SAFETY: takes no arguments and returns the calling thread's id by value.
    let ours = unsafe { GetCurrentThreadId() };

    if theirs == ours {
        // The foreground window belongs to this very thread. `AttachThreadInput` fails when the
        // two ids are equal, and there is nothing to attach to: the queue is already ours.
        return Some(action());
    }

    // SAFETY: both ids name live threads — `theirs` was answered by the OS for a window that
    // existed a moment ago, `ours` is this thread. The call only couples two input queues and
    // writes nothing through a pointer. It is undone below on every path.
    let attached = unsafe { AttachThreadInput(ours, theirs, true) }.as_bool();

    if !attached {
        // NFR-13: examined. The foreground thread is at a higher integrity level, or it went
        // away. Nothing was attached, so nothing has to be detached.
        return None;
    }

    let done = action();

    // SAFETY: the exact inverse of the call above, with the same two ids, and it runs on every
    // path out of this function that reached the attach. Leaving the queues attached would tie
    // this thread's responsiveness to a foreign thread's for ever, which is the FR-80 failure.
    let detached = unsafe { AttachThreadInput(ours, theirs, false) }.as_bool();

    if !detached {
        // NFR-13: examined and counted. There is no second way to detach, so the only honest
        // answer is to record it.
        DETACH_FAILED.fetch_add(1, Ordering::Relaxed);
    }

    Some(done)
}

/// `ActivateKeyboardLayout` on the calling thread, with the return value examined (NFR-13).
///
/// The flags are `0`: `KLF_REORDER` would move the layout to the head of the session's list and
/// `KLF_SETFORPROCESS` would bind it to this process — both are changes to state that outlives
/// the switch, and FR-50 asks for a switch.
fn activate_here(target: LayoutId) -> bool {
    // SAFETY: the `HKL` is rebuilt from the numeric value module `layouts` keeps, which came
    // from `GetKeyboardLayoutList` or from `GetKeyboardLayout`. Constructing a pointer is safe;
    // this one is a handle and is never dereferenced, by us or by the OS. The call writes
    // nothing through a pointer of ours and returns the previous layout by value.
    let previous =
        unsafe { ActivateKeyboardLayout(hkl(target), ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0)) };

    match previous {
        // A null previous handle is the documented failure indication, not a layout.
        Ok(handle) => !handle.is_invalid(),
        Err(error) => {
            // NFR-13: examined and reported.
            crate::app::report_non_critical("ActivateKeyboardLayout", &error);
            false
        }
    }
}

// ---------------------------------------------------------------------------------------
// Method 3 — the handover to the watcher thread, decision R-31
// ---------------------------------------------------------------------------------------

/// The private message that tells the watcher thread there is a method 3 waiting for it.
///
/// `WM_APP + 8`, the next free number: `WM_APP + 1` is the wake-up of module `app`, `+ 2` is the
/// tray callback, `+ 3` and `+ 4` belong to `hook`, `+ 5` is `WM_APP_CONFIGURED`, and `+ 6` and
/// `+ 7` are `watchdog::WM_APP_FLUSH` and `watchdog::WM_APP_LAYOUT`.
///
/// **SEC-05.** The message carries nothing: `wparam` and `lparam` are zero and the target
/// travels in [`PENDING_TARGET`], which only this process writes. A forged `WM_APP_SWITCH` from
/// another process at the same integrity level finds no pending target and does nothing at all
/// — see [`run_pending`]. It is the same design as `watchdog::WM_APP_FLUSH` and for the same
/// reason.
pub const WM_APP_SWITCH: u32 = WM_APP + 8;

/// The target method 3 is to switch to, as the numeric value of its `HKL`. Zero means "nothing
/// pending".
///
/// Written by the input thread in [`publish_pending`], taken by the watcher thread in
/// [`take_pending`]. One `usize` of shared state and no mutex: section 6.3 and NFR-04.
static PENDING_TARGET: AtomicUsize = AtomicUsize::new(0);

/// The scope of FR-51 that went with [`PENDING_TARGET`]: `0` for [`Scope::PerWindow`], `1` for
/// [`Scope::Session`].
///
/// Read by the watcher thread rather than re-read there, so that method 3 uses the same scope
/// the first two methods did, even if the user changes the setting in between.
static PENDING_SCOPE: AtomicU32 = AtomicU32::new(0);

/// **The input thread's half of the handover of decision R-31.**
///
/// Publishes the target and nudges the watcher thread. One atomic store and one `PostMessageW`:
/// this does not block, does not touch COM and does not wait for method 3's verdict, which is
/// the whole point of the decision.
///
/// Answers whether the watcher thread had a window to be nudged at. When it did not — the
/// program is starting up or shutting down — the pending target is taken back, because a target
/// nobody will ever consume would be handed to the *next* `WM_APP_SWITCH` and would switch a
/// layout the user did not ask about.
pub fn hand_over_to_watcher(target: LayoutId, scope: Scope) -> bool {
    publish_pending(target, scope);

    if crate::app::post_to_watcher_thread(WM_APP_SWITCH) {
        return true;
    }

    take_pending();
    false
}

/// Puts a target on the channel of [`WM_APP_SWITCH`], without posting anything.
///
/// Split out from [`hand_over_to_watcher`] so that `tests\switch.rs` can drive both ends of the
/// channel in a test binary, which has no watcher thread and no window to post to.
pub fn publish_pending(target: LayoutId, scope: Scope) {
    PENDING_SCOPE.store(u32::from(scope == Scope::Session), Ordering::Relaxed);

    // Released after the scope, so a watcher that sees a target sees the scope that goes with
    // it: the acquire in `take_pending` pairs with this store.
    PENDING_TARGET.store(target.raw(), Ordering::Release);
}

/// **The watcher thread's half of the channel.** Takes the pending target, if there is one.
///
/// `None` means there was nothing pending, which is what a forged [`WM_APP_SWITCH`] finds
/// (SEC-05). The swap is what makes the target single-use: two messages cannot make two
/// switches out of one request.
pub fn take_pending() -> Option<(LayoutId, Scope)> {
    let raw = PENDING_TARGET.swap(0, Ordering::AcqRel);

    if raw == 0 {
        return None;
    }

    let scope = if PENDING_SCOPE.load(Ordering::Relaxed) == 0 {
        Scope::PerWindow
    } else {
        Scope::Session
    };

    Some((LayoutId::from_raw(raw), scope))
}

/// **FR-50 method 3, on the watcher thread** — what `app::window_proc` calls when
/// [`WM_APP_SWITCH`] arrives at the watcher thread's window.
///
/// `None` when nothing was pending. Otherwise the verdict of method 3, taken the way decision
/// R-32 requires of every method: by re-reading FR-52, not from the `HRESULT`.
///
/// ⚠ **Only the watcher thread may call this.** It creates a COM object, and section 6.1 forbids
/// COM on the input thread; the caller in `app::window_proc` checks that the message arrived at
/// the watcher thread's own window before it gets here.
pub fn run_pending() -> Option<bool> {
    let (target, scope) = take_pending()?;

    if !activate_profile(target, scope) {
        ACTIVATE_PROFILE_REJECTED.fetch_add(1, Ordering::Relaxed);
    }

    // R-32 applies here too: the `HRESULT` above says the profile was activated, not that the
    // foreground window is on it.
    let settled = settled(&mut System, target);

    if !settled {
        TEXT_SERVICES.fetch_add(1, Ordering::Relaxed);
    }

    Some(settled)
}

/// `ITfInputProcessorProfileMgr::ActivateProfile` for a plain keyboard layout.
///
/// # **This is where FR-51 changes what the program does**
///
/// `ActivateProfile` is scoped by its flags argument, and that argument is the most direct
/// reading of "определяющая область действия переключения" anywhere in this program. The two
/// scopes are two different calls, in both the flag and the input queue they act on:
///
/// * [`Scope::PerWindow`] — the setting is **on**, its default: each app window may keep its own
///   input method. Flags `0`, which activates the profile for the **calling thread's** input
///   queue — so the call is wrapped in [`with_foreground_attached`], because the queue that has
///   to change is the foreground window's and not the watcher thread's. Without the attach the
///   watcher thread would switch its own layout and nothing else, which is the same trap
///   decision R-32 is about, one level down.
///
/// * [`Scope::Session`] — the setting is **off**: the input language is one value for the whole
///   session. Flags `TF_IPPMF_FORSESSION`, which is the documented way to say exactly that, and
///   **no attach**: a session-wide activation is not scoped to an input queue, so coupling this
///   thread to a foreign one would be a risk taken for nothing (FR-80).
///
/// `TF_PROFILETYPE_KEYBOARDLAYOUT` with `GUID_NULL` for both the class and the profile is how a
/// plain keyboard layout — as opposed to a text service — is named to this interface; the `HKL`
/// is what identifies it, and FR-35 has already refused anything that is not one.
fn activate_profile(target: LayoutId, scope: Scope) -> bool {
    match scope {
        Scope::PerWindow => {
            with_foreground_attached(|| activate_profile_here(target, scope)).unwrap_or(false)
        }
        Scope::Session => activate_profile_here(target, scope),
    }
}

/// The COM half of [`activate_profile`], on whatever input queue the caller has arranged.
fn activate_profile_here(target: LayoutId, scope: Scope) -> bool {
    // SAFETY: `CoCreateInstance` is called on a thread that entered an STA — `app::thread_body`
    // does it for the watcher thread and this function is only reached from that thread's
    // window procedure. The class id is a `'static` constant of the `windows` crate, the outer
    // aggregate is `None`, and the interface is inferred from the binding's type, so no raw
    // pointer of ours is involved.
    let manager: ITfInputProcessorProfileMgr = match unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
    } {
        Ok(manager) => manager,
        Err(error) => {
            // NFR-13: examined and reported. TSF is not available on this desktop.
            crate::app::report_non_critical("CoCreateInstance", &error);
            return false;
        }
    };

    let flags = match scope {
        Scope::PerWindow => 0,
        Scope::Session => TF_IPPMF_FORSESSION,
    };

    let null = GUID::zeroed();

    // SAFETY: `manager` is a live interface pointer `CoCreateInstance` just returned and is
    // released by its `Drop`. The two GUID arguments are pointers to `null`, which lives for the
    // whole call — `GUID_NULL` is what this interface documents for a keyboard-layout profile —
    // and the `HKL` is a handle, never dereferenced. Nothing writes back into memory of ours.
    let activated = unsafe {
        manager.ActivateProfile(
            TF_PROFILETYPE_KEYBOARDLAYOUT,
            target.language_id(),
            &raw const null,
            &raw const null,
            hkl(target),
            flags,
        )
    };

    match activated {
        Ok(()) => true,
        Err(error) => {
            // NFR-13: examined and reported.
            crate::app::report_non_critical("ITfInputProcessorProfileMgr::ActivateProfile", &error);
            false
        }
    }
}

// ---------------------------------------------------------------------------------------
// Counters — SEC-04a, section 11.5, and module `diag` when task T-06-4 arrives
// ---------------------------------------------------------------------------------------

/// Method 1 attempted and the layout did not become the target.
static POST_MESSAGE: AtomicU32 = AtomicU32::new(0);
/// Method 2 attempted and the layout did not become the target.
static ATTACH_ACTIVATE: AtomicU32 = AtomicU32::new(0);
/// Method 3 attempted, on the watcher thread, and the layout did not become the target.
static TEXT_SERVICES: AtomicU32 = AtomicU32::new(0);
/// `PostMessageW` itself refused the message — NFR-13, not the verdict on method 1.
static POST_REJECTED: AtomicU32 = AtomicU32::new(0);
/// `ActivateKeyboardLayout` itself reported failure — NFR-13, not the verdict on method 2.
static ACTIVATE_REJECTED: AtomicU32 = AtomicU32::new(0);
/// `ActivateProfile` itself reported failure — NFR-13, not the verdict on method 3.
static ACTIVATE_PROFILE_REJECTED: AtomicU32 = AtomicU32::new(0);
/// `AttachThreadInput` would not undo an attach that succeeded.
static DETACH_FAILED: AtomicU32 = AtomicU32::new(0);
/// The target was an IME-based layout and was refused — FR-35.
static IME_TARGET: AtomicU32 = AtomicU32::new(0);
/// The target was the zero handle and was refused.
static NO_TARGET: AtomicU32 = AtomicU32::new(0);
/// FR-52 answered zero: there was no foreground window to switch.
static NO_FOREGROUND: AtomicU32 = AtomicU32::new(0);
/// All three methods were unavailable: the first two failed and the handover found no watcher.
static EXHAUSTED: AtomicU32 = AtomicU32::new(0);
/// The setting of FR-51 could not be read and the default was assumed.
static SCOPE_UNREADABLE: AtomicU32 = AtomicU32::new(0);

/// Everything this module counts.
///
/// **SEC-01, SEC-07.** Every field is a count of *events of the program*. Not one of them is
/// derived from a character, a scan code or the contents of the typing buffer, and none may ever
/// become so: this structure travels out through the debug channel of SEC-04a, which section 9
/// allows to carry metadata and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Failures {
    /// **The verdict counters of FR-50** — a method ran and the layout did not change.
    pub post_message: u32,
    /// Method 2 ran and the layout did not change.
    pub attach_activate: u32,
    /// Method 3 ran, on the watcher thread, and the layout did not change.
    pub text_services: u32,
    /// `PostMessageW` refused to queue the message of method 1 (NFR-13). Distinct from
    /// [`Failures::post_message`] on purpose: this is the return value, that is the verdict, and
    /// decision R-32 is the statement that the two are different things.
    pub post_rejected: u32,
    /// `ActivateKeyboardLayout` reported failure in method 2 (NFR-13).
    pub activate_rejected: u32,
    /// `ActivateProfile` reported failure in method 3 (NFR-13).
    pub activate_profile_rejected: u32,
    /// An `AttachThreadInput` that succeeded could not be undone.
    pub detach_failed: u32,
    /// **FR-35.** A caller asked for a TSF/IME layout as the target and was refused.
    pub ime_target: u32,
    /// A caller asked for the zero handle and was refused.
    pub no_target: u32,
    /// There was no foreground window to switch.
    pub no_foreground: u32,
    /// The chain ran out: methods 1 and 2 failed and method 3 could not be handed over.
    pub exhausted: u32,
    /// The setting of FR-51 could not be read; [`Scope::default`] was used.
    pub scope_unreadable: u32,
}

/// The counters of this module, for the debug channel of SEC-04a, for the bench of section 11.5
/// and for the journal of module `diag` when task T-06-4 wires it in.
pub fn failures() -> Failures {
    Failures {
        post_message: POST_MESSAGE.load(Ordering::Relaxed),
        attach_activate: ATTACH_ACTIVATE.load(Ordering::Relaxed),
        text_services: TEXT_SERVICES.load(Ordering::Relaxed),
        post_rejected: POST_REJECTED.load(Ordering::Relaxed),
        activate_rejected: ACTIVATE_REJECTED.load(Ordering::Relaxed),
        activate_profile_rejected: ACTIVATE_PROFILE_REJECTED.load(Ordering::Relaxed),
        detach_failed: DETACH_FAILED.load(Ordering::Relaxed),
        ime_target: IME_TARGET.load(Ordering::Relaxed),
        no_target: NO_TARGET.load(Ordering::Relaxed),
        no_foreground: NO_FOREGROUND.load(Ordering::Relaxed),
        exhausted: EXHAUSTED.load(Ordering::Relaxed),
        scope_unreadable: SCOPE_UNREADABLE.load(Ordering::Relaxed),
    }
}

/// Puts every counter back to zero.
///
/// The counters are process-wide statics, and `tests\switch.rs` runs its cases in one process:
/// a test that asserts "this refusal was counted" has to be able to start from a known point.
/// The program itself never calls this.
pub fn reset_failures() {
    for counter in [
        &POST_MESSAGE,
        &ATTACH_ACTIVATE,
        &TEXT_SERVICES,
        &POST_REJECTED,
        &ACTIVATE_REJECTED,
        &ACTIVATE_PROFILE_REJECTED,
        &DETACH_FAILED,
        &IME_TARGET,
        &NO_TARGET,
        &NO_FOREGROUND,
        &EXHAUSTED,
        &SCOPE_UNREADABLE,
    ] {
        counter.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------------------
// Small shared helper
// ---------------------------------------------------------------------------------------

/// Rebuilds the `HKL` a Win32 call takes from the numeric value module `layouts` keeps.
///
/// `layouts::LayoutId` deliberately holds the number and not the handle, so that a layout can
/// be moved between threads without an `unsafe impl Send`; this is the one place that has to
/// turn it back. Creating a pointer is a safe operation — only a dereference would not be — and
/// this value is a handle that neither this program nor the OS ever dereferences.
fn hkl(layout: LayoutId) -> HKL {
    HKL(layout.raw() as *mut c_void)
}
