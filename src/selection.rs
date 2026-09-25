//! The selection path: working with the clipboard, saving and restoring it.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: **FR-62** (retries around every clipboard access),
//! **FR-63** (`AddClipboardFormatListener` and telling our own changes from the user's) and
//! **FR-64** (the snapshot and the restore of every listed format under one four-megabyte
//! budget) — task **T-07-1**, together with the sequence-number wait step 3 of FR-61 asks for;
//! **FR-60** (which of the two sources a press converts), **FR-61** (the eight steps, whole) and
//! **FR-65** (the switch of section 7) — task **T-07-2**, built on top of those primitives.
//! Implemented by backlog tasks: T-07-1 (done), T-07-2 (done).
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
//! That sentence has **one exception** and had **one hole**, and task **T-13-11** — the audit of
//! 2026-08-24 — is where both were settled.
//!
//! The exception is **decision П-5**, written into the note to step 8 of §4.7: the snapshot is
//! *not* put back when another process changed the clipboard after this program wrote its result.
//! "Comes back" may not mean writing over a copy the user made inside the two hundred
//! milliseconds step 8 waits — that would be the same loss with the program on the other side of
//! it. [`restore_after`] asks the question and [`restore_is_due`] answers it.
//!
//! ⚠ **The exception ate the rule for a year, and task Т-18-1 — the audit of 2026-08-31 — is
//! where that was settled.** The answer used to be [`is_own_change`] and nothing else, and
//! [`is_own_change`] knows only the writes **this program** made. On every failure path of steps
//! 4 to 6 the program has written nothing: the clipboard holds the selection because the
//! *application* put it there in answer to the probe of step 2. So the ring was silent, the gate
//! read that silence as «somebody else copied something», and the snapshot — by then the only
//! copy of the user's clipboard left anywhere — was dropped and the loss journalled as a lawful
//! П-5 skip. A file selected in Explorer, a picture, a text with no direction, a refused write:
//! any of them and the user's clipboard was gone for good.
//!
//! П-5 is a shield against a **third** writer, and the answer to our own probe is not one. So the
//! gate is now told that number too — it travels from [`Wait::Changed`] through [`Owed`] to
//! [`Path::restore_clipboard`] and into [`restore_after`], as a parameter and never through the
//! ring, because putting it in the ring would make [`handle_clipboard_message`] call an
//! application's write this program's own and the counters of FR-63 would lie.
//!
//! The hole was step 3 timing out. `Ctrl+C` had gone out, the clipboard had not moved *yet*, and
//! the snapshot was dropped unread — so an application that answered the probe late left the user
//! with a clipboard nobody was going to put back. [`run`] now asks the sequence number once more
//! before it gives up, and [`Path::reclaim_clipboard`] is the restore that follows.
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
//!   [`crate::buffer`] gives the typing buffer for SEC-02 — literally the same write, since task
//!   **T-13-15**: `crate::buffer::zero_slice`, which is `write_volatile` plus a
//!   `compiler_fence`, and not the `fill(0)` it used to be;
//! * nothing in this module panics with a payload built from clipboard data — nothing in this
//!   module panics at all, which the source sweep in `tests\selection.rs` checks;
//! * the journal is reached with [`crate::diag::Operation`] values and `HRESULT`s only, never
//!   with text.
//!
//! # Which buffers are zeroed, and with what — SEC-01, SEC-02, task T-13-15
//!
//! The audit of 2026-08-24 found this module promising more than it did, in two ways, and both
//! are settled here rather than softened.
//!
//! **The form of the write.** [`Snapshot`], [`Recoded`] and [`recode`] zeroed their buffers with
//! `slice.fill(0)` immediately before releasing them. That is the store module `buffer` calls
//! deletable in its own documentation — «a plain assignment into a slot that is about to be
//! overwritten or dropped is a dead store the compiler is allowed to delete» — and under the
//! Release profile of section 3.2 (`opt-level 3`, fat LTO, `codegen-units 1`) a memset in front
//! of a free is what dead-store elimination exists to remove. All three now use
//! `crate::buffer::zero_slice`, the ring's own volatile write.
//!
//! **The buffers that were missed.** Three transit copies of the user's text were released
//! untouched: the block [`read_unicode_text`] takes off the clipboard, the `Vec<u16>` inside
//! `decode_utf16`, and the encoded block [`write_unicode_text`] puts on. All three are zeroed
//! now, the last of them on the failure paths as well. Note what this is and is not: SEC-02 of
//! SPEC is a rule about the **typing buffer**, and none of this was a breach of it. It is the
//! rule this module wrote for itself in the list above, kept in the places it was not being kept.
//!
//! **And the copy nobody was counting — finding С29 of the audit of 2026-09-04, task T-38-9A.**
//! The text step 4 decodes out of that block, the one copy steps 5 and 6 work from, was still a
//! bare `String` released as it stood, while the source sweep meant to catch such a thing counted
//! calls of the zeroing and found its seven. It is born inside [`ClipboardText`] now, which zeroes
//! it when it is dropped, and the sweep names every copy the module makes instead of counting.
//!
//! **And the copies a growth makes — finding Н34, task T-38-9B.** A `Vec<u16>` or a `String` that
//! grows copies the text into a new block and hands the old one back as it stands, beyond the
//! reach of any zeroing. The working buffers of steps 4 and 5 are made once, at a size counted
//! before they are filled; `tests\selection_growth.rs` counts the growths and wants none.

use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use core::time::Duration;
use std::sync::{Mutex, PoisonError};
use std::thread::sleep;
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats,
    GetClipboardData, GetClipboardOwner, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_C, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, MSG, PM_NOREMOVE, PeekMessageW, WM_APP,
    WM_CLIPBOARDUPDATE, WM_USER,
};
use windows::core::{Error as WinError, Free, PCWSTR, Result as WinResult, w};

use crate::convert::Keystroke;
use crate::hook::INJECTED_SIGNATURE;
use crate::inject::{Dispatched, Modifiers};
use crate::layouts::{Cycle, KeyPress, LayoutId, LayoutMap, Mods};
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
///
/// ⚠ Since task T-76-1 the loop of [`wait_for_change`] does let **sent** messages through between
/// these looks — see [`let_sent_messages_through`]. That is not a pump and changes nothing here:
/// `WM_CLIPBOARDUPDATE` is posted, not sent, and is still not dispatched during step 3.
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
// The privacy markers — finding С25, task T-38-6, decision 121.7
// ---------------------------------------------------------------------------------------

/// The three registered formats Windows reads to keep a clipboard out of its history, out of its
/// cloud and out of the sight of clipboard monitors — **finding С25 of the audit of 2026-09-04,
/// task T-38-6, decision 121.7**.
///
/// A password manager puts them beside a secret. `CanIncludeInClipboardHistory` and
/// `CanUploadToCloudClipboard` carry a `DWORD`, and zero is «no»; for
/// `ExcludeClipboardContentFromMonitorProcessing` any data will do — its presence is the message.
/// Measured 2026-09-10 (premise П2): **with** data all three already survived a snapshot and a
/// restore; **without** data — a promise of delayed rendering that nobody kept — [`snapshot`]
/// counted them refused, and step 8 put the secret back unmarked.
const PRIVACY_MARKERS: [PCWSTR; 3] = [
    w!("CanIncludeInClipboardHistory"),
    w!("CanUploadToCloudClipboard"),
    w!("ExcludeClipboardContentFromMonitorProcessing"),
];

/// The one form this program puts a privacy marker on the clipboard in: a `DWORD` of zero — «not
/// into the history», «not into the cloud», and for the third marker a presence, which is all it
/// needs. Decision 121.7: the restrictive reading of a marker that arrived without a value.
const PRIVACY_MARKER_DATA: [u8; 4] = [0; 4];

/// The format numbers of [`PRIVACY_MARKERS`] in this session — zero for a name the system would not
/// register, which then matches no format.
fn privacy_markers() -> [u32; 3] {
    // SAFETY: each name is a `'static` wide literal; the call registers or looks up a format name
    // in the window station's table and dereferences nothing of ours. Zero is the documented
    // failure, and no clipboard format is numbered zero, so a failed name matches nothing.
    PRIVACY_MARKERS.map(|name| unsafe { RegisterClipboardFormatW(name) })
}

/// Whether `format` is one of the three privacy markers — task T-38-6.
pub fn is_privacy_marker(format: u32) -> bool {
    format != 0 && privacy_markers().contains(&format)
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

/// Snapshots whose list of formats was cut short at [`MAX_FORMATS`] — **finding Н16 of the audit
/// of 2026-09-04, task T-38-7**. The neighbour of [`TRUNCATIONS`], and kept apart from it: that
/// one is the budget of FR-64 speaking, this one the bound of the enumeration — a clipboard that
/// listed more formats than this module walks, and whose tail the snapshot never saw.
static FORMAT_LIST_TRUNCATIONS: AtomicU32 = AtomicU32::new(0);

/// Snapshots of a clipboard that listed formats and kept none of them — **finding С11 of the audit
/// of 2026-09-04, task T-38-8, decision 121.2**. A metafile and nothing else, a clipboard over the
/// budget of FR-64 with no text, every delayed format refused: step 1 answered success, the
/// `Ctrl+C` of step 2 goes out regardless, and step 8 will have nothing to put back. The behaviour
/// is not changed; this is the trace of it, which no other count gives: [`REFUSED_FORMATS`] and
/// [`HANDLE_FORMATS_SEEN`] add up over every snapshot, and not one of them says a snapshot came out
/// empty.
static SNAPSHOTS_SAVED_NOTHING: AtomicU32 = AtomicU32::new(0);

/// Formats that were listed on the clipboard and could not be read out of it.
///
/// Delayed rendering, mostly: see [`snapshot`]. Counted rather than escalated, because FR-64
/// asks for the other formats to survive one format's refusal.
static REFUSED_FORMATS: AtomicU32 = AtomicU32::new(0);

/// Formats that were listed and are not memory blocks — see [`HANDLE_FORMATS`].
static HANDLE_FORMATS_SEEN: AtomicU32 = AtomicU32::new(0);

/// `RemoveClipboardFormatListener` calls that failed — FR-63.
static LISTENER_REMOVE_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Restores of step 8 that were **not** made — the note to step 8 of §4.7, decision П-5.
///
/// The clipboard had moved away from this program's own write while step 8 waited out its delay,
/// so the snapshot would have been written over somebody's new copy. Counted because a restore
/// that does not happen is a promise of this module deliberately not kept, and the one thing it
/// may not be is invisible — the same argument [`CLOSE_FAILURES`] is counted on.
static RESTORE_SKIPS: AtomicU32 = AtomicU32::new(0);

/// ⭐ **Task Т-22-6.** Restores that were **attempted and did not put the snapshot back**.
///
/// ⚠ **Not the same thing as [`RESTORE_SKIPS`], and deliberately not folded into it.** A П-5 skip
/// is a decision of this program: step 8 was due, the clipboard had moved on, and the snapshot was
/// withheld **on purpose** to protect somebody's new copy. This counter is the opposite — the
/// restore was due, was not withheld, and failed anyway. Counting the two together would let a
/// broken restore hide behind a lawful refusal, which is exactly the invisibility finding м6 is
/// about.
///
/// The three ways in, all inside [`restore`]:
///
/// * [`Clipboard::open`] refused — a thread that may not block, or ten attempts of FR-62 against
///   another process holding the lock;
/// * `EmptyClipboard` refused — and the clipboard is then still the user's, untouched;
/// * every `SetClipboardData` refused while the snapshot had something to place
///   ([`restore_placed_nothing`]) — the clipboard **was** emptied and nothing went back into it,
///   which is the worst outcome this module has and was the most silent.
static RESTORE_FAILURES: AtomicU32 = AtomicU32::new(0);

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
    /// Snapshots whose list of formats was cut short at [`MAX_FORMATS`] — finding Н16, task
    /// T-38-7. A count of the bound being reached, and nothing about what the formats held.
    pub format_list_truncations: u32,
    /// Snapshots of a clipboard that listed formats and kept none of them — finding С11, task
    /// T-38-8, see [`snapshot_saved_nothing`]. A count of snapshots of this program that leave
    /// step 8 nothing to put back, and nothing about what the formats held.
    pub snapshots_saved_nothing: u32,
    /// Listed formats that could not be read — delayed rendering, mostly.
    pub refused_formats: u32,
    /// Listed formats that are not memory blocks — see [`HANDLE_FORMATS`].
    pub handle_formats: u32,
    /// `RemoveClipboardFormatListener` calls that failed.
    pub listener_remove_failures: u32,
    /// Restores of step 8 skipped because the clipboard had moved on — the note to step 8 of
    /// §4.7, decision П-5. **A count of decisions of this program, not of anything in the
    /// clipboard**: it says how often somebody else's copy was left standing, and nothing about
    /// what either copy held.
    pub restore_skips: u32,
    /// ⭐ **Task Т-22-6.** Restores that were attempted and did not put the snapshot back — see
    /// [`RESTORE_FAILURES`] for the three ways in and for why this is **not** `restore_skips`.
    /// A count of this program's own failures, and nothing about anybody's clipboard.
    pub restore_failures: u32,
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
        format_list_truncations: FORMAT_LIST_TRUNCATIONS.load(Ordering::Relaxed),
        snapshots_saved_nothing: SNAPSHOTS_SAVED_NOTHING.load(Ordering::Relaxed),
        refused_formats: REFUSED_FORMATS.load(Ordering::Relaxed),
        handle_formats: HANDLE_FORMATS_SEEN.load(Ordering::Relaxed),
        listener_remove_failures: LISTENER_REMOVE_FAILURES.load(Ordering::Relaxed),
        restore_skips: RESTORE_SKIPS.load(Ordering::Relaxed),
        restore_failures: RESTORE_FAILURES.load(Ordering::Relaxed),
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

    /// The format numbers on the clipboard right now, in the system's own order, and whether the
    /// list goes on past [`MAX_FORMATS`].
    ///
    /// `EnumClipboardFormats` needs the clipboard open, which is why this is a method. It
    /// answers zero both at the end of the list and on failure, and the two are told apart the
    /// way the documentation prescribes — by the thread's last error, which the binding has
    /// already turned into an `Err` for us.
    ///
    /// # The bound, and the one question past it — finding Н16, task T-38-7
    ///
    /// The walk stops at [`MAX_FORMATS`]. Until task T-38-7 it then answered the same `Ok` the end
    /// of a list answers, so a snapshot of a clipboard that listed more was partial and could not
    /// say so. Now the list is asked once more: zero with a clean last error is the end, and the
    /// list was whole; anything else — another format, or a failure at that point — means the
    /// tail was not seen, and the second value says `true`. The snapshot still answers `Ok`, as it
    /// always did: a long list is not an error, it is a fact to be told.
    fn formats(&self) -> WinResult<(Vec<u32>, bool)> {
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
                    Ok((formats, false))
                } else {
                    Err(error)
                };
            }

            formats.push(next);
            current = next;
        }

        // SAFETY: as in the loop — the clipboard is open and `current` is the format the last
        // iteration returned, passed by value. The answer is examined on the next line, and
        // nothing of ours is dereferenced.
        let beyond = unsafe { EnumClipboardFormats(current) };

        // NFR-13: the end of the list is zero with a clean last error, exactly as in the loop.
        let ended = beyond == 0 && WinError::from_thread().code().is_ok();

        Ok((formats, !ended))
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
    /// Whether the clipboard listed more formats than [`MAX_FORMATS`], so that the snapshot never
    /// saw the tail of the list — finding Н16, task T-38-7. The neighbour of `truncated`, and a
    /// different fact: that one is the budget, this one the bound of the enumeration.
    formats_truncated: bool,
    /// How many privacy markers the clipboard listed **without data**, kept here in their
    /// restrictive form — finding С25, task T-38-6, decision 121.7. They are among `captured`,
    /// holding [`PRIVACY_MARKER_DATA`] rather than bytes read off the clipboard.
    markers_without_data: usize,
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
            formats_truncated: false,
            markers_without_data: 0,
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

    /// Whether the list of formats was cut short at [`MAX_FORMATS`] — the snapshot is then partial
    /// in a way no other count shows: [`Snapshot::listed_formats`] answers the bound, not the list.
    /// Task T-38-7, finding Н16.
    pub fn is_format_list_truncated(&self) -> bool {
        self.formats_truncated
    }

    /// How many privacy markers were listed without data and are kept in their restrictive form
    /// — task T-38-6, decision 121.7. A count of formats, never of anything in them.
    pub fn markers_without_data(&self) -> usize {
        self.markers_without_data
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
    /// would print `captured`, and `captured` is the user's clipboard. The counts and the flag
    /// below say everything a diagnosis needs and nothing a person's data would be recognisable
    /// in.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("captured_formats", &self.captured.len())
            .field("listed_formats", &self.listed)
            .field("handle_formats", &self.handle_formats)
            .field("refused_formats", &self.refused)
            .field("total_bytes", &self.bytes)
            .field("truncated", &self.truncated)
            .field("formats_truncated", &self.formats_truncated)
            .field("markers_without_data", &self.markers_without_data)
            .finish()
    }
}

