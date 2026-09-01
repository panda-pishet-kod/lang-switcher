//! Detecting password fields, the process exclusion list.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-70, FR-71, FR-72, FR-73, SEC-06 — task **T-06-1** — and
//! FR-84, the process exclusion list — task **T-06-3**, which reuses the publication built by
//! T-06-1 rather than raising a second one.
//! Implemented by backlog tasks: T-06-1 (done), T-06-3 (done).
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
//! | input thread, message loop | reads the published state once per message and switches the typing buffer on or off | two relaxed loads |
//! | watcher thread, message loop | the three levels of FR-72 and the name comparison of FR-84, UI Automation included | tens of milliseconds |
//!
//! **The callback does not even read the flags**, and that is stronger than FR-71 asks rather
//! than weaker. They are read by the input thread — which is what section 6.3 prescribes,
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
//!   EVENT_SYSTEM_FOREGROUND / EVENT_OBJECT_FOCUS      (system)
//!     └─ watchdog::win_event_proc          (watcher thread, unchanged)
//!          └─ WM_APP_FLUSH ──────────────► input thread
//!               └─ guard::note_focus_moved  state := Pending, buffering off, buffer wiped
//!                    └─ WM_APP_PROBE ────► watcher thread
//!                         └─ guard::run_pending_probe
//!                              one read of the process name, then
//!                              FR-84's comparison and the three levels of FR-72
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
//! one. That is the trade SEC-06 requires, and it is bounded — by 1550 milliseconds in the worst
//! case, see [`PROBE_BUDGET_MS`] for the four terms that make the number and for the two calls
//! that stand outside it.
//!
//! ⚠ **[`Field::Pending`] is not FR-73, and the two must not be confused.** FR-73 speaks of the
//! **impossibility** of determining the field — a timeout, an unknown control — and answers it
//! with buffering **on**. [`Field::Pending`] is the interval **before an answer exists at all**,
//! and it answers with buffering **off**. Opposite answers to different questions, and they are
//! separate arms of [`Field`] precisely so that neither can be reached by the other's path:
//! [`Field::Undetermined`] is stored only by [`determine`] returning it, and [`Field::Pending`]
//! only by [`enter_pending`], the input thread's half of [`note_focus_moved`]. [`publish`] is
//! never called with the interval state and refuses it outright if it ever is, which is what
//! holds the two apart in the code and not only in this paragraph.
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
//! # FR-84 — the process exclusion list, task T-06-3
//!
//! > Настраиваемый список имён процессов, в которых буферизация не ведётся (по умолчанию пуст).
//! > Назначение: игры с системами защиты от читерства и приложения, использующие Raw Input
//! > в обход стандартного ввода.
//!
//! **It is the same construction, deliberately, and not a second one.** The question "which
//! process owns the foreground window" is answered by
//! `GetForegroundWindow` → `GetWindowThreadProcessId` → `OpenProcess` →
//! `QueryFullProcessImageNameW`, and that chain is one to two orders of magnitude over the
//! hundred microseconds NFR-01 gives the callback — `OpenProcess` alone is a kernel object
//! creation that can be refused, and a refusal costs the same as a success. Level 1 of FR-72 pays
//! it already, on the watcher thread, once per focus change; FR-84 asks the same question of the
//! same string, so it is answered **in the same pass** and published into a flag beside the one
//! FR-70 publishes into. See [`EXCLUDED`] for the flag and [`determine`] for the single read.
//!
//! What the callback does with it is nothing, in the exact sense of the previous section: the
//! flag is read by the input thread, in [`crate::app::apply_buffering_gate`], and what the gate
//! does with it is take the typing buffer away. A stroke in an excluded process therefore meets
//! the same thread-local presence check it always met, takes the shorter arm of it, and is
//! **passed on to the application untouched** — [`crate::buffer::record`] neither records nor
//! suppresses when there is no buffer, and no return value of it reaches the decision of
//! [`crate::hook::classify`] at all.
//!
//! ⚠ **"Не ведётся" is "we do not remember", not "we do not let through".** FR-84 exists for
//! programs that read input in ways this one must stay out of; suppressing their keystrokes would
//! be the opposite of staying out of the way. Nothing on this path can return
//! [`crate::hook::Decision::Suppress`].
//!
//! ⚠ **FR-96 is unaffected, and that is a property of where it sits rather than a promise made
//! here.** The emergency combination is handled at the top of
//! [`crate::hook::keyboard_hook_proc`], before `classify` is called and before anything of this
//! module could be consulted even in principle — there is no flag of this module on that path to
//! read. See the check in `tests\guard.rs`.
//!
//! ## How the list gets from the file to the watcher thread — section 6.3
//!
//! Section 6.3 gives two mechanisms for publishing configuration: «через `arc_swap`-подобный
//! механизм на базе `Atomic` указателя либо через `PostMessage`». What is used here is the first
//! one with the pointer removed, which is what module [`crate::layouts`] already does for the
//! `cycle` list of section 7: a **fixed table of atomics** written by the UI thread and read by
//! the watcher thread, with no allocation and no lock on either side. See
//! [`publish_exclusions`].
//!
//! A pointer would buy an unbounded list at the price of a reclamation problem — a reader may be
//! inside the old list when the writer swaps, and a resident program under NFR-06's eight
//! megabytes cannot answer that by never freeing. A bounded table has no such question to answer:
//! [`MAX_EXCLUSIONS`] names and [`MAX_EXCLUSION_NAME_BYTES`] bytes each, four kilobytes of `.bss`,
//! and an entry that does not fit is **refused and counted** rather than truncated — a truncated
//! name is a name that matches a different program.
//!
//! ⚠ **One thing is added over a bare table of atomics: a generation counter**, and the difference
//! that asks for it is real. An entry here is a row of bytes read one at a time, and a reader that
//! raced the writer could assemble a name that was never in either list.
//! [`EXCLUSIONS_GENERATION`] is odd while the table is being written and is re-read after the
//! search; a reader that sees it move discards what it read and asks again. Requirement: no mutex
//! on the read path (NFR-04, section 6.3), and there is none — the reader takes no lock, waits for
//! nothing and can be preempted anywhere without holding anything up.
//!
//! ⚠ What used to stand here was that module [`crate::layouts`] needs no such counter because each
//! of its published entries is a single `usize`. **That stopped being true with task T-13-22**,
//! which found the same torn read in `[layouts]` — a hotkey press taking one half of a pair from
//! the configuration that was standing and the other from the one being written — and gave that
//! module a generation of its own. The two counters, and the stamp of module [`crate::diag`]'s
//! ring, are one mechanism written three times; the barriers of all three are the canon named in
//! [`EXCLUSIONS_GENERATION`].
//!
//! ## Comparison — the name, case-insensitively
//!
//! FR-84 says «имён процессов», so what is compared is the last component of the image path and
//! never the path: see [`file_name_of`], which level 1 of FR-72 uses for the same reason. Both
//! sides go through [`fold_process_name`], which takes the file name, trims it and lower-cases it
//! with `str::to_lowercase` — the configured names once, at publication, and the observed name
//! once per probe, both on threads that are allowed to allocate.
//!
//! ⚠ **`to_lowercase` and not `eq_ignore_ascii_case`, and this is the one place this module
//! departs from level 1's comparison.** Level 1 compares against three names that are ASCII by
//! construction. This list is written by a user, and `Игра.exe` in a configuration file has to
//! match `ИГРА.EXE` on the disk, because that is what «регистр в именах файлов Windows незначим»
//! means to the person writing the file. What the fold does **not** reproduce exactly is NTFS's
//! own `$UpCase` table, which is a per-volume snapshot of Unicode taken when the volume was
//! formatted; the residual disagreements are exotic (Cherokee, Deseret, the Turkish dotted and
//! dotless `I`, which `to_lowercase` folds locale-independently and Windows folds the same way
//! for file names) and none of them is reachable by an executable name a user would type.
//!
//! ## The default, and the direction an unknown answer takes
//!
//! Section 7 gives `processes = []`, and an empty list excludes nothing: [`is_excluded_name`]
//! searches an empty table and answers `false`, [`excluded`] stays `false` for the life of the
//! process, and every path of tasks T-06-1 and earlier is reached exactly as it was. **The
//! default is not to be changed.**
//!
//! A process whose name **cannot be read** — `OpenProcess` refused across an integrity level, no
//! foreground window at all — is likewise not excluded. That is the opposite direction from
//! FR-73's, and the asymmetry is the point: FR-73's unknown answer keeps a *feature* on, while
//! guessing "excluded" here would switch the whole program off silently for every window this
//! process may not open. The failure that costs the user something is the loud one.
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
//! process. The observed name of the foreground window is read into a local, folded, compared
//! against [`CREDENTIAL_PROCESSES`] and against the published list, and dropped when the probe
//! returns — the only names this module **stores** are the ones the user wrote into their own
//! `[exclusions] processes`, and even those never leave it. What leaves is two bits.
//! SEC-02 is met by [`crate::app::apply_buffering_gate`], which flushes through
//! [`crate::buffer::reset`] — the one function of module `buffer` that overwrites the ring with
//! zeroes — before the buffer is taken away.
//!
//! # NFR-13, NFR-14
//!
//! Every Win32 result here is examined, and a failure is an *answer* rather than an escalation:
//! a process whose name cannot be read is not a credential process as far as level 1 is
//! concerned, and a UI Automation call that fails is [`Field::Undetermined`], which FR-73 fixes
//! the meaning of. Every `unsafe` block carries a `// SAFETY:` comment.

