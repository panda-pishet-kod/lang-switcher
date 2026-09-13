//! **Finding Н34 of the audit of 2026-09-04, task T-38-9B — the working buffers of steps 4 and 5
//! are allocated once, at a size counted before they are filled.**
//!
//! A growing `Vec<u16>` or `String` is not only slow: every growth copies the user's text into a
//! new block and hands the old one back to the allocator **as it stands**, and no zeroing written
//! after the fact reaches a block that is already freed. Task T-38-9A named every copy the module
//! makes and zeroes; a copy made by a growth has no name, and this binary is where it is caught.
//!
//! # Why a binary of its own
//!
//! The counting allocator below is the allocator of the whole binary it is linked into — every
//! test of `tests\selection.rs` would run under it. So it lives here, alone, and it counts on
//! **one thread**: the thread a measurement is taken on, named by its Win32 id, because the harness
//! of `cargo test` has threads of its own and a count that took theirs in could report a growth
//! that is not the product's. The measurements are serialised as well, one at a time.
//!
//! # What it counts, and why by alignment
//!
//! `realloc` of a block aligned to 2 is a `Vec<u16>` — the code units; aligned to 1, a `String` or
//! a `Vec<u8>` — the text itself. Allocations and frees are not counted: a buffer made once at its
//! final size is exactly what is wanted, and it is the growth that leaves a copy behind.
//!
//! # The controls
//!
//! A counter that cannot fail proves nothing. The first two tests show it both ways: buffers
//! grown by pushes are counted, each under its own alignment, and the same buffers made at their
//! size first are not.
//!
//! Touches no clipboard, no window and no input: step 4 is driven through
//! `selection::decode_utf16`, and the eight steps through a [`SelectionPath`] whose read is a tape.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use lang_switcher::convert;
use lang_switcher::inject::{Dispatched, Modifiers};
use lang_switcher::layouts::{Cycle, KeyMapping, LayoutId, LayoutMap, LayoutMapBuilder, MAX_UNITS};
use lang_switcher::selection::{
    self, CHORD_EVENTS, ClipboardError, ClipboardText, Outcome, Path as SelectionPath, Plan,
    ProbeAnswer, Snapshot, Wait,
};

use windows::Win32::System::Threading::GetCurrentThreadId;

// ---------------------------------------------------------------------------------------
// The counting allocator
// ---------------------------------------------------------------------------------------

/// The Win32 id of the thread a measurement is being taken on, or zero between measurements.
static MEASURED_THREAD: AtomicU32 = AtomicU32::new(0);

/// `realloc` of blocks aligned to 2 — `Vec<u16>`, the code units — on the measured thread.
static UNIT_GROWTHS: AtomicUsize = AtomicUsize::new(0);

/// `realloc` of blocks aligned to 1 — `String` and `Vec<u8>`, the text — on the measured thread.
static BYTE_GROWTHS: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, with every `realloc` on the measured thread counted by alignment.
struct Counting;

// SAFETY: every call is forwarded to `System` with the arguments it was given, so the contract of
// `GlobalAlloc` is the one `System` keeps. The counting reads and writes atomics and asks the id
// of the calling thread; none of that allocates, and none of it unwinds.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let measured = MEASURED_THREAD.load(Ordering::Relaxed);

        // SAFETY: `GetCurrentThreadId` takes no argument and cannot fail.
        if measured != 0 && measured == unsafe { GetCurrentThreadId() } {
            match layout.align() {
                2 => {
                    UNIT_GROWTHS.fetch_add(1, Ordering::Relaxed);
                }
                1 => {
                    BYTE_GROWTHS.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }

        // SAFETY: forwarded unchanged; the caller's obligations are those of `System`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// What one measurement found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Growths {
    /// `realloc` of code units.
    units: usize,
    /// `realloc` of text.
    bytes: usize,
}

impl Growths {
    /// Nothing grew.
    const NONE: Self = Self { units: 0, bytes: 0 };
}

/// Serialises the measurements — see the module documentation.
static MEASUREMENTS: Mutex<()> = Mutex::new(());

fn serialised() -> MutexGuard<'static, ()> {
    MEASUREMENTS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `action` on this thread with the counter armed, and answers its result and the growths.
fn measured<R>(action: impl FnOnce() -> R) -> (R, Growths) {
    UNIT_GROWTHS.store(0, Ordering::Relaxed);
    BYTE_GROWTHS.store(0, Ordering::Relaxed);

    // SAFETY: `GetCurrentThreadId` takes no argument and cannot fail.
    MEASURED_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);

    let result = action();

    MEASURED_THREAD.store(0, Ordering::Relaxed);

    let growths = Growths {
        units: UNIT_GROWTHS.load(Ordering::Relaxed),
        bytes: BYTE_GROWTHS.load(Ordering::Relaxed),
    };

    (result, growths)
}