impl Drop for Snapshot {
    /// Overwrites the captured bytes before the allocator gets them back.
    ///
    /// The same treatment [`crate::buffer`] gives the typing buffer for SEC-02, and for the same
    /// reason: a freed heap block is readable by whatever allocates next, and the content of
    /// somebody's clipboard is precisely what SEC-07 names.
    ///
    /// The write is `crate::buffer::zero_slice` — `write_volatile` and a `compiler_fence`, the
    /// ring's own write. It used to be `fill(0)`, argued for on the grounds that a store through
    /// a slice cannot be dead; the audit of 2026-08-24 refused that argument, and task
    /// **T-13-15** replaced it. The vector is freed the moment this function returns, the
    /// optimiser can see that as clearly as any other dead store, and `buffer.rs` says so in as
    /// many words about its own ring.
    fn drop(&mut self) {
        for entry in &mut self.captured {
            crate::buffer::zero_slice(&mut entry.bytes);
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
/// ⭐ **Except the privacy markers — finding С25 of the audit of 2026-09-04, task T-38-6, decision
/// 121.7.** [`PRIVACY_MARKERS`] listed with no data are neither refused nor lost: they are kept in
/// their restrictive form, [`PRIVACY_MARKER_DATA`], so that step 8 does not put a secret back
/// without the marks that kept it out of the history and the cloud. Only these three, and only in
/// that form: presence without data can be restored only as a promise of delayed rendering owned
/// by this program's window, and that would route every other program's read of the format through
/// the UI thread and answer it with nothing. Counted in [`Snapshot::markers_without_data`].
///
/// **Handle formats.** See [`HANDLE_FORMATS`] for why bitmaps, palettes and metafiles are left
/// alone and what the system's format synthesis gives back for free.
pub fn snapshot(owner: HWND) -> Result<Snapshot, ClipboardError> {
    let clipboard = Clipboard::open(owner)?;

    let (listed, formats_truncated) = clipboard.formats()?;

    // ⭐ **Finding Н16 of the audit of 2026-09-04, task T-38-7.** A list cut short at the bound is
    // counted and journalled once, the way the budget of FR-64 below is — the fact and nothing
    // else: not how many formats there were, not which (SEC-01, SEC-07). The snapshot carries the
    // flag and goes on with the formats it did see.
    if formats_truncated {
        FORMAT_LIST_TRUNCATIONS.fetch_add(1, Ordering::Relaxed);
        note_format_list_truncation();
    }

    // Pass one: what is there, how big it is, and what cannot be taken. No bytes move here.
    let mut sizes: Vec<(u32, HGLOBAL, usize)> = Vec::with_capacity(listed.len());
    let mut handle_formats = 0_usize;
    let mut refused = 0_usize;
    let mut total = 0_usize;

    // Task T-38-6: the privacy markers of this session, and the ones found without data.
    let markers = privacy_markers();
    let mut markers_found_empty: Vec<u32> = Vec::new();

    for &format in &listed {
        if !is_memory_format(format) {
            handle_formats += 1;
            HANDLE_FORMATS_SEEN.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let Some(handle) = clipboard.block(format) else {
            // A promise nobody kept. For a privacy marker that is the shape premise П2 measured
            // being lost, and the marker is kept in its restrictive form instead — decision 121.7.
            if markers.contains(&format) {
                markers_found_empty.push(format);
                continue;
            }

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
            // that behaves like one — or the block is empty. Both are nothing to restore, except
            // that a privacy marker is kept in its restrictive form, as above.
            if markers.contains(&format) {
                markers_found_empty.push(format);
                continue;
            }

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
    let mut captured = Vec::with_capacity(sizes.len() + markers_found_empty.len());
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

    // Task T-38-6, decision 121.7: the markers found without data, in their restrictive form. Not
    // over the budget — FR-64 then keeps the text alone, and a marker is not text: the rule every
    // marker **with** data already follows a few lines above.
    let markers_without_data = if truncated {
        0
    } else {
        markers_found_empty.len()
    };

    if !truncated {
        for format in markers_found_empty {
            bytes += PRIVACY_MARKER_DATA.len();
            captured.push(Captured {
                format,
                bytes: PRIVACY_MARKER_DATA.to_vec(),
            });
        }
    }

    // ⭐ **Finding С11 of the audit of 2026-09-04, task T-38-8.** Something listed and nothing of it
    // kept is counted and journalled once, on the final count: after the copying, and without the
    // markers above, which go back in this program's own restrictive form and are not something
    // kept. The answer changes nothing here — decision 121.2.
    let _saved_nothing = snapshot_saved_nothing(
        listed.len(),
        captured.len().saturating_sub(markers_without_data),
    );

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
        formats_truncated,
        markers_without_data,
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

/// Puts the journal entry a list of formats cut short at [`MAX_FORMATS`] leaves behind — **finding
/// Н16 of the audit of 2026-09-04, task T-38-7, decision 121.3**.
///
/// The shape of [`note_truncation`], and for its reasons: the *fact* and nothing else, under a name
/// of the closed vocabulary of module `diag` — «clipboard formats truncated», the row decision
/// 121.3 allowed — with [`crate::diag::OsCode::NONE`] beside it, because no Win32 call failed and a
/// number built from the list would be a shape of somebody's clipboard (SEC-01, SEC-07).
fn note_format_list_truncation() {
    crate::diag::record(
        crate::diag::Operation::from_name("clipboard formats truncated"),
        crate::diag::OsCode::NONE,
    );
}

/// ⭐ **Finding С11 of the audit of 2026-09-04, task T-38-8, decision 121.2.** Whether a snapshot of
/// a clipboard that listed formats kept none of them — and, when it did, the count and the journal
/// entry of that fact.
///
/// `listed` is how many formats the clipboard listed; `captured` is how many entries were kept
/// **of the clipboard** — the markers of decision 121.7 found without data are left out by the
/// caller, because the bytes they go back with are this program's own and not something kept. The
/// asymmetry [`restore_placed_nothing`] keeps for step 8, on the side of step 1: an **empty**
/// clipboard snapshotted empty answers `false` — there was nothing to keep, and keeping nothing is
/// the whole of the correct behaviour — and so does a snapshot that kept a part, the text of a
/// clipboard over the budget of FR-64. The fact is the asymmetry: something listed, nothing kept.
///
/// # Why the count is taken here
///
/// «Counted where it is decided», the rule [`console_refuses_selection`] follows: the fact is a
/// function of two numbers, so acceptance drives it — three listed, none kept — with no clipboard
/// at all, and sees the counter move and the journal grow by one entry. [`snapshot`] is the one
/// caller, and the one place that has the real numbers.
///
/// # What does not change
///
/// The behaviour. Step 1 still answers success and the `Ctrl+C` of step 2 still goes out: a gate
/// before step 2 is a question of its own (decision 121.2). This is the trace of the fact, and
/// only that.
pub fn snapshot_saved_nothing(listed: usize, captured: usize) -> bool {
    let saved_nothing = listed > 0 && captured == 0;

    if saved_nothing {
        SNAPSHOTS_SAVED_NOTHING.fetch_add(1, Ordering::Relaxed);
        note_snapshot_saved_nothing();
    }

    saved_nothing
}

/// Puts the journal entry a snapshot that saved nothing leaves behind — **finding С11 of the audit
/// of 2026-09-04, task T-38-8, decisions 121.2 and 121.3**.
///
/// The shape of [`note_truncation`] and [`note_format_list_truncation`], and for their reasons: the
/// *fact* and nothing else, under the row decision 121.3 allowed — «clipboard snapshot saved
/// nothing» — with [`crate::diag::OsCode::NONE`] beside it, because no Win32 call failed and a
/// number built from the snapshot would be a shape of somebody's clipboard (SEC-01, SEC-07).
fn note_snapshot_saved_nothing() {
    crate::diag::record(
        crate::diag::Operation::from_name("clipboard snapshot saved nothing"),
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
/// Until task **Т-18-1** that sentence was true of this function and false of the program: the
/// snapshot did reach [`restore_after`] on every one of those paths, and the gate of decision
/// П-5 sent it away again. This function was never the place that failed — it is named here
/// because a reader who checks the promise stops at the first line that makes it, and this was
/// that line.
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
    // ⭐ **Task Т-22-6, finding м6 of the audit of 2026-09-01 — the three exits that left nothing
    // behind.** Every one of them ends with the user's clipboard not holding what this module
    // promised to put back, and until this task every one of them was silent: the callers of
    // `restore` and `restore_after` drop the result (`let _ =`), so an `Err` here reached nobody
    // at all, and `placed == 0` reached nobody even in the `Ok` case.
    let clipboard = match Clipboard::open(owner) {
        Ok(clipboard) => clipboard,
        Err(error) => return Err(note_restore_failed(error)),
    };

    if let Err(error) = clipboard.empty() {
        // The clipboard is still the user's here — nothing was wiped — but the restore is over
        // and did not happen, which is the fact that has to leave a trace.
        return Err(note_restore_failed(ClipboardError::from(error)));
    }

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

    // ⭐ **Task Т-22-6, the third exit and the worst of them.** `EmptyClipboard` succeeded, so the
    // user's content is already gone, and not one format went back. This is an `Ok` — the calls
    // this function makes all behaved — and it is the state the module exists to prevent, so it
    // is counted and journalled like the two failures above.
    if restore_placed_nothing(restored.placed, snapshot.captured.len()) {
        // No Win32 call failed as such: every `put` refused one format of somebody's clipboard,
        // and `Clipboard::put` has already dropped those errors for the reason above.
        note_restore_failed_without_error();
    }

    drop(clipboard);
    let sequence = own_change_number(owner);

    // Task T-19-3. `None` means the clipboard is not ours any more and the number standing now
    // is a stranger's — see [`own_change_number`].
    if let Some(sequence) = sequence {
        note_own_write(sequence);
    }

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
/// the dangerous half on the wrong thread before finding out. **That order is load-bearing** —
/// FR-80 and NFR-01 rest on the thread being asked before anything sleeps — and the question
/// added below sits deliberately on the far side of the sleep, because it is a question about
/// what happened *during* the delay.
///
/// # The delay window belongs to the user too — the note to step 8, decision П-5
///
/// Two hundred milliseconds is long enough for a person to press `Ctrl+C` inside it, and a
/// clipboard manager needs far less. Until decision П-5 this function slept and restored
/// unconditionally — which was **the literal wording of step 8**, a decision of the specification
/// rather than a departure of the code from it; FR-63 asked only that changes be *tracked*, and
/// they were and are. What the wording did not say, and what the note now does, is what becomes
/// of a clipboard that has moved on:
///
/// > Восстановление пропускается, если после записи результата программой буфер обмена изменил
/// > другой процесс (сверка `GetClipboardSequenceNumber` с меткой собственной записи): затирать
/// > новую копию пользователя снимком недопустимо.
///
/// So the sequence number is read once more when the sleep ends and put to [`restore_is_due`],
/// **together with `answer`**. Between them they describe everything the clipboard is allowed to
/// be holding: a write of this program's own, which step 6 left behind in [`write_unicode_text`]
/// and the ring of [`is_own_change`] remembers, or the answer the application gave to the probe
/// of step 2, whose numbers `answer` carries. Anything else is the third writer П-5 exists for.
///
/// # `answer`, and why it is a parameter — tasks Т-18-1 and T-38-2
///
/// It is the answer to the probe of step 2 as [`run`] saw it — the number [`Wait::Changed`]
/// reported at step 3 and, since task T-38-2, the number standing after step 4 read the
/// clipboard — handed down through [`Owed::StepEight`] and [`Path::restore_clipboard`]. See
/// [`ProbeAnswer`] for why one number was not enough. Callers that are not step 8 of a selection
/// pass — and there are none in the product — pass [`ProbeAnswer::at`] the number the clipboard
/// held when they last knew it to be theirs to put back, or [`ProbeAnswer::NONE`].
///
/// **Not through the ring.** Putting the answer into [`OWN_MARK`] would have been one line fewer
/// and would have made [`handle_clipboard_message`] classify an application's write as this
/// program's own: `own_updates` would count a change the program did not make, `foreign_updates`
/// would lose it, and FR-63 asks those two to mean what they say. The ring holds writes; this
/// parameter holds numbers the caller vouches for.
///
/// `Ok(Restored::default())` is the answer when the restore is skipped: nothing was placed and
/// nothing was refused, which is the truth of it. What happened is in
/// [`Counters::restore_skips`] and in the journal.
pub fn restore_after(
    owner: HWND,
    snapshot: &Snapshot,
    delay: Duration,
    answer: ProbeAnswer,
) -> Result<Restored, ClipboardError> {
    require_blocking_thread()?;

    sleep(delay);

    if !restore_is_due(sequence_number(), answer) {
        return Ok(Restored::default());
    }

    restore(owner, snapshot)
}

/// Whether step 8 may still write — **the note to step 8 of §4.7, decision П-5**.
///
/// `current` is the clipboard sequence number as it stands when the delay of step 8 has run out.
/// `answer` is the run of numbers the clipboard took while the application answered the `Ctrl+C`
/// of step 2 — see [`restore_after`], which is where both come from, and [`ProbeAnswer`].
///
/// Two questions, and either one of them owes the snapshot back:
///
/// * `answer.contains(current)` — **nothing has happened since the application answered the probe
///   but that answer itself.** The window station's counter only ever goes up, so a number inside
///   the answer is not evidence, it is proof. What is on the clipboard is the selection this
///   program asked for, and putting the user's content back over it is the whole purpose of the
///   module;
/// * [`is_own_change`] against the ring [`OWN_MARK`], which holds the numbers this process's own
///   writes produced — step 6 of FR-61 among them. A `current` the ring knows means nothing has
///   touched the clipboard since this program wrote.
///
/// A `current` that is neither means somebody wrote after both, and the note forbids putting the
/// snapshot over that.
///
/// # The second question alone was a defect — task Т-18-1
///
/// Until the audit of 2026-08-31 this asked [`is_own_change`] and nothing else, and the doc
/// comment here claimed that the inexactness [`note_own_write`] describes «runs in the safe
/// direction here too». On the path this function was written for — step 6 wrote, and then the
/// user copied something — that was true. On **every failure path of steps 4 to 6 it was exactly
/// backwards**: there is no write of ours anywhere on those paths, the ring is silent by
/// construction, and the price of reading that silence as a foreign copy was not a redundant
/// restore but the user's clipboard, permanently. The answer to the probe is what makes the
/// sentence true again, by giving the safe direction something to be safe about.
///
/// # One number was not enough — task T-38-2
///
/// Т-18-1 carried the number step 3 saw, and that number is the **first** of the answer, not
/// always the last: an application still writing when step 3 looks goes on moving the counter,
/// and a gate that asked «`current` equals that number» read the application's own later numbers
/// as a stranger's copy — on exactly the failure paths Т-18-1 was about. Measured on Word, one
/// copy in twenty; see [`ProbeAnswer`]. The answer now runs to the number standing after step 4's
/// read. What П-5 gives up for it is honest and small: a third writer's copy that lands **between
/// step 3's look and step 4's read** now reads as part of the answer. That gap is the application
/// writing with the clipboard held open, and a stranger gets in only between its close and the
/// read's open; a copy made **after** the read is past `last` and keeps its place, as before.
///
/// A zero answer is refused — see [`ProbeAnswer::contains`].
///
/// # Why a refusal is counted and journalled
///
/// A restore that does not happen is the promise of this module's first paragraph deliberately
/// not kept. It is right here — and it is not nothing, so it leaves a counter and one entry in
/// the journal behind it. **Only a real П-5 skip reaches those two lines**: a restore made on a
/// failure path is a restore, not a skip, and the counter says what it always said.
fn restore_is_due(current: u32, answer: ProbeAnswer) -> bool {
    if answer.contains(current) {
        return true;
    }

    if is_own_change(current) {
        return true;
    }

    RESTORE_SKIPS.fetch_add(1, Ordering::Relaxed);

    note_restore_skipped();

    false
}

/// The clipboard sequence numbers of the application's answer to the probe of step 2 — **task
/// T-38-2, finding С14 of the audit of 2026-09-04**.
///
/// One copy is not one number. `EmptyClipboard` moves the window station's counter, every
/// `SetClipboardData` moves it again, and `CloseClipboard` moves it once more for each format it
/// synthesises — Э19 measured +1, +1 and +3 (see [`own_change_number`]). [`wait_for_change`]
/// looks every [`POLL_INTERVAL`] and answers the **first** number it sees move, so an application
/// still writing when step 3 looks leaves it holding the first number of many rather than the
/// last. Measured 2026-09-10 on the machine this program was written for: one copy out of Word
/// moves the counter by nineteen, and on one copy in twenty step 3 answered 1182 while Word went
/// on to 1200; Notepad and Edge, forty copies between them, finished inside a single look.
///
/// `first` is the number step 3 answered with. `last` is the number standing once step 4 has read
/// the clipboard — a read that has to wait for the writer to close it first, which is why in all
/// twenty copies out of Word that number was already the final one. Every number from `first` to
/// `last` is the same answer, and step 8's gate owes the snapshot for any of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProbeAnswer {
    /// The number [`Wait::Changed`] reported at step 3.
    pub first: u32,
    /// The number standing after step 4 read the clipboard — `first` when it did not.
    pub last: u32,
}

impl ProbeAnswer {
    /// No probe was answered: the value for a restore that is not step 8 of a selection pass.
    pub const NONE: Self = Self { first: 0, last: 0 };

    /// An answer known by the one number step 3 saw.
    pub const fn at(sequence: u32) -> Self {
        Self {
            first: sequence,
            last: sequence,
        }
    }

    /// The same answer, known to have run on to `sequence`.
    ///
    /// A number below `last` widens nothing: the window station's counter only goes up, so a lower
    /// number is not a later look. A counter that wrapped past `u32::MAX` inside one pass would
    /// need four thousand million changes in a few hundred milliseconds, and is not a case.
    pub const fn through(self, sequence: u32) -> Self {
        Self {
            first: self.first,
            last: if sequence > self.last {
                sequence
            } else {
                self.last
            },
        }
    }

    /// Whether `current` is a number of this answer.
    ///
    /// Zero is never one, for the reason [`is_own_change`] refuses a zero sequence: it is what
    /// `GetClipboardSequenceNumber` answers to a process without clipboard access, and a zero
    /// compared with a zero is not a fact about anybody's clipboard.
    pub const fn contains(self, current: u32) -> bool {
        self.first != 0 && current >= self.first && current <= self.last
    }
}

/// Puts the journal entry the skipped restore of decision П-5 leaves behind.
///
/// # SEC-01, SEC-07 — what this entry can and cannot say
///
/// «clipboard restore skipped» is a fact about **a decision of this program**: step 8 was due and
/// was not made. The string is chosen at compile time, it is a row of the closed table of module
/// `diag`, and nothing in it is derived from the clipboard — not the content that was left
/// standing, not its size, not its formats, not the two sequence numbers that decided it.
/// [`crate::diag::OsCode::NONE`] goes with it for the reason [`note_truncation`] passes it: no
/// Win32 call failed here, and a number built from a measurement would be a shape of somebody's
/// contents.
///
/// # Where this runs — NFR-01…NFR-05
///
/// On the caller of [`restore_after`], which [`require_blocking_thread`] has already confined to
/// the thread [`listen`] claimed — the UI thread of section 6.1. The hook callback cannot reach
/// it: it is refused several lines earlier, before the sleep.
fn note_restore_skipped() {
    crate::diag::record(
        crate::diag::Operation::from_name("clipboard restore skipped"),
        crate::diag::OsCode::NONE,
    );
}

/// ⭐ **Task Т-22-6.** Whether a restore emptied the clipboard and put nothing back.
///
/// `placed` is [`Restored::placed`], `captured` is how many entries the snapshot held. A pure
/// function of two numbers, for the reason [`read_refuses_size`] is one: the boundary is then
/// driven by a unit test rather than by whatever happens to be on somebody's clipboard, and a
/// mutation of the comparison fails a test instead of passing quietly.
///
/// An **empty** snapshot placing nothing is not a failure and answers `false`: the clipboard held
/// nothing this module could keep, so putting nothing back is the whole of the correct behaviour.
/// The failure is the asymmetry — something to place, and nothing placed.
#[must_use]
pub const fn restore_placed_nothing(placed: usize, captured: usize) -> bool {
    captured > 0 && placed == 0
}

/// ⭐ **Task Т-22-6.** Counts and journals a restore that did not put the snapshot back, and hands
/// the error on so that the caller's `?` — or its `let _ =` — is unchanged.
///
/// # SEC-01, SEC-07 — what this entry can and cannot say
///
/// «clipboard restore failed» is a fact about **this program's own attempt**: step 8 was due, was
/// not withheld, and did not happen. The string is chosen at compile time and is a row of the
/// closed table of module `diag`. Nothing in it is derived from the clipboard — not the content
/// that was lost, not its size, not its formats, not how many entries the snapshot held.
///
/// The code beside it is the `HRESULT` of the Win32 call that refused, and only that:
/// [`ClipboardError::Os`] carries a `windows::core::Error`, whose `HRESULT` module `diag` takes
/// and whose message it drops unread. The module's own two refusals carry no system error at all
/// and go with [`crate::diag::OsCode::NONE`] — a number invented here would be a claim about a
/// call nobody made.
///
/// ⚠ **Never reached by a П-5 skip.** A skip returns before [`restore`] is entered, so the two
/// counters cannot be confused however the callers drop their results — which is what keeps
/// [`Counters::restore_skips`] meaning "withheld on purpose" and this one meaning "tried and
/// failed".
fn note_restore_failed(error: ClipboardError) -> ClipboardError {
    let code = match &error {
        ClipboardError::Os(os) => crate::diag::OsCode::of(os),
        ClipboardError::WrongThread | ClipboardError::Busy => crate::diag::OsCode::NONE,
    };

    RESTORE_FAILURES.fetch_add(1, Ordering::Relaxed);

    crate::diag::record(
        crate::diag::Operation::from_name("clipboard restore failed"),
        code,
    );

    error
}

/// [`note_restore_failed`] for the third exit, which has no error to carry — see [`restore`].
fn note_restore_failed_without_error() {
    RESTORE_FAILURES.fetch_add(1, Ordering::Relaxed);

    crate::diag::record(
        crate::diag::Operation::from_name("clipboard restore failed"),
        crate::diag::OsCode::NONE,
    );
}

// ---------------------------------------------------------------------------------------
// Reading and writing text — steps 4 and 6 of FR-61 as primitives
// ---------------------------------------------------------------------------------------

/// Is a `CF_UNICODETEXT` block too big to read — the ceiling of step 4, symmetrical to FR-64.
///
/// # The finding, and why the number is not a new one
///
/// The audit of 2026-08-31 (№7) found step 4 unbounded: FR-64 gives the *snapshot* a budget of
/// four megabytes and says so, while the read of the selection had none at all — a selection of
/// any size was copied off the clipboard by [`read_block`], decoded into a second copy by
/// [`decode_utf16`] and recoded into a third on the way to step 6. Decision 77 of the user is
/// symmetry: **the same four megabytes**, and the same constant — [`SNAPSHOT_BUDGET_BYTES`],
/// named once, because two spellings of one number are two numbers as soon as one of them is
/// edited.
///
/// # What it decides, and what it does not
///
/// A block over the ceiling is treated as **«there is no text»**: [`read_unicode_text`] answers
/// `Ok(None)`, step 4 turns that into [`Refusal::NoText`], and the snapshot taken at step 1 goes
/// back through the refusal path of Э18 — `Owed::StepEight` was armed the moment the probe
/// changed the clipboard, and nothing here disarms it. It is not an error: an error would say
/// «the clipboard could not be read», and the clipboard was read perfectly well — it holds more
/// than this program is willing to copy, which is a decision and not a failure.
///
/// A pure function of one number, so that the boundary is driven by unit tests rather than by
/// whatever is on somebody's clipboard, and so that a mutation of the comparison fails a test
/// instead of passing quietly. Exactly the budget is allowed: the ceiling is the largest block
/// that is read, not the smallest that is refused.
pub const fn read_refuses_size(size: usize) -> bool {
    size > SNAPSHOT_BUDGET_BYTES
}

/// Puts the journal entry an oversized read of step 4 leaves behind.
///
/// # SEC-01, SEC-07 — what this entry can and cannot say
///
/// «clipboard read oversized» is a fact about **a decision of this program**: step 4 found a
/// block above the ceiling of [`read_refuses_size`] and copied nothing. The string is chosen at
/// compile time and is a row of the closed table of module `diag`. It does not carry the size it
/// refused, the ceiling it compared against, the format, or a single byte of what was there —
/// and there is no branch here through which any of those could reach the ring, because the
/// caller passes no number at all. [`crate::diag::OsCode::NONE`] goes with it for the reason
/// [`note_truncation`] passes it: no Win32 call failed, and a number built from a measurement
/// would be a shape of somebody's contents.
fn note_read_oversized() {
    crate::diag::record(
        crate::diag::Operation::from_name("clipboard read oversized"),
        crate::diag::OsCode::NONE,
    );
}

/// Reads `CF_UNICODETEXT`, or `None` when the clipboard has no text — step 4 of FR-61.
///
/// The block is UTF-16 terminated by a null; the terminator is dropped and everything after it
/// is ignored, which is what every clipboard reader does and what a writer that padded the block
/// expects.
///
/// SEC-01, SEC-07: the answer is returned to the caller and is not logged, formatted or
/// journalled anywhere on the way. It comes in a [`ClipboardText`], which zeroes it when it is
/// dropped — finding С29, task T-38-9A.
///
/// The copy `read_block` makes is the user's clipboard in plain form, and it is zeroed before
/// this frame releases it — SEC-01, SEC-02, task **T-13-15**. Until the audit of 2026-08-24 it
/// was not zeroed at all, which made it one of the three holes in a rule the rest of this module
/// keeps.
pub fn read_unicode_text(owner: HWND) -> Result<Option<ClipboardText>, ClipboardError> {
    let clipboard = Clipboard::open(owner)?;

    let Some(handle) = clipboard.block(CF_UNICODETEXT) else {
        return Ok(None);
    };

    // SAFETY: `handle` is a live block borrowed from the open clipboard.
    let size = unsafe { GlobalSize(handle) };

    // ⚠ **The ceiling of step 4 — finding №7 of the audit of 2026-08-31, decision 77.** Asked
    // here and not a line lower, because `GlobalSize` reads a header and copies nothing while
    // `read_block` allocates a vector of exactly this many bytes: a ceiling asked after the copy
    // would refuse the text and still have made the copy it exists to avoid. Over it the answer
    // is `Ok(None)` — «there is no text» — which step 4 turns into `Refusal::NoText` and step 8
    // of Э18 answers by putting the snapshot back. See [`read_refuses_size`].
    if read_refuses_size(size) {
        note_read_oversized();

        return Ok(None);
    }

    let Some(mut bytes) = read_block(handle, size) else {
        return Ok(None);
    };

    let text = decode_utf16(&bytes);

    crate::buffer::zero_slice(&mut bytes);

    Ok(Some(text))
}

/// The text step 4 of FR-61 reads off the clipboard, and the buffer holding it — **finding С29 of
/// the audit of 2026-09-04, task T-38-9A**.
///
/// Overwritten with zeroes when it is dropped, the treatment [`Recoded`] gives the text of step 5
/// and [`Snapshot`] gives the blocks of step 1. Until task T-38-9A this was a bare `String`: the
/// block it was decoded from and the code units in between were zeroed, and the text itself — the
/// one copy steps 5 and 6 work from, alive from the read to the end of the pass — went back to the
/// allocator as it stood, while the sweep meant to catch that counted zeroing calls and found its
/// seven.
///
/// Reads as a `str` and as nothing else: no `Display`, and a hand-written `Debug` that prints a
/// count — SEC-01, SEC-07. `From<String>` is for a [`Path`] that reads no real clipboard, the
/// bench of the tests, whose text is its own.
pub struct ClipboardText {
    text: String,
}

impl Deref for ClipboardText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl From<String> for ClipboardText {
    fn from(text: String) -> Self {
        Self { text }
    }
}

impl fmt::Debug for ClipboardText {
    /// Counts, never characters — SEC-01, SEC-07. A derived `Debug` would print the text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipboardText")
            .field("chars", &self.text.chars().count())
            .finish()
    }
}

impl Drop for ClipboardText {
    fn drop(&mut self) {
        // SAFETY: the argument of the `Drop` of `Recoded`, word for word — every byte becomes
        // `0x00`, a run of NUL bytes is valid UTF-8, the length does not change and the vector is
        // not reallocated; `crate::buffer::zero_slice` writes into the elements of the slice it is
        // handed and does nothing else.
        crate::buffer::zero_slice(unsafe { self.text.as_mut_vec() }.as_mut_slice());
    }
}

/// Turns the bytes of a `CF_UNICODETEXT` block into the text of step 4.
///
/// A pure function, so that the awkward parts — an odd number of bytes, a missing terminator, an
/// unpaired surrogate — are driven by unit tests rather than by whatever happens to be on the
/// clipboard. Lossy on purpose: a lone surrogate is somebody else's malformed data, and refusing
/// the whole clipboard over it would be the wrong trade.
///
/// `units` is a second copy of the same text, and it is zeroed before it is released — SEC-01,
/// SEC-02, task **T-13-15**. The text that comes out is born inside [`ClipboardText`], which
/// zeroes it when it is dropped — task **T-38-9A** — and [`Recoded`] looks after what step 5
/// makes of it.
///
/// Public for the growth probe of task **T-38-9B**, `tests\selection_growth.rs`, which drives step 4
/// without a clipboard — the reason [`read_refuses_size`] is public.
pub fn decode_utf16(bytes: &[u8]) -> ClipboardText {
    // Task T-38-9B, finding Н34: there are at most half as many code units as bytes, so a vector
    // of that capacity is filled without growing. A collect after `take_while` knows no length and
    // grew it — ten times for four thousand units, each growth a copy of the text left behind.
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);

    units.extend(
        bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|&unit| unit != 0),
    );

    let text = string_from_units(&units);

    crate::buffer::zero_slice(&mut units);

    ClipboardText { text }
}

/// The text of `units`, in a string allocated once at its final length — **finding Н34 of the
/// audit of 2026-09-04, task T-38-9B**.
///
/// The answer of `String::from_utf16_lossy`, lossiness included — an unpaired surrogate becomes
/// U+FFFD — without its allocation: that one collects, a collect grows the string it collects
/// into, and every growth hands a block holding part of the user's text back to the allocator as
/// it stands, where no zeroing done afterwards can reach it. Measured before this task: twice for
/// 3500 characters of Russian, three times for 12 500 units of a ligature. So the length in UTF-8
/// is counted first, over the same decoding, and the string is made at that length and filled.
///
/// Hands the string back bare, and every caller moves it at once into the type that zeroes it:
/// [`decode_utf16`] into [`ClipboardText`], [`recode`] and [`recode_words`] into [`Recoded`].
fn string_from_units(units: &[u16]) -> String {
    let decoded = || {
        char::decode_utf16(units.iter().copied())
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
    };

    let mut text = String::with_capacity(decoded().map(char::len_utf8).sum());

    for ch in decoded() {
        text.push(ch);
    }

    text
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
///
/// The block `encode_utf16` builds carries the user's text — the recoded selection of step 6 —
/// and it is zeroed before this frame releases it on **every** path, the failures included
/// (SEC-01, SEC-02, task **T-13-15**). That is why the three fallible steps are inside a closure
/// rather than behind a `?` of this function: a `?` here would carry the vector out unzeroed,
/// which is exactly what it did until the audit of 2026-08-24.
///
/// # The mark follows the change, not the success — task Т-18-1
///
/// `EmptyClipboard` moves the window station's counter by itself. So a pass where `empty`
/// succeeded and `put` refused has **changed the user's clipboard — to nothing at all** — and
/// this function used to return from that state without a mark, because the mark stood after the
/// `?`. The clipboard was then empty, the change was ours, the ring said otherwise, and step 8
/// skipped the only restore that could have undone it: of every shape of this defect the audit
/// of 2026-08-31 found, that one ended with the user holding an empty clipboard and no copy of
/// anything, anywhere.
///
/// `emptied` is therefore the question the mark is hung on, and it is a truthful FR-63 mark
/// rather than a convenience: this program did make that change. `Clipboard::open` refusing, or
/// `empty` itself refusing, leaves the clipboard untouched and no mark is put — and none is
/// needed, since the number step 3 saw is then still the current one and `restore_is_due` owes
/// the snapshot on that ground.
///
/// # The privacy markers — task T-38-6, finding С25, decision 121.7
///
/// The text goes up with the three [`PRIVACY_MARKERS`] beside it, each in its restrictive form
/// [`PRIVACY_MARKER_DATA`]: the recoded selection is the user's text in another layout, on the
/// clipboard for the length of a paste, and it belongs in neither the history of Win+V nor the
/// cloud clipboard. Only beside a text that went up, and best effort — a marker the clipboard
/// refuses costs the paste nothing.
pub fn write_unicode_text(owner: HWND, text: &str) -> Result<(), ClipboardError> {
    let mut bytes = encode_utf16(text);
    let mut emptied = false;
    let mut sequence = None;

    let outcome = Clipboard::open(owner).and_then(|clipboard| {
        clipboard.empty()?;

        // Past this line the clipboard is not what the user left there, whatever happens next.
        emptied = true;

        // ⭐ **Task T-19-3, Э18-Д-1.** `put` is not behind a `?` any more, and the reason is the
        // two lines after it: a `?` here would leave through the closure and take the number of
        // our own change with it, past the O(n) zeroing below. The result is carried out
        // instead, so both outcomes of `put` leave through the same close and the same read.
        let put = clipboard.put(CF_UNICODETEXT, &bytes);

        // ⭐ **Task T-38-6, decision 121.7.** The privacy markers beside the text — see above.
        // Their errors are dropped for the reason `restore` drops a refused format's: they belong
        // to one format, the text is already up, and nothing a caller could do follows from them.
        if put.is_ok() {
            for marker in privacy_markers() {
                if marker != 0 {
                    let _ = clipboard.put(marker, &PRIVACY_MARKER_DATA);
                }
            }
        }

        // Closed before the write is announced, as it always was: the listener of FR-63 can see
        // the change the moment the clipboard is released.
        drop(clipboard);
        sequence = own_change_number(owner);

        put.map_err(ClipboardError::from)
    });

    crate::buffer::zero_slice(&mut bytes);

    // `None` is the clipboard having changed hands between the close and the read — see
    // [`own_change_number`]. Nothing is marked then, because the number now standing belongs to
    // somebody else and storing it is exactly the harm decision П-5 forbids.
    if emptied && let Some(sequence) = sequence {
        note_own_write(sequence);
    }

    outcome?;

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

/// ⭐⭐ **Lets the system deliver messages *sent* to this thread's windows — task T-76-1.**
///
/// # The defect this exists for, measured
///
/// A press whose selection path ran while **this program owned the clipboard** came back as the
/// idle tone of FR-100, and the same press repeated worked. It lived through every delivery that
/// had a selection path, and stage E75 spent a whole one raising the timeout of step 3 from three
/// hundred milliseconds to nine hundred before measuring that the timeout is not the dimension it
/// lives in at all (decisions 135в, 135г, 135д).
///
/// The chain is three links, and every one of them is this program's own doing:
///
/// 1. **We become the clipboard's owner whenever we write to it** — step 6 of FR-61 puts the
///    converted text there and step 8 puts the user's snapshot back, and both go through
///    [`Clipboard::empty`], which is what makes the window passed to `OpenClipboard` the owner.
/// 2. **The next press waits on this very thread without answering it.** [`wait_for_change`] polls
///    the sequence number every [`POLL_INTERVAL`] and sleeps in between, and it runs on the UI
///    thread — the thread of the window that is now the owner.
/// 3. **The application's `Ctrl+C` blocks on us.** When it answers step 2 it calls
///    `EmptyClipboard`, and the system delivers `WM_DESTROYCLIPBOARD` to the current owner with a
///    **blocking send**. The owner is a window of a thread asleep in the loop of link 2, so the
///    application sits inside `EmptyClipboard` until that loop gives up. Then the sequence number
///    has not moved, step 3 answers «there is no selection», and the press falls back to a typing
///    buffer that a `Ctrl+A` has just emptied — one click, nothing converted.
///
/// ⚠ `tests\e2e\scenarios.rs` describes this from the other side and measured what it costs: a
/// bench that owned the clipboard and pumped nothing held up the product's own `Ctrl+C` **5091
/// ms**, the system's hung-window timeout. It was read there as a fact about the bench. It is a
/// fact about any owner that stops answering, and this program is one for the length of a wait.
///
/// # Why a filtered `PeekMessage` and not a pump
///
/// Messages **sent** to a thread are delivered by the system inside any `PeekMessage`, whatever
/// filter it carries — the filter selects among **posted** messages, which sit in the queue. So a
/// range no posted message can match (`WM_USER..=WM_USER`) with `PM_NOREMOVE` answers the blocking
/// send and **dispatches nothing**: not the hotkey the input thread posts, not the tray's menu,
/// not the settings dialog. That matters more here than the brevity: the selection path holds a
/// [`Session`] with the user's snapshot in it, and a real pump in the middle of it would let a
/// second press, a menu or a dialog re-enter the module while that snapshot is owed back.
///
/// ⭐ The claim in the paragraph above is **measured, not quoted**:
/// `a_thread_that_owns_the_clipboard_blocks_everybody_until_it_answers` in `tests\selection.rs`
/// times a competing `EmptyClipboard` against a thread that sleeps the way step 3 sleeps, with
/// this call and without it.
pub fn let_sent_messages_through() {
    let mut message = MSG::default();

    // SAFETY: the pointer is to a live local, the window filter is `None` — every window of this
    // thread — and the message range is `WM_USER..=WM_USER`, which no posted message of this
    // program can match. `PM_NOREMOVE` takes nothing out of the queue. The call touches no state
    // of ours and its `BOOL` answer is deliberately dropped: whether a posted message happened to
    // match is not the question, and the delivery of sent messages is not reported by it.
    let _ = unsafe { PeekMessageW(&raw mut message, None, WM_USER, WM_USER, PM_NOREMOVE) };
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

        // ⭐⭐ **Task T-76-1 — this is not a courtesy, it is the repair.** While this program owns
        // the clipboard, the application answering the `Ctrl+C` of step 2 blocks inside
        // `EmptyClipboard` on a `WM_DESTROYCLIPBOARD` **sent** to our window; a loop that naps
        // without answering it waits for something that is waiting for this loop, and no timeout
        // can be large enough. It stands **before** the nap so the answer goes out at once rather
        // than one poll late. See [`let_sent_messages_through`] for the whole chain and for why
        // nothing posted is dispatched here.
        let_sent_messages_through();

        sleep(POLL_INTERVAL.min(left));
    }
}

/// The number of the change this frame has just made, or `None` if the clipboard is not this
/// process's any more — **task T-19-3, Э18-Д-1, finding 6 of the audit of 2026-08-31**.
///
/// # Why the number cannot be taken any earlier — measured, not reasoned
///
/// The audit asked for the number to be read **under the open clipboard**, the way [`snapshot`]
/// reads its baseline. That is impossible on a path that *writes*, and the measurement says so
/// plainly. One `OpenClipboard`, `EmptyClipboard`, `SetClipboardData`, `CloseClipboard` on the
/// machine's real clipboard, with `GetClipboardSequenceNumber` between every pair:
///
/// ```text
/// before_open=1025  after_open=1025  after_empty=1026  after_put=1027  after_close=1030
/// ```
///
/// The counter moves once for `EmptyClipboard`, once for `SetClipboardData` — and **three more
/// times on `CloseClipboard`**, where the window station synthesises `CF_TEXT`, `CF_OEMTEXT` and
/// `CF_LOCALE` out of the `CF_UNICODETEXT` that went up. The number a `WM_CLIPBOARDUPDATE`
/// handler later reads is the one after the close, so a number read under the open clipboard is
/// guaranteed to be the wrong one: the first attempt at this repair took it there and
/// `our_own_change_is_recognised_and_a_foreign_one_is_not` went red on the spot. [`snapshot`] is
/// not a counter-example — it only reads, so it moves nothing and its baseline is exact.
///
/// # What is done instead
///
/// The earliest instant the number exists at all is immediately after `CloseClipboard`, and that
/// is where both callers read it, with nothing whatever in between. That alone is most of the
/// repair: the read used to stand after the O(n) volatile zeroing of a block that can be
/// megabytes — milliseconds of window on a large selection — and now stands one instruction
/// after the close.
///
/// One instruction is still not none, so the owner is asked as well, and **after** the number,
/// never before. A process that wrote in between leaves an owner that is not ours; the number is
/// thrown away and nothing is marked. Asked the other way round, that same process would have had
/// its number stored as this program's own — which is finding 6 exactly: step 8 would then read a
/// stranger's copy as ours and overwrite it with the snapshot, the one thing decision П-5 exists
/// to forbid. So the classification is now sound in the direction that costs the user something:
/// a foreign number can no longer enter the ring at all. The price is the other direction — a
/// change of ours can go unmarked if somebody wrote in that instant — and that is the direction
/// [`note_own_write`] has always called the safe one.
///
/// `GetClipboardOwner` answers the window that last emptied the clipboard, which is `owner`
/// itself: [`Clipboard::open`] opens with it and `EmptyClipboard` makes it the owner. A process
/// cannot put anything on the clipboard without taking ownership the same way, so a stranger's
/// write is exactly what this notices.
fn own_change_number(owner: HWND) -> Option<u32> {
    let sequence = sequence_number();

    // SAFETY: takes no arguments and dereferences nothing; it reads a window station value and
    // is callable from any thread with or without the clipboard open. The `HWND` it answers is
    // compared and never used, so a window that has since been destroyed is harmless here.
    let current = unsafe { GetClipboardOwner() }.ok();

    (current == Some(owner)).then_some(sequence)
}

/// Remembers the sequence number this process's own write just produced — FR-63.
///
/// # The number is handed in, never fetched here — task T-19-3, Э18-Д-1
///
/// ⭐ **This function used to read `GetClipboardSequenceNumber` itself, and that was the defect.**
/// It is called after the clipboard guard is gone, and in [`write_unicode_text`] it used to be
/// called after the O(n) volatile zeroing of a block that can be megabytes. Anything that changed
/// the clipboard inside that window put **its** number here: a clipboard manager answering
/// `WM_CLIPBOARDUPDATE`, or a `Ctrl+C` of the user's. The ring of FR-63 then held a stranger's
/// number, [`is_own_change`] answered "ours" for the stranger's change, and step 8 overwrote it
/// with the snapshot — the one thing decision П-5 exists to forbid.
///
/// The number now arrives from [`own_change_number`], which reads it one instruction after the
/// close and throws it away if the clipboard changed hands. There are two callers,
/// [`write_unicode_text`] and [`restore`], and the sweep
/// `the_number_of_our_own_write_is_taken_the_instant_the_clipboard_is_released` holds both to it.
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
/// handler can only ask what the sequence number is **now**. So the recognition is exact when our
/// message is taken before the next change and inexact when it is not: if a foreign write lands
/// between our write and our reading of the queue, both changes look foreign.
///
/// ⚠ **That the mis-classification is one-directional is a property of the ring holding our own
/// number, and until task T-19-3 the ring did not always hold it.** With the number fetched here,
/// after the close and after the zeroing, a foreign write in that window was recorded as ours and
/// the program went on to overwrite the user's copy — the unsafe direction, and the audit of
/// 2026-08-31 found it. [`own_change_number`] is what makes the sentence true again: a number
/// taken while the clipboard has already changed hands never reaches this function, so the only
/// inexactness left is a change of ours read as the user's, which makes the program consider a
/// clipboard it may treat as new, whereas the reverse would make it ignore a real user action.
pub fn note_own_write(sequence: u32) {
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

/// ⭐ **The ceiling of `[selection] clipboard_timeout_ms` — task T-13-13.**
///
/// The *default* is the three hundred milliseconds [`timeout_of`] names one line above, and it
/// lives in `src\settings.rs`; the ceiling is the other end of the same field and lives here,
/// beside the function that states the default. The pair is the shape [`crate::buffer`] already
/// has for `[buffer] capacity` — `DEFAULT_CAPACITY` and `MAX_CAPACITY` side by side, with
/// `effective_capacity` between them.
///
/// ⛔⛔ **Read this before raising the default. It has been raised once, for nothing.**
///
/// Delivery `e75` moved it from three hundred to nine hundred against the defect «the first press
/// on a selection is idle, the second one works», on a hypothesis that fitted three probes: that
/// the application answers the `Ctrl+C` of step 2 later than the wait allows. The hypothesis was
/// **wrong**, the nine hundred fixed nothing, and `e77` put the number back. What the instrument of
/// task T-76-1 measured afterwards (decisions 135в–135д):
///
/// * a wait that **succeeds** takes **five milliseconds** — the very first look of
///   [`POLL_INTERVAL`]. Not two hundred, not two hundred and eighty. Five;
/// * a wait that **fails** is not helped by three hundred, by nine hundred, or by any number,
///   because the application was blocked **on this program**: we own the clipboard after every
///   step 6 and every step 8, and its `EmptyClipboard` waited on a `WM_DESTROYCLIPBOARD` sent to a
///   window whose thread was asleep in this very loop;
/// * **there is nothing in between.** The two outcomes are five milliseconds and never.
///
/// So three hundred is not a guess and not a compromise: it is sixty times the only duration a
/// successful wait has ever been measured to need. A larger number buys nothing and is paid for by
/// the one press that was going to be idle anyway — the pause before its tone. If a press is still
/// idle, the answer is **not** here: it is a question about what is holding the other application,
/// and task T-76-1 is where that was found the first time.
///
/// ⚠ **Nothing in this module enforces it.** The clamp stands on the one publication, in
/// `crate::app::publish_configuration`, so [`published_timeout`] already answers a value inside
/// this bound; see [`crate::inject::MAX_INTER_EVENT_DELAY_MS`] for why the check belongs to the
/// publication and not to the place that waits.
///
/// # Why five thousand
///
/// Step 3 of FR-61 is a **wait**, and [`wait_for_change`] runs it on the thread [`listen`] claimed
/// — the UI thread of section 6.1, the thread that owns the tray icon, the menu and the settings
/// dialog. So this field is not the freeze of `[replacement] inter_event_delay_ms`: the hook and
/// the watchdog are on the input thread and keep running, and FR-65's own promise holds — with the
/// selection path off, the hotkey never comes here at all. What an unbounded value costs instead
/// is the interface: a `u32::MAX` in the file is about forty-nine days in which the tray does not
/// answer a click and the dialog does not repaint, and the user's only remaining move is to kill
/// the process.
///
/// Five seconds is the same order as the default `LowLevelHooksTimeout` of FR-80 — the budget the
/// neighbouring ceiling of [`crate::inject::MAX_INTER_EVENT_DELAY_MS`] is cut against, so that the
/// three numbers of this repair are read against one measure rather than three — and it is longer
/// than any application takes to answer a `Ctrl+C` while still being a wait a person recognises as
/// a wait. NFR-09's thirty milliseconds are the *typing-buffer* path and do not reach here; this
/// path is the one that waits for another process by design.
///
/// The number is not derived here: it is the one `reports\ТЗ-Э13-ремонт.md` fixes for task
/// T-13-13, so that its origin does not get lost.
pub const MAX_CLIPBOARD_TIMEOUT_MS: u32 = 5_000;

/// ⭐ **The ceiling of `[selection] clipboard_restore_delay_ms` — task T-13-13.**
///
/// The default is the two hundred milliseconds of section 7, which [`restore_delay_of`] reads out
/// of the configuration; this is the other end of the same field. Everything
/// [`MAX_CLIPBOARD_TIMEOUT_MS`] says about the thread, about the ceiling being enforced at the
/// publication rather than here, and about where the number comes from holds word for word, and
/// the two are equal for that reason.
///
/// One thing is this field's own: the delay of step 8 is time in which **the user's own clipboard
/// is still overwritten by this program's text**. A file that set it to days would not merely hang
/// the interface, it would leave somebody else's copy standing in place of the user's for as long;
/// decision П-5 already refuses to put the snapshot back over a copy made inside that window, and
/// a bounded window is what keeps that refusal a rare case rather than the normal one.
pub const MAX_CLIPBOARD_RESTORE_DELAY_MS: u32 = 5_000;

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

// =========================================================================================
// T-07-2 — the selection path itself: FR-60, FR-61, FR-65
// =========================================================================================

// ---------------------------------------------------------------------------------------
// FR-65 — `[selection]` of section 7, published to the thread that has to branch
// ---------------------------------------------------------------------------------------

/// `[selection] enabled` — FR-65. Section 7 has it `true`, and so does this.
///
/// # Why it is published and not read where it is needed
///
/// The branch of FR-60 is taken on the **input thread**, in the window procedure, one message
/// after the hotkey. `[selection]` lives in the configuration, the configuration belongs to the
/// UI thread (section 6.1) and section 6.3 says a value that crosses threads is *published*
/// rather than fetched — the same route `[replacement]`, `[layouts]` and `[exclusions]` already
/// take.
///
/// It matters more here than for any of those three, and the reason is FR-65 itself. With the
/// path switched off the program has to behave **exactly** as it did before this task: the
/// hotkey goes down the typing-buffer path, on the input thread, with nothing waited for and
/// nobody asked. A flag the input thread could only learn by asking the UI thread would make a
/// disabled feature depend on the availability of a thread it must not need — and NFR-09 gives
/// the whole typing-buffer path thirty milliseconds.
static PATH_ENABLED: AtomicBool = AtomicBool::new(true);

/// `[selection] clipboard_timeout_ms` — step 3 of FR-61, published for the same reason.
static TIMEOUT_MS: AtomicU32 = AtomicU32::new(0);

/// `[selection] clipboard_restore_delay_ms` — step 8 of FR-61.
static RESTORE_DELAY_MS: AtomicU32 = AtomicU32::new(0);

/// Whether a published value has ever been stored.
///
/// Distinguishes "the configuration says zero" from "nothing has been published yet"; without
/// it, a program whose UI thread has not reached the configuration would run step 3 with a zero
/// timeout, which answers `TimedOut` before the application has had any chance to copy.
static PUBLISHED: AtomicBool = AtomicBool::new(false);

/// Publishes `[selection]` of section 7 — FR-65 and the two timings of steps 3 and 8.
///
/// Called by the UI thread from `app::publish_configuration_to_input_thread`, beside the
/// publications of `[replacement]`, `[layouts]` and `[exclusions]`. Three plain stores; nothing
/// is applied to anything already built, so the next press reads whatever stands here then.
pub fn publish(selection: &Selection) {
    TIMEOUT_MS.store(selection.clipboard_timeout_ms, Ordering::Relaxed);
    RESTORE_DELAY_MS.store(selection.clipboard_restore_delay_ms, Ordering::Relaxed);
    PUBLISHED.store(true, Ordering::Release);

    // Last, and with a release fence behind it: a thread that sees the path enabled must also
    // see the timings that go with it.
    PATH_ENABLED.store(is_enabled(selection), Ordering::Release);
}

/// Whether the selection path is switched on — **FR-65**, as the input thread reads it.
pub fn path_enabled() -> bool {
    PATH_ENABLED.load(Ordering::Acquire)
}

/// The timeout of step 3 as it stands, or the default of section 7 before anything is published.
pub fn published_timeout() -> Duration {
    if PUBLISHED.load(Ordering::Acquire) {
        Duration::from_millis(u64::from(TIMEOUT_MS.load(Ordering::Relaxed)))
    } else {
        timeout_of(&Selection::default())
    }
}

/// The delay of step 8 as it stands, or the default of section 7.
pub fn published_restore_delay() -> Duration {
    if PUBLISHED.load(Ordering::Acquire) {
        Duration::from_millis(u64::from(RESTORE_DELAY_MS.load(Ordering::Relaxed)))
    } else {
        restore_delay_of(&Selection::default())
    }
}

// ---------------------------------------------------------------------------------------
// Steps 2 and 6 — `Ctrl+C` and `Ctrl+V`
// ---------------------------------------------------------------------------------------

/// How many `INPUT` structures one chord takes: modifier down, key down, key up, modifier up.
pub const CHORD_EVENTS: usize = 4;

/// One keyboard `INPUT` for a virtual key — **FR-03**.
///
/// ⚠ **Why this exists here rather than in `inject`.** Module `inject` owns the two `INPUT`
/// builders of this program and says so in its own documentation; this is a third, and it is
/// here only because `src\inject.rs` is closed to task T-07-2 and its builders are private. The
/// one thing that must not differ is the one thing FR-03 is about: `dwExtraInfo` is
/// [`INJECTED_SIGNATURE`], the same constant, taken from the same module — so `hook::classify`
/// recognises these four events as this program's own, passes them to the application and keeps
/// them out of the typing buffer. `tests\selection.rs` checks every field of every event this
/// function produces against `inject`'s own rules.
///
/// `time` is zero so that the system stamps the event, which FR-12 needs to be able to compare
/// timestamps; `wScan` is zero so that the system derives the scan code from `wVk` in the layout
/// of the receiving thread, which this thread does not know.
fn key_event(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);

    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_SIGNATURE,
            },
        },
    }
}

/// Writes `Ctrl` + `key` into `out` and answers how many events it wrote.
///
/// Four events in the order a person makes them: the modifier goes down, the key goes down and
/// up, the modifier comes up. The chord is **balanced inside the packet** — the only `Ctrl`
/// events in it are one down and one up, in that order — which is what leaves the keyboard as it
/// was found: a `Ctrl` of ours left down would stay down on a machine whose owner is holding
/// nothing. (Until task T-40-3 step 6 of FR-40 would even have read it as the user's and pressed
/// it again; step 6 is not performed now — finding С12 — and this balance is the whole of the
/// hygiene.)
///
/// `VK_CONTROL` and not `VK_LCONTROL`: the aggregate is what an application testing
/// `GetKeyState(VK_CONTROL)` for its own accelerator sees, and the two `Ctrl` keys of the
/// modifier table of `inject` are about the keys the **user** is physically holding.
fn build_chord(key: VIRTUAL_KEY, out: &mut [INPUT]) -> usize {
    if out.len() < CHORD_EVENTS {
        return 0;
    }

    out[0] = key_event(VK_CONTROL, false);
    out[1] = key_event(key, false);
    out[2] = key_event(key, true);
    out[3] = key_event(VK_CONTROL, true);

    CHORD_EVENTS
}

/// The four events of `Ctrl+C` — **step 2 of FR-61**.
pub fn copy_chord() -> [INPUT; CHORD_EVENTS] {
    let mut events = [INPUT::default(); CHORD_EVENTS];
    let _ = build_chord(VK_C, &mut events);

    events
}

/// The four events of `Ctrl+V` — **step 6 of FR-61**.
pub fn paste_chord() -> [INPUT; CHORD_EVENTS] {
    let mut events = [INPUT::default(); CHORD_EVENTS];
    let _ = build_chord(VK_V, &mut events);

    events
}

/// Whether the system took every event of a chord — **finding Н17 of the audit of 2026-09-04,
/// task T-38-5**.
///
/// `dispatched` is what [`crate::inject::dispatch`] answered for [`copy_chord`] or [`paste_chord`].
/// «Taken whole» is more than [`Dispatched::is_complete`]: a chord is never empty, and an answer
/// that requested nothing — zero taken of zero — sent nothing, which is the trap finding Н18 names
/// in module `inject`. The sum over portions is what counts, so the opt-in pause of FR-44 changes
/// nothing here.
pub const fn chord_delivered(dispatched: Dispatched) -> bool {
    dispatched.requested > 0 && dispatched.accepted == dispatched.requested
}

// ---------------------------------------------------------------------------------------
// Step 5 of FR-61 — the only place in the program where content is looked at
// ---------------------------------------------------------------------------------------

/// The result of step 5, and the buffer holding it.
///
/// The text is overwritten with zeroes when this value is dropped — the treatment
/// [`crate::buffer`] gives the typing buffer for SEC-02 and [`Snapshot`] gives captured
/// clipboard blocks. It is the user's text and it lives in this process for a few hundred
/// milliseconds; it does not have to be left in the allocator afterwards.
pub struct Recoded {
    text: String,
    target: LayoutId,
    mapped: usize,
    carried: usize,
}

impl Recoded {
    /// The recoded text — what step 6 puts on the clipboard.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The layout step 7 switches the foreground window to.
    pub fn target(&self) -> LayoutId {
        self.target
    }

    /// Characters that had a reverse mapping in the source layout and were recoded.
    pub fn mapped(&self) -> usize {
        self.mapped
    }

    /// Characters carried over unchanged — **FR-23 read backwards**, see [`recode`].
    pub fn carried(&self) -> usize {
        self.carried
    }
}

impl fmt::Debug for Recoded {
    /// Counts, never characters — SEC-01, SEC-07. A derived `Debug` would print the text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recoded")
            .field("chars", &self.text.chars().count())
            .field("target", &self.target)
            .field("mapped", &self.mapped)
            .field("carried", &self.carried)
            .finish()
    }
}

impl Drop for Recoded {
    fn drop(&mut self) {
        // SAFETY: every byte of the string is overwritten with `0x00`, and a run of NUL bytes is
        // valid UTF-8, so the invariant `String` carries is preserved for the rest of its life —
        // which is the few instructions between this line and the deallocation. The length is not
        // changed, no byte is added or removed, and the vector is not reallocated;
        // `crate::buffer::zero_slice` writes into the elements of the slice it is handed and does
        // nothing else. Task T-13-15 made that write volatile: a `fill(0)` in a destructor, in
        // front of the free that follows it, is the dead store `buffer.rs` warns about.
        crate::buffer::zero_slice(unsafe { self.text.as_mut_vec() }.as_mut_slice());
    }
}

/// Whether `ch` is worth counting when the script of a text is being decided.
///
/// Spaces, line breaks and control characters carry no information about which layout the text
/// was typed under: every layout has a space bar and the reverse index of every layout answers
/// for it. Counting them would add the same number to every candidate and dilute the letters
/// that actually decide.
fn decides_script(ch: char) -> bool {
    !ch.is_whitespace() && !ch.is_control()
}

/// **Which layout the text was typed under — the script test of step 5 of FR-61.**
///
/// Returns an index into `maps`, or `None` when nothing decides and `fallback` is not among the
/// candidates either.
///
/// # How the script is decided, and why it is counted rather than classified by Unicode block
///
/// FR-61 says «определить письменность текста (кириллица / латиница)». The direct reading —
/// count Cyrillic code points against Latin ones — answers that one question and no other: it
/// would have to be rewritten the day a third layout is Greek, and it would say nothing at all
/// about digits, punctuation or the `ё`/`` ` `` key.
///
/// What is counted instead is the property the *program* cares about, and for the RU/EN pair it
/// gives exactly the answer the requirement asks for. A character **distinguishes** a layout
/// when that layout's reverse index answers for it and no other candidate's does. `привет` is
/// six characters that only the Russian layout can produce, `ghbdtn` is six only the English one
/// can, and `123` is three that both produce and that therefore decide nothing. The candidate
/// with the strict maximum of distinguishing characters is the source layout.
///
/// # The tie, and what it means
///
/// A tie — no distinguishing characters at all (`123`, `!!!`, an empty selection) or the same
/// number for two candidates (`ghbdtn привет`, six each) — is not an error and not a guess. It
/// is resolved by `fallback`, the layout of the foreground window at the moment the hotkey was
/// pressed, which is the same input FR-52 already uses and the closest thing to the user's
/// intent that exists without asking them. A `fallback` that is not one of the candidates
/// (FR-30: «остальные раскладки игнорируются») leaves the answer `None`, and step 5 then does
/// nothing rather than convert in a direction nobody chose.
pub fn detect_source(text: &str, maps: &[LayoutMap], fallback: LayoutId) -> Option<usize> {
    if maps.is_empty() {
        return None;
    }

    let mut scores = vec![0usize; maps.len()];

    for ch in text.chars().filter(|&ch| decides_script(ch)) {
        let mut only = None;

        for (index, map) in maps.iter().enumerate() {
            if map.find_key(ch).is_none() {
                continue;
            }

            if only.is_some() {
                // A second candidate produces it, so it distinguishes nobody.
                only = None;
                break;
            }

            only = Some(index);
        }

        if let Some(index) = only {
            scores[index] += 1;
        }
    }

    let best = scores.iter().copied().max().unwrap_or(0);
    let winners = scores.iter().filter(|&&score| score == best).count();

    if best > 0 && winners == 1 {
        return scores.iter().position(|&score| score == best);
    }

    maps.iter().position(|map| map.layout() == fallback)
}

/// The decimal separator of the numeric keypad — `VK_DECIMAL`, which reports the scan code `0x53`
/// in the plain half of the cache (see the module documentation of `layouts`), with no modifier.
/// Question **146.1**, task **T-87-1**.
const KEYPAD_DECIMAL: KeyPress = KeyPress {
    scan: 0x53,
    extended: false,
    mods: Mods::NONE,
};

/// **Which key of `source` typed `ch`, standing between `before` and `after`** — the backward
/// lookup of step 5 of FR-61, made here and nowhere else. Task **T-87-1**, question **146.1**.
///
/// The answer is that of [`LayoutMap::find_key`] — the plainest key, the main block before the
/// keypad — with one exception. The character the keypad decimal key gives in `source`, standing
/// between two digits `0`…`9`, is that key: `,` of Russian, `.` of US. The reverse index cannot say
/// so by itself: it keeps the first writer, and the keys are walked in ascending order, so Russian
/// `,` is found on `Shift` + the `/?` key, which is `?` in US, and US `.` on the `.>` key, which is
/// `ю` in Russian. The owner's finding of 2026-09-25 was exactly that — `афиду 5,1` came out as
/// `fable 5?1` — and between two digits a separator is what the keypad was pressed for: `5,1` ↔
/// `5.1`. Everywhere else the reverse index stands, and `црфе,` is still `what?`.
///
/// The typing-buffer path never comes here: it holds the scan code the hook saw, `0x53` included.
/// A map with nothing on the keypad key — the hardwired table of FR-25 — has no key to name, and
/// the answer stays that of the reverse index.
///
/// ⚠ **The one place the key is picked.** [`recode_into`] writes by it, and [`recoded_len`] and
/// [`units_bound`] count by it ahead of the writing. Were one of them to ask the reverse index
/// itself, the vector made at the count would grow between two digits and leave a copy of the
/// user's text behind — task **T-38-9B**, finding **Н34**; `tests\selection.rs` sweeps the three.
fn key_between(
    source: &LayoutMap,
    before: Option<char>,
    ch: char,
    after: Option<char>,
) -> Option<KeyPress> {
    let digit = |neighbour: Option<char>| neighbour.is_some_and(|ch| ch.is_ascii_digit());

    if digit(before) && digit(after) {
        let keypad = source.lookup(
            KEYPAD_DECIMAL.scan,
            KEYPAD_DECIMAL.extended,
            KEYPAD_DECIMAL.mods,
        );

        // Dead keys stay out, for the reason the reverse index keeps them out of itself.
        if !keypad.is_dead() && keypad.single_char() == Some(ch) {
            return Some(KEYPAD_DECIMAL);
        }
    }

    source.find_key(ch)
}

/// Every character of `text` with the one before it and the one after it — what [`key_between`]
/// decides by. Task **T-87-1**.
///
/// One walk over `chars()` with a character of memory and one of look-ahead, both held by the walk
/// itself: no `Vec<char>` and no second `String` of the user's text is made for the neighbours —
/// SEC-01, SEC-02, and the copies `tests\selection.rs` names stay nine.
fn with_neighbours(text: &str) -> impl Iterator<Item = (Option<char>, char, Option<char>)> + '_ {
    let mut before = None;
    let mut chars = text.chars().peekable();

    core::iter::from_fn(move || {
        let ch = chars.next()?;
        let neighbours = (before, ch, chars.peek().copied());

        before = Some(ch);

        Some(neighbours)
    })
}

/// **Recodes `text` from `source` into `target` — the second half of step 5 of FR-61.**
///
/// Two lookups per character and nothing else:
///
/// 1. **backwards**, [`key_between`] — which physical key produces this character in the
///    layout the text was typed under. This is the step the main path never has to take: the
///    typing buffer *holds* scan codes (FR-04), and FR-32 rests on that. Here there is only
///    text, so the scan code is reconstructed from the character, and that reconstruction is
///    ambiguous in principle — the specification says as much where it says «скан-коды здесь
///    недоступны». The reverse index answers with the plainest key; a separator between two
///    digits is the keypad key since task **T-87-1**;
/// 2. **forwards**, [`crate::convert::convert_stroke`] — what the same physical key gives in the
///    target layout. That is FR-22 unchanged, the accepted engine of section 11.1, reached
///    through its own public function and not re-implemented here.
///
/// # A character with no reverse mapping
///
/// It is **carried over unchanged**, and conversion continues — FR-23 read backwards, and the
/// same answer the forward direction gives to a key the target layout has nothing on. Three
/// kinds of character arrive here: text of the other script (the Cyrillic half of a mixed
/// selection, once the source has been decided as Latin), characters no keyboard layout
/// produces at all (an em dash, a currency sign, an emoji), and characters of a layout that is
/// not a participant. Dropping any of them would silently delete the user's text; refusing the
/// whole selection over one of them would make the feature useless on any real sentence.
///
/// Allocates the code units and the string once each, at the sizes [`recoded_len`] and
/// [`string_from_units`] count before filling them — task **T-38-9B** — and nothing per character.
pub fn recode(text: &str, source: &LayoutMap, target: &LayoutMap) -> Recoded {
    // Finding Н34: the old estimate was the length of `text` in bytes, and a key may make up to
    // `MAX_UNITS` code units — a ligature grew this vector twice, leaving the text behind each time.
    let mut units: Vec<u16> = Vec::with_capacity(recoded_len(text, source, target));

    let (mapped, carried) = recode_into(text, source, target, &mut units);

    let recoded = string_from_units(&units);

    // The working buffer held the user's text — SEC-01, SEC-02. Volatile since task T-13-15:
    // `units` is dropped two lines below, and a plain `fill` in front of that is a dead store.
    crate::buffer::zero_slice(&mut units);

    Recoded {
        text: recoded,
        target: target.layout(),
        mapped,
        carried,
    }
}

/// Appends `text` recoded from `source` into `target` onto `units`, and answers the two counts.
///
/// The two lookups of [`recode`] and nothing else, in a shape both [`recode`] and
/// [`recode_words`] can call: the first converts a whole selection in one direction, the second
/// calls this once per word with a direction of that word's own. Task **Т-48-3**.
fn recode_into(
    text: &str,
    source: &LayoutMap,
    target: &LayoutMap,
    units: &mut Vec<u16>,
) -> (usize, usize) {
    let mut mapped = 0usize;
    let mut carried = 0usize;

    for (before, ch, after) in with_neighbours(text) {
        match key_between(source, before, ch, after) {
            Some(key) => {
                let stroke = Keystroke::recorded_in(source, key.scan, key.extended, key.mods);
                units.extend_from_slice(crate::convert::convert_stroke(stroke, target).units());
                mapped += 1;
            }
            None => {
                let mut buffer = [0u16; 2];
                units.extend_from_slice(ch.encode_utf16(&mut buffer));
                carried += 1;
            }
        }
    }

    (mapped, carried)
}

/// The number of code units [`recode`] writes for `text` — the walk of [`recode_into`] without the
/// writing, the question [`crate::convert::converted_len`] answers for strokes. **Task T-38-9B,
/// finding Н34**: a vector made at this capacity is filled without growing.
fn recoded_len(text: &str, source: &LayoutMap, target: &LayoutMap) -> usize {
    with_neighbours(text)
        .map(
            |(before, ch, after)| match key_between(source, before, ch, after) {
                Some(key) => {
                    let stroke = Keystroke::recorded_in(source, key.scan, key.extended, key.mods);
                    crate::convert::convert_stroke(stroke, target).units().len()
                }
                None => ch.len_utf16(),
            },
        )
        .sum()
}

/// The most code units [`recode_words`] can write for `text` under the layouts of a plan — **task
/// T-38-9B, finding Н34**.
///
/// For each character, the larger of what it is as it stands and of what the key [`key_between`]
/// names for it in any of `maps` makes in any of `maps`: the word it stands in is either recoded
/// between two of them or carried over, and both answers are among those counted. Where every key
/// of the layouts makes one unit, as the layouts of a pair of real scripts do, this is the exact
/// count; a ligature on a key of a layout the text is not in makes it larger than needed, and
/// never smaller.
///
/// The neighbours are those of the whole text, where [`recode_into`] sees those of one word, and
/// the two agree: a word ends at a separator or at an end of the text, and neither is a digit —
/// task **T-87-1**.
///
/// # Why not the exact count, the way [`recoded_len`] gives it for [`recode`]
///
/// The exact count needs the source and the target of every word, and the target is
/// [`target_for`], which asks [`crate::layouts::Cycle::target`] — and that function counts every
/// refusal it makes. A counting pass that decided every word a second time would count every
/// refused word twice. This bound decides nothing and counts nothing.
fn units_bound(text: &str, maps: &[LayoutMap]) -> usize {
    with_neighbours(text)
        .map(|(before, ch, after)| {
            maps.iter()
                .filter_map(|source| {
                    key_between(source, before, ch, after).map(|key| (source, key))
                })
                .flat_map(|(source, key)| {
                    let stroke = Keystroke::recorded_in(source, key.scan, key.extended, key.mods);

                    maps.iter().map(move |target| {
                        crate::convert::convert_stroke(stroke, target).units().len()
                    })
                })
                .fold(ch.len_utf16(), usize::max)
        })
        .sum()
}

/// Appends `text` onto `units` exactly as it stands, and answers how many characters that was.
///
/// The separators of step 5 — task **Т-48-3**. They belong to no layout: every layout has a
/// space bar and a line break is not a key at all, so putting them through the two lookups
/// would be asking a question with no answer and counting the reply as a conversion.
fn carry_into(text: &str, units: &mut Vec<u16>) -> usize {
    let mut carried = 0usize;

    for ch in text.chars() {
        let mut buffer = [0u16; 2];
        units.extend_from_slice(ch.encode_utf16(&mut buffer));
        carried += 1;
    }

    carried
}

/// Whether `ch` separates two words — step 5 of FR-61 since task **Т-48-3**.
///
/// The same test [`decides_script`] applies to a character before counting it, and deliberately
/// the same one: a character that says nothing about which layout the text was typed under is a
/// character that cannot belong to a word for the purpose of deciding that word's layout.
fn separates_words(ch: char) -> bool {
    !decides_script(ch)
}

/// **Recodes `text` word by word, each word out of its own layout** — step 5 of FR-61 since
/// task **Т-48-3**, question **111.3**.
///
/// `fallback` is the source layout decided over the **whole** text, and it does two jobs: it is
/// what a word with nothing to decide by follows, and it is what step 7 switches by — see
/// [`recode_with`], which owns that half.
///
/// # Why the script is decided per word and not once
///
/// The user's third finding, word for word — 2026-09-09:
///
/// > если набрать текст: `z [jntk yfgbcfnm ckjdj нфтвуч yj pf,sk cvtybnm` затем выделить
/// > набранный текст и нажать pause, то получится: `я хотел написать слово нфтвуч но забыл
/// > сменить`
///
/// Twenty-nine Latin characters against six Cyrillic ones: the strict maximum of
/// [`detect_source`] named English, the whole selection was recoded English → Russian, and the
/// one Russian word had no reverse mapping in the English map. FR-23 read backwards then carried
/// it over untouched — correctly, by the rule as it stood, and uselessly, by what the user
/// meant. A sentence typed in two layouts is two sentences as far as the reverse lookup is
/// concerned, and the unit that has one layout is the **word**.
///
/// # The four answers, in order
///
/// 1. **a word that distinguishes a layout** goes to the next layout after **that** one — FR-30,
///    FR-31, the cycle of §4.4 asked from the word's own source and not from the text's;
/// 2. **a word that distinguishes nothing** — digits, punctuation — follows `fallback`, which is
///    the decision over the whole text. `123` beside `ghbdtn` goes where `ghbdtn` goes;
/// 3. **a word whose source has no target in the cycle** is carried over unchanged, which is the
///    refusal of FR-30 applied to one word instead of to the selection;
/// 4. **separators** — whitespace and control characters — are carried over exactly as they
///    stand, two spaces as two spaces and a line break as a line break.
///
/// Allocates the code units once, at [`units_bound`], and the string once, at the length
/// [`string_from_units`] counts — task **T-38-9B** — and one small score vector per word inside
/// [`detect_source`]. This runs on the UI thread in answer to a hotkey, never in
/// the hook callback: NFR-01 to NFR-05 are about that callback and are untouched here.
fn recode_words(text: &str, plan: &Plan, fallback: &LayoutMap, target: &LayoutMap) -> Recoded {
    // Finding Н34: made at the most code units the text can come to, counted without deciding a
    // single word — see [`units_bound`] for why not the exact count.
    let mut units: Vec<u16> = Vec::with_capacity(units_bound(text, &plan.maps));
    let mut mapped = 0usize;
    let mut carried = 0usize;

    // One pass, splitting on the fly: `split_word_bounds`-style crates are not in this program
    // and are not needed — the boundary of step 5 is "whitespace or control", which
    // `char::is_whitespace` and `char::is_control` answer between them.
    for piece in text.split_inclusive(separates_words) {
        // `split_inclusive` hands back the separator attached to the end of the piece before it,
        // so each piece is a word followed by at most one separator. Splitting it here keeps the
        // two rules apart without a second pass over the text.
        let end = piece
            .char_indices()
            .rev()
            .find(|&(_, ch)| !separates_words(ch))
            .map_or(0, |(index, ch)| index + ch.len_utf8());

        let (word, separator) = piece.split_at(end);

        if !word.is_empty() {
            let source = detect_source(word, &plan.maps, fallback.layout())
                .and_then(|index| plan.maps.get(index));

            match source.and_then(|source| target_for(source, plan)) {
                Some((source, word_target)) => {
                    let (word_mapped, word_carried) =
                        recode_into(word, source, word_target, &mut units);

                    mapped += word_mapped;
                    carried += word_carried;
                }
                // Answer 3: no source, or a source the cycle has no target for. The word is the
                // user's text and is handed back as it stands rather than dropped.
                None => carried += carry_into(word, &mut units),
            }
        }

        carried += carry_into(separator, &mut units);
    }

    let recoded = string_from_units(&units);

    // SEC-01, SEC-02 — the same wipe [`recode`] does, and for the same reason.
    crate::buffer::zero_slice(&mut units);

    Recoded {
        text: recoded,
        target: target.layout(),
        mapped,
        carried,
    }
}

/// The map one step along the cycle from `source` — FR-30, FR-31 — paired with `source` itself.
///
/// `None` is the refusal of FR-30 («остальные раскладки игнорируются») and of FR-35, both
/// counted inside module `layouts`. Split out so that [`recode_words`] and [`recode_with`] ask
/// the question in exactly one way.
fn target_for<'plan>(
    source: &'plan LayoutMap,
    plan: &'plan Plan,
) -> Option<(&'plan LayoutMap, &'plan LayoutMap)> {
    let target = plan.cycle.target(source.layout(), 1).ok()?;
    let target = plan.maps.iter().find(|map| map.layout() == target)?;

    Some((source, target))
}