use core::sync::atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering, fence};
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
/// window as a number rather than as a hope. **1550.**
///
/// Level 1 is bounded by `OpenProcess` and `QueryFullProcessImageNameW`, neither of which waits
/// on another process; level 2 by [`PASSWORD_CHAR_TIMEOUT_MS`]; level 3 by [`UIA_TIMEOUT_MS`],
/// **three times over**, and the three are counted one by one because they are three separate
/// budgets of `IUIAutomation2` (see [`automation`] and [`is_password_element`]):
///
/// | Term | Where it is spent | ms |
/// |---|---|---|
/// | [`PASSWORD_CHAR_TIMEOUT_MS`] | level 2, `SendMessageTimeout(EM_GETPASSWORDCHAR)` | 50 |
/// | `SetConnectionTimeout` | reaching the provider at all | 500 |
/// | `SetTransactionTimeout` | `GetFocusedElement` | 500 |
/// | `SetTransactionTimeout` | `CurrentIsPassword` | 500 |
///
/// ⚠ **Two transactions and not one**, which is what task **T-13-12** corrected: level 3 asks the
/// provider twice — once for the focused element and once for its `IsPasswordProperty` — and a
/// transaction timeout bounds **each** cross-process call rather than the level as a whole. The
/// constant said 1050 until that task and understated the window it documents by five hundred
/// milliseconds. The direction of the error was safe — [`Field::Pending`] is buffering **off**,
/// so a window that is wider than advertised risks nothing and only costs the user keystrokes the
/// buffer did not keep — but the number was arithmetically wrong, and this constant is quoted as
/// a guarantee (see [`crate::app::window_proc`]).
///
/// ⚠ **And it is a budget for the levels, not for the first call ever made.** The
/// `CoCreateInstance` in [`automation`] loads and initialises `UIAutomationCore.dll` on the first
/// probe of the process and is bounded by **nothing** — the two properties that bound the rest
/// belong to the object that call returns, so they cannot apply to the call that returns it.
/// Neither does anything bound a client whose `cast::<IUIAutomation2>()` was refused: that path is
/// best-effort by decision, journals the refusal and returns a client with no budgets at all. Both
/// are named here rather than hidden behind the sum, because a caller that reads this constant as
/// "the probe can never take longer than this" would be wrong on the first probe and on a system
/// without `IUIAutomation2`.
///
/// The sum is what a caller may assume of every probe after the first on a system that has
/// `IUIAutomation2`, and the measured figures are in the report of task T-06-1.
pub const PROBE_BUDGET_MS: u32 = PASSWORD_CHAR_TIMEOUT_MS + 3 * UIA_TIMEOUT_MS;

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

/// Whether the typing buffer may be kept for `field` in a process `excluded` says this much
/// about — **FR-70, FR-73 and FR-84 in one function**.
///
/// FR-84 is a veto and not a fourth kind of field: «в которых буферизация не ведётся» admits of
/// no state of the focused control that would put it back on, so the exclusion is `&&`-ed over
/// the field rule rather than folded into [`Field`]. Keeping them apart is also what keeps
/// [`password_field`] — the flag of SEC-04a — meaning what it says: an excluded process is not a
/// password field, it is a process this program does not record in.
///
/// A `const fn` of its arguments so that all eight combinations are reachable from a test without
/// a window, a focus, a process or a keyboard.
pub const fn buffering_allowed_in(field: Field, excluded: bool) -> bool {
    !excluded && buffering_allowed_for(field)
}

/// **Both published answers, taken in one load** — the field of FR-70 and the exclusion of FR-84.
///
/// ⚠ **One atomic and not two, and that is the whole of the concurrency argument of this task.**
/// The two facts are read together by [`buffering_allowed`] and the gate acts on the pair, so a
/// reader that saw one of them updated and the other not would act on a state that never existed.
/// The dangerous half is exact: buffering is switched **on** only for `excluded == false` and a
/// field of FR-73 or `Ordinary`, so a reader that got the new field beside the previous window's
/// `excluded == false` would keep recording inside a process FR-84 says nothing may be recorded
/// in. Two `Relaxed` atomics do not forbid that — `Relaxed` orders nothing between two locations —
/// and the answer is not to reach for `Acquire`/`Release`, which would make the pair consistent by
/// paying for an ordering nothing here needs. The answer is to have one location, and then there
/// is no pair to be inconsistent.
///
/// So [`FIELD`] carries the field code in its low bits and the exclusion in [`EXCLUDED_BIT`], and
/// this is the one place either is decoded. Section 6.3 is met to the letter — one flag, one
/// atomic, `Relaxed` — and the gate got **cheaper** rather than dearer for FR-84: one load, where
/// task T-06-1 already paid one.
///
/// ⚠ **Still one load after task T-13-12 widened the word**, and that is a requirement rather
/// than a nicety: FR-71 says «результат кэшируется в атомарный флаг, который хук читает **одной
/// операцией**». The word now carries the focus generation above [`STATE_BITS`] (see [`FIELD`]),
/// and the reader does not look at it: the truncating cast keeps exactly the byte [`pack`] wrote
/// and drops the rest, which is one instruction on a value already in a register and **not** a
/// second visit to the atomic. Every use of the generation — the packing and the unpacking alike
/// — is on the writers' side, in [`enter_pending`] and [`publish`].
fn state() -> (Field, bool) {
    unpack(FIELD.load(Ordering::Relaxed) as u8)
}

/// The two answers as the one byte [`FIELD`] holds — the inverse of [`unpack`].
///
/// A `const fn` of its arguments, like every other decision in this module that could be made
/// one, so that the packing is checked against the unpacking for all eight combinations without a
/// window, a focus or a process.
pub const fn pack(field: Field, excluded: bool) -> u8 {
    let bit = if excluded { EXCLUDED_BIT } else { 0 };

    field as u8 | bit
}

/// The two answers a byte of [`FIELD`] carries — the inverse of [`pack`].
///
/// Total, for the reason [`Field::from_code`] is: the value comes out of an atomic, and a total
/// function is one less thing that can panic on a path the window procedure runs. A code the mask
/// leaves that this build does not understand is FR-73's arm, which is also what
/// [`Field::from_code`] answers on its own.
pub const fn unpack(raw: u8) -> (Field, bool) {
    (Field::from_code(raw & FIELD_MASK), raw & EXCLUDED_BIT != 0)
}

/// The state as published — one relaxed load, which is what section 6.3 prescribes for this
/// flag and what FR-71 means by «одной операцией».
pub fn field() -> Field {
    state().0
}

/// Whether the foreground process is one of `[exclusions] processes` — **FR-84**, as published.
///
/// One relaxed load, out of the same word [`field`] comes from: see [`state`].
pub fn excluded() -> bool {
    state().1
}

