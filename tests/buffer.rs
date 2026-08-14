//! Integration tests for task T-03-2 — the ring buffer of key strokes.
//!
//! # Why these tests need no keyboard and no layouts
//!
//! [`lang_switcher::buffer::Recorder`] is a plain owned value, the same split module `hook`
//! made for `classify`: the rules of FR-10, the eviction of FR-07 and the zeroing of SEC-02
//! are decided by code that touches neither Win32 nor a global, so every row of the flush
//! table can be driven deterministically, one stroke at a time. The mapping cache the
//! decoding of FR-06 reads is built here by hand out of the public parts of module `layouts`,
//! so the tests answer the same on any machine and whatever layouts happen to be installed.
//!
//! Two tests do go through [`lang_switcher::hook::classify`], because FR-03 — our own injected
//! input must never come back into the buffer — is a property of the *seam* between the two
//! modules and cannot be shown from either side alone.
//!
//! **No test here installs a real hook**, for the reason `tests\hook.rs` states: a
//! `WH_KEYBOARD_LL` hook installed by a process that is not pumping messages freezes the
//! keyboard of whoever is running `cargo test`.
//!
//! # Printing
//!
//! `Stroke` and `StrokeMods` have no `Debug` on purpose (SEC-01, SEC-07), so `assert_eq!` does
//! not compile on them and the comparisons below are written with `==` and a message. That is
//! the intended friction: the product cannot format a keystroke by accident either. Characters
//! that do appear in messages here are the string constants of this file, never user input.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use std::alloc::System;

use lang_switcher::buffer::{
    self, DEFAULT_CAPACITY, MAX_CAPACITY, ReadError, Recorded, Recorder, ResetOutcome, Stroke,
    StrokeMods,
};
use lang_switcher::convert::{Keystroke, convert_stroke, max_units};
use lang_switcher::hook::{self, Decision, Edge, HotkeyState, INJECTED_SIGNATURE, KeyEvent, Mode};
use lang_switcher::layouts::{
    KeyMapping, LayoutCache, LayoutId, LayoutMap, LayoutMapBuilder, MAX_UNITS, Mods,
};
use lang_switcher::settings;
use windows::Win32::UI::WindowsAndMessaging::{LLKHF_ALTDOWN, LLKHF_EXTENDED};

// -------------------------------------------------------------------------------------
// The allocation counter of point 21 — NFR-03
// -------------------------------------------------------------------------------------

/// Counts every allocation made by the thread that armed it.
///
/// A global allocator is the only honest way to check NFR-03: "no `Vec` in the source" is a
/// code review, whereas this is a measurement. The counters are thread-local, so the arming of
/// one test cannot see the allocations of another test running beside it, and they are
/// `const`-initialised `Cell`s with no destructor, so touching them never allocates and can
/// never recurse into the allocator.
struct CountingAllocator;

thread_local! {
    /// Whether this thread is inside a region that must not allocate.
    static ARMED: Cell<bool> = const { Cell::new(false) };
    /// Allocator calls made by this thread while armed.
    static EVENTS: Cell<usize> = const { Cell::new(0) };
}

impl CountingAllocator {
    /// Records one allocator call, if this thread is watching for them.
    fn note() {
        let _ = ARMED.try_with(|armed| {
            if armed.get() {
                let _ = EVENTS.try_with(|events| events.set(events.get() + 1));
            }
        });
    }
}

// SAFETY: every method below forwards its arguments unchanged to `std::alloc::System`, which
// is a correct `GlobalAlloc`, and returns what it returns; the counter in front of the call
// touches only two thread-local `Cell`s of `Copy` types with no destructor, so it cannot
// allocate, cannot panic and cannot re-enter the allocator. The layouts and pointers this
// implementation hands on are exactly the ones it was given, so every contract it has to
// uphold is the one `System` already upholds.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::note();
        // SAFETY: `layout` is the caller's, forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        Self::note();
        // SAFETY: `ptr` was produced by this allocator, which is `System`, under `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Self::note();
        // SAFETY: as for `dealloc`, and `new_size` is the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Self::note();
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Runs `action` with the allocation counter armed and returns how many calls it made.
fn allocations_during<R>(action: impl FnOnce() -> R) -> (R, usize) {
    EVENTS.with(|events| events.set(0));
    ARMED.with(|armed| armed.set(true));

    let result = action();

    ARMED.with(|armed| armed.set(false));

    (result, EVENTS.with(Cell::get))
}

// -------------------------------------------------------------------------------------
// Synthetic layouts
// -------------------------------------------------------------------------------------

/// US, the layout the strokes below are recorded under.
const EN: LayoutId = LayoutId::from_raw(0x0409_0409);
/// Russian, the layout they are converted into.
const RU: LayoutId = LayoutId::from_raw(0x0419_0419);
/// A layout that is in no cache — the "no mapping" outcome of FR-06.
const UNKNOWN: LayoutId = LayoutId::from_raw(0x0407_0407);

/// Scan code of the `A` key of the main block.
const SCAN_A: u16 = 0x1E;
/// Scan code of the `2` key — the `Shift+2` of section 4.1, `@` in English and `"` in Russian.
const SCAN_2: u16 = 0x03;
/// Scan code shared by the `/?` key of the main block and the `/` of the keypad.
const SCAN_SLASH: u16 = 0x35;
/// Scan code of the `Z` key, which carries the four-unit ligature below.
const SCAN_Z: u16 = 0x2C;
/// Scan code of the space bar — the `Space` of `Win+Space`, task T-03-3b.
const SCAN_SPACE: u16 = 0x39;

/// A key of the main block: the `LLKHF_EXTENDED` of FR-05 is clear.
const MAIN_BLOCK: bool = false;
/// A key of the keypad: the flag is set.
const KEYPAD: bool = true;

/// A ligature of the full [`MAX_UNITS`] — the longest result FR-04 can carry.
const LIGATURE: [u16; MAX_UNITS] = [b'a' as u16, b'b' as u16, b'c' as u16, b'd' as u16];

/// Builds a layout map out of `(scan, extended, mods, mapping)` rows.
fn map_of(layout: LayoutId, rows: &[(u16, bool, Mods, KeyMapping)]) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(layout);

    for &(scan, extended, mods, mapping) in rows {
        builder.set(scan, extended, mods, mapping);
    }

    builder.finish()
}

