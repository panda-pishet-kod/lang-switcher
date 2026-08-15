//! Detecting password fields, the process exclusion list.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-70, FR-71, FR-72, FR-73, SEC-06 — task **T-06-1**.
//! Still to come: FR-84, the process exclusion list, task T-06-3, which reuses the publication
//! built here rather than raising a second one.
//! Implemented by backlog tasks: T-06-1 (done), T-06-3.
//!
//! # What SEC-06 costs, and why the whole shape of this module follows from one sentence
//!
//! FR-71: «Вызовы UI Automation межпроцессные и занимают десятки миллисекунд — их выполнение
//! внутри хука **категорически запрещено**». That is the strongest word in the specification,
//! and it is not a performance note: NFR-01 gives [`crate::hook::keyboard_hook_proc`] under a
//! hundred microseconds, a single UI Automation call is two to three orders of magnitude
//! above it, and a callback that overruns `LowLevelHooksTimeout` is removed by the system
//! **without a word** (FR-80). A program that asked UI Automation from inside the callback
//! would not be slow; it would stop receiving keystrokes altogether and never learn why.
//!
//! So the work is split in three, along the three threads of section 6.1:
//!
//! | Where | What runs there | Cost |
//! |---|---|---|
//! | hook callback (input thread) | **nothing of this module** — see below | zero |
//! | input thread, message loop | reads the published state once per message and switches the typing buffer on or off | one relaxed load |
//! | watcher thread, message loop | the three levels of FR-72, UI Automation included | tens of milliseconds |
//!
//! **The callback does not even read the flag**, and that is stronger than FR-71 asks rather
//! than weaker. The flag is read by the input thread — which is what section 6.3 prescribes,
//! «Флаг «поле пароля» пишется потоком наблюдения, **читается потоком ввода**» — and what the
//! input thread does with it is to take the typing buffer away, so that the one operation left
//! on the callback path is the thread-local presence check [`crate::buffer::record`] already
//! performs for every stroke of every program on the machine. Buffering off therefore costs
//! the callback *less* than buffering on, and NFR-01 to NFR-05 are untouched: no atomic was
//! added to the hot path, no branch, no call. See [`crate::app::apply_buffering_gate`] for the
//! other half of that sentence.
//!
//! ⚠ **Nor is the `WinEvent` callback a place for any of this.** A `WinEvent` callback is
//! called by the system from inside the event source's own delivery — module `watchdog` says
//! so in its own documentation and holds itself to one compare-and-swap and one
//! `PostMessageW`. Tens of milliseconds there do not merely slow this program down: they slow
//! down whatever raised the event, for every process in the session. The callback of
//! `watchdog` is left exactly as task T-03-3 wrote it, and this module attaches to what it
//! already posts.
//!
//! # How the focus change reaches this module without a second subscription
//!
//! `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` are subscribed **once**, by
//! [`crate::watchdog::watch`], on the watcher thread — task T-03-3. This module adds no
//! `SetWinEventHook` of its own and contains no such call. What it attaches to is the message
//! that subscription already posts: [`crate::watchdog::WM_APP_FLUSH`] reaches the input
//! thread's window for each of those two events and for nothing else, and
//! [`crate::app::window_proc`] hands it here as [`note_focus_moved`].
//!
//! The chain is therefore:
//!
//! ```text
//!   EVENT_OBJECT_FOCUS                     (system)
//!     └─ watchdog::win_event_proc          (watcher thread, unchanged)
//!          └─ WM_APP_FLUSH ──────────────► input thread
//!               └─ guard::note_focus_moved  state := Pending, buffering off, buffer wiped
//!                    └─ WM_APP_PROBE ────► watcher thread
//!                         └─ guard::run_pending_probe   the three levels of FR-72
//!                              └─ WM_APP_FIELD ───────► input thread
//!                                   └─ app::apply_buffering_gate   buffering on or off
//! ```
//!
//! Two hops rather than one, because the answer has to be produced on the thread that owns the
//! COM apartment and applied on the thread that owns the buffer, and those are different
//! threads (section 6.1, section 6.3). Neither hop blocks: both are `PostMessageW`.
//!
//! # ⚠ The race FR-71 leaves open, and how it is closed — [`Field::Pending`]
//!
//! Between the focus moving and the probe answering there are tens of milliseconds, and a user
//! moving to a password field is a user who is **about to type into it**. The first characters
//! of the password would arrive while the program still believed it was in the previous field.
//!
//! The resolution is that the unknown interval is not treated as "probably ordinary" but as its
//! own state: on the focus change the state becomes [`Field::Pending`] and buffering is switched
//! **off at once**, and the buffer is emptied and its memory overwritten (SEC-02). Whatever is
//! typed while the answer is outstanding is therefore not recorded, and whatever was typed
//! before it is gone. When the answer arrives it either keeps buffering off (a password field)
//! or turns it back on (anything else) — and in the second case the strokes of the interval are
//! **not** recovered, because they were never stored. Nothing is saved retroactively.
//!
//! The price is stated plainly: a handful of keystrokes made in the first tens of milliseconds
//! after a focus change are not in the buffer, in an ordinary field as much as in a password
//! one. That is the trade SEC-06 requires, and it is bounded — see [`PROBE_BUDGET_MS`].
//!
//! ⚠ **[`Field::Pending`] is not FR-73, and the two must not be confused.** FR-73 speaks of the
//! **impossibility** of determining the field — a timeout, an unknown control — and answers it
//! with buffering **on**. [`Field::Pending`] is the interval **before an answer exists at all**,
//! and it answers with buffering **off**. Opposite answers to different questions, and they are
//! separate arms of [`Field`] precisely so that neither can be reached by the other's path:
//! [`Field::Undetermined`] is stored only by [`determine`] returning it, and [`Field::Pending`]
//! only by [`note_focus_moved`].
//!
//! # FR-73 — why the default is "buffer", and why it stays
//!
//! FR-73 gives its own reason: «отказ от буферизации по умолчанию сделал бы программу
//! неработоспособной в большинстве полей на базе Electron и веб-технологий, где UI Automation
//! отвечает медленно». A program that stopped working in every browser and every Electron
//! editor would be a program nobody could use, and SEC-06 would then be enforced by the user
//! uninstalling it. The default is [`buffering_allowed`] answering `true` for
//! [`Field::Undetermined`], and it is not to be tightened.
//!
//! # SEC-01, SEC-02, SEC-07
//!
//! Nothing that passes through this module is a keystroke or is derived from one. What it
//! reads is a window class, a process name and one boolean property of an accessibility
//! element; what it publishes is one of four named states and a row of counts. **The contents
//! of the field are never read** — not by `WM_GETTEXT`, not through `ValuePattern`, not
//! through `TextPattern` — and there is no call in this module that could return them.
//! `EM_GETPASSWORDCHAR` returns the *mask* character the control paints, which is a property of
//! the control and not of what is in it, and even that is narrowed to a boolean before it is
//! stored (see [`field_for_password_char`]).
//!
//! The process names of [`CREDENTIAL_PROCESSES`] are compared and discarded; none of them
//! reaches a journal, a file or a panic message, and neither does the name of any other
//! process. SEC-02 is met by [`crate::app::apply_buffering_gate`], which flushes through
//! [`crate::buffer::reset`] — the one function of module `buffer` that overwrites the ring with
//! zeroes — before the buffer is taken away.
//!
//! # NFR-13, NFR-14
//!
//! Every Win32 result here is examined, and a failure is an *answer* rather than an escalation:
//! a process whose name cannot be read is not a credential process as far as level 1 is
//! concerned, and a UI Automation call that fails is [`Field::Undetermined`], which FR-73 fixes
//! the meaning of. Every `unsafe` block carries a `// SAFETY:` comment.

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::cell::RefCell;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, MAX_PATH, WPARAM};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation, IUIAutomation2};
use windows::Win32::UI::Controls::EM_GETPASSWORDCHAR;
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_APP,
};
use windows::core::{Interface, PWSTR, Result as WinResult};