// ---------------------------------------------------------------------------------------
// What one press of the hotkey hands over to the UI thread
// ---------------------------------------------------------------------------------------

/// Everything the selection path needs, decided on the input thread and carried to the UI one.
///
/// # Why the maps travel rather than being built where they are used
///
/// The mapping cache of FR-20 belongs to the input thread: section 6.3 gives it to the thread
/// that owns the typing buffer, FR-21 rebuilds it there on `WM_INPUTLANGCHANGE`, and no other
/// thread can reach a thread-local. The UI thread could sweep `ToUnicodeEx` for itself, which is
/// thousands of calls and about five milliseconds — for a second copy of a table that already
/// exists and that FR-21 would then have to keep in step twice.
///
/// So the press copies the participating layouts out of the live cache, exactly as
/// `inject::take_press` copies the one map the typing-buffer path needs, and hands them over.
/// Which of them the text was typed under is **not** decided here: that is step 5, it needs the
/// text, and the text does not exist until step 4 has run.
///
/// Public with a constructor so that `tests\selection.rs` can build one out of the hardwired
/// RU/EN maps of FR-25 ([`crate::convert::fallback_map`]) and drive the eight steps without a
/// keyboard, a clipboard or a foreground window.
pub struct Plan {
    /// The participating layouts of section 4.4, in the order of the cycle.
    maps: Vec<LayoutMap>,
    /// The cycle those layouts form — FR-30, FR-31, so that step 5 asks it for the target
    /// instead of restating the rule.
    cycle: Cycle,
    /// The layout of the foreground window when the hotkey was pressed — the tie-break of
    /// [`detect_source`] and the origin FR-26 would use if there were strokes.
    foreground: LayoutId,
    /// Step 3 of FR-61, out of section 7.
    timeout: Duration,
    /// Step 8 of FR-61, out of section 7.
    restore_delay: Duration,
    /// FR-44, so that the two chords are paced the way every other injection of this program is.
    delay_ms: u32,
}