/// The English half of the synthetic cache.
fn english() -> LayoutMap {
    map_of(
        EN,
        &[
            (SCAN_A, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('a')),
            (SCAN_A, MAIN_BLOCK, Mods::SHIFT, KeyMapping::from_char('A')),
            (SCAN_A, MAIN_BLOCK, Mods::CAPS, KeyMapping::from_char('A')),
            (SCAN_2, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('2')),
            (SCAN_2, MAIN_BLOCK, Mods::SHIFT, KeyMapping::from_char('@')),
            (
                SCAN_SLASH,
                MAIN_BLOCK,
                Mods::NONE,
                KeyMapping::from_char('/'),
            ),
            // The same scan code on the keypad, which only the extended flag of FR-05 tells
            // apart, and a different character on it so that the two can be distinguished by
            // the result alone.
            (SCAN_SLASH, KEYPAD, Mods::NONE, KeyMapping::from_char('*')),
            (
                SCAN_Z,
                MAIN_BLOCK,
                Mods::NONE,
                KeyMapping::from_to_unicode(MAX_UNITS as i32, &LIGATURE),
            ),
        ],
    )
}

/// The Russian half of the synthetic cache.
fn russian() -> LayoutMap {
    map_of(
        RU,
        &[
            (SCAN_A, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('ф')),
            (SCAN_A, MAIN_BLOCK, Mods::SHIFT, KeyMapping::from_char('Ф')),
            (SCAN_2, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('2')),
            (SCAN_2, MAIN_BLOCK, Mods::SHIFT, KeyMapping::from_char('"')),
            (
                SCAN_SLASH,
                MAIN_BLOCK,
                Mods::NONE,
                KeyMapping::from_char('.'),
            ),
        ],
    )
}

/// The cache of FR-20, as module `layouts` would hand it over.
fn cache() -> LayoutCache {
    LayoutCache::from_maps(vec![english(), russian()]).expect("two non-empty maps")
}

// -------------------------------------------------------------------------------------
// Driving the recorder
// -------------------------------------------------------------------------------------

/// Virtual keys used below. The letters are their own codes.
const VK_A: u16 = 0x41;
const VK_B: u16 = 0x42;
const VK_2: u16 = 0x32;
const VK_Z: u16 = 0x5A;
const VK_BACK: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0D;
const VK_ESCAPE: u16 = 0x1B;
const VK_SPACE: u16 = 0x20;
const VK_PRIOR: u16 = 0x21;
const VK_NEXT: u16 = 0x22;
const VK_END: u16 = 0x23;
const VK_HOME: u16 = 0x24;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_INSERT: u16 = 0x2D;
const VK_DELETE: u16 = 0x2E;
const VK_LSHIFT: u16 = 0xA0;
const VK_LCONTROL: u16 = 0xA2;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;
const VK_LWIN: u16 = 0x5B;
const VK_CAPITAL: u16 = 0x14;
/// `Pause`, the default hotkey of section 7.
const VK_PAUSE: u16 = 0x13;

/// `dwExtraInfo` of a stroke that is not ours — anything but [`INJECTED_SIGNATURE`].
const FOREIGN_SIGNATURE: usize = 0x00CA_FE01;

/// Timestamp used where the value does not matter.
const SOME_TIME: u32 = 4_242;

/// A recorder with the synthetic cache published and English active.
fn fresh() -> Recorder {
    fresh_of(DEFAULT_CAPACITY)
}

/// The same, of a chosen capacity.
fn fresh_of(capacity: usize) -> Recorder {
    let mut recorder = Recorder::with_capacity(capacity);
    recorder.set_cache(cache());
    recorder.set_active_layout(EN);
    recorder
}

/// One key event, exactly as the callback would deliver it.
fn deliver(
    recorder: &mut Recorder,
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    edge: Edge,
) -> Recorded {
    recorder.record(KeyEvent {
        vk,
        edge,
        extra_info: FOREIGN_SIGNATURE,
        scan,
        flags,
        time,
    })
}

/// A press of an ordinary key.
fn press(recorder: &mut Recorder, vk: u16, scan: u16) -> Recorded {
    deliver(recorder, vk, scan, 0, SOME_TIME, Edge::Down)
}

/// A press and a release of a modifier.
fn hold(recorder: &mut Recorder, vk: u16) -> Recorded {
    deliver(recorder, vk, 0, 0, SOME_TIME, Edge::Down)
}

/// Releases a modifier.
fn release(recorder: &mut Recorder, vk: u16) -> Recorded {
    deliver(recorder, vk, 0, 0, SOME_TIME, Edge::Up)
}

/// Fills the buffer with `count` presses of `A` and checks that they arrived.
fn fill(recorder: &mut Recorder, count: usize) {
    for _ in 0..count {
        assert_eq!(press(recorder, VK_A, SCAN_A), Recorded::Stored);
    }

    assert_eq!(recorder.len(), count);
}

/// The characters the live strokes produced, as a `String` — for readable assertions.
///
/// A test-only convenience over data this test file put in itself; the product has no such
/// function and could not have one (SEC-07).
fn typed(recorder: &Recorder) -> String {
    let mut units = Vec::new();

    for index in 0..recorder.len() {
        let stroke = recorder.stroke(index).expect("index below the length");
        units.extend_from_slice(stroke.units());
    }

    String::from_utf16(&units).expect("the synthetic layouts carry valid text")
}

// -------------------------------------------------------------------------------------
// Point 9 — FR-04: the stroke carries every field of section 4.1
// -------------------------------------------------------------------------------------

