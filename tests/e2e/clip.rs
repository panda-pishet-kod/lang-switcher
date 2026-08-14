//! The user's clipboard, saved before a scenario and put back after it.
//!
//! Requirement 5 of §11.5 lists the clipboard among the things restored after every scenario.
//! The bench does not currently paste anything — the selection path of §4.7 is task T-07-2 and
//! position 15 is pending on it — but the requirement is about the state of the machine after
//! the run, not about whether this particular run had a reason to disturb it. Capturing and
//! comparing also turns "the clipboard was not touched" from a claim into a measurement.
//!
//! ⚠ **Only `CF_UNICODETEXT` is preserved.** A clipboard holding an image or a file list is
//! reported as "not text, left alone" and is not emptied — the bench never calls
//! `EmptyClipboard` unless it has text of its own to put back. This is a deliberate limit:
//! faithfully round-tripping every clipboard format needs delayed rendering and ownership
//! transfer, and getting that subtly wrong on somebody's real clipboard is a worse outcome
//! than not touching it.

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GHND, GlobalAlloc, GlobalLock, GlobalUnlock};

/// `CF_UNICODETEXT`.
const CF_UNICODETEXT: u32 = 13;

/// What the clipboard held when the scenario started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// Unicode text, which can be put back exactly.
    Text(String),
    /// Something the bench does not round-trip, or nothing at all. Left untouched.
    Other,
}

impl Saved {
    /// A short description for the report.
    pub fn describe(&self) -> String {
        match self {
            Self::Text(text) if text.chars().count() > 40 => {
                format!("текст, {} символов", text.chars().count())
            }
            Self::Text(text) => format!("текст {text:?}"),
            Self::Other => "не текст либо пусто — не трогается".to_owned(),
        }
    }
}

/// A scope guard: reads the clipboard now, puts it back when dropped.
///
/// `Drop` rather than an explicit call so that the restoration also happens on the panic path,
/// which is the path requirement 5 is least likely to be honoured on by hand.
pub struct Guard {
    saved: Saved,
}

impl Guard {
    pub fn capture() -> Self {
        Self { saved: read() }
    }

    /// What was found, so the report can state it.
    pub fn saved(&self) -> &Saved {
        &self.saved
    }

    /// Whether the clipboard right now still matches what was captured.
    pub fn unchanged(&self) -> bool {
        read() == self.saved
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Saved::Text(text) = &self.saved
            && read() != self.saved
        {
            let text = text.clone();
            if !write(&text) {
                eprintln!("warning: could not restore the clipboard text of the user");
            }
        }
    }
}

/// Opens the clipboard, runs `body`, closes it.
///
/// The clipboard is a single system-wide resource and another process can hold it; a failure
/// to open is an ordinary outcome answered with `None`, not an error path.
fn with_clipboard<T>(body: impl FnOnce() -> Option<T>) -> Option<T> {
    // SAFETY: `None` asks for the clipboard to be associated with no window, which is what a
    // process that only reads and restores wants. NFR-13: the `Result` is examined and a
    // failure returns before anything else is attempted, so `CloseClipboard` is never called
    // without a matching successful open.
    unsafe { OpenClipboard(None) }.ok()?;

    let result = body();

    // SAFETY: the clipboard was opened successfully immediately above and this is the matching
    // close. NFR-13: examined; a failure is reported and cannot be acted on further.
    if let Err(error) = unsafe { CloseClipboard() } {
        eprintln!("warning: CloseClipboard failed: {error}");
    }

    result
}

/// Reads `CF_UNICODETEXT`, or reports that there is nothing the bench round-trips.
fn read() -> Saved {
    with_clipboard(|| {
        // SAFETY: no dereference; asks whether the format is present. An error means "not
        // available", which is the `Other` case.
        if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
            return Some(Saved::Other);
        }

        // SAFETY: the clipboard is open and the format was just confirmed present. The handle
        // returned belongs to the clipboard and must not be freed here — it is only locked.
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.ok()?;

        // SAFETY: `handle` is a global memory handle from the clipboard. `GlobalLock` returns
        // a pointer to its bytes or null. NFR-13: the null is examined below.
        let pointer = unsafe { GlobalLock(windows::Win32::Foundation::HGLOBAL(handle.0)) };
        if pointer.is_null() {
            return Some(Saved::Other);
        }

        // SAFETY: `pointer` points at a NUL-terminated UTF-16 string — that is what
        // `CF_UNICODETEXT` is defined to be, and the clipboard guarantees the terminator. The
        // length is bounded by scanning for that terminator before any read of the body.
        let text = unsafe {
            let mut length = 0usize;
            while *pointer.cast::<u16>().add(length) != 0 {
                length += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(pointer.cast::<u16>(), length))
        };

        // SAFETY: the same handle that was locked immediately above, unlocked exactly once.
        let _ = unsafe { GlobalUnlock(windows::Win32::Foundation::HGLOBAL(handle.0)) };

        Some(Saved::Text(text))
    })
    .unwrap_or(Saved::Other)
}

/// Puts Unicode text back on the clipboard.
fn write(text: &str) -> bool {
    with_clipboard(|| {
        let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = std::mem::size_of_val(units.as_slice());

        // SAFETY: `GHND` asks for moveable, zero-initialised memory of the given size. NFR-13:
        // the `Result` is examined; a failure returns before anything is handed to the
        // clipboard.
        let global = unsafe { GlobalAlloc(GHND, bytes) }.ok()?;

        // SAFETY: `global` was just allocated and is locked exactly once here.
        let pointer = unsafe { GlobalLock(global) };
        if pointer.is_null() {
            return None;
        }

        // SAFETY: `pointer` addresses `bytes` writable bytes — the size the allocation was
        // made with — and `units` holds exactly that many bytes. Source and destination do not
        // overlap: one is a fresh global allocation, the other a local `Vec`.
        unsafe {
            std::ptr::copy_nonoverlapping(units.as_ptr().cast::<u8>(), pointer.cast::<u8>(), bytes);
            let _ = GlobalUnlock(global);
        }

        // SAFETY: the clipboard is open. `EmptyClipboard` is required before `SetClipboardData`
        // and also transfers ownership to this process, which is what makes the handover of
        // `global` below legal. NFR-13: examined.
        unsafe { EmptyClipboard() }.ok()?;

        // SAFETY: ownership of `global` passes to the clipboard on success, so it must not be
        // freed here — and it is not. NFR-13: the `Result` is examined and reported.
        unsafe { SetClipboardData(CF_UNICODETEXT, Some(HANDLE(global.0))) }.ok()?;

        Some(())
    })
    .is_some()
}