/// Whether the typing buffer may be kept right now — the question
/// [`crate::app::apply_buffering_gate`] asks once per message of the input thread.
///
/// One relaxed load and one `match`; no allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05).
/// FR-84 added no load to this path at all — see [`state`].
pub fn buffering_allowed() -> bool {
    let (field, excluded) = state();

    buffering_allowed_in(field, excluded)
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
    /// Probes that found the foreground process in `[exclusions] processes` — **FR-84**.
    pub excluded_verdicts: u32,
    /// Names published into the table by the last [`publish_exclusions`] — **FR-84**, section 7.
    pub exclusions: u32,
    /// Names the last [`publish_exclusions`] refused: empty, over
    /// [`MAX_EXCLUSION_NAME_BYTES`], or past [`MAX_EXCLUSIONS`].
    pub exclusions_refused: u32,
    /// Searches that gave up because the table moved under every attempt — see
    /// [`EXCLUSIONS_GENERATION`]. Expected to stay at zero for the life of a process.
    pub exclusion_read_retries: u32,
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
        excluded_verdicts: EXCLUDED_VERDICTS.load(Ordering::Relaxed),
        exclusions: EXCLUSIONS_PUBLISHED.load(Ordering::Relaxed),
        exclusions_refused: EXCLUSIONS_REFUSED.load(Ordering::Relaxed),
        exclusion_read_retries: EXCLUSION_READ_RETRIES.load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// **The published state**: a [`Field`] code in [`FIELD_MASK`], FR-84's answer in
/// [`EXCLUDED_BIT`] and the focus generation above [`STATE_BITS`].
///
/// Section 6.3 names the ordering: «`AtomicBool` с упорядочением `Relaxed`». It is not an
/// `AtomicBool` because the requirement's two answers turned out to be four states — FR-71's
/// interval and FR-73's impossibility are not the same thing as "password" and "not password" —
/// and the ordering is the one section 6.3 gives: `Relaxed`, because there is no other datum
/// whose visibility has to be ordered against it. The reader acts on the value alone.
///
/// ⚠ **Task T-06-3 put FR-84's flag in the top bit of the state byte rather than beside it**, and
/// the reason is the sentence above turned around: the moment there are two published facts the
/// gate acts on **together**, "no other datum whose visibility has to be ordered against it" stops
/// being true of either of them taken alone. One word restores it. [`state`] carries the whole
/// argument; it is the only place this value is decoded, and [`publish`] and
/// [`enter_pending`] are the only two that write it.
///
/// # ⚠ Task **T-13-12**: the focus generation moved **into** this word, and why it had to
///
/// The generation used to be a second atomic of its own, read by [`run_pending_probe`] before the
/// levels ran and re-read after them, with the publication a plain store that followed the
/// re-read. Between that re-read and that store there was no atomic tie of any kind, and the
/// window between two instructions is a window a preempted thread can be held in for a whole
/// scheduling quantum. What fell into it is exactly what the generation was raised to prevent:
/// the input thread runs the whole of [`note_focus_moved`] — generation up, state to
/// [`Field::Pending`], a fresh probe asked for — and the verdict of the **previous** field is
/// then stored on top of the `Pending` of a field that may be a password box. `Ordinary` there is
/// buffering **on** while the caret sits in a password field, and it stands until the new probe
/// answers: tens of milliseconds, and up to [`PROBE_BUDGET_MS`] against a silent provider.
///
/// Two locations cannot be tied by an ordering — `Relaxed` or otherwise — so the two facts became
/// one location, which is the same answer task T-06-3 gave for the field and the exclusion. The
/// generation lives above the state byte, [`enter_pending`] raises it and publishes `Pending` in
/// **one** read-modify-write, and [`publish`] is a compare-and-swap that succeeds only while the
/// generation is still the one the probe was started under. A stale verdict now loses a CAS
/// instead of winning a race, and it is counted in [`STALE_VERDICTS`] exactly as a verdict
/// dropped by the old pre-check was.
///
/// **The reader paid nothing for this** — see [`state`]: one relaxed load and a truncating cast,
/// which is what FR-71's «одной операцией» asks for and what it already was.
///
/// Starts at [`Field::Undetermined`] in generation zero with the bit clear, which is FR-73's
/// answer and therefore buffering on, beside section 7's `processes = []` and therefore nothing
/// excluded: before the first focus event this program has determined nothing and has been told
/// nothing.
static FIELD: AtomicU32 = AtomicU32::new(Field::Undetermined as u32);

/// Bit of the state byte of [`FIELD`] that carries FR-84's answer: set while the foreground
/// process is one of `[exclusions] processes`.
///
/// The top bit **of the byte**, so that the field codes keep the values [`Field`] gives them and
/// `Field::from_code` keeps meaning what it meant — see [`state`] for why the two facts share a
/// word at all.
const EXCLUDED_BIT: u8 = 0b1000_0000;

/// The bits of the state byte of [`FIELD`] that carry the field code — everything
/// [`EXCLUDED_BIT`] does not.
const FIELD_MASK: u8 = !EXCLUDED_BIT;

/// Width of the state byte of [`FIELD`] — everything [`pack`] produces, and the shift the focus
/// generation sits above. Task **T-13-12**.
///
/// A whole byte and not seven bits, so that the reader's decode is the truncating cast of
/// [`state`] and needs no mask of its own.
const STATE_BITS: u32 = 8;

/// What one focus change adds to [`FIELD`] — one step of the generation, the state byte
/// untouched. Task **T-13-12**.
const GENERATION_STEP: u32 = 1 << STATE_BITS;

/// The largest generation the word can hold before it wraps — **the ABA question of task
/// T-13-12, as a number**.
///
/// Twenty-four bits, which is 16 777 215 focus changes. For a stale verdict to be mistaken for a
/// live one the generation would have to come the whole way round **inside a single probe**, and
/// a probe is bounded by [`PROBE_BUDGET_MS`]: sixteen million `EVENT_OBJECT_FOCUS` events, each
/// one a system callback plus a `PostMessageW` plus a full pass of the input thread's message
/// loop, would have to be delivered in at most one and a half seconds — ten million focus changes
/// per second on a thread that also runs the keyboard hook. The counter is not merely unlikely to
/// wrap in time; it cannot be driven that fast by the mechanism that drives it.
const GENERATION_MASK: u32 = u32::MAX >> STATE_BITS;

/// The generation a word of [`FIELD`] was published under — the writer's half of the decode.
///
/// Never called by the reader: see [`state`] for why that matters and for what the reader does
/// instead.
const fn generation_of(word: u32) -> u32 {
    word >> STATE_BITS
}

/// A word of [`FIELD`]: a generation over the two published answers [`pack`] folds into a byte.
///
/// A `const fn` of its arguments, like every other decision in this module that could be made
/// one, so that the layout is checked against [`generation_of`] and [`unpack`] without a window,
/// a focus or a process.
const fn word(generation: u32, field: Field, excluded: bool) -> u32 {
    ((generation & GENERATION_MASK) << STATE_BITS) | pack(field, excluded) as u32
}

/// The interval state is the zero code, which is what lets [`enter_pending`] reach it by clearing
/// [`FIELD_MASK`] and keeping everything else — [`EXCLUDED_BIT`] and the generation alike.
///
/// Checked here rather than trusted: the discriminant is written out in [`Field`], and a future
/// edit that renumbered the arms would turn that one line into a silent bug.
const _: () = assert!(Field::Pending as u8 == 0);

/// The state byte is exactly the byte [`pack`] fills, so the truncating cast of [`state`] is a
/// complete decode and the generation can never reach the reader's `match`. Task **T-13-12**.
const _: () = assert!(EXCLUDED_BIT as u32 | FIELD_MASK as u32 == GENERATION_STEP - 1);

/// Whether a probe has been asked for and not yet run — **nought is "nobody is waiting"**, and
/// every other value is "yes".
///
/// SEC-05: this is why [`WM_APP_PROBE`] carries nothing. A forged message finds this nought and
/// the handler returns without touching UI Automation.
///
/// It also coalesces: two focus changes that arrive before the watcher thread has drained its
/// queue are one probe, and the probe reads the focus that is current when it runs rather than
/// the one that caused it. [`run_pending_probe`] therefore takes the whole counter to nought in
/// one swap rather than subtracting one — the question it asks is "did anybody want a probe",
/// never "how many".
///
/// # ⚠ Why this is a counter and not the boolean it was until task Т-22-2
///
/// A boolean cannot tell one writer's `true` from another's, and [`request_probe`] has to undo
/// **its own** request when the post fails without touching a request somebody else made in the
/// meantime. Finding м2 of the audit of 2026-09-01 (finding №13 of the audit of 2026-08-31) is
/// exactly that: the input thread raised the flag, the post failed, the UI thread raised it again
/// and got its post accepted, and the input thread then wrote back the `false` it had read before
/// either of them started — cancelling a probe that had been accepted, and leaving buffering off
/// until the next focus change.
///
/// A ticket makes the two distinguishable: the undo is a compare-and-exchange from the value this
/// caller's own increment produced, so it can only ever take back its own step and never
/// somebody else's.
static PROBE_PENDING: AtomicU32 = AtomicU32::new(0);

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

/// Probes that found the foreground process excluded — FR-84.
static EXCLUDED_VERDICTS: AtomicU32 = AtomicU32::new(0);

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
/// Two relaxed atomics — one fetch-add and one compare-and-swap — and one `PostMessageW`, which
/// queues and returns. No allocation, no lock, no I/O. This runs on the input thread, in its
/// message loop, with the hook callback long returned.
pub fn note_focus_moved() {
    FOCUS_CHANGES.fetch_add(1, Ordering::Relaxed);

    // Order matters: the state is published **before** the probe is asked for, so a watcher
    // thread that answers instantly cannot have its verdict overwritten by this transition.
    let generation = enter_pending();

    if !request_probe() {
        // The watcher thread has no window — it has not created one yet, or it is already gone.
        // There is nobody to answer, and leaving the state at `Pending` would mean a program
        // that never buffers again. FR-73 covers exactly this: the field cannot be determined,
        // so buffering is on.
        //
        // FR-84 answers `false` on the same path and for the same kind of reason: with no thread
        // to compare the name on, this program has not been shown an excluded process. Guessing
        // `true` would leave a program that never records again — see the module documentation on
        // which way an unknown answer goes for each of the two requirements, and why they differ.
        //
        // The generation is the one this call has just opened, so the publication below is the
        // one publication that cannot be stale: nothing has run between the two lines but a
        // `PostMessageW` that failed.
        publish(
            Probe {
                field: Field::Undetermined,
                excluded: false,
            },
            generation,
        );
    }
}

/// Opens a new focus generation and publishes [`Field::Pending`] into it — **the whole of what
/// the input thread does to [`FIELD`]**, task **T-13-12**. Answers the generation it opened.
///
/// # Why one read-modify-write and not two
///
/// The two things that happen here — the generation goes up, the state goes to `Pending` — are
/// what a publication of a verdict has to be refused *between*. Splitting them into two atomic
/// operations would put a word on display that says "a new focus, the previous field's verdict"
/// and would let this thread's second half undo a verdict the watcher thread published, correctly,
/// under the new generation. One compare-and-swap has no between.
///
/// ⚠ **A compare-and-swap loop and not a `fetch_*`**, and the reason is arithmetic: no single
/// fetch primitive both adds to the generation and clears [`FIELD_MASK`]. It is not a lock and
/// NFR-04 is untouched — nothing is held, nothing waits, and the loop can only turn while the one
/// other writer in the program ([`publish`], on the watcher thread, once per probe) lands a store
/// in between. This runs on the input thread's message loop and never in the hook callback, which
/// names no item of this module at all (see `tests\guard.rs`).
///
/// # What is kept and what is cleared
///
/// `Field::Pending` is the zero code, so "publish `Pending`" is "clear [`FIELD_MASK`]" — the
/// generation above it and [`EXCLUDED_BIT`] below it are both carried over.
///
/// The exclusion bit is **carried over** rather than cleared, and that is the honest value:
/// FR-84's answer for the window now arriving is not known yet either, and until the probe says
/// otherwise the last thing this program was told is the best it has. Nothing rests on the choice
/// — `Pending` switches buffering off through [`buffering_allowed_in`] whatever the bit says — so
/// what decides it is that clearing it would be a claim, and this is the one moment when the
/// program is entitled to none.
fn enter_pending() -> u32 {
    let mut current = FIELD.load(Ordering::Relaxed);

    loop {
        // `wrapping_add`: the generation is a tag and not a quantity, and the wrap is the case
        // `GENERATION_MASK` argues about rather than a case to avoid. The state byte takes no
        // carry from it — the step is one whole byte over — and the mask then clears the field
        // code alone.
        let next = current.wrapping_add(GENERATION_STEP) & !(FIELD_MASK as u32);

        match FIELD.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return generation_of(next),
            Err(seen) => current = seen,
        }
    }
}

