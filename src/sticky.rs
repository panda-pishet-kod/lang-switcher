//! The keyboard shortcut of the sticky keys of Windows — **task T-95-6, решение 159.25**.
//!
//! # Why this module exists
//!
//! Five presses of `Shift` in a row, with nothing between them, make Windows offer to turn sticky
//! keys on — a window with a question, in the middle of typing. The double press of `Shift`
//! (task T-95-1) makes four of them a habit: a pair converts the word, a pair brings it back, and the
//! capital that begins the next word is the fifth. The owner met that window on the acceptance of
//! the first build of stage E95 (159б) and decided: while the hotkey is the double press and the
//! program runs, **the shortcut of Windows is off**, and it comes back as it was.
//!
//! # What is touched, and what never is
//!
//! * One bit — `SKF_HOTKEYACTIVE`, «turn sticky keys on and off with five presses of `Shift`» — and
//!   only in **the memory of the session**: `SPI_SETSTICKYKEYS` with `fWinIni = 0`, never
//!   `SPIF_UPDATEINIFILE` (the profile the next logon reads) and never `SPIF_SENDCHANGE`. The way
//!   Microsoft describes for games («Disabling Shortcut Keys in Games»), with one difference that
//!   is the controller's note of 2026-10-07: that sample writes the whole saved structure back on
//!   the way out, which would undo whatever the person changed in the meantime. This module
//!   remembers only **that it took the bit** ([`TAKEN`]) and gives back only the bit, and only if it
//!   is still off.
//! * Sticky keys **turned on** (`SKF_STICKYKEYSON`) are the person's: the shortcut is how they turn
//!   them off again, and nothing is taken then.
//! * A shortcut already off — by the person or by another program — has nothing to give, and
//!   nothing is owed for it.
//!
//! # When
//!
//! * Taken on a publication of the configuration whose hotkey is the double press
//!   ([`follow_hotkey`], from `app::publish_configuration` — the start and every «Применить»).
//! * Given back on a publication of any other hotkey ([`follow_hotkey`] again), on the ordinary end
//!   of the process (the main thread, once every thread is joined), on `WM_ENDSESSION` (the end of
//!   the session and the uninstaller of 155.25) — all through [`give_back`] — and by FR-96, after
//!   the hook is already off ([`give_back_quietly`]).
//! * The suspension of FR-90 changes nothing: the process runs and the hotkey is still the one
//!   chosen (the letter of 159.25: «пока назначен двойной Shift и программа работает»).
//!
//! # NFR-01
//!
//! None of this runs in the callback of a live hook. The one call from inside a callback is
//! FR-96's, and it comes after `hook::uninstall`: the keyboard is already the system's.
//!
//! # The edges — written down rather than mended (SPEC, FR-95; README)
//!
//! * A crash, or a process ended from outside, leaves the shortcut off until the session ends —
//!   nothing was written to the profile, so the next logon has it back.
//! * «Параметры» of Windows may write the whole structure into the profile when the person changes
//!   sticky keys there while this program runs — the temporary «off» would then be kept. It follows
//!   from how `SystemParametersInfo` works; it is not measured.
//! * A game that takes the same shortcut the same way and starts after this program takes «off» for
//!   the original; one that ends before it gives «on» back while the double press is still the
//!   hotkey.

use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::UI::Accessibility::{
    SKF_HOTKEYACTIVE, SKF_STICKYKEYSON, STICKYKEYS, STICKYKEYS_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETSTICKYKEYS, SPI_SETSTICKYKEYS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};
use windows::core::Result as WinResult;

/// Whether this process took `SKF_HOTKEYACTIVE` and owes it back — **the only thing it remembers**
/// (the controller's note on 159.25: not the flags of the start, which would undo on the way out
/// whatever the person changed in the meantime).
///
/// Written by the publication on the UI thread and by every way out — the main thread, the window
/// that receives `WM_ENDSESSION`, the callback of FR-96 — so it is an atomic, and the ways out take
/// the debt with a `swap`: of two that race, one gives back and the other finds nothing owed.
static TAKEN: AtomicBool = AtomicBool::new(false);

/// The name of the system's refusal in the journal — a row of `src\diag.rs`, `Kind::Process`.
const REFUSED: &str = "SystemParametersInfoW (sticky keys)";

/// The name of the shortcut taken for the double press — a row of `src\diag.rs`.
const TURNED_OFF: &str = "sticky keys shortcut turned off";

/// The name of the shortcut given back — a row of `src\diag.rs`.
const GIVEN_BACK: &str = "sticky keys shortcut given back";

/// What one publication or one way out did to the shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Nothing was written.
    Nothing,
    /// The shortcut was turned off for the double press, and is owed back.
    TurnedOff,
    /// The shortcut this process took was given back.
    GivenBack,
}

/// The flags with the shortcut turned off for the double press, or `None` when nothing is to be
/// written: sticky keys turned on are the person's (the shortcut is how they turn them off again),
/// and a shortcut already off has nothing to give. Every other bit stays as it is —
/// `SKF_CONFIRMHOTKEY` among them: it is the setting of a window that no longer comes.
pub fn taken(flags: u32) -> Option<u32> {
    let on = flags & SKF_STICKYKEYSON.0 != 0;
    let shortcut = flags & SKF_HOTKEYACTIVE.0 != 0;

    (!on && shortcut).then_some(flags & !SKF_HOTKEYACTIVE.0)
}

/// The flags with the shortcut given back, or `None` when it is on already — the person or another
/// program put it back, and nothing else of the flags is this program's to write.
pub fn given_back(flags: u32) -> Option<u32> {
    (flags & SKF_HOTKEYACTIVE.0 == 0).then_some(flags | SKF_HOTKEYACTIVE.0)
}