// ---------------------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------------------

/// The private message that asks the **watcher** thread to run the three levels of FR-72.
///
/// `WM_APP + 10`, the next free number: `+ 1` is the wake-up of [`crate::app`], `+ 2` the tray
/// callback, `+ 3` and `+ 4` belong to [`crate::hook`], `+ 5` is the configuration nudge,
/// `+ 6` and `+ 7` are [`crate::watchdog::WM_APP_FLUSH`] and
/// [`crate::watchdog::WM_APP_LAYOUT`], `+ 8` is [`crate::switch::WM_APP_SWITCH`] and `+ 9` is
/// [`crate::watchdog::WM_APP_REHOOK`]. A test asserts that the whole set is distinct, because
/// two private messages that collide do so silently.
///
/// Posted by the input thread from [`note_focus_moved`], and answered on the watcher window and
/// on no other — see [`crate::app::window_proc`].
///
/// SEC-05: it carries nothing. Whether a probe is wanted travels in [`PROBE_PENDING`], an atomic
/// of this process no sender can reach, and a forged message that finds it empty does nothing at
/// all.
pub const WM_APP_PROBE: u32 = WM_APP + 10;

/// The private message that tells the **input** thread a verdict has been published.
///
/// `WM_APP + 11`, the next free number after [`WM_APP_PROBE`].
///
/// SEC-05: it carries nothing either — the verdict travels in [`FIELD`], and the handler
/// re-reads that atomic rather than anything the sender supplied. A forged message buys the
/// sender one re-application of a state this program publishes to itself.
pub const WM_APP_FIELD: u32 = WM_APP + 11;

/// The system windows of credentials FR-72 names at level 1, lower-cased.
///
/// ⚠ **A safeguard, not a scenario.** Limitation 1 of section 10 states that `LogonUI.exe` and
/// `consent.exe` live on the **secure desktop**, which this program never reaches: a
/// `WH_KEYBOARD_LL` hook installed on the interactive desktop is not called there at all. So
/// level 1 is expected never to fire for those two, and the list exists because FR-72 names it
/// and because `CredentialUIBroker.exe` — the broker behind the modern credential prompts —
/// **can** appear on the interactive desktop.
///
/// Lower-cased at rest so that the comparison in [`is_credential_process`] is a plain
/// `eq_ignore_ascii_case` against a value that is already in the shape it will be compared in.
pub const CREDENTIAL_PROCESSES: [&str; 3] =
    ["logonui.exe", "credentialuibroker.exe", "consent.exe"];

/// Class name of the classic Win32 single-line and multi-line editor — level 2 of FR-72.
pub const EDIT_CLASS: &str = "Edit";

/// Timeout of the `SendMessageTimeout` of level 2, in milliseconds.
///
/// ⚠ **Fifty, because FR-72 writes fifty.** It is a number out of the requirement and not a
/// tuning parameter, and it is `pub` so that a test can assert the call is made with it rather
/// than take the constant's name for the fact.
pub const PASSWORD_CHAR_TIMEOUT_MS: u32 = 50;

/// Timeout given to UI Automation at level 3, in milliseconds — `IUIAutomation2`.
///
/// **This is what makes FR-73 reachable rather than theoretical.** UI Automation is a
/// cross-process call and has no timeout argument; without a bound, an element whose provider
/// does not answer would leave the watcher thread inside the call and this module in
/// [`Field::Pending`] — that is, with buffering off — for as long as the provider felt like it.
/// FR-73 says the opposite must happen: an undeterminable field buffers. `IUIAutomation2`
/// carries two properties that bound the call, and both are set in [`automation`]; a call that
/// runs out of time returns a failing `HRESULT`, which [`determine`] turns into
/// [`Field::Undetermined`] and therefore into buffering **on**.
///
/// ⚠ **Five hundred, and the number is measured rather than chosen.** Fifty is FR-72's figure
/// for `SendMessageTimeout` and says nothing about this call. The first value tried here was two
/// hundred, and against a local page with `<input type="password">` in Chromium it was **too
/// short**: the document tree did not arrive inside it, level 3 answered "no answer", and FR-73
/// then did what FR-73 does — it turned buffering **on** in a password field. That is the exact
/// failure mode FR-73 warns about in its own text, «где UI Automation отвечает медленно», and it
/// is what makes this constant a security-relevant number and not a tuning knob. At five hundred
/// the same page is recognised, reproducibly and in both configurations of the browser; the
/// figures are in the report of task T-06-1.
///
/// What the bound has to guarantee is that the answer *arrives*, not that it arrives quickly: a
/// provider that answers in three hundred milliseconds costs three hundred, not five hundred.
/// The full budget is paid only by a provider that does not answer at all, and what it buys is
/// the FR-73 verdict instead of an indefinite [`Field::Pending`].
pub const UIA_TIMEOUT_MS: u32 = 500;

/// The worst time [`determine`] can take, in milliseconds — the width of the [`Field::Pending`]
/// window as a number rather than as a hope.
///
/// Level 1 is bounded by `OpenProcess` and `QueryFullProcessImageNameW`, neither of which waits
/// on another process; level 2 by [`PASSWORD_CHAR_TIMEOUT_MS`]; level 3 by
/// [`UIA_TIMEOUT_MS`], twice — the connection and the transaction are separate budgets. The sum
/// is what a caller may assume, and the measured figures are in the report of task T-06-1.
pub const PROBE_BUDGET_MS: u32 = PASSWORD_CHAR_TIMEOUT_MS + 2 * UIA_TIMEOUT_MS;

// ---------------------------------------------------------------------------------------
// The published state — FR-70, FR-71, FR-73
// ---------------------------------------------------------------------------------------