/// Asks the watcher thread to run a probe. Answers whether the request reached a window.
///
/// The two callers ask for the same thing for different reasons — [`note_focus_moved`] because the
/// focus moved, [`publish_exclusions`] because the list the last probe compared against has been
/// replaced — and both need the same care on failure, which is why the request is one function and
/// not two copies of four lines.
///
/// ⚠ **A failed request takes back its own step and nothing else** — task **Т-22-2**. The two
/// callers run on different threads (section 6.1 puts the configuration on the UI thread and the
/// focus on the input thread), so one of them finding no watcher window must not cancel a probe
/// the other has already asked for and had accepted. SEC-05 is why the step is taken back at all
/// rather than simply left standing: [`WM_APP_PROBE`] carries nothing, and a request left waiting
/// for nobody is a request a forged message could spend.
///
/// # The protocol, and the interleaving it is written against
///
/// Until task Т-22-2 the two lines were `swap(true)` and, on failure, `store(previous)` — and the
/// comment above them promised precisely the property they did not have. `previous` is read
/// **before** the post is attempted, so a request accepted by the other thread in between was
/// overwritten by a value that predated it:
///
/// ```text
///   input thread                     UI thread
///   ------------                     ---------
///   swap(true) -> previous = false
///                                    swap(true) -> previous = true
///                                    post accepted, returns true
///   post refused
///   store(false)                     <- the accepted probe is gone
/// ```
///
/// The counter makes the two steps distinguishable. `fetch_add` answers with the value this
/// caller's own increment produced, and the undo is a `compare_exchange` from exactly that value:
/// if anybody — the other caller, or [`run_pending_probe`] draining the counter — has moved it
/// since, the exchange fails and the request stays where the other thread put it. In the trace
/// above the input thread would now find `2` where it expects `1`, decline to undo, and leave the
/// UI thread's accepted probe alone.
///
/// Declining to undo leaves this caller's own step standing beside the accepted one, and that is
/// deliberate rather than tolerated: the only reader of the counter is [`run_pending_probe`],
/// which asks "did anybody want a probe" and takes the whole counter to nought in one swap, so
/// one and two are the same answer to the only question anybody asks. Subtracting unconditionally
/// instead would be the old defect wearing a different arithmetic — a drain that happened in
/// between would be turned into a wrap to `u32::MAX`, and the program would believe a probe was
/// wanted for ever.
///
/// The counter can only run away if the post fails four billion times in a row while every single
/// undo loses its exchange, and even then the wrap lands on nought — "no probe wanted", the same
/// safe state a refused request ends in today.
fn request_probe() -> bool {
    let ticket = PROBE_PENDING
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);

    if crate::app::post_to_watcher_thread(WM_APP_PROBE) {
        return true;
    }

    // SEC-04a, feature `testing`, absent from the Release configuration: the one interleaving
    // this protocol is about, raised on purpose here instead of waited for. See [`stage`].
    #[cfg(feature = "testing")]
    stage::interleave();

    // An `Err` is not a failure and is deliberately not examined further (NFR-13 asks that a
    // return be *used*, and this one is — as the condition of the whole undo). It says that the
    // counter no longer holds this caller's own step, which is the one case in which the step
    // must be left exactly where it is.
    let _ = PROBE_PENDING.compare_exchange(
        ticket,
        ticket.wrapping_sub(1),
        Ordering::Relaxed,
        Ordering::Relaxed,
    );

    false
}

/// **SEC-04a, feature `testing`, absent from the Release configuration.** What `tests\guard.rs`
/// needs to raise the interleaving of task Т-22-2 on purpose, instead of running two threads and
/// hoping to lose a race that is two instructions wide.
///
/// The same shape, and the same reason, as `hook::fault`: a staging point compiled out of every
/// build that does not ask for the feature, and acceptance criterion 8 of section 13 checks that
/// the shipped binary carries no trace of it.
#[cfg(feature = "testing")]
pub mod stage {
    use super::{Ordering, PROBE_PENDING};
    use core::sync::atomic::AtomicBool;

    /// Whether the next refused request must find another one accepted underneath it.
    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Arms one interleaving: the next post that `request_probe` finds refused will see a request
    /// accepted by "the other thread" between the refusal and its own undo.
    pub fn arm_one_accepted_request_under_the_next_refusal() {
        ARMED.store(true, Ordering::Relaxed);
    }

    /// What the counter holds right now. Nought is "nobody is waiting".
    pub fn probe_requests() -> u32 {
        PROBE_PENDING.load(Ordering::Relaxed)
    }

    /// Back to "nobody is waiting", so that one test cannot bias the next.
    pub fn clear_probe_requests() {
        PROBE_PENDING.store(0, Ordering::Relaxed);
        ARMED.store(false, Ordering::Relaxed);
    }

