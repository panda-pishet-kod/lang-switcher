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
    self, DEFAULT_CAPACITY, MAX_CAPACITY, Physical, ReadError, Recorded, Recorder, ResetOutcome,
    Stroke, StrokeMods,
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
/// `R`, the other half of `Win+R` — the command of FR-10 that criterion 12 of task T-10-12
/// names beside the `Win+Space` of FR-11.
const VK_R: u16 = 0x52;
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
const VK_RWIN: u16 = 0x5C;
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
/// ⚠ **Updated by task T-03-3c, and only in the `Win+Space` block.** The fact this test is
/// named after has not moved — `Win+Space` is still the one switcher of the three that reaches
/// the row — but what the row does with it has: the user's decision on question 43 (commit
/// `e6ba407`) rewrote it as "`Ctrl`/`Alt`/`Win` + клавиша, **кроме комбинаций смены раскладки,
/// названных FR-11**", so the switch is no longer a command and no longer flushes. The two
/// modifier-only blocks and the bare-`Space` control block are untouched.
///
/// `Space` remains a boundary key of FR-10 in its own right, which is what the last block
/// records and why the exception in [`Recorder::record`] is written as narrowly as it is: with
/// no `Win` held, the very same key still answers [`Recorded::Flushed`].
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

    // `Win+Space`. `Space` is not a modifier, so this one — and only this one of the three —
    // does reach the row, which is what this test is named after and what it still measures.
    //
    // ⚠ What the row **does** with it changed, and this is the one assertion of this test that
    // task T-03-3c had to move. The row now reads "кроме комбинаций смены раскладки, названных
    // FR-11" (`SPEC.md` §4.2, commit e6ba407, the user's decision on question 43), so the
    // switch is no longer a command: nothing is flushed, and nothing is stored either.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Ignored,
        "Win+Space reaches the row and the row lets it past — FR-11"
    );
    assert_eq!(recorder.len(), 3, "FR-11: a layout switch keeps the buffer");
    release(&mut recorder, VK_LWIN);

    // And `Space` alone still flushes, from the boundary-key row instead — the row FR-11 says
    // nothing about, and the reason the exception above is written as narrowly as it is.
    let mut recorder = fresh();
    fill(&mut recorder, 3);

    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed,
        "a bare Space is a flush, exactly as it was"
    );
    assert_eq!(recorder.len(), 0);
}

// -------------------------------------------------------------------------------------
// FR-11 against FR-10 — task T-03-3c, the user's decision on question 43
// -------------------------------------------------------------------------------------

/// **`Win`+`Space` does not flush, and the strokes keep the layout they were typed under** —
/// FR-11, and the whole of part 1 of task T-03-3c.
///
/// The requirement in one sentence: "Смена раскладки самим пользователем (`Alt+Shift`,
/// `Win+Space`) **не сбрасывает** буфер: HKL хранится по каждому нажатию отдельно."
///
/// This is the sequence the user actually performs, in the order they perform it, and it is the
/// sequence that used to lose their text: type, notice the wrong layout, switch it with
/// `Win+Space`, carry on typing, and only then reach for the hotkey. What the hotkey has to
/// find at the end is **everything** — the part typed before the switch and the part typed
/// after — with each stroke still carrying the `hkl` that was active when that stroke was made,
/// because FR-26 converts by the recorded `hkl` and by nothing else.
///
/// `set_active_layout` between the two halves is what the probe of part 2 causes the input
/// thread to do (`app::refresh_layout_and_cache`), and it is called here directly for the same
/// reason the rest of this file calls the recorder directly: no window, no hook, no keyboard.
#[test]
fn a_layout_switch_with_win_and_space_keeps_everything_typed_before_it() {
    let mut recorder = fresh();

    // Two strokes in English, the layout that was active when they were typed.
    fill(&mut recorder, 2);

    // The switch itself. It reaches the command row of FR-10 — `Space` is not a modifier — and
    // the row lets it past, because FR-11 names this combination.
    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Ignored,
        "FR-11: Win+Space is a layout switch, not a command"
    );
    assert_eq!(recorder.len(), 2, "FR-11: the buffer survives the switch");
    assert_eq!(release(&mut recorder, VK_LWIN), Recorded::Modifier);

    // Nothing was stored either: the system consumes the switch and puts no space into the
    // text, so a space in the ring would be a character the user never typed.
    assert_eq!(recorder.len(), 2, "the switch is not text");

    // The layout the probe of part 2 discovers, published exactly as the input thread does it.
    recorder.set_active_layout(RU);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(recorder.len(), 3);

    // FR-11, the second half of the sentence, and FR-26: the `hkl` is per stroke, so the two
    // halves of the buffer keep their own and the same physical key reads as two characters.
    assert_eq!(recorder.stroke(0).expect("stored").hkl(), EN);
    assert_eq!(recorder.stroke(1).expect("stored").hkl(), EN);
    assert_eq!(recorder.stroke(2).expect("stored").hkl(), RU);
    assert_eq!(typed(&recorder), "aaф");
}

/// **The FR-11 exception is exactly `Win`+`Space` and not one combination wider** — FR-10.
///
/// Every row of the FR-10 table that was a flush before task T-03-3c is still a flush. The
/// exception is one combination, held to three conditions: the key is `Space`, `Win` is down,
/// and neither `Ctrl` nor `Alt` is — `Ctrl+Win+Space` and `Alt+Win+Space` switch no layout on
/// this system and stay commands.
///
/// The `Space` row of the table ("`Space`, `Tab`, `Enter`, `Esc` — полный сброс") is the one
/// this could most easily have damaged, and the first case below is it: with no modifier held
/// the very same key flushes exactly as it always did.
#[test]
fn everything_but_win_plus_space_flushes_exactly_as_it_did() {
    // `Space` with nothing held — the boundary-key row of FR-10, untouched.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed
    );
    assert_eq!(recorder.len(), 0, "a bare Space still flushes");

    // `Win` plus a key that is not `Space` — the command row, untouched.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    hold(&mut recorder, VK_LWIN);
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Flushed);
    assert_eq!(recorder.len(), 0, "Win+A is still a command");
    release(&mut recorder, VK_LWIN);

    // `Win` plus the other boundary keys: `Space` is excepted by name, and `Tab`, `Enter` and
    // `Esc` are not.
    for vk in [VK_TAB, VK_RETURN, VK_ESCAPE] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);
        hold(&mut recorder, VK_LWIN);
        assert_eq!(
            press(&mut recorder, vk, SCAN_A),
            Recorded::Flushed,
            "Win plus vk {vk:#04x} is still a command"
        );
        assert_eq!(recorder.len(), 0, "vk {vk:#04x}");
    }

    // `Space` with `Win` and a command modifier on top of it. Not the switch FR-11 names, and
    // therefore not excepted.
    for extra in [VK_LCONTROL, VK_LMENU] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);
        hold(&mut recorder, VK_LWIN);
        hold(&mut recorder, extra);
        assert_eq!(
            press(&mut recorder, VK_SPACE, SCAN_SPACE),
            Recorded::Flushed,
            "Win+{extra:#04x}+Space is not the switcher of FR-11"
        );
        assert_eq!(recorder.len(), 0, "extra modifier {extra:#04x}");
    }

    // And the right `Win` is the same key as the left one, which is how the tracker reads it.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    hold(&mut recorder, VK_RWIN);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Ignored,
        "the right Win switches the layout too"
    );
    assert_eq!(recorder.len(), 3);
}