/// One publication or one way out, as a function of what is owed, of the hotkey and of the two
/// calls of the system — **the seam of the tests**: they pass closures, and the person's session is
/// never touched by a test.
///
/// Returns what is owed afterwards, and what was done — or the refusal of the system, which leaves
/// the debt as it was (NFR-13: examined, and reported by the caller).
///
/// * The double press, nothing owed: the flags are read and [`taken`] decides.
/// * The double press, owed already — another «Применить»: nothing is asked. A shortcut someone put
///   back in the meantime stays on until this program starts again (an edge SPEC names).
/// * Another hotkey, owed: the flags are read and [`given_back`] decides; the debt is closed either
///   way.
/// * Another hotkey, nothing owed: nothing is asked.
pub fn follow_via(
    owed: bool,
    double_tap: bool,
    read: impl FnOnce() -> WinResult<u32>,
    write: impl FnOnce(u32) -> WinResult<()>,
) -> (bool, WinResult<Change>) {
    if double_tap == owed {
        return (owed, Ok(Change::Nothing));
    }

    let flags = match read() {
        Ok(flags) => flags,
        Err(error) => return (owed, Err(error)),
    };

    let (wanted, change) = if double_tap {
        (taken(flags), Change::TurnedOff)
    } else {
        (given_back(flags), Change::GivenBack)
    };

    let Some(wanted) = wanted else {
        return (false, Ok(Change::Nothing));
    };

    match write(wanted) {
        Ok(()) => (double_tap, Ok(change)),
        Err(error) => (owed, Err(error)),
    }
}

/// Follows the hotkey just published — called by `app::publish_configuration` (the start and every
/// «Применить», on the UI thread) with whether the hotkey is the double press of `Shift`.
pub fn follow_hotkey(double_tap: bool) {
    let (owed, done) = follow_via(
        TAKEN.load(Ordering::Relaxed),
        double_tap,
        read_flags,
        write_flags,
    );

    TAKEN.store(owed, Ordering::Relaxed);
    note(done);
}

/// Gives the shortcut back if this process took it — the ordinary end of the process and
/// `WM_ENDSESSION`. Asks the system nothing when nothing is owed.
pub fn give_back() {
    if !TAKEN.swap(false, Ordering::Relaxed) {
        return;
    }

    let (owed, done) = follow_via(true, false, read_flags, write_flags);

    TAKEN.store(owed, Ordering::Relaxed);
    note(done);
}

/// The same for FR-96, from inside the callback **after the hook is off**: no entry of the
/// journal (NFR-05, as the termination failure beside it), and nothing kept of a refusal — the
/// process ends on the next line.
pub fn give_back_quietly() {
    if TAKEN.swap(false, Ordering::Relaxed) {
        let _ = follow_via(true, false, read_flags, write_flags);
    }
}

/// Writes down what a step did: the two changes of a setting of Windows are facts worth a line
/// (the shape of «configuration read-only attribute cleared» — a person's setting the program
/// changed), and a refusal of the system is reported (NFR-13). A fact and no value: the flags are
/// not in any entry (SEC-01, SEC-07).
fn note(done: WinResult<Change>) {
    let name = match done {
        Ok(Change::Nothing) => return,
        Ok(Change::TurnedOff) => TURNED_OFF,
        Ok(Change::GivenBack) => GIVEN_BACK,
        Err(error) => {
            crate::app::report_non_critical(REFUSED, &error);
            return;
        }
    };

    crate::diag::record(
        crate::diag::Operation::from_name(name),
        crate::diag::OsCode::NONE,
    );
}

/// The size of the structure, as `cbSize` and `uiParam` both say it.
const SIZE: u32 = size_of::<STICKYKEYS>() as u32;

/// Reads the flags of the sticky keys in force for the session — `SPI_GETSTICKYKEYS`.
fn read_flags() -> WinResult<u32> {
    let mut keys = STICKYKEYS {
        cbSize: SIZE,
        dwFlags: STICKYKEYS_FLAGS(0),
    };

    // SAFETY: `SPI_GETSTICKYKEYS` is a query whose `pvParam` is documented to be a `STICKYKEYS`
    // with `cbSize` set, which the OS fills in. `keys` is exactly that structure, lives on this
    // frame for the whole call, and the pointer is derived from a live mutable borrow of it, so
    // it is aligned, non-null and writable for the `uiParam` bytes the OS may touch. No update flag
    // is passed: a query writes nothing anywhere.
    unsafe {
        SystemParametersInfoW(
            SPI_GETSTICKYKEYS,
            SIZE,
            Some(core::ptr::from_mut(&mut keys).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }?;

    Ok(keys.dwFlags.0)
}

/// Writes the flags of the sticky keys **for the session only** — `SPI_SETSTICKYKEYS` with no
/// update flag: never `SPIF_UPDATEINIFILE`, so the profile the next logon reads keeps the person's
/// own flags whatever happens to this process, and never `SPIF_SENDCHANGE`, so no window of the
/// desktop is told.
fn write_flags(flags: u32) -> WinResult<()> {
    let mut keys = STICKYKEYS {
        cbSize: SIZE,
        dwFlags: STICKYKEYS_FLAGS(flags),
    };

    // SAFETY: `SPI_SETSTICKYKEYS` reads a `STICKYKEYS` with `cbSize` set from `pvParam`. `keys` is
    // that structure, alive on this frame for the whole call; the pointer comes from a live
    // mutable borrow of it, so it is aligned and valid for the `uiParam` bytes the OS reads. The
    // update flags are zero — the session's memory and nothing else.
    unsafe {
        SystemParametersInfoW(
            SPI_SETSTICKYKEYS,
            SIZE,
            Some(core::ptr::from_mut(&mut keys).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
}