impl Plan {
    /// A plan over `maps`, whose order is the order of `cycle`.
    pub fn new(
        maps: Vec<LayoutMap>,
        cycle: Cycle,
        foreground: LayoutId,
        timeout: Duration,
        restore_delay: Duration,
        delay_ms: u32,
    ) -> Self {
        Self {
            maps,
            cycle,
            foreground,
            timeout,
            restore_delay,
            delay_ms,
        }
    }
}

/// The plan the input thread published, waiting for the UI thread to take it.
///
/// A mutex and not a set of atomics, unlike [`crate::switch::publish_pending`]: what travels
/// here is a `Vec` of layout maps and not a handle, and there is no atomic of one. It is locked
/// on the input thread in its **message loop** — never in the hook callback, which is the whole
/// of NFR-04 — and held for the length of one `Option::take`.
static PENDING: Mutex<Option<Plan>> = Mutex::new(None);

/// Stores `plan`, replacing whatever the previous press left.
fn publish_pending(plan: Plan) {
    let mut slot = PENDING.lock().unwrap_or_else(PoisonError::into_inner);

    *slot = Some(plan);
}

/// Takes the plan, if there is one.
fn take_pending() -> Option<Plan> {
    let mut slot = PENDING.lock().unwrap_or_else(PoisonError::into_inner);

    slot.take()
}