#[test]
fn a_stroke_carries_every_field_of_section_4_1() {
    let mut recorder = fresh();

    assert_eq!(hold(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(
        deliver(&mut recorder, VK_2, SCAN_2, 0, 90_210, Edge::Down),
        Recorded::Stored
    );

    let stroke = recorder.stroke(0).expect("one stroke was recorded");

    // `vk`, `scan`, `mods`, `hkl`, `time`, `chars`, `len` — the seven fields of section 4.1,
    // with the meaning section 4.1 gives them.
    assert_eq!(stroke.vk(), VK_2);
    assert_eq!(stroke.scan(), SCAN_2);
    assert!(stroke.mods().contains(StrokeMods::SHIFT));
    assert_eq!(stroke.hkl(), EN);
    assert_eq!(stroke.time(), 90_210);
    assert_eq!(stroke.chars(), [u16::from(b'@'), 0, 0, 0]);
    assert_eq!(stroke.len(), 1);
    assert_eq!(stroke.units(), [u16::from(b'@')]);

    // And the reason section 4.1 gives for storing the scan code rather than the character:
    // the *same physical key* is `@` in English and `"` in Russian, and conversion asks the
    // target layout what the key gives rather than what the character maps to.
    let converted = convert_stroke(stroke.keystroke(), &russian());

    assert_eq!(converted.units(), [u16::from(b'"')]);
}

// -------------------------------------------------------------------------------------
// Point 10 — FR-05: `LLKHF_EXTENDED` is kept in the mask
// -------------------------------------------------------------------------------------

#[test]
fn the_extended_flag_of_fr05_is_kept_in_the_mask() {
    let mut recorder = fresh();

    // The same scan code twice: once from the main block, once from the keypad. Only the flag
    // tells them apart, and the synthetic layout gives them different characters so that the
    // difference is visible in the result as well.
    assert_eq!(
        deliver(&mut recorder, VK_A, SCAN_SLASH, 0, SOME_TIME, Edge::Down),
        Recorded::Stored
    );
    assert_eq!(
        deliver(
            &mut recorder,
            VK_A,
            SCAN_SLASH,
            LLKHF_EXTENDED.0,
            SOME_TIME,
            Edge::Down
        ),
        Recorded::Stored
    );

    let main_block = recorder.stroke(0).expect("the first stroke");
    let keypad = recorder.stroke(1).expect("the second stroke");

    assert!(
        !main_block.mods().extended(),
        "the main block is not extended"
    );
    assert!(
        keypad.mods().extended(),
        "FR-05: the flag is kept in `mods`"
    );

    assert_eq!(typed(&recorder), "/*");

    // FR-05 also says the flag is restored on injection, which is task T-04-1's half. What
    // this task owes it is the flag on the `Keystroke` it produces.
    assert!(!main_block.keystroke().extended());
    assert!(keypad.keystroke().extended());
}

// -------------------------------------------------------------------------------------
// Point 11 — FR-06: the characters come from the cache, and a ligature is kept whole
// -------------------------------------------------------------------------------------

#[test]
fn characters_come_from_the_cache_and_a_ligature_is_kept_whole() {
    let mut recorder = fresh();

    // Plain, shifted and with `CapsLock` on: three different rows of the same cached key.
    press(&mut recorder, VK_A, SCAN_A);

    assert_eq!(hold(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    press(&mut recorder, VK_A, SCAN_A);
    release(&mut recorder, VK_LSHIFT);

    // `CapsLock` is a toggle: the press flips it and the release does nothing.
    hold(&mut recorder, VK_CAPITAL);
    release(&mut recorder, VK_CAPITAL);
    press(&mut recorder, VK_A, SCAN_A);

    assert_eq!(typed(&recorder), "aAA");

    // A ligature of the full four units survives whole — the width of `chars` in FR-04 and of
    // `MAX_UNITS` in module `layouts`.
    let mut recorder = fresh();
    assert_eq!(press(&mut recorder, VK_Z, SCAN_Z), Recorded::Stored);

    let ligature = recorder.stroke(0).expect("the ligature");

    assert_eq!(ligature.len(), MAX_UNITS);
    assert_eq!(ligature.units(), LIGATURE);
    assert_eq!(ligature.chars(), LIGATURE);
    assert!(ligature.produced().is_ligature());
}

#[test]
fn a_combination_the_cache_has_no_answer_for_is_stored_empty() {
    // A key the layout carries nothing on: stored, with `len` zero. An outcome, not an error —
    // FR-23 carries such a stroke through conversion unchanged.
    let mut recorder = fresh();

    assert_eq!(press(&mut recorder, VK_B, 0x30), Recorded::Stored);

    let stroke = recorder.stroke(0).expect("the stroke was stored anyway");

    assert_eq!(stroke.len(), 0);
    assert!(stroke.is_empty());
    assert_eq!(stroke.chars(), [0; MAX_UNITS]);
    assert_eq!(stroke.scan(), 0x30, "what was typed is still known");

    // The same when the active layout is not in the cache, and when there is no cache at all.
    let mut recorder = fresh();
    recorder.set_active_layout(UNKNOWN);
    press(&mut recorder, VK_A, SCAN_A);

    let mut bare = Recorder::with_capacity(8);
    assert!(!bare.has_cache());
    press(&mut bare, VK_A, SCAN_A);

    assert_eq!(recorder.stroke(0).expect("stored").len(), 0);
    assert_eq!(bare.stroke(0).expect("stored").len(), 0);
    assert_eq!(bare.stroke(0).expect("stored").scan(), SCAN_A);
}

// -------------------------------------------------------------------------------------
// Point 12 — FR-07: the capacity comes from the configuration, the oldest is evicted
// -------------------------------------------------------------------------------------

#[test]
fn the_capacity_comes_from_the_configuration() {
    // Section 7 says 256 and FR-07 says 256; the value is read from the configuration rather
    // than written into the code, and the two agree.
    let default = Recorder::from_config(&settings::Buffer::default());

    assert_eq!(default.capacity(), DEFAULT_CAPACITY);
    assert_eq!(default.capacity(), 256);

    // A configured value is honoured...
    let configured = Recorder::from_config(&settings::Buffer { capacity: 12 });
    assert_eq!(configured.capacity(), 12);

    // ...and a value that cannot be meant is not allocated.
    let absurd = Recorder::from_config(&settings::Buffer {
        capacity: usize::MAX,
    });
    assert_eq!(absurd.capacity(), MAX_CAPACITY);
}

#[test]
fn overflow_evicts_the_oldest_stroke() {
    let mut recorder = fresh_of(4);

    // Four strokes fill it exactly.
    for vk in [VK_A, VK_B, VK_2, VK_Z] {
        assert_eq!(press(&mut recorder, vk, SCAN_A), Recorded::Stored);
    }
    assert_eq!(recorder.len(), 4);

    // The fifth pushes the first one out, and the buffer stays at its capacity: FR-07 evicts
    // the oldest, it does not stop accepting input and it does not grow.
    assert_eq!(press(&mut recorder, VK_B, SCAN_A), Recorded::Evicted);
    assert_eq!(press(&mut recorder, VK_2, SCAN_A), Recorded::Evicted);

    assert_eq!(recorder.len(), 4);
    assert_eq!(recorder.capacity(), 4);

    // What is left is the four newest, oldest first.
    let live: Vec<u16> = (0..recorder.len())
        .map(|index| recorder.stroke(index).expect("live").vk())
        .collect();

    assert_eq!(live, vec![VK_2, VK_Z, VK_B, VK_2]);
}

// -------------------------------------------------------------------------------------
// Point 13 — FR-10: `Space`, `Tab`, `Enter`, `Esc` flush the whole buffer
// -------------------------------------------------------------------------------------

#[test]
fn space_tab_enter_and_escape_flush_the_whole_buffer() {
    for vk in [VK_SPACE, VK_TAB, VK_RETURN, VK_ESCAPE] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);

        assert_eq!(
            press(&mut recorder, vk, SCAN_A),
            Recorded::Flushed,
            "vk {vk:#04x} ends the word and flushes"
        );
        assert_eq!(recorder.len(), 0, "vk {vk:#04x}");
        assert!(recorder.is_empty());
    }
}

// -------------------------------------------------------------------------------------
// Point 14 — FR-10: `Backspace` takes one element out and does not flush
// -------------------------------------------------------------------------------------

#[test]
fn backspace_takes_one_stroke_out_instead_of_flushing() {
    let mut recorder = fresh();

    press(&mut recorder, VK_A, SCAN_A);
    press(&mut recorder, VK_2, SCAN_2);
    press(&mut recorder, VK_A, SCAN_A);
    assert_eq!(typed(&recorder), "a2a");

    assert_eq!(press(&mut recorder, VK_BACK, 0x0E), Recorded::Popped);

    // One element, not the buffer: the two strokes before it are still there, in order.
    assert_eq!(recorder.len(), 2);
    assert_eq!(typed(&recorder), "a2");

    assert_eq!(press(&mut recorder, VK_BACK, 0x0E), Recorded::Popped);
    assert_eq!(press(&mut recorder, VK_BACK, 0x0E), Recorded::Popped);
    assert_eq!(recorder.len(), 0);

    // On an empty buffer it removes nothing and is not an error.
    assert_eq!(press(&mut recorder, VK_BACK, 0x0E), Recorded::Ignored);
    assert_eq!(recorder.len(), 0);
}

// -------------------------------------------------------------------------------------
// Point 15 — FR-10: editing and navigation keys flush the whole buffer
// -------------------------------------------------------------------------------------

#[test]
fn the_editing_and_navigation_keys_flush_the_whole_buffer() {
    let keys = [
        VK_DELETE, VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_INSERT,
    ];

    for vk in keys {
        let mut recorder = fresh();
        fill(&mut recorder, 2);

        assert_eq!(
            press(&mut recorder, vk, SCAN_A),
            Recorded::Flushed,
            "vk {vk:#04x} moves the caret where the buffer cannot follow"
        );
        assert_eq!(recorder.len(), 0, "vk {vk:#04x}");
    }

    // The extended forms of the same keys — the navigation block reports them with
    // `LLKHF_EXTENDED` — are the same keys and flush the same way.
    for vk in keys {
        let mut recorder = fresh();
        fill(&mut recorder, 2);

        assert_eq!(
            deliver(
                &mut recorder,
                vk,
                SCAN_A,
                LLKHF_EXTENDED.0,
                SOME_TIME,
                Edge::Down
            ),
            Recorded::Flushed,
            "extended vk {vk:#04x}"
        );
    }
}

// -------------------------------------------------------------------------------------
// Point 16 — FR-10: `Ctrl`/`Alt`/`Win` plus a key is a command and flushes
// -------------------------------------------------------------------------------------

#[test]
fn ctrl_alt_and_win_combinations_are_commands_and_flush() {
    for modifier in [VK_LCONTROL, VK_LMENU, VK_LWIN] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);

        // Holding the modifier is not yet a command: nothing is flushed and nothing is stored.
        assert_eq!(hold(&mut recorder, modifier), Recorded::Modifier);
        assert_eq!(recorder.len(), 3, "modifier {modifier:#04x} held alone");

        // The key that follows makes it one.
        assert_eq!(
            press(&mut recorder, VK_A, SCAN_A),
            Recorded::Flushed,
            "modifier {modifier:#04x} plus a key is a command"
        );
        assert_eq!(recorder.len(), 0, "modifier {modifier:#04x}");

        // And the command is not text either: it is not in the buffer afterwards.
        release(&mut recorder, modifier);
        assert_eq!(recorder.len(), 0);
    }
}

