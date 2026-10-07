//! Switching the keyboard layout — **one method**, and the verdict on it.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-50 (the one method a switch is made with), FR-51 (the
//! Windows setting that decides the scope of a switch), FR-52 (how the layout of the window in
//! front is read, its addendum included), and the half of FR-35 that says a TSF/IME layout can
//! never be a **target** — the backlog row of task T-05-1 names exactly these four, and the
//! backlog is the source of truth for that distribution (decision R-17).
//!
//! Implemented by backlog tasks: T-05-1, T-10-20 (the new FR-52), **Т-14-4** (the new FR-50).
//!
//! # ⭐ There used to be three methods here — task **Т-14-4**
//!
//! Until 2026-08-25 FR-50 prescribed a chain of three, each tried when the one before it had not
//! taken:
//!
//! 1. `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`;
//! 2. `AttachThreadInput` to the foreground window's thread + `ActivateKeyboardLayout` +
//!    `DetachThreadInput`;
//! 3. `ITfInputProcessorProfileMgr::ActivateProfile` — COM, and therefore handed to the watcher
//!    thread, because section 6.1 forbids COM on the input thread.
//!
//! The user struck methods 2 and 3 out of the requirement (question 63 of `DECISIONS.md`) on two
//! measurements, and the numbers are the whole argument:
//!
//! * **T-13-10** (304 circles) and **Т-14-2** (300 circles: a control window with a message loop,
//!   Windows Terminal, the classic console host in two series, and a window whose thread
//!   deliberately stops pumping) — between them the two fallbacks saved **not one circle out of
//!   504 executions**: method 2 gave a verdict 0 times out of 228, method 3 0 times out of 276.
//! * Method 1 switched the layout **everywhere a window's queue was alive**: control 60 of 60,
//!   Windows Terminal 60 of 60, the classic console host 60 of 60 by an independent channel of
//!   truth, plus 160 of 160 in T-13-10.
//! * Where method 1 is powerless — a window that has stopped pumping messages — **all three are**
//!   (0 of 60 each). There was nothing there for a fallback to save.
//! * Method 2 was not merely dead but **dangerous**: against a window that has stopped pumping,
//!   `ActivateKeyboardLayout` under `AttachThreadInput` blocked the calling thread without limit,
//!   60 times out of 60 (over 2000 ms each; the probe's own first edition sat in it for some ten
//!   minutes). In this program the caller is the input thread, which holds the low-level hook, so
//!   that is the whole keyboard of the session hanging until Windows removes the hook — FR-80,
//!   silently. ⚠ The attach itself returns fast and successfully; it is the activation under it
//!   that hangs, so no return value NFR-13 could examine would ever have caught this.
//!
//! What the chain cost while it lived: three waits of the budget below, about **95 ms on the input
//! thread for every press** in a classic console window (93.1 to 97.2 ms measured, mean 95.4) —
//! spent to reach two methods that could not help, because the judge of FR-52 is blind there and
//! never let the chain stop at the method that had already worked.
//!
//! # ⭐ The judge is blind in a classic console — FR-52's addendum, and [`Outcome::Sent`]
//!
//! Т-14-2 measured a second thing, and the user wrote it into FR-52: in a `ConsoleWindowClass`
//! window `GetWindowThreadProcessId` answers with a **counterfeit** thread id — the console
//! client's, kept for compatibility — which belongs to no thread with an input queue.
//! `GetGUIThreadInfo` on it refuses with «Параметр задан неверно» **120 times out of 120**, FR-52's
//! own fallback then reads `GetKeyboardLayout` of that counterfeit id and gets **0**, also 120 of
//! 120 — while the switch has in fact happened, 60 of 60.
//!
//! So a verdict is not merely absent there, it is *impossible*. FR-52's addendum says what to do
//! about it in as many words: the switch counts as **sent without confirmation** and no waiting for
//! confirmation is performed. [`to_in`] answers [`Outcome::Sent`], the case has a counter of its
//! own ([`Failures::sent_unconfirmed`]) so that it can never be mistaken for a real confirmation,
//! and the whole call costs one `PostMessageW` — about 110 µs measured — instead of the 95 ms it
//! used to.
//!
//! ⭐ **And the stamp of FR-04 follows it anyway — on trust.** Those same 60 of 60 are what the
//! user decided on 2026-08-25: «Двигать штамп на веру». The module answers that with a **second**
//! predicate, [`stamp_follows`], and not by widening [`confirmed`] — the belief needed a name that
//! says it is a belief, because a `confirmed` that meant "or else very likely" would have erased
//! the very distinction [`Failures::sent_unconfirmed`] exists to count. See [`stamp_follows`] for
//! what the trust rests on and for the cost of withholding it.
//!
//! ## How "the judge is blind" is decided, and why not by the window class
//!
//! By the **refusal of `GetGUIThreadInfo` itself** ([`FocusThread::Refused`]), which is the
//! condition FR-52's addendum states, and not by comparing the foreground window's class against
//! `inject::CONSOLE_WINDOW_CLASSES`. Three reasons, in the order of their weight:
//!
//! 1. The refusal *is* the fact that makes a verdict impossible. A class name is a symptom of it
//!    that happens to correlate today and would have to be maintained by hand for ever.
//! 2. That list is **wrong for this question**, and measurably so. It exists for FR-42а and holds
//!    `CASCADIA_HOSTING_WINDOW_CLASS` — Windows Terminal — whose judge Т-14-2 found perfectly
//!    **sighted**: the two threads coincided 60 of 60 and the verdict landed 60 of 60. Gating on
//!    the class would stop verifying a case that verifies every time.
//! 3. The refusal is already in hand, on a path that has just made it. Reading a class costs
//!    another Win32 call and a `String`.
//!
//! # The rule that survives the chain — decision R-32
//!
//! `PostMessage` returns success when the message has been **queued**, not when the layout has
//! changed, and an application is free to ignore `WM_INPUTLANGCHANGEREQUEST`. So the verdict is
//! taken from **re-reading the layout by FR-52** and comparing it with the target, never from the
//! return value. The return values of the Win32 calls are still examined — NFR-13 requires it and
//! this module does it — but they are counted, never branched on. See [`settled`] for the wait
//! between the attempt and the check and for the number it is bounded by.
//!
//! R-32 was written when there was a chain for a false "success" to short-circuit. With one method
//! it matters just as much and for a plainer reason: believing the return value would report a
//! switch to `inject` — through [`confirmed`] — every time an application dropped the message, and
//! the stamp of FR-04 would follow a layout that never moved.
//!
//! # ⭐ One reader of the layout, and only one — task T-10-20
//!
//! [`current`] is the whole of how this program learns what layout a foreign window is in. Until
//! task T-10-20 there was a second copy of the same four Win32 calls in `app::foreground_layout`,
//! and the price of it came due in FR-52: the requirement changed, and a change that had to be
//! made in two places by hand is a change that will one day be made in one. The duplicate is
//! gone; `app::refresh_layout_and_cache` calls this function, and so do `buffer::Recorder::
//! restamp` (task T-10-14), the selection path of FR-60/FR-61 and the verdict of FR-50's method
//! (decision R-32).
//!
//! # What is not here
//!
//! Choosing *which* layout to switch to — the "Пара" and "Цикл" modes of FR-30 to FR-34 and the
//! cycle position — is task **T-05-2**'s. The target arrives as a parameter of [`to`] and this
//! module has no opinion about it beyond refusing an IME (FR-35).
//!
//! COM is not here either, and no longer can be: method 3 was the only reason this module ever
//! touched it, and with it went the handover to the watcher thread of decision R-31. Nothing in
//! this module blocks on a foreign thread any more — there is no `AttachThreadInput` left to do it
//! with.
//!
//! # SEC-01 and SEC-07
//!
//! Nothing in this module ever sees a character, a scan code or the contents of the typing
//! buffer, and nothing that could is allowed into its counters, its arguments or its panic
//! messages. An `HKL` is an identifier of a layout, not a keystroke, and is fair game.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, PostMessageW,
    SPI_GETTHREADLOCALINPUTSETTINGS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    WM_APP, WM_INPUTLANGCHANGEREQUEST,
};
use windows::core::BOOL;