/// Builds the plan out of the live cache — the input thread's half of the hand-over.
///
/// `None` when there is nothing to hand over, and then FR-60 has nothing to decide: no typing
/// buffer on this thread, installed or parked (which is every thread but the input one), no
/// mapping cache yet, or a refusal from module `layouts` — an IME among the participants (FR-35),
/// fewer than two layouts, or a foreground layout that is not a participant (FR-30). Every one of
/// those is counted where it is decided.
///
/// Crate-visible rather than private for one reason: the check of task T-40-6 lives beside the
/// gate of FR-70 in `app.rs`, which is the only place a recorder can be parked from.
///
/// # ⭐ Task T-40-6, finding С56 — the recorder is reached wherever FR-70 keeps it
///
/// The plan needs the cache of FR-20 and nothing else from the recorder, and after every focus
/// change the recorder is **parked** — `app::park_buffer` takes it off the thread until the verdict
/// of FR-72 comes back, for up to `guard::PROBE_BUDGET_MS`. Asked through [`crate::buffer::with`]
/// alone, the plan was not built in that interval, and a press in it did nothing: exactly when the
/// user has just clicked into another window, selected text and pressed the hotkey. The same trap
/// was found for the layout probe of FR-21 and repaired in task T-10-0f by
/// `app::with_recorder_wherever_it_is`; this is its second caller, and there is still one copy of
/// the helper. The gate half of the finding — `buffer::is_installed` in the condition of the hotkey
/// branch — had already moved in task Т-49-2.
///
/// # SEC-05, SEC-06 and FR-70 — the analysis repeated for this caller
///
/// * **FR-70 is not weakened.** What is read off a parked recorder is its cache and nothing else:
///   the ring was emptied and zeroed by `park_buffer` before the recorder left the thread
///   (SEC-02), nothing here records, and a recorder that is not installed cannot be recorded into.
/// * **SEC-05 — a forged `WM_APP_HOTKEY`.** Since task Т-49-2 this function is reached on every one
///   of the three windows such a message can land on. On the UI and watcher threads nothing was
///   ever installed or parked — `PARKED_BUFFER` is a thread-local of `app`, and only the input
///   thread's gate writes into it — so the answer there is `None`, exactly as it was. On the input
///   window a forged press now buys, during the interval of FR-71, what a forged press buys outside
///   it and what the user's own press buys: the selection path of FR-61 over the window in front.
///   Repetition changes nothing about that — the argument of SEC-05 for `WM_APP_FLUSH` (task
///   T-41-13).
/// * **SEC-06 — the `Ctrl+C` of step 2 into a field with no verdict yet.** In the interval the field
///   is `guard::Field::Pending`, and `guard::password_field` answers `false` for it, so nothing
///   upstream stops the probe going into a password field whose verdict is still on its way. Before
///   this task the one thing that did was this function answering `None` — which is why the
///   stage measured, before touching this line, whether `Ctrl+C` puts the contents of a password
///   field on the clipboard at all: premise П5, the `ES_PASSWORD` edit of a stand, a
///   `type="password"` field in Chrome and the password field of an installed application —
///   **not in one of them** (decision 127а). A probe that copies nothing is answered by step 3 as
///   «no selection», the press is handed back, and the parked buffer answers it as it answers
///   every press in a password field. FR-71 and the budget of the probe are untouched.
pub(crate) fn plan_for_press() -> Option<Plan> {
    let foreground = crate::switch::current();

    crate::app::with_recorder_wherever_it_is(|recorder| {
        let cache = recorder.cache()?;

        let mut available = [LayoutId::default(); crate::layouts::MAX_CYCLE];
        let count = cache.layouts(&mut available);

        // **Task Т-22-5**, as in `inject::take_press`: whether the array above holds the session
        // or only the front of it. See `layouts::Session`.
        let session = crate::layouts::Session::of(count, cache.len());

        let cycle =
            crate::layouts::cycle_for(crate::layouts::published(), &available[..count], session)
                .ok()?;

        let mut maps = Vec::with_capacity(cycle.len());
        for &layout in cycle.layouts() {
            maps.push(cache.get(layout)?.clone());
        }

        Some(Plan {
            maps,
            cycle,
            foreground,
            timeout: published_timeout(),
            restore_delay: published_restore_delay(),
            delay_ms: crate::inject::inter_event_delay_ms(),
        })
    })
    .flatten()
}

// ---------------------------------------------------------------------------------------
// FR-61 — the eight steps, against a seam
// ---------------------------------------------------------------------------------------

/// Everything the eight steps need from outside this module.
///
/// # Why it is a trait
///
/// FR-61 is an **order**, and an order is not observable from outside a function that performs
/// it — the same argument module `inject` makes for [`crate::inject::Environment`] and FR-40.
/// With the outside world behind this trait, `tests\selection.rs` can watch the sequence the
/// eight steps really produce, and can make any one of them fail and see what the rest do —
/// which is the only honest way to check that **step 8 runs when steps 4 to 7 did not**.
///
/// The member that is not a numbered step is step 3 of **FR-40**: the modifiers the user is
/// holding come off before anything is injected. Without it a `Shift` held over the hotkey would
/// turn the `Ctrl+C` of step 2 into `Ctrl+Shift+C`, which is a different command in half the
/// applications on the machine.
///
/// ⛔ **There is no member for step 6 of FR-40 since task T-40-3** (finding С12, decision 127.2).
/// It was `restore_modifiers`, «press again whatever the user is holding now», and on the real
/// machine it could only ask the asynchronous key state — which had followed step 3's own release
/// and reported the user's keys as up. §10 of SPEC names the limitation.
pub trait Path {
    /// **Step 1** — the snapshot of FR-64.
    fn snapshot(&mut self) -> Result<Snapshot, ClipboardError>;