// ---------------------------------------------------------------------------------------
// Layouts, plans and a tape
// ---------------------------------------------------------------------------------------

/// A layout with a four-unit ligature on every letter key of US — the most code units one key may
/// make ([`MAX_UNITS`]), and the case a buffer sized in bytes of the source text is too small for.
const LIGATURE_LAYOUT: LayoutId = LayoutId::from_raw(0x0449_0449);

/// Four UTF-16 units of Tamil, three bytes each in UTF-8.
const LIGATURE: [u16; MAX_UNITS] = [0x0B95, 0x0BCD, 0x0BB7, 0x0BBF];

fn us() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_US).expect("the US map of FR-25")
}

fn ru() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_RUSSIAN).expect("the Russian map of FR-25")
}

fn ligature(us: &LayoutMap) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(LIGATURE_LAYOUT);

    for ch in 'a'..='z' {
        let key = us.find_key(ch).expect("every Latin letter is a key of US");
        let mapping = KeyMapping::from_to_unicode(MAX_UNITS as i32, &LIGATURE);

        assert!(
            builder.set(key.scan, key.extended, key.mods, mapping),
            "the ligature is set on the key of {ch}"
        );
    }

    builder.finish()
}

fn plan(maps: Vec<LayoutMap>, layouts: &[LayoutId]) -> Plan {
    Plan::new(
        maps,
        Cycle::from_layouts(layouts).expect("a cycle of two layouts"),
        layouts[0],
        Duration::from_millis(300),
        Duration::from_millis(200),
        0,
    )
}