// ---------------------------------------------------------------------------------------
// Defect D — the latch in `held`, task T-10-12
// ---------------------------------------------------------------------------------------

/// The system answering "nobody is holding anything" — the state of a keyboard while these
/// tests feed their synthetic strokes, stated instead of assumed.
fn nothing_is_physically_held() -> Physical {
    Physical {
        ctrl: false,
        alt_left: false,
        alt_right: false,
        win: false,
    }
}

/// The system answering "`Win` really is down" — a user holding it for `Win+Space` or `Win+R`.
fn win_is_physically_held() -> Physical {
    Physical {
        win: true,
        ..nothing_is_physically_held()
    }
}

/// The system agreeing with everything the stream said — every command modifier really down.
fn everything_is_physically_held() -> Physical {
    Physical {
        ctrl: true,
        alt_left: true,
        alt_right: true,
        win: true,
    }
}

/// **The detector of defect D** — criterion 11 of task T-10-12.
///
/// # What broke, and how it was found
///
/// `Win+E`, a click in the search box of the Explorer window it opened, and six letters typed
/// there: from that moment the product recorded nothing at all, in **every** application, until
/// the process was restarted. The channel showed the callback alive and counting, `buffer_len`
/// stuck at zero, and every flush counter frozen — nobody was clearing the ring, so nothing was
/// ever entering it. A human pressed and released `Win` once, six and a half minutes later, and
/// the product started working again in the same second.
///
/// The mechanism is the whole of it: `Held::win` is raised by a press the hook saw and lowered
/// only by a release the hook sees. Task T-10-12 measured that the release of that `Win+E` never
/// reached the callback — and that it was **not** lost in the hook-reinstallation gap, which is
/// two microseconds wide once every thirty seconds. One release missed, and the command row of
/// FR-10 then throws away every keystroke for the life of the process.
///
/// # Why the test is written on the release and not on the trigger
///
/// The trigger is a shell window, and the bench may not touch one — position 10 of §11.3 is
/// marked **П**. It does not need to: what the shell did was *withhold one release*, and that is
/// exactly what this test feeds. No timing, no live Explorer, no race.
///
/// # Both sides
///
/// On the code before the repair this test **fails**: the letter comes back `Flushed` and the
/// buffer stays empty, for ever. After it, the letter is stored. Both runs are in the report.
#[test]
fn a_win_release_that_never_arrived_does_not_silence_the_buffer_for_ever() {
    let mut recorder = fresh();
    recorder.verify_held_with(nothing_is_physically_held);

    // The user types a word. Ordinary, and it works.
    fill(&mut recorder, 3);

    // `Win` goes down and the hook sees it...
    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);

    // ...and the release never arrives. Nothing else happens: no reinstallation, no suspension,
    // no panic. The hook is up and the callback is being called, which is what the channel of
    // the broken run showed.

    // Before the repair this is `Flushed` and the buffer is empty. The keystroke is the user's
    // own text and it must be recorded.
    assert_eq!(
        press(&mut recorder, VK_A, SCAN_A),
        Recorded::Stored,
        "defect D: a `Win` release that never arrived must not turn text into a command"
    );

    // And it must not be a single lucky stroke, either: the latch was permanent, so the check
    // is that the buffer keeps working for the rest of the session.
    for _ in 0..8 {
        assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    }

    assert_eq!(
        recorder.len(),
        3 + 1 + 8,
        "the word that was already there survives, and everything typed after it is recorded"
    );

    // The belief itself has been put right, not merely stepped around once per keystroke: with
    // `held.win` genuinely down, `Space` is the boundary key of the first row of FR-10 again,
    // and not the layout switch of FR-11 that a raised `Win` would have made of it.
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed,
        "the latch is down, not merely bypassed"
    );
}

/// The same latch on `Ctrl` and on `Alt` — the other two bits of the command row of FR-10.
///
/// `Win` is the one defect D was caught on, because `Win+E` is the combination the shell owns.
/// The row reads `Ctrl`, `Alt` and `Win` alike, so a lost release of any of them latches the
/// same way, and the repair is checked on all three rather than on the one that was reported.
#[test]
fn a_lost_release_of_any_command_modifier_is_recovered_from() {
    for modifier in [VK_LCONTROL, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN] {
        let mut recorder = fresh();
        recorder.verify_held_with(nothing_is_physically_held);

        assert_eq!(hold(&mut recorder, modifier), Recorded::Modifier);

        assert_eq!(
            press(&mut recorder, VK_A, SCAN_A),
            Recorded::Stored,
            "a lost release of {modifier:#04x} must not latch the command row"
        );
        assert_eq!(recorder.len(), 1, "modifier {modifier:#04x}");
    }
}