/// What this program currently believes about the field that has the keyboard focus.
///
/// Four arms and not two, because the two questions FR-71 and FR-73 ask are different and the
/// task specification requires them to be told apart in the code rather than in a comment:
/// [`Pending`](Self::Pending) is «ответа ещё нет» and [`Undetermined`](Self::Undetermined) is
/// «определить невозможно», and they answer [`buffering_allowed`] in **opposite** directions.
///
/// **SEC-01, SEC-07.** Four named constants. There is no arm here that could carry a character,
/// a scan code or anything the user typed, and none may ever be added: this value leaves the
/// process through the channel of SEC-04a, and the whole argument for letting it out is that the
/// set of things it can say is closed and listed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Field {
    /// The focus has moved and no verdict exists yet — **the race of FR-71**.
    ///
    /// Buffering is **off** and the buffer has been emptied. Not FR-73: see the module
    /// documentation.
    Pending = 0,
    /// Determined: an ordinary field. Buffering is on.
    Ordinary = 1,
    /// Determined: a password field — FR-70, FR-72. Buffering is **off**.
    Password = 2,
    /// **FR-73.** The probe ran and could not tell — a timeout, or a control neither level 2 nor
    /// level 3 recognises. Buffering is **on**, deliberately, and this is also the state the
    /// program starts in: nothing has been determined yet, which is the same answer.
    #[default]
    Undetermined = 3,
}

impl Field {
    /// The word the channel of SEC-04a would print for this state.
    ///
    /// Written out here rather than in [`crate::control`] so that the list of words and the list
    /// of arms cannot drift apart — the same shape [`crate::watchdog::Reason::name`] uses.
    /// The channel itself publishes the **flag** of condition 2 of SEC-04a and not this word;
    /// see [`password_field`].
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ordinary => "ordinary",
            Self::Password => "password",
            Self::Undetermined => "undetermined",
        }
    }

    /// The state a stored code names, or [`Field::Undetermined`] for anything else.
    ///
    /// Total, because the value comes out of an atomic and a total function is one less thing
    /// that can panic on a path the window procedure runs. The fall-back is the FR-73 arm on
    /// purpose: a code this build does not understand is a field it cannot determine.
    const fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Pending,
            1 => Self::Ordinary,
            2 => Self::Password,
            _ => Self::Undetermined,
        }
    }
}

/// Whether the typing buffer may be kept for `field` — **FR-70 and FR-73 in one function**.
///
/// The whole of the product decision is these two lines, and they are a `const fn` of their
/// argument so that every combination is reachable from a test without a window, a focus or a
/// keyboard.
pub const fn buffering_allowed_for(field: Field) -> bool {
    match field {
        // FR-70: «В поле ввода пароля буфер набора не ведётся».
        Field::Password => false,
        // FR-71's interval: there is no answer yet, so nothing may be kept — see the module
        // documentation for why this is the safe direction and FR-73's is the other one.
        Field::Pending => false,
        // FR-73: «Поведение по умолчанию при невозможности определить тип поля … буферизация
        // включена». Not to be tightened; the requirement carries its own justification.
        Field::Undetermined => true,
        Field::Ordinary => true,
    }
}

/// The state as published — one relaxed load, which is what section 6.3 prescribes for this
/// flag and what FR-71 means by «одной операцией».
pub fn field() -> Field {
    Field::from_code(FIELD.load(Ordering::Relaxed))
}

/// Whether the typing buffer may be kept right now — the question
/// [`crate::app::apply_buffering_gate`] asks once per message of the input thread.
///
/// One relaxed load and one `match`; no allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05).
pub fn buffering_allowed() -> bool {
    buffering_allowed_for(field())
}

/// The flag of SEC-04a: `1` in a password field, `0` everywhere else.
///
/// ⚠ **Condition 2 of SEC-04a: a flag, never the content.** What leaves the process is one bit
/// derived from one of four named states. The contents of the field are not read anywhere in
/// this module, so there is nothing else that *could* leave.
///
/// [`Field::Pending`] answers `0`, and that is the truthful answer: at that moment the program
/// does not know it is in a password field, it merely refuses to record anything until it does.
pub fn password_field() -> bool {
    matches!(field(), Field::Password)
}

/// Counts of what the probe has done — SEC-07 allows counts and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Focus changes that put the state into [`Field::Pending`].
    pub focus_changes: u32,
    /// Probes [`run_pending_probe`] carried out.
    pub probes: u32,
    /// Probes whose verdict was discarded because the focus had moved again meanwhile.
    pub stale_verdicts: u32,
    /// Verdicts of [`Field::Password`].
    pub password_verdicts: u32,
    /// Verdicts of [`Field::Ordinary`].
    pub ordinary_verdicts: u32,
    /// Verdicts of [`Field::Undetermined`] — the FR-73 arm.
    pub undetermined_verdicts: u32,
    /// Verdicts level 1 of FR-72 decided — a credential process.
    pub level1_verdicts: u32,
    /// Verdicts level 2 of FR-72 decided — an `Edit` with a mask character.
    pub level2_verdicts: u32,
    /// `SendMessageTimeout` calls of level 2 that ran out of their fifty milliseconds.
    pub level2_timeouts: u32,
    /// Level 3 calls that failed or ran out of time — the FR-73 source.
    pub level3_failures: u32,
}