    /// Called by [`super::request_probe`] between a refused post and the undoing of its own step —
    /// the one point where the interleaving matters. Fires at most once per arming.
    pub(super) fn interleave() {
        if ARMED.swap(false, Ordering::Relaxed) {
            // Exactly what a `request_probe` that got its post accepted leaves behind.
            PROBE_PENDING.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------------------
// The watcher thread's half — the three levels of FR-72
// ---------------------------------------------------------------------------------------

/// What one probe found — **the two answers of one pass over the foreground window**.
///
/// Together, and not one after the other, because they are read out of the same string: FR-84
/// asks whether the name of the foreground process is in a list and level 1 of FR-72 asks whether
/// it is in a different list, and reading it twice would pay `OpenProcess` twice for one focus
/// change.
///
/// **SEC-01, SEC-07.** One of four named states and one boolean. The name they were derived from
/// does not survive the function that read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Probe {
    /// What kind of field has the focus — FR-70 to FR-73.
    field: Field,
    /// Whether the process that owns it is excluded — FR-84.
    excluded: bool,
}

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
    // The whole counter to nought in one swap, however many requests it holds: the question is
    // "did anybody want a probe", never "how many", and the probe reads the focus that is current
    // when it runs rather than the one that caused it. That is the coalescing [`PROBE_PENDING`]
    // describes, unchanged by task Т-22-2 — only the type under it changed.
    if PROBE_PENDING.swap(0, Ordering::Relaxed) == 0 {
        return false;
    }

    PROBES.fetch_add(1, Ordering::Relaxed);

    // Captured before the levels run and **carried into the publication**, which refuses it
    // unless the generation is still this one — see `FIELD` and `publish`. Task **T-13-12**: the
    // capture used to be paired with a re-read here and a plain store inside `publish`, and the
    // two instructions between them were the window a stale `Ordinary` came through.
    let generation = generation_of(FIELD.load(Ordering::Relaxed));

    let verdict = determine();

    // The answer is `false` when the focus moved while this ran: the verdict describes a field
    // the user has left, the probe that belongs to the new one is already queued, and the state
    // stays at `Pending`, which is buffering off — the safe direction while nothing is known
    // (SEC-06). It is dropped rather than acted on, and `STALE_VERDICTS` counts it.
    publish(verdict, generation);

    true
}

/// Publishes a verdict **into the generation it was determined under**, and tells the input thread
/// to act on it. Answers whether it was published at all.
///
/// # ⚠ The compare-and-swap is the fix of task T-13-12, and it replaces a check
///
/// The caller used to compare the generation and then store, and between the comparison and the
/// store the input thread could run the whole of [`note_focus_moved`] — see [`FIELD`] for the
/// consequence. Here the comparison **is** the store: the word carries the generation, and a
/// verdict determined under a generation the focus has since left cannot win the swap. There is
/// no instant at which a stale `Ordinary` is on display, not even one this thread would undo a
/// moment later; the safe state stands untouched, which is what SEC-06 asks of an uncertainty.
///
/// The predicate is the generation and **only** the generation, deliberately. It is not "the state
/// is still `Pending`": [`publish_exclusions`] asks for a probe without a focus change, so a
/// second verdict for a generation that already has one is an ordinary event and must land.
///
/// # ABA
///
/// A stale verdict would be taken for a live one only if the generation came the whole way round
/// between the capture in [`run_pending_probe`] and the swap here. That is [`GENERATION_MASK`]
/// focus changes inside one probe, and the note there works out why the mechanism that raises the
/// generation cannot be driven at that rate.
fn publish(verdict: Probe, generation: u32) -> bool {
    if matches!(verdict.field, Field::Pending) {
        // `publish` is never called with the interval state: `enter_pending` stores that one
        // directly, which is what keeps the two apart at the level of the code and not only of
        // the comments.
        return false;
    }

    let mut current = FIELD.load(Ordering::Relaxed);

    loop {
        if generation_of(current) != generation {
            // The focus moved after this verdict was determined. Dropping it leaves whatever the
            // newer generation published — `Pending` until its own probe answers.
            STALE_VERDICTS.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // **Both answers in one store**, which is what makes them impossible to observe apart —
        // see `state` and `FIELD`. There is no order to get right here, because there is no
        // second store: the swap writes the generation, the field and the exclusion as one word.
        let next = word(generation, verdict.field, verdict.excluded);

        match FIELD.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(seen) => current = seen,
        }
    }

    // Counted **after** the swap, so that the counters describe what was published rather than
    // what was attempted, and so that `STALE_VERDICTS` and the verdict counters partition the
    // probes instead of overlapping.
    match verdict.field {
        Field::Password => PASSWORD_VERDICTS.fetch_add(1, Ordering::Relaxed),
        Field::Ordinary => ORDINARY_VERDICTS.fetch_add(1, Ordering::Relaxed),
        Field::Undetermined => UNDETERMINED_VERDICTS.fetch_add(1, Ordering::Relaxed),
        // Unreachable — refused at the top of this function — and written out rather than swept
        // into a wildcard, so that a fifth state could not be added without this arm objecting.
        Field::Pending => 0,
    };

    if verdict.excluded {
        EXCLUDED_VERDICTS.fetch_add(1, Ordering::Relaxed);
    }

    // The buffer is a thread-local of the input thread (section 6.3), so the thread that owns it
    // has to be the one that switches it. One `PostMessageW`, which queues and returns.
    crate::app::post_to_input_thread(WM_APP_FIELD);

    true
}

/// **FR-84's comparison and the three levels of FR-72, over one read of the process name.**
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
/// # ⚠ FR-84 comes before all three, and ends the probe when it answers
///
/// The exclusion is decided from the same string level 1 needs, so it costs nothing beyond a
/// search of a table that is empty by default. When it answers **yes**, the probe stops there and
/// the two remaining levels are not run at all, and that is a decision rather than an
/// optimisation:
///
/// * they exist to decide whether to keep a typing buffer, and FR-84 has already decided it —
///   «в которых буферизация не ведётся» admits of no control that would put it back on;
/// * level 2 sends a window message into that process and level 3 opens a cross-process COM
///   channel to it, and the programs FR-84 is written for are «игры с системами защиты от
///   читерства» — a program asked to stay out of the way should not be probing the accessibility
///   tree of an anti-cheat every time the game takes the foreground;
/// * and it is the only way FR-84 also buys back what it is supposed to buy: tens of milliseconds
///   of UI Automation per focus change, in exactly the program the user excluded (NFR-10).
///
/// With the default list of section 7 nothing is ever excluded, this branch is never taken, and
/// every path below it is reached exactly as task T-06-1 left it.
///
/// # Threading
///
/// Watcher thread only. Level 3 needs the COM single-threaded apartment section 6.1 puts there,
/// and levels 1 and 2 need to be nowhere near the hook (FR-71).
fn determine() -> Probe {
    let Some(window) = foreground_window() else {
        // No foreground window at all — which happens while the desktop switches and on the
        // secure desktop. Nothing can be determined, and FR-73 says what that means; nothing is
        // excluded either, because there is no process here to be in a list.
        return Probe {
            field: Field::Undetermined,
            excluded: false,
        };
    };

    // **The one read of the process name.** `OpenProcess` and `QueryFullProcessImageNameW` are
    // paid here, once, on the watcher thread, and both questions below are answered from the
    // result. `None` is a name that could not be read — a process of another user or a higher
    // integrity level — and it is not a failure: it is "not recognised" for level 1 and "not
    // excluded" for FR-84, which is what each of them does with a name it does not know.
    let process = process_name_of(window);

    // ---- FR-84: the exclusion list ---------------------------------------------------
    if process.as_deref().is_some_and(is_excluded_name) {
        return Probe {
            field: Field::Undetermined,
            excluded: true,
        };
    }

    // ---- Level 1: the process behind the active window -------------------------------
    if process.as_deref().is_some_and(is_credential_process_name) {
        LEVEL1_VERDICTS.fetch_add(1, Ordering::Relaxed);
        return Probe {
            field: Field::Password,
            excluded: false,
        };
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
        return Probe {
            field: Field::Password,
            excluded: false,
        };
    }

    // ---- Level 3: UI Automation ------------------------------------------------------
    let field = match is_password_element() {
        Some(true) => Field::Password,
        Some(false) => Field::Ordinary,
        None => {
            LEVEL3_FAILURES.fetch_add(1, Ordering::Relaxed);
            Field::Undetermined
        }
    };

    Probe {
        field,
        excluded: false,
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

/// The bare executable name of the process behind `window`, or `None` when it cannot be read.
///
/// **The single read both requirements are answered from** — level 1 of FR-72 and the list of
/// FR-84. Task T-06-1 read it for level 1 alone; task T-06-3 lifted the read out of that test so
/// that adding a second question added no second `OpenProcess`. See [`determine`].
///
/// `None` is not an error and is never escalated into one: a window that died between the two
/// calls, a process this one may not open, a name Windows will not report. Each caller decides
/// what an unknown name means to it, and the two of them decide differently on purpose — see the
/// module documentation.
///
/// SEC-01, SEC-07: the name is returned to the one caller, compared and dropped. Nothing here
/// reaches a journal or a panic message, and no name of any observed process is stored anywhere
/// in this module.
fn process_name_of(window: HWND) -> Option<String> {
    let mut pid = 0u32;

    // SAFETY: `window` is the live foreground window the caller obtained and checked. The second
    // argument is an optional out-pointer to a live local of exactly the expected type; the call
    // writes one `u32` through it and dereferences nothing else. NFR-13: a zero return means the
    // window died between the two calls and is examined below.
    let thread = unsafe { GetWindowThreadProcessId(window, Some(&raw mut pid)) };

    if thread == 0 || pid == 0 {
        return None;
    }

    process_file_name(pid)
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
///
/// ⚠ **Public so that a test can drive the live read**, the same reason [`class_name`] and
/// [`password_char`] are, and a sharper one: **both** things this module decides from a process
/// name stand on these two Win32 calls — level 1 of FR-72 and the whole of FR-84 — and neither of
/// them can be driven from a test any other way. Level 1 needs a credential process, which
/// limitation 1 of section 10 puts on the secure desktop, and until task T-06-3 a `None` from here
/// was **invisible**: it merely let the chain fall through to levels 2 and 3, which is also what a
/// correct read of an ordinary process does. A test process asking for its own name is the one
/// place the answer can be checked against something known.
pub fn process_file_name(pid: u32) -> Option<String> {
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
// FR-84 — the process exclusion list (task T-06-3)
// ---------------------------------------------------------------------------------------

/// Names `[exclusions] processes` may publish before the rest are refused.
///
/// A bound is what lets the list live in a table of atomics instead of behind a pointer, and
/// therefore what removes the reclamation problem a published pointer would bring with it — see
/// the module documentation. Thirty-two is far past what the requirement is for: FR-84 names
/// «игры с системами защиты от читерства и приложения, использующие Raw Input», of which a machine
/// has a handful, and the default of section 7 is none at all.
///
/// `pub` so that a test can drive the refusal rather than take this number on trust.
pub const MAX_EXCLUSIONS: usize = 32;

/// Bytes one published name may occupy, in the UTF-8 of [`fold_process_name`].
///
/// A hundred and twenty-eight, against a `MAX_PATH` of two hundred and sixty for a whole *path*:
/// what is stored here is the last component of one, and an executable whose bare name does not
/// fit in this does not exist outside a test.
///
/// ⚠ **A name that does not fit is refused, never truncated.** A truncated name is a name that
/// matches a *different* program, and matching the wrong program here means silently not
/// recording in it.
pub const MAX_EXCLUSION_NAME_BYTES: usize = 128;

/// How many times [`is_excluded_name`] re-reads a table that moved under it.
///
/// Four, and the number is not load-bearing: a publication is a few dozen stores by the UI thread
/// and the only reader runs once per focus change, so a single collision is already a coincidence
/// and four in a row is not reachable in practice. What matters is that the loop is **bounded** —
/// this runs on the watcher thread, which holds the `WinEvent` subscriptions of section 6.1, and a
/// thread that spins there stops delivering focus events to the whole program.
const EXCLUSION_READ_ATTEMPTS: u32 = 4;

/// The published `[exclusions] processes`, as a table of atomics — **section 6.3**.
///
/// One row per name, [`MAX_EXCLUSION_NAME_BYTES`] bytes each, holding the UTF-8 of
/// [`fold_process_name`]. Written by the UI thread in [`publish_exclusions`] and read by the
/// watcher thread in [`is_excluded_name`]; no other thread touches it, and the input thread — the
/// one NFR-04 is about — never reads it at all.
static EXCLUSION_NAMES: [[AtomicU8; MAX_EXCLUSION_NAME_BYTES]; MAX_EXCLUSIONS] =
    [const { [const { AtomicU8::new(0) }; MAX_EXCLUSION_NAME_BYTES] }; MAX_EXCLUSIONS];

/// Bytes in use in each row of [`EXCLUSION_NAMES`].
static EXCLUSION_LENS: [AtomicUsize; MAX_EXCLUSIONS] =
    [const { AtomicUsize::new(0) }; MAX_EXCLUSIONS];

/// Rows of [`EXCLUSION_NAMES`] that hold a name. **Zero is the default of section 7.**
static EXCLUSION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// **Odd while [`publish_exclusions`] is writing the table, even when it is readable.**
///
/// Every cell of the table is an atomic, so no choice of orderings here can be a data race or
/// undefined behaviour; what the counter guards against is narrower and real — a search that ran
/// across a publication and assembled its answer out of **two different lists**, matching a name
/// that was in neither. [`is_excluded_name`] reads this before and after its search and throws the
/// answer away if it moved.
///
/// This program holds three of these and they are one mechanism: the stamp of module
/// [`crate::diag`]'s ring, this counter, and the [`crate::layouts`] generation task T-13-22 added
/// for the same reason. The sentence that used to stand here — that the module next door does not
/// need one — was made false by that task and is gone rather than qualified.
///
/// # The barriers are Boehm's, not the ones that read naturally
///
/// Hans-J. Boehm, *"Can seqlocks get along with programming language memory models?"*, MSPC 2012.
/// The canon that paper settles, and the form every seqlock in this program is written in — the
/// same words stand over [`crate::layouts::published`] and over `diag`'s `Slot`:
///
/// * the **reader** loads the counter, reads the rows with `Relaxed` loads, then executes
///   `fence(Acquire)` **before** the control load of the counter, which may itself be `Relaxed`.
///   Putting `Acquire` *on* the control load instead is the trap the paper is about: acquire on a
///   load orders the operations that come **after** it, and what has to be pinned here is the row
///   loads that came **before** — nothing stops them from being sunk past an acquire load. That
///   trap is exactly what stood here until task T-13-16, and this comment asserted the opposite in
///   so many words; the audit of 2026-08-24 quoted the assertion back and called it wrong. On
///   x86_64 (TSO) nothing was observable, and on the ARM64 build section 3 admits, a load-load
///   reordering is not a thought experiment;
/// * the **writer** must not let a row store be hoisted above the odd value that announces the
///   write. Module `layouts` gets that from a `fence(Release)` placed after a `Relaxed` bump; here
///   it comes from the bump itself, `fetch_add` with `AcqRel`, whose acquire half is precisely the
///   promise that nothing after it moves before it — and the closing `AcqRel` bump is what carries
///   the rows to a reader that sees the even value. Two spellings of one rule. The audit examined
///   this side and did not fault it, so task T-13-16 left it spelled as it was rather than churn a
///   correct writer for symmetry.
static EXCLUSIONS_GENERATION: AtomicU32 = AtomicU32::new(0);

/// Names the last [`publish_exclusions`] accepted — see [`Counters`].
static EXCLUSIONS_PUBLISHED: AtomicU32 = AtomicU32::new(0);

/// Names the last [`publish_exclusions`] refused.
static EXCLUSIONS_REFUSED: AtomicU32 = AtomicU32::new(0);

/// Searches abandoned because the table moved under them.
static EXCLUSION_READ_RETRIES: AtomicU32 = AtomicU32::new(0);

/// The form a process name is compared in — **the whole of FR-84's comparison rule**.
///
/// Three steps, and each is a sentence of the requirement or of the task specification:
///
/// 1. `trim`, because `processes = [" game.exe "]` is a person writing a list in a text file;
/// 2. [`file_name_of`], because FR-84 says «имён процессов» and not of paths — a user who wrote
///    `C:\Games\game.exe` meant the same program the system reports as `game.exe`, and level 1 of
///    FR-72 already reads the observed side the same way and for the same reason;
/// 3. `to_lowercase`, because file names in Windows are case-insensitive.
///
/// ⚠ **Unicode lowercasing and not `eq_ignore_ascii_case`.** The three names of level 1 are ASCII
/// by construction and are compared with the ASCII fold; this list is written by a user and
/// `Игра.exe` has to match `ИГРА.EXE`, which an ASCII fold leaves apart. `str::to_lowercase`
/// applies the Unicode default case conversion, is locale-independent — the Turkish dotted and
/// dotless `I` included, which is the case a locale-aware fold would get *differently* from the
/// way Windows treats file names — and allocates, which is why this is called on the UI thread
/// once per publication and on the watcher thread once per focus change, and nowhere else.
///
/// What it does not reproduce exactly is NTFS's own `$UpCase` table, a per-volume snapshot of
/// Unicode taken when the volume was formatted. The residual disagreements are confined to
/// characters no executable name a user would type contains.
///
/// Public so that both sides of the comparison and the whole rule can be driven from a test
/// without a process.
pub fn fold_process_name(name: &str) -> String {
    file_name_of(name.trim()).to_lowercase()
}

/// **Whether `name` is in the published `[exclusions] processes` — FR-84.**
///
/// `name` is the observed executable name, in whatever spelling Windows reported it;
/// [`fold_process_name`] puts it into the form the table holds.
///
/// Watcher thread, once per focus change, from [`determine`]. **Not on the hook path and not in a
/// `WinEvent` callback**: the string it is given cost an `OpenProcess`, and where that is paid is
/// the whole subject of this module.
///
/// # NFR-04
///
/// No lock, no mutex, no wait. A table of atomics, a bounded retry, one `fence` and an answer —
/// and a fence is an ordering instruction rather than a synchronisation primitive: it takes
/// nothing, waits for nobody and cannot block. On x86_64 it emits no instruction at all.
///
/// # An empty list
///
/// [`EXCLUSION_COUNT`] is zero, the search examines nothing and answers `false` — which is section
/// 7's default reached by doing nothing, rather than by a special case somebody has to remember to
/// keep correct.
pub fn is_excluded_name(name: &str) -> bool {
    let folded = fold_process_name(name);
    let wanted = folded.as_bytes();

    if wanted.is_empty() || wanted.len() > MAX_EXCLUSION_NAME_BYTES {
        // Neither could have been published: `publish_exclusions` refuses both. Answering here
        // saves the search and, more to the point, says so.
        return false;
    }

    for _ in 0..EXCLUSION_READ_ATTEMPTS {
        let before = EXCLUSIONS_GENERATION.load(Ordering::Acquire);

        if before.is_multiple_of(2) {
            let found = search_published(wanted);

            // Boehm's fence, and it stands **before** the control load rather than inside it — see
            // `EXCLUSIONS_GENERATION`. This is what keeps the row loads of the search above from
            // being sunk below the check that is supposed to vouch for them.
            fence(Ordering::Acquire);

            // Unmoved: what was searched was one list, and the answer stands.
            if EXCLUSIONS_GENERATION.load(Ordering::Relaxed) == before {
                return found;
            }
        }

        EXCLUSION_READ_RETRIES.fetch_add(1, Ordering::Relaxed);
    }

    // A publication in flight through every attempt — see `EXCLUSION_READ_ATTEMPTS` for why this
    // is not reachable in practice. The answer is the default of section 7, which is the direction
    // an unknown answer takes for FR-84: not excluded, so the program goes on recording, and the
    // fresh probe `publish_exclusions` asks for on its way out settles it a moment later.
    false
}

/// One pass over the table. Split out so that [`is_excluded_name`] reads as the generation check
/// it is.
///
/// Every load is `Relaxed`: what orders them is the `Acquire` load of the counter before this call
/// and the `fence(Acquire)` [`is_excluded_name`] executes after it, before the control load — the
/// canon of [`EXCLUSIONS_GENERATION`].
fn search_published(wanted: &[u8]) -> bool {
    let count = EXCLUSION_COUNT.load(Ordering::Relaxed).min(MAX_EXCLUSIONS);

    (0..count).any(|index| {
        EXCLUSION_LENS[index].load(Ordering::Relaxed) == wanted.len()
            && EXCLUSION_NAMES[index]
                .iter()
                .zip(wanted)
                .all(|(cell, byte)| cell.load(Ordering::Relaxed) == *byte)
    })
}

/// **Publishes `[exclusions] processes` to the watcher thread — FR-84, section 6.3.**
///
/// Called on the **UI thread**, which is the thread section 6.1 lets touch a file and the one that
/// owns the configuration, together with the other published values of section 7. Returns how many
/// names were accepted, so that a caller — and a test — can see the refusals rather than infer
/// them.
///
/// Idempotent and repeatable: the settings dialog of FR-92 (task T-08-1) calls it again with a new
/// list and nothing else has to be told. Publishing an empty slice is how the list is emptied, and
/// it is the state the program starts in.
///
/// # What is refused, and why refusing is right
///
/// An empty name after folding, a name over [`MAX_EXCLUSION_NAME_BYTES`], and everything past
/// [`MAX_EXCLUSIONS`]. All three are counted into [`Counters::exclusions_refused`] and none is
/// truncated or silently accepted in part: this list decides whether the program records at all,
/// and a half-stored name is a name that matches the wrong program.
///
/// # The probe on the way out
///
/// The flag describes the window that had the focus when the *previous* list was compared, so a
/// new list that nobody re-compared would not take effect until the user happened to change
/// windows. The last line asks the watcher thread for a fresh probe — the same request
/// [`note_focus_moved`] makes, through the same message, and it is a request and not a command:
/// a program with no watcher window yet leaves the answer to the first focus change, which is
/// exactly what start-up looks like.
///
/// **The buffer is not touched.** Unlike [`note_focus_moved`] this does not move the focus
/// generation and does not publish [`Field::Pending`]: a user editing a list of process names has
/// not moved the caret, and emptying their typing buffer for it would be a reset FR-10 does not
/// list.
pub fn publish_exclusions(names: &[String]) -> usize {
    // Odd for the whole of the write — see `EXCLUSIONS_GENERATION`.
    EXCLUSIONS_GENERATION.fetch_add(1, Ordering::AcqRel);

    let mut written = 0usize;
    let mut refused = 0u32;

    for name in names {
        let folded = fold_process_name(name);
        let bytes = folded.as_bytes();

        if written == MAX_EXCLUSIONS || bytes.is_empty() || bytes.len() > MAX_EXCLUSION_NAME_BYTES {
            refused = refused.saturating_add(1);
            continue;
        }

        for (cell, byte) in EXCLUSION_NAMES[written].iter().zip(bytes) {
            cell.store(*byte, Ordering::Relaxed);
        }

        // After the bytes it bounds, which is the order module `layouts` publishes its own list
        // in: a row is described only once it is written.
        EXCLUSION_LENS[written].store(bytes.len(), Ordering::Relaxed);
        written += 1;
    }

    EXCLUSION_COUNT.store(written, Ordering::Relaxed);
    EXCLUSIONS_PUBLISHED.store(
        u32::try_from(written).unwrap_or(u32::MAX),
        Ordering::Relaxed,
    );
    EXCLUSIONS_REFUSED.store(refused, Ordering::Relaxed);

    // Even again: the table is a list rather than a state somebody is in the middle of writing.
    EXCLUSIONS_GENERATION.fetch_add(1, Ordering::AcqRel);

    request_probe();

    written
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

    /// **FR-84 is a veto over FR-70 and FR-73, and it is one in all eight combinations.**
    ///
    /// The table is written out rather than derived, because what is being checked is that no
    /// state of the focused control puts buffering back on inside an excluded process — and a
    /// check that computed the expectation the same way the code does would check nothing.
    #[test]
    fn an_excluded_process_buffers_in_no_state_of_the_field() {
        for field in [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ] {
            assert!(
                !buffering_allowed_in(field, true),
                "FR-84: «буферизация не ведётся» in {}, whatever the field",
                field.name()
            );
        }

        // And with nothing excluded the rule is exactly the one task T-06-1 wrote — this is
        // acceptance point 17, that an empty list leaves the previous behaviour alone.
        assert!(!buffering_allowed_in(Field::Password, false), "FR-70");
        assert!(!buffering_allowed_in(Field::Pending, false), "FR-71");
        assert!(buffering_allowed_in(Field::Ordinary, false));
        assert!(buffering_allowed_in(Field::Undetermined, false), "FR-73");

        for field in [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ] {
            assert_eq!(
                buffering_allowed_in(field, false),
                buffering_allowed_for(field),
                "with nothing excluded the two rules answer alike for {}",
                field.name()
            );
        }
    }

    /// **The two published facts share one word and survive the round trip** — all eight
    /// combinations.
    ///
    /// This is the assertion the whole concurrency argument of task T-06-3 rests on: there is one
    /// atomic, so a reader cannot see one fact updated and the other not. If the packing lost a
    /// state the argument would be worth nothing.
    #[test]
    fn the_field_and_the_exclusion_share_one_word_without_colliding() {
        for field in [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ] {
            for excluded in [false, true] {
                assert_eq!(
                    unpack(pack(field, excluded)),
                    (field, excluded),
                    "{} / {excluded}",
                    field.name()
                );
            }
        }

        // No field code reaches the bit the exclusion lives in, which is what makes the two
        // independent rather than merely usually independent.
        for field in [
            Field::Pending,
            Field::Ordinary,
            Field::Password,
            Field::Undetermined,
        ] {
            assert_eq!(field as u8 & EXCLUDED_BIT, 0, "{}", field.name());
        }

        assert_eq!(
            EXCLUDED_BIT & FIELD_MASK,
            0,
            "the two halves do not overlap"
        );

        // `enter_pending` reaches `Pending` by clearing `FIELD_MASK` and keeping everything else,
        // which is only "publish Pending, keep the bit and the generation" while the interval
        // state is the zero code.
        assert_eq!(pack(Field::Pending, false), 0);
        assert_eq!(pack(Field::Pending, true), EXCLUDED_BIT);
    }

    /// **The generation rides above the two published facts and cannot reach the reader** — task
    /// **T-13-12**, and the whole of what makes the widened word safe for FR-71.
    ///
    /// The reader is one relaxed load and a truncating cast (see [`state`]), so what has to hold
    /// is that the cast is a *complete* decode: for every generation and every combination of the
    /// two answers, the low byte of the word is exactly the byte [`pack`] would have produced on
    /// its own, and the generation comes back out of the same word for the writer.
    #[test]
    fn the_generation_shares_the_word_and_never_reaches_the_reader() {
        for generation in [
            0u32,
            1,
            2,
            0xFF,
            0x100,
            12_345,
            GENERATION_MASK - 1,
            GENERATION_MASK,
        ] {
            for field in [
                Field::Pending,
                Field::Ordinary,
                Field::Password,
                Field::Undetermined,
            ] {
                for excluded in [false, true] {
                    let raw = word(generation, field, excluded);

                    assert_eq!(
                        generation_of(raw),
                        generation,
                        "{generation} / {} / {excluded}",
                        field.name()
                    );

                    // This cast is the reader's whole decode — `state` performs exactly it.
                    assert_eq!(
                        unpack(raw as u8),
                        (field, excluded),
                        "{generation} / {} / {excluded}",
                        field.name()
                    );
                }
            }
        }

        // And the generation is wide enough for the ABA argument of `GENERATION_MASK`: a stale
        // verdict is mistaken for a live one only after this many focus changes inside one probe.
        assert_eq!(GENERATION_MASK, 0x00FF_FFFF);
        assert_eq!(GENERATION_STEP, 0x100);
    }

    /// **⚠ The TOCTOU of the audit of 2026-08-24, driven through the window it lived in** — task
    /// **T-13-12**, finding «средняя: TOCTOU в `run_pending_probe`».
    ///
    /// The accepted code compared [`FIELD`]'s generation and then stored the verdict, and between
    /// those two instructions the input thread could run the whole of [`note_focus_moved`]: the
    /// generation went up, the state went to [`Field::Pending`] for a field that may be a password
    /// box, a fresh probe was asked for — and then the *previous* field's `Ordinary` landed on top
    /// of it. Buffering back **on**, with the caret already in the password field, until the new
    /// probe answered: tens of milliseconds ordinarily and up to [`PROBE_BUDGET_MS`] against a
    /// provider that does not reply.
    ///
    /// # What is interleaved, and why it is interleaved by hand
    ///
    /// The window is between two instructions of the watcher thread, so a test that raced two real
    /// threads for it would be a test that passes by luck. What is driven here instead is the seam
    /// itself: the probe's two halves — capture the generation, publish under it — are called
    /// apart, and the input thread's half of [`note_focus_moved`] is run **between** them, which is
    /// the interleaving named in the finding and nothing weaker.
    ///
    /// [`enter_pending`] and not the whole of [`note_focus_moved`], for one reason: the other half
    /// of that function posts [`WM_APP_PROBE`], a test process has no watcher window to post it to,
    /// and the FR-73 fall-back that follows the failure would publish a verdict of its own and hide
    /// the state under test. `enter_pending` **is** everything [`note_focus_moved`] does to
    /// [`FIELD`] — that is why it is a function — so nothing of the interleaving is lost.
    ///
    /// The seam needs no new element of feature `testing` (R-53) and there is none: both halves are
    /// ordinary private functions of this module, called from the module's own test.
    #[test]
    fn a_stale_verdict_cannot_overwrite_the_pending_of_a_new_field() {
        // ---- the state a probe starts in --------------------------------------------------
        let generation = enter_pending();

        assert_eq!(
            field(),
            Field::Pending,
            "the focus moved and nothing is known"
        );

        // `determine()` would run here, on the watcher thread, for tens of milliseconds. Its
        // verdict — for the field the user is about to leave.
        let stale = Probe {
            field: Field::Ordinary,
            excluded: false,
        };

        // ---- ⚠ the interleaving ------------------------------------------------------------
        // The focus moves again, entirely, after the probe captured its generation and before it
        // publishes. This is the input thread running while the watcher thread is preempted.
        let newer = enter_pending();

        assert_ne!(newer, generation, "a focus change opens a new generation");
        assert_eq!(
            field(),
            Field::Pending,
            "and republishes the interval state"
        );

        let before = counters();

        // ---- the stale publication ---------------------------------------------------------
        assert!(
            !publish(stale, generation),
            "a verdict determined under a generation the focus has left is refused"
        );

        assert_eq!(
            field(),
            Field::Pending,
            "SEC-06: the interval state of the new field stands; the previous field's Ordinary \
             did not overwrite it"
        );
        assert!(
            !buffering_allowed(),
            "FR-70: buffering is still off, which is what the caret being in a password field \
             requires"
        );

        let after = counters();

        assert_eq!(
            after.stale_verdicts,
            before.stale_verdicts + 1,
            "the refusal is counted where the audit asked for it"
        );
        assert_eq!(
            after.ordinary_verdicts, before.ordinary_verdicts,
            "and a verdict that was not published is not counted as one"
        );

        // ---- and nothing else is refused ---------------------------------------------------
        // The predicate is the generation and only the generation: the same verdict published
        // under the generation that is current lands, so the fix costs no correct publication.
        assert!(publish(stale, newer));
        assert_eq!(field(), Field::Ordinary);
        assert_eq!(counters().ordinary_verdicts, before.ordinary_verdicts + 1);

        // A second verdict for a generation that already has one lands too — this is the probe
        // `publish_exclusions` asks for without a focus change, and refusing it would leave a new
        // exclusion list unapplied until the user moved the focus.
        assert!(publish(
            Probe {
                field: Field::Password,
                excluded: true,
            },
            newer
        ));
        assert_eq!(field(), Field::Password);
        assert!(excluded());

        // ---- the process is left as it starts ----------------------------------------------
        // FR-73's state with nothing excluded, which is what `FIELD` is initialised to.
        assert!(publish(
            Probe {
                field: Field::Undetermined,
                excluded: false,
            },
            newer
        ));
        assert_eq!(field(), Field::Undetermined);
        assert!(!excluded());
    }

    /// **The comparison rule of FR-84, as a function of a string.**
    ///
    /// Acceptance point 18: the name and not the path, case-insensitively, and what happens to a
    /// name that is not ASCII.
    #[test]
    fn the_exclusion_comparison_is_by_name_and_ignores_case() {
        // The name, never the path — FR-84 says «имён процессов».
        assert_eq!(fold_process_name(r"C:\Games\Game.exe"), "game.exe");
        assert_eq!(fold_process_name("game.exe"), "game.exe");
        assert_eq!(
            fold_process_name(r"\Device\HarddiskVolume3\Games\GAME.EXE"),
            "game.exe"
        );

        // A person writing a list in a text file.
        assert_eq!(fold_process_name("  Game.exe  "), "game.exe");
        assert_eq!(fold_process_name(""), "");
        assert_eq!(fold_process_name("   "), "");

        // Case, in every spelling the same program is reported in.
        for spelling in ["Game.exe", "GAME.EXE", "gAmE.ExE", "game.exe"] {
            assert_eq!(fold_process_name(spelling), "game.exe", "{spelling}");
        }

        // ⚠ Not ASCII, and the reason this module folds with `to_lowercase` rather than with the
        // `eq_ignore_ascii_case` level 1 of FR-72 uses: an ASCII fold leaves these three apart,
        // and a user who wrote a Cyrillic executable name into their configuration means the
        // program of that name however Windows happens to report its case.
        for spelling in ["Игра.exe", "ИГРА.EXE", "игра.EXE"] {
            assert_eq!(fold_process_name(spelling), "игра.exe", "{spelling}");
        }

        assert_eq!(fold_process_name("ÜBERSETZER.EXE"), "übersetzer.exe");

        // Different programs stay different: the comparison is on the whole name and matches no
        // prefix, suffix or substring of one.
        for other in ["game", "game.exe.exe", "mygame.exe", "game.ex", "gameexe"] {
            assert_ne!(fold_process_name(other), "game.exe", "{other}");
        }
    }

    /// **The published list, end to end without a window** — FR-84 and section 6.3.
    ///
    /// One test and not five, on the pattern `tests\control.rs` set: the table is process-wide
    /// state, `cargo test` runs the tests of a binary in parallel, and two of these would collide
    /// over it. Everything asserted here is asserted about a list this body published itself, and
    /// the body ends by putting the default of section 7 back.
    #[test]
    fn the_published_list_is_searched_by_name_and_defaults_to_empty() {
        // The default of section 7, before anything is published: nothing is excluded, and the
        // program behaves exactly as it did before this requirement existed.
        assert_eq!(publish_exclusions(&[]), 0);
        assert!(!is_excluded_name("game.exe"));
        assert!(!is_excluded_name("anything.exe"));
        assert_eq!(counters().exclusions, 0);

        // A list, in the spellings a configuration file arrives in.
        let published = publish_exclusions(&[
            "Game.exe".to_owned(),
            r"C:\Games\Second.exe".to_owned(),
            "  Третья.EXE  ".to_owned(),
        ]);

        assert_eq!(published, 3);
        assert_eq!(counters().exclusions, 3);
        assert_eq!(counters().exclusions_refused, 0);

        for name in ["game.exe", "GAME.EXE", "Game.Exe"] {
            assert!(is_excluded_name(name), "FR-84: {name} is in the list");
        }

        assert!(
            is_excluded_name("second.exe"),
            "the path lost its directory"
        );
        assert!(is_excluded_name("ТРЕТЬЯ.exe"), "and its case, past ASCII");

        // And nothing else is.
        for name in ["notepad.exe", "game", "gam.exe", "game.exe.exe", ""] {
            assert!(!is_excluded_name(name), "{name} is not in the list");
        }

        // Refusals are counted rather than truncated — a truncated name matches a different
        // program. Two of them here: the empty one and the over-long one.
        let long = "a".repeat(MAX_EXCLUSION_NAME_BYTES + 1);
        let accepted = publish_exclusions(&["ok.exe".to_owned(), String::new(), long.clone()]);

        assert_eq!(accepted, 1);
        assert_eq!(counters().exclusions_refused, 2);
        assert!(is_excluded_name("ok.exe"));
        assert!(
            !is_excluded_name(&long),
            "the over-long name was not stored"
        );

        // The previous list is gone rather than merged into the new one.
        assert!(!is_excluded_name("game.exe"), "a publication replaces");

        // Everything past `MAX_EXCLUSIONS` is refused, and the ones that fit still work.
        let many: Vec<String> = (0..MAX_EXCLUSIONS + 4)
            .map(|index| {
                let mut name = index.to_string();
                name.push_str(".exe");
                name
            })
            .collect();

        assert_eq!(publish_exclusions(&many), MAX_EXCLUSIONS);
        assert_eq!(counters().exclusions_refused, 4);
        assert!(is_excluded_name("0.exe"));
        assert!(!is_excluded_name("999.exe"));

        // No search ever had to be repeated: nothing else in this process publishes.
        assert_eq!(counters().exclusion_read_retries, 0);

        // Section 7's default put back, so that whatever runs after this sees the program as it
        // starts.
        assert_eq!(publish_exclusions(&[]), 0);
        assert!(!is_excluded_name("ok.exe"));
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