use crate::layouts::LayoutId;

// ---------------------------------------------------------------------------------------
// The bounded wait — section 6.1, FR-80
// ---------------------------------------------------------------------------------------

/// How long the method of FR-50 is given to take effect before it is declared a failure, in
/// milliseconds.
///
/// # Why there has to be a wait at all
///
/// `PostMessage` is asynchronous. The message is put on the foreground window's queue and that
/// window processes it on its own next turn round its own message loop, so a check made in the
/// instruction after the post reads the **old** layout even when the method is working perfectly.
/// With no wait every press would be declared a failure.
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
/// when the message arrives.
///
/// ⭐ **The worst case of the whole module is now this one number**, and task Т-14-4 is what made
/// it so. While FR-50 was a chain of three the worst case on the input thread was two budgets end
/// to end, and in a classic console window it was three — about 95 ms per press, measured by
/// Т-14-2, spent on methods that could not help. One budget of 20 ms is **one part in 250** of
/// `LowLevelHooksTimeout`, and in a console window not even that is spent: no verdict is possible
/// there, so no wait is performed at all (see [`Outcome::Sent`]).
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
// The retired number of method 3 — task Т-14-4
// ---------------------------------------------------------------------------------------

/// `WM_APP + 8` — **retired**, and kept only as a reservation of the number.
///
/// It used to be the message with which the input thread handed method 3 of FR-50 to the watcher
/// thread (decision R-31). Method 3 is gone with the rest of the chain — see the module
/// documentation and question 63 of `DECISIONS.md` — and with it went the atomic channel the
/// target travelled in, the handover, the watcher thread's half of it and the arm of
/// `app::window_proc` that answered this message. **Nothing in this program posts it and nothing
/// answers it.**
///
/// The name stays because the number is an entry in a register, not a private detail: the map of
/// `WM_APP + N` is written out in the documentation of `hook`, `guard`, `watchdog` and `settings`,
/// every one of which names `+ 8` as this constant, and four tests assert that no two private
/// messages of this process share a number. Freeing `+ 8` for reuse would make those four
/// documents wrong in the one way that fails silently — two messages with one number, told apart
/// by nothing.
///
/// ⚠ Do not give this number to a new message. Take the next free one.
pub const WM_APP_SWITCH: u32 = WM_APP + 8;

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

impl Scope {
    /// The word the dump of module `diag` prints for this scope — **task T-39-8, finding Н118**.
    ///
    /// A `const fn` over a closed list, which is what `diag::row_word` asks of every word it
    /// prints: never anything this program read from the outside.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PerWindow => "per_window",
            Self::Session => "session",
        }
    }
}

/// Reads the setting of FR-51 — `SPI_GETTHREADLOCALINPUTSETTINGS`.
///
/// `TRUE` means input settings are **thread-local**, which is the same statement as "each app
/// window may have its own input method", so it maps to [`Scope::PerWindow`].
///
/// # ⚠ Nothing takes this as an argument any more — task Т-14-4
///
/// It used to be passed down the chain, because methods 2 and 3 of the old FR-50 each behaved
/// differently under the two scopes: method 3 took it as the documented flags argument of
/// `ActivateProfile`, and method 2 had to attach in both. The one method FR-50 now prescribes —
/// `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)` — has no scope argument and
/// no scope-dependent behaviour: it asks *that window* to change, and the setting is what decides
/// how far the change then reaches.
///
/// So FR-51 is honoured here the only way one call permits: the setting is **read and reported**,
/// so that the diagnostics of SEC-04a say which reach was in force when a switch was made. It is
/// deliberately **not** read on the switching path, where it would be a Win32 call made for
/// nothing.
///
/// ⭐ **Until task T-39-8 nothing read it at all (finding Н118)** — the paragraph above described a
/// report nobody made, and its counter could never move. The dump of module `diag` reads it now,
/// on the thread that renders the dump — the UI thread — and prints it as the row `switch.scope`
/// beside `switch.scope_unreadable`.
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

    scope_from(read.map(|()| thread_local.as_bool()))
}

