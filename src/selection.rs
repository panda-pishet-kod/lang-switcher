//! The selection path: working with the clipboard, saving and restoring it.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: **FR-62** (retries around every clipboard access),
//! **FR-63** (`AddClipboardFormatListener` and telling our own changes from the user's) and
//! **FR-64** (the snapshot and the restore of every listed format under one four-megabyte
//! budget) — task **T-07-1**, together with the sequence-number wait step 3 of FR-61 asks for.
//! Requirements this module will cover: **FR-60**, **FR-61** as a whole and **FR-65** — task
//! **T-07-2**, which is the single user of everything below.
//! Implemented by backlog tasks: T-07-1 (done), T-07-2.
//!
//! # The one sentence this module is built around
//!
//! **The clipboard is not ours.** It is a single, process-wide, system-wide resource that the
//! user keeps things in, and this program borrows it in the middle of an operation the user did
//! not ask to involve the clipboard at all. Two consequences run through every line below.
//!
//! **First: whatever is changed comes back.** Not on the happy path — on every path. A snapshot
//! is taken before anything is written and is restored afterwards, including when the steps in
//! between failed, which is why [`restore`] takes a `&Snapshot` and cannot consume it, and why
//! nothing in this module returns early between an [`EmptyClipboard`] and the writes that follow
//! it.
//!
//! **Second: the clipboard is closed.** `OpenClipboard` takes a system-wide lock. A process that
//! opens it and does not close it does not break itself — it breaks `Ctrl+C` in **every
//! application on the machine** until it exits. That is the most visible failure this task is
//! able to produce, and [`Clipboard`] exists so that it cannot be produced by forgetting: the
//! functions that need an open clipboard are methods on a guard whose `Drop` closes it, and the
//! guard is the only way to reach them.
//!
//! # Where the code of this module runs — FR-80, NFR-01, NFR-05
//!
//! Everything here is **slow by construction**. FR-62 prescribes ten attempts twenty
//! milliseconds apart, so a single clipboard access can cost 180 ms before it gives up; step 3
//! of FR-61 waits up to 300 ms for a sequence number to move; step 8 waits another 200 ms
//! before restoring. Against that, FR-80 says the system removes a `WH_KEYBOARD_LL` hook
//! **silently** when its callback overruns `LowLevelHooksTimeout`, and NFR-01 gives that
//! callback a hundred microseconds. A low-level keyboard hook is called back on the thread that
//! installed it, and that thread can only answer while it is pumping messages — so a thread that
//! sits inside a 300 ms wait is a thread that is not answering the hook, and 300 ms is three
//! thousand times NFR-01.
//!
//! Hence a hard rule, enforced rather than documented: **no blocking primitive of this module
//! runs on the thread that owns the hook.** [`caller_may_block`] is the check, every entry point
//! that can sleep begins with it, and the answer when it fails is
//! [`ClipboardError::WrongThread`] — a refusal, not a slow success. The thread that *may* run
//! them is the one that registered the listener, which [`listen`] records, and `app` calls
//! [`listen`] from the **UI thread** of section 6.1:
//!
//! | Thread of section 6.1 | Why it is or is not the host |
//! |---|---|
//! | input | owns the `WH_KEYBOARD_LL` hook (FR-01) and the typing buffer (section 6.3). **Excluded** — this is the whole of the paragraph above |
//! | watcher | owns the focus probe of FR-71 and FR-72, whose budget is 50 ms per level and on whose answer SEC-06 holds the typing buffer switched off. A 300 ms clipboard wait here would lengthen exactly the window in which the user's typing is deliberately dropped |
//! | **UI** | already the thread section 6.1 gives «Чтение и запись конфигурации» — the process's slow, deadline-free work — and the only one of the three with nothing on a latency budget. **The host** |
//!
//! Two independent tests answer [`caller_may_block`], and they cover each other: the typing
//! buffer of section 6.3 is installed on the input thread and on no other, and the worker thread
//! id published by [`listen`] names the UI thread positively. The first survives a failed
//! listener registration; the second survives [`crate::app::park_buffer`], which takes the
//! typing buffer away from the input thread for the duration of a password field (SEC-06) and
//! would otherwise make the input thread look like any other.
//!
//! # What may be written down — SEC-01, SEC-07
//!
//! SEC-07 names the clipboard directly: «Символы, коды клавиш и **содержимое буфера обмена** в
//! журнал не попадают ни при каком уровне детализации». [`Snapshot`] holds clipboard content,
//! because holding it is the point, and everything about the type is arranged so that the
//! content has nowhere to leak to:
//!
//! * no `Display`, and a hand-written [`Debug`](core::fmt::Debug) that prints six counts and not
//!   one byte — a derived one would print the bytes;
//! * no accessor returns the captured bytes. The public surface answers *how many* formats,
//!   *which* format numbers and *how many* bytes, and format numbers are constants of the
//!   operating system, not user data;
//! * `Drop` overwrites every captured block with zeroes before it is freed, the same treatment
//!   [`crate::buffer`] gives the typing buffer for SEC-02;
//! * nothing in this module panics with a payload built from clipboard data — nothing in this
//!   module panics at all, which the source sweep in `tests\selection.rs` checks;
//! * the journal is reached with [`crate::diag::Operation`] values and `HRESULT`s only, never
//!   with text.

use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use core::time::Duration;
use std::thread::sleep;
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats,
    GetClipboardData, GetClipboardSequenceNumber, OpenClipboard, RemoveClipboardFormatListener,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::WM_CLIPBOARDUPDATE;
use windows::core::{Error as WinError, Free, Result as WinResult};

use crate::settings::Selection;

// ---------------------------------------------------------------------------------------
// The numbers the specification fixes
// ---------------------------------------------------------------------------------------

/// FR-62: how many times a clipboard access is attempted before it is given up on.
///
/// «Число повторов — 10» — ten attempts in total, not one attempt and ten more. That is the
/// reading acceptance point 13 of the task specification writes out in full: «10 попыток,
/// интервал 20 мс».
pub const OPEN_ATTEMPTS: u32 = 10;

/// FR-62: the interval between two consecutive attempts.
///
/// Ten attempts have nine intervals between them, so the worst case of a refused access is
/// `9 * 20 = 180` milliseconds and not two hundred. See [`WORST_CASE_OPEN`].
pub const OPEN_RETRY_INTERVAL: Duration = Duration::from_millis(20);

/// The longest a single clipboard access can take before it answers [`ClipboardError::Busy`].
///
/// Stated as a constant because it is the number that decides which thread may call into this
/// module at all, and a number that decides something should be visible rather than implied.
pub const WORST_CASE_OPEN: Duration =
    Duration::from_millis(OPEN_RETRY_INTERVAL.as_millis() as u64 * (OPEN_ATTEMPTS as u64 - 1));

/// FR-64: the budget a snapshot has for **all formats together**.
///
/// «охватывает все перечисленные форматы **суммарным объёмом** до 4 МБ» — one budget for the
/// whole clipboard, not four megabytes per format. Over it, [`snapshot`] keeps
/// [`CF_UNICODETEXT`] and nothing else, and says so in the journal.
pub const SNAPSHOT_BUDGET_BYTES: usize = 4 * 1024 * 1024;

/// How often [`wait_for_change`] looks at the sequence number.
///
/// Five milliseconds: sixty looks across the 300 ms of step 3 of FR-61, each of them one call
/// to `GetClipboardSequenceNumber`, which needs no clipboard lock and cannot block. Polling is
/// the only option available — the system has no "wait until the clipboard changes" primitive,
/// and the notification it does have, `WM_CLIPBOARDUPDATE`, is a message that would have to be
/// pumped by a thread that is at that moment inside step 3 of a sequence of eight.
pub const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Standard clipboard format `CF_UNICODETEXT`.
///
/// # Why the number and not the constant
///
/// The `windows` crate declares `CF_UNICODETEXT` in `Win32::System::Ole`, a feature that is not
/// in the closed list of section 3.2 of SPEC and that this task may not add. The value is not a
/// detail of that crate: standard clipboard format numbers are part of the Windows API contract,
/// have been thirteen and two and one since Windows 3.0, and are what travels over the wire of
/// `GetClipboardData` in any case — the binding takes a plain `u32`. Writing them here costs one
/// table and keeps the dependency list untouched.
pub const CF_UNICODETEXT: u32 = 13;

/// The largest number of formats [`snapshot`] will walk before it stops enumerating.
///
/// `EnumClipboardFormats` terminates by answering zero, and this bound is not a substitute for
/// that: it is the answer to a clipboard whose enumeration does not terminate, which is a state
/// this process cannot rule out because it does not own the clipboard. A real clipboard carries
/// single digits of formats; the richest this project has measured is Microsoft Word at
/// seventeen.
const MAX_FORMATS: usize = 256;

// ---------------------------------------------------------------------------------------
// Which formats can be copied as bytes, and which cannot — FR-64
// ---------------------------------------------------------------------------------------

