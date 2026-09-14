//! **Task T-69-4, backlog line T-11-27 — the one test that reads the mirrors of SEC-04a out of
//! module `buffer` lives in a binary of its own.**
//!
//! # Why a binary of its own
//!
//! Module `buffer` publishes two mirrors for the channel of SEC-04a — the length of the ring
//! (`Ring::set_len`) and the position counter of FR-32 (`Recorder::set_cycle`) — and both are
//! process-wide statics of module `control`, because the buffer itself is a thread-local of the
//! input thread and a reader outside that thread needs the numbers published. `cargo test` runs
//! the tests of one binary on parallel threads, and in `tests\buffer.rs` nearly every test drives
//! a recorder through a path that publishes: building one publishes the length of its new ring,
//! every stroke publishes the length again, every flush publishes the counter's zero. Measured on
//! `877dd7a`, over the bodies of the tests: **96 of the 97** do, this one among them.
//!
//! This test read the mirrors there, twice, around a resize — and whatever another test published
//! in between was what it read. Measured on `877dd7a` and again after task T-69-3, by running the
//! binary of `tests\buffer.rs` with the `testing` feature: **22 red runs of 600** in parallel and
//! **12 of 300** after T-69-3 — the length mirror at 1, 2 or 6 where the resize had just
//! published 0, the counter at 3 where it had published 0, or at 0 where `advance_cycle` had just
//! published 3 — and **0 of 300** with `--test-threads=1`. The backlog's "about one run in four"
//! was not reproduced; the mechanism was.
//!
//! Alone in its process, nobody else publishes, and the test says exactly what it was written to
//! say: a resize publishes the zero of the counter beside the zero of the length.
//!
//! # Why not the other cures
//!
//! * **A mutex in `tests\buffer.rs`** would have had to be taken by the reader and by every writer
//!   — 96 tests of 97, which is the whole binary run in single file — and kept honest by a second
//!   guard for the next test somebody writes.
//! * **Asserting only a difference the test made itself** does not close the window: the length
//!   and the counter are overwritten by value, and a writer can land between the resize and the
//!   read however the assertion is phrased.
//! * **A bounded retry** would hide the defect it exists to catch: a resize that stopped publishing
//!   would still pass whenever another test's zero happened to land in the window.
//!
//! # The rule of this file
//!
//! **One test.** A second test here that drives a recorder brings the race back into this binary,
//! so the test counts the tests of its own file first. And `tests\buffer.rs` names nothing of
//! module `control`: a mirror read there would be the old race in a new test, so the test sweeps
//! that file as well — with a positive control on this file and a negative one on a copy of that
//! file with a read planted in it.

#![cfg(feature = "testing")]

use std::path::Path;

use lang_switcher::buffer::{Recorded, Recorder};
use lang_switcher::control;
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::layouts::{KeyMapping, LayoutCache, LayoutId, LayoutMapBuilder, Mods};

/// US — the layout of the strokes below, as in `tests\buffer.rs`.
const EN: LayoutId = LayoutId::from_raw(0x0409_0409);
/// `A`, the key and its scan code.
const VK_A: u16 = 0x41;
const SCAN_A: u16 = 0x1E;
/// `dwExtraInfo` of a stroke that is not ours.
const FOREIGN_SIGNATURE: usize = 0x00CA_FE01;

/// One test file, read as text with its line endings normalised.
fn test_source(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file);

    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("tests\\{file} must be readable: {error}"))
        .replace("\r\n", "\n")
}

/// Lines of `text` that are code and name module `control` — a `use` of it or a path through it.
///
/// The needle is put together at run time so that this file, which is swept too, does not carry
/// it as a literal outside its prose.
fn lines_naming_module_control(text: &str) -> Vec<usize> {
    let needle = ["control", "::"].concat();

    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| line.contains(&needle))
        .map(|(index, _)| index + 1)
        .collect()
}

/// **SEC-04a: a resize publishes the position counter, and the pair of mirrors stays possible.**
///
/// Task T-04-3-3 made the resize publish the zero it stored. Since task T-39-2 that zero comes
/// from the general flush the resize goes through (`Recorder::reset` → `clear_ring`), and the new
/// ring's length goes out through `Ring::set_len`, so both mirrors read zero the moment the resize
/// is done. Before task T-04-3-3 the position mirror was left alone, and a snapshot of SEC-04a
/// could therefore carry `buffer_len = 0` beside a non-zero `cycle_position` — a pair the program
/// itself can never be in, and the pair the bench of §11.5 reads for positions 16 and 17.
///
/// Moved here out of `tests\buffer.rs` by task T-69-4, with one assertion added: the length
/// mirror is checked off zero before the resize as well, which alone in this process is a fact
/// and not a guess.
#[test]
fn a_resize_publishes_the_position_counter_next_to_the_length() {
    // The rule of this file, before anything is published.
    let own = test_source("buffer_mirror.rs");
    let tests_here = own.lines().filter(|line| line.trim() == "#[test]").count();

    assert_eq!(
        tests_here, 1,
        "tests\\buffer_mirror.rs holds one test, alone in its process — a second one that drives a \
         recorder is the race of T-11-27 back"
    );

    let readers_here = lines_naming_module_control(&own);
    assert!(
        !readers_here.is_empty(),
        "the sweep finds no use of module control even in this file, which reads the mirrors — it \
         is measuring nothing"
    );

    let buffer_tests = test_source("buffer.rs");
    let readers_there = lines_naming_module_control(&buffer_tests);
    assert!(
        readers_there.is_empty(),
        "tests\\buffer.rs names module control on lines {readers_there:?} — a mirror read in that \
         binary races the writers beside it (T-11-27); it belongs in this file"
    );

    let planted = buffer_tests.replacen(
        "\n#[test]\n",
        &format!(
            "\n#[test]\nfn planted() {{ let _ = lang_switcher::{}snapshot(); }}\n",
            ["control", "::"].concat()
        ),
        1,
    );
    assert_ne!(
        planted, buffer_tests,
        "the negative control lost its anchor"
    );
    assert!(
        !lines_naming_module_control(&planted).is_empty(),
        "the sweep does not see a mirror read planted in tests\\buffer.rs — it cannot fail"
    );

    // The scenario — task T-04-3-3, and the test `tests\buffer.rs` carried until task T-69-4.
    let mut map = LayoutMapBuilder::new(EN);
    map.set(SCAN_A, false, Mods::NONE, KeyMapping::from_char('a'));

    let mut recorder = Recorder::with_capacity(8);
    recorder.set_cache(LayoutCache::from_maps(vec![map.finish()]).expect("one non-empty map"));
    recorder.set_active_layout(EN);

    for time in [4_240, 4_241, 4_242] {
        let stored = recorder.record(KeyEvent {
            vk: VK_A,
            edge: Edge::Down,
            extra_info: FOREIGN_SIGNATURE,
            scan: SCAN_A,
            flags: 0,
            time,
        });

        assert_eq!(stored, Recorded::Stored);
    }

    for _ in 0..3 {
        recorder.advance_cycle(4);
    }

    assert_eq!(recorder.len(), 3);
    assert_eq!(recorder.cycle_position(), 3, "the counter is off zero");

    // Both mirrors off zero before the resize, which is what makes the zeroes after it a check on
    // `set_capacity` rather than on atomics that happened to be zero already.
    let before = control::snapshot();
    assert_eq!(
        before.cycle_position, 3,
        "advance_cycle publishes, and the mirror is off zero before the resize"
    );
    assert_eq!(
        before.buffer_len, 3,
        "the strokes publish their length, and that mirror is off zero before the resize too"
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