/// **Which of the three layout switchers reaches the command row of FR-10** — task T-03-3b,
/// and the measurement decision Р-40 rests on.
///
/// Р-40 proposes to re-read the keyboard layout whenever the FR-10 row "`Ctrl`/`Alt`/`Win` +
/// клавиша — полный сброс (команда, а не текст)" fires, on the stated ground that all three
/// layout switchers of Windows — `Alt+Shift`, `Ctrl+Shift` and `Win+Space` — pass through it.
/// Two of the three do not, and this is the measurement rather than the claim.
///
/// A switcher made of modifiers alone never reaches the row. [`Recorder::record`] recognises a
/// modifier key before any rule of FR-10 is consulted and answers [`Recorded::Modifier`],
/// because holding `Ctrl` is not yet a command and `Shift` is not text. Only `Win+Space` has a
/// non-modifier in it.
///
/// And the outcome does not identify the row even for `Win+Space`: `Space` is a boundary key
/// of FR-10 in its own right, so the plain `Space` at the end of this test answers
/// [`Recorded::Flushed`] with no modifier held at all. The two rows are one value from the
/// outside, which is what the last block records.
///
/// Nothing here is a defect. FR-11 says a layout change must not flush the buffer, and the two
/// switchers that leave it untouched are FR-11 working exactly as written. The test exists so
/// that the next reader of Р-40 has the fact measured instead of assumed.
#[test]
fn only_one_of_the_three_layout_switchers_reaches_the_command_row_of_fr10() {
    // `Alt+Shift`. Two modifiers and no key: the row is never reached and the buffer, which
    // FR-11 protects, survives the switch intact.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(hold(&mut recorder, VK_LMENU), Recorded::Modifier);
    assert_eq!(hold(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LMENU), Recorded::Modifier);
    assert_eq!(recorder.len(), 3, "Alt+Shift is modifiers and nothing else");

    // `Ctrl+Shift`. The same shape, the same answer.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(hold(&mut recorder, VK_LCONTROL), Recorded::Modifier);
    assert_eq!(hold(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LCONTROL), Recorded::Modifier);
    assert_eq!(
        recorder.len(),
        3,
        "Ctrl+Shift is modifiers and nothing else"
    );

    // `Win+Space`. `Space` is not a modifier, so this one is a command and the buffer goes.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed,
        "Win+Space is the one switcher of the three that is a command"
    );
    assert_eq!(recorder.len(), 0);
    release(&mut recorder, VK_LWIN);

    // And `Space` alone answers the same, from the boundary-key row instead. A caller that
    // watched the outcome could not tell a command from an ordinary word break.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed,
        "a bare Space is a flush too, and an indistinguishable one"
    );
}

#[test]
fn altgr_is_text_and_not_a_command() {
    // `AltGr` reaches the hook as `Ctrl` plus the *right* `Alt`, which is exactly the shape of
    // a command. Reading it as one would break every layout that puts characters on `AltGr`,
    // so the mask carries the combination as its own bit and FR-10 lets it through.
    let mut recorder = fresh();

    hold(&mut recorder, VK_LCONTROL);
    hold(&mut recorder, VK_RMENU);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);

    let stroke = recorder.stroke(0).expect("stored");

    assert!(stroke.mods().altgr());
    assert!(stroke.mods().ctrl(), "the fake `Ctrl` is recorded as held");
    assert!(stroke.mods().alt());
    // And it narrows to the `AltGr` combination of the cache, not to a bare one.
    assert_eq!(stroke.mods().to_layout_mods(), Mods::ALTGR);

    // The right `Alt` without `Ctrl` is a plain `Alt`, which is a command again.
    release(&mut recorder, VK_LCONTROL);
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Flushed);
}