/// **Criterion 12 of task T-10-12: FR-10 and FR-11 are exactly what they were.**
///
/// The repair asks the system only on the row that is about to throw the stroke away, and it
/// only ever *lowers* a belief the system contradicts. When the user really is holding the key
/// — which is what these probes state — every row of the FR-10 table and the exception of FR-11
/// answer precisely as they did before the repair existed.
///
/// This is the behaviour the user decided on question 43 (commit `e6ba407`) and decision Р-44
/// gave to task T-10-6. Breaking it while repairing the latch would have been the worst
/// available outcome of task T-10-12, so it is asserted here against the repair itself and not
/// only in the tests that predate it.
#[test]
fn the_physical_check_leaves_fr10_and_fr11_alone_when_the_key_really_is_held() {
    // FR-11: `Win+Space` switches the layout and **does not** flush the buffer.
    let mut recorder = fresh();
    recorder.verify_held_with(win_is_physically_held);
    fill(&mut recorder, 3);
    hold(&mut recorder, VK_LWIN);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Ignored,
        "FR-11: Win+Space must still not flush the buffer"
    );
    assert_eq!(recorder.len(), 3, "FR-11: the strokes are still there");

    // FR-10: `Win+R` is a command and **does** flush.
    let mut recorder = fresh();
    recorder.verify_held_with(win_is_physically_held);
    fill(&mut recorder, 3);
    hold(&mut recorder, VK_LWIN);
    assert_eq!(
        press(&mut recorder, VK_R, SCAN_A),
        Recorded::Flushed,
        "FR-10: Win+R is a command and must still flush"
    );
    assert_eq!(recorder.len(), 0);

    // FR-10: `Ctrl` and `Alt` combinations are still commands when the keys really are down.
    for modifier in [VK_LCONTROL, VK_LMENU] {
        let mut recorder = fresh();
        recorder.verify_held_with(everything_is_physically_held);
        fill(&mut recorder, 3);
        hold(&mut recorder, modifier);
        assert_eq!(
            press(&mut recorder, VK_A, SCAN_A),
            Recorded::Flushed,
            "FR-10: {modifier:#04x}+A is still a command"
        );
        assert_eq!(recorder.len(), 0, "modifier {modifier:#04x}");
    }

    // `AltGr` is still text and not a command, which is the exception the mask itself carries.
    let mut recorder = fresh();
    recorder.verify_held_with(everything_is_physically_held);
    hold(&mut recorder, VK_LCONTROL);
    hold(&mut recorder, VK_RMENU);
    assert_eq!(
        press(&mut recorder, VK_A, SCAN_A),
        Recorded::Stored,
        "AltGr must still produce text"
    );
    assert!(recorder.stroke(0).expect("stored").mods().altgr());
}

/// A bare `Space` after a latched `Win` flushes, which is the row of FR-10 it belongs to.
///
/// The interesting half of the repair: the exception of FR-11 is read off `held.win` as well, so
/// a latched `Win` did not merely swallow letters — it also turned every `Space` into a silent
/// no-op instead of the boundary key of the first row of FR-10. Putting the belief right fixes
/// both, and this pins the second one so that a later change cannot quietly re-introduce it.
#[test]
fn a_bare_space_after_a_lost_win_release_is_the_boundary_key_it_should_be() {
    let mut recorder = fresh();
    recorder.verify_held_with(nothing_is_physically_held);

    fill(&mut recorder, 3);
    hold(&mut recorder, VK_LWIN);
    // The release is lost here.

    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed,
        "with no `Win` actually held, `Space` is the boundary key of FR-10 and not the switch of FR-11"
    );
    assert_eq!(recorder.len(), 0);
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