/// What the probe has done so far.
pub fn counters() -> Counters {
    Counters {
        focus_changes: FOCUS_CHANGES.load(Ordering::Relaxed),
        probes: PROBES.load(Ordering::Relaxed),
        stale_verdicts: STALE_VERDICTS.load(Ordering::Relaxed),
        password_verdicts: PASSWORD_VERDICTS.load(Ordering::Relaxed),
        ordinary_verdicts: ORDINARY_VERDICTS.load(Ordering::Relaxed),
        undetermined_verdicts: UNDETERMINED_VERDICTS.load(Ordering::Relaxed),
        level1_verdicts: LEVEL1_VERDICTS.load(Ordering::Relaxed),
        level2_verdicts: LEVEL2_VERDICTS.load(Ordering::Relaxed),
        level2_timeouts: LEVEL2_TIMEOUTS.load(Ordering::Relaxed),
        level3_failures: LEVEL3_FAILURES.load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// The published state, as a [`Field`] code.
///
/// Section 6.3 names the ordering: «`AtomicBool` с упорядочением `Relaxed`». It is an
/// `AtomicU8` rather than an `AtomicBool` because the requirement's two answers turned out to be
/// four states — FR-71's interval and FR-73's impossibility are not the same thing as "password"
/// and "not password" — and the ordering is the one section 6.3 gives: `Relaxed`, because there
/// is no other datum whose visibility has to be ordered against it. The reader acts on the value
/// alone.
///
/// Starts at [`Field::Undetermined`], which is FR-73's answer and therefore buffering on: before
/// the first focus event this program has determined nothing.
static FIELD: AtomicU8 = AtomicU8::new(Field::Undetermined as u8);

/// Whether a probe has been asked for and not yet run.
///
/// SEC-05: this is why [`WM_APP_PROBE`] carries nothing. A forged message finds this `false` and
/// the handler returns without touching UI Automation.
///
/// It also coalesces: two focus changes that arrive before the watcher thread has drained its
/// queue are one probe, and the probe reads the focus that is current when it runs rather than
/// the one that caused it.
static PROBE_PENDING: AtomicBool = AtomicBool::new(false);

/// Bumped on every focus change; captured by [`run_pending_probe`] before it starts and compared
/// after it finishes.
///
/// **What it prevents.** A probe takes tens of milliseconds, and the focus can move again inside
/// them. Publishing the finished verdict blindly would then describe the field the user has
/// already left, and in the worst order — an `Ordinary` verdict for the previous field switching
/// buffering back **on** while the caret sits in a password box. A generation that has moved on
/// means the verdict is dropped and the state stays [`Field::Pending`] until the probe that
/// belongs to the current focus answers.
static FOCUS_GENERATION: AtomicU32 = AtomicU32::new(0);

/// Focus changes seen — see [`Counters`].
static FOCUS_CHANGES: AtomicU32 = AtomicU32::new(0);

/// Probes carried out.
static PROBES: AtomicU32 = AtomicU32::new(0);

/// Verdicts dropped because the focus moved again while the probe ran.
static STALE_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Verdicts of [`Field::Password`].
static PASSWORD_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Verdicts of [`Field::Ordinary`].
static ORDINARY_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Verdicts of [`Field::Undetermined`] — FR-73.
static UNDETERMINED_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Verdicts decided by level 1 of FR-72.
static LEVEL1_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Verdicts decided by level 2 of FR-72.
static LEVEL2_VERDICTS: AtomicU32 = AtomicU32::new(0);

/// Level 2 calls that ran out of their fifty milliseconds.
static LEVEL2_TIMEOUTS: AtomicU32 = AtomicU32::new(0);

/// Level 3 calls that failed or ran out of time.
static LEVEL3_FAILURES: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------------------
// The input thread's half — FR-71's race
// ---------------------------------------------------------------------------------------

/// The focus has moved: publish [`Field::Pending`] and ask the watcher thread for a verdict.
///
/// Called by [`crate::app::window_proc`] on [`crate::watchdog::WM_APP_FLUSH`], which the
/// `WinEvent` callback of module `watchdog` posts for `EVENT_SYSTEM_FOREGROUND` and
/// `EVENT_OBJECT_FOCUS` and for nothing else. **This module subscribes to nothing**: the
/// subscription is the one task T-03-3 made.
///
/// # Why the state goes to `Pending` here rather than in the `WinEvent` callback
///
/// It could be stored in the callback — a relaxed store is exactly what a `WinEvent` callback is
/// allowed to do. It is not, for one reason: `src\watchdog.rs` is accepted code and this task's
/// file scope does not include it, and reaching the same instant through the message that
/// callback already posts costs a queue hop the input thread takes while it is idle in
/// `GetMessageW`. The strokes that could slip into that hop are removed all the same, because
/// the gate empties the buffer when it switches buffering off — it does not merely stop adding
/// to it. See [`crate::app::apply_buffering_gate`].
///
/// # NFR-01 to NFR-05
///
/// Two relaxed atomics, one fetch-add and one `PostMessageW`, which queues and returns. No
/// allocation, no lock, no I/O. This runs on the input thread, in its message loop, with the
/// hook callback long returned.
pub fn note_focus_moved() {
    FOCUS_CHANGES.fetch_add(1, Ordering::Relaxed);
    FOCUS_GENERATION.fetch_add(1, Ordering::Relaxed);

    // Order matters: the state is published **before** the probe is asked for, so a watcher
    // thread that answers instantly cannot have its verdict overwritten by this store.
    FIELD.store(Field::Pending as u8, Ordering::Relaxed);

    PROBE_PENDING.store(true, Ordering::Relaxed);

    if !crate::app::post_to_watcher_thread(WM_APP_PROBE) {
        // The watcher thread has no window — it has not created one yet, or it is already gone.
        // There is nobody to answer, and leaving the state at `Pending` would mean a program
        // that never buffers again. FR-73 covers exactly this: the field cannot be determined,
        // so buffering is on.
        PROBE_PENDING.store(false, Ordering::Relaxed);
        publish(Field::Undetermined);
    }
}

// ---------------------------------------------------------------------------------------
// The watcher thread's half — the three levels of FR-72
// ---------------------------------------------------------------------------------------

/// Runs the three levels of FR-72 and publishes the verdict — **the heavy part**, on the
/// watcher thread and on no other.
///
/// Called by [`crate::app::window_proc`] for [`WM_APP_PROBE`] arriving at the **watcher**
/// window, which is what puts this on the thread section 6.1 gives the COM apartment to. The
/// same shape [`crate::switch::run_pending`] has, and for the same reason: the work belongs to a
/// thread that is not the one that noticed it was needed.
///
/// Answers whether a probe really was pending. SEC-05: a forged [`WM_APP_PROBE`] finds it was
/// not and returns having touched nothing.
pub fn run_pending_probe() -> bool {
    if !PROBE_PENDING.swap(false, Ordering::Relaxed) {
        return false;
    }

    PROBES.fetch_add(1, Ordering::Relaxed);

    // Captured before the levels run and compared after them — see `FOCUS_GENERATION`.
    let generation = FOCUS_GENERATION.load(Ordering::Relaxed);

    let verdict = determine();

    if FOCUS_GENERATION.load(Ordering::Relaxed) != generation {
        // The focus moved while this ran. The verdict describes a field the user has left, and
        // the probe that belongs to the new one is already queued. Dropping it leaves the state
        // at `Pending`, which is buffering off — the safe direction while nothing is known.
        STALE_VERDICTS.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    publish(verdict);
    true
}

/// Publishes a verdict and tells the input thread to act on it.
fn publish(verdict: Field) {
    match verdict {
        Field::Password => PASSWORD_VERDICTS.fetch_add(1, Ordering::Relaxed),
        Field::Ordinary => ORDINARY_VERDICTS.fetch_add(1, Ordering::Relaxed),
        Field::Undetermined => UNDETERMINED_VERDICTS.fetch_add(1, Ordering::Relaxed),
        // `publish` is never called with the interval state: `note_focus_moved` stores that
        // one directly, which is what keeps the two apart at the level of the code and not
        // only of the comments.
        Field::Pending => return,
    };

    FIELD.store(verdict as u8, Ordering::Relaxed);

    // The buffer is a thread-local of the input thread (section 6.3), so the thread that owns it
    // has to be the one that switches it. One `PostMessageW`, which queues and returns.
    crate::app::post_to_input_thread(WM_APP_FIELD);
}

/// **The three levels of FR-72, in the order FR-72 fixes.**
///
/// > 1. Имя процесса активного окна в списке системных окон учётных данных … → буферизация
/// >    отключена.
/// > 2. Класс сфокусированного элемента `Edit` → `SendMessageTimeout(EM_GETPASSWORDCHAR)`
/// >    с таймаутом 50 мс; ненулевой результат → буферизация отключена.
/// > 3. UI Automation: `IsPasswordProperty` сфокусированного элемента → истина → буферизация
/// >    отключена.
///
/// The order is «от дешёвого к дорогому» and it is obligatory: level 1 is two Win32 calls and a
/// string comparison, level 2 is a message with a bounded wait, level 3 is a cross-process COM
/// call. Running them in any other order would pay the expensive one for every focus change in
/// the session.
///
/// ⚠ **Only a positive answer ends the chain early.** FR-72 names three conditions under which
/// buffering is switched off; it names none under which the search stops without one. So level 1
/// answering "this is not a credential process" and level 2 answering "this `Edit` has no mask
/// character" are not verdicts — they are the absence of one, and the chain goes on. The one
/// place a negative *is* a verdict is level 3, because there is no fourth level to defer to:
/// UI Automation answering `false` is [`Field::Ordinary`] and UI Automation not answering at all
/// is [`Field::Undetermined`], which is FR-73.
///
/// # Threading
///
/// Watcher thread only. Level 3 needs the COM single-threaded apartment section 6.1 puts there,
/// and levels 1 and 2 need to be nowhere near the hook (FR-71).
fn determine() -> Field {
    let Some(window) = foreground_window() else {
        // No foreground window at all — which happens while the desktop switches and on the
        // secure desktop. Nothing can be determined, and FR-73 says what that means.
        return Field::Undetermined;
    };

    // ---- Level 1: the process behind the active window -------------------------------
    if is_credential_process(window) {
        LEVEL1_VERDICTS.fetch_add(1, Ordering::Relaxed);
        return Field::Password;
    }

    // ---- Level 2: a classic `Edit`, asked for its mask character ----------------------
    //
    // A mask character of zero, and a call that did not answer at all, both fall through to
    // level 3: neither of them says the focus is not in a password field — see the note above —
    // and only the positive answer is a verdict here.
    if let Some(focused) = focused_control(window)
        && class_is_edit(&class_name(focused))
        && field_for_password_char(password_char(focused)) == Some(Field::Password)
    {
        LEVEL2_VERDICTS.fetch_add(1, Ordering::Relaxed);
        return Field::Password;
    }

    // ---- Level 3: UI Automation ------------------------------------------------------
    match is_password_element() {
        Some(true) => Field::Password,
        Some(false) => Field::Ordinary,
        None => {
            LEVEL3_FAILURES.fetch_add(1, Ordering::Relaxed);
            Field::Undetermined
        }
    }
}

// ---------------------------------------------------------------------------------------
// Level 1 — the process of the active window
// ---------------------------------------------------------------------------------------

/// The window the user is typing into, or `None` when there is none.
fn foreground_window() -> Option<HWND> {
    // SAFETY: `GetForegroundWindow` takes no arguments, returns a handle by value and touches no
    // memory of ours. A null result is documented — no window has the focus, which happens while
    // the desktop is switching and on the secure desktop — and is handled by the caller.
    let window = unsafe { GetForegroundWindow() };

    if window.is_invalid() {
        // NFR-13: examined. A null window would get a thread id of zero out of
        // `GetWindowThreadProcessId`, and zero means "the calling thread" to half the API.
        return None;
    }

    Some(window)
}

/// Whether the process behind `window` is one of [`CREDENTIAL_PROCESSES`] — **level 1 of
/// FR-72**.
///
/// A failure to read the name is `false` and not an error: level 1 is a *recogniser*, and a
/// process whose name cannot be read is simply not recognised. The chain then goes on to levels
/// 2 and 3, which is the behaviour FR-72 prescribes for a field level 1 says nothing about.
///
/// SEC-01, SEC-07: the name is compared and dropped. Nothing here reaches a journal or a panic
/// message, and no name of any process is stored anywhere in this module.
fn is_credential_process(window: HWND) -> bool {
    let mut pid = 0u32;

    // SAFETY: `window` is the live foreground window the caller obtained and checked. The second
    // argument is an optional out-pointer to a live local of exactly the expected type; the call
    // writes one `u32` through it and dereferences nothing else. NFR-13: a zero return means the
    // window died between the two calls and is examined below.
    let thread = unsafe { GetWindowThreadProcessId(window, Some(&raw mut pid)) };

    if thread == 0 || pid == 0 {
        return false;
    }

    let Some(name) = process_file_name(pid) else {
        return false;
    };

    is_credential_process_name(&name)
}

/// Whether a bare executable name is one of [`CREDENTIAL_PROCESSES`].
///
/// Case-insensitive, because file names in Windows are: the same executable is `LogonUI.exe` in
/// the specification and `logonui.exe` in half the places the system reports it, and a
/// case-sensitive comparison would make level 1 depend on which of them arrived.
///
/// `eq_ignore_ascii_case` and not a Unicode-aware fold: all three names are ASCII, an
/// ASCII-only fold cannot be surprised by a locale, and a name that is not ASCII is by
/// construction not one of these three.
///
/// Split out from [`is_credential_process`] so the list can be driven from a test without a
/// window and without a process.
pub fn is_credential_process_name(name: &str) -> bool {
    CREDENTIAL_PROCESSES
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// The bare file name of the executable of `pid`, or `None` when it cannot be read.
///
/// `PROCESS_QUERY_LIMITED_INFORMATION` and not `PROCESS_QUERY_INFORMATION`: the limited right is
/// the one that is granted across integrity levels, which is the whole point — an elevated
/// credential window is exactly the case level 1 exists for. Asking for more than is needed
/// would turn "the name is unreadable" into the ordinary outcome.
fn process_file_name(pid: u32) -> Option<String> {
    // SAFETY: `OpenProcess` takes three values and returns a handle or an error; it dereferences
    // nothing of ours. `false` for the inheritance flag is what a handle that must not leave this
    // process needs. NFR-13: the result is a `Result` and is examined here — a refusal is the
    // ordinary answer for a process of another user or a higher integrity level, and it is not
    // journalled, because a refusal per focus change would fill the ring of module `diag` with
    // an event that is not a fault.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;

    let process = OwnedProcess(handle);

    let mut buffer = [0u16; MAX_PATH as usize];
    let mut length = buffer.len() as u32;

    // SAFETY: `process.0` is the handle just opened and not yet closed. `buffer` is a live local
    // array owned by this frame and `length` says how many **characters** of it may be written,
    // which is what the call's contract asks for; it cannot overrun the array. The call writes
    // the path and updates `length` to the number of characters actually written. NFR-13:
    // checked with `ok()?` below.
    let read = unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut length,
        )
    };

    read.ok()?;

    let path = String::from_utf16_lossy(&buffer[..length as usize]);

    Some(file_name_of(&path))
}

/// The last component of a Windows path — the executable name FR-72 compares.
///
/// FR-72 names «имя процесса», not its path, and the two differ in exactly the way that matters:
/// a program dropped somewhere else under the name `consent.exe` would still be recognised, and
/// the real `consent.exe` would still be recognised when the system reports its path in the
/// `\Device\HarddiskVolume3\…` form rather than the `C:\…` one.
///
/// Both separators, because a native path uses `\` and a path that has been through anything
/// else may use `/`.
///
/// Split out so it can be driven from a test without a process.
pub fn file_name_of(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_owned()
}

/// A process handle that closes itself.
///
/// The same shape [`crate::control`] uses for its token and its pipe, and for the same reason:
/// the handle has exactly one owner at every moment and no path out of a function leaks it.
struct OwnedProcess(HANDLE);

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the handle `OpenProcess` returned into this value and has not been
        // closed anywhere else — nothing copies it out. NFR-13: the result is examined; a handle
        // that will not close is not worth ending the process over, and the failure is recorded
        // through the one journal of the program.
        if let Err(error) = unsafe { CloseHandle(self.0) } {
            crate::app::report_non_critical("CloseHandle(process)", &error);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Level 2 — a classic `Edit`, and its mask character
// ---------------------------------------------------------------------------------------

/// The window with the keyboard focus inside the thread that owns `window`, or `None`.
///
/// `GetGUIThreadInfo` and not `GetFocus`: `GetFocus` answers for the **calling** thread's queue,
/// and the watcher thread of this program never has the keyboard focus, so it would report
/// nothing for ever. The information belongs to the thread that owns the foreground window, and
/// `GetGUIThreadInfo` is the one call that will answer for another thread.
fn focused_control(window: HWND) -> Option<HWND> {
    // SAFETY: `window` is the live foreground window; `None` for the process id is the documented
    // way to ask for the thread id alone and is what keeps the call from writing through a
    // pointer of ours. NFR-13: a zero return means the window died and is examined below.
    let thread = unsafe { GetWindowThreadProcessId(window, None) };

    if thread == 0 {
        return None;
    }

    let mut info = GUITHREADINFO {
        cbSize: u32::try_from(size_of::<GUITHREADINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: `info` is a live, properly aligned `GUITHREADINFO` owned by this frame, and its
    // `cbSize` describes it, which is the contract the call demands before it writes anything.
    // `thread` is the id the call above returned. NFR-13: the `Result` is examined — a thread
    // that is not a GUI thread, or one that has gone, is a refusal and not a fault.
    unsafe { GetGUIThreadInfo(thread, &raw mut info) }.ok()?;

    if info.hwndFocus.is_invalid() {
        // The thread has no focus window: the foreground window itself is then the best answer
        // there is, and it is what a classic dialog without a focused child looks like.
        return Some(window);
    }

    Some(info.hwndFocus)
}

/// The class name of `window`, or an empty string when it cannot be read.
///
/// Public so that `tests\guard.rs` can put a window of its own under level 2 — the same reason
/// [`class_is_edit`] and [`field_for_password_char`] are: between the three of them level 2 is
/// reachable from a test against a real `ES_PASSWORD` control without a password anywhere near
/// the machine.
pub fn class_name(window: HWND) -> String {
    // 256 characters is the documented maximum length of a registered class name, plus room for
    // the terminator the call writes.
    let mut buffer = [0u16; 257];

    // SAFETY: `buffer` is a live local array and the call is given the slice itself, so the
    // binding derives the bound from the array and cannot write past it. NFR-13: a zero return
    // is the documented failure and is turned into an empty name below, which no comparison
    // matches.
    let length = unsafe { GetClassNameW(window, &mut buffer) };

    if length <= 0 {
        return String::new();
    }

    String::from_utf16_lossy(&buffer[..length as usize])
}

/// Whether a class name is the classic editor FR-72 names at level 2.
///
/// Case-insensitive: window class names are compared case-insensitively by Windows itself, and
/// the framework that registered the control decides the spelling — `Edit` in Win32, `EDIT` in
/// some resource compilers.
///
/// ⚠ **Exactly `Edit`, and nothing that merely contains it.** `RichEdit20W`, `RICHEDIT50W` and
/// the `Edit` of a browser's accessibility tree are different controls with different
/// behaviours, and `EM_GETPASSWORDCHAR` means nothing to them; a substring match would send the
/// message to a window that may answer anything at all. They are level 3's business.
pub fn class_is_edit(class: &str) -> bool {
    class.eq_ignore_ascii_case(EDIT_CLASS)
}

/// Asks `window` for its mask character — **level 2 of FR-72**, with the timeout FR-72 names.
///
/// `None` means the call did not answer inside [`PASSWORD_CHAR_TIMEOUT_MS`], or failed.
///
/// # ⚠ `SendMessageTimeout`, never `SendMessage`
///
/// FR-72 names `SendMessageTimeout` and the reason is not style. A plain `SendMessage` to a
/// window whose owning thread is not pumping messages blocks the **calling** thread for as long
/// as that lasts, which in the general case is for ever — and the calling thread here is the
/// watcher thread, which holds the three `WinEvent` subscriptions of section 6.1 and the COM
/// apartment. A hung text box in somebody else's program would silently stop two rows of the
/// FR-10 flush table and the whole of FR-80's desktop-switch recovery. There is no
/// `SendMessage` in this module, and a test asserts the absence for the whole of `src\`.
///
/// `SMTO_ABORTIFHUNG` on top of the timeout: it returns immediately when the system already
/// knows the target is not responding, instead of waiting out fifty milliseconds to learn what
/// it could have been told at once.
///
/// Public for the reason [`class_name`] is: it is the half of level 2 that needs a real window,
/// and `tests\guard.rs` supplies one of its own with and without `ES_PASSWORD`.
pub fn password_char(window: HWND) -> Option<usize> {
    let mut answer = 0usize;

    // SAFETY: `window` is the focus window `focused_control` obtained from the system. The
    // out-pointer addresses a live local of exactly the type the binding asks for, and the call
    // writes the target's reply through it only when it returns non-zero. `EM_GETPASSWORDCHAR`
    // takes no parameters, so both `wparam` and `lparam` are zero and neither is dereferenced by
    // anybody. The call is bounded by `PASSWORD_CHAR_TIMEOUT_MS` and by `SMTO_ABORTIFHUNG`, which
    // is the whole reason it and not `SendMessage` is here. NFR-13: the return is examined below.
    let result = unsafe {
        SendMessageTimeoutW(
            window,
            EM_GETPASSWORDCHAR,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            PASSWORD_CHAR_TIMEOUT_MS,
            Some(&raw mut answer),
        )
    };

    if result.0 == 0 {
        // NFR-13: zero is the documented failure — the timeout expired, or the target is hung,
        // or the window has gone. All three are "no answer", which FR-73 gives the meaning of
        // once the chain has run out of levels.
        LEVEL2_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        return None;
    }

    Some(answer)
}

/// What a mask character means — **the whole of level 2's verdict**, as a function of one number.
///
/// * `Some(Field::Password)` — «ненулевой результат → буферизация отключена», FR-72 word for
///   word. The control paints a mask instead of what is typed, which is what `ES_PASSWORD` does
///   and what nothing else does.
/// * `Some(Field::Ordinary)` — the control is a plain editor. **Not a verdict on its own**: see
///   [`determine`] for why the chain goes on to level 3 all the same.
/// * `None` — no answer arrived. FR-73's case, and the caller carries it on rather than
///   deciding here.
///
/// A `const fn` of its argument so that every one of the three outcomes is reachable from a test
/// without a window.
///
/// ⚠ **SEC-01, SEC-07.** The mask character is the character the control **paints**, not one the
/// user typed — a bullet or an asterisk, chosen by the control and identical for every password
/// in the world. It is narrowed to a state here and is stored nowhere.
pub const fn field_for_password_char(answer: Option<usize>) -> Option<Field> {
    match answer {
        None => None,
        Some(0) => Some(Field::Ordinary),
        Some(_) => Some(Field::Password),
    }
}

// ---------------------------------------------------------------------------------------
// Level 3 — UI Automation
// ---------------------------------------------------------------------------------------

thread_local! {
    /// The UI Automation client object of the watcher thread.
    ///
    /// A thread-local because a COM apartment is a property of a thread: the object is created
    /// inside the STA that section 6.1 gives the watcher thread and may be used from that thread
    /// and no other. Every other thread of this program leaves the slot empty for its whole
    /// life.
    ///
    /// Cached rather than created per probe because `CoCreateInstance` for UI Automation loads
    /// and initialises `UIAutomationCore.dll` the first time, and paying that on every focus
    /// change would put a measurable cost on `Alt+Tab` for nothing.
    ///
    /// ⚠ **Released explicitly**, by [`release_automation`], which
    /// [`crate::app::thread_body`] calls while the apartment is still entered. A thread-local
    /// holding a COM interface would otherwise be dropped by the thread-local destructor, which
    /// runs **after** the `CoUninitialize` of that apartment — releasing an interface into an
    /// apartment that no longer exists.
    static AUTOMATION: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
}

/// Whether the focused accessibility element is a password field — **level 3 of FR-72**.
///
/// `Some(true)` is `IsPasswordProperty`; `Some(false)` is an element that answered and is not
/// one; `None` is "no answer" — the client could not be created, there is no focused element, or
/// the provider did not reply inside [`UIA_TIMEOUT_MS`]. FR-73 fixes what `None` means.
///
/// # Why this is the level that can say "ordinary"
///
/// It is the last one. Levels 1 and 2 recognise two particular shapes of password field and stay
/// silent about everything else; this one asks the accessibility framework the actual question
/// FR-72 asks — `IsPasswordProperty` — of whatever has the focus, browsers, Electron and Qt
/// included. There is nothing after it to defer to, so its `false` is the verdict `Ordinary` and
/// its silence is the verdict `Undetermined`.
fn is_password_element() -> Option<bool> {
    AUTOMATION.with(|cell| {
        let mut slot = cell.borrow_mut();

        if slot.is_none() {
            match automation() {
                Ok(client) => *slot = Some(client),
                Err(error) => {
                    // NFR-13: examined and journalled once per failure. A client that will not be
                    // created is a level that cannot answer, which FR-73 has a rule for; it is
                    // not a reason to end the program, and the program without level 3 still has
                    // levels 1 and 2.
                    crate::app::report_non_critical("CoCreateInstance(CUIAutomation8)", &error);
                    return None;
                }
            }
        }

        let client = slot.as_ref()?;

        // SAFETY: `client` is the interface `CoCreateInstance` returned into this thread's slot,
        // and this runs on the thread that created it, inside the apartment it was created in —
        // which is the whole obligation an STA interface pointer carries. The call takes no
        // arguments of ours and returns a new reference the `Result` owns. NFR-13: the result is
        // examined — no focused element is an ordinary outcome on a desktop that is switching
        // windows, and a provider that ran out of `UIA_TIMEOUT_MS` reports a failing `HRESULT`
        // here, which is exactly the FR-73 case.
        let element = unsafe { client.GetFocusedElement() }.ok()?;

        // SAFETY: `element` is the interface the call above returned, used on the same thread and
        // in the same apartment. The property read takes no arguments of ours. NFR-13: examined
        // — a provider that does not implement the property, or that stopped answering, is a
        // failing `HRESULT` and therefore `None`, which FR-73 turns into buffering on.
        let is_password = unsafe { element.CurrentIsPassword() }.ok()?;

        Some(is_password.as_bool())
    })
}

/// Creates the UI Automation client of the calling thread, with the two bounds FR-73 needs.
///
/// `CUIAutomation8` and not `CUIAutomation`: the newer class is what supports `IUIAutomation2`,
/// and `IUIAutomation2` is what carries `SetConnectionTimeout` and `SetTransactionTimeout`. The
/// interface asked for is still plain [`IUIAutomation`] — everything level 3 does is on that
/// interface, and `IUIAutomation2` is queried for only to set the two budgets.
///
/// ⚠ **Without those two calls FR-73 would be unreachable in the case it was written for.** A
/// provider that never answers would leave `GetFocusedElement` inside a cross-process call with
/// no way out, the watcher thread stuck in it, and the state at [`Field::Pending`] — buffering
/// **off**, which is the opposite of what FR-73 requires of an undeterminable field.
fn automation() -> WinResult<IUIAutomation> {
    // SAFETY: `CUIAutomation8` is a `'static` GUID constant; `None` for the aggregate is the
    // documented way to ask for a non-aggregated object; `CLSCTX_INPROC_SERVER` names the only
    // form UI Automation is registered in. The generic parameter fixes the interface the call
    // returns, so nothing is transmuted and no pointer of ours is passed. NFR-13: propagated
    // with `?`, and the caller turns a failure into FR-73's answer.
    let client: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }?;

    // The two budgets are best-effort: an implementation that does not offer `IUIAutomation2`
    // still gives a usable client, and losing the bound is worse than losing the level but not
    // worth refusing the level over. A failure is journalled and the client is returned.
    match client.cast::<IUIAutomation2>() {
        Ok(bounded) => {
            // SAFETY: `bounded` is the same object under a wider interface, obtained by
            // `QueryInterface` and used on the thread and in the apartment it belongs to. Both
            // calls take one value each and dereference nothing of ours. NFR-13: both results are
            // examined below.
            let set = unsafe {
                bounded
                    .SetConnectionTimeout(UIA_TIMEOUT_MS)
                    .and_then(|()| bounded.SetTransactionTimeout(UIA_TIMEOUT_MS))
            };

            if let Err(error) = set {
                crate::app::report_non_critical("IUIAutomation2::SetTimeout", &error);
            }
        }
        Err(error) => crate::app::report_non_critical("QueryInterface(IUIAutomation2)", &error),
    }

    Ok(client)
}

/// Releases the UI Automation client of the calling thread.
///
/// Called by [`crate::app::thread_body`] on the watcher thread after its message loop has ended
/// and **before** the COM apartment is left. See [`AUTOMATION`] for why the thread-local
/// destructor is too late.
///
/// Idempotent, and a no-op on every thread that never created a client — which is every thread
/// but one.
pub fn release_automation() {
    AUTOMATION.with(|cell| {
        cell.replace(None);
    });
}

// ---------------------------------------------------------------------------------------
// Tests of what can be driven without a window, a focus or a keyboard
// ---------------------------------------------------------------------------------------

/// The decisions of this module are functions of their arguments wherever they could be made
/// ones, and this is what that buys: FR-70, FR-72's level-2 rule, FR-73's default and the
/// separation of the two "unknown" states are all checked here, deterministically, with no
/// password field anywhere near the test binary.
///
/// The Win32 halves — the three levels against a live window — are verified on the running
/// program and by the bench of section 11.5, which is the only honest place for them.
#[cfg(test)]
mod tests {
    use super::*;

    /// **FR-70 and FR-73 in one table.** The two states that switch buffering off and the two
    /// that leave it on, all four named.
    #[test]
    fn buffering_is_off_only_where_the_requirements_say_it_is() {
        assert!(!buffering_allowed_for(Field::Password), "FR-70");
        assert!(!buffering_allowed_for(Field::Pending), "FR-71's interval");
        assert!(buffering_allowed_for(Field::Ordinary));
        assert!(buffering_allowed_for(Field::Undetermined), "FR-73");
    }

    /// ⚠ **The two "unknown" states are not the same state.** Task specification point 3: the
    /// interval before an answer exists and FR-73's impossibility of determining are different
    /// things and answer in opposite directions.
    #[test]
    fn the_pending_interval_and_fr73_are_opposite_answers() {
        assert_ne!(Field::Pending, Field::Undetermined);
        assert_ne!(
            buffering_allowed_for(Field::Pending),
            buffering_allowed_for(Field::Undetermined),
            "an interval with no answer buffers nothing; a field that cannot be determined buffers"
        );
    }

    /// The state the program starts in is FR-73's, which is the only honest one: nothing has been
    /// determined before the first focus event.
    #[test]
    fn the_starting_state_is_the_fr73_one() {
        assert_eq!(Field::default(), Field::Undetermined);
        assert!(buffering_allowed_for(Field::default()));
    }

    /// Every code round-trips, and an unknown one falls back to FR-73 rather than to a state that
    /// would switch buffering off.
    #[test]
    fn an_unknown_code_falls_back_to_the_fr73_state() {
        for field in [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ] {
            assert_eq!(Field::from_code(field as u8), field);
        }

        for code in [4u8, 5, 128, 255] {
            assert_eq!(Field::from_code(code), Field::Undetermined);
            assert!(buffering_allowed_for(Field::from_code(code)));
        }
    }

    /// **Level 2 of FR-72, word for word:** «ненулевой результат → буферизация отключена».
    #[test]
    fn a_non_zero_mask_character_is_a_password_field() {
        // A bullet, an asterisk, and the largest value the reply can carry.
        for mask in [0x25CFusize, 0x2A, usize::MAX] {
            assert_eq!(
                field_for_password_char(Some(mask)),
                Some(Field::Password),
                "mask {mask:#x}"
            );
        }

        assert_eq!(field_for_password_char(Some(0)), Some(Field::Ordinary));

        // The timeout of FR-72 is not a verdict of level 2 at all — it is carried on, and the
        // chain decides. `None` here is what keeps FR-73 from being reached by accident.
        assert_eq!(field_for_password_char(None), None);
    }

    /// **Level 1 of FR-72:** the three names, case-insensitively, and nothing else.
    #[test]
    fn level_one_recognises_the_three_names_of_fr72_and_no_others() {
        for name in ["LogonUI.exe", "logonui.exe", "LOGONUI.EXE"] {
            assert!(is_credential_process_name(name), "{name}");
        }
        for name in ["CredentialUIBroker.exe", "credentialuibroker.EXE"] {
            assert!(is_credential_process_name(name), "{name}");
        }
        for name in ["consent.exe", "CONSENT.exe"] {
            assert!(is_credential_process_name(name), "{name}");
        }

        // The list is exact: a name that merely contains one of them is a different program.
        for name in [
            "notepad.exe",
            "consent",
            "myconsent.exe",
            "consent.exe.exe",
            "",
            "LangSwitcher.exe",
        ] {
            assert!(!is_credential_process_name(name), "{name}");
        }

        assert_eq!(CREDENTIAL_PROCESSES.len(), 3);
    }

    /// The comparison is on the name and not on the path — FR-72 says «имя процесса».
    #[test]
    fn the_name_is_taken_from_the_end_of_the_path() {
        assert_eq!(
            file_name_of(r"C:\Windows\System32\consent.exe"),
            "consent.exe"
        );
        assert_eq!(
            file_name_of(r"\Device\HarddiskVolume3\Windows\System32\LogonUI.exe"),
            "LogonUI.exe"
        );
        assert_eq!(file_name_of("consent.exe"), "consent.exe");
        assert_eq!(
            file_name_of("C:/mixed/separators/consent.exe"),
            "consent.exe"
        );
        assert_eq!(file_name_of(""), "");

        assert!(is_credential_process_name(&file_name_of(
            r"C:\Windows\System32\LogonUI.exe"
        )));
    }

    /// **Level 2 matches the class exactly.** `RichEdit` is not `Edit`, and sending
    /// `EM_GETPASSWORDCHAR` to one would be asking a question it does not answer.
    #[test]
    fn only_the_classic_edit_class_reaches_level_two() {
        for class in ["Edit", "EDIT", "edit"] {
            assert!(class_is_edit(class), "{class}");
        }

        for class in [
            "RichEdit20W",
            "RICHEDIT50W",
            "Edit2",
            "MyEdit",
            "Chrome_RenderWidgetHostHWND",
            "",
        ] {
            assert!(!class_is_edit(class), "{class}");
        }
    }

    /// The two private messages of this module are distinct from each other and from every other
    /// one in the program. Two private messages that collide do so silently.
    #[test]
    fn the_private_messages_are_distinct_from_every_other_one() {
        let all = [
            crate::hook::WM_APP_HOTKEY,
            crate::hook::WM_APP_FAIL_SAFE,
            crate::watchdog::WM_APP_FLUSH,
            crate::watchdog::WM_APP_LAYOUT,
            crate::watchdog::WM_APP_REHOOK,
            crate::switch::WM_APP_SWITCH,
            WM_APP_PROBE,
            WM_APP_FIELD,
        ];

        for (index, message) in all.iter().enumerate() {
            for other in &all[index + 1..] {
                assert_ne!(message, other, "two private messages share a number");
            }
        }
    }

    /// The timeout of level 2 is the number FR-72 writes and not one that drifted.
    #[test]
    fn the_level_two_timeout_is_the_fifty_milliseconds_of_fr72() {
        assert_eq!(PASSWORD_CHAR_TIMEOUT_MS, 50);
    }

    /// The words the channel would print are four distinct ones and carry nothing but a state.
    #[test]
    fn every_state_has_its_own_word() {
        let words: Vec<&str> = [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ]
        .iter()
        .map(|field| field.name())
        .collect();

        assert_eq!(words, ["pending", "ordinary", "password", "undetermined"]);
    }
}