/// `text` as the bytes of a `CF_UNICODETEXT` block, terminator included.
fn block_of(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// A chord the system took whole.
fn taken_whole() -> Dispatched {
    Dispatched {
        calls: 1,
        requested: CHORD_EVENTS,
        accepted: CHORD_EVENTS,
    }
}

/// A path whose outside world answers «there is a selection» and hands `reads` to step 4.
struct Tape {
    reads: Option<String>,
}

impl SelectionPath for Tape {
    fn snapshot(&mut self) -> Result<Snapshot, ClipboardError> {
        Ok(Snapshot::empty())
    }

    fn release_modifiers(&mut self) -> Modifiers {
        Modifiers::NONE
    }

    fn copy(&mut self) -> Dispatched {
        taken_whole()
    }

    fn wait(&mut self, _baseline: u32) -> Wait {
        Wait::Changed {
            sequence: 1,
            waited: Duration::from_millis(7),
        }
    }

    fn sequence(&mut self) -> u32 {
        1
    }

    fn read(&mut self) -> Result<Option<ClipboardText>, ClipboardError> {
        Ok(self.reads.take().map(ClipboardText::from))
    }

    fn write(&mut self, _text: &str) -> Result<(), ClipboardError> {
        Ok(())
    }

    fn paste(&mut self) -> Dispatched {
        taken_whole()
    }

    fn switch(&mut self, _target: LayoutId) {}

    fn restore_clipboard(&mut self, _snapshot: &Snapshot, _answer: ProbeAnswer) {}

    fn reclaim_clipboard(&mut self, _snapshot: &Snapshot) {}
}

// ---------------------------------------------------------------------------------------
// The controls
// ---------------------------------------------------------------------------------------

/// **The control, one way — the counter finds a growth.** Code units and text grown by pushes are
/// both counted, each under its own alignment.
#[test]
fn buffers_grown_by_pushes_are_counted_under_their_own_alignment() {
    let _serialised = serialised();

    let ((), grown) = measured(|| {
        let mut units: Vec<u16> = Vec::new();
        let mut text = String::new();

        for unit in 0..1000_u16 {
            units.push(unit);
            text.push('щ');
        }
    });

    assert!(
        grown.units > 0,
        "the code units grew and were counted: {grown:?}"
    );
    assert!(grown.bytes > 0, "the text grew and was counted: {grown:?}");
}

/// **The control, the other way — the counter invents no growth.** The same buffers, made at their
/// size first.
#[test]
fn buffers_made_at_their_size_are_not_counted() {
    let _serialised = serialised();

    let ((), grown) = measured(|| {
        let mut units: Vec<u16> = Vec::with_capacity(1000);
        let mut text = String::with_capacity(2000);

        for unit in 0..1000_u16 {
            units.push(unit);
            text.push('щ');
        }
    });

    assert_eq!(grown, Growths::NONE, "nothing grew");
}

// ---------------------------------------------------------------------------------------
// The product
// ---------------------------------------------------------------------------------------

/// **Step 4 — `decode_utf16`.** Four thousand and four code units, the read the premise of the
/// report measured growing its code units ten times and its text once.
#[test]
fn step_four_decodes_into_buffers_made_once() {
    let _serialised = serialised();

    let block = block_of(&"ghbdtn ".repeat(572));

    let (text, grown) = measured(|| selection::decode_utf16(&block));

    assert_eq!(text.len(), 4004, "the whole block was decoded");
    assert_eq!(
        grown,
        Growths::NONE,
        "neither the code units nor the text grew"
    );
}

/// **Step 5 — `recode` into a layout of one unit per key.** Latin into Russian: the code units fit
/// the old estimate in bytes of the source, and the text did not — it grew out of a collect.
#[test]
fn recode_makes_its_buffers_once() {
    let _serialised = serialised();

    let (us, ru) = (us(), ru());
    let latin = "ghbdtn ".repeat(500);

    let (recoded, grown) = measured(|| selection::recode(&latin, &us, &ru));

    assert_eq!(
        recoded.text().chars().count(),
        3500,
        "every character came out"
    );
    assert_eq!(
        grown,
        Growths::NONE,
        "neither the code units nor the text grew"
    );
}

/// **Step 5 — `recode` into a layout of four units per key.** The case the old estimate in bytes of
/// the source was too small for: 3500 bytes of Latin make 12 500 code units.
#[test]
fn recode_into_ligatures_makes_its_buffers_once() {
    let _serialised = serialised();

    let us = us();
    let ligature = ligature(&us);
    let latin = "ghbdtn ".repeat(500);

    let (recoded, grown) = measured(|| selection::recode(&latin, &us, &ligature));

    assert_eq!(
        recoded.text().encode_utf16().count(),
        12_500,
        "four units per letter"
    );
    assert_eq!(
        grown,
        Growths::NONE,
        "neither the code units nor the text grew"
    );
}

/// **Step 5 — `recode_words`, through the eight steps.** The same Latin text under two plans, word
/// by word: RU/EN, where the text grew, and US with a four-unit ligature, where both buffers did.
#[test]
fn recode_words_makes_its_buffers_once_through_the_eight_steps() {
    let _serialised = serialised();

    let (us, ru) = (us(), ru());
    let ligature = ligature(&us);
    let latin = "ghbdtn ".repeat(500);

    let pair = plan(
        vec![us.clone(), ru],
        &[convert::FALLBACK_US, convert::FALLBACK_RUSSIAN],
    );
    let ligatures = plan(vec![us, ligature], &[convert::FALLBACK_US, LIGATURE_LAYOUT]);

    let mut found = Vec::new();

    for (plan, what) in [(&pair, "RU/EN"), (&ligatures, "US and a ligature")] {
        let mut tape = Tape {
            reads: Some(latin.clone()),
        };

        let (outcome, grown) = measured(|| selection::run(&mut tape, plan));

        assert!(
            matches!(outcome, Outcome::Converted { .. }),
            "{what}: the text was converted, and it was {outcome:?}"
        );

        found.push((what, grown));
    }

    assert!(
        found.iter().all(|&(_, grown)| grown == Growths::NONE),
        "neither the code units nor the text grew: {found:?}"
    );
}