#[test]
fn the_alt_flag_of_the_hook_is_believed_as_well_as_the_tracker() {
    // `LLKHF_ALTDOWN` is the system's own statement about this very event. It matters for an
    // `Alt` that went down before the hook was installed, which the tracker cannot have seen.
    let mut recorder = fresh();
    fill(&mut recorder, 2);

    assert_eq!(
        deliver(
            &mut recorder,
            VK_A,
            SCAN_A,
            LLKHF_ALTDOWN.0,
            SOME_TIME,
            Edge::Down
        ),
        Recorded::Flushed
    );
}

// -------------------------------------------------------------------------------------
// Point 17 — FR-10: any key after a conversion ends the session
// -------------------------------------------------------------------------------------

#[test]
fn the_first_key_after_a_conversion_ends_the_session() {
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    // The conversion itself leaves the strokes alone: FR-32 renders the *original* scan codes
    // again on the next press of the hotkey, so the buffer must survive its own conversion.
    recorder.note_conversion();
    assert!(recorder.in_conversion());
    assert_eq!(recorder.len(), 3);

    // The next key ends the session: the old strokes go, and the key that ended it starts the
    // new buffer.
    assert_eq!(press(&mut recorder, VK_2, SCAN_2), Recorded::Stored);
    assert!(!recorder.in_conversion());
    assert_eq!(recorder.len(), 1);
    assert_eq!(typed(&recorder), "2");

    // A modifier does not end it — it is not a key of the user's text, and `Shift` then a
    // letter must be one session end and not two.
    let mut recorder = fresh();
    fill(&mut recorder, 2);
    recorder.note_conversion();

    hold(&mut recorder, VK_LSHIFT);
    assert!(recorder.in_conversion());
    assert_eq!(recorder.len(), 2);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(recorder.len(), 1);
    assert_eq!(typed(&recorder), "A");
}

// -------------------------------------------------------------------------------------
// Point 18 — FR-11: a layout change flushes nothing
// -------------------------------------------------------------------------------------

#[test]
fn a_layout_change_does_not_flush_the_buffer() {
    let mut recorder = fresh();

    press(&mut recorder, VK_A, SCAN_A);
    press(&mut recorder, VK_A, SCAN_A);

    // The user presses `Alt+Shift` or `Win+Space`; the program is told which layout is active
    // now. FR-11: **nothing is flushed.** The strokes typed before the switch are still what
    // the user typed, and each of them remembers the layout it was typed under — which is the
    // whole reason `hkl` is a field of the stroke and not a field of the buffer.
    recorder.set_active_layout(RU);

    assert_eq!(
        recorder.len(),
        2,
        "FR-11 forbids flushing on a layout change"
    );

    press(&mut recorder, VK_A, SCAN_A);

    assert_eq!(recorder.len(), 3);
    assert_eq!(recorder.active_layout(), RU);

    assert_eq!(recorder.stroke(0).expect("live").hkl(), EN);
    assert_eq!(recorder.stroke(1).expect("live").hkl(), EN);
    assert_eq!(recorder.stroke(2).expect("live").hkl(), RU);

    // And the characters follow the layout that was active at the time, stroke by stroke.
    assert_eq!(typed(&recorder), "aaф");

    // The rebuild of FR-21 arrives on the same `WM_INPUTLANGCHANGE` and must not flush either.
    recorder.set_cache(cache());
    assert_eq!(recorder.len(), 3);
}

// -------------------------------------------------------------------------------------
// Points 19 and 20 — SEC-02: the memory is overwritten with zeroes
// -------------------------------------------------------------------------------------

/// How many slots of the backing array are not zero.
fn non_zero_slots(recorder: &Recorder) -> usize {
    recorder
        .slots()
        .iter()
        .filter(|slot| **slot != Stroke::ZEROED)
        .count()
}

#[test]
fn a_flush_overwrites_the_backing_array_with_zeroes() {
    let mut recorder = fresh_of(8);
    fill(&mut recorder, 5);

    assert_eq!(
        non_zero_slots(&recorder),
        5,
        "the strokes are really in the array before the flush"
    );

    recorder.reset();

    // SEC-02: "явно перезаписывается нулями... а не просто помечается пустым". The check is on
    // the backing array itself, not on the length, and it covers the slots outside the live
    // window, where a ring that merely moved its indices would have left the strokes.
    assert_eq!(recorder.len(), 0);
    assert_eq!(recorder.slots().len(), 8, "the array is still there");

    for (index, slot) in recorder.slots().iter().enumerate() {
        assert!(
            *slot == Stroke::ZEROED,
            "SEC-02: slot {index} still holds a stroke after the flush"
        );
    }

    // Every flush, not only the explicit one: the rules of FR-10 zero the same way.
    let mut recorder = fresh_of(8);
    fill(&mut recorder, 5);
    press(&mut recorder, VK_SPACE, SCAN_A);

    assert_eq!(non_zero_slots(&recorder), 0, "SEC-02 after a FR-10 flush");
}

#[test]
fn eviction_and_backspace_zero_the_slots_they_free() {
    // `Backspace` frees a slot and the slot is zeroed, not merely stepped over.
    let mut recorder = fresh_of(8);
    fill(&mut recorder, 5);

    for expected in (0..5).rev() {
        assert_eq!(press(&mut recorder, VK_BACK, 0x0E), Recorded::Popped);
        assert_eq!(
            non_zero_slots(&recorder),
            expected,
            "SEC-02: the slot `Backspace` freed still holds a stroke"
        );
    }

    // Eviction. The evicted stroke is identifiable by its virtual key, and after the eviction
    // it is nowhere in the array — the slot was zeroed before the new stroke was written into
    // it, and no copy of it survives anywhere else.
    let mut recorder = fresh_of(3);

    assert_eq!(press(&mut recorder, VK_Z, SCAN_Z), Recorded::Stored);
    fill_after(&mut recorder, 2);

    assert!(
        recorder.slots().iter().any(|slot| slot.vk() == VK_Z),
        "the stroke to be evicted is in the array to begin with"
    );

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Evicted);

    assert!(
        !recorder.slots().iter().any(|slot| slot.vk() == VK_Z),
        "SEC-02: the evicted stroke is still in the array"
    );
    assert_eq!(recorder.len(), 3);

    // And when everything is taken out again, the whole array is zero.
    while !recorder.is_empty() {
        press(&mut recorder, VK_BACK, 0x0E);
    }

    assert_eq!(non_zero_slots(&recorder), 0);
}