/// Formats whose clipboard data is **not** a movable memory block.
///
/// `GetClipboardData` answers a handle whose *type depends on the format*: `HBITMAP` for
/// `CF_BITMAP`, `HPALETTE` for `CF_PALETTE`, `HENHMETAFILE` for `CF_ENHMETAFILE`, nothing at all
/// for `CF_OWNERDISPLAY`. `GlobalSize` and `GlobalLock` are meaningful for exactly one of those
/// kinds, and calling them on the others is asking the memory manager about a handle that was
/// never its.
///
/// `CF_METAFILEPICT` is in the list although it *is* an `HGLOBAL`: the block holds a
/// `METAFILEPICT` whose first field is an `HMETAFILE`, and `EmptyClipboard` frees that inner
/// handle along with the outer one. Copying the sixteen bytes and putting them back would
/// restore a structure pointing at a metafile that no longer exists — worse than not restoring
/// it, because it looks restored.
///
/// Duplicating these would need `CopyImage`, `CopyEnhMetaFile` and `CreatePalette`, that is,
/// GDI, and each of them a second lifetime to manage. The trade this module takes instead rests
/// on **clipboard format synthesis**, which the system performs for free: an application that
/// puts a picture on the clipboard puts `CF_DIB` or `CF_DIBV5` there as well — both ordinary
/// memory blocks, both captured here — and the system synthesises `CF_BITMAP` from either on
/// demand. The same holds for `CF_TEXT`, `CF_OEMTEXT` and `CF_LOCALE` around `CF_UNICODETEXT`.
/// What is genuinely lost is content that exists **only** as a metafile or only as a palette,
/// which is the limitation the report and `README.md` state in the user's words.
const HANDLE_FORMATS: &[u32] = &[
    2,   // CF_BITMAP           — HBITMAP
    3,   // CF_METAFILEPICT     — HGLOBAL naming an HMETAFILE that EmptyClipboard frees
    9,   // CF_PALETTE          — HPALETTE
    14,  // CF_ENHMETAFILE      — HENHMETAFILE
    128, // CF_OWNERDISPLAY     — no data handle at all; the owner paints
    130, // CF_DSPBITMAP        — HBITMAP
    131, // CF_DSPMETAFILEPICT  — as CF_METAFILEPICT
    142, // CF_DSPENHMETAFILE   — HENHMETAFILE
];

/// First and last of `CF_PRIVATEFIRST..=CF_PRIVATELAST`.
///
/// Private formats belong to one application and are documented as **not** freed by
/// `EmptyClipboard`, which is the system saying that it does not know what they are. A handle
/// whose kind is unknown is one this module declines to treat as memory.
const PRIVATE_FIRST: u32 = 0x0200;
const PRIVATE_LAST: u32 = 0x02FF;

/// First and last of `CF_GDIOBJFIRST..=CF_GDIOBJLAST` — GDI object handles by definition.
const GDIOBJ_FIRST: u32 = 0x0300;
const GDIOBJ_LAST: u32 = 0x03FF;

/// Whether the data of `format` is a movable memory block this module can copy.
///
/// A pure function, and the reason it is one: it is the whole of the FR-64 policy on what a
/// snapshot can and cannot hold, and the unit tests at the end of this file drive it over every
/// standard format number without a clipboard anywhere in sight.
pub const fn is_memory_format(format: u32) -> bool {
    let mut index = 0;

    while index < HANDLE_FORMATS.len() {
        if HANDLE_FORMATS[index] == format {
            return false;
        }

        index += 1;
    }

    !(format >= PRIVATE_FIRST && format <= PRIVATE_LAST)
        && !(format >= GDIOBJ_FIRST && format <= GDIOBJ_LAST)
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// Value in [`LISTENER_WINDOW`] meaning "no listener is registered".
const NO_WINDOW: usize = 0;

/// Value in [`WORKER_THREAD`] meaning "nobody has claimed the clipboard work".
///
/// Zero is not a valid thread id on Windows, so it cannot collide with a real one.
const NO_THREAD: u32 = 0;

/// How many sequence numbers of our own writes are remembered — FR-63.
///
/// A ring rather than a single slot because one pass of FR-61 writes **twice**: step 6 puts the
/// converted text on the clipboard and step 8 puts the user's own content back. Both are ours,
/// both raise `WM_CLIPBOARDUPDATE`, and a single slot would let the second overwrite the first
/// before the first message had been taken out of the queue. Eight is four passes deep, which is
/// more than the queue can hold behind a thread that pumps.
const OWN_MARKS: usize = 8;

/// The window [`listen`] registered, as a `usize` — `HWND` is a raw pointer and not storable in
/// an atomic. The same device, for the same reason, as `app::WAKE_TARGETS`.
static LISTENER_WINDOW: AtomicUsize = AtomicUsize::new(NO_WINDOW);

/// The thread the blocking primitives of this module may run on — see the module documentation.
static WORKER_THREAD: AtomicU32 = AtomicU32::new(NO_THREAD);

/// Sequence numbers this process produced itself, newest overwriting oldest.
static OWN_MARK: [AtomicU32; OWN_MARKS] = [const { AtomicU32::new(0) }; OWN_MARKS];

/// Where the next entry of [`OWN_MARK`] goes.
static OWN_NEXT: AtomicUsize = AtomicUsize::new(0);

/// Successful `OpenClipboard` calls. Compared against [`CLOSES`] by the tests: the whole of
/// «`CloseClipboard` на каждом пути выхода» is that these two numbers are equal afterwards.
static OPENS: AtomicU32 = AtomicU32::new(0);

/// `CloseClipboard` calls that succeeded.
static CLOSES: AtomicU32 = AtomicU32::new(0);

/// `CloseClipboard` calls that failed. Recorded because a failure here is the one that would
/// leave the machine without `Ctrl+C`, so it may not be invisible.
static CLOSE_FAILURES: AtomicU32 = AtomicU32::new(0);

/// `OpenClipboard` attempts that were refused and slept — FR-62.
static OPEN_RETRIES: AtomicU32 = AtomicU32::new(0);

/// Accesses that used all of [`OPEN_ATTEMPTS`] and gave up.
static OPEN_REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Calls that were refused because the caller was not allowed to block — FR-80, NFR-01.
static WRONG_THREAD_REFUSALS: AtomicU32 = AtomicU32::new(0);

/// `WM_CLIPBOARDUPDATE` messages taken at the registered window — FR-63.
static UPDATES: AtomicU32 = AtomicU32::new(0);

/// Of those, the ones whose sequence number this process had produced itself.
static OWN_UPDATES: AtomicU32 = AtomicU32::new(0);

/// Of those, the ones that were somebody else's.
static FOREIGN_UPDATES: AtomicU32 = AtomicU32::new(0);

/// Snapshots that went over [`SNAPSHOT_BUDGET_BYTES`] and kept text only — FR-64.
static TRUNCATIONS: AtomicU32 = AtomicU32::new(0);

/// Formats that were listed on the clipboard and could not be read out of it.
///
/// Delayed rendering, mostly: see [`snapshot`]. Counted rather than escalated, because FR-64
/// asks for the other formats to survive one format's refusal.
static REFUSED_FORMATS: AtomicU32 = AtomicU32::new(0);

/// Formats that were listed and are not memory blocks — see [`HANDLE_FORMATS`].
static HANDLE_FORMATS_SEEN: AtomicU32 = AtomicU32::new(0);

/// `RemoveClipboardFormatListener` calls that failed — FR-63.
static LISTENER_REMOVE_FAILURES: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------------------
// The public error and the outcome types
// ---------------------------------------------------------------------------------------

/// What can go wrong on the way to the clipboard.
///
/// SEC-01, SEC-07: there is no variant here that can carry clipboard content. [`Self::Os`]
/// carries a `windows::core::Error`, whose payload is an `HRESULT` and a message the operating
/// system wrote; module `diag` takes the `HRESULT` from it and drops the message unread, exactly
/// as [`crate::app::report_non_critical`] describes.
#[derive(Debug)]
pub enum ClipboardError {
    /// The caller is not allowed to block — the thread it runs on owns the low-level hook, or
    /// is not the thread [`listen`] claimed. **FR-80, NFR-01.** See the module documentation.
    WrongThread,
    /// FR-62: [`OPEN_ATTEMPTS`] attempts [`OPEN_RETRY_INTERVAL`] apart and another process still
    /// held the clipboard. Not a defect of this program and not a reason to fail loudly: the
    /// clipboard is shared, and somebody else was in it.
    Busy,
    /// A Win32 call other than `OpenClipboard` refused. NFR-13: every return is examined, and
    /// this is where the ones that cannot be recovered from arrive.
    Os(WinError),
}

impl From<WinError> for ClipboardError {
    fn from(error: WinError) -> Self {
        Self::Os(error)
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongThread => {
                f.write_str("clipboard work asked for on a thread that may not block")
            }
            Self::Busy => f.write_str("the clipboard is held by another process"),
            Self::Os(error) => write!(f, "clipboard call failed: 0x{:08X}", error.code().0),
        }
    }
}

/// What [`wait_for_change`] saw — step 3 of FR-61.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    /// The sequence number moved. `sequence` is the new value, `waited` is how long it took.
    ///
    /// For step 3 of FR-61 this reads "there was a selection, and `Ctrl+C` copied it".
    Changed { sequence: u32, waited: Duration },
    /// The sequence number did not move within the timeout.
    ///
    /// For step 3 of FR-61 this reads "there is no selection" — and FR-60 then sends the hotkey
    /// down the typing-buffer path. It is an ordinary answer, not a failure.
    TimedOut { waited: Duration },
    /// The caller may not block here. **FR-80, NFR-01.**
    WrongThread,
}