/// What the reading of [`scope`] means — **the seam of task T-39-8 (finding Н118)**, so that the
/// failure can be driven from a test: the OS does not fail `SPI_GETTHREADLOCALINPUTSETTINGS` on
/// request.
///
/// `Ok(true)` — input settings are thread-local — is [`Scope::PerWindow`]; `Ok(false)` is
/// [`Scope::Session`]; a failure is counted in [`Failures::scope_unreadable`], reported, and read
/// as the default rather than as `FALSE` (NFR-13).
pub fn scope_from(read: windows::core::Result<bool>) -> Scope {
    match read {
        Ok(true) => Scope::PerWindow,
        Ok(false) => Scope::Session,
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

/// What [`focus_thread_of`] answered — **three outcomes, and not two**.
///
/// FR-52 names three cases and gives two of them the *same* behaviour, which is exactly the shape
/// an `Option` would flatten. They are kept apart because they are different facts about the
/// machine, they are counted separately in [`Failures`], and the review of task T-10-19 recorded
/// that neither of the two fallback cases occurred once in 660 circles — a branch nobody has ever
/// seen is a branch worth being able to name when it finally happens.
///
/// ⭐ Since task Т-14-4 one of them decides more than which thread is read: [`FocusThread::Refused`]
/// is the condition of FR-52's addendum, and it is what makes a verdict impossible. See
/// [`Reading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusThread {
    /// `GetGUIThreadInfo` answered and `hwndFocus` named a window that belongs to this thread.
    Thread(u32),
    /// `GetGUIThreadInfo` answered, and the thread has **no** window with the keyboard focus:
    /// `hwndFocus` came back null. FR-52 sentence 2 — the foreground thread is used.
    NoFocus,
    /// `GetGUIThreadInfo` **refused**. FR-52 sentence 2, the other half — the foreground thread
    /// is used, which is the behaviour that was in place before this requirement changed.
    ///
    /// ⚠ **This is also the addendum of Т-14-2.** In a classic console window the refusal is what
    /// happens every time (120 of 120), and the fallback then reads a counterfeit thread id and
    /// answers `0`. Whatever layout comes out of this branch is therefore not a verdict about
    /// anything, and [`read`] marks it so.
    Refused,
}

/// **FR-52, sentence 1.** The thread that owns the window with the keyboard focus inside
/// `foreground_thread`.
///
/// `GetGUIThreadInfo` and not `GetFocus`: `GetFocus` answers for the **calling** thread's queue,
/// and no thread of this program ever holds the keyboard focus, so it would answer nothing for
/// ever. `GetGUIThreadInfo` is the one call in Win32 that answers for somebody else's thread.
///
/// # NFR-01 to NFR-05 — this runs inside the hook callback
///
/// [`current`] is called from `buffer::Recorder::restamp`, which runs in the low-level keyboard
/// callback, so this function is on the hot path and obeys its rules: `GUITHREADINFO` is a plain
/// structure on this frame (no allocation, NFR-03), the call is a synchronous read of window-
/// manager state (it cannot block, NFR-04), nothing is formatted and nothing is journalled
/// (NFR-05). ⚠ In particular the refusal is **counted and not reported**: `report_non_critical`
/// formats a string and allocates, which [`scope`] may do because it is never in the callback and
/// this must not.
fn focus_thread_of(foreground_thread: u32) -> FocusThread {
    let mut info = GUITHREADINFO {
        cbSize: u32::try_from(size_of::<GUITHREADINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: `info` is a live, properly aligned `GUITHREADINFO` owned by this frame, and its
    // `cbSize` describes it, which is the contract the call demands before it writes anything
    // into it. `foreground_thread` is the non-zero id the caller has just obtained from the
    // system. Nothing else of ours is reachable from the call. NFR-13: the `Result` is examined
    // below and a refusal is carried out of here as itself, never flattened into "no focus".
    if unsafe { GetGUIThreadInfo(foreground_thread, &raw mut info) }.is_err() {
        // NFR-13: examined and counted. A thread that is not a GUI thread, one that has just
        // gone, or the counterfeit id a classic console window answers with — and FR-52 says in
        // as many words what to do about all three.
        GUI_THREAD_INFO_REFUSED.fetch_add(1, Ordering::Relaxed);
        return FocusThread::Refused;
    }

    if info.hwndFocus.is_invalid() {
        // The thread answered and has no focus window. FR-52 calls this «hwndFocus пуст».
        FOCUS_WINDOW_ABSENT.fetch_add(1, Ordering::Relaxed);
        return FocusThread::NoFocus;
    }

    // SAFETY: `info.hwndFocus` is the handle `GetGUIThreadInfo` has just written and was checked
    // non-null. `None` for the process id is the documented way to ask for the thread id alone
    // and is what keeps this call from writing back through a pointer of ours.
    let thread = unsafe { GetWindowThreadProcessId(info.hwndFocus, None) };

    if thread == 0 {
        // NFR-13: zero is the documented failure — the focus window died between the two calls —
        // and zero means "the calling thread" to `GetKeyboardLayout`, which is the one answer
        // that would be wrong rather than merely unknown. Treated as a refusal, which FR-52
        // already has a rule for, and which is honest in the stronger sense too: the window whose
        // layout would have been the verdict no longer exists.
        GUI_THREAD_INFO_REFUSED.fetch_add(1, Ordering::Relaxed);
        return FocusThread::Refused;
    }

    FocusThread::Thread(thread)
}

/// **FR-52, the whole rule, as a function of its two inputs.**
///
/// Split out from [`read`] for one reason, and it is the reason the four previous repairs of
/// this defect are in the backlog: the two fallback cases of FR-52 **did not occur once in the
/// 660 circles task T-10-19 measured**, so a live run cannot reach them, and a branch no test can
/// reach is a branch that is not known to work. Here they are reachable from `tests\switch.rs`
/// without a foreground window, without a focus window and without Windows agreeing to refuse a
/// call on demand.
///
/// Public for that test and for nothing else — it has no callers outside [`read`].
#[must_use]
pub fn reading_thread(foreground_thread: u32, focus: FocusThread) -> u32 {
    match focus {
        FocusThread::Thread(thread) => thread,
        // ⚠ Both of these are FR-52's second sentence verbatim: «Если фокусное окно недоступно —
        // GetGUIThreadInfo отказал или hwndFocus пуст, — используется поток переднего окна, то
        // есть прежнее поведение.»
        FocusThread::NoFocus | FocusThread::Refused => foreground_thread,
    }
}

/// **FR-52 and its addendum, in one value:** the layout, and whether that layout may be believed.
///
/// # Why the two facts travel together
///
/// The addendum of Т-14-2 is not a second reading, it is a *property of the same one*: in a
/// classic console window FR-52 runs to the end and answers `0` — a number that looks exactly
/// like "there is no foreground window" and means something else entirely. Splitting the answer
/// into two calls would mean asking the window manager the same three questions twice and would
/// leave a window between them for the foreground to change; carrying both out of one read leaves
/// no such window.
///
/// [`current`] is the same reading with [`Reading::blind`] dropped, which is why every caller
/// outside this module — `app::refresh_layout_and_cache`, `buffer::Recorder::restamp`, the
/// selection path — is unchanged by the addendum: they ask what the layout is, and the answer is
/// the same one FR-52 has always given.
/// The foreground window one [`read`] was taken of — **task Т-22-4**.
///
/// A number and not an `HWND`, for the reason [`LayoutId`] is a number and not an `HKL`: it makes
/// [`Reading`] a plain value that `tests\switch.rs` can build by hand, compare, hash and print
/// without a window manager anywhere near it. The handle is rebuilt at the one place that posts.
///
/// [`Window::default`] is "there was no foreground window", which is the same `0` the system
/// answers with and is refused wherever it matters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Window(usize);

impl Window {
    /// The window a handle names.
    #[must_use]
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// The number the handle is.
    #[must_use]
    pub const fn raw(self) -> usize {
        self.0
    }

    /// Whether this names no window at all.
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Reading {
    /// **Task Т-22-4.** The foreground window this reading was taken of, so that the request of
    /// FR-50 can be sent to the window the verdict is about rather than to whatever is in front
    /// by the time it is sent. See [`to_in`] and [`post_request`].
    pub window: Window,
    /// What FR-52 answers, its own fallback included. [`LayoutId::default`] means there is no
    /// foreground window, or that nothing could be read of the one there is.
    pub layout: LayoutId,
    /// **FR-52's addendum, question 63.** `GetGUIThreadInfo` refused for the thread of the window
    /// in front, so [`Reading::layout`] is not a statement about that window and no comparison
    /// with it can be a verdict.
    ///
    /// Measured cause: a `ConsoleWindowClass` window, where this happens 120 times out of 120.
    pub blind: bool,
}

/// **FR-52 with its addendum.** The keyboard layout of the window the user is typing into, and
/// whether it may be used as a verdict.
///
/// ⭐ **This is the only place in the program that reads a keyboard layout of a foreign thread**,
/// and it has to stay the only one. Until task T-10-20 the same four Win32 calls were written out
/// a second time in `app::foreground_layout`, and the whole cost of that duplication was paid in
/// this very requirement: the two copies were kept in step by hand across four repairs of defect
/// E and would have had to be corrected twice more here.
///
/// # The requirement, and why it is not the expression it used to be
///
/// FR-52 used to give the expression word for word, and it read the layout of the thread that
/// owns the **foreground window**. Task **T-10-19** measured that this is the wrong thread: a
/// keyboard layout in Windows is a property **of a thread**, and in the packaged applications of
/// Windows 11 the top-level foreground window and the window that receives the input belong to
/// **different threads of the same process** — `Notepad` and `RichEditD2DPT`. They did not
/// coincide once in 220 circles of the packaged Notepad, and after a layout switch that moved
/// only the focus thread the foreground thread's layout disagreed with what was actually being
/// typed **120 times out of 120, and held for the whole two seconds** the series watched. The
/// focus thread's layout disagreed **0 times out of 660**. The user rewrote FR-52 on that
/// measurement; this is the new text of it, and the order below is its order:
///
/// 1. foreground window → its thread;
/// 2. `GetGUIThreadInfo` of that thread → `hwndFocus`;
/// 3. the thread of *that* window → `GetKeyboardLayout`.
///
/// **Fallback is part of the requirement, not a precaution.** If `GetGUIThreadInfo` refuses or
/// `hwndFocus` is empty, the foreground thread is used — which is exactly the behaviour that was
/// here before. See [`reading_thread`], which is that rule alone and is where it is tested.
///
/// This is still **not** `GetKeyboardLayout(0)`, which answers for the *calling* thread: no
/// thread of this program ever holds the keyboard focus, so that form would report a layout of
/// ours and never change.
///
/// A zero answer is not an error. It means there is no foreground window — which happens while
/// the desktop is switching and on the secure desktop — or that the window went away between two
/// of the calls, or that FR-52's fallback ran into the counterfeit thread id of a classic console
/// window. The last of the three is told from the other two by [`Reading::blind`], and only by it:
/// as a number it is the same `0`.
///
/// # What it costs, measured on the branch that really runs
///
/// One `GetGUIThreadInfo` and one more `GetWindowThreadProcessId` are added to a path that
/// includes the hook callback (`Recorder::restamp`, task T-10-14). Measured with
/// `--experiment-latency --words`, in which one callback sample in fourteen takes that branch:
/// the numbers are in the report of task T-10-20.
pub fn read() -> Reading {
    // SAFETY: `GetForegroundWindow` takes no arguments, returns a handle by value and touches
    // no memory of ours. A null result is documented and is checked immediately below.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        // NFR-13: examined. Passing a null window on would yield a thread id of zero, and zero
        // means "the calling thread" to `GetKeyboardLayout` — the one answer that would be
        // wrong rather than merely unknown. Not blind: there is nothing to be blind *to*, and
        // `to_in` has a refusal of its own for this.
        return Reading::default();
    }

    // SAFETY: `foreground` is the handle the call above returned and was checked non-null.
    // `None` for the process id is the documented way to ask for the thread id alone, and it is
    // what keeps this call from writing back through a pointer of ours.
    let foreground_thread = unsafe { GetWindowThreadProcessId(foreground, None) };

    if foreground_thread == 0 {
        // NFR-13: zero is the documented failure — the window was destroyed between the two
        // calls — and must not be forwarded as "the calling thread".
        return Reading::default();
    }

    let focus = focus_thread_of(foreground_thread);

    // FR-52 sentence 1, with sentence 2 as the fallback. Both live in `reading_thread`.
    let thread = reading_thread(foreground_thread, focus);

    // SAFETY: takes a thread id by value, returns a layout handle by value, dereferences
    // nothing. `thread` is non-zero on every path into here: `foreground_thread` was checked
    // above and `focus_thread_of` refuses a zero of its own. The handle is not dereferenced here
    // either: `LayoutId` keeps the numeric value, which is what module `layouts` identifies a
    // layout by.
    let layout = unsafe { GetKeyboardLayout(thread) };

    Reading {
        // **Task Т-22-4.** The window this whole reading is about, carried out with it. Every
        // number below was derived from this handle and from nothing else, so a caller that posts
        // to anything else is posting to a window no part of this answer describes.
        window: Window::from_raw(foreground.0 as usize),
        layout: LayoutId::from_raw(layout.0 as usize),
        // FR-52's addendum, question 63: a refusal means no verdict is possible about this
        // window, whatever number the fallback produced.
        blind: focus == FocusThread::Refused,
    }
}

/// **FR-52.** The keyboard layout of the window the user is typing into.
///
/// [`read`] with the addendum's flag dropped — the question every caller outside this module
/// asks, and the answer FR-52 has always given them. `app::refresh_layout_and_cache`,
/// `buffer::Recorder::restamp` (task T-10-14) and the selection path of FR-60/FR-61 all call this
/// one function, and task T-10-20 is the reason there is only one.
#[must_use]
pub fn current() -> LayoutId {
    read().layout
}

// ---------------------------------------------------------------------------------------
// FR-50 — the outcome
// ---------------------------------------------------------------------------------------

/// What one call of [`to`] achieved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The foreground window was already on the target layout and nothing was sent.
    ///
    /// Not an error and not a failure: it is the ordinary answer when the user presses the hotkey
    /// twice, and no counter moves for it.
    AlreadyActive,
    /// The layout of the foreground window **is** the target, and this call is what changed it —
    /// verified by re-reading FR-52, never by a return value (decision R-32).
    Switched,
    /// ⭐ **FR-52's addendum.** The message was posted and **no verdict is obtainable**:
    /// `GetGUIThreadInfo` refused for the window in front, which is what a classic console window
    /// does every time, so nothing this module can read is a statement about that window.
    ///
    /// The requirement's own words are «переключение считается отправленным без подтверждения,
    /// ожидания подтверждения не выполняются» — so nothing is waited for and this is not a
    /// failure. It is not a confirmation either: [`confirmed`] answers `false` for it, because
    /// the one thing that would make it true — a re-read of FR-52 — is exactly what cannot be had
    /// here. Т-14-2 measured that the switch does in fact happen in such a window 60 times out of
    /// 60; this module still may not *claim* it, and [`Failures::sent_unconfirmed`] is where the
    /// gap between the two is counted.
    ///
    /// ⭐ **The stamp of FR-04 follows it all the same** — [`stamp_follows`], the user's decision
    /// of 2026-08-25. That is a belief about this program's own model resting on those 60 of 60,
    /// and it is deliberately a different word from "confirmed".
    Sent,
}

/// Why [`to`] refused, or failed.
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
    /// FR-52 answered zero and was not blind: there is no foreground window to switch. Happens on
    /// the secure desktop and while the desktop is being switched.
    NoForeground,
    /// The message of FR-50 was posted, FR-52 could see the window, and the layout did not become
    /// the target inside [`VERIFY_BUDGET_MS`].
    ///
    /// ⚠ **This is a real answer about the machine, not a chain running out.** Т-14-2 measured
    /// the case it names: a window whose thread has stopped pumping messages never processes the
    /// posted message, so the layout cannot move (0 of 60) — and neither of the two fallback
    /// methods FR-50 used to prescribe moved it either (0 of 60 each), while one of them hung the
    /// calling thread without limit (60 of 60). There is nothing to fall back to; there never
    /// was.
    NotSwitched,
}

// ---------------------------------------------------------------------------------------
// The seam — what FR-50 needs from the machine
// ---------------------------------------------------------------------------------------

/// Everything FR-50 needs from outside this program.
///
/// # Why it is a trait
///
/// The requirement of this task is a **rule about failure**: the verdict comes from re-reading the
/// layout and never from the return value of the call that attempted it (decision R-32). That rule
/// is invisible from outside a function that obeys it — an implementation that trusted
/// `PostMessage` behaves identically to a correct one on every machine where the post is honoured,
/// which is most of them.
///
/// With the machine behind this trait, `tests\switch.rs` can build the cases that tell them apart:
/// a machine whose [`post_request`] answers `true` and whose layout never moves, and — since task
/// Т-14-4 — a machine whose [`read`] comes back blind, which is what a classic console window is
/// and what no test could otherwise produce on demand. Neither needs a keyboard, a foreground
/// window or a layout of the user's touched.
///
/// It is the same seam, for the same reason, that `inject::Environment` is for the order of
/// FR-40.
///
/// [`post_request`]: Machine::post_request
/// [`read`]: Machine::read
pub trait Machine {
    /// **FR-52 with its addendum.** What can be said about the window in front, right now.
    ///
    /// Asked once before the attempt and again after every slice of the wait: this is the verdict,
    /// and [`Reading::blind`] is the statement that there can be no verdict at all.
    fn read(&mut self) -> Reading;

    /// **FR-50.** `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`.
    ///
    /// Answers whether the message was **queued**, which NFR-13 requires to be examined and
    /// which decision R-32 forbids to be believed. A `false` is counted and the verification runs
    /// either way.
    ///
    /// ⭐ **`window` is a parameter since task Т-22-4, and that is the whole of the repair.** It
    /// is the window [`read`](Machine::read) answered about, carried here unchanged, so the
    /// request goes to the window the verdict was taken of. Until then this method read the
    /// foreground **again** for itself, and everything decided upstairs — "no foreground",
    /// "already on the target", "the judge is blind" — was decided about a window that no longer
    /// had to be the one receiving the message.
    fn post_request(&mut self, window: Window, target: LayoutId) -> bool;

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
    fn read(&mut self) -> Reading {
        read()
    }

    fn post_request(&mut self, window: Window, target: LayoutId) -> bool {
        post_request(window, target)
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
// FR-50 — the switch
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
pub fn to(target: LayoutId) -> Result<Outcome, SwitchError> {
    to_in(&mut System, target)
}

/// [`to`] against any [`Machine`] — this is where FR-50 actually lives.
///
/// # The order, and what ends it
///
/// 1. the refusals that need no machine at all: an IME target (FR-35), the zero handle;
/// 2. **FR-52 is read once, before anything is attempted** — and since task **Т-22-4** that
///    sentence is true of the *address* as well as of the verdict: the window the reading was
///    taken of travels out with it in [`Reading::window`] and is what the message is posted to,
///    so no step below can be decided about one window and acted on against another;
/// 3. ⭐ if that reading is **blind** — FR-52's addendum, the classic console window — the message
///    is posted and the call ends there with [`Outcome::Sent`]. No wait is performed, because
///    there is nothing that waiting could reveal: the judge answers the same `0` for ever;
/// 4. no foreground window at all, or the target is already active — two refusals that need the
///    reading;
/// 5. the message of FR-50, then the verification of [`settled`]: [`Outcome::Switched`] or
///    [`SwitchError::NotSwitched`].
///
/// ⚠ **The verification is the verdict — decision R-32.** The return value of the attempt is
/// bound to a name and counted, because NFR-13 forbids discarding it, and it is never what the
/// next line branches on.
///
/// ⚠ **The blind case is decided before the "already active" one**, and that order is the point
/// rather than an accident. A blind reading may carry any number at all — in a console window it
/// is `0`, but the branch is about the *refusal* and not about the number — and a value that is
/// not a statement about the window in front must not be compared with the target and must not
/// silence a switch the user asked for.
pub fn to_in(machine: &mut impl Machine, target: LayoutId) -> Result<Outcome, SwitchError> {
    // ---- the refusals that need nothing from the machine --------------------------------
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

    let before = machine.read();

    // ---- FR-52's addendum — the judge cannot answer for this window at all ---------------
    if before.blind {
        // The message goes out exactly as it would anywhere else. What does not happen is the
        // wait: `settled` would re-read the same blind `0` twenty times and end in a failure that
        // says nothing about the machine, which is the ~95 ms per press Т-14-2 measured.
        if !machine.post_request(before.window, target) {
            // NFR-13: examined and counted, here as everywhere.
            POST_REJECTED.fetch_add(1, Ordering::Relaxed);
        }

        SENT_UNCONFIRMED.fetch_add(1, Ordering::Relaxed);
        return Ok(Outcome::Sent);
    }

    if before.layout == LayoutId::default() {
        NO_FOREGROUND.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::NoForeground);
    }

    if before.layout == target {
        // Already there. Not a failure, so no counter moves, and — the point of this branch — no
        // message is sent: pressing the hotkey twice must not post a redundant
        // `WM_INPUTLANGCHANGEREQUEST` to a window that is already on the layout it asks for.
        return Ok(Outcome::AlreadyActive);
    }

    // ---- the one method of FR-50 --------------------------------------------------------
    if !machine.post_request(before.window, target) {
        // NFR-13: the return value is examined and its failure is counted. R-32: it is not, and
        // must never become, the thing the next line branches on.
        POST_REJECTED.fetch_add(1, Ordering::Relaxed);
    }

    if settled(machine, target) {
        return Ok(Outcome::Switched);
    }

    POST_MESSAGE.fetch_add(1, Ordering::Relaxed);
    Err(SwitchError::NotSwitched)
}

/// ⭐ **FR-50 for a classic console window — stage Э96, the keystroke path of decision 160.8.**
/// The message, to the window in front, and **no verdict at all**: [`Outcome::Sent`] every time.
///
/// The blind branch of [`to_in`], taken by the class of the window rather than by the refusal of
/// `GetGUIThreadInfo` — because the premise behind that branch is not the premise of every classic
/// console. **Measured, П5 of stage Э96:** in the console of the stand the thread a classic console
/// window's handle names is a thread with windows of its own, `GetGUIThreadInfo` answered for it,
/// and the layout read of it was the one it was born with — while the console switched 20 times out
/// of 20. The chain of [`to_in`] then reads that frozen layout as a verdict: it posts nothing when
/// the frozen value equals the target ([`Outcome::AlreadyActive`]) and calls the switch failed when
/// it does not move ([`SwitchError::NotSwitched`], after [`VERIFY_BUDGET_MS`] of waiting). The
/// keystroke path's keys mean what the layout of the console makes of them, so it may not skip the
/// message on a reading that is not about the console: this posts it always, and waits for nothing.
///
/// Refuses what [`to_in`] refuses before it reads anything — the zero handle and an IME target —
/// and a moment with no foreground window at all; the counters are the ones [`to_in`] keeps
/// ([`Failures::sent_unconfirmed`] counts every message sent here, as it counts the blind branch).
pub fn to_classic_console_in(
    machine: &mut impl Machine,
    target: LayoutId,
) -> Result<Outcome, SwitchError> {
    if target == LayoutId::default() {
        NO_TARGET.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::NoTarget);
    }

    if target.is_ime() {
        IME_TARGET.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::ImeTarget);
    }

    let before = machine.read();

    if before.window.is_none() {
        NO_FOREGROUND.fetch_add(1, Ordering::Relaxed);
        return Err(SwitchError::NoForeground);
    }

    if !machine.post_request(before.window, target) {
        // NFR-13: examined and counted, as in `to_in`.
        POST_REJECTED.fetch_add(1, Ordering::Relaxed);
    }

    SENT_UNCONFIRMED.fetch_add(1, Ordering::Relaxed);
    Ok(Outcome::Sent)
}