/// Adds `count` presses of `A` to a buffer that already holds something.
fn fill_after(recorder: &mut Recorder, count: usize) {
    for _ in 0..count {
        assert_eq!(press(recorder, VK_A, SCAN_A), Recorded::Stored);
    }
}

// -------------------------------------------------------------------------------------
// Point 21 — Р-22, NFR-03: `convert::Keystroke` is produced without allocating
// -------------------------------------------------------------------------------------

#[test]
fn the_buffer_produces_keystrokes_without_allocating() {
    // Everything that allocates happens before the counter is armed: the ring itself, which
    // FR-07 sizes once at creation, and the synthetic cache.
    let mut recorder = fresh_of(DEFAULT_CAPACITY);
    let mut out = [Keystroke::default(); DEFAULT_CAPACITY];
    let mut strokes = [Stroke::ZEROED; DEFAULT_CAPACITY];

    let (written, allocations) = allocations_during(|| {
        // A whole session: two hundred strokes through every branch that touches the ring,
        // then the two ways of reading it out, then a flush.
        for _ in 0..100 {
            press(&mut recorder, VK_A, SCAN_A);
            press(&mut recorder, VK_2, SCAN_2);
        }

        press(&mut recorder, VK_BACK, 0x0E);

        let written = recorder.keystrokes(&mut out).expect("the buffer fits");
        let same = recorder.strokes(&mut strokes).expect("the buffer fits");
        assert_eq!(written, same);

        recorder.reset();

        written
    });

    assert_eq!(
        allocations, 0,
        "NFR-03: the buffer path allocated {allocations} times"
    );
    assert_eq!(written, 199);

    // And what it produced is the conversion form of decision Р-22, in order.
    assert_eq!(out[0], strokes[0].keystroke());
    assert_eq!(out[0].scan(), SCAN_A);
    assert_eq!(out[0].layout(), EN);
    assert_eq!(out[1].scan(), SCAN_2);
    assert_eq!(
        String::from_utf16(convert_stroke(out[0], &russian()).units()).expect("valid text"),
        "ф"
    );

    // The output buffer belongs to the caller, and one that is too short is an error carrying
    // the length needed rather than a truncation.
    let mut recorder = fresh();
    fill(&mut recorder, 4);

    let mut too_short = [Keystroke::default(); 3];

    assert_eq!(
        recorder.keystrokes(&mut too_short),
        Err(ReadError::OutputTooSmall { needed: 4 })
    );
    assert_eq!(
        recorder.strokes(&mut [Stroke::ZEROED; 3]),
        Err(ReadError::OutputTooSmall { needed: 4 })
    );

    // `convert::max_units` sizes what comes after, at the capacity of FR-07.
    assert_eq!(max_units(DEFAULT_CAPACITY), DEFAULT_CAPACITY * MAX_UNITS);
}

// -------------------------------------------------------------------------------------
// Point 22 — FR-03: our own injected input never reaches the buffer
// -------------------------------------------------------------------------------------

/// The ordinary running state: armed, healthy, `Pause` as the hotkey.
fn armed() -> Mode {
    Mode {
        active: true,
        fail_safe: false,
        hotkey_vk: VK_PAUSE,
    }
}

/// Delivers one stroke the way the callback does: one whole `KeyEvent`, one `classify`.
fn through_the_hook(state: &mut HotkeyState, vk: u16, scan: u16, extra_info: usize) -> Decision {
    hook::classify(
        armed(),
        state,
        KeyEvent {
            vk,
            edge: Edge::Down,
            extra_info,
            scan,
            flags: 0,
            time: SOME_TIME,
        },
    )
    .decision
}

#[test]
fn our_own_injected_input_never_reaches_the_buffer() {
    // The thread-local buffer of section 6.3, on this test's own thread.
    buffer::install_recorder(fresh());
    assert!(buffer::is_installed());

    let mut state = HotkeyState::default();

    // The user types: the stroke is passed on to the application and recorded.
    assert_eq!(
        through_the_hook(&mut state, VK_A, SCAN_A, FOREIGN_SIGNATURE),
        Decision::Pass
    );
    assert_eq!(buffer::len(), 1);

    // Our own replacement comes back through the hook carrying the signature of FR-03. It is
    // passed on — it was sent *to* the application on purpose — and it must not be buffered:
    // a program that read its own output back would convert it again on the next press.
    assert_eq!(
        through_the_hook(&mut state, VK_A, SCAN_A, INJECTED_SIGNATURE),
        Decision::Pass
    );
    assert_eq!(buffer::len(), 1, "FR-03: our own input is not the user's");

    // A stroke injected by somebody else — the on-screen keyboard, a password manager — is
    // ordinary user input, because FR-03 filters on the signature and never on
    // `LLKHF_INJECTED`.
    assert_eq!(
        through_the_hook(&mut state, VK_A, SCAN_A, 0),
        Decision::Pass
    );
    assert_eq!(buffer::len(), 2);

    // The hotkey itself is not text and never enters the buffer.
    assert_eq!(
        through_the_hook(&mut state, VK_PAUSE, 0x45, FOREIGN_SIGNATURE),
        Decision::Suppress
    );
    assert_eq!(buffer::len(), 2);

    buffer::uninstall();
    assert!(!buffer::is_installed());
}

#[test]
fn a_suspended_or_failed_program_buffers_nothing() {
    buffer::install_recorder(fresh());

    let mut state = HotkeyState::default();

    // FR-90: suspended. FR-99: the fail-safe has been tripped. In both states `classify`
    // returns before the buffering point, and it must, because both mean "this program is not
    // taking part in what the user is typing".
    for mode in [
        Mode {
            active: false,
            ..armed()
        },
        Mode {
            fail_safe: true,
            ..armed()
        },
    ] {
        hook::classify(
            mode,
            &mut state,
            KeyEvent {
                vk: VK_A,
                edge: Edge::Down,
                extra_info: FOREIGN_SIGNATURE,
                scan: SCAN_A,
                flags: 0,
                time: SOME_TIME,
            },
        );
    }

    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
}

#[test]
fn a_thread_without_a_buffer_records_nothing_and_does_not_fail() {
    // Every thread of the program except the input one, and every test of `tests\hook.rs`:
    // `classify` calls the buffer, the buffer is not installed, and nothing happens.
    assert!(!buffer::is_installed());
    assert_eq!(buffer::len(), 0);
    assert!(buffer::is_empty());
    assert!(!buffer::reset());
    assert!(!buffer::note_conversion());
    assert!(!buffer::uninstall());

    assert_eq!(
        buffer::record(KeyEvent {
            vk: VK_A,
            edge: Edge::Down,
            extra_info: FOREIGN_SIGNATURE,
            scan: SCAN_A,
            flags: 0,
            time: SOME_TIME,
        }),
        Recorded::Ignored
    );
}

// -------------------------------------------------------------------------------------
// The public flush the other sources of FR-10 attach to
// -------------------------------------------------------------------------------------