    /// **FR-40 step 3** — release the modifiers the user is holding. Answers which.
    fn release_modifiers(&mut self) -> Modifiers;

    /// **Step 2** — `Ctrl+C`, through `inject`, with the signature of FR-03.
    ///
    /// Answers what the system took, and [`run`] reads the answer: a chord not taken whole is a
    /// refusal before step 3 — see [`chord_delivered`], task T-38-5.
    fn copy(&mut self) -> Dispatched;

    /// **Step 3** — wait for the clipboard sequence number to leave `baseline`.
    fn wait(&mut self, baseline: u32) -> Wait;

    /// **Not a numbered step** — the clipboard sequence number as it stands *now*.
    ///
    /// Step 3 asks whether the number left the baseline **within** the timeout; this asks for the
    /// number outside that wait — after the timeout gave up, or after step 3 or step 2 was
    /// refused, which is the late `Ctrl+C` of the audit of 2026-08-24 (tasks T-13-11, T-38-4 and
    /// T-38-5), and after step 4 read the clipboard, where the answer to the probe ends (task
    /// T-38-2). It is behind the trait for the reason every other member is: a test that had to
    /// move the machine's real clipboard to reach those branches would be a test of the machine.
    fn sequence(&mut self) -> u32;

    /// **Step 4** — read `CF_UNICODETEXT`.
    ///
    /// The text comes in a [`ClipboardText`], which zeroes it when it is dropped — finding С29,
    /// task T-38-9A.
    fn read(&mut self) -> Result<Option<ClipboardText>, ClipboardError>;

    /// **Step 6, first half** — put the recoded text on the clipboard.
    fn write(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// **Step 6, second half** — `Ctrl+V`.
    ///
    /// Answers what the system took, and [`run`] reads the answer: a paste not taken whole is a
    /// refusal, and step 8 is owed as on every refusal after step 3 — see [`chord_delivered`],
    /// task T-38-5.
    fn paste(&mut self) -> Dispatched;

    /// **Step 7** — switch the layout of the foreground window, §4.6.
    fn switch(&mut self, target: LayoutId);

    /// **Step 8** — put the user's clipboard back, after the delay of section 7.
    ///
    /// `answer` is the run of sequence numbers the clipboard took while the application answered
    /// the `Ctrl+C` of step 2: from the number step 3 saw to the number standing after step 4 read
    /// the clipboard. It travels this far because the gate of decision П-5 is at the far end of
    /// the delay and needs to know which changes are this pass's own — see [`restore_after`], task
    /// **Т-18-1** for what the absence of the first number cost, and task **T-38-2** for what the
    /// absence of the last one did.
    fn restore_clipboard(&mut self, snapshot: &Snapshot, answer: ProbeAnswer);

    /// **Not step 8** — the user's clipboard back **at once**, after a late `Ctrl+C`.
    ///
    /// Reached from the `LateCopy` arm of [`Session`]'s `Drop` and from nowhere else: step 3 gave
    /// up or was refused (tasks T-13-11 and T-38-4), the clipboard moved anyway, and what is on it
    /// is a selection this program asked for.
    /// Two things separate it from step 8, and both are in the name:
    ///
    /// * **no delay.** The delay of step 8 exists to let a paste land first, and on this path
    ///   nothing was pasted — there is nothing to wait for, and every millisecond of waiting is a
    ///   millisecond of the user's clipboard being wrong;
    /// * **no refusal.** The note П-5 added to step 8 compares the clipboard against this
    ///   program's own write, and on this path there is no own write to compare with — see the
    ///   comment in [`run`] for what that means and why the restore is made anyway.
    fn reclaim_clipboard(&mut self, snapshot: &Snapshot);
}

/// Why the eight steps did not convert anything.
///
/// The reasons that stop a press **before** the eight steps begin — FR-65, a password field, no
/// mapping cache, no UI window — are not here: they are answered by [`wants_selection_path`] on
/// the input thread, which returns a `bool` because at that point nothing has happened yet and
/// there is nothing to describe. Every variant below is a state [`run`] can reach.
///
/// SEC-01, SEC-07: not one variant carries a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The message arrived with no plan behind it — the press was taken back, or the message was
    /// posted by somebody else (SEC-05).
    NoPlan,
    /// A clipboard call refused — a thread that may not block, or another process holding it
    /// through all ten attempts of FR-62 — or a chord of step 2 or step 6 the system did not take
    /// whole (task T-38-5).
    Clipboard,
    /// Step 4 found no `CF_UNICODETEXT`, or found it empty. The application answered `Ctrl+C`
    /// with something that is not text — a picture, a file list, a spreadsheet range.
    NoText,
    /// Step 5 could not decide a direction: nothing in the selection distinguishes a layout and
    /// the foreground window is not in one of the participants either.
    NoDirection,
}

/// What one press did — **FR-60 and FR-61 in one value**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// All eight steps ran. `mapped` and `carried` are the two halves of step 5.
    Converted { mapped: usize, carried: usize },
    /// **Step 3 timed out — there is no selection.** FR-60 sends the press down the
    /// typing-buffer path, and this is the ordinary outcome for a press made while nothing is
    /// selected, not a failure.
    NoSelection,
    /// The path did not run, or ran and could not finish. The clipboard is the user's either
    /// way — see [`Session`].
    Refused(Refusal),
}

impl Outcome {
    /// Whether the typing-buffer path of FR-60 is the one that should run now.
    ///
    /// Everything except a completed conversion: no selection is the ordinary case, and every
    /// refusal means the selection path produced nothing, so the press must still do what it did
    /// before this task existed rather than be swallowed.
    pub const fn falls_back(self) -> bool {
        !matches!(self, Self::Converted { .. })
    }
}

/// The eight steps in progress — **and the guarantee that step 8 of FR-61 runs.**
///
/// # What this type is for, and why it is a type
///
/// FR-61 step 8 says the clipboard is restored, and the task specification adds that it is
/// restored **even when something between steps 4 and 7 failed**. There are nine places in
/// [`run`] where the work can stop early — seven until task T-38-5 read the answers of steps 2
/// and 6 — plus the `?`-shaped ones inside them, plus a panic in a Debug build. A restore written
/// at each of them would be a convention: correct today, and one early `return` away from being
/// wrong.
///
/// So it is not written at any of them. The whole of the body below runs **through** this value,
/// the borrow of the [`Path`] lives in it, and the two put-backs are in `Drop`:
///
/// | Way out of [`run`] | What restores | Why |
/// |---|---|---|
/// | the conversion finished | `Drop` | end of scope |
/// | step 4 found no text | `Drop` | early `return`, scope ends |
/// | step 5 could not decide | `Drop` | early `return`, scope ends |
/// | step 6 could not write | `Drop` | early `return`, scope ends |
/// | step 6's `Ctrl+V` was not taken whole | `Drop` | early `return`, scope ends — task T-38-5 |
/// | step 2's `Ctrl+C` was not taken whole | `Drop`: nothing when the clipboard did not move, the other door when it did — task T-38-5 |
/// | step 3 said there is no selection | `Drop`, and there is usually nothing to put back — see [`Owed`] |
/// | step 3 gave up and the clipboard moved anyway | `Drop`, by the other door — [`Path::reclaim_clipboard`], task T-13-11 |
/// | step 3 was refused — the thread may not block | `Drop`: nothing when the clipboard did not move, the same other door when it did — task T-38-4 |
/// | a panic, Debug build | `Drop`, run by the unwind |
/// | a panic, Release build (`panic = "abort"`) | nothing here; the system frees the clipboard with the owning thread, measured in T-07-1 |
///
/// `tests\selection.rs` checks that mechanically: `restore_clipboard(` and `reclaim_clipboard(`
/// each appear **once** in the module outside the trait and the bench, and that once is inside
/// `impl Drop for Session`. (A third row, `restore_modifiers(` — step 6 of FR-40 — left with the
/// step itself in task T-40-3.)
///
/// ⚠ **The table above is a promise about the door, and until task Т-18-1 it was read as a
/// promise about the outcome.** It was not one: the door opened on every one of those rows and
/// the gate of decision П-5 behind it refused, because on the failure paths of steps 4 to 6 this
/// program has written nothing for [`is_own_change`] to recognise. Four rows of a table that
/// says «restores» described a clipboard that never came back. The answer [`Owed::StepEight`]
/// now carries — the first number step 3 saw since Т-18-1, and the last one step 4's read saw
/// since T-38-2 — is what makes the table true, and the price of getting it wrong again is the
/// user's clipboard — so both halves are tested: that the door opens, and that what comes
/// through it is an answer the gate accepts.
///
/// # [`Owed`], and why a restore is not always right
///
/// Before step 3 answers, the clipboard **is still the user's**: step 1 only read it and step 2
/// asked the application to copy. Putting the snapshot back at that point would replace the
/// user's clipboard with this program's copy of it — losing exactly the formats FR-64 admits it
/// cannot capture, the metafiles and the delay-rendered ones. So the restore is armed at the
/// instant the clipboard stops being the user's, which is the instant step 3 reports the
/// sequence number moved — or, since task T-13-11, the instant it turns out to have moved after
/// step 3 gave up. See [`Owed`].
struct Session<'a, P: Path> {
    path: &'a mut P,
    snapshot: Snapshot,
    /// Which restore the clipboard is owed, if any.
    owed: Owed,
    // ⛔ No `hygiene` flag since task T-40-3: it said «step 3 has taken the user's modifiers down,
    // so step 6 owes them back», and step 6 of FR-40 is not performed any more (finding С12).
}

/// What [`Session`] owes the user's clipboard when it goes out of scope.
///
/// Three states rather than the `bool` this was until task **T-13-11**, because the two restores
/// are not the same restore. They differ in **when** — after the delay of section 7, or at once —
/// and in **whether they may be refused** — the note П-5 added to step 8 can skip a restore, and
/// the late copy may not be skipped. A `bool` could carry neither difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owed {
    /// Nothing. The clipboard is still exactly what the user left there: step 1 only read it, and
    /// step 3 answered that the `Ctrl+C` of step 2 changed nothing.
    ///
    /// Putting the snapshot back here would replace the user's clipboard with this program's copy
    /// of it, losing precisely the formats FR-64 admits it cannot capture.
    Nothing,
    /// **Step 8 of FR-61.** Step 3 said the clipboard holds the selection, so what is on it is
    /// this program's to put back — after the delay of section 7, and subject to the note П-5.
    ///
    /// `answer` is the answer to the probe — the number [`Wait::Changed`] reported, widened to the
    /// number standing after step 4 read the clipboard — carried here rather than in a field of
    /// [`Session`] so that it exists exactly where it means something: there is no step 8 owed
    /// without a probe that was answered, and no answered probe that owes no step 8. Tasks
    /// **Т-18-1** and **T-38-2** — it is what lets the gate of П-5 tell this pass's own doing
    /// from a third writer's on the paths where the program never wrote at all.
    StepEight { answer: ProbeAnswer },
    /// **The late `Ctrl+C`.** Step 3 gave up, or was refused, and the clipboard moved anyway —
    /// see [`run`] and [`Session::owe_the_snapshot_if_the_clipboard_moved`].
    LateCopy,
}

impl<P: Path> Drop for Session<'_, P> {
    fn drop(&mut self) {
        // ⛔ **FR-40 step 6 used to come first here — task T-40-3, finding С12, decision 127.2.**
        // It pressed again «whatever the user is holding now», read from the asynchronous key
        // state, and on this path that state had followed step 3's own release half a second
        // earlier: the user's keys read as up and were never restored, and the only key the step
        // could press was one nobody held. What the guard owes is the clipboard, and only that.
        match self.owed {
            Owed::Nothing => {}
            Owed::StepEight { answer } => self.path.restore_clipboard(&self.snapshot, answer),
            Owed::LateCopy => self.path.reclaim_clipboard(&self.snapshot),
        }
    }
}

impl<P: Path> Session<'_, P> {
    /// **The late `Ctrl+C`** — asks the sequence number once more against `baseline`, and when it
    /// has moved arms [`Owed::LateCopy`] and counts it. Tasks T-13-11 and T-38-4.
    ///
    /// Step 2 sent a real `Ctrl+C`, and a step 3 that ends without seeing the clipboard move —
    /// because the wait ran out, or because it was refused before it began — is not a promise that
    /// nothing will happen: the application can still answer. One door for both ends of step 3, so
    /// the rule «`Ctrl+C` has gone out — the snapshot is insured» is written once and cannot be
    /// kept on one arm and forgotten on the other, which is what finding Н14 found.
    fn owe_the_snapshot_if_the_clipboard_moved(&mut self, baseline: u32) {
        if self.path.sequence() != baseline {
            LATE_COPIES.fetch_add(1, Ordering::Relaxed);

            self.owed = Owed::LateCopy;
        }
    }
}

/// **FR-61, all eight steps, in the order the requirement writes them.**
///
/// The order is the requirement, so this function is the requirement: every step is one call
/// through [`Path`], they are in the order 1, 2, 3, 4, 5, 6, 7, 8, and step 3 of FR-40 comes
/// before the first injection (step 6 of FR-40 is not performed — task T-40-3). Nothing else in
/// this module performs a step.
pub fn run<P: Path>(path: &mut P, plan: &Plan) -> Outcome {
    // ---- step 1 — save the clipboard, FR-64 ---------------------------------------------
    //
    // Before anything is sent. A snapshot taken after `Ctrl+C` would be a snapshot of the
    // selection, and the user's clipboard would be gone with nothing to put back.
    let snapshot = match path.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => return Outcome::Refused(Refusal::Clipboard),
    };

    // The baseline of step 3, read inside the same access that took the snapshot.
    let baseline = snapshot.sequence();

    let mut session = Session {
        path,
        snapshot,
        owed: Owed::Nothing,
    };

    // ---- FR-40 step 3 — the user's modifiers come off -----------------------------------
    session.path.release_modifiers();

    // ---- step 2 — Ctrl+C ----------------------------------------------------------------
    //
    // ⭐ **Finding Н17, its second variant — task T-38-5.** The answer of `SendInput` is looked
    // at. A probe the system did not take whole may never have reached the application, and
    // waiting for it would let step 3 read a stranger's copy as its answer — so the press is
    // refused here, before the wait. A chord taken in part may still have reached it, though, and
    // then `Ctrl+C` has gone out: the clipboard is asked once more through the door of T-38-4, and
    // one that moved gets the user's snapshot back at once.
    if !chord_delivered(session.path.copy()) {
        session.owe_the_snapshot_if_the_clipboard_moved(baseline);

        return Outcome::Refused(Refusal::Clipboard);
    }

    // ---- step 3 — did the clipboard move? -----------------------------------------------
    //
    // **This is the whole of FR-60's question.** There is no call that answers "does the
    // foreground window have a selection"; the answer is the result of this wait and of nothing
    // else. No change means no selection, and the press goes down the typing-buffer path.
    //
    // The number the wait answers with is **kept** — task Т-18-1. It is the first number of the
    // application's answer to the probe, and step 8's gate is the only thing that can tell that
    // answer apart from a stranger's copy two hundred milliseconds later. It used to be dropped
    // here, and the gate had nothing but the ring of FR-63 to ask, which is silent on every path
    // where step 6 never wrote. It is the **first** number and not always the last — task T-38-2,
    // at step 4 below.
    let probe = match session.path.wait(baseline) {
        Wait::Changed { sequence, .. } => sequence,
        Wait::TimedOut { .. } => {
            // ---- the late `Ctrl+C` — the audit of 2026-08-24, task T-13-11 ------------------
            //
            // A timeout is not a promise that nothing will happen. Step 2 sent a real `Ctrl+C`,
            // and a busy application — the case the three hundred milliseconds exist for — can
            // answer it after the wait has given up. So the sequence number is asked once more
            // against the mark step 1 left: if it has moved, the clipboard is no longer what the
            // user put there, and the snapshot in hand is the only copy of it left anywhere.
            //
            // Until this was here the answer was to walk away with the snapshot unread, because
            // the put-back was armed only by a `Wait::Changed`. That cost the user their
            // clipboard without a single sign of it, on the one path where the change was made
            // by this program's own probe.
            //
            // **Whose change it was cannot be told apart here, and this does not pretend to.**
            // There is no mark either way: the probe made the *application* write, not this
            // program, so the ring of FR-63 is silent for our own late copy exactly as it is for
            // a stranger's. It can only be one of those two, and putting the user's own content
            // back is the honest answer to both — it undoes damage this program did in the first
            // case, and in the second it costs a copy made in the moment since the wait's last
            // look, against a clipboard the user had before this program touched anything.
            //
            // ⚠ The window this closes is the gap between that last look and this one; a change
            // that lands later still arrives after everything here has returned. What the check
            // ends is the *structural* blindness — a path that could not restore however plainly
            // the clipboard had moved.
            session.owe_the_snapshot_if_the_clipboard_moved(baseline);

            return Outcome::NoSelection;
        }
        Wait::WrongThread => {
            // ⭐ **Finding Н14 of the audit of 2026-09-04, task T-38-4 — the same insurance, by
            // the same code.** The wait refused to begin, and today it cannot: step 1 asked the
            // same question of the same thread moments ago (`snapshot` opens the clipboard through
            // `require_blocking_thread`) and nothing between the two changes the answer. But the
            // `Ctrl+C` of step 2 has already gone out by the time step 3 is asked, and a refusal
            // that walked away unarmed would lose the user's clipboard the first time this work is
            // moved to a thread where the two answers differ. So the clipboard is asked once more,
            // exactly as after a timeout, through the one door both arms share.
            session.owe_the_snapshot_if_the_clipboard_moved(baseline);

            return Outcome::Refused(Refusal::Clipboard);
        }
    };

    // From here the clipboard holds the selection and not what the user put there. Step 8 is
    // now owed, whatever happens below — see [`Session`] — and it is owed **with** the numbers
    // that say what the clipboard held when it stopped being the user's.
    session.owed = Owed::StepEight {
        answer: ProbeAnswer::at(probe),
    };

    // ---- step 4 — read CF_UNICODETEXT ---------------------------------------------------
    let read = session.path.read();

    // ⭐ **Task T-38-2, finding С14.** The number step 3 answered with is the first number of the
    // application's answer, not always the last: an application still writing moves the counter
    // on after step 3 looked — Word by nineteen per copy, and once in twenty step 3 saw the first
    // of them (measured 2026-09-10). A successful read has had to wait for the writer to close
    // the clipboard, so the number standing now is the end of the same answer, and step 8 is owed
    // with all of it. A read that failed looked at nothing and widens nothing. The one gap this
    // opens, and why it is the only one, is in the documentation of [`restore_is_due`].
    if read.is_ok() {
        session.owed = Owed::StepEight {
            answer: ProbeAnswer::at(probe).through(session.path.sequence()),
        };
    }

    let text = match read {
        Ok(Some(text)) if !text.is_empty() => text,
        Ok(_) => return Outcome::Refused(Refusal::NoText),
        Err(_) => return Outcome::Refused(Refusal::Clipboard),
    };

    // ---- step 5 — the script, and the recoding ------------------------------------------
    let Some(source) = detect_source(&text, &plan.maps, plan.foreground) else {
        return Outcome::Refused(Refusal::NoDirection);
    };

    let Some(recoded) = recode_with(&text, source, plan) else {
        return Outcome::Refused(Refusal::NoDirection);
    };

    // ---- step 6 — write, then Ctrl+V ----------------------------------------------------
    if session.path.write(recoded.text()).is_err() {
        return Outcome::Refused(Refusal::Clipboard);
    }

    // ⭐ **Finding Н17 — task T-38-5.** A `Ctrl+V` the system did not take whole pasted nothing, or
    // not all of it: the press is a refusal and not a conversion — no tone of success, no layout
    // switch at step 7, and the typing-buffer path gets the press as after every refusal. Step 8
    // is owed as on every refusal after step 3, and it is our own write that it finds.
    if !chord_delivered(session.path.paste()) {
        return Outcome::Refused(Refusal::Clipboard);
    }

    // ---- step 7 — switch the layout, §4.6 -----------------------------------------------
    //
    // After the paste and never before it, for the reason FR-43 gives the same ordering on the
    // typing-buffer path: everything the application still has to process was sent under the
    // layout it was formed for.
    session.path.switch(recoded.target());

    Outcome::Converted {
        mapped: recoded.mapped(),
        carried: recoded.carried(),
    }

    // ---- step 8 happens here, in `Session::drop` (FR-40 step 6 is not performed, T-40-3) ---
}