/// **Does this outcome mean the foreground window is now verifiably on the layout it was asked
/// for?** — task **T-10-5**.
///
/// The two `true` arms are the two outcomes decision R-32 has already *verified* by re-reading
/// FR-52: [`Outcome::Switched`] is "the message went out and the layout became the target",
/// [`Outcome::AlreadyActive`] is "it was the target before anything was sent". Neither is a
/// return value believed on trust — that is the whole of R-32 — so a caller may take the target
/// as fact after either of them.
///
/// The `false` cases are as much of the answer as the `true` ones:
///
/// * ⭐ [`Outcome::Sent`] — the message went out and **no verdict is obtainable** (FR-52's
///   addendum). Т-14-2 measured that in a classic console window the switch really does happen,
///   60 times out of 60; what it also measured is that nothing this program can read will say so,
///   120 times out of 120. "Almost certainly yes" is not "verified", so this answers `false` and
///   goes on answering `false`. ⚠ **Since the user's decision of 2026-08-25 this is no longer the
///   gate the stamp of FR-04 follows** — [`stamp_follows`] is, and it is `true` here. The two
///   questions were split rather than one of them widened, precisely so that this word keeps
///   meaning "re-read and seen"; [`Failures::sent_unconfirmed`] counts the gap between the two.
/// * every [`SwitchError`] — nothing was sent, or nothing took, and the window kept whatever
///   layout it had.
///
/// # Why the question is asked here and not at the call site
///
/// Because it is a statement about *this module's* vocabulary. Task T-10-5 measured that the
/// stamp of FR-04 has to follow step 5 of FR-40 — the acceptance session's «первое нажатие
/// моргает» is that stamp not following — and the only honest source for "the window is on
/// layout X now" is the verification this module already performs. A call site that matched on
/// the outcome itself would be a second copy of R-32's reasoning, free to drift from this one.
/// The same argument is why the belief of [`stamp_follows`] lives here beside it and not in
/// `inject` either.
#[must_use]
pub const fn confirmed(outcome: Result<Outcome, SwitchError>) -> bool {
    matches!(outcome, Ok(Outcome::Switched | Outcome::AlreadyActive))
}