/// **SEC-04a: a resize publishes the position counter, and the pair of mirrors stays possible.**
///
/// The third of the three places `Recorder::cycle` is written — task T-04-3-3. `set_capacity`
/// zeroes the counter and builds a new ring whose length goes out through `Ring::set_len`, so
/// the length mirror reads zero the moment the resize is done. Before this task the position
/// mirror was left alone, and a snapshot of SEC-04a could therefore carry `buffer_len = 0`
/// beside a non-zero `cycle_position` — a pair the program itself can never be in, and the pair
/// the bench of §11.5 reads for positions 16 and 17.
///
/// Under the `testing` feature because the mirror exists only there; the counter's own behaviour
/// is asserted without it, a few tests above.
#[cfg(feature = "testing")]
#[test]
fn a_resize_publishes_the_position_counter_next_to_the_length() {
    use lang_switcher::control;

    let mut recorder = fresh_of(8);
    fill(&mut recorder, 3);
    counter_at_three(&mut recorder);

    // `advance_cycle` published the non-zero value, which is what makes the next assertion a
    // check on `set_capacity` rather than on an atomic that happened to be zero already.
    assert_eq!(
        control::snapshot().cycle_position,
        3,
        "advance_cycle publishes, and the mirror is off zero before the resize"
    );

    recorder.set_capacity(64);

    let published = control::snapshot();
    assert_eq!(
        published.cycle_position, 0,
        "SEC-04a: the resize publishes the zero it stored"
    );
    assert_eq!(
        published.cycle_position,
        recorder.cycle_position(),
        "the mirror agrees with the counter it mirrors"
    );
    assert_eq!(
        published.buffer_len, 0,
        "and the length mirror is the zero the new ring published through set_len"
    );
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

// -------------------------------------------------------------------------------------
// The position counter of FR-32 — task T-05-2
// -------------------------------------------------------------------------------------

/// Puts the counter somewhere other than zero, so that a flush that fails to reset it is
/// visible.
///
/// Three steps along a cycle of four, which leaves it at `3` — a value no rule below could
/// arrive at by accident.
fn counter_at_three(recorder: &mut Recorder) {
    for _ in 0..3 {
        recorder.advance_cycle(4);
    }

    assert_eq!(recorder.cycle_position(), 3, "the counter is off zero");
}

#[test]
fn the_position_counter_walks_the_cycle_and_stays_inside_it() {
    let mut recorder = fresh();

    // FR-32: the counter starts where "what is on the screen is what was typed" is.
    assert_eq!(recorder.cycle_position(), 0);

    // A cycle of three: one press per step, and the third brings it back to the start —
    // FR-31, and it is the same counter that makes FR-33 work at a length of two.
    assert_eq!(recorder.advance_cycle(3), 1);
    assert_eq!(recorder.advance_cycle(3), 2);
    assert_eq!(recorder.advance_cycle(3), 0);
    assert_eq!(recorder.advance_cycle(3), 1);

    // It is a *position*, reduced by the length every time, so there is no value it can grow
    // into: a thousand presses leave it inside `0..len` like the first one.
    for _ in 0..1_000 {
        recorder.advance_cycle(2);
        assert!(recorder.cycle_position() < 2);
    }

    // No cycle at all is no position at all, rather than a division by zero.
    assert_eq!(recorder.advance_cycle(0), 0);
    assert_eq!(recorder.cycle_position(), 0);
}

// -------------------------------------------------------------------------------------
// Point 16 — FR-34: every rule of FR-10 zeroes the counter with the buffer
// -------------------------------------------------------------------------------------

/// Every row of the FR-10 table that says "полный сброс", one by one, with the counter off
/// zero before it and at zero after it.
///
/// The rows are the table of §4.2 in the order it prints them. Four of them arrive through
/// [`Recorder::record`] — the boundary keys, the editing keys, the command combinations and the
/// last row, any key after a conversion. The other five are asynchronous and all five arrive
/// through [`Recorder::reset`] or [`Recorder::reset_up_to`], which is where module `buffer`
/// documents them: the mouse click of FR-13 over Raw Input, `EVENT_SYSTEM_FOREGROUND`,
/// `EVENT_OBJECT_FOCUS`, `WM_WTSSESSION_CHANGE` and the user's own pause from the tray.
///
/// `Backspace` is in the table and is **not** a flush — it takes one element out — so the
/// counter stays where it is, and that is asserted too.
#[test]
fn every_flush_rule_of_fr10_zeroes_the_position_counter() {
    // Row 1 of the table: `Space`, `Tab`, `Enter`, `Esc`.
    for vk in [VK_SPACE, VK_TAB, VK_RETURN, VK_ESCAPE] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);
        counter_at_three(&mut recorder);

        assert_eq!(press(&mut recorder, vk, SCAN_A), Recorded::Flushed);
        assert_eq!(recorder.len(), 0);
        assert_eq!(
            recorder.cycle_position(),
            0,
            "FR-34: the counter goes with the buffer"
        );
    }

    // Row 3: `Delete`, the arrows, `Home`/`End`/`PgUp`/`PgDn`, `Insert`.
    for vk in [
        VK_DELETE, VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_INSERT,
    ] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);
        counter_at_three(&mut recorder);

        assert_eq!(press(&mut recorder, vk, SCAN_A), Recorded::Flushed);
        assert_eq!(recorder.len(), 0);
        assert_eq!(recorder.cycle_position(), 0);
    }

    // Row 4: `Ctrl`/`Alt`/`Win` + key — a command, not text.
    for modifier in [VK_LCONTROL, VK_LMENU, VK_LWIN] {
        let mut recorder = fresh();
        fill(&mut recorder, 3);
        counter_at_three(&mut recorder);

        hold(&mut recorder, modifier);
        assert_eq!(press(&mut recorder, VK_B, SCAN_A), Recorded::Flushed);
        assert_eq!(recorder.len(), 0);
        assert_eq!(recorder.cycle_position(), 0);
    }

    // The last row: any key after a conversion.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    recorder.note_conversion();
    counter_at_three(&mut recorder);

    assert_eq!(press(&mut recorder, VK_2, SCAN_2), Recorded::Stored);
    assert_eq!(
        recorder.len(),
        1,
        "the key that ended the session started a new buffer"
    );
    assert_eq!(recorder.cycle_position(), 0);

    // Rows 5 to 9 — the mouse click of FR-13, the two `WinEvent` subscriptions, the session
    // change and the tray's pause. All five are `reset`, which is the entry point module
    // `buffer` gives them, and one of them is enough to show the reset because there is one
    // function underneath.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    counter_at_three(&mut recorder);

    recorder.reset();
    assert_eq!(recorder.len(), 0);
    assert_eq!(recorder.cycle_position(), 0);

    // The same five when they carry a timestamp and FR-12 makes the clearance a full one.
    let mut recorder = fresh();
    press_at(&mut recorder, 100);
    press_at(&mut recorder, 200);
    counter_at_three(&mut recorder);

    assert_eq!(
        recorder.reset_up_to(300),
        ResetOutcome::Cleared { removed: 2 }
    );
    assert_eq!(recorder.cycle_position(), 0);

    // And through the thread-local entry point the asynchronous sources really call.
    let mut recorder = fresh();
    fill(&mut recorder, 2);
    counter_at_three(&mut recorder);
    buffer::install_recorder(recorder);

    assert!(buffer::reset());
    assert_eq!(
        buffer::with(|recorder| recorder.cycle_position()),
        Some(0),
        "FR-34 through the public flush of the FR-10 table"
    );

    buffer::uninstall();

    // `Backspace` is the one row of the table that is not a flush: it takes the last stroke
    // out and leaves the rest, so the position the screen is at does not change either.
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    counter_at_three(&mut recorder);

    assert_eq!(press(&mut recorder, VK_BACK, SCAN_A), Recorded::Popped);
    assert_eq!(recorder.len(), 2);
    assert_eq!(
        recorder.cycle_position(),
        3,
        "Backspace is not a flush, so FR-34 has nothing to say about it"
    );
}

// -------------------------------------------------------------------------------------
// Point 17 — FR-11: a layout change flushes neither the buffer nor the counter
// -------------------------------------------------------------------------------------

#[test]
fn a_layout_change_flushes_neither_the_buffer_nor_the_counter() {
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    counter_at_three(&mut recorder);

    // The user presses `Alt+Shift` or `Win+Space` and the program is told which layout is
    // active now. FR-11: nothing is flushed — and FR-34 is not triggered either, because what
    // FR-34 follows is the buffer and not the layout.
    recorder.set_active_layout(RU);

    assert_eq!(
        recorder.len(),
        3,
        "FR-11 forbids flushing on a layout change"
    );
    assert_eq!(recorder.cycle_position(), 3, "and the counter goes with it");

    // The rebuild of FR-21 arrives on the same `WM_INPUTLANGCHANGE` and must not flush either.
    recorder.set_cache(cache());

    assert_eq!(recorder.len(), 3);
    assert_eq!(recorder.cycle_position(), 3);

    // `Alt+Shift` itself: modifiers alone, answered as modifiers, and nothing is touched.
    hold(&mut recorder, VK_LMENU);
    hold(&mut recorder, VK_LSHIFT);
    release(&mut recorder, VK_LSHIFT);
    release(&mut recorder, VK_LMENU);

    assert_eq!(recorder.len(), 3);
    assert_eq!(recorder.cycle_position(), 3);
}