/// Where a `WM_CLIPBOARDUPDATE` came from — FR-63.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// This process wrote it. The listener must not treat it as a user action, or the program
    /// reacts to itself: step 6 of FR-61 writes, step 8 writes again, and both would look like
    /// the user copying something.
    Own,
    /// Somebody else wrote it.
    Foreign,
}

/// One `WM_CLIPBOARDUPDATE`, classified — FR-63.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Update {
    /// The clipboard sequence number as it stood when the message was taken.
    pub sequence: u32,
    /// Whose change it was.
    pub origin: Origin,
}

/// What [`restore`] managed to put back — FR-64.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Restored {
    /// Formats that reached the clipboard.
    pub placed: usize,
    /// Formats the clipboard refused. **The rest were still placed** — that is the whole point
    /// of counting rather than returning early.
    pub refused: usize,
}

/// The counters this module keeps, taken together so that a reader sees one consistent picture.
///
/// SEC-01, SEC-07: every field is a count of program events. Not one of them is proportional to
/// what the user copied, and none of them can be turned back into it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Successful `OpenClipboard` calls.
    pub opens: u32,
    /// Successful `CloseClipboard` calls. **Equal to `opens` whenever no access is in flight.**
    pub closes: u32,
    /// `CloseClipboard` calls that failed.
    pub close_failures: u32,
    /// Attempts that were refused and retried — FR-62.
    pub open_retries: u32,
    /// Accesses that exhausted [`OPEN_ATTEMPTS`].
    pub open_refusals: u32,
    /// Calls refused because the caller may not block — FR-80, NFR-01.
    pub wrong_thread_refusals: u32,
    /// `WM_CLIPBOARDUPDATE` messages taken — FR-63.
    pub updates: u32,
    /// Of them, ours.
    pub own_updates: u32,
    /// Of them, the user's.
    pub foreign_updates: u32,
    /// Snapshots cut down to text by the budget of FR-64.
    pub truncations: u32,
    /// Listed formats that could not be read — delayed rendering, mostly.
    pub refused_formats: u32,
    /// Listed formats that are not memory blocks — see [`HANDLE_FORMATS`].
    pub handle_formats: u32,
    /// `RemoveClipboardFormatListener` calls that failed.
    pub listener_remove_failures: u32,
}