#[test]
fn the_public_flush_is_reachable_from_the_thread_local_layer() {
    // The rows of the FR-10 table this task does not own — the mouse over Raw Input, the two
    // `WinEvent` subscriptions, the session change (tasks T-03-3 and T-06-2) and the "приостановка
    // пользователем" of the tray — all need one flush to call. This is it.
    buffer::install_recorder(fresh());

    buffer::record(KeyEvent {
        vk: VK_A,
        edge: Edge::Down,
        extra_info: FOREIGN_SIGNATURE,
        scan: SCAN_A,
        flags: 0,
        time: SOME_TIME,
    });

    assert_eq!(buffer::len(), 1);
    assert!(buffer::reset());
    assert_eq!(buffer::len(), 0);

    // And the zeroing is the same one, because it is the same code.
    let zeroed = buffer::with(|recorder| non_zero_slots(recorder)).expect("installed");
    assert_eq!(zeroed, 0);

    buffer::uninstall();
}

#[test]
fn a_released_buffer_is_zeroed_before_the_memory_goes_back() {
    // SEC-02 at the end of the buffer's life: `uninstall` drops the recorder, and the drop
    // zeroes the ring before the allocator can hand the block to anybody else. The check is
    // indirect by necessity — reading freed memory would be undefined behaviour — so what is
    // asserted here is that the flush the drop performs is the one the audit view sees.
    let mut recorder = fresh_of(4);
    fill(&mut recorder, 3);

    recorder.reset();
    assert_eq!(non_zero_slots(&recorder), 0);

    drop(recorder);
}

// -------------------------------------------------------------------------------------
// Task T-03-2a — the whole stroke in one value, and the capacity that arrives late
// -------------------------------------------------------------------------------------

#[test]
fn one_key_event_is_one_whole_stroke() {
    // The point of task T-03-2a. Until it, the physical half of a stroke travelled to the
    // buffer through a thread-local published by the callback a moment before the decision,
    // and "one publication, one stroke" was an invariant held up by the order of two calls
    // inside the most dangerous function of the program. Now it is one value: what `record`
    // stores can only have come from the `KeyEvent` it was handed.
    let mut recorder = fresh_of(4);

    let stored = recorder.record(KeyEvent {
        vk: VK_A,
        edge: Edge::Down,
        extra_info: FOREIGN_SIGNATURE,
        scan: SCAN_SLASH,
        flags: LLKHF_EXTENDED.0,
        time: SOME_TIME,
    });

    assert_eq!(stored, Recorded::Stored);

    let stroke = recorder.stroke(0).expect("one stroke");

    assert!(stroke.vk() == VK_A, "the virtual key of FR-04");
    assert!(stroke.scan() == SCAN_SLASH, "the scan code of FR-04");
    assert!(stroke.time() == SOME_TIME, "the timestamp FR-12 needs");
    assert!(stroke.mods().extended(), "the LLKHF_EXTENDED of FR-05");

    // And it is the extended reading of the shared scan code that was looked up, which is the
    // keypad key of task T-02-1a and not the `/?` key of the main block.
    assert_eq!(String::from_utf16(stroke.units()).expect("valid text"), "*");
}

#[test]
fn the_capacity_a_buffer_really_has_is_the_effective_one() {
    // `app` compares "what the configuration asks for" against "what the buffer has" on every
    // message of the input thread, and it has to compare like with like: the rule that clamps
    // the configured number is published so that the comparison can use it.
    for requested in [
        0,
        1,
        7,
        DEFAULT_CAPACITY,
        MAX_CAPACITY,
        MAX_CAPACITY + 1,
        usize::MAX,
    ] {
        assert_eq!(
            Recorder::with_capacity(requested).capacity(),
            buffer::effective_capacity(requested),
            "capacity {requested}"
        );
    }

    assert_eq!(buffer::effective_capacity(0), DEFAULT_CAPACITY);
    assert_eq!(buffer::effective_capacity(usize::MAX), MAX_CAPACITY);
}

#[test]
fn a_resize_keeps_the_cache_and_the_layout_and_leaves_no_strokes_behind() {
    // FR-07 with the configuration arriving after the buffer is already up — the case NFR-08
    // creates by forbidding the input thread to wait for a file. Re-installing the recorder
    // would answer it too, and would throw the cache of FR-20 away with it: a second sweep of
    // every layout in the session, to change one number.
    let mut recorder = fresh_of(8);
    fill(&mut recorder, 3);

    recorder.set_capacity(64);

    assert_eq!(recorder.capacity(), 64);
    assert!(recorder.has_cache(), "FR-20: the cache survives a resize");
    assert_eq!(recorder.active_layout(), EN, "FR-04: so does the layout");

    // The new ring is empty and every slot of it is zero — SEC-02 holds for the array that
    // replaced the old one exactly as it holds for a flush.
    assert_eq!(recorder.len(), 0);
    assert_eq!(non_zero_slots(&recorder), 0);

    // And the buffer works afterwards, decoding from the cache it kept.
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(typed(&recorder), "a");

    // Clamped exactly as construction clamps it.
    recorder.set_capacity(0);
    assert_eq!(recorder.capacity(), DEFAULT_CAPACITY);
    recorder.set_capacity(usize::MAX);
    assert_eq!(recorder.capacity(), MAX_CAPACITY);
}

// -------------------------------------------------------------------------------------
// Task T-03-3, points 14 to 18 — FR-12: the flush that carries a timestamp
// -------------------------------------------------------------------------------------
//
// The whole of FR-12 is decidable here, with no mouse, no window and no clock: the buffer
// stores the timestamp of every stroke, the flush takes the timestamp of the event, and what
// happens between them is arithmetic. The live half of the requirement — that the timestamps
// of the three Win32 sources really are the same counter — is argued in the report of task
// T-03-3 and shown on the running program; it is not something a test can assert.

/// Types one ordinary key stamped `time`.
fn press_at(recorder: &mut Recorder, time: u32) {
    assert_eq!(
        deliver(recorder, VK_A, SCAN_A, 0, time, Edge::Down),
        Recorded::Stored
    );
}

/// The timestamps of the live strokes, oldest first.
///
/// A timestamp is not a keystroke: it carries no key code, no scan code and no character, so
/// printing it in an assertion message is not the thing SEC-07 forbids.
fn times(recorder: &Recorder) -> Vec<u32> {
    (0..recorder.len())
        .map(|index| {
            recorder
                .stroke(index)
                .expect("index below the length")
                .time()
        })
        .collect()
}