/// **Decision Р-44.** `Win+Space` immediately after a conversion keeps the buffer, the counter
/// and the conversion session.
///
/// The exception of FR-11 used to sit inside the "Ctrl/Alt/Win + клавиша" row of the FR-10
/// table, where it answered that row alone. `Win+Space` also falls under the **last** row —
/// "любая клавиша после конвертации" — and that row cleared the ring before the exception was
/// consulted, so the user who converted a word, corrected the layout by hand and reached for the
/// hotkey again found nothing left to roll back. FR-11 is unconditional; Р-44 gives the repair
/// to this task, and this is the test that fixes it in place.
#[test]
fn win_space_right_after_a_conversion_keeps_the_buffer_and_the_counter() {
    let mut recorder = fresh();
    fill(&mut recorder, 3);
    recorder.note_conversion();
    counter_at_three(&mut recorder);

    // `Win` goes down, `Space` follows. `Space` is not a modifier, so it reaches the rules —
    // and FR-11 is answered before any of them.
    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(
        deliver(
            &mut recorder,
            VK_SPACE,
            SCAN_SPACE,
            0,
            SOME_TIME,
            Edge::Down
        ),
        Recorded::Ignored,
        "Р-44: FR-11 covers the last row of the FR-10 table as well"
    );
    assert_eq!(release(&mut recorder, VK_LWIN), Recorded::Modifier);

    assert_eq!(recorder.len(), 3, "the strokes survive the switch — FR-11");
    assert_eq!(recorder.cycle_position(), 3, "and so does the counter");
    assert!(
        recorder.in_conversion(),
        "the session is still the one the hotkey opened, so the next press rolls it back"
    );
    assert_eq!(typed(&recorder), "aaa", "and nothing was recorded either");

    // The switch is where the *layout* changes, and the buffer is told about it the usual way.
    recorder.set_active_layout(RU);

    assert_eq!(recorder.len(), 3);
    assert_eq!(recorder.cycle_position(), 3);
    assert!(recorder.in_conversion());

    // What still ends the session is an ordinary key, exactly as the last row says.
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert!(!recorder.in_conversion());
    assert_eq!(recorder.len(), 1);
    assert_eq!(recorder.cycle_position(), 0);

    // The neighbouring combinations are untouched: they switch no layout and stay commands.
    for modifier in [VK_LCONTROL, VK_LMENU] {
        let mut recorder = fresh();
        fill(&mut recorder, 2);
        counter_at_three(&mut recorder);

        hold(&mut recorder, VK_LWIN);
        hold(&mut recorder, modifier);

        assert_eq!(
            deliver(
                &mut recorder,
                VK_SPACE,
                SCAN_SPACE,
                0,
                SOME_TIME,
                Edge::Down
            ),
            Recorded::Flushed
        );
        assert_eq!(recorder.len(), 0);
        assert_eq!(recorder.cycle_position(), 0);
    }
}

// -------------------------------------------------------------------------------------
// Point 20 — SEC-02: what the reset frees is zeroed, counter included
// -------------------------------------------------------------------------------------

#[test]
fn the_flush_that_zeroes_the_counter_zeroes_every_slot_with_it() {
    let mut recorder = fresh_of(8);
    fill(&mut recorder, 5);
    counter_at_three(&mut recorder);

    assert_eq!(non_zero_slots(&recorder), 5);

    recorder.reset();

    // SEC-02 for the ring: the whole backing array, live window and free slots alike.
    assert_eq!(non_zero_slots(&recorder), 0);
    for slot in recorder.slots() {
        assert!(*slot == Stroke::ZEROED, "SEC-02: the slot is zeroed");
    }

    // And the counter, which is the other thing this task freed: zeroing a `usize` *is* its
    // overwrite, and it happens in the same function, `Recorder::clear_ring`.
    assert_eq!(recorder.cycle_position(), 0);

    // The same through a key of the FR-10 table rather than through the public flush.
    fill(&mut recorder, 4);
    counter_at_three(&mut recorder);

    assert_eq!(press(&mut recorder, VK_ESCAPE, SCAN_A), Recorded::Flushed);

    assert_eq!(non_zero_slots(&recorder), 0);
    assert_eq!(recorder.cycle_position(), 0);
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

// -------------------------------------------------------------------------------------
// ⭐ Defect E — the layout is read where it is used. Task T-10-14
// -------------------------------------------------------------------------------------

/// A machine whose foreground window is really running Russian, whatever the stamp believes.
///
/// The product's probe is `switch::current` — FR-52; a test states the premise instead of
/// depending on what the keyboard of the machine running it happens to be doing. Exactly the
/// arrangement `Physical`/`PhysicalProbe` already gives the repair of defect D, and the reason
/// the detector below is two-sided without a live keyboard.
fn really_russian() -> LayoutId {
    RU
}

/// No foreground window at all — the desktop is switching, or this is the secure desktop.
fn no_foreground() -> LayoutId {
    LayoutId::default()
}

/// A layout the cache of FR-20 does not hold.
fn really_unknown() -> LayoutId {
    UNKNOWN
}

/// How many times [`counting_russian`] was asked. Read by one test and by no other.
static PROBE_CALLS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// [`really_russian`], counting the calls.
fn counting_russian() -> LayoutId {
    PROBE_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    RU
}

/// ⭐ **The detector of defect E** — a word typed while the stamp and the real layout disagree.
///
/// # The premise, stated rather than hoped for
///
/// The stamp says English; the machine is really on Russian. That is not a contrived state: task
/// T-10-14 produced it on the running product six times out of six with a synthetic `Alt+Shift`
/// into the window that already had the focus. The probe of T-03-3c fired on the modifier
/// release — `layout_probes` rose every time — and read `en-US`, the layout on its way out,
/// because the system applies the switch after the callback has returned. Nothing corrected it
/// afterwards: `focus_changes` never moved, and no other occasion arrives while somebody is
/// typing a word.
///
/// # ⚠ Why both assertions, and why the first one is the one that mattered
///
/// The stale stamp costs **two** things and only the second is obvious. FR-26 takes the
/// direction of the conversion from `Stroke::hkl`, which is the famous half. The other half is
/// that [`Recorder::lookup`] decodes the scan code through *the map of the stamped layout*, so a
/// stale stamp also makes the buffer believe the user typed something they did not — and that is
/// what makes the defect invisible on screen: the six keys of `ghbdtn` under `ru-RU` put `привет`
/// in the field, the buffer stamped `en-US` believes it holds `ghbdtn`, converts `en-US → ru-RU`
/// and produces `привет` — the text that is already there. The user presses the hotkey and
/// nothing happens.
///
/// Before the repair this test read `"a"` and `EN`; after it, `"ф"` and `RU`. Both runs are in
/// the report of task T-10-14.
#[test]
fn a_word_is_recorded_under_the_layout_really_active_and_not_under_a_stale_stamp() {
    let mut recorder = fresh();
    recorder.stamp_layout_with(really_russian);

    // The premise: the stamp is the value an event left behind, and it is wrong.
    assert_eq!(
        recorder.active_layout(),
        EN,
        "the premise of this test is a stamp that says English"
    );

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);

    // FR-06: decoded through the map of the layout the user is really typing in.
    assert_eq!(
        typed(&recorder),
        "ф",
        "FR-06: the stroke is decoded through the layout that was really active"
    );
    // FR-26: the direction of the conversion is taken from this value.
    assert_eq!(
        recorder.stroke(0).expect("stored").hkl(),
        RU,
        "FR-26: the stroke carries the layout that was really active"
    );
    // And the stamp itself has caught up, so the rest of the word agrees with the first letter.
    assert_eq!(recorder.active_layout(), RU);
}