/// **Does the stamp of FR-04 follow this outcome?** — the user's decision of 2026-08-25,
/// «Двигать штамп на веру».
///
/// [`confirmed`] answers what this program has **verified**. This answers what this program's
/// **model of the world** may be moved to. The two agree about every outcome but one, and the one
/// is [`Outcome::Sent`]: verified `false`, believed `true`.
///
/// # ⚠ Why this is a second function and not a widened [`confirmed`]
///
/// Making `confirmed` answer `true` for [`Outcome::Sent`] would have been a one-word change and
/// would have cost this module the only thing it is for. Decision R-32 is that the verdict of
/// FR-50 is a **re-reading of FR-52** and never a return value believed on trust; `Sent` is
/// exactly the case where no re-reading is possible at all. A `confirmed` that included it would
/// no longer mean "verified" but "verified, or else very likely" — and the two are the distinction
/// [`Failures::sent_unconfirmed`] exists to count. One word would have been left with two
/// meanings, and the counter would have been counting a case its own vocabulary had stopped
/// telling apart.
///
/// So the belief gets a name of its own, and the name says it is a belief about the stamp rather
/// than knowledge about the window. Every caller then picks the question it can defend:
/// `switch_layout` moves a stamp and asks this one; anything that would *report* the layout of a
/// foreign window must go on asking [`confirmed`].
///
/// # ⚠ What the belief rests on, exactly
///
/// For [`Outcome::Sent`] this answers `true` **on trust** — there is no observation inside this
/// process that supports it, and there cannot be one, which is what FR-52's addendum says.
///
/// The ground under it is task **Т-14-2**: 60 circles through a classic console window, with the
/// layout read back through a channel this program does not own — the real UI thread of `conhost`,
/// found through Toolhelp, because the counterfeit thread id the window reports is what blinds
/// FR-52 in the first place. The switch had happened **60 times out of 60**, and it had already
/// happened by the end of the one wait the retired chain's first method performed. The same run
/// measured the judge: blind **120 times out of 120**. That is the whole of the evidence, it was
/// gathered from outside, and this function is where the program acts on it anyway.
///
/// The cost of *not* believing it was measured too — by the review of task Т-14-4. With the stamp
/// frozen, `buffer::Recorder::active` keeps whatever the last readable window left there while the
/// console window runs another layout; `buffer::Recorder::record` stamps the `hkl` of every
/// following stroke with it; `inject::take_press` reads the direction of FR-26 out of the first of
/// those strokes; and `app::layout_refresh_needed` answers `false` for an unreadable layout, so
/// the stale value is never cleared either. The next conversion in that window then goes the wrong way — not
/// occasionally, but every time. Between a model that is right 60 times out of 60 and a model that
/// is wrong from the first switch onwards, the user chose the first, in as many words.
///
/// # What this does not change
///
/// [`confirmed`] and [`Failures::sent_unconfirmed`] keep precisely the meanings they had. Nothing
/// here re-reads a layout, waits for anything, or turns a return value into a verdict — R-32 is
/// untouched, because this function makes no claim about the machine at all. The counter is what
/// keeps the belief **visible**: it is still the number of switches this program sent and cannot
/// vouch for, `diag` still prints it under `switch.sent_unconfirmed`, and it is now also the
/// number of times the stamp moved without a confirmation behind it.
#[must_use]
pub const fn stamp_follows(outcome: Result<Outcome, SwitchError>) -> bool {
    // Written in terms of [`confirmed`] rather than as a second `matches!` over the same enums, so
    // that the two cannot drift: everything verified is also believed, and `Sent` is the single
    // addition this function exists for.
    confirmed(outcome) || matches!(outcome, Ok(Outcome::Sent))
}