/// The counters as they stand.
pub fn counters() -> Counters {
    Counters {
        opens: OPENS.load(Ordering::Relaxed),
        closes: CLOSES.load(Ordering::Relaxed),
        close_failures: CLOSE_FAILURES.load(Ordering::Relaxed),
        open_retries: OPEN_RETRIES.load(Ordering::Relaxed),
        open_refusals: OPEN_REFUSALS.load(Ordering::Relaxed),
        wrong_thread_refusals: WRONG_THREAD_REFUSALS.load(Ordering::Relaxed),
        updates: UPDATES.load(Ordering::Relaxed),
        own_updates: OWN_UPDATES.load(Ordering::Relaxed),
        foreign_updates: FOREIGN_UPDATES.load(Ordering::Relaxed),
        truncations: TRUNCATIONS.load(Ordering::Relaxed),
        refused_formats: REFUSED_FORMATS.load(Ordering::Relaxed),
        handle_formats: HANDLE_FORMATS_SEEN.load(Ordering::Relaxed),
        listener_remove_failures: LISTENER_REMOVE_FAILURES.load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------------------
// Which thread may block — FR-80, NFR-01, section 6.1
// ---------------------------------------------------------------------------------------

/// Whether the calling thread is allowed to run a primitive that can sleep.
///
/// Two independent tests, described in the module documentation, and the order matters only in
/// that the cheaper one is first. Both are process state of ours; neither can be influenced from
/// outside the process.
///
/// A `false` here is not a diagnosis of a broken caller — it is the guarantee acceptance point
/// 19 of the task specification asks to be *shown*, and the test
/// `the_wait_refuses_the_thread_that_owns_the_hook` is that showing.
fn caller_may_block() -> bool {
    // Section 6.3 gives the typing buffer to the input thread and to no other, and FR-01 gives
    // that same thread the one `WH_KEYBOARD_LL` hook. This is the test `app::window_proc`
    // already uses to mean "this is the input thread", four times over.
    if crate::buffer::is_installed() {
        return false;
    }

    // And positively: the thread that registered the listener is the thread that may do the
    // work. `NO_THREAD` means no listener is registered at all — the state of the unit and
    // integration tests, and of the process before `app::serve_window` has run — and then there
    // is no hook either, so there is nothing to protect.
    let worker = WORKER_THREAD.load(Ordering::Acquire);

    worker == NO_THREAD || worker == current_thread_id()
}

/// The calling thread's id.
fn current_thread_id() -> u32 {
    // SAFETY: `GetCurrentThreadId` takes no arguments, touches no memory of ours and cannot
    // fail — it reads a field of the thread's own environment block. The same call, for the same
    // kind of comparison, as `switch::attach_to_foreground`.
    unsafe { GetCurrentThreadId() }
}

/// [`caller_may_block`], counted, for the entry points that return a [`ClipboardError`].
fn require_blocking_thread() -> Result<(), ClipboardError> {
    if caller_may_block() {
        return Ok(());
    }

    WRONG_THREAD_REFUSALS.fetch_add(1, Ordering::Relaxed);

    Err(ClipboardError::WrongThread)
}

// ---------------------------------------------------------------------------------------
// FR-62 — the open clipboard, and the close that cannot be forgotten
// ---------------------------------------------------------------------------------------

/// An open clipboard. **Closed when this value is dropped, on every path.**
///
/// # What guarantees the close — FR-62, acceptance point 14
///
/// | Path out of an access | What closes the clipboard |
/// |---|---|
/// | ordinary return | `Drop`, at the end of the scope |
/// | `?` on a failed Win32 call | `Drop`, as the frame unwinds the ordinary way — the guard is a local |
/// | early `return` | `Drop` |
/// | panic, Debug configuration | `Drop`, run by the unwinder. `panic = "abort"` is set for `[profile.release]` only |
/// | panic, Release configuration | the process aborts, and the system releases the clipboard when the owning **thread** ends. Measured, not assumed — point 26 of the task specification |
/// | `TerminateProcess` — FR-96, FR-97 | the same system cleanup as above |
///
/// The type is the mechanism rather than a convention around it: `EnumClipboardFormats`,
/// `GetClipboardData`, `EmptyClipboard` and `SetClipboardData` are wrapped as **methods** of this
/// guard and are not reachable without one, so an access that forgot to close is an access that
/// could not be written. Nothing in this module calls `CloseClipboard` by hand.
///
/// Neither `Send` nor `Sync`: the clipboard lock belongs to the thread that took it, and a guard
/// that could travel would invite a close from a thread that never opened it. Same shape, same
/// reason, as [`crate::hook::Installed`].
struct Clipboard {
    _not_send: PhantomData<*const ()>,
}

impl Clipboard {
    /// Opens the clipboard with the retries of FR-62.
    ///
    /// `owner` becomes the clipboard's window for the duration, and it may not be `None`: the
    /// documented behaviour of `OpenClipboard(NULL)` is that a subsequent `EmptyClipboard` sets
    /// the owner to null and **`SetClipboardData` then fails**, which would turn every restore
    /// into a silent loss of the user's clipboard.
    ///
    /// # FR-62 in full
    ///
    /// «Все обращения к буферу обмена выполняются с повторами» — *all*, not the first. This is
    /// the only door to an open clipboard in the module, so every access made anywhere below
    /// carries the ten attempts: [`snapshot`], [`restore`], [`read_unicode_text`] and
    /// [`write_unicode_text`] each go through it, and a fifth caller added later cannot avoid
    /// it.
    fn open(owner: HWND) -> Result<Self, ClipboardError> {
        require_blocking_thread()?;

        for attempt in 1..=OPEN_ATTEMPTS {
            // SAFETY: `owner` is a window handle the caller owns, passed by value; the call
            // takes the system-wide clipboard lock for this thread and dereferences nothing of
            // ours. NFR-13: the binding turns a `FALSE` return into `Err`, which is the failure
            // FR-62 exists for — another process holds the lock — and it is the branch below.
            match unsafe { OpenClipboard(Some(owner)) } {
                Ok(()) => {
                    OPENS.fetch_add(1, Ordering::Relaxed);

                    return Ok(Self {
                        _not_send: PhantomData,
                    });
                }
                Err(_) => {
                    // The error is deliberately dropped rather than reported: a busy clipboard
                    // is the ordinary case FR-62 describes, not a fault, and nine journal
                    // entries per contended access would drown the ring the one time something
                    // really went wrong.
                    if attempt < OPEN_ATTEMPTS {
                        OPEN_RETRIES.fetch_add(1, Ordering::Relaxed);
                        sleep(OPEN_RETRY_INTERVAL);
                    }
                }
            }
        }

        OPEN_REFUSALS.fetch_add(1, Ordering::Relaxed);

        Err(ClipboardError::Busy)
    }

    /// The format numbers on the clipboard right now, in the system's own order.
    ///
    /// `EnumClipboardFormats` needs the clipboard open, which is why this is a method. It
    /// answers zero both at the end of the list and on failure, and the two are told apart the
    /// way the documentation prescribes — by the thread's last error, which the binding has
    /// already turned into an `Err` for us.
    fn formats(&self) -> WinResult<Vec<u32>> {
        let mut formats = Vec::new();
        let mut current = 0_u32;

        while formats.len() < MAX_FORMATS {
            // SAFETY: the clipboard is open — the existence of `self` is that fact — which is
            // the precondition of the call. `current` is passed by value; the first iteration
            // passes zero, which is the documented request for the first format, and every
            // later one passes the format the previous iteration returned. Nothing of ours is
            // dereferenced.
            let next = unsafe { EnumClipboardFormats(current) };

            if next == 0 {
                // NFR-13: zero is both "no more formats" and "the call failed", and the
                // documented way to separate them is the thread's last error.
                let error = WinError::from_thread();

                return if error.code().is_ok() {
                    Ok(formats)
                } else {
                    Err(error)
                };
            }

            formats.push(next);
            current = next;
        }

        Ok(formats)
    }

    /// The data of `format` as a movable memory block, if there is any.
    ///
    /// `Ok(None)` covers the two ordinary absences: the format is not on the clipboard, and the
    /// format is there but its owner would not render it. **The second is the delayed-rendering
    /// case**, and it is why this answers an option instead of an error — see [`snapshot`].
    fn block(&self, format: u32) -> Option<HGLOBAL> {
        if !is_memory_format(format) {
            return None;
        }

        // SAFETY: the clipboard is open, which is the precondition. `format` is a number.
        // NFR-13: the binding turns the null return into `Err`, and a null return is exactly
        // what a format that is absent — or that its owner declined to render — produces. The
        // handle that comes back belongs to the clipboard: it is borrowed for as long as the
        // clipboard stays open, is not freed here, and does not outlive `self`.
        let handle = unsafe { GetClipboardData(format) }.ok()?;

        Some(HGLOBAL(handle.0))
    }

    /// Empties the clipboard and takes ownership of it.
    ///
    /// Every handle that was on it is freed by the system here, which is why a snapshot has to
    /// be a **copy** of the bytes and cannot be a list of handles.
    fn empty(&self) -> WinResult<()> {
        // SAFETY: the clipboard is open and was opened with a window of this thread, which is
        // what `EmptyClipboard` requires in order to make that window the owner. The call takes
        // no arguments and dereferences nothing of ours. NFR-13: the binding checks the `BOOL`.
        unsafe { EmptyClipboard() }
    }

    /// Puts `bytes` on the clipboard under `format`.
    ///
    /// The block is allocated movable, filled and handed over; on success the **system** owns it
    /// and must not be freed by us, and on failure [`Block`] still owns it and frees it as this
    /// frame leaves. That split is the whole reason [`Block::release`] exists.
    fn put(&self, format: u32, bytes: &[u8]) -> WinResult<()> {
        let mut block = Block::alloc(bytes.len())?;

        block.fill(bytes)?;

        let handle = HANDLE(block.release().0);

        // SAFETY: the clipboard is open and was emptied by us, so this process owns it — the
        // precondition of `SetClipboardData`. `handle` is a movable global block this frame
        // allocated and unlocked, and `release` has already given up our claim on it, so the
        // system's taking ownership is not a double owner. NFR-13: the binding turns the null
        // return into `Err`.
        match unsafe { SetClipboardData(format, Some(handle)) } {
            Ok(_) => Ok(()),
            Err(error) => {
                // Ownership did not pass after all. Reclaiming it here is the difference
                // between a refused format and a leak of the user's own data.
                let mut orphan = HGLOBAL(handle.0);

                // SAFETY: `orphan` is the block allocated above. `SetClipboardData` failed, so
                // the system did not take it and this frame is still its only owner; the
                // `Free` implementation checks the handle for validity and frees it once.
                unsafe { orphan.free() };

                Err(error)
            }
        }
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        // SAFETY: this thread opened the clipboard — the existence of this value is that fact,
        // and the type is neither `Copy` nor `Clone`, so the close happens exactly once. The
        // call takes no arguments and dereferences nothing.
        match unsafe { CloseClipboard() } {
            Ok(()) => {
                CLOSES.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                // NFR-13: examined. There is nothing left to try — a clipboard that will not
                // close is a state only the end of the process resolves — so it is counted and
                // journalled, and the program carries on. This is the one counter of this module
                // whose non-zero value means the machine is worse off than before.
                CLOSE_FAILURES.fetch_add(1, Ordering::Relaxed);
                crate::app::report_non_critical("CloseClipboard", &error);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// A movable global block, owned until the clipboard takes it
// ---------------------------------------------------------------------------------------

/// A `GMEM_MOVEABLE` block this frame owns.
///
/// Exists for one transition: everything allocated for the clipboard is ours until
/// `SetClipboardData` succeeds and the system's from that instant. A raw `HGLOBAL` cannot say
/// which side of that line it is on; this type can, and its `Drop` covers every path that does
/// not cross it.
struct Block(HGLOBAL);

impl Block {
    /// Allocates `len` bytes, movable, as `SetClipboardData` requires.
    ///
    /// A zero-length block is refused rather than allocated: `GlobalAlloc(GMEM_MOVEABLE, 0)`
    /// answers a handle to a *discarded* object that `GlobalLock` then declines, and a format
    /// whose data is empty carries nothing to restore in any case.
    fn alloc(len: usize) -> WinResult<Self> {
        if len == 0 {
            return Err(WinError::from_hresult(
                windows::Win32::Foundation::E_INVALIDARG,
            ));
        }

        // SAFETY: a plain allocation request of `len` bytes. `GMEM_MOVEABLE` is what
        // `SetClipboardData` documents as required for clipboard data. NFR-13: the binding
        // turns the null return into `Err`. Nothing of ours is dereferenced.
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, len) }?;

        Ok(Self(handle))
    }

    /// Copies `bytes` into the block. `bytes.len()` must be the length it was allocated with.
    fn fill(&mut self, bytes: &[u8]) -> WinResult<()> {
        // SAFETY: `self.0` is a live movable block allocated by `alloc` and not yet released.
        // `GlobalLock` pins it and answers a pointer to at least the allocated length.
        let pointer = unsafe { GlobalLock(self.0) };

        // NFR-13: a null pointer is the documented failure of `GlobalLock`.
        if pointer.is_null() {
            return Err(WinError::from_thread());
        }

        // SAFETY: `pointer` is non-null and, by the contract of `GlobalLock`, points at a region
        // of at least the length this block was allocated with, which is `bytes.len()` — `fill`
        // is called once, immediately after `alloc`, with the same slice whose length was passed
        // there. The source and the destination are distinct allocations, so they cannot
        // overlap. The region is uninitialised memory being written, not read.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
        }

        unlock(self.0)
    }

    /// Gives up ownership and answers the handle, for `SetClipboardData` to take over.
    fn release(self) -> HGLOBAL {
        let handle = self.0;

        // The block must outlive this frame — the clipboard is about to own it — so the `Drop`
        // below is skipped deliberately. Not a leak: the caller hands the handle to
        // `SetClipboardData` on the very next line and reclaims it itself if that refuses.
        core::mem::forget(self);

        handle
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a block this type allocated and has not released — `release`
        // consumes the value and forgets it, so a `Drop` that runs is a `Drop` on a block the
        // clipboard never took. The `Free` implementation checks the handle and frees it once.
        unsafe { self.0.free() };
    }
}

/// `GlobalUnlock`, examined the way its documentation asks — NFR-13.
///
/// The call answers `FALSE` **on success** when it takes the lock count down to zero, leaving
/// `GetLastError` at `NO_ERROR`; the binding sees the `FALSE` and builds an `Err` whose
/// `HRESULT` is `S_OK`. Treating that as a failure would turn every correct unlock in the module
/// into an error, and treating the result as uninteresting would be the ignoring NFR-13 forbids.
/// This is the third option: examine it, and know what the answer means.
fn unlock(handle: HGLOBAL) -> WinResult<()> {
    // SAFETY: `handle` is a live movable block this module locked exactly once, on the line
    // above every call site. Unlocking a block that was locked is the documented pairing.
    match unsafe { GlobalUnlock(handle) } {
        Ok(()) => Ok(()),
        Err(error) if error.code().is_ok() => Ok(()),
        Err(error) => Err(error),
    }
}

// ---------------------------------------------------------------------------------------
// FR-64 — the snapshot
// ---------------------------------------------------------------------------------------

/// One captured format.
struct Captured {
    /// The clipboard format number. A constant of the operating system, never user data.
    format: u32,
    /// The bytes. **The only place in this program where clipboard content lives**, and it lives
    /// here for as long as one pass of FR-61 takes.
    bytes: Vec<u8>,
}

/// The clipboard as it was, ready to be put back — FR-64.
///
/// Take one before writing anything, hand it to [`restore`] afterwards, and hand it to
/// [`restore`] **also** when the steps in between failed: FR-61 step 8 is not conditional on
/// steps 4 to 7 having worked, and `restore` takes the snapshot by reference so that it can be
/// used from a cleanup path that does not own it.
///
/// See the module documentation for what this type deliberately cannot do.
pub struct Snapshot {
    /// What was copied, in the order the system listed it.
    captured: Vec<Captured>,
    /// How many formats the clipboard listed, whether or not they were captured.
    listed: usize,
    /// How many listed formats are not memory blocks — see [`HANDLE_FORMATS`].
    handle_formats: usize,
    /// How many listed formats could not be read out.
    refused: usize,
    /// The sum of `captured[..].bytes.len()`.
    bytes: usize,
    /// Whether the budget of FR-64 cut this snapshot down to [`CF_UNICODETEXT`].
    truncated: bool,
    /// `GetClipboardSequenceNumber` as it stood when the snapshot was taken.
    sequence: u32,
}

impl Snapshot {
    /// An empty snapshot — what a restore of "the clipboard held nothing we could keep" is.
    ///
    /// Useful to T-07-2 as the value to start a pass with, so that its cleanup path always has
    /// something to restore from even if step 1 itself failed.
    pub fn empty() -> Self {
        Self {
            captured: Vec::new(),
            listed: 0,
            handle_formats: 0,
            refused: 0,
            bytes: 0,
            truncated: false,
            sequence: 0,
        }
    }

    /// The format numbers that were captured, in the order they will be restored.
    pub fn formats(&self) -> impl Iterator<Item = u32> + '_ {
        self.captured.iter().map(|entry| entry.format)
    }

    /// Whether `format` was captured.
    pub fn contains(&self, format: u32) -> bool {
        self.captured.iter().any(|entry| entry.format == format)
    }

    /// How many formats were captured.
    pub fn captured_formats(&self) -> usize {
        self.captured.len()
    }

    /// How many formats the clipboard listed when the snapshot was taken.
    pub fn listed_formats(&self) -> usize {
        self.listed
    }

    /// How many listed formats are not memory blocks and were therefore not captured.
    pub fn handle_formats(&self) -> usize {
        self.handle_formats
    }

    /// How many listed formats refused to be read — delayed rendering, mostly.
    pub fn refused_formats(&self) -> usize {
        self.refused
    }

    /// The total size of the captured formats.
    pub fn total_bytes(&self) -> usize {
        self.bytes
    }

    /// Whether the budget of FR-64 was exceeded and only [`CF_UNICODETEXT`] was kept.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Nothing was captured.
    pub fn is_empty(&self) -> bool {
        self.captured.is_empty()
    }

    /// The clipboard sequence number at the moment of the snapshot.
    ///
    /// This is the baseline step 3 of FR-61 waits away from: take the snapshot, remember this,
    /// send `Ctrl+C`, and ask [`wait_for_change`] whether the number moved.
    pub fn sequence(&self) -> u32 {
        self.sequence
    }
}

impl fmt::Debug for Snapshot {
    /// **SEC-01, SEC-07.** Hand-written, and this is the reason: the derived implementation
    /// would print `captured`, and `captured` is the user's clipboard. Six counts and a flag say
    /// everything a diagnosis needs and nothing a person's data would be recognisable in.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("captured_formats", &self.captured.len())
            .field("listed_formats", &self.listed)
            .field("handle_formats", &self.handle_formats)
            .field("refused_formats", &self.refused)
            .field("total_bytes", &self.bytes)
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl Drop for Snapshot {
    /// Overwrites the captured bytes before the allocator gets them back.
    ///
    /// The same treatment [`crate::buffer`] gives the typing buffer for SEC-02, and for the same
    /// reason: a freed heap block is readable by whatever allocates next, and the content of
    /// somebody's clipboard is precisely what SEC-07 names. `fill` writes through the slice, so
    /// the compiler cannot drop the stores as dead: the vector is read by nobody afterwards, but
    /// it is not a local whose address never escaped.
    fn drop(&mut self) {
        for entry in &mut self.captured {
            entry.bytes.fill(0);
        }
    }
}

/// Takes a snapshot of every format on the clipboard — FR-64, step 1 of FR-61.
///
/// # The budget, and why it is measured before anything is copied
///
/// FR-64 gives all formats together four megabytes. The sizes are known before the copying
/// starts — `GlobalSize` reads a header and copies nothing — so this walks the clipboard once
/// collecting handles and sizes, adds them up, and only then decides:
///
/// * within budget — every memory-backed format is copied;
/// * over budget — **only** [`CF_UNICODETEXT`] is copied, whatever its own size, because the
///   requirement says «сохраняется только `CF_UNICODETEXT`» and the point of the rule is that
///   the text survives. The fact is counted and put in the journal (SEC-07 permits it: it is an
///   event of the program, not a byte of the content).
///
/// One `OpenClipboard` covers the whole of it. The handles `GetClipboardData` answers are
/// borrowed from the clipboard and are valid only while it stays open, which is another reason
/// the two passes are inside one access rather than two.
///
/// # Formats that are not captured, and why that is not a failure
///
/// **Delayed rendering.** An application may put a *promise* on the clipboard instead of data —
/// `SetClipboardData(format, NULL)` — and the data is produced by its window only when somebody
/// asks. `GetClipboardData` then calls back into that application, and if it has closed, or
/// declines, the answer is null. There is no way to keep such a format: the bytes do not exist
/// anywhere to be copied, and the only party that could produce them is gone. Each one is
/// counted and skipped, **and the walk continues** — acceptance point 12: one format's refusal
/// may not cost the others their restore.
///
/// **Handle formats.** See [`HANDLE_FORMATS`] for why bitmaps, palettes and metafiles are left
/// alone and what the system's format synthesis gives back for free.
pub fn snapshot(owner: HWND) -> Result<Snapshot, ClipboardError> {
    let clipboard = Clipboard::open(owner)?;

    let listed = clipboard.formats()?;

    // Pass one: what is there, how big it is, and what cannot be taken. No bytes move here.
    let mut sizes: Vec<(u32, HGLOBAL, usize)> = Vec::with_capacity(listed.len());
    let mut handle_formats = 0_usize;
    let mut refused = 0_usize;
    let mut total = 0_usize;

    for &format in &listed {
        if !is_memory_format(format) {
            handle_formats += 1;
            HANDLE_FORMATS_SEEN.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let Some(handle) = clipboard.block(format) else {
            refused += 1;
            REFUSED_FORMATS.fetch_add(1, Ordering::Relaxed);
            continue;
        };

        // SAFETY: `handle` came from `GetClipboardData` for a format this module classified as
        // memory-backed, and the clipboard is still open, so the block is alive. `GlobalSize`
        // reads the block's header and copies nothing.
        let size = unsafe { GlobalSize(handle) };

        if size == 0 {
            // Either the handle is not a global block after all — a format outside the table
            // that behaves like one — or the block is empty. Both are nothing to restore.
            refused += 1;
            REFUSED_FORMATS.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        total = total.saturating_add(size);
        sizes.push((format, handle, size));
    }

    // The decision of FR-64, taken once, on the sum.
    let truncated = total > SNAPSHOT_BUDGET_BYTES;

    if truncated {
        TRUNCATIONS.fetch_add(1, Ordering::Relaxed);
        note_truncation();
    }

    // Pass two: the copying.
    let mut captured = Vec::with_capacity(sizes.len());
    let mut bytes = 0_usize;

    for (format, handle, size) in sizes {
        if truncated && format != CF_UNICODETEXT {
            continue;
        }

        match read_block(handle, size) {
            Some(block) => {
                bytes += block.len();
                captured.push(Captured {
                    format,
                    bytes: block,
                });
            }
            None => {
                refused += 1;
                REFUSED_FORMATS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    // Read after the copying and while the clipboard is still ours, so that the baseline step 3
    // of FR-61 compares against is the state the snapshot actually describes.
    let sequence = sequence_number();

    drop(clipboard);

    Ok(Snapshot {
        captured,
        listed: listed.len(),
        handle_formats,
        refused,
        bytes,
        truncated,
        sequence,
    })
}

/// Copies `size` bytes out of a clipboard-owned block, or answers `None` if it will not lock.
fn read_block(handle: HGLOBAL, size: usize) -> Option<Vec<u8>> {
    // SAFETY: `handle` is a live block borrowed from the open clipboard; `GlobalLock` pins it
    // and answers a pointer to at least `GlobalSize` bytes, which is what `size` is.
    let pointer = unsafe { GlobalLock(handle) };

    // NFR-13: a null pointer is the documented failure, and it is an ordinary one here — a
    // discarded block answers null.
    if pointer.is_null() {
        return None;
    }

    let mut bytes = vec![0_u8; size];

    // SAFETY: `pointer` is non-null and points at `size` readable bytes — `size` came from
    // `GlobalSize` on this very handle a few lines earlier, under the same clipboard lock, so
    // the block has neither moved nor changed size. The destination is a fresh `Vec` of exactly
    // `size` bytes; the two regions are distinct allocations and cannot overlap.
    unsafe {
        core::ptr::copy_nonoverlapping(pointer.cast::<u8>(), bytes.as_mut_ptr(), size);
    }

    // NFR-13: examined through the helper, which knows what a `FALSE` from `GlobalUnlock`
    // means. A block that would not unlock is still a block whose bytes are already copied, so
    // the failure costs the caller nothing and is not escalated.
    let _unlocked = unlock(handle);

    Some(bytes)
}

/// Puts the journal entry FR-64 asks for when the budget is exceeded.
///
/// # SEC-07, and a deviation this task could not resolve on its own
///
/// «факт фиксируется в журнале» — the *fact*, and nothing else; the size of the clipboard and
/// its content stay out, which is why [`crate::diag::OsCode::NONE`] is passed rather than a
/// number built from the measurement.
///
/// Module `diag` (task T-06-4) keeps a **closed vocabulary**: `Operation::from_name` maps any
/// name that is not in its table onto `Operation::UNLISTED`, and that narrowing is the whole
/// mechanism SEC-07 rests on. The table has no clipboard row and `src\diag.rs` is outside the
/// file scope of this task, so this entry reaches the ring as `unlisted`. The fact is in the
/// journal — the entry exists, it is ordinal-stamped and it is dumped — but it is not named
/// there. The report of T-07-1 states this in full and proposes the row to the controller;
/// adding it is one line of `src\diag.rs` and a `Kind` for it.
fn note_truncation() {
    crate::diag::record(
        crate::diag::Operation::from_name("clipboard snapshot truncated"),
        crate::diag::OsCode::NONE,
    );
}

// ---------------------------------------------------------------------------------------
// FR-64 — the restore
// ---------------------------------------------------------------------------------------

/// Puts a snapshot back on the clipboard — FR-64, step 8 of FR-61.
///
/// Takes the snapshot **by reference** on purpose: step 8 of FR-61 has to run on the failure
/// paths of steps 4 to 7 as well, so the value has to be reachable from a cleanup that does not
/// own it, and it has to survive being restored more than once.
///
/// # One format's refusal does not end the restore — acceptance point 12
///
/// The loop counts refusals and carries on. The alternative — returning at the first refusal —
/// would mean a clipboard left holding the two formats that happened to come before the awkward
/// one, which is a worse state than either the original or nothing.
///
/// # The write is marked as ours — FR-63
///
/// The sequence number the write produced is remembered, so that the `WM_CLIPBOARDUPDATE` it
/// raises is recognised by [`handle_clipboard_message`] as this program's own. See
/// [`note_own_write`].
pub fn restore(owner: HWND, snapshot: &Snapshot) -> Result<Restored, ClipboardError> {
    let clipboard = Clipboard::open(owner)?;

    clipboard.empty()?;

    let mut restored = Restored::default();

    for entry in &snapshot.captured {
        match clipboard.put(entry.format, &entry.bytes) {
            Ok(()) => restored.placed += 1,
            Err(_) => {
                // The error is dropped rather than reported: it belongs to one format of
                // somebody's clipboard, the other formats are still going up, and the count is
                // what the caller can act on. SEC-01, SEC-07 — a count, never a byte.
                restored.refused += 1;
            }
        }
    }

    drop(clipboard);

    note_own_write();

    Ok(restored)
}

/// Waits `delay`, then restores — step 8 of FR-61 with its own delay.
///
/// FR-61 step 8 says «восстановить исходное содержимое буфера обмена **с задержкой** (по
/// умолчанию 200 мс)», and the delay is there because step 6 has just asked another application
/// to paste: a restore that beat the paste would hand it the wrong bytes. The value comes from
/// `[selection] clipboard_restore_delay_ms` of section 7 — see [`restore_delay_of`] — and is
/// never written down here.
///
/// The sleep is inside this module so that it is inside the guard: [`Clipboard::open`] refuses a
/// thread that may not block, and a caller that slept first and called second would have done
/// the dangerous half on the wrong thread before finding out.
pub fn restore_after(
    owner: HWND,
    snapshot: &Snapshot,
    delay: Duration,
) -> Result<Restored, ClipboardError> {
    require_blocking_thread()?;

    sleep(delay);

    restore(owner, snapshot)
}

// ---------------------------------------------------------------------------------------
// Reading and writing text — steps 4 and 6 of FR-61 as primitives
// ---------------------------------------------------------------------------------------

/// Reads `CF_UNICODETEXT`, or `None` when the clipboard has no text — step 4 of FR-61.
///
/// The block is UTF-16 terminated by a null; the terminator is dropped and everything after it
/// is ignored, which is what every clipboard reader does and what a writer that padded the block
/// expects.
///
/// SEC-01, SEC-07: the answer is returned to the caller and is not logged, formatted or
/// journalled anywhere on the way.
pub fn read_unicode_text(owner: HWND) -> Result<Option<String>, ClipboardError> {
    let clipboard = Clipboard::open(owner)?;

    let Some(handle) = clipboard.block(CF_UNICODETEXT) else {
        return Ok(None);
    };

    // SAFETY: `handle` is a live block borrowed from the open clipboard.
    let size = unsafe { GlobalSize(handle) };

    let Some(bytes) = read_block(handle, size) else {
        return Ok(None);
    };

    Ok(Some(decode_utf16(&bytes)))
}

/// Turns the bytes of a `CF_UNICODETEXT` block into a string.
///
/// A pure function, so that the awkward parts — an odd number of bytes, a missing terminator, an
/// unpaired surrogate — are driven by unit tests rather than by whatever happens to be on the
/// clipboard. Lossy on purpose: a lone surrogate is somebody else's malformed data, and refusing
/// the whole clipboard over it would be the wrong trade.
fn decode_utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|&unit| unit != 0)
        .collect();

    String::from_utf16_lossy(&units)
}

/// Turns a string into the bytes of a `CF_UNICODETEXT` block, terminator included.
fn encode_utf16(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((text.len() + 1) * 2);

    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }

    bytes.extend_from_slice(&0_u16.to_le_bytes());

    bytes
}

/// Replaces the clipboard with `text` as `CF_UNICODETEXT` — step 6 of FR-61.
///
/// **This is a write to something that belongs to the user**, and it is only ever correct after
/// a [`snapshot`] has been taken and with a [`restore`] arranged for afterwards. The signature
/// cannot enforce that; the report of T-07-1 says it in words and T-07-2 is the single caller.
///
/// The write is marked as ours — FR-63, see [`note_own_write`].
pub fn write_unicode_text(owner: HWND, text: &str) -> Result<(), ClipboardError> {
    let bytes = encode_utf16(text);

    let clipboard = Clipboard::open(owner)?;

    clipboard.empty()?;
    clipboard.put(CF_UNICODETEXT, &bytes)?;

    drop(clipboard);

    note_own_write();

    Ok(())
}

// ---------------------------------------------------------------------------------------
// The sequence number — step 3 of FR-61, and the "is it ours" of FR-63
// ---------------------------------------------------------------------------------------

/// `GetClipboardSequenceNumber`, which needs no clipboard lock and cannot block.
///
/// Zero means the calling process has no `WINSTA_ACCESSCLIPBOARD` access, which does not happen
/// in an interactive session; it is passed on rather than turned into an error, because a caller
/// comparing two zeroes learns the same thing it would learn from two equal numbers — nothing
/// changed.
pub fn sequence_number() -> u32 {
    // SAFETY: takes no arguments, dereferences nothing, and reads a counter the window station
    // keeps. Safe to call from any thread at any time, with or without the clipboard open.
    unsafe { GetClipboardSequenceNumber() }
}

/// Waits for the clipboard sequence number to leave `baseline` — **step 3 of FR-61**.
///
/// `timeout` comes from `[selection] clipboard_timeout_ms` of section 7 — see [`timeout_of`] —
/// and is not written down here.
///
/// # Why this is a wait and not a question
///
/// There is no call that answers "does the foreground window have a selection". FR-61 gets the
/// answer indirectly: send `Ctrl+C`, and see whether the clipboard changed. A change means there
/// was a selection; no change within the timeout means there was not, and FR-60 then sends the
/// hotkey down the typing-buffer path. [`Wait::TimedOut`] is therefore an **ordinary answer**,
/// not an error, and the two are different types here rather than the same one.
///
/// # Where it may run — FR-80, NFR-01
///
/// Not on the thread that owns the hook. See the module documentation; [`Wait::WrongThread`] is
/// the refusal, and it is counted.
pub fn wait_for_change(baseline: u32, timeout: Duration) -> Wait {
    if !caller_may_block() {
        WRONG_THREAD_REFUSALS.fetch_add(1, Ordering::Relaxed);

        return Wait::WrongThread;
    }

    let started = Instant::now();

    loop {
        let current = sequence_number();

        if current != baseline {
            return Wait::Changed {
                sequence: current,
                waited: started.elapsed(),
            };
        }

        let waited = started.elapsed();

        let Some(left) = timeout.checked_sub(waited) else {
            return Wait::TimedOut { waited };
        };

        if left.is_zero() {
            return Wait::TimedOut { waited };
        }

        sleep(POLL_INTERVAL.min(left));
    }
}

/// Remembers the sequence number this process's own write just produced — FR-63.
///
/// # Why the sequence number and not a marker format
///
/// The other way of recognising one's own clipboard writes is to put a private format on the
/// clipboard alongside the data and look for it. That is out of the question here for the reason
/// the whole module exists: step 8 of FR-61 restores the user's clipboard **exactly**, and a
/// marker in it would be a byte of ours left in something that is not ours. The sequence number
/// is outside the clipboard's content entirely, and FR-61 already names it.
///
/// # What it can and cannot tell apart
///
/// `WM_CLIPBOARDUPDATE` says *that* the clipboard changed, never *which* change it was; the
/// handler can only ask what the sequence number is **now**. So the recognition is exact when
/// our message is taken before the next change and inexact when it is not: if a foreign write
/// lands between our write and our reading of the queue, both changes look foreign. That
/// mis-classification is one-directional and it is the safe direction — a change of ours read as
/// the user's makes the program consider a clipboard it may treat as new, whereas the reverse
/// would make it ignore a real user action. The window in which it can happen is the few
/// microseconds between `CloseClipboard` and the next `GetMessage`.
pub fn note_own_write() {
    let sequence = sequence_number();
    let index = OWN_NEXT.fetch_add(1, Ordering::Relaxed) % OWN_MARKS;

    OWN_MARK[index].store(sequence, Ordering::Release);
}

/// Whether `sequence` is one this process produced — FR-63.
pub fn is_own_change(sequence: u32) -> bool {
    // Zero is the "never written" value of the ring and also what `GetClipboardSequenceNumber`
    // answers without clipboard access. Neither is a change of ours.
    sequence != 0
        && OWN_MARK
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) == sequence)
}

// ---------------------------------------------------------------------------------------
// Section 7 — the two numbers, read from the configuration and not from here
// ---------------------------------------------------------------------------------------

/// `[selection] clipboard_timeout_ms` as a duration — step 3 of FR-61.
///
/// Acceptance point 18: the three hundred milliseconds are the *default of section 7* and live
/// in `src\settings.rs`; this module reads whatever the configuration carries and has no number
/// of its own to fall back on.
pub fn timeout_of(selection: &Selection) -> Duration {
    Duration::from_millis(u64::from(selection.clipboard_timeout_ms))
}

/// `[selection] clipboard_restore_delay_ms` as a duration — step 8 of FR-61.
pub fn restore_delay_of(selection: &Selection) -> Duration {
    Duration::from_millis(u64::from(selection.clipboard_restore_delay_ms))
}

/// `[selection] enabled` — FR-65, so that T-07-2 reads the flag through the module that owns it.
pub fn is_enabled(selection: &Selection) -> bool {
    selection.enabled
}

// ---------------------------------------------------------------------------------------
// FR-63 — the listener
// ---------------------------------------------------------------------------------------

/// The clipboard format listener, withdrawn when this value is dropped — FR-63.
///
/// `RemoveClipboardFormatListener` is not a courtesy: the registration is a slot the window
/// station keeps against a window handle, and a program that took one and did not give it back
/// leaves the system delivering updates to a handle that will be reused. `Drop` is what makes
/// the withdrawal unconditional, and `app::serve_window` declares this guard **after** the
/// window it names so that drop order takes it first.
pub struct Listener {
    /// The window the registration names, so that the withdrawal names the same one.
    window: HWND,
    /// Neither `Send` nor `Sync`: the registration is tied to a window of the registering
    /// thread, and that thread is also the one this module publishes as allowed to block.
    _not_send: PhantomData<*const ()>,
}

/// Registers `window` for `WM_CLIPBOARDUPDATE` — FR-63.
///
/// # Which window, and why it is allowed to be a hidden one
///
/// `AddClipboardFormatListener` **posts to the window named here**; it is not a broadcast, so
/// it is not subject to the rule that kept FR-81 off `HWND_MESSAGE` (decision R-20 point 2). A
/// message-only window would do. What the window does have to have is a thread that pumps its
/// messages, and all three windows of this process have one.
///
/// `app` calls this on the **UI thread**, which is also the thread the blocking primitives are
/// then confined to; see the module documentation for the choice between the three threads of
/// section 6.1. Registering here is what publishes that choice: this function records the
/// calling thread as the one [`caller_may_block`] will accept.
pub fn listen(window: HWND) -> WinResult<Listener> {
    // SAFETY: `window` is the caller's live window, passed by value. The call adds the handle to
    // the window station's listener list and dereferences nothing of ours. NFR-13: the binding
    // turns the `FALSE` return into `Err`. The guard returned here is dropped before the window
    // is destroyed, because `app::serve_window` declares it after the window.
    unsafe { AddClipboardFormatListener(window) }?;

    // Published only after the registration is known good — the order `watchdog` uses for the
    // same kind of pairing.
    LISTENER_WINDOW.store(window.0 as usize, Ordering::Release);
    WORKER_THREAD.store(current_thread_id(), Ordering::Release);

    Ok(Listener {
        window,
        _not_send: PhantomData,
    })
}

impl Drop for Listener {
    fn drop(&mut self) {
        // Unpublished first, so that no message is answered on a registration that is going and
        // no work is admitted on a thread that is about to stop pumping.
        LISTENER_WINDOW.store(NO_WINDOW, Ordering::Release);
        WORKER_THREAD.store(NO_THREAD, Ordering::Release);

        // SAFETY: `self.window` is the handle a successful `AddClipboardFormatListener` was
        // given, and this type is neither `Copy` nor `Clone`, so the withdrawal happens exactly
        // once. The window is still alive: `app::serve_window` declares this guard after the
        // window and drop order takes it first.
        if let Err(error) = unsafe { RemoveClipboardFormatListener(self.window) } {
            // NFR-13: examined. The system drops the registration when the window is destroyed
            // in any case, so there is nothing further to do.
            LISTENER_REMOVE_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::app::report_non_critical("RemoveClipboardFormatListener", &error);
        }
    }
}

/// Answers `WM_CLIPBOARDUPDATE` at the registered window — FR-63.
///
/// Called from `app::window_proc` for every message of every window of the process; answers
/// `None` for everything that is not a clipboard update at the window [`listen`] registered.
///
/// # SEC-05
///
/// A process at the same integrity level can post `WM_CLIPBOARDUPDATE` to any of this program's
/// windows. The `hwnd` test below confines the effect to the registered window, and what it buys
/// a sender even there is one read of a system counter and one comparison against a ring of
/// numbers this process wrote. Nothing is opened, nothing is written and nothing is decided: the
/// classification is *returned*, and the decision belongs to T-07-2.
pub fn handle_clipboard_message(hwnd: HWND, message: u32) -> Option<Update> {
    if message != WM_CLIPBOARDUPDATE {
        return None;
    }

    if hwnd.0 as usize != LISTENER_WINDOW.load(Ordering::Acquire) {
        return None;
    }

    let sequence = sequence_number();

    let origin = if is_own_change(sequence) {
        OWN_UPDATES.fetch_add(1, Ordering::Relaxed);
        Origin::Own
    } else {
        FOREIGN_UPDATES.fetch_add(1, Ordering::Relaxed);
        Origin::Foreign
    };

    UPDATES.fetch_add(1, Ordering::Relaxed);

    Some(Update { sequence, origin })
}

/// The window the listener is registered on, or zero. For the tests and for `app`.
pub fn listener_window_raw() -> usize {
    LISTENER_WINDOW.load(Ordering::Acquire)
}

/// The thread the blocking primitives are confined to, or zero. For the tests.
pub fn worker_thread_id() -> u32 {
    WORKER_THREAD.load(Ordering::Acquire)
}

// ---------------------------------------------------------------------------------------
// Unit tests — the parts that are functions of their arguments
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_retry_numbers_are_the_ones_fr62_writes() {
        assert_eq!(OPEN_ATTEMPTS, 10);
        assert_eq!(OPEN_RETRY_INTERVAL, Duration::from_millis(20));
        // Ten attempts have nine gaps between them.
        assert_eq!(WORST_CASE_OPEN, Duration::from_millis(180));
    }

    #[test]
    fn the_budget_is_the_four_megabytes_of_fr64() {
        assert_eq!(SNAPSHOT_BUDGET_BYTES, 4 * 1024 * 1024);
        assert_eq!(SNAPSHOT_BUDGET_BYTES, 4_194_304);
    }

    #[test]
    fn the_handle_formats_are_the_ones_that_are_not_memory() {
        // Bitmaps, palettes and metafiles, in both their ordinary and their display-format
        // spellings, plus the owner-display format that carries no data at all.
        for format in [2, 3, 9, 14, 128, 130, 131, 142] {
            assert!(
                !is_memory_format(format),
                "format {format} must not be treated as memory"
            );
        }
    }

    #[test]
    fn the_text_and_data_formats_are_memory() {
        // CF_TEXT, CF_OEMTEXT, CF_DIB, CF_DIBV5, CF_LOCALE, CF_HDROP, CF_UNICODETEXT and a
        // registered format such as "HTML Format" or "Rich Text Format".
        for format in [1, 7, 8, 17, 16, 15, CF_UNICODETEXT, 0xC0F1] {
            assert!(
                is_memory_format(format),
                "format {format} must be treated as memory"
            );
        }
    }

    #[test]
    fn the_private_and_gdi_ranges_are_not_memory() {
        for format in [
            PRIVATE_FIRST,
            PRIVATE_FIRST + 1,
            PRIVATE_LAST,
            GDIOBJ_FIRST,
            GDIOBJ_LAST,
        ] {
            assert!(!is_memory_format(format));
        }

        // And the neighbours on either side of the two ranges are ordinary again.
        assert!(is_memory_format(PRIVATE_FIRST - 1));
        assert!(is_memory_format(GDIOBJ_LAST + 1));
    }

    #[test]
    fn text_survives_a_round_trip_through_the_block_encoding() {
        for text in ["", "ghbdtn", "привет", "a\u{0}b", "🙂 mixed привет 123"] {
            let bytes = encode_utf16(text);

            // The block is terminated, which is what every clipboard reader expects.
            assert_eq!(&bytes[bytes.len() - 2..], &[0, 0]);

            let back = decode_utf16(&bytes);

            // `a\0b` is the one that does not round-trip, and deliberately: the null is the
            // terminator of the format, so everything after it is not part of the text.
            let expected = text.split('\u{0}').next().unwrap_or_default();

            assert_eq!(back, expected, "round trip of {text:?}");
        }
    }

    #[test]
    fn an_odd_or_truncated_block_decodes_instead_of_panicking() {
        // Three bytes: one complete UTF-16 unit and a stray one. `chunks_exact` drops the tail.
        assert_eq!(decode_utf16(&[0x41, 0x00, 0x42]), "A");
        // No terminator at all.
        assert_eq!(decode_utf16(&[0x41, 0x00, 0x42, 0x00]), "AB");
        // Nothing at all.
        assert_eq!(decode_utf16(&[]), "");
        // An unpaired surrogate is replaced rather than refused.
        assert_eq!(decode_utf16(&[0x00, 0xD8, 0x00, 0x00]), "\u{FFFD}");
    }

    #[test]
    fn the_two_timings_come_out_of_section_seven_and_not_out_of_this_module() {
        let default = Selection::default();

        assert_eq!(timeout_of(&default), Duration::from_millis(300));
        assert_eq!(restore_delay_of(&default), Duration::from_millis(200));
        assert!(is_enabled(&default));

        // And they follow the configuration rather than a constant of this module.
        let configured = Selection {
            enabled: false,
            clipboard_timeout_ms: 45,
            clipboard_restore_delay_ms: 900,
        };

        assert_eq!(timeout_of(&configured), Duration::from_millis(45));
        assert_eq!(restore_delay_of(&configured), Duration::from_millis(900));
        assert!(!is_enabled(&configured));
    }

    #[test]
    fn the_debug_of_a_snapshot_prints_counts_and_no_content() {
        let snapshot = Snapshot {
            captured: vec![Captured {
                format: CF_UNICODETEXT,
                bytes: b"SECRET-CLIPBOARD-CONTENT".to_vec(),
            }],
            listed: 3,
            handle_formats: 1,
            refused: 1,
            bytes: 24,
            truncated: true,
            sequence: 77,
        };

        let printed = format!("{snapshot:?}");

        // SEC-01, SEC-07: the bytes are not there under any spelling.
        assert!(!printed.contains("SECRET"));
        assert!(!printed.contains("83")); // the first byte of the payload, as a decimal
        assert!(printed.contains("captured_formats: 1"));
        assert!(printed.contains("total_bytes: 24"));
        assert!(printed.contains("truncated: true"));
    }

    #[test]
    fn an_empty_snapshot_answers_the_questions_a_caller_asks_of_one() {
        let snapshot = Snapshot::empty();

        assert!(snapshot.is_empty());
        assert!(!snapshot.is_truncated());
        assert_eq!(snapshot.captured_formats(), 0);
        assert_eq!(snapshot.listed_formats(), 0);
        assert_eq!(snapshot.total_bytes(), 0);
        assert_eq!(snapshot.sequence(), 0);
        assert!(!snapshot.contains(CF_UNICODETEXT));
        assert_eq!(snapshot.formats().count(), 0);
    }

    #[test]
    fn a_write_of_ours_is_recognised_and_a_number_nobody_wrote_is_not() {
        // The ring is process-global, so this test picks numbers no clipboard produces.
        let mine = u32::MAX - 12;
        let index = OWN_NEXT.fetch_add(1, Ordering::Relaxed) % OWN_MARKS;
        OWN_MARK[index].store(mine, Ordering::Release);

        assert!(is_own_change(mine));
        assert!(!is_own_change(u32::MAX - 13));
        // Zero is never ours: it is the empty value of the ring.
        assert!(!is_own_change(0));
    }

    #[test]
    fn the_ring_of_own_marks_holds_a_whole_pass_and_more() {
        // Two writes per pass of FR-61 — step 6 and step 8 — so eight slots are four passes.
        assert_eq!(OWN_MARKS, 8);
        assert_eq!(OWN_MARK.len(), OWN_MARKS);
    }

    #[test]
    fn a_message_that_is_not_a_clipboard_update_is_not_answered() {
        // Every other message of the process, at any window: the module must answer `None` so
        // that `app::window_proc` passes it on unchanged.
        for message in [0_u32, 1, WM_CLIPBOARDUPDATE - 1, WM_CLIPBOARDUPDATE + 1] {
            assert_eq!(
                handle_clipboard_message(HWND(core::ptr::null_mut()), message),
                None
            );
        }
    }

    #[test]
    fn a_clipboard_update_at_a_window_that_is_not_the_listener_is_not_answered() {
        // SEC-05: the registered window is zero in a test process, and a message claiming to be
        // at some other window is ignored rather than counted.
        let before = counters().updates;

        // A handle value that is not the registered window and is not null. It is never
        // dereferenced — `handle_clipboard_message` compares it and nothing more.
        let elsewhere = HWND(core::ptr::dangling_mut::<core::ffi::c_void>());

        assert_eq!(
            handle_clipboard_message(elsewhere, WM_CLIPBOARDUPDATE),
            None
        );

        assert_eq!(counters().updates, before);
    }

    #[test]
    fn the_display_of_an_error_names_the_kind_and_carries_no_content() {
        assert!(
            ClipboardError::WrongThread
                .to_string()
                .contains("may not block")
        );
        assert!(ClipboardError::Busy.to_string().contains("another process"));

        let os = ClipboardError::Os(WinError::from_hresult(
            windows::Win32::Foundation::E_INVALIDARG,
        ));

        assert!(os.to_string().contains("0x80070057"));
    }

    #[test]
    fn the_poll_interval_divides_the_timeout_of_section_seven_into_useful_pieces() {
        let default = timeout_of(&Selection::default());

        assert!(POLL_INTERVAL < default);
        assert_eq!(default.as_millis() / POLL_INTERVAL.as_millis(), 60);
    }

    // -------------------------------------------------------------------------------------
    // The close on the panicking path — FR-62, acceptance point 14
    //
    // These two need a real clipboard and a real window, which puts them at the edge of what
    // belongs in a unit test. They are here rather than in `tests\selection.rs` for one reason:
    // [`Clipboard`] is private, deliberately — it is the mechanism that makes forgetting the
    // close impossible — and a test of what its `Drop` does has to be inside the module.
    //
    // ⚠ **Neither of them writes to the clipboard.** They open it and close it again, which
    // changes nothing the user can observe and does not move the sequence number.
    // -------------------------------------------------------------------------------------

    /// Serialises the tests below, which read process-global counters around a system-global
    /// lock. `cargo test` runs the tests of a binary in parallel and there is one clipboard.
    static ACCESS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A message-only window of a system class, destroyed when the value is dropped.
    ///
    /// `STATIC` is a predefined class, so no registration and no window procedure of ours is
    /// needed: `OpenClipboard` wants a window handle of the calling thread and nothing more.
    struct TestWindow(HWND);

    impl TestWindow {
        fn create() -> Self {
            use windows::Win32::UI::WindowsAndMessaging::{
                CreateWindowExW, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
            };
            use windows::core::w;

            // SAFETY: `STATIC` is a predefined window class that is always registered, the
            // window name is a `'static` literal and the parent is `HWND_MESSAGE`, which asks
            // for a message-only window. No `lpParam` is passed, so nothing of ours reaches the
            // class's own window procedure. NFR-13: the binding turns a null handle into `Err`.
            let handle = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("langsw-selection-test"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    None,
                    None,
                )
            }
            .expect("a message-only window of the STATIC class");

            Self(handle)
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

            // SAFETY: `self.0` came from a successful `CreateWindowExW` on this thread and is
            // destroyed once — the type is neither `Copy` nor `Clone`.
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }

    #[test]
    fn an_ordinary_access_opens_and_closes_exactly_once() {
        let _serialised = ACCESS.lock().unwrap_or_else(|poison| poison.into_inner());

        let window = TestWindow::create();
        let before = counters();

        {
            let _clipboard = Clipboard::open(window.0).expect("the clipboard opens");
        }

        let after = counters();

        assert_eq!(after.opens, before.opens + 1);
        assert_eq!(after.closes, before.closes + 1);
        assert_eq!(after.close_failures, before.close_failures);
    }

    #[test]
    fn the_clipboard_is_closed_even_when_the_frame_panics() {
        let _serialised = ACCESS.lock().unwrap_or_else(|poison| poison.into_inner());

        let window = TestWindow::create();
        let before = counters();

        // The default hook prints the panic and a backtrace note, which is noise in a test that
        // means to panic. Replaced for the duration and put back afterwards, under the same
        // mutex that serialises everything else here.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let handle = window.0;

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _clipboard = Clipboard::open(handle).expect("the clipboard opens");

            panic!("a panic with the clipboard open");
        }));

        std::panic::set_hook(previous);

        assert!(outcome.is_err(), "the frame must have panicked");

        let after = counters();

        // **Acceptance point 14.** The guard was dropped by the unwinder, so the close happened
        // although nothing in the frame asked for it. This is the Debug configuration, where
        // panics unwind; `panic = "abort"` belongs to `[profile.release]`, and there the process
        // ends and the system releases the clipboard with the thread — measured on the running
        // program, point 26 of the task specification.
        assert_eq!(after.opens, before.opens + 1);
        assert_eq!(after.closes, before.closes + 1);
        assert_eq!(after.close_failures, before.close_failures);

        // And the proof that the close really happened: the clipboard opens again at once. A
        // clipboard left locked by this process would refuse for the whole 180 ms of FR-62 and
        // then answer `Busy`.
        let reopened = Clipboard::open(window.0);

        assert!(reopened.is_ok(), "the clipboard is not left locked");
    }
}