/// The read happens **on the first stroke of a word and on no other stroke** — NFR-01.
///
/// The whole argument that three Win32 calls are affordable inside the hook callback rests on
/// this: a word costs them once, not once per letter. Asserted by counting rather than described,
/// so that a later edit which moves the call out of its branch fails here instead of quietly
/// putting a system call on every keystroke in the machine.
///
/// The second half is the boundary key: `Space` empties the ring (FR-10), so the next letter is
/// a first stroke again and asks again. That is the property that makes the repair cover the
/// household case at all — a person switches the layout between words.
#[test]
fn the_layout_is_read_once_per_word_and_not_once_per_stroke() {
    use core::sync::atomic::Ordering;

    PROBE_CALLS.store(0, Ordering::Relaxed);

    let mut recorder = fresh();
    recorder.stamp_layout_with(counting_russian);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(
        PROBE_CALLS.load(Ordering::Relaxed),
        1,
        "the first stroke of the word asks"
    );

    for _ in 0..5 {
        assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    }
    assert_eq!(
        PROBE_CALLS.load(Ordering::Relaxed),
        1,
        "NFR-01: the rest of the word does not ask again"
    );

    // FR-10, the boundary-key row: the ring is emptied, so the next letter starts a word.
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Flushed
    );
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(
        PROBE_CALLS.load(Ordering::Relaxed),
        2,
        "the first stroke of the next word asks again"
    );
}

/// The three answers the repair deliberately ignores — and it ignores them by leaving the stamp
/// exactly where it was.
///
/// * **No probe at all.** Every other test in this file is that case, and it is why none of them
///   had to be touched: a recorder that was not told what the keyboard is doing behaves exactly
///   as this module behaved before task T-10-14.
/// * **A zero answer.** No foreground window — the desktop is switching, or this is the secure
///   desktop. `app::layout_refresh_needed` already refuses to treat that as a change, and so does
///   this: publishing it would tell the buffer to record under a layout no cache contains.
/// * **⚠ A layout the cache of FR-20 does not hold.** Adopting it would resolve `active_index` to
///   `None` and turn a working decode into an empty one for the whole word, in the callback,
///   where the rebuild that would fix it is fifty times the budget of NFR-01. The stamp is left
///   alone, the decode keeps working, and a layout genuinely new to the session stays the
///   business of FR-21's rebuild on the message path.
#[test]
fn a_probe_that_answers_nothing_useful_leaves_the_stamp_exactly_where_it_was() {
    // No probe: the behaviour of this module before task T-10-14.
    let mut recorder = fresh();
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(recorder.active_layout(), EN);
    assert_eq!(typed(&recorder), "a");

    // No foreground window.
    let mut recorder = fresh();
    recorder.stamp_layout_with(no_foreground);
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(recorder.active_layout(), EN);
    assert_eq!(typed(&recorder), "a");

    // A layout the cache does not hold: the decode keeps working rather than going empty.
    let mut recorder = fresh();
    recorder.stamp_layout_with(really_unknown);
    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);
    assert_eq!(recorder.active_layout(), EN);
    assert_eq!(
        typed(&recorder),
        "a",
        "the stroke still decodes: the repair never turns a working lookup into an empty one"
    );
}

/// ⛔ **FR-10 and FR-11 are not what this repair touches** — decision Р-44, the user's decision
/// on question 43, asserted against a recorder that *does* have the probe.
///
/// Every existing test of those two rules runs without a probe and therefore could not have
/// noticed if the repair had damaged them. This one gives the recorder a probe that would change
/// the stamp on any stroke that reached the read, and then walks the three rows the decisions are
/// about:
///
/// * `Alt+Shift` — modifiers alone; the first statement of `record` answers `Modifier` for all
///   four events and the buffer survives, exactly as task T-03-3b measured;
/// * `Win+Space` — reaches `record` proper and is `Ignored` by the FR-11 exception, the buffer
///   survives, nothing is stored and no layout is read, because the ring is not empty;
/// * `Win+R` — a command, and it still flushes.
///
/// The strokes typed before the switches keep the layout they were typed under, which is the
/// per-stroke `hkl` FR-11 rests on and which this repair does not go near.
#[test]
fn the_layout_read_leaves_fr10_and_fr11_exactly_as_they_were() {
    let mut recorder = fresh();
    recorder.stamp_layout_with(really_russian);

    // Two strokes. The first reads the layout, so both are Russian — that is the repair working,
    // and it is the state the rules below have to survive.
    fill(&mut recorder, 2);
    assert_eq!(recorder.active_layout(), RU);
    assert_eq!(typed(&recorder), "фф");

    // FR-11, first combination: `Alt+Shift` is modifiers alone and never reaches the rows below.
    assert_eq!(hold(&mut recorder, VK_LMENU), Recorded::Modifier);
    assert_eq!(hold(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LSHIFT), Recorded::Modifier);
    assert_eq!(release(&mut recorder, VK_LMENU), Recorded::Modifier);
    assert_eq!(recorder.len(), 2, "FR-11: Alt+Shift does not flush");

    // FR-11, second combination: `Win+Space` reaches `record` and is the exception Р-44 protects.
    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(
        press(&mut recorder, VK_SPACE, SCAN_SPACE),
        Recorded::Ignored,
        "FR-11: Win+Space is a layout switch, not a command"
    );
    assert_eq!(recorder.len(), 2, "FR-11: Win+Space does not flush");
    assert_eq!(release(&mut recorder, VK_LWIN), Recorded::Modifier);

    // FR-10: `Win+R` is a command and flushes, repair or no repair.
    assert_eq!(hold(&mut recorder, VK_LWIN), Recorded::Modifier);
    assert_eq!(press(&mut recorder, VK_R, 0x13), Recorded::Flushed);
    assert_eq!(recorder.len(), 0, "FR-10: Win+R is a command and flushes");
    assert_eq!(release(&mut recorder, VK_LWIN), Recorded::Modifier);
}