/// **Decision R-32 in one function:** did the layout of the foreground window actually become
/// `target`?
///
/// Re-reads FR-52 and compares. The first read happens **after** one slice of the wait and not
/// before it, because the caller has just made an asynchronous attempt and a read taken in the
/// same instruction stream is guaranteed to see the old value; a caller that wants the
/// unattempted state reads [`Machine::read`] itself, as [`to_in`] does.
///
/// The loop is bounded by [`VERIFY_BUDGET_MS`] of time actually waited, which is what
/// [`Machine::wait`] answers. Time, and not a count of slices: see the constant.
///
/// ⭐ **Time is the rule; turns are the fuse — finding Н8, task T-39-7.** Windows can come back
/// from a sleep before a whole millisecond has passed, the measurement then rounds to zero, and
/// a budget counted in time alone never grows: the loop went on for ever, on the input thread
/// that serves the keyboard hook. [`VERIFY_TURNS`] — the budget over the step, plus one — ends
/// it all the same. A machine whose waits report honest time never meets the fuse, because the
/// budget runs out a turn earlier; the numbers of task Т-14-4 do not move.
///
/// ⚠ A reading that turns **blind** in the middle of the wait is not treated specially. It cannot
/// equal the target — `LayoutId` carries no such value — so the wait runs out and the answer is
/// "not settled". That is the honest answer to what such a reading means: the foreground window
/// changed under the switch, to one no verdict can be taken about. FR-52's addendum is about the
/// window that was in front when the switch was made, and [`to_in`] applies it there.
fn settled(machine: &mut impl Machine, target: LayoutId) -> bool {
    let mut waited: u32 = 0;
    let mut turns: u32 = 0;

    while waited < VERIFY_BUDGET_MS && turns < VERIFY_TURNS {
        waited = waited.saturating_add(machine.wait(VERIFY_POLL_MS));
        turns += 1;

        let reading = machine.read();

        if !reading.blind && reading.layout == target {
            return true;
        }
    }

    false
}