/// The second half of step 5 for the source layout `index` of `plan` — **the decision over the
/// whole text**, which is now one of two decisions and no longer the one that recodes.
///
/// `None` when the cycle has no target for that source — the refusals of FR-30 and FR-35, all of
/// them counted inside module `layouts`.
///
/// # What `index` still decides after task Т-48-3
///
/// Two things, and the recoding is not one of them. It is the **tie-break** a word with no
/// distinguishing characters follows — `123` beside `ghbdtn` goes where `ghbdtn` goes — and it
/// is the layout **step 7** switches the foreground window to, which is what the user goes on
/// typing in. Both were true before; what changed is that the words in between now each answer
/// for themselves. See [`recode_words`].
fn recode_with(text: &str, index: usize, plan: &Plan) -> Option<Recoded> {
    let source = plan.maps.get(index)?;

    // The step is always one: the selection path keeps no counter of its own. It does not need
    // one — the text it converts is in the user's document, not in a buffer of ours, so the
    // second press reads back what the first press produced and step 5 detects *that* as the
    // source. See the report on where this is reversible and where it is not.
    let (source, target) = target_for(source, plan)?;

    Some(recode_words(text, plan, source, target))
}

// ---------------------------------------------------------------------------------------
// The real machine
// ---------------------------------------------------------------------------------------

/// The [`Path`] the program runs on — every member is a system call.
struct Machine {
    /// The window every clipboard access is opened against. T-07-1: it must be a live window of
    /// the calling thread, because `EmptyClipboard` clears the owner otherwise and every
    /// `SetClipboardData` after it refuses.
    owner: HWND,
    /// FR-44.
    delay_ms: u32,
    /// Step 8 of FR-61, out of section 7.
    restore_delay: Duration,
    /// Step 3 of FR-61, out of section 7.
    timeout: Duration,
}

impl Path for Machine {
    fn snapshot(&mut self) -> Result<Snapshot, ClipboardError> {
        snapshot(self.owner)
    }

    fn release_modifiers(&mut self) -> Modifiers {
        let held = crate::inject::held_modifiers();
        let mut events = [INPUT::default(); crate::inject::MODIFIER_COUNT];

        if let Ok(len) = crate::inject::build_release(held, &mut events) {
            let _ = crate::inject::dispatch(&events[..len], self.delay_ms);
        }

        held
    }

    fn copy(&mut self) -> Dispatched {
        crate::inject::dispatch(&copy_chord(), self.delay_ms)
    }

    fn wait(&mut self, baseline: u32) -> Wait {
        wait_for_change(baseline, self.timeout)
    }

    fn sequence(&mut self) -> u32 {
        sequence_number()
    }

    fn read(&mut self) -> Result<Option<ClipboardText>, ClipboardError> {
        read_unicode_text(self.owner)
    }

    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        write_unicode_text(self.owner, text)
    }

    fn paste(&mut self) -> Dispatched {
        crate::inject::dispatch(&paste_chord(), self.delay_ms)
    }

    fn switch(&mut self, target: LayoutId) {
        // The chain of §4.6, module `switch`. The verdict is dropped because that module counts
        // every refused method itself, and there is nothing the selection path could do about a
        // layout that would not change. SEC-01, SEC-07 — a layout handle, never a character.
        let _ = crate::switch::to(target);
    }

    fn restore_clipboard(&mut self, snapshot: &Snapshot, answer: ProbeAnswer) {
        // The result is dropped: `Restored` counts what went back and what the clipboard
        // refused, this module counts the refusals of FR-62, and there is nothing a caller could
        // do with the answer that it is not already doing. What must not happen is an early
        // return that skips this, and there is none — this is the whole body.
        let _ = restore_after(self.owner, snapshot, self.restore_delay, answer);
    }

    fn reclaim_clipboard(&mut self, snapshot: &Snapshot) {
        // The undelayed door, and the unrefusable one. The result is dropped for the reason
        // above; `self.restore_delay` is deliberately not read here, because the delay of step 8
        // waits for a paste that never happened on this path.
        let _ = restore(self.owner, snapshot);
    }
}

// ---------------------------------------------------------------------------------------
// FR-60 — the branch, and the two messages that carry it
// ---------------------------------------------------------------------------------------

/// Asks the UI thread to run the selection path — posted by the input thread.
///
/// `WM_APP + 12`, the next free number: `+ 1` is the wake-up of [`crate::app`], `+ 2` the tray
/// callback, `+ 3` and `+ 4` are [`crate::hook`]'s, `+ 5` the configuration nudge, `+ 6` and
/// `+ 7` are [`crate::watchdog`]'s, `+ 8` is [`crate::switch`]'s, `+ 9` the rehook and `+ 10`
/// and `+ 11` are [`crate::guard`]'s.
///
/// **SEC-05.** The message carries nothing — `wparam` and `lparam` are zero — and what it is
/// about travels in [`PENDING`], which only this process writes. A forged one finds no plan and
/// does nothing at all.
pub const WM_APP_SELECTION: u32 = WM_APP + 12;

/// Hands the press back to the typing-buffer path — posted by the UI thread.
///
/// `WM_APP + 13`. This is FR-60's other branch arriving late: the selection path found no
/// selection (step 3 timed out) or could not run, and the press has to do what it would have
/// done before this task existed. It is a message of its own rather than a re-post of
/// [`crate::hook::WM_APP_HOTKEY`] because a re-post would be offered the selection path again
/// and would loop.
///
/// **SEC-05.** Carries nothing, and buys a sender exactly what a forged `WM_APP_HOTKEY` already
/// bought: one conversion of text the sender can neither see nor influence.
pub const WM_APP_BUFFER_PATH: u32 = WM_APP + 13;

/// Presses handed to the UI thread.
static HANDOVERS: AtomicU32 = AtomicU32::new(0);

/// Presses the selection path converted.
static CONVERSIONS: AtomicU32 = AtomicU32::new(0);

/// Presses that went to the typing-buffer path because there was no selection.
static NO_SELECTION: AtomicU32 = AtomicU32::new(0);

/// Presses the selection path refused, for any of the reasons of [`Refusal`].
static REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Presses the selection path did not take because the window in front is a console.
///
/// **The note to §4.7 — decision П-3.** Kept apart from [`REFUSALS`] deliberately: the other
/// refusals are the path failing (no cache, no window, a password field), this one is the path
/// working exactly as the note prescribes, and an acceptance that could not tell them apart
/// could not tell П-3 from a regression.
static CONSOLE_REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Presses the selection path did not take because the window in front belongs to the thread
/// that would run it — **finding С9 of the audit of 2026-09-04, task T-38-3**.
///
/// Kept apart from [`REFUSALS`] and from [`CONSOLE_REFUSALS`] for the reason the second is kept
/// apart from the first: this is the path declining by rule, not failing, and an acceptance that
/// could not tell the seventh reason from the sixth, or from a regression, could not tell anything.
static OWN_WINDOW_REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Presses where the clipboard moved **after** step 3 had given up, or had been refused — tasks
/// T-13-11 and T-38-4.
///
/// The late `Ctrl+C` of the audit of 2026-08-24: the probe of step 2 was answered past the
/// timeout, and the snapshot of step 1 went back instead of being thrown away. Kept apart from
/// every other counter because it is the one number that says how often the race is real on a
/// given machine, and because the press itself still ends as [`Outcome::NoSelection`] — or, on a
/// refused step 3, as [`Refusal::Clipboard`] — and the two facts are independent and must be
/// readable apart.
static LATE_COPIES: AtomicU32 = AtomicU32::new(0);

/// The counters of the selection path — SEC-01, SEC-07: seven counts of program events.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PathCounters {
    /// Presses handed to the UI thread.
    pub handovers: u32,
    /// Presses converted through the selection.
    pub conversions: u32,
    /// Presses that found no selection and fell back — FR-60.
    pub no_selection: u32,
    /// Presses the path refused.
    pub refusals: u32,
    /// Presses the path did not take because a console was in front — the note to §4.7, П-3.
    pub console_refusals: u32,
    /// Presses the path did not take because a window of the thread that would run it was in
    /// front — finding С9, task T-38-3.
    pub own_window_refusals: u32,
    /// Presses whose clipboard moved after step 3 gave up or was refused, and whose snapshot
    /// therefore went back at once — the late `Ctrl+C` of tasks T-13-11 and T-38-4.
    pub late_copies: u32,
}

/// The counters of the selection path as they stand.
pub fn path_counters() -> PathCounters {
    PathCounters {
        handovers: HANDOVERS.load(Ordering::Relaxed),
        conversions: CONVERSIONS.load(Ordering::Relaxed),
        no_selection: NO_SELECTION.load(Ordering::Relaxed),
        refusals: REFUSALS.load(Ordering::Relaxed),
        console_refusals: CONSOLE_REFUSALS.load(Ordering::Relaxed),
        own_window_refusals: OWN_WINDOW_REFUSALS.load(Ordering::Relaxed),
        late_copies: LATE_COPIES.load(Ordering::Relaxed),
    }
}

/// **FR-60, on the input thread: does this press belong to the selection path?**
///
/// Answers `true` when the press has been handed over and the input thread must do nothing more
/// with it; `false` when the typing-buffer path is to run **right here, right now**, exactly as
/// it did before this task existed.
///
/// Seven reasons to answer `false`, and not one of them sends a `Ctrl+C`, opens the clipboard or
/// changes anything the user can observe:
///
/// | Reason | Requirement | What was touched |
/// |---|---|---|
/// | `[selection] enabled = false` | **FR-65** | nothing. No `Ctrl+C`, no clipboard, no message |
/// | the typing buffer is not empty | **Р-62, FR-60, FR-10** | nothing |
/// | the focus is in a password field | **SEC-06, FR-70** | nothing |
/// | no cache, or no target layout | FR-30, FR-35 | nothing |
/// | a console is in front | **note to §4.7 (П-3), FR-42а** | two read-only `user32` queries |
/// | a window of the UI thread is in front | **finding С9, task T-38-3** | two read-only `user32` queries |
/// | the UI thread has no window | — | nothing |
///
/// ⚠ **The first row is the whole of acceptance point 17.** With the path switched off this
/// function reads one atomic and returns, and the clipboard of the machine is not opened, not
/// read and not written — not even for step 1.
///
/// # The second row — decision Р-62
///
/// The probe of steps 2–3 of FR-61 is a real `Ctrl+C`, and `Ctrl+C` is not "copy" everywhere:
/// in a console it is an **interrupt** that can break a running process, and in editors such as
/// VS Code it copies the *whole line* when nothing is selected, so the sequence number moves,
/// the probe reads "there is a selection", and the replacement is pasted without erasing
/// (`ghbdtnпривет`). T-10-0c measured both harms.
///
/// Р-62 rests on the flush table of FR-10: **every gesture that makes a selection empties the
/// typing buffer** — a mouse click (Raw Input), the arrows and `Home`/`End` (the LL-hook rows),
/// and `Shift+arrow`, the keyboard-selection gesture, which task T-10-0d measured on the live
/// hook rather than assuming (buffer `6 → 0`). So a **non-empty buffer proves no selection
/// gesture has happened since the last stroke**, which means there is nothing selected, and
/// FR-60's rule "no selection → convert the typing buffer" is settled *without* the probe. The
/// press then runs `inject::on_hotkey` exactly as it did before the selection path existed.
///
/// The read is [`crate::buffer::is_empty`], a thread-local read of the calling thread's own
/// buffer (section 6.3). ⚠ **The sentence that stood here said this function is only ever
/// reached on the input thread, because `app::window_proc` guarded the call with
/// `buffer::is_installed` — that gate left in task Т-49-2**, and the call is now reached on any of
/// the three windows a `WM_APP_HOTKEY` lands on (task T-40-6 wrote the consequence down in
/// [`plan_for_press`]: on a thread with no recorder, installed or parked, there is no plan to
/// build). No atomic, no system call, no counter — a non-empty buffer is the ordinary "convert
/// what I typed" case, not a refusal, so it is answered as quietly as FR-65 is. When the buffer is empty — the only state a real
/// selection can coexist with — the function proceeds unchanged, which is why the selection
/// path of position 15 is untouched.
///
/// # The fifth row — the sixth reason, and the note to §4.7 (decision П-3)
///
/// Р-62 above closes only **half** of the harm its own comment names, and the audit of
/// 2026-08-24 measured which half is left open. In a console the typing buffer is empty almost
/// always — `Enter` is a boundary key of FR-10 and flushes it completely, and a console line is
/// finished with `Enter` — so the emptiness the second row falls through on is the console's
/// normal state, and the probe of step 2 of FR-61 goes out as a real interrupt into whatever
/// command the user is running. The note to §4.7 answers it: **in the console classes of FR-42а
/// the selection path does not run at all**, and the press on an empty buffer does nothing.
///
/// The key is still suppressed. FR-95 and record 8 of §10 put that decision in the hook, taken
/// synchronously and long before this function is reached, and nothing here can or should
/// reconsider it: what changes is what the press *does*, not whether the system sees it.
///
/// The rule and the Win32 call are split the way FR-42а is split —
/// [`console_refuses_selection`] is the rule and [`foreground_window_class`] is the line that asks
/// a window for its class — and the class names come from `inject`, so the consoles of this
/// program are named once ([`crate::inject::CONSOLE_WINDOW_CLASSES`]).
///
/// # The sixth row — the seventh reason, finding С9 of the audit of 2026-09-04 (task T-38-3)
///
/// The eight steps run on the UI thread — the thread [`listen`] claimed — and that thread owns
/// windows of its own: the settings dialog, the About box, the tray menu. With one of them in
/// front, the `Ctrl+C` of step 2 goes into the queue of the very thread that is then asleep inside
/// step 3, and nothing pumps it. The wait runs out, the snapshot is thrown away as «no selection»,
/// and when the dialog's modal loop picks the chord up afterwards it copies out of our own edit
/// control over the user's clipboard — no snapshot left, not one line of the journal. So with a
/// window of that thread in front the selection path does not run, and the press on an empty
/// typing buffer does nothing, exactly as in a console.
///
/// Decided by thread and not by a list of this program's window classes: a list would have to be
/// remembered with every new window, and the thread is what the harm is made of.
/// [`own_window_refuses_selection`] is the rule, [`foreground_window_thread`] the Win32 half, and
/// the count is [`PathCounters::own_window_refusals`].
pub fn wants_selection_path() -> bool {
    // FR-65, first, and before anything else can have an opinion.
    if !path_enabled() {
        return false;
    }

    // ⚠ **Р-62, FR-60, FR-10 — task T-10-0d.** A non-empty typing buffer proves no selection
    // gesture has happened since the last stroke (every such gesture flushes the buffer by
    // FR-10; `Shift+arrow` was measured on the live hook, not assumed). There is therefore no
    // selection, FR-60 resolves to the typing-buffer path, and the destructive `Ctrl+C` probe
    // of FR-61 must not run. Read of the input thread's own thread-local buffer — no system
    // call, no clipboard, no message, and no counter, because this is the ordinary case and not
    // a refusal. Empty buffer — the only state a selection can share — falls through unchanged.
    if !crate::buffer::is_empty() {
        return false;
    }

    // SEC-06 and FR-70. The typing buffer is already switched off in a password field; sending
    // `Ctrl+C` into one and reading what comes back would put the very thing SEC-06 protects on
    // the clipboard, which would be worse than not having the feature. One atomic read — the
    // flag module `guard` publishes from the watcher thread (FR-71).
    if crate::guard::password_field() {
        REFUSALS.fetch_add(1, Ordering::Relaxed);

        return false;
    }

    let Some(plan) = plan_for_press() else {
        REFUSALS.fetch_add(1, Ordering::Relaxed);

        return false;
    };

    // ⚠ **The note to §4.7 — decision П-3, audit of 2026-08-24 (inject-selection №1).** The probe
    // of step 2 of FR-61 is a real `Ctrl+C`, and in a console `Ctrl+C` is an **interrupt** that
    // can kill the command the user is running. Р-62 above does not reach this case: `Enter`
    // flushes the typing buffer completely by FR-10, so in a console the buffer is empty almost
    // always. The press becomes a no-op here, which is what the note prescribes; FR-95 is not
    // affected, the suppression having been decided in the hook (§10, record 8).
    //
    // ⚠ **Asked here and not earlier, and both halves of that are deliberate.** It and the seventh
    // reason below are the only reasons that cost a system call, so every cheaper reason is asked
    // first — the ordinary press, the one with a non-empty buffer, still leaves this function
    // without a single system call, which is what the comment on the Р-62 read above promises.
    // And it is asked *after* the plan, so the count below means «presses this decision took away
    // from a path that would otherwise have run»: a press with no cache is refused for a reason
    // that holds in every window, and attributing it to the console would overstate what П-3
    // does. Before `publish_pending`, so that a refusal never leaves a plan behind for a later
    // message.
    if console_refuses_selection(foreground_window_class().as_deref()) {
        return false;
    }

    // ⚠ **Finding С9 of the audit of 2026-09-04, task T-38-3 — the seventh reason.** The eight
    // steps run on the UI thread, and a window of that thread in front — the settings dialog, the
    // About box, the tray menu — is a window whose queue that very thread will not pump while
    // step 3 sleeps: the `Ctrl+C` of step 2 would wait there past the timeout, the snapshot would
    // go as «no selection», and the queued chord would later copy out of our own dialog over the
    // user's clipboard. Asked where the sixth reason is asked and for its reasons: after the
    // plan, before anything is published.
    if own_window_refuses_selection(
        foreground_window_thread(),
        WORKER_THREAD.load(Ordering::Acquire),
    ) {
        return false;
    }

    // Published before the message is posted, so a thread woken by it cannot find an empty slot
    // — the order `app::publish_buffer_capacity` uses for the same reason.
    publish_pending(plan);

    if !crate::app::post_to_ui_thread(WM_APP_SELECTION) {
        // No UI window: the program is starting or leaving. The plan is taken back rather than
        // left for a later message to act on — the shape `switch::hand_over_to_watcher` uses.
        let _ = take_pending();
        REFUSALS.fetch_add(1, Ordering::Relaxed);

        return false;
    }

    HANDOVERS.fetch_add(1, Ordering::Relaxed);

    true
}

/// **The sixth reason of FR-60, decided from a class name — the note to §4.7, decision П-3.**
///
/// `true` when a foreground window of class `class` is one of the consoles FR-42а names, which
/// is the state in which the selection path does not run: no `Ctrl+C`, no clipboard, no
/// hand-over, and the press on an empty typing buffer does nothing at all.
///
/// # Why it is a function of a string
///
/// Split from the Win32 call for the reason FR-42а is split — [`crate::inject::resolve_auto`] is
/// the rule and [`crate::inject::window_class`] is the one place that really asks a window — so
/// that acceptance can drive this rule with any class name it likes and **no live window at
/// all**. The Win32 half is [`foreground_window_class`], and it is the only thing on this path
/// that touches the system.
///
/// The names themselves are [`crate::inject::CONSOLE_WINDOW_CLASSES`], not a second list: the
/// consoles of this program are named once, in the module whose measurement (T-10-7) named them.
/// A class nobody measured has no business in that list, and a copy of it here would be a second
/// place for one to appear.
///
/// # Why the count is taken here
///
/// «Counted where it is decided», the rule [`plan_for_press`] states for its own refusals — and
/// into [`CONSOLE_REFUSALS`] rather than [`REFUSALS`], because acceptance has to tell this
/// refusal from a missing cache or a password field: this one is П-3 working, the others are the
/// path failing.
///
/// # `None`, and the direction of that refusal — NFR-13
///
/// `None` means there is no foreground window at all, or that `GetClassNameW` refused
/// ([`crate::inject::window_class`] is where the zero return is examined). It answers **`false`**
/// — the path runs — and that is the safe direction rather than the lazy one: a window that
/// cannot be named is a window that is going away or already gone, and a `Ctrl+C` sent at a
/// desktop switch or a lock screen has no console session to interrupt. Refusing instead would
/// turn every such moment into a silently dead hotkey, and it is the same direction FR-42а
/// already chose for an unreadable class (the `_` arm of `resolve_auto`).
///
/// Case-insensitively, because window class names are case-insensitive to Windows itself — the
/// comparison `inject::resolve_auto` and `guard::class_is_edit` both make.
pub fn console_refuses_selection(class: Option<&str>) -> bool {
    let console = match class {
        Some(class) => crate::inject::CONSOLE_WINDOW_CLASSES
            .iter()
            .any(|console| class.eq_ignore_ascii_case(console)),
        // No window, or a class that cannot be read. NFR-13: examined, and answered as "not a
        // console" for the reason the section above gives.
        None => false,
    };

    if console {
        CONSOLE_REFUSALS.fetch_add(1, Ordering::Relaxed);
    }

    console
}

