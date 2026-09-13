//! **Finding Н96 of the audit of 2026-09-04, task T-40-4 — the replacement path frees no heap block
//! that still holds the user's text.**
//!
//! `inject::replace_in_with` holds the text of a replacement in two working buffers and zeroes both
//! before they are released (task T-13-15, SEC-01, SEC-02). Under the `testing` feature it also
//! publishes the shape of the packet for SEC-04a, and the count of distinct code units
//! (`distinct_units`, task T-10-6) used to be taken over a **third** copy — a `Vec<u16>` of the
//! character events — that went back to the allocator as it stood. The comment beside the working
//! buffers said they were the only place outside module `buffer` holding the text in plain form;
//! in a `testing` build that was not true.
//!
//! # Why a binary of its own, and why only with the feature
//!
//! The inspecting allocator below is the allocator of the whole binary it is linked into, so it
//! lives here alone, as `tests\selection_growth.rs` does for the same reason. It looks at a block
//! **at the moment it is freed**, on one thread — the thread a measurement is taken on — and the
//! measurements are serialised. The binary compiles only with the `testing` feature: the copy the
//! finding is about exists only there, and a check that ran without it would be green for the
//! wrong reason.
//!
//! # What it looks for
//!
//! The UTF-16LE bytes of `пр` — the first two characters of the replacement `ghbdtn` → `привет` —
//! anywhere in a freed block, whatever its alignment. A zeroed block cannot contain them; a copy of
//! the text that was not zeroed contains them however it was laid out.
//!
//! # The controls
//!
//! A detector that cannot fail proves nothing: the first test shows a block of the text freed as it
//! stands is found, and the same block zeroed first is not.
//!
//! Touches no window, no clipboard and no input: the replacement runs against an `Environment` that
//! accepts every event and sends none.

#![cfg(feature = "testing")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use lang_switcher::convert::{self, Keystroke};
use lang_switcher::inject::{self, Environment, Modifiers};
use lang_switcher::layouts::Mods;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::INPUT;

// ---------------------------------------------------------------------------------------
// The inspecting allocator
// ---------------------------------------------------------------------------------------

/// The UTF-16LE bytes of `пр`: `U+043F`, `U+0440`.
const NEEDLE: [u8; 4] = [0x3F, 0x04, 0x40, 0x04];

/// The Win32 id of the thread a measurement is being taken on, or zero between measurements.
static MEASURED_THREAD: AtomicU32 = AtomicU32::new(0);

/// Blocks freed on the measured thread that still held the needle.
static TEXT_BLOCKS: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, with every block freed on the measured thread read before it goes back.
struct Inspecting;

/// Whether `bytes` contains [`NEEDLE`]. No allocation: a window over the slice.
fn holds_the_needle(bytes: &[u8]) -> bool {
    bytes.windows(NEEDLE.len()).any(|window| window == NEEDLE)
}

// SAFETY: every call is forwarded to `System` with the arguments it was given, so the contract of
// `GlobalAlloc` is the one `System` keeps. The inspection reads a block the caller still owns, the
// whole of `layout.size()` of it, before it is handed back; it reads atomics and asks the id of the
// calling thread. None of that allocates and none of it unwinds.
unsafe impl GlobalAlloc for Inspecting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let measured = MEASURED_THREAD.load(Ordering::Relaxed);

        // SAFETY: `GetCurrentThreadId` takes no argument and cannot fail.
        if measured != 0 && measured == unsafe { GetCurrentThreadId() } && layout.size() > 0 {
            // SAFETY: `ptr` is a live block of `layout.size()` bytes that this allocator handed out
            // and the caller has not yet given back — the contract of `dealloc` — so every byte of
            // it is readable for the length of this call. It is read, never written.
            let bytes = unsafe { std::slice::from_raw_parts(ptr, layout.size()) };

            if holds_the_needle(bytes) {
                TEXT_BLOCKS.fetch_add(1, Ordering::Relaxed);
            }
        }

        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A growth hands the old block back as well, and the text in it would be a copy nobody
        // zeroed — so it is read exactly as a freed block is.
        let measured = MEASURED_THREAD.load(Ordering::Relaxed);

        // SAFETY: `GetCurrentThreadId` takes no argument and cannot fail.
        if measured != 0 && measured == unsafe { GetCurrentThreadId() } && layout.size() > 0 {
            // SAFETY: as in `dealloc` — a live block of `layout.size()` bytes, read before it moves.
            let bytes = unsafe { std::slice::from_raw_parts(ptr, layout.size()) };

            if holds_the_needle(bytes) {
                TEXT_BLOCKS.fetch_add(1, Ordering::Relaxed);
            }
        }

        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Inspecting = Inspecting;