/// **The fuse of [`settled`] — finding Н8, task T-39-7.** The budget over the step, plus one:
/// a machine whose waits report honest time runs out of [`VERIFY_BUDGET_MS`] a turn before
/// this, and one whose waits report zero stops here instead of never.
const VERIFY_TURNS: u32 = VERIFY_BUDGET_MS / VERIFY_POLL_MS + 1;

// ---------------------------------------------------------------------------------------
// The method of FR-50 — PostMessage
// ---------------------------------------------------------------------------------------

/// **FR-50**, exactly as the requirement writes it:
/// `PostMessage(hwndForeground, WM_INPUTLANGCHANGEREQUEST, 0, hkl)`.
///
/// Answers whether the message was queued. ⚠ That answer is not the answer to "did the layout
/// change" — see decision R-32 and the module documentation. It exists so that NFR-13 has
/// something to examine.
///
/// ⚠ **It does not block, and that is a requirement rather than a convenience.** This runs on the
/// input thread, which holds the low-level hook of section 6.1; `PostMessageW` queues and returns
/// — 110 µs even against a window whose thread has stopped pumping, measured by Т-14-2 — whereas
/// the `ActivateKeyboardLayout` of the old method 2 hung there without limit, 60 times out of 60.
/// That measurement is why this is the only call left here.
///
/// # ⭐ Why the window is a parameter — task Т-22-4, finding м4 of the audit of 2026-09-01
///
/// It used to call `GetForegroundWindow` for itself. That is one reading of the foreground for the
/// **verdict** — [`to_in`] takes it through [`Machine::read`] — and a second, later one for the
/// **message**, with the whole of `to_in`'s reasoning in between. Nothing holds the foreground
/// still across that gap: a window appearing, a taskbar click, an installer stealing focus, and
/// the message of FR-50 went to a window nobody had asked a single question about. What it cost,
/// depending on which way the gap fell: the new window switched instead of the one the user was
/// typing in; or the switch was silently skipped because the *old* window was found already on
/// the target; or `settled` spent the whole [`VERIFY_BUDGET_MS`] on the input thread waiting for
/// a window that was never sent anything to change its mind.
///
/// The reading now carries the window it was taken of ([`Reading::window`]) and it is that window
/// this posts to. The window dying in the gap is unchanged and still handled where it always was:
/// `PostMessageW` refuses, the refusal is examined, counted and reported, and the verification
/// runs either way (decision R-32).
fn post_request(window: Window, target: LayoutId) -> bool {
    if window.is_none() {
        // Posting to a null window would be posting to a thread queue, which is a different
        // operation with a different meaning.
        return false;
    }

    let foreground = HWND(window.raw() as *mut c_void);

    // `wparam` is zero, as FR-50 writes it; `lparam` carries the `HKL`. Nothing here is a
    // pointer into memory of ours, so the receiving process cannot be handed anything.
    //
    // SAFETY: `foreground` is the handle `read` obtained from `GetForegroundWindow`, carried here
    // in `Reading::window` and checked non-null above. A window handle is not a pointer this
    // program dereferences, and a handle that has since gone stale is exactly what `PostMessageW`
    // answers `Err` to — which is examined below. `PostMessageW` only queues the message and
    // returns; it dereferences neither `wparam` nor `lparam`, and the `HKL` in `lparam` is a
    // handle the OS owns, not a pointer of ours. It does not block, which is what keeps the input
    // thread off the FR-80 timeout.
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
// Counters — SEC-04a, section 11.5, and module `diag`
// ---------------------------------------------------------------------------------------

/// The method of FR-50 was attempted and the layout did not become the target.
static POST_MESSAGE: AtomicU32 = AtomicU32::new(0);
/// `PostMessageW` itself refused the message — NFR-13, not the verdict.
static POST_REJECTED: AtomicU32 = AtomicU32::new(0);
/// **FR-52's addendum.** The message was sent to a window no verdict can be taken about.
static SENT_UNCONFIRMED: AtomicU32 = AtomicU32::new(0);
/// The target was an IME-based layout and was refused — FR-35.
static IME_TARGET: AtomicU32 = AtomicU32::new(0);
/// The target was the zero handle and was refused.
static NO_TARGET: AtomicU32 = AtomicU32::new(0);
/// FR-52 answered zero and could see: there was no foreground window to switch.
static NO_FOREGROUND: AtomicU32 = AtomicU32::new(0);
/// The setting of FR-51 could not be read and the default was assumed.
static SCOPE_UNREADABLE: AtomicU32 = AtomicU32::new(0);
/// **FR-52, fallback.** `GetGUIThreadInfo` refused for the foreground thread, or the focus window
/// it named died before its thread could be read; the foreground thread was used.
static GUI_THREAD_INFO_REFUSED: AtomicU32 = AtomicU32::new(0);
/// **FR-52, fallback.** `GetGUIThreadInfo` answered and the foreground thread has no window with
/// the keyboard focus; the foreground thread was used.
static FOCUS_WINDOW_ABSENT: AtomicU32 = AtomicU32::new(0);

/// Everything this module counts.
///
/// **SEC-01, SEC-07.** Every field is a count of *events of the program*. Not one of them is
/// derived from a character, a scan code or the contents of the typing buffer, and none may ever
/// become so: this structure travels out through the debug channel of SEC-04a, which section 9
/// allows to carry metadata and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Failures {
    /// **The verdict counter of FR-50** — the message went out, FR-52 could see, and the layout
    /// did not become the target inside [`VERIFY_BUDGET_MS`]. The same event as
    /// [`SwitchError::NotSwitched`].
    pub post_message: u32,
    /// `PostMessageW` refused to queue the message (NFR-13). Distinct from
    /// [`Failures::post_message`] on purpose: this is the return value, that is the verdict, and
    /// decision R-32 is the statement that the two are different things.
    pub post_rejected: u32,
    /// ⭐ **FR-52's addendum, task Т-14-4.** The message went out to a window `GetGUIThreadInfo`
    /// refuses to answer for — a classic console window — so no confirmation was waited for and
    /// none was had. See [`Outcome::Sent`].
    ///
    /// ⚠ **Not a failure, and not a confirmation.** It is here so that the two can never be
    /// confused: a number here is the count of switches this program sent and cannot vouch for,
    /// and Т-14-2 measured that they do in fact happen, 60 times out of 60.
    ///
    /// ⭐ Since task **Т-14-6** it is also the count of times the stamp of FR-04 moved on trust
    /// rather than on a verdict — see [`stamp_follows`]. That is what keeps the belief visible
    /// from outside instead of silent.
    pub sent_unconfirmed: u32,
    /// **FR-35.** A caller asked for a TSF/IME layout as the target and was refused.
    pub ime_target: u32,
    /// A caller asked for the zero handle and was refused.
    pub no_target: u32,
    /// There was no foreground window to switch.
    pub no_foreground: u32,
    /// The setting of FR-51 could not be read; [`Scope::default`] was used.
    pub scope_unreadable: u32,
    /// **FR-52.** `GetGUIThreadInfo` refused, or the focus window it named died before its thread
    /// could be read (NFR-13). The foreground thread was used — which is what the requirement
    /// says to do, so this is not an error; it is the count of how often the requirement's second
    /// sentence was the one that applied. ⚠ Task T-10-19 measured **0 of 660** in ordinary
    /// windows, and Т-14-2 measured **120 of 120** in a classic console one — which is what
    /// [`Failures::sent_unconfirmed`] then counts.
    pub focus_probe_refused: u32,
    /// **FR-52.** `GetGUIThreadInfo` answered and the foreground thread had no window with the
    /// keyboard focus. The foreground thread was used. Also **0 of 660** in T-10-19.
    pub focus_window_absent: u32,
}