// -------------------------------------------------------------------------------------
// Task T-13-4 — the seed of `CapsLock`
// -------------------------------------------------------------------------------------

/// **Criterion 6 of task T-13-4: a seeded `CapsLock` reaches the stroke, on both sides of the
/// transition.**
///
/// `Recorder::set_caps_lock` was written by task T-03-2, documented — and called from nowhere in
/// the product until task T-13-4, so `Held::caps` started `false` in every session whatever the
/// keyboard's light said. What that costs is visible in one line of this test: the `CAPS` bit is
/// the cache key of FR-20, so a session started with `CapsLock` on typed «a» where the user saw
/// «A», keyed the cache on the wrong row, and injected the replacement of FR-22 through
/// `KEYEVENTF_UNICODE`, which carries the character literally and knows nothing of the real
/// `CapsLock`.
///
/// Both directions are driven, because a seed is an assignment and not a toggle: the point that
/// seeds does not know whether it is correcting or confirming, and must be right either way.
#[test]
fn a_seeded_capslock_is_carried_by_the_strokes_that_follow_it() {
    let mut recorder = fresh();

    // On: the machine was already in this state when the program started, and nothing but a seed
    // could ever tell the recorder so.
    recorder.set_caps_lock(true);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);

    let seeded = recorder.stroke(0).expect("the stroke typed under the seed");

    assert!(
        seeded.mods().contains(StrokeMods::CAPS),
        "FR-04: the mask of the stroke carries the CapsLock the seed established"
    );
    assert!(
        seeded.keystroke().mods().caps(),
        "and so does the key the conversion of FR-22 will be asked about"
    );

    // Off: the other side of the transition, seeded onto the very same recorder.
    recorder.set_caps_lock(false);

    assert_eq!(press(&mut recorder, VK_A, SCAN_A), Recorded::Stored);

    let cleared = recorder.stroke(1).expect("the stroke typed after the seed");

    assert!(
        !cleared.mods().contains(StrokeMods::CAPS),
        "a seed of `false` clears the bit as surely as a seed of `true` sets it"
    );

    // The bit is not decoration: it is the row of the cache FR-20 reads, so the two strokes of
    // one physical key came out as two different characters.
    assert_eq!(typed(&recorder), "Aa");
}

/// Counts the calls the `CapsLock` probe of the test below is asked for.
///
/// A `CapsProbe` is a bare `fn() -> bool` — the same shape `PhysicalProbe` and `LayoutProbe` have
/// and for the same reasons — so it cannot capture, and a counter it can reach has to be a static.
/// Only one test touches these two.
static CAPS_PROBE_CALLS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Whether the machine the test below is standing in for has `CapsLock` on.
static CAPS_PROBE_ANSWER: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// The machine, as that test states it — the seam that stands in for `hook::caps_lock_on`.
fn stated_machine() -> bool {
    CAPS_PROBE_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    CAPS_PROBE_ANSWER.load(core::sync::atomic::Ordering::Relaxed)
}

/// **Criterion 7 of task T-13-4: a `CapsLock` that never reached `record` is repaired by the seed
/// of points 3 and 4.**
///
/// # What is being driven
///
/// `buffer::set_caps_lock` is the entry point all five points of the task go through: the
/// start-up pipeline, `hook::install`, the resumption of FR-90 (point 3, whose message arrives at
/// `hook::handle_input_message` — driven in `tests\hook.rs`), the return of the buffer from the
/// gate of FR-70 (point 4, driven against `app::restore_buffer` in the unit tests of that module)
/// and the return of the session. Here it is driven with the test's **own probe**, which is what
/// makes a test of this defect possible at all: the state that has to be repaired is one no
/// keystroke can produce, because the whole defect is that the keystroke was **not seen**.
///
/// # The three acts
///
/// 1. the machine's `CapsLock` goes on while the program is suspended (FR-90) or the buffer is
///    parked (FR-70, FR-84) — the tracker keeps saying "off" and the letters come out lower case;
/// 2. the point seeds, and the bits agree with the machine again;
/// 3. it goes off the same way, and the seed agrees again — the transition has two sides.
///
/// The fourth assertion is the thread rule of the task: on a thread with no typing buffer the
/// probe is **not called at all**. That is what keeps `GetKeyState` — which answers about the
/// calling thread's own input queue and would describe nothing on the UI thread — off every
/// thread but the input one, by construction rather than by discipline.
#[test]
fn a_capslock_missed_by_the_hook_is_repaired_by_the_seed() {
    use std::sync::atomic::Ordering;

    // Act 0 — a thread with no buffer: nothing is seeded and the machine is not even asked.
    assert!(!buffer::is_installed());
    CAPS_PROBE_CALLS.store(0, Ordering::Relaxed);

    assert!(
        !buffer::set_caps_lock(stated_machine),
        "a thread with no typing buffer has nothing to seed"
    );
    assert_eq!(
        CAPS_PROBE_CALLS.load(Ordering::Relaxed),
        0,
        "and Win32 is never reached from it — the probe runs inside `buffer::with`"
    );

    buffer::install_recorder(fresh_of(16));

    // Act 1 — the user pressed `CapsLock` where the hook could not see it. The tracker still
    // says "off", and every letter comes out in the wrong case.
    CAPS_PROBE_ANSWER.store(true, Ordering::Relaxed);

    assert_eq!(
        buffer::record(user_press(VK_A, SCAN_A)),
        Recorded::Stored,
        "the strokes the user makes after the missed press are recorded as usual"
    );
    assert_eq!(
        buffer::with(|recorder| recorder.held().caps()),
        Some(false),
        "the defect: the belief cannot follow a press that never reached `record`"
    );

    // Act 2 — the point seeds: the resumption of FR-90, or the buffer coming back from the gate
    // of FR-70. The bits agree with the machine again.
    assert!(buffer::set_caps_lock(stated_machine));
    assert_eq!(CAPS_PROBE_CALLS.load(Ordering::Relaxed), 1);

    assert_eq!(
        buffer::with(|recorder| recorder.held().caps()),
        Some(true),
        "after the seed the belief is the machine's"
    );
    assert_eq!(
        buffer::record(user_press(VK_A, SCAN_A)),
        Recorded::Stored,
        "and the stroke that follows carries it"
    );

    // Act 3 — the other side of the transition, missed the same way and repaired the same way.
    CAPS_PROBE_ANSWER.store(false, Ordering::Relaxed);

    assert!(buffer::set_caps_lock(stated_machine));
    assert_eq!(
        buffer::with(|recorder| recorder.held().caps()),
        Some(false),
        "a machine whose CapsLock went off while nobody was looking is followed as well"
    );
    assert_eq!(buffer::record(user_press(VK_A, SCAN_A)), Recorded::Stored);

    // One physical key, three presses, three rows of the cache of FR-20 — which is what the
    // `CAPS` bit decides and what the defect was silently getting wrong for whole sessions.
    assert_eq!(
        buffer::with(|recorder| typed(recorder)).expect("the buffer of this thread"),
        "aAa"
    );

    buffer::uninstall();
    CAPS_PROBE_ANSWER.store(false, Ordering::Relaxed);
}