#[test]
fn a_flush_removes_only_the_strokes_that_are_not_newer_than_the_event() {
    let mut recorder = fresh_of(8);

    for time in [100, 200, 300, 400, 500] {
        press_at(&mut recorder, time);
    }

    // FR-12: "при обработке события сброса с меткой `T` из буфера удаляются все нажатия с
    // `time <= T`". The click is stamped 300, so the three strokes made at or before it go and
    // the two made after it stay.
    assert_eq!(
        recorder.reset_up_to(300),
        ResetOutcome::Partial {
            removed: 3,
            kept: 2
        }
    );
    assert_eq!(times(&recorder), vec![400, 500]);
    assert_eq!(recorder.len(), 2);
}

#[test]
fn the_whole_buffer_goes_only_when_the_event_is_newer_than_every_stroke() {
    let mut recorder = fresh_of(8);

    for time in [100, 200, 300] {
        press_at(&mut recorder, time);
    }

    // One millisecond short of the newest stroke: FR-12's "полная очистка выполняется только
    // если метка события новее всех нажатий" is not satisfied, so this is not a full clear.
    assert_eq!(
        recorder.reset_up_to(299),
        ResetOutcome::Partial {
            removed: 2,
            kept: 1
        }
    );
    assert_eq!(times(&recorder), vec![300]);

    // The newest stroke's own timestamp: `time <= T` holds for it, nothing is left, and the
    // flush is the full one.
    assert_eq!(
        recorder.reset_up_to(300),
        ResetOutcome::Cleared { removed: 1 }
    );
    assert_eq!(recorder.len(), 0);

    // An empty buffer takes the same arm — "newer than all of them" is vacuously true of no
    // strokes — and the whole backing array is zeroed for it (SEC-02).
    assert_eq!(
        recorder.reset_up_to(0),
        ResetOutcome::Cleared { removed: 0 }
    );
    assert_eq!(non_zero_slots(&recorder), 0);
}

#[test]
fn a_click_processed_late_does_not_erase_what_was_typed_after_it() {
    let mut recorder = fresh_of(8);

    // The situation FR-12 was written for. The user clicked at millisecond 1000; the `WM_INPUT`
    // for it is still sitting in the input thread's queue. The keyboard hook, which is called
    // synchronously, has meanwhile delivered three keystrokes made *after* the click.
    for time in [1_001, 1_002, 1_003] {
        press_at(&mut recorder, time);
    }

    // The naive reading of the FR-10 row — "нажатие любой кнопки мыши: полный сброс" — would
    // throw all three away here, which is precisely the defect FR-12 exists to forbid.
    assert_eq!(recorder.reset_up_to(1_000), ResetOutcome::Kept { kept: 3 });
    assert_eq!(times(&recorder), vec![1_001, 1_002, 1_003]);
    assert_eq!(typed(&recorder), "aaa");
}

#[test]
fn a_partial_flush_zeroes_the_slots_it_frees() {
    let mut recorder = fresh_of(8);

    for time in [10, 20, 30, 40, 50] {
        press_at(&mut recorder, time);
    }

    assert_eq!(
        non_zero_slots(&recorder),
        5,
        "the strokes are really in the array before the flush"
    );

    assert_eq!(
        recorder.reset_up_to(30),
        ResetOutcome::Partial {
            removed: 3,
            kept: 2
        }
    );

    // SEC-02: "перезаписывается нулями, а не просто помечается пустым" is a rule about every
    // flush, and a partial one frees memory exactly as a full one does. Two strokes are live,
    // and there are exactly two slots in the whole array that are not zero — so the three the
    // removal freed hold zeroes and not the strokes that were in them.
    assert_eq!(recorder.len(), 2);
    assert_eq!(non_zero_slots(&recorder), 2);
    assert_eq!(times(&recorder), vec![40, 50]);

    // And the rest of the array goes when the last two do.
    assert_eq!(
        recorder.reset_up_to(50),
        ResetOutcome::Cleared { removed: 2 }
    );
    assert_eq!(non_zero_slots(&recorder), 0);
}

#[test]
fn the_wrap_of_the_millisecond_counter_is_not_a_special_case() {
    // The comparison on its own, at the four places it can go wrong.
    assert!(buffer::is_newer_than(2, 1));
    assert!(!buffer::is_newer_than(1, 2));
    assert!(!buffer::is_newer_than(7, 7), "equal is not newer");
    assert!(
        buffer::is_newer_than(100, u32::MAX - 100),
        "201 ms later, across the turn of the counter"
    );
    assert!(!buffer::is_newer_than(u32::MAX - 100, 100));

    // And the buffer straddling the wrap: three strokes in the last three milliseconds before
    // the counter turned over, two in the first two after it.
    let mut recorder = fresh_of(8);

    for time in [u32::MAX - 2, u32::MAX - 1, u32::MAX, 0, 1] {
        press_at(&mut recorder, time);
    }

    // A click stamped `u32::MAX` — the last millisecond before the turn. A plain `time <= T`
    // would have called all five strokes older than it and emptied the buffer, throwing away
    // two keystrokes the user made after the click.
    assert_eq!(
        recorder.reset_up_to(u32::MAX),
        ResetOutcome::Partial {
            removed: 3,
            kept: 2
        }
    );
    assert_eq!(times(&recorder), vec![0, 1]);
}

#[test]
fn the_conversion_session_ends_with_a_full_clearance_and_survives_a_partial_one() {
    let mut recorder = fresh_of(8);

    for time in [10, 20] {
        press_at(&mut recorder, time);
    }

    recorder.note_conversion();
    assert!(recorder.in_conversion());

    // One of the converted strokes is still in the buffer, so the session it belongs to is
    // still open and the next key still ends it (the last row of FR-10).
    assert_eq!(
        recorder.reset_up_to(10),
        ResetOutcome::Partial {
            removed: 1,
            kept: 1
        }
    );
    assert!(recorder.in_conversion());

    // Nothing of it is left: there is no session to end any more.
    assert_eq!(
        recorder.reset_up_to(20),
        ResetOutcome::Cleared { removed: 1 }
    );
    assert!(!recorder.in_conversion());
}

#[test]
fn the_thread_local_flush_reaches_the_buffer_of_the_calling_thread() {
    // Every thread but the input one owns no buffer (section 6.3), and the window procedure
    // runs on all three, so the flush has to be harmless there.
    assert_eq!(buffer::reset_up_to(1_000), None);

    let mut recorder = fresh_of(8);
    press_at(&mut recorder, 100);
    press_at(&mut recorder, 200);
    buffer::install_recorder(recorder);

    assert_eq!(
        buffer::reset_up_to(100),
        Some(ResetOutcome::Partial {
            removed: 1,
            kept: 1
        })
    );
    assert_eq!(buffer::len(), 1);

    assert_eq!(
        buffer::reset_up_to(200),
        Some(ResetOutcome::Cleared { removed: 1 })
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
    assert_eq!(buffer::reset_up_to(300), None);
}