/// The counters of this module, for the debug channel of SEC-04a, for the bench of section 11.5
/// and for the journal of module `diag`.
pub fn failures() -> Failures {
    Failures {
        post_message: POST_MESSAGE.load(Ordering::Relaxed),
        post_rejected: POST_REJECTED.load(Ordering::Relaxed),
        sent_unconfirmed: SENT_UNCONFIRMED.load(Ordering::Relaxed),
        ime_target: IME_TARGET.load(Ordering::Relaxed),
        no_target: NO_TARGET.load(Ordering::Relaxed),
        no_foreground: NO_FOREGROUND.load(Ordering::Relaxed),
        scope_unreadable: SCOPE_UNREADABLE.load(Ordering::Relaxed),
        focus_probe_refused: GUI_THREAD_INFO_REFUSED.load(Ordering::Relaxed),
        focus_window_absent: FOCUS_WINDOW_ABSENT.load(Ordering::Relaxed),
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
        &POST_REJECTED,
        &SENT_UNCONFIRMED,
        &IME_TARGET,
        &NO_TARGET,
        &NO_FOREGROUND,
        &SCOPE_UNREADABLE,
        &GUI_THREAD_INFO_REFUSED,
        &FOCUS_WINDOW_ABSENT,
    ] {
        counter.store(0, Ordering::Relaxed);
    }
}