/// One press of an ordinary key, delivered through the module-level entry point the hook uses.
///
/// The tests above drive an owned `Recorder`; the seed is reached through `buffer::with`, so the
/// test of it needs the buffer to be **installed on the thread**, which is the situation of the
/// input thread and the one the five points are written for.
fn user_press(vk: u16, scan: u16) -> lang_switcher::hook::KeyEvent {
    KeyEvent {
        vk,
        edge: Edge::Down,
        extra_info: FOREIGN_SIGNATURE,
        scan,
        flags: 0,
        time: SOME_TIME,
    }
}

// -------------------------------------------------------------------------------------
// SEC-02 outside the ring — the form of `buffer::zero_slice`. Task T-13-15.
// -------------------------------------------------------------------------------------

/// The text of a module under `src\`, with the line endings normalised — task **T-13-30**.
///
/// The one place this file reads a source file, and it collapses `\r\n` to `\n` before anybody
/// downstream sees the text. `.gitattributes` declares `* text=auto eol=crlf`, so **the canonical
/// checkout of this repository is CRLF**, while `cargo fmt` writes LF: one commit is one text
/// after a checkout and another after a format. Any sweep whose needle carries a newline answers
/// differently for the two, and the answer it gives on the canonical tree is the wrong one.
///
/// The same reading, and for the same reason, as `read_normalised` in `tests\guard.rs` after
/// task T-13-12 — see the long note there for what a sweep that has quietly stopped bounding what
/// it reads costs. Not reinvented here: normalising at the read is the only place it can be done
/// once.
fn source_of(module: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("src\\{module} must be readable: {error}"));

    text.replace("\r\n", "\n")
}

/// The helper the buffers of `inject` and `selection` are zeroed with is a **volatile** write.
///
/// The value checks of that helper — put text in, call it, read the slots back — live next to it
/// in `src\buffer.rs`, because `zero_slice` is `pub(crate)` and an integration test is a separate
/// crate that cannot call it. What no value check can show is the property the audit of
/// 2026-08-24 was about: that the zeroes are written with `ptr::write_volatile` and fenced, so
/// that an optimiser looking at a buffer nobody reads again may not delete them. A plain
/// `slice.fill(0)` would pass every value check and fail the requirement, so this reads the
/// source, the way the SEC-02 tests of the ring read the backing array.
#[test]
fn the_zeroing_helper_outside_the_ring_is_a_volatile_fenced_write() {
    let source = source_of("buffer.rs");

    let helper = source
        .split("pub(crate) fn zero_slice")
        .nth(1)
        .expect("the helper of task T-13-15 exists");

    // ⚠ **No silent fall-back — task T-13-30.**
    //
    // These three lines used to read `.split("\n}\n").next().expect(…)`. `.next()` on a `Split`
    // that has just been handed a non-empty string **never** answers `None`, so the `expect` was
    // unreachable and said nothing; and on the canonical CRLF tree the needle does not occur at
    // all — the file holds `\r\n}\r\n` — so the "first piece" was the **whole rest of
    // `src\buffer.rs`**: 84 960 bytes where the body of the helper is 1 585.
    //
    // Which way that erred is worth stating exactly, because it is not one way. The two positive
    // assertions became satisfiable by any later function of the module — moving the volatile
    // write into a neighbour of `zero_slice` was measured to pass the old form and fail this one
    // — while the third, the one that forbids `.fill(`, became **stricter** than it claims: it
    // was being asserted about the whole tail. A cut that has stopped cutting is not a cut,
    // whichever way it errs.
    //
    // A boundary that cannot be found is a broken cut, and a broken cut must fail. The shape is
    // the one `tests\guard.rs` settled on in task T-13-12.
    let end = helper.find("\n}\n").expect(
        "src\\buffer.rs: no closing brace in the first column after zero_slice, so the body of \
         the helper cannot be bounded — the assertions below would be made about the rest of the \
         file",
    );

    let body = &helper[..end + "\n}\n".len()];

    // And the cut really cut: a body that is the whole remainder is the degenerate case above
    // wearing a different mask, so it is asserted against rather than trusted.
    assert!(
        body.len() < helper.len(),
        "the body of zero_slice was bounded ({} bytes) rather than taken as the rest of the file \
         ({} bytes)",
        body.len(),
        helper.len()
    );

    assert!(
        helper.starts_with("<T: Copy + Default>(slice: &mut [T])"),
        "the bound is what makes the write sound for `INPUT` and `Keystroke` as well as for \
         numbers, and `Copy` is what says no destructor is being skipped"
    );
    assert!(
        body.contains("ptr::write_volatile(slot, T::default())"),
        "the zeroes are written volatile, exactly as `Ring::zero_slot` writes its own"
    );
    assert!(
        body.contains("compiler_fence(Ordering::SeqCst)"),
        "and fenced, so they cannot be sunk past what the caller does next"
    );
    assert!(
        !body.contains(".fill("),
        "the helper is not a `fill` behind a new name"
    );

    // And the sample it stands next to is untouched: the ring still zeroes its own slots the
    // same way, which is what the audit named a strength of this module.
    let ring = source
        .split("fn zero_slot(&mut self, index: usize)")
        .nth(1)
        .expect("the ring still zeroes its slots");

    assert!(
        ring.contains("ptr::write_volatile(slot, Stroke::ZEROED)"),
        "the ring path of SEC-02 is unchanged"
    );
}