/// **The seventh reason of FR-60, decided from two thread ids — finding С9 of the audit of
/// 2026-09-04, task T-38-3.**
///
/// `true` when the window in front belongs to `worker` — the thread [`listen`] claimed, which is
/// the thread [`handle_selection_message`] runs the eight steps on. That is the state in which the
/// path stands in its own way: the `Ctrl+C` of step 2 goes into the queue of the one thread that
/// is at that moment asleep inside step 3 and will not pump it. The wait runs out, the snapshot is
/// thrown away as «no selection», and when the modal loop of the settings dialog, the About box or
/// the tray menu picks the queued chord up afterwards, it copies out of an edit control of ours
/// over the user's clipboard — with no snapshot left and not one line of the journal.
///
/// # Why it is a function of two numbers
///
/// Split from the Win32 calls for the reason [`console_refuses_selection`] is split from
/// [`foreground_window_class`]: acceptance drives the rule with any pair it likes and no live
/// window at all. The Win32 half is [`foreground_window_thread`].
///
/// # `None`, and a worker of zero — NFR-13
///
/// `None` means there is no foreground window, or its thread could not be read. It answers
/// **`false`** — the path runs — the direction the sixth reason takes for an unreadable class: a
/// window that cannot be asked is going away, and it is no window of ours to be stuck behind. A
/// `worker` of zero is [`NO_THREAD`] — no listener is registered, so there is no UI thread to be
/// stuck on — and it answers `false` as well.
///
/// # Why the count is taken here
///
/// «Counted where it is decided», as the sixth reason counts, and into [`OWN_WINDOW_REFUSALS`]
/// rather than [`REFUSALS`] or [`CONSOLE_REFUSALS`] for the reason given there.
pub fn own_window_refuses_selection(foreground: Option<u32>, worker: u32) -> bool {
    let own = match foreground {
        Some(thread) => worker != NO_THREAD && thread == worker,
        // No window in front, or its thread could not be read. NFR-13: examined, and answered as
        // "not ours" for the reason the section above gives.
        None => false,
    };

    if own {
        OWN_WINDOW_REFUSALS.fetch_add(1, Ordering::Relaxed);
    }

    own
}

/// The class of the window in front, or `None` when there is none or it cannot be read.
///
/// **The Win32 half of the sixth reason** — the shape of [`crate::inject::window_class`], and
/// one of the two call sites of this path that ask the system anything; the other is
/// [`foreground_window_thread`], the Win32 half of the seventh.
///
/// # Which thread this runs on, and why it is not the hook callback — NFR-01…NFR-05
///
/// The **input thread**, inside [`wants_selection_path`], which is the same thread and the same
/// call the `crate::buffer::is_empty()` read above it happens on: `app::window_proc` guards that
/// call with `buffer::is_installed`, and the typing buffer of section 6.3 is installed on the
/// input thread and on no other. The hook callback ends at `PostMessageW` (module `hook`); every
/// line below runs after that hand-off, under NFR-09's thirty milliseconds rather than NFR-01's
/// hundred microseconds. Two read-only `user32` queries, which is exactly what
/// `inject::effective_method` already spends once per replacement on the same thread — no
/// allocation on the callback's behalf, no blocking primitive, no I/O.
///
/// `GetClassNameW` is **not** written a second time: the call, its 257-character buffer and the
/// NFR-13 examination of its zero return all live in [`crate::inject::window_class`], and this
/// reuses them.
fn foreground_window_class() -> Option<String> {
    // SAFETY: `GetForegroundWindow` takes no arguments, returns a handle by value and touches no
    // memory of ours; it is callable from any thread. The documented NULL return is checked
    // immediately below.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        // NFR-13: examined. There is no foreground window — the documented NULL, seen around
        // desktop switches and the lock screen. Nothing to read a class from, and
        // `console_refuses_selection` documents why that answers "not a console".
        return None;
    }

    crate::inject::window_class(foreground)
}

/// The thread that owns the window in front, or `None` when there is no such window or its thread
/// cannot be read — **the Win32 half of the seventh reason**, finding С9, task T-38-3.
///
/// Runs where [`foreground_window_class`] runs and costs what it costs: two read-only `user32`
/// queries on the input thread, after the hook callback's hand-off — no allocation, no blocking
/// primitive, no I/O. The window in front is asked for again rather than shared with the class
/// query: each of the two reasons is safe on its own answer, and a foreground window that changes
/// between the two calls changes nothing either of them decides.
fn foreground_window_thread() -> Option<u32> {
    // SAFETY: as in `foreground_window_class` — no arguments, a handle by value, callable from any
    // thread; the documented NULL return is checked immediately below.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        // NFR-13: examined. No window in front, and `own_window_refuses_selection` documents why
        // that answers "not ours".
        return None;
    }

    // SAFETY: `foreground` is a handle value the system has just returned, passed by value. No
    // process-id pointer is passed, so nothing of ours is written; a window that went away in
    // between answers zero, which is examined below.
    let thread = unsafe { GetWindowThreadProcessId(foreground, None) };

    // NFR-13: zero is the documented failure — the handle no longer names a window.
    (thread != 0).then_some(thread)
}

/// **Runs the selection path — the UI thread's half of the hand-over.**
///
/// Called from `app::window_proc` for every message of every window; answers `None` for
/// everything that is not [`WM_APP_SELECTION`] at the window [`listen`] registered, which is
/// every window of the input and watcher threads.
///
/// The caller acts on the answer in one way and one way only: [`Outcome::falls_back`] means the
/// press has to be given back to the input thread as [`WM_APP_BUFFER_PATH`].
///
/// # SEC-05
///
/// A process at the same integrity level can post [`WM_APP_SELECTION`] to this window. What that
/// buys it is a look into [`PENDING`], which is empty unless this program's own input thread put
/// something there one message ago — and an empty plan is answered with a refusal before the
/// clipboard is opened.
pub fn handle_selection_message(hwnd: HWND, message: u32) -> Option<Outcome> {
    if message != WM_APP_SELECTION {
        return None;
    }

    if hwnd.0 as usize != LISTENER_WINDOW.load(Ordering::Acquire) {
        return None;
    }

    let Some(plan) = take_pending() else {
        REFUSALS.fetch_add(1, Ordering::Relaxed);

        return Some(Outcome::Refused(Refusal::NoPlan));
    };

    let mut machine = Machine {
        owner: hwnd,
        delay_ms: plan.delay_ms,
        restore_delay: plan.restore_delay,
        timeout: plan.timeout,
    };

    let outcome = run(&mut machine, &plan);

    match outcome {
        Outcome::Converted { .. } => CONVERSIONS.fetch_add(1, Ordering::Relaxed),
        Outcome::NoSelection => NO_SELECTION.fetch_add(1, Ordering::Relaxed),
        Outcome::Refused(_) => REFUSALS.fetch_add(1, Ordering::Relaxed),
    };

    Some(outcome)
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

    /// The ceiling of step 4 — finding №7 of the audit of 2026-08-31, decision 77.
    ///
    /// Both sides of the boundary, so that `>` turning into `>=`, or the budget being replaced by
    /// a number of this function's own, fails here rather than passing quietly.
    #[test]
    fn the_read_refuses_only_what_is_over_the_budget_of_fr64() {
        assert!(!read_refuses_size(0));
        assert!(!read_refuses_size(1));
        assert!(!read_refuses_size(SNAPSHOT_BUDGET_BYTES - 1));
        // Exactly the budget is read: the ceiling is the largest block allowed through.
        assert!(!read_refuses_size(SNAPSHOT_BUDGET_BYTES));
        assert!(read_refuses_size(SNAPSHOT_BUDGET_BYTES + 1));
        assert!(read_refuses_size(usize::MAX));
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

            assert_eq!(&*back, expected, "round trip of {text:?}");
        }
    }

    #[test]
    fn an_odd_or_truncated_block_decodes_instead_of_panicking() {
        // Three bytes: one complete UTF-16 unit and a stray one. `chunks_exact` drops the tail.
        assert_eq!(&*decode_utf16(&[0x41, 0x00, 0x42]), "A");
        // No terminator at all.
        assert_eq!(&*decode_utf16(&[0x41, 0x00, 0x42, 0x00]), "AB");
        // Nothing at all.
        assert_eq!(&*decode_utf16(&[]), "");
        // An unpaired surrogate is replaced rather than refused.
        assert_eq!(&*decode_utf16(&[0x00, 0xD8, 0x00, 0x00]), "\u{FFFD}");
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
            formats_truncated: true,
            markers_without_data: 2,
            sequence: 77,
        };

        let printed = format!("{snapshot:?}");

        // SEC-01, SEC-07: the bytes are not there under any spelling.
        assert!(!printed.contains("SECRET"));
        assert!(!printed.contains("83")); // the first byte of the payload, as a decimal
        assert!(printed.contains("captured_formats: 1"));
        assert!(printed.contains("total_bytes: 24"));
        // The separator in front keeps this from being satisfied by `formats_truncated` below.
        assert!(printed.contains(", truncated: true"));
        // Task T-38-7: the list of formats cut short at the bound is a flag of its own, printed.
        assert!(printed.contains("formats_truncated: true"));
        // Task T-38-6: the markers kept in their restrictive form are a count, and are printed.
        assert!(printed.contains("markers_without_data: 2"));
    }

    #[test]
    fn an_empty_snapshot_answers_the_questions_a_caller_asks_of_one() {
        let snapshot = Snapshot::empty();

        assert!(snapshot.is_empty());
        assert!(!snapshot.is_truncated());
        assert_eq!(snapshot.captured_formats(), 0);
        assert_eq!(snapshot.listed_formats(), 0);
        assert_eq!(snapshot.total_bytes(), 0);
        assert_eq!(snapshot.markers_without_data(), 0);
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

    /// **The note to step 8 of §4.7 — decision П-5.** A change of ours leaves the restore owed; a
    /// change of somebody else's takes it away, says so twice, and opens no clipboard.
    ///
    /// The three cases are one test on purpose: [`RESTORE_SKIPS`] is process-wide and the
    /// assertions below are about it moving by **exactly** one, which two tests running beside
    /// each other could not both claim.
    ///
    /// The numbers are chosen the way `a_write_of_ours_is_recognised_and_a_number_nobody_wrote`
    /// chooses them — values the window station's counter cannot reach while this test runs —
    /// and one slot of the ring is written, so that a test running beside this one keeps its own.
    #[test]
    fn a_foreign_change_in_the_delay_window_takes_the_restore_away() {
        // Task Т-18-1 gave this counter a second exact reader — see [`P5`]. Nothing below is
        // changed by the lock; it only keeps the two readers out of each other's arithmetic.
        let _serialised = P5.lock().unwrap_or_else(PoisonError::into_inner);

        // Т-18-1 also gave the gate a second thing to ask about — since T-38-2, the whole answer
        // to the probe. This test is about the first one — the ring — so the second is the "no
        // probe was answered" value throughout, and every assertion below still turns on
        // `is_own_change` alone, exactly as it did.
        const NO_PROBE: ProbeAnswer = ProbeAnswer::NONE;

        let ours = u32::MAX - 24;
        let theirs = u32::MAX - 25;

        let index = OWN_NEXT.fetch_add(1, Ordering::Relaxed) % OWN_MARKS;
        OWN_MARK[index].store(ours, Ordering::Release);

        let before = counters();

        // Nothing has touched the clipboard since this program wrote: step 8 is owed, and the
        // behaviour is the one that was there before decision П-5.
        assert!(
            restore_is_due(ours, NO_PROBE),
            "our own write is not a foreign change"
        );
        assert_eq!(
            counters().restore_skips,
            before.restore_skips,
            "an owed restore counts no skip"
        );

        // And somebody else's copy inside the delay window takes the restore away.
        assert!(
            !restore_is_due(theirs, NO_PROBE),
            "П-5: a foreign change is skipped"
        );
        assert_eq!(
            counters().restore_skips,
            before.restore_skips + 1,
            "exactly one skip is counted"
        );

        // The fact is in the journal, under a name of the closed vocabulary — SEC-07.
        assert_ne!(
            crate::diag::Operation::from_name("clipboard restore skipped"),
            crate::diag::Operation::UNLISTED,
            "the row is in the table of module diag, so the entry is named"
        );
        assert!(
            crate::diag::snapshot()
                .iter()
                .any(|event| event.operation.name() == "clipboard restore skipped"),
            "the skipped restore left no entry behind it"
        );

        // Behaviourally, through the door step 8 uses. The branch is deterministic here:
        // nothing in this binary writes to a clipboard, so the ring holds no number
        // `GetClipboardSequenceNumber` can answer and the skip is the arm taken. The owner
        // handle is an invalid one on purpose — it is the safety net rather than the subject.
        // Were this ever to stop skipping, `OpenClipboard` would refuse the handle and the
        // assertion would go red, instead of `EmptyClipboard` throwing away the clipboard of
        // whoever is running the tests.
        let bogus = HWND(core::ptr::dangling_mut::<core::ffi::c_void>());

        let restored = restore_after(bogus, &Snapshot::empty(), Duration::ZERO, NO_PROBE);

        assert!(
            matches!(
                restored,
                Ok(Restored {
                    placed: 0,
                    refused: 0
                })
            ),
            "a skipped restore answers that it placed nothing: {restored:?}"
        );

        let after = counters();

        assert_eq!(after.opens, before.opens, "the clipboard was not opened");
        assert_eq!(after.closes, before.closes);
        assert_eq!(
            after.open_retries, before.open_retries,
            "and it was not even attempted"
        );
        assert_eq!(
            after.restore_skips,
            before.restore_skips + 2,
            "the second skip is the one `restore_after` made"
        );
    }

    /// Serialises the two tests that make **exact** claims about [`RESTORE_SKIPS`].
    ///
    /// The counter is process-wide and `cargo test` runs the tests of a binary in parallel. Until
    /// task Т-18-1 there was one such test, and its own documentation says why it holds three
    /// cases rather than three tests — «which two tests running beside each other could not both
    /// claim». Т-18-1 needed a second one, so the reason moved into a lock instead of into a
    /// prohibition. The same device, and the same reason, as [`ACCESS`] further down.
    static P5: Mutex<()> = Mutex::new(());

    /// **И-1, И-2 and И-5 of Э18 — the gate of decision П-5 on a path where nothing was written.**
    ///
    /// The defect the audit of 2026-08-31 found is one question asked of the wrong witness. After
    /// step 3 the clipboard holds the selection — put there by the *application*, in answer to the
    /// probe of step 2 — and every refusal of steps 4 to 6 leaves the user's content in the
    /// snapshot and nowhere else. [`OWN_MARK`] is silent on such a path, because this program
    /// wrote nothing, so the gate that asked [`is_own_change`] alone read «somebody else copied
    /// something» off a clipboard nobody but our own probe had touched, and threw the snapshot
    /// away — while counting and journalling it as a lawful П-5 skip.
    ///
    /// So the gate is given the number of the probe's answer as well. The numbers below are the
    /// ones the window station's counter cannot reach while the test runs, and **not one of them
    /// is put into the ring** — that is the point: the path being measured is the one with no
    /// write of ours on it at all.
    #[test]
    fn the_answer_to_the_probe_opens_the_gate_and_a_third_writer_shuts_it() {
        let _serialised = P5.lock().unwrap_or_else(PoisonError::into_inner);

        let probe = u32::MAX - 36;
        let third = u32::MAX - 37;

        assert!(
            !is_own_change(probe) && !is_own_change(third),
            "neither number is a write of this program, and the test rests on that"
        );

        let before = counters();

        // И-1. Nothing has touched the clipboard since the application answered the probe, so
        // what is on it is this program's doing and the snapshot is owed.
        assert!(
            restore_is_due(probe, ProbeAnswer::at(probe)),
            "И-1: the answer to our own probe is not a stranger's copy"
        );

        // И-5. A restore that happens is not a skip, and must not be counted as one.
        assert_eq!(
            counters().restore_skips,
            before.restore_skips,
            "И-5: an owed restore counts no skip"
        );

        // И-2. П-5 still holds: a third writer after the answer to the probe keeps its copy.
        assert!(
            !restore_is_due(third, ProbeAnswer::at(probe)),
            "И-2: a copy made after the probe was answered is not ours to overwrite"
        );
        assert_eq!(
            counters().restore_skips,
            before.restore_skips + 1,
            "И-2: exactly one skip is counted"
        );

        // And it is in the journal under a name of the closed vocabulary — SEC-07, И-8.
        assert!(
            crate::diag::snapshot()
                .iter()
                .any(|event| event.operation.name() == "clipboard restore skipped"),
            "the skipped restore left no entry behind it"
        );
    }

    /// **Task T-38-2, finding С14 — the whole of the answer opens the gate, and nothing past it.**
    ///
    /// Step 3 answers the first number it sees move, and an application still writing moves the
    /// counter on: Word, measured 2026-09-10, by nineteen per copy, and once in twenty step 3
    /// looked at the first of them. The gate is given the answer from that first number to the
    /// one standing after step 4's read, so a number the application went on to is the same
    /// answer and owes the snapshot — while a number one past the read is a third writer's, and
    /// П-5 keeps its copy exactly as before. The numbers are ones no clipboard in this binary
    /// reaches, and none is put into the ring: the path measured is the one with no write of
    /// ours on it.
    #[test]
    fn a_number_the_answer_ran_on_to_opens_the_gate_and_one_past_its_last_does_not() {
        let _serialised = P5.lock().unwrap_or_else(PoisonError::into_inner);

        let baseline = u32::MAX - 60;
        // Step 3 looked while the application was still writing and saw the first number of the
        // answer; step 4's read saw the answer end four numbers later.
        let answer = ProbeAnswer::at(baseline + 1).through(baseline + 5);

        assert!(
            (baseline + 1..=baseline + 6).all(|n| !is_own_change(n)),
            "no number of the test is a write of this program, and the test rests on that"
        );

        let before = counters();

        assert!(
            restore_is_due(baseline + 2, answer),
            "a number the application went on to after step 3 looked is the same answer: \
             {answer:?}"
        );
        assert_eq!(
            counters().restore_skips,
            before.restore_skips,
            "an owed restore counts no skip"
        );

        assert!(
            !restore_is_due(baseline + 6, answer),
            "П-5: a number past the last look of step 4 is a third writer's: {answer:?}"
        );
        assert_eq!(
            counters().restore_skips,
            before.restore_skips + 1,
            "exactly one skip is counted"
        );
    }

    /// **Task T-38-2 — the contract of [`ProbeAnswer`].** An answer widens only upwards, holds both
    /// of its ends and nothing outside them, and one that begins at zero is no answer at all.
    #[test]
    fn an_answer_widens_only_upwards_and_zero_is_never_one() {
        let answer = ProbeAnswer::at(100);

        assert_eq!(
            answer.through(99),
            answer,
            "a lower number is not a later look"
        );

        let widened = answer.through(119);

        assert_eq!(
            widened,
            ProbeAnswer {
                first: 100,
                last: 119
            }
        );
        assert!(
            widened.contains(100) && widened.contains(119),
            "both ends are the answer"
        );
        assert!(
            !widened.contains(99) && !widened.contains(120),
            "and nothing outside them is"
        );
        assert!(
            !ProbeAnswer::NONE.contains(0),
            "zero compared with zero is not a fact about a clipboard"
        );
        assert!(
            !ProbeAnswer::at(0).through(5).contains(3),
            "an answer that begins at zero is no answer"
        );
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