/// Serialises the measurements — see the module documentation.
static MEASUREMENTS: Mutex<()> = Mutex::new(());

fn serialised() -> MutexGuard<'static, ()> {
    MEASUREMENTS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `action` on this thread with the inspection armed, and answers its result and how many
/// freed blocks still held the text.
fn measured<R>(action: impl FnOnce() -> R) -> (R, usize) {
    TEXT_BLOCKS.store(0, Ordering::Relaxed);

    // SAFETY: `GetCurrentThreadId` takes no argument and cannot fail.
    MEASURED_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);

    let result = action();

    MEASURED_THREAD.store(0, Ordering::Relaxed);

    (result, TEXT_BLOCKS.load(Ordering::Relaxed))
}

// ---------------------------------------------------------------------------------------
// The replacement
// ---------------------------------------------------------------------------------------

/// An environment that holds nothing, takes every event whole and sends none.
struct Silent;

impl Environment for Silent {
    fn held(&mut self) -> Modifiers {
        Modifiers::NONE
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        u32::try_from(events.len()).unwrap_or(u32::MAX)
    }

    fn pause(&mut self, _delay_ms: u32) {}
}

/// Scan codes of `g h b d t n`.
const GHBDTN: [u16; 6] = [0x22, 0x23, 0x30, 0x20, 0x14, 0x31];

// ---------------------------------------------------------------------------------------
// The checks
// ---------------------------------------------------------------------------------------

/// The code units of `text` in a block allocated once at its final size — no growth, so the
/// controls below count exactly the one block they free.
fn units_of(text: &str) -> Vec<u16> {
    let mut units = Vec::with_capacity(text.encode_utf16().count());
    units.extend(text.encode_utf16());
    units
}

/// **The controls.** A block of the text freed as it stands is found; the same block zeroed first is
/// not; a block that never held the text is not; and a block that **grew** is found twice — once
/// for the old block the growth handed back, once for the new one — because a growth is a copy too.
#[test]
fn the_detector_finds_a_block_of_the_text_and_passes_a_zeroed_one() {
    let _serialised = serialised();

    let ((), found) = measured(|| drop(std::hint::black_box(units_of("привет"))));
    assert_eq!(found, 1, "a block of the text freed as it stands is found");

    let ((), found) = measured(|| {
        let mut units = units_of("привет");
        for unit in &mut units {
            // SAFETY: `unit` is an exclusive borrow of an element of a live vector — aligned,
            // in bounds and valid for a write of one `u16`.
            unsafe { std::ptr::write_volatile(unit, 0) };
        }
        drop(std::hint::black_box(units));
    });
    assert_eq!(found, 0, "the same block zeroed first is not");

    let ((), found) = measured(|| drop(std::hint::black_box(units_of("ghbdtn"))));
    assert_eq!(found, 0, "a block that never held the text is not");

    let ((), found) = measured(|| {
        let mut units: Vec<u16> = Vec::with_capacity(2);
        units.extend("привет".encode_utf16());
        drop(std::hint::black_box(units));
    });
    assert!(
        found >= 2,
        "a block that grew leaves the text in the block it gave back, and is found for it: {found}"
    );
}

/// **Н96.** The replacement of `ghbdtn` by `привет` frees no block that still holds `пр` — not the
/// two working buffers, and not the count of distinct units that SEC-04a publishes in a `testing`
/// build.
#[test]
fn the_replacement_path_frees_no_block_that_still_holds_the_text() {
    let _serialised = serialised();

    // Everything the press is given is built before the measurement starts: the strokes and the
    // maps are the test's own, and are freed after it.
    let us = convert::fallback_map(convert::FALLBACK_US).expect("the US map of FR-25");
    let russian = convert::fallback_map(convert::FALLBACK_RUSSIAN).expect("the Russian map");
    let strokes: Vec<Keystroke> = GHBDTN
        .iter()
        .map(|&scan| Keystroke::recorded_in(&us, scan, false, Mods::NONE))
        .collect();

    let (outcome, found) = measured(|| inject::replace_in(&mut Silent, &strokes, &russian, 0));

    let replaced = outcome.expect("the packet is sized from the lengths it is built with");
    assert_eq!(
        (replaced.erased, replaced.typed),
        (6, 6),
        "the premise: six characters really went through the path"
    );
    assert_eq!(
        found, 0,
        "SEC-01, SEC-02: a block freed on the replacement path still held the text"
    );
}
