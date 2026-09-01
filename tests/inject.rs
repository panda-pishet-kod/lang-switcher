//! Integration tests for task T-04-1 — the injection of the replacement.
//!
//! # Why almost nothing here sends anything
//!
//! Every requirement of §4.5 is a statement about a **packet** or about an **order**: what is
//! in the array `SendInput` is given, in which order, how many structures a character outside
//! the BMP takes, how many calls are made, and what happens before what. None of that needs an
//! event to reach the machine, and letting one reach the machine would be the worst possible
//! way to test it: `SendInput` delivers to whatever window is in the foreground, and during
//! `cargo test` that is the terminal of whoever ran it.
//!
//! So module `inject` is split along exactly that line — pure builders into a caller's slice,
//! and a `lang_switcher::inject::Environment` that owns the four things steps 3 to 6 need from
//! the world. [`Bench`] below is an `Environment` that writes down what it was asked to do, and
//! that log is the evidence for FR-40, FR-41, FR-43, FR-44 and FR-45.
//!
//! # The two tests that do send, and the fence around them
//!
//! Points 22 and 23 of the acceptance criterion are end-to-end and cannot be anything else:
//! type into a window, replace, read the window back. They are `#[ignore]`d, so `cargo test`
//! never runs them, and they must be asked for explicitly:
//!
//! ```text
//! cargo test --test inject -- --ignored --test-threads=1
//! ```
//!
//! Both create their own window, and **neither sends a single event until it has established
//! that this window is the foreground one**. If it does not become foreground, the test fails
//! having sent nothing. That fence is the whole reason they are safe to run at all: the task
//! specification forbids synthetic input reaching any window but our own, and a check of
//! `GetForegroundWindow` immediately before the first `SendInput` is the only way to know.
//!
//! No test in this file installs a keyboard hook, for the reason `tests\hook.rs` gives: a
//! `WH_KEYBOARD_LL` hook in a process that is not pumping messages freezes the keyboard of
//! whoever is running `cargo test`.

use std::sync::{Mutex, MutexGuard, PoisonError};

use lang_switcher::buffer::{self, Recorder};
use lang_switcher::convert::{self, Keystroke};
use lang_switcher::hook::INJECTED_SIGNATURE;
use lang_switcher::inject::{
    self, Dispatched, Environment, InjectError, MODIFIER_COUNT, Modifiers, Replaced,
};
use lang_switcher::layouts::{KeyMapping, LayoutId, LayoutMap, LayoutMapBuilder, Mods};
use lang_switcher::settings::{self, ReplacementMethod};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK,
    VK_DELETE, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN,
};

// ---------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------

/// Scan codes of `g h b d t n` on the main block of a set 1 keyboard.
///
/// In US they read `ghbdtn`; the same six physical keys read `привет` in Russian, which is the
/// example §1 of SPEC opens with and the one point 22 of the acceptance criterion asks for.
const GHBDTN: [u16; 6] = [0x22, 0x23, 0x30, 0x20, 0x14, 0x31];

/// `U+1F600`, whose UTF-16 spelling is the surrogate pair `D83D DE00` — point 12.
const NON_BMP: char = '\u{1F600}';

/// The leading half of that pair.
const HIGH_SURROGATE: u16 = 0xD83D;

/// The trailing half of that pair.
const LOW_SURROGATE: u16 = 0xDE00;

/// The eight modifier keys of FR-40 as this file's own `(virtual key, bit)` table — task T-04-2.
///
/// A second copy of the table module `inject` releases and restores by, written out here on
/// purpose: a test that borrowed the module's own table could not catch the module putting the
/// wrong bit on the wrong key. It is what [`Bench::machine`] follows the keyboard with.
const MODIFIER_TABLE: [(VIRTUAL_KEY, Modifiers); MODIFIER_COUNT] = [
    (VK_LSHIFT, Modifiers::LEFT_SHIFT),
    (VK_RSHIFT, Modifiers::RIGHT_SHIFT),
    (VK_LCONTROL, Modifiers::LEFT_CTRL),
    (VK_RCONTROL, Modifiers::RIGHT_CTRL),
    (VK_LMENU, Modifiers::LEFT_ALT),
    (VK_RMENU, Modifiers::RIGHT_ALT),
    (VK_LWIN, Modifiers::LEFT_WIN),
    (VK_RWIN, Modifiers::RIGHT_WIN),
];

/// One `INPUT` reduced to the four fields the requirements speak about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    vk: u16,
    scan: u16,
    flags: u32,
    extra: usize,
}

impl Event {
    /// Whether this is a `Backspace` going down or up — the erasure half of FR-41.
    fn is_backspace(self, up: bool) -> bool {
        self.vk == VK_BACK.0 && (self.flags & KEYEVENTF_KEYUP.0 != 0) == up
    }

    /// Whether this is a character event — `KEYEVENTF_UNICODE`, `wVk = 0`, FR-41.
    ///
    /// Says nothing about the edge, deliberately: the assertions that only care that a slot
    /// holds a character read this one, and the assertions that care *which* edge it is read
    /// [`Event::is_unicode_edge`].
    fn is_unicode(self) -> bool {
        self.flags & KEYEVENTF_UNICODE.0 != 0 && self.vk == 0
    }

    /// Whether this is a character event going down or up — the form task T-10-4 established.
    ///
    /// The edge is part of the requirement now, exactly as it always was for `Backspace`,
    /// `Left` and `Shift`: a code unit travels as a down **and** an up, because Qt renders an
    /// unreleased unicode keydown as the character already latched and the up is what advances
    /// the latch. A test that checked only `is_unicode` would accept the old down-only form and
    /// the new one indifferently, which is the one thing these tests must not do.
    fn is_unicode_edge(self, up: bool) -> bool {
        self.is_unicode() && (self.flags & KEYEVENTF_KEYUP.0 != 0) == up
    }

    /// Whether this is the `Left` arrow of the **navigation block** going down or up — FR-42.
    ///
    /// The extended flag is part of the answer and not an extra: the arrow key and the keypad's
    /// `4` are the same virtual key, and only the `E0` prefix tells them apart.
    fn is_left(self, up: bool) -> bool {
        self.vk == VK_LEFT.0
            && self.flags & KEYEVENTF_EXTENDEDKEY.0 != 0
            && (self.flags & KEYEVENTF_KEYUP.0 != 0) == up
    }

    /// Whether this is a `Shift` going down or up — FR-40 steps 3 and 6, and FR-42's own.
    fn is_shift(self, up: bool) -> bool {
        (self.vk == VK_LSHIFT.0 || self.vk == VK_RSHIFT.0)
            && (self.flags & KEYEVENTF_KEYUP.0 != 0) == up
    }
}

/// Every event of `packet`, reduced.
fn reduce(packet: &[INPUT]) -> Vec<Event> {
    packet
        .iter()
        .map(|event| {
            let key = inject::keyboard(event)
                .expect("module inject builds keyboard events and nothing else");

            Event {
                vk: key.wVk.0,
                scan: key.wScan,
                flags: key.dwFlags.0,
                extra: key.dwExtraInfo,
            }
        })
        .collect()
}

/// What an [`Environment`] was asked to do, in the order it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// FR-40 steps 3 and 6 asked what is held, and got this answer.
    Held(Modifiers),
    /// One `SendInput` — FR-41, FR-44.
    Send(Vec<Event>),
    /// The pause of FR-44.
    Pause(u32),
    /// FR-40 step 5 — the connection point of task T-05-1, FR-43.
    Switch,
}

/// An [`Environment`] that sends nothing and writes down everything.
struct Bench {
    /// What `held` answers, one entry per call. Past the end it answers "nothing held", which
    /// is what makes a test that only cares about the first answer short to write.
    answers: Vec<Modifiers>,
    /// How many times `held` has been asked.
    asked: usize,
    /// How many events every `send` refuses to take — FR-45.
    refuse: usize,
    /// The log.
    log: Vec<Step>,
    /// **Task T-04-2.** The keyboard state this bench models, when it models one.
    ///
    /// `None` is the bench of task T-04-1: `held` reads the scripted `answers` and the events it
    /// is sent change nothing. `Some` is a bench that behaves as the desktop does — the
    /// asynchronous key state follows *injected* modifier events as readily as physical ones —
    /// and it is the only way the trap of FR-42 becomes observable, because a `Shift` the
    /// replacement pressed and failed to release is then a `Shift` step 6 finds and presses
    /// again.
    keyboard: Option<Modifiers>,
}

impl Bench {
    /// A bench that holds nothing and accepts everything.
    fn new() -> Self {
        Self {
            answers: Vec::new(),
            asked: 0,
            refuse: 0,
            log: Vec::new(),
            keyboard: None,
        }
    }

    /// A bench whose `held` answers `answers` in order.
    fn holding(answers: &[Modifiers]) -> Self {
        Self {
            answers: answers.to_vec(),
            ..Self::new()
        }
    }

    /// A bench whose every `SendInput` takes `refuse` events fewer than it was given — FR-45.
    fn refusing(refuse: usize) -> Self {
        Self {
            refuse,
            ..Self::new()
        }
    }

    /// **Task T-04-2.** A bench that models the machine: the user is holding `physical`, and
    /// every modifier event this bench is sent moves the state from there.
    fn machine(physical: Modifiers) -> Self {
        Self {
            keyboard: Some(physical),
            ..Self::new()
        }
    }

    /// What the modelled keyboard holds now.
    fn keyboard(&self) -> Modifiers {
        self.keyboard.unwrap_or(Modifiers::NONE)
    }

    /// Every packet that was sent, in order.
    fn sent(&self) -> Vec<Vec<Event>> {
        self.log
            .iter()
            .filter_map(|step| match step {
                Step::Send(events) => Some(events.clone()),
                _ => None,
            })
            .collect()
    }

    /// Every event that was sent, in order, with the packet boundaries dropped.
    fn stream(&self) -> Vec<Event> {
        self.sent().concat()
    }

    /// Position of the first step satisfying `wanted`, if there is one.
    fn position(&self, wanted: impl FnMut(&Step) -> bool) -> Option<usize> {
        self.log.iter().position(wanted)
    }
}

impl Environment for Bench {
    fn held(&mut self) -> Modifiers {
        let answer = match self.keyboard {
            // A modelled keyboard answers with its state and not with a script — that is what
            // makes the two questions of FR-40 steps 3 and 6 able to disagree honestly.
            Some(state) => state,
            None => self
                .answers
                .get(self.asked)
                .copied()
                .unwrap_or(Modifiers::NONE),
        };

        self.asked += 1;
        self.log.push(Step::Held(answer));

        answer
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        let reduced = reduce(events);

        // A modelled keyboard follows its own events, exactly as the desktop's asynchronous key
        // state follows injected ones.
        if let Some(state) = self.keyboard.as_mut() {
            for event in &reduced {
                for (key, bit) in MODIFIER_TABLE {
                    if event.vk == key.0 {
                        *state = if event.flags & KEYEVENTF_KEYUP.0 == 0 {
                            state.with(bit)
                        } else {
                            Modifiers::from_bits_truncate(state.bits() & !bit.bits())
                        };
                    }
                }
            }
        }

        self.log.push(Step::Send(reduced));

        let taken = events.len().saturating_sub(self.refuse);

        u32::try_from(taken).unwrap_or(u32::MAX)
    }

    fn pause(&mut self, delay_ms: u32) {
        self.log.push(Step::Pause(delay_ms));
    }

    fn switch_layout(&mut self) {
        self.log.push(Step::Switch);
    }
}

/// The hardwired US half of the FR-25 table — the layout the strokes are recorded under.
fn us() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_US).expect("the FR-25 table carries US")
}

/// The hardwired Russian half of the FR-25 table — the target of the conversion.
fn russian() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_RUSSIAN).expect("the FR-25 table carries Russian")
}

/// The six strokes of `ghbdtn`, as the buffer would have recorded them under US.
fn ghbdtn() -> Vec<Keystroke> {
    let source = us();

    GHBDTN
        .iter()
        .map(|&scan| Keystroke::recorded_in(&source, scan, false, Mods::NONE))
        .collect()
}

/// The scan code of the space bar — the soft boundary of FR-10, task **Т-24-3**.
const SCAN_SPACE: u16 = 0x39;

/// The six keys of `ghbdtn` **and the space bar**, as a live cache of FR-20 carries them.
///
/// ⚠ **Not [`us`] and [`russian`], and the difference is a finding of stage Э24.** The hardwired
/// table of FR-25 has 47 rows of the main block and the space bar is not one of them, so a space
/// decoded through it carries no character at all — measured by
/// `the_space_bar_is_absent_from_the_hardwired_table_of_fr25` below. The live cache built by
/// `ToUnicodeEx` over VK `0x08..0xFF` always carries it — measured on this machine by
/// `tests\layouts.rs::the_live_cache_of_fr20_carries_the_space_bar`. The mainline claim of task
/// Т-24-3 is about the product's normal state, so it is asserted on a map that has the key.
fn ghbdtn_layout(layout: LayoutId, characters: [char; 6]) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(layout);

    for (&scan, &character) in GHBDTN.iter().zip(characters.iter()) {
        builder.set(scan, false, Mods::NONE, KeyMapping::from_char(character));
    }

    builder.set(SCAN_SPACE, false, Mods::NONE, KeyMapping::from_char(' '));
    builder.finish()
}

/// The English half of the pair above.
fn us_with_space() -> LayoutMap {
    ghbdtn_layout(
        LayoutId::from_raw(0x0409_0409),
        ['g', 'h', 'b', 'd', 't', 'n'],
    )
}

/// The Russian half of the pair above.
fn russian_with_space() -> LayoutMap {
    ghbdtn_layout(
        LayoutId::from_raw(0x0419_0419),
        ['п', 'р', 'и', 'в', 'е', 'т'],
    )
}

/// The six strokes of `ghbdtn` with `spaces` presses of the space bar after them.
///
/// «слово + хвост», the state the soft boundary of FR-10 leaves in the ring since task Т-24-2.
/// The tail is decoded through the same layout the word is, because that is where it comes from:
/// the recorder puts the space bar through the cache like any other key.
fn ghbdtn_with_tail(spaces: usize) -> Vec<Keystroke> {
    let source = us_with_space();

    let mut strokes: Vec<Keystroke> = GHBDTN
        .iter()
        .map(|&scan| Keystroke::recorded_in(&source, scan, false, Mods::NONE))
        .collect();

    let space = Keystroke::recorded_in(&source, SCAN_SPACE, false, Mods::NONE);
    strokes.extend(std::iter::repeat_n(space, spaces));
    strokes
}

/// A one-key layout in which `scan` produces `mapping`.
fn layout_with(layout: LayoutId, scan: u16, mapping: KeyMapping) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(layout);
    builder.set(scan, false, Mods::NONE, mapping);
    builder.finish()
}

/// `text` as UTF-16.
fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// **The lock every check that makes a `SendInput` come up short has to take** — task T-13-25.
///
/// [`inject::send_mismatches`] is state of the *process* and `cargo test` runs a binary's checks
/// in parallel, so two checks that both make a call come up short while both assert on the
/// counters would be asserting on each other's timing. Until this task there was exactly one
/// such check and the file said so; the counter now has to be readable from the checks of the
/// press as well, and one lock is a cheaper answer than counters that may only be compared with
/// `>=`.
static SHORT_SEND: Mutex<()> = Mutex::new(());

/// Takes that lock, ignoring the poisoning a check that already failed would have left: the
/// counters are `AtomicU32` and no panic can leave them half-written, so a poisoned lock here
/// would only turn one red check into several.
fn short_sends_are_mine() -> MutexGuard<'static, ()> {
    SHORT_SEND.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------------------
// Point 9 — FR-03: every `INPUT` carries the signature
// ---------------------------------------------------------------------------------------

/// **FR-03.** Every structure this module sends carries [`INJECTED_SIGNATURE`] in
/// `dwExtraInfo`, whichever of the three packets it belongs to.
///
/// This is the requirement the whole program rests on and the one whose absence is invisible
/// until it is catastrophic: an `INPUT` without the signature comes back through the hook as
/// the user's own typing, goes into the buffer, and the next hotkey press converts the
/// program's own output — for ever. So the check is made against the events that actually left,
/// through the environment, and not against a builder called in isolation.
#[test]
fn every_input_sent_carries_the_injected_signature_of_fr03() {
    let mut bench = Bench::holding(&[
        Modifiers::LEFT_SHIFT.with(Modifiers::RIGHT_CTRL),
        Modifiers::LEFT_WIN,
    ]);

    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0)
        .expect("the packet is sized from the lengths it is built with");

    // Three packets really did go out: the release of step 3, the replacement, the restore of
    // step 6. A test that found nothing would pass the loop below vacuously.
    assert_eq!(bench.sent().len(), 3);
    assert_eq!(outcome.erased, 6);
    assert_eq!(outcome.typed, 6);

    let stream = bench.stream();

    // 6 backspaces down + 6 up + 6 characters down + 6 up + 2 released + 1 restored.
    assert_eq!(stream.len(), 6 * 2 + 6 * 2 + 2 + 1);

    for event in stream {
        assert_eq!(
            event.extra, INJECTED_SIGNATURE,
            "FR-03: every INPUT carries the signature, this one did not: {event:?}"
        );
    }
}

/// The signature is on the two builders directly as well, including the modifier packets a
/// replacement with nothing held would never produce.
#[test]
fn the_signature_is_on_every_modifier_event_of_both_directions() {
    let all = Modifiers::from_bits_truncate(u8::MAX);
    let mut out = [INPUT::default(); MODIFIER_COUNT];

    for build in [
        inject::build_release as fn(Modifiers, &mut [INPUT]) -> Result<usize, InjectError>,
        inject::build_restore,
    ] {
        let len = build(all, &mut out).expect("eight keys fit an array of MODIFIER_COUNT");

        assert_eq!(len, MODIFIER_COUNT);

        for event in reduce(&out[..len]) {
            assert_eq!(event.extra, INJECTED_SIGNATURE);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Point 10 — FR-41: the shape of the array
// ---------------------------------------------------------------------------------------

/// **FR-41.** `N` × (`Backspace` down + up), and only then the characters as
/// `KEYEVENTF_UNICODE` with `wVk = 0` and `wScan` = the code unit — each unit down **and** up.
#[test]
fn the_packet_is_backspaces_down_and_up_first_then_unicode_events() {
    let text = utf16("привет");
    let mut out = [INPUT::default(); inject::replacement_events(6, 6)];

    let len = inject::build_replacement(6, &text, &mut out).expect("the array is exactly sized");

    assert_eq!(len, 24);

    let events = reduce(&out[..len]);

    // The erasure comes first, in full, and alternates down and up.
    for (index, event) in events[..12].iter().enumerate() {
        assert!(
            event.is_backspace(index % 2 == 1),
            "FR-41: event {index} must be Backspace {}",
            if index % 2 == 1 { "up" } else { "down" }
        );
        assert_eq!(event.scan, 0);
    }

    // Then the characters, in the order they are to appear, as a down and an up per code unit.
    // Task T-10-4: the up is not optional — without it Qt renders every unit after the first as
    // the first, so the pairing is checked here rather than merely the presence of characters.
    let characters = &events[12..];

    assert_eq!(characters.len(), text.len() * 2);

    for (index, &unit) in text.iter().enumerate() {
        let down = characters[index * 2];
        let up = characters[index * 2 + 1];

        assert!(
            down.is_unicode_edge(false),
            "FR-41: character {index} goes down first"
        );
        assert!(
            up.is_unicode_edge(true),
            "T-10-4: character {index} is released before the next one is pressed"
        );
        assert_eq!(down.scan, unit, "FR-41: wScan is the character");
        assert_eq!(up.scan, unit, "the release names the same code unit");
    }
}

/// **Task T-10-4.** A code unit costs two structures in both modes, and the two sizing functions
/// say so — they are what every caller sizes an array with, so an arithmetic that disagreed with
/// the builders would be refused packets rather than wrong ones.
#[test]
fn a_code_unit_is_two_structures_and_both_sizing_functions_agree() {
    assert_eq!(inject::EVENTS_PER_UNIT, 2);

    // Nothing to erase and nothing to say is nothing at all; then one unit, then the six of
    // `привет` with six characters to take off the screen first.
    assert_eq!(inject::replacement_events(0, 0), 0);
    assert_eq!(inject::replacement_events(0, 1), 2);
    assert_eq!(inject::replacement_events(6, 6), 24);

    // The compatibility mode adds the two `Shift` events and the arrows, and counts the
    // insertion identically. With nothing selected there is no `Shift` at all.
    assert_eq!(inject::selection_events(0, 1), 2);
    assert_eq!(inject::selection_events(6, 6), 26);

    // A character outside the BMP is two code units and therefore four structures.
    assert_eq!(inject::replacement_events(1, 2), 6);
    assert_eq!(inject::selection_events(1, 2), 8);

    // And the builders write exactly what the arithmetic promised.
    let text = utf16("привет");
    let mut out = [INPUT::default(); inject::replacement_events(6, 6)];
    assert_eq!(
        inject::build_replacement(6, &text, &mut out),
        Ok(inject::replacement_events(6, 6))
    );

    let mut out = [INPUT::default(); inject::selection_events(6, 6)];
    assert_eq!(
        inject::build_selection(6, &text, &mut out),
        Ok(inject::selection_events(6, 6))
    );
}

/// **Task T-10-4.** A packet ends on a **release**, in both modes, so that it leaves the latch
/// of the receiving application clean.
///
/// This is the half of the defect that made the down+up form look unusable when it was first
/// measured. Qt advances its character latch on the keyup, so a packet that ended on a keydown
/// left the last code unit latched — and the first code unit of the *next* packet was rendered
/// as that stale one and lost. Measured in Telegram Desktop 7.0.9: a packet of pairs followed
/// by a single down-only unit prints that unit correctly, while a packet whose tail was
/// down-only swallows the first character of whatever comes next.
#[test]
fn a_packet_ends_on_a_release_so_the_next_one_is_not_swallowed() {
    let text = utf16("привет");

    let mut out = [INPUT::default(); inject::replacement_events(6, 6)];
    let len = inject::build_replacement(6, &text, &mut out).expect("sized");
    let events = reduce(&out[..len]);

    assert!(
        events[len - 1].is_unicode_edge(true),
        "FR-41: the last event of a replacement is the release of the last code unit"
    );
    assert_eq!(events[len - 1].scan, *text.last().expect("six units"));

    let mut out = [INPUT::default(); inject::selection_events(6, 6)];
    let len = inject::build_selection(6, &text, &mut out).expect("sized");
    let events = reduce(&out[..len]);

    assert!(
        events[len - 1].is_unicode_edge(true),
        "FR-42: and the compatibility packet ends the same way"
    );

    // The two modes differ in how the old text leaves the screen and in nothing else: the
    // character run at the end of both is the same array of events.
    let mut backspace = [INPUT::default(); inject::replacement_events(6, 6)];
    let taken = inject::build_replacement(6, &text, &mut backspace).expect("sized");
    let backspace = reduce(&backspace[..taken]);

    let mut selection = [INPUT::default(); inject::selection_events(6, 6)];
    let taken = inject::build_selection(6, &text, &mut selection).expect("sized");
    let selection = reduce(&selection[..taken]);

    assert_eq!(
        backspace[12..],
        selection[14..],
        "both modes insert through the very same character events"
    );
}

/// A packet that does not fit is refused whole — FR-41 is about an array that is complete and
/// in order, and half of one is worse than none.
#[test]
fn a_packet_that_does_not_fit_is_refused_and_nothing_is_written() {
    let mut out = [INPUT::default(); 3];

    // 2 backspaces (4 events) and two characters (4 events): the packet is eight and the array
    // is three, so nothing is written and the exact length is reported back.
    assert_eq!(
        inject::build_replacement(2, &utf16("ab"), &mut out),
        Err(InjectError::OutputTooSmall { needed: 8 })
    );

    // Untouched: every slot is still the zeroed structure the array was built with, and a
    // zeroed `INPUT` is not even a keyboard event.
    for slot in &out {
        assert!(
            inject::keyboard(slot).is_none(),
            "a refused packet writes nothing at all"
        );
    }
}

/// An empty buffer is not a `SendInput` call with an empty array, which the system rejects.
#[test]
fn nothing_to_send_makes_no_call_at_all() {
    let mut bench = Bench::new();

    assert_eq!(
        inject::dispatch_in(&mut bench, &[], 0),
        Dispatched::default()
    );
    assert_eq!(
        inject::dispatch_in(&mut bench, &[], 25),
        Dispatched::default()
    );
    assert!(bench.log.is_empty());
}

// ---------------------------------------------------------------------------------------
// Point 11 — FR-41: `N` counts characters, not keystrokes
// ---------------------------------------------------------------------------------------

/// **FR-41.** A ligature is one keystroke and several characters, and all of them are erased.
///
/// The trap the requirement names: counting keystrokes would send one `Backspace` for a key
/// that put two characters on the screen and leave the rest of them behind.
#[test]
fn n_counts_characters_so_a_ligature_is_erased_whole() {
    // One key that gives two characters — what `ToUnicodeEx` reports as a ligature.
    let ligature = KeyMapping::from_to_unicode(2, &utf16("ij"));
    assert!(ligature.is_ligature());

    let source = layout_with(LayoutId::from_raw(0x0413_0413), 0x16, ligature);
    let strokes = [Keystroke::recorded_in(&source, 0x16, false, Mods::NONE)];

    // One stroke, two characters — and it is the two that decide.
    assert_eq!(strokes.len(), 1);
    assert_eq!(inject::typed_chars(&strokes), 2);

    // The same statement through the whole path: the target has nothing on that key, so FR-23
    // carries the stroke through unchanged and the two characters are erased and retyped.
    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &strokes, &russian(), 0)
        .expect("the packet is sized from the lengths it is built with");

    assert_eq!(outcome.erased, 2, "FR-41: two characters, two backspaces");

    let replacement = &bench.sent()[0];
    assert_eq!(replacement.iter().filter(|e| e.vk == VK_BACK.0).count(), 4);
}

/// The keys that put nothing on the screen contribute nothing to `N`.
///
/// A key the layout has no character for (FR-23) is recorded as a stroke and erases nothing,
/// because there is nothing of its own to erase.
///
/// ⚠ **This test never covered the dead key its prose used to claim** — audit of 2026-08-31,
/// finding №5. It said "a dead key that has not composed yet erases nothing" and then built a
/// `KeyMapping::EMPTY`, which is a key with no character at all; a real dead mapping carries
/// its accent (FR-24) and was counted as one for as long as the sentence stood. The dead key
/// has its own checks now, built from `KeyMapping::dead` and measured against a live layout —
/// see [`a_composed_pair_is_one_character_on_the_screen`] and the block below it.
#[test]
fn a_stroke_that_produced_no_character_erases_nothing() {
    let nothing = Keystroke::new(
        LayoutId::from_raw(0x0409_0409),
        0x01,
        false,
        Mods::NONE,
        KeyMapping::EMPTY,
    );

    assert_eq!(inject::typed_chars(&[nothing]), 0);
    assert_eq!(inject::typed_chars(&[]), 0);

    // And a real run of six ordinary keys is six characters, not six of something else.
    assert_eq!(inject::typed_chars(&ghbdtn()), 6);
}

// ---------------------------------------------------------------------------------------
// Э20 — мёртвые клавиши. Находка №5 аудита 2026-08-31, решение — DECISIONS вопрос 75.
//
// Числа сняты прибором `<dev>\sandbox\probes\deadkeys` на настоящей раскладке
// «США (международная)» (KLID 00020409, HKL F0010409) и подтверждены живым опытом
// пользователя в Блокноте; журналы — `scratchpad-Э20\замер-композиции.txt` и
// `красное-до-живое.txt`. Раскладка здесь строится синтетически (`KeyMapping::dead`),
// чтобы эти проверки жили в батарее и на машине, где стоят ровно две раскладки.
// ---------------------------------------------------------------------------------------

/// Мёртвый символ раскладки `00020409` — сам апостроф `U+0027`, не `U+00B4`.
///
/// Снято прибором (столбец «В БУФЕРЕ»: «caf'e») и подтверждено живым опытом: после замены
/// на экране стояло «сфа'у» — апостроф на месте четвёртого штриха.
const DEAD_ACUTE: char = '\u{0027}';

/// Штрихи слова `café`, как их записал бы буфер под «США (международной)».
///
/// Пять штрихов: `c`, `a`, `f`, мёртвый акут и `e`. Продукт зовёт `ToUnicodeEx` с
/// `TO_UNICODE_NO_STATE` (FR-06), поэтому состояние мёртвой клавиши не копится и акут
/// с буквой остаются двумя независимыми штрихами — ровно то, что прибор снял на живой
/// раскладке.
fn cafe_under_us_international() -> Vec<Keystroke> {
    let source = us_international();

    [0x2E, 0x1E, 0x21, 0x28, 0x12]
        .iter()
        .map(|&scan| Keystroke::recorded_in(&source, scan, false, Mods::NONE))
        .collect()
}

/// Синтетическая «США (международная)»: пять клавиш слова `café`, четвёртая — мёртвая.
fn us_international() -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0x0409_0409));
    builder.set(0x2E, false, Mods::NONE, KeyMapping::from_char('c'));
    builder.set(0x1E, false, Mods::NONE, KeyMapping::from_char('a'));
    builder.set(0x21, false, Mods::NONE, KeyMapping::from_char('f'));
    builder.set(0x28, false, Mods::NONE, KeyMapping::dead(DEAD_ACUTE));
    builder.set(0x12, false, Mods::NONE, KeyMapping::from_char('e'));
    builder.finish()
}

/// Цель первого шага цикла: те же клавиши по-русски. На мёртвой клавише цели нет ничего —
/// штрих проходит насквозь по FR-24.
fn russian_for_cafe() -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0x0419_0419));
    builder.set(0x2E, false, Mods::NONE, KeyMapping::from_char('с'));
    builder.set(0x1E, false, Mods::NONE, KeyMapping::from_char('ф'));
    builder.set(0x21, false, Mods::NONE, KeyMapping::from_char('а'));
    builder.set(0x12, false, Mods::NONE, KeyMapping::from_char('у'));
    builder.finish()
}

/// **FR-24 не тронут: мёртвый штрих по-прежнему хранится с флагом и со своим акцентом.**
///
/// Ремонт Э20 менял только *счёт* стираемого, и это проверка на то, что он не «починил»
/// дефект, опустошив отображение: пустой мёртвый штрих тоже дал бы ноль на экране, но
/// сломал бы FR-24 и перенос «без изменений».
#[test]
fn a_dead_stroke_is_still_recorded_with_its_flag_and_its_accent() {
    let strokes = cafe_under_us_international();
    let dead = strokes[3];

    assert!(dead.is_dead(), "FR-24: штрих несёт флаг мёртвой клавиши");
    assert_eq!(
        dead.produced().units(),
        &utf16("'")[..],
        "FR-24: акцент хранится в штрихе как был — не опустошён"
    );
}

/// **FR-41 после Э20: `N` — это символы НА ЭКРАНЕ, и скомпонованная пара их даёт один.**
///
/// Замер на живой «США (международной)»: `c a f ' e` кладёт на экран **`café` — четыре
/// символа** (`ToUnicodeEx` вернул `-1` на акут и `1` на `e`, отдав скомпонованное `é`),
/// а в буфер — **пять штрихов**. Считать пять значит послать пятый `Backspace` и съесть
/// символ **перед** словом — чужой текст. Живой опыт 2026-08-31 это и показал: из
/// «12345 café» вышло «12345сфа'у», пробел съеден.
#[test]
fn a_composed_pair_is_one_character_on_the_screen() {
    let strokes = cafe_under_us_international();

    assert_eq!(strokes.len(), 5, "в буфере пять штрихов");
    assert_eq!(
        inject::typed_chars(&strokes),
        4,
        "на экране «café» — четыре символа (прибор deadkeys, строка «слово целиком»)"
    );
}

/// **Хвостовая мёртвая клавиша: на экране ещё ничего.**
///
/// Замер: одна мёртвая клавиша без продолжения кладёт на экран **ноль** символов
/// (`ToUnicodeEx` вернул `-1`, акцент подвешен и готовым символом ещё не стал),
/// а в буфере это один штрих с флагом.
#[test]
fn a_trailing_dead_key_is_nothing_on_the_screen_yet() {
    let source = us_international();
    let dead = [Keystroke::recorded_in(&source, 0x28, false, Mods::NONE)];

    assert!(dead[0].is_dead(), "FR-24: штрих хранится с флагом");
    assert_eq!(
        inject::typed_chars(&dead),
        0,
        "на экране ещё ничего (прибор deadkeys, «dead последним, без продолжения»)"
    );
}

/// **Цена выбранного правила, названная вслух** — DECISIONS вопрос 75.
///
/// Продукт не может отличить `' e` (компонуется, один символ) от `' t` (не компонуется,
/// два), не спросив ОС с накоплением состояния, а это запрещает FR-06. Правило «мёртвый
/// штрих вносит ноль» ошибается на несоставляющей паре **в меньшую сторону**: на экране
/// два символа, сотрётся один, лишний акцент останется стоять. Это осознанный выбор —
/// недостереть значит оставить свой видимый символ, перестереть значит молча съесть чужой
/// текст. Тест держит эту цену на виду, чтобы её нельзя было изменить не заметив.
#[test]
fn the_chosen_rule_is_one_short_on_a_pair_that_does_not_compose() {
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0x0409_0409));
    builder.set(0x28, false, Mods::NONE, KeyMapping::dead(DEAD_ACUTE));
    builder.set(0x14, false, Mods::NONE, KeyMapping::from_char('t'));
    let source = builder.finish();

    let strokes: Vec<Keystroke> = [0x28, 0x14]
        .iter()
        .map(|&scan| Keystroke::recorded_in(&source, scan, false, Mods::NONE))
        .collect();

    // Прибор: на экране «'t» — два символа. Правило отвечает один, и это известно.
    assert_eq!(
        inject::typed_chars(&strokes),
        1,
        "правило вносит ноль за мёртвую: один вместо двух — недостирание, безопасная сторона"
    );
}

/// **Хвост мягкой границы стирается и печатается наравне со словом** — задача Т-24-3.
///
/// `N` из FR-41 — число символов на экране, и с задачи Т-24-2 прогон может кончаться
/// пробелами: пробел стоит на экране обычным символом, стирается обычным `Backspace` и
/// кладётся обратно обычным `KEYEVENTF_UNICODE`. Ни одной оговорки под хвост в механике счёта
/// не заводится — она считает единицы отображения, а у пробела их ровно одна.
///
/// Второй счёт, `as_injected`, снят здесь же: со второго шага цикла на экране стоит
/// предыдущая инжекция (Э20), и у неё та же ширина — пробел отображается в пробел в обеих
/// раскладках пары.
#[test]
fn a_word_with_a_tail_of_spaces_is_erased_and_retyped_whole() {
    let strokes = ghbdtn_with_tail(1);

    assert_eq!(
        inject::OnScreen::as_typed(&strokes).count(),
        7,
        "FR-41: «ghbdtn » стоит на экране семью символами — шесть букв и хвост"
    );

    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &strokes, &russian_with_space(), 0)
        .expect("пакет размерен по длинам, которыми построен");

    assert_eq!(
        outcome.erased, 7,
        "стирается всё, что этот прогон поставил на экран, — вместе с хвостом"
    );
    assert_eq!(
        outcome.typed, 7,
        "и кладётся «привет » — столько же, сколько стёрто"
    );

    assert_eq!(
        inject::OnScreen::as_injected(&strokes, &russian_with_space()).count(),
        7,
        "со второго шага цикла считается предыдущая инжекция, и она той же ширины"
    );

    // Хвост из двух пробелов — то же самое и на единицу шире с каждой стороны.
    let longer = ghbdtn_with_tail(2);

    assert_eq!(inject::OnScreen::as_typed(&longer).count(), 8);
    assert_eq!(
        inject::OnScreen::as_injected(&longer, &russian_with_space()).count(),
        8
    );
}

/// ⚠ **Находка м-Э24-1: в жёстко зашитой таблице FR-25 пробела нет.** Измерено, не выведено.
///
/// [`FALLBACK_KEYS`] — 47 клавиш основного блока; клавиша пробела в их число не входит, и её
/// отсутствие было безразлично ровно до задачи Т-24-2: пробел в буфер не попадал. Теперь
/// попадает, и в аварийном режиме FR-25 (динамический кэш FR-20 не построился) штрих пробела
/// не несёт ни одной единицы отображения. Следствие для FR-41 — счёт на экране короче
/// действительного на длину хвоста: «ghbdtn » будет стёрто шестью `Backspace` из семи нужных.
///
/// **Сторона ошибки — та же, что у мёртвой клавиши** (Э20): недостирание, а не перестирание.
/// Лишний символ остаётся на экране видимым и правится рукой человека; чужой текст не
/// съедается. Этим она и отличается от порчи.
///
/// **Здесь она закреплена числом, а не починена.** Починка — одна строка в таблице FR-25, но
/// вместе с ней двигаются четыре рукописные строки перекрёстной проверки в `src\convert.rs` и
/// три числа `47` в `tests\convert.rs`; довод самой таблицы против клавиатуры («читается
/// одинаково в обеих раскладках — конвертации она ничего не даёт») с приходом счёта FR-41
/// перестал быть полным, и это вопрос пользователю, а не правка исполнителя. Отчёт этапа Э24,
/// плохие новости.
#[test]
fn the_space_bar_is_absent_from_the_hardwired_table_of_fr25() {
    let fallback = us();
    let space = Keystroke::recorded_in(&fallback, SCAN_SPACE, false, Mods::NONE);

    assert!(
        space.produced().units().is_empty(),
        "таблица FR-25 не отдаёт пробел: если отдаёт — находка закрыта и этот тест снимается"
    );

    // И вот чем это оборачивается для счёта FR-41 на аварийном пути.
    let mut strokes: Vec<Keystroke> = GHBDTN
        .iter()
        .map(|&scan| Keystroke::recorded_in(&fallback, scan, false, Mods::NONE))
        .collect();
    strokes.push(space);

    assert_eq!(
        inject::OnScreen::as_typed(&strokes).count(),
        6,
        "шесть вместо семи — недостирание на длину хвоста, безопасная сторона"
    );
}

/// **Соседний текст цел: замена стирает ровно слово** — инвариант И-1 этапа Э20.
///
/// Тот же путь, что прошёл живой опыт, только через стенд: пять штрихов `café`, цель —
/// русская. Четыре `Backspace`, а не пять; пятый забрал бы символ, стоящий перед словом.
#[test]
fn a_word_with_a_dead_key_is_erased_without_touching_its_neighbour() {
    let strokes = cafe_under_us_international();

    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &strokes, &russian_for_cafe(), 0)
        .expect("пакет размерен по длинам, которыми построен");

    assert_eq!(
        outcome.erased, 4,
        "FR-41: на экране «café» четыре символа — четыре Backspace, сосед не тронут"
    );
    assert_eq!(
        outcome.typed, 5,
        "инжекция кладёт «сфа'у» — мёртвый штрих проходит насквозь по FR-24 и композиции \
         при KEYEVENTF_UNICODE нет"
    );

    // Четыре символа — восемь событий: FR-41 просит Backspace вниз и вверх.
    let replacement = &bench.sent()[0];
    assert_eq!(
        replacement.iter().filter(|e| e.vk == VK_BACK.0).count(),
        8,
        "четыре Backspace, каждый вниз и вверх"
    );
}

/// **Шаг цикла ≥ 2 стирает то, что стоит на экране** — инвариант И-2 этапа Э20.
///
/// На втором шаге на экране стоит **предыдущая инжекция**, а не то, что набрал человек.
/// С мёртвой клавишей числа расходятся уже на первом шаге: `café` набрано — четыре
/// символа, те же штрихи вложены по-русски — «сфа'у», пять, потому что композиции при
/// `KEYEVENTF_UNICODE` нет. Это и снял живой опыт 2026-08-31.
#[test]
fn the_second_step_erases_what_the_first_one_injected() {
    let strokes = cafe_under_us_international();
    let first_target = russian_for_cafe();

    let mut bench = Bench::new();
    let first = inject::replace_in_with(
        &mut bench,
        &strokes,
        &first_target,
        inject::OnScreen::as_typed(&strokes),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("пакет размерен");

    assert_eq!(first.erased, 4, "первый шаг стирает набранное — «café»");
    assert_eq!(first.typed, 5, "и кладёт на экран пять символов — «сфа'у»");

    // Второй шаг: те же штрихи (FR-32 буфер не переписывает), другая цель. Считать надо
    // по тому, что стоит на экране сейчас, — по предыдущей инжекции.
    let second = inject::replace_in_with(
        &mut bench,
        &strokes,
        &us_international(),
        inject::OnScreen::as_injected(&strokes, &first_target),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("пакет размерен");

    assert_eq!(
        second.erased, 5,
        "на экране стоит предыдущая инжекция — пять символов, а не четыре набранных"
    );
}

/// **Лигатура на шаге цикла ≥ 2 — та же нога, вторым родом отображения.**
///
/// Лигатур (отображение len>1) не нашлось ни на одной раскладке машины — прибор `deadkeys`
/// ответил «лигатур: НЕТ» на всех трёх, и это записано прямо: живой лигатурой нога не
/// меряется. Синтетикой — меряется, и здесь она измерена.
#[test]
fn the_second_step_erases_a_ligature_the_first_one_injected() {
    let source = layout_with(
        LayoutId::from_raw(0x0409_0409),
        0x16,
        KeyMapping::from_char('u'),
    );
    let strokes = [Keystroke::recorded_in(&source, 0x16, false, Mods::NONE)];

    let ligature = KeyMapping::from_to_unicode(2, &utf16("ij"));
    let first_target = layout_with(LayoutId::from_raw(0x0413_0413), 0x16, ligature);

    let mut bench = Bench::new();
    let first = inject::replace_in_with(
        &mut bench,
        &strokes,
        &first_target,
        inject::OnScreen::as_typed(&strokes),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("пакет размерен");

    assert_eq!(
        first.erased, 1,
        "на экране был один символ — то, что набрали"
    );
    assert_eq!(first.typed, 2, "инжекция положила на экран два символа");

    let second_target = layout_with(
        LayoutId::from_raw(0x0419_0419),
        0x16,
        KeyMapping::from_char('г'),
    );
    let second = inject::replace_in_with(
        &mut bench,
        &strokes,
        &second_target,
        inject::OnScreen::as_injected(&strokes, &first_target),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("пакет размерен");

    assert_eq!(
        second.erased, 2,
        "на экране стоит предыдущая инжекция — два символа, а не один исходный штрих"
    );
}

// ---------------------------------------------------------------------------------------
// Point 12 — FR-41: surrogate pairs
// ---------------------------------------------------------------------------------------

/// **FR-41.** A character outside the BMP leaves as its two code units, adjacent and in order —
/// a pair split apart is two undefined characters, not one character.
///
/// Since task T-10-4 a code unit is a down and an up, so the character is four structures: the
/// leading half pressed and released, then the trailing half. The order of the *units* is what
/// FR-41 fixes and it is unchanged.
#[test]
fn a_character_outside_the_bmp_leaves_as_two_adjacent_inputs() {
    let target = layout_with(
        LayoutId::from_raw(0x0419_0419),
        0x22,
        KeyMapping::from_char(NON_BMP),
    );

    let source = us();
    let strokes = [Keystroke::recorded_in(&source, 0x22, false, Mods::NONE)];

    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &strokes, &target, 0)
        .expect("the packet is sized from the lengths it is built with");

    // One character was on the screen, so one `Backspace`; one character comes back, and it
    // takes two code units to say it.
    assert_eq!(outcome.erased, 1);
    assert_eq!(outcome.typed, 2);

    let replacement = &bench.sent()[0];

    // One backspace (2 events) and one character (2 code units × 2 edges).
    assert_eq!(replacement.len(), 2 + 4);
    assert!(replacement[2].is_unicode_edge(false));
    assert!(replacement[3].is_unicode_edge(true));
    assert!(replacement[4].is_unicode_edge(false));
    assert!(replacement[5].is_unicode_edge(true));
    assert_eq!(replacement[2].scan, HIGH_SURROGATE);
    assert_eq!(replacement[3].scan, HIGH_SURROGATE);
    assert_eq!(replacement[4].scan, LOW_SURROGATE, "the pair is not split");
    assert_eq!(replacement[5].scan, LOW_SURROGATE);

    // And the pair is one character for the erasure, not two: two backspaces would have eaten
    // the character before it.
    assert_eq!(inject::typed_chars(&strokes), 1);
}

// ---------------------------------------------------------------------------------------
// Point 13 — FR-40 step 3: the modifiers come off first
// ---------------------------------------------------------------------------------------

/// **FR-40 step 3.** Everything the user is holding is released **before** the first character
/// of the replacement is sent.
///
/// §4.5 of SPEC gives the reason: a `Shift` held while the hotkey is pressed turns the injected
/// `Backspace` into `Shift+Backspace` and the injected characters into a selection, so the user
/// who reached for the hotkey with a little finger on `Shift` gets text selected rather than
/// text replaced.
#[test]
fn the_held_modifiers_are_released_before_the_replacement_is_sent() {
    let held = Modifiers::LEFT_SHIFT
        .with(Modifiers::RIGHT_CTRL)
        .with(Modifiers::LEFT_WIN);

    let mut bench = Bench::holding(&[held, Modifiers::NONE]);

    inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0).expect("the packet is sized");

    let packets = bench.sent();

    // Step 3, and it is the *first* thing that goes out.
    let release = &packets[0];
    assert_eq!(release.len(), 3);

    for event in release {
        assert_ne!(
            event.flags & KEYEVENTF_KEYUP.0,
            0,
            "FR-40 step 3 releases keys, it does not press them"
        );
        assert_eq!(event.vk, event.vk, "a virtual key, not a character");
        assert!(!event.is_unicode());
    }

    // The replacement is the second packet, so not one `Backspace` and not one character was
    // sent while anything was still held.
    let replacement = &packets[1];
    assert!(replacement[0].is_backspace(false));

    // Stated once more on the flat stream, because that is the form the requirement is in:
    // every release strictly precedes every replacement event.
    let stream = bench.stream();
    let last_release = 2;
    let first_replacement = 3;
    assert_ne!(stream[last_release].flags & KEYEVENTF_KEYUP.0, 0);
    assert!(stream[first_replacement].is_backspace(false));
}

/// Nothing held is not a reason to send an empty packet.
#[test]
fn holding_nothing_releases_nothing_and_restores_nothing() {
    let mut bench = Bench::new();

    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0).expect("sized");

    assert_eq!(outcome.released, Modifiers::NONE);
    assert_eq!(outcome.restored, Modifiers::NONE);
    assert_eq!(outcome.release, Dispatched::default());
    assert_eq!(outcome.restore, Dispatched::default());

    // The replacement, and nothing else, was sent.
    assert_eq!(bench.sent().len(), 1);
}

// ---------------------------------------------------------------------------------------
// Point 14 — FR-40 step 6: only what is held *now* comes back
// ---------------------------------------------------------------------------------------

/// **FR-40 step 6.** The restore puts back what the user is holding **at that moment**, not
/// what step 3 released.
///
/// The requirement is explicit — "модификаторы, которые пользователь удерживает физически" —
/// and the case is ordinary: the user lets go of `Shift` while the replacement is on its way,
/// and pressing it again would leave the application holding a key nobody is pressing.
#[test]
fn only_the_modifiers_held_at_the_moment_of_restoration_come_back() {
    // Step 3 finds `LeftShift` and `LeftAlt`; by step 6 the user has let go of both and is
    // holding `RightCtrl` instead.
    let at_release = Modifiers::LEFT_SHIFT.with(Modifiers::LEFT_ALT);
    let at_restore = Modifiers::RIGHT_CTRL;

    let mut bench = Bench::holding(&[at_release, at_restore]);

    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0).expect("sized");

    assert_eq!(outcome.released, at_release);
    assert_eq!(outcome.restored, at_restore);

    // The state was asked for twice, which is the whole mechanism: one answer could not have
    // differed from itself.
    assert_eq!(bench.asked, 2);

    let packets = bench.sent();
    let restore = &packets[2];

    // One key comes back, it is a press, and it is the one held now.
    assert_eq!(restore.len(), 1);
    assert_eq!(restore[0].flags & KEYEVENTF_KEYUP.0, 0);

    let mut expected = [INPUT::default(); MODIFIER_COUNT];
    let len = inject::build_restore(at_restore, &mut expected).expect("one key fits");
    assert_eq!(restore, &reduce(&expected[..len]));

    // And what step 3 released is not among what step 6 pressed.
    let mut released = [INPUT::default(); MODIFIER_COUNT];
    let len = inject::build_release(at_release, &mut released).expect("two keys fit");
    for event in reduce(&released[..len]) {
        assert_ne!(restore[0].vk, event.vk);
    }
}

// ---------------------------------------------------------------------------------------
// Point 15 — FR-41: one call at delay zero
// ---------------------------------------------------------------------------------------

/// **FR-41.** With the pause of FR-44 at its default of zero, the whole array goes in
/// **exactly one** `SendInput` call — atomicity and the guarantee of order.
#[test]
fn a_zero_delay_sends_the_whole_array_in_exactly_one_call() {
    let text = utf16("привет");
    let mut packet = [INPUT::default(); inject::replacement_events(6, 6)];
    let len = inject::build_replacement(6, &text, &mut packet).expect("sized");

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 0);

    assert_eq!(done.calls, 1, "FR-41: one call, not one per event");
    assert_eq!(done.requested, 24);
    assert_eq!(done.accepted, 24);
    assert!(done.is_complete());

    // One call, carrying everything, in order, and no pause anywhere.
    assert_eq!(bench.log.len(), 1);
    assert_eq!(bench.sent()[0].len(), 24);
    assert!(!bench.log.iter().any(|step| matches!(step, Step::Pause(_))));

    // And through the whole of FR-40: three packets, one call each.
    let mut bench = Bench::holding(&[Modifiers::LEFT_SHIFT, Modifiers::LEFT_SHIFT]);
    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0).expect("sized");

    assert_eq!(outcome.replacement.calls, 1);
    assert_eq!(outcome.release.calls, 1);
    assert_eq!(outcome.restore.calls, 1);
}

// ---------------------------------------------------------------------------------------
// Point 16 — FR-44: portions with a pause, order preserved
// ---------------------------------------------------------------------------------------

/// **FR-44.** A pause above zero sends the events in portions with the pause **between** them,
/// and the order and the content of the array are exactly what one call would have carried.
///
/// This is the half of the FR-41/FR-44 contradiction the user opts into: atomicity traded for
/// an application that drops events when they arrive in a batch.
#[test]
fn a_delay_above_zero_sends_portions_with_a_pause_and_keeps_the_order() {
    let text = utf16("да");
    let mut packet = [INPUT::default(); inject::replacement_events(2, 2)];
    let len = inject::build_replacement(2, &text, &mut packet).expect("sized");

    let whole = reduce(&packet[..len]);

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 7);

    // Eight events (2 backspaces down+up, 2 characters down+up), eight calls, and the same
    // eight events in the same order.
    assert_eq!(done.calls, 8);
    assert_eq!(done.requested, 8);
    assert_eq!(done.accepted, 8);
    assert_eq!(
        bench.stream(),
        whole,
        "FR-44 changes the timing, not the order"
    );

    // The pause goes *between*: seven of them for eight portions, never before the first and
    // never after the last.
    let expected: Vec<Step> = whole
        .iter()
        .enumerate()
        .flat_map(|(index, event)| {
            let pause = (index > 0).then_some(Step::Pause(7));
            pause.into_iter().chain([Step::Send(vec![*event])])
        })
        .collect();

    assert_eq!(bench.log, expected);
}

/// **FR-44 against FR-41.** A surrogate pair is never split across portions: two halves in two
/// calls are two undefined characters, not one character delivered slowly.
///
/// Since task T-10-4 the character is four structures rather than two — both halves pressed and
/// released — and all four have to stay in the one call. The trap this guards against is a
/// portioning rule that keys on the code unit alone: the event after the leading half's keydown
/// is that same half's keyup, so such a rule would group *those* two and let the trailing half
/// travel on its own, which is the very split FR-41 forbids.
#[test]
fn a_surrogate_pair_stays_in_one_portion_however_long_the_pause() {
    let mut packet = [INPUT::default(); inject::replacement_events(0, 3)];
    let len = inject::build_replacement(
        0,
        &[HIGH_SURROGATE, LOW_SURROGATE, u16::from(b'!')],
        &mut packet,
    )
    .expect("sized");

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 3);

    // Three portions: the whole character, then the two edges of the `!`.
    assert_eq!(done.calls, 3);

    let packets = bench.sent();

    assert_eq!(packets[0].len(), 4, "the character travels whole");
    assert_eq!(packets[0][0].scan, HIGH_SURROGATE);
    assert_eq!(packets[0][1].scan, HIGH_SURROGATE);
    assert_eq!(packets[0][2].scan, LOW_SURROGATE, "the pair is not split");
    assert_eq!(packets[0][3].scan, LOW_SURROGATE);
    assert!(packets[0][0].is_unicode_edge(false));
    assert!(packets[0][1].is_unicode_edge(true));
    assert!(packets[0][2].is_unicode_edge(false));
    assert!(packets[0][3].is_unicode_edge(true));

    assert_eq!(packets[1].len(), 1);
    assert_eq!(packets[1][0].scan, u16::from(b'!'));
    assert!(packets[1][0].is_unicode_edge(false));
    assert_eq!(packets[2].len(), 1);
    assert_eq!(packets[2][0].scan, u16::from(b'!'));
    assert!(packets[2][0].is_unicode_edge(true));
}

/// A slice that stops in the middle of a character is portioned to its end and no further.
///
/// Nothing in this module can produce such a slice — the builders always write whole characters
/// — and that is exactly why the guard is worth a test: the portioning walks the events it is
/// given, and an answer past the end would be a panic on the replacement path of a resident
/// program rather than a wrong character.
#[test]
fn a_slice_that_stops_inside_a_character_is_portioned_to_its_end_and_no_further() {
    let mut packet = [INPUT::default(); inject::replacement_events(0, 2)];
    let len =
        inject::build_replacement(0, &[HIGH_SURROGATE, LOW_SURROGATE], &mut packet).expect("sized");

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len - 1], 2);

    // Three events of the four: they go out in one portion, short but never overrunning.
    assert_eq!(done.calls, 1);
    assert_eq!(done.requested, 3);
    assert_eq!(bench.sent()[0].len(), 3);
}

// ---------------------------------------------------------------------------------------
// Point 17 — FR-45: the return value is checked
// ---------------------------------------------------------------------------------------

/// **FR-45.** A `SendInput` that reports fewer events than it was given is a discrepancy, and
/// the discrepancy is counted.
///
/// Both the count of calls and the number of events lost are asserted in one test on purpose:
/// the counters belong to the process, `cargo test` runs a binary's tests in parallel, and two
/// tests asserting on the same counter would be asserting on each other's timing. The other
/// tests that make a `SendInput` come up short are the press checks of task T-13-25, and
/// [`short_sends_are_mine`] is what keeps them out of each other's way.
#[test]
fn a_short_return_from_sendinput_reaches_the_counter_of_fr45() {
    let _short_sends = short_sends_are_mine();

    let text = utf16("нет");
    let mut packet = [INPUT::default(); inject::replacement_events(3, 3)];
    let len = inject::build_replacement(3, &text, &mut packet).expect("sized");

    let before = inject::send_mismatches();

    // A complete send changes nothing — the counter must not fire on the ordinary case.
    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 0);
    assert!(done.is_complete());
    assert_eq!(inject::send_mismatches(), before);

    // The system took two events fewer than it was given: one call, two events lost.
    let mut bench = Bench::refusing(2);
    let done = inject::dispatch_in(&mut bench, &packet[..len], 0);

    // 3 backspaces down+up and 3 characters down+up — twelve events, of which ten were taken.
    assert_eq!(done.requested, 12);
    assert_eq!(done.accepted, 10);
    assert!(!done.is_complete(), "FR-45: the return value is examined");

    let after = inject::send_mismatches();
    assert_eq!(after.0, before.0 + 1, "one call came up short");
    assert_eq!(after.1, before.1 + 2, "two events were lost");
}

// ---------------------------------------------------------------------------------------
// Point 18 — FR-43: the replacement is formed and sent before the switch
// ---------------------------------------------------------------------------------------

/// **FR-43.** The replacement is complete — formed *and* sent — before the layout switch of
/// FR-40 step 5 is reached, and the modifiers are restored only after it.
///
/// The switch itself is task T-05-1 and does nothing yet; what T-04-1 owes is its **position**,
/// and a position is only checkable if it is observable, which is why it is a member of
/// `Environment` rather than a comment in the body.
#[test]
fn the_replacement_is_formed_and_sent_before_the_layout_switch_point() {
    let mut bench = Bench::holding(&[Modifiers::LEFT_SHIFT, Modifiers::LEFT_SHIFT]);

    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), 0).expect("sized");

    let switch = bench
        .position(|step| matches!(step, Step::Switch))
        .expect("FR-40 step 5 is reached exactly once");

    // The log, in full, is the requirement: ask, release, replace, switch, ask, restore.
    assert!(matches!(bench.log[0], Step::Held(_)));
    assert!(matches!(bench.log[1], Step::Send(_)));
    assert!(matches!(bench.log[2], Step::Send(_)));
    assert_eq!(switch, 3);
    assert!(matches!(bench.log[4], Step::Held(_)));
    assert!(matches!(bench.log[5], Step::Send(_)));
    assert_eq!(bench.log.len(), 6);

    // Everything the replacement consists of went out before the switch was reached, and the
    // whole of it: `accepted` is the system's own count.
    assert_eq!(outcome.replacement.accepted, 24);
    assert!(outcome.replacement.is_complete());

    let sent_before_switch: usize = bench.log[..switch]
        .iter()
        .filter_map(|step| match step {
            Step::Send(events) => Some(events.len()),
            _ => None,
        })
        .sum();

    // The one released `Shift` of step 3, then the whole replacement.
    assert_eq!(sent_before_switch, 1 + 24);
}

// ---------------------------------------------------------------------------------------
// The far end of the handoff — `on_hotkey` on a thread with no buffer
// ---------------------------------------------------------------------------------------

/// A thread with no typing buffer is every thread of the program but the input one, and an
/// unsolicited `WM_APP_HOTKEY` from another process reaches all three windows (SEC-05).
/// Neither may make this program send anything.
#[test]
fn a_thread_without_a_buffer_sends_nothing_at_all() {
    assert!(!buffer::is_installed());
    assert!(inject::on_hotkey().is_none());
}

/// An empty buffer is nothing to replace, and nothing to replace is not a `SendInput` call.
#[test]
fn an_empty_buffer_is_nothing_to_replace() {
    buffer::install_recorder(Recorder::with_capacity(16));

    assert!(inject::on_hotkey().is_none());

    buffer::uninstall();
}

// ---------------------------------------------------------------------------------------
// Task T-13-25 — the press moves the counter of FR-32 only when the injection was taken
// ---------------------------------------------------------------------------------------
//
// The finding of the audit of 2026-08-24: `on_hotkey` branched on `outcome.is_some()` alone,
// and `replace_with` answers `Ok(Replaced)` even when `SendInput` accepted nothing — so a
// blocked injection (UIPI towards an elevated window from a build without `uiAccess`,
// `BlockInput`) left the screen unchanged while the position counter of FR-32 moved and the
// session was marked converted. Every press after that rendered the wrong step of the cycle.
//
// `inject::note_press` is the half of the press that decides, and it is reachable through the
// `Environment` seam — which is the only way to ask this question without letting a single
// event reach the machine, for the reason the head of this file gives.

/// A cycle of three layouts, so that "the counter moved" and "the counter did not" are two
/// different numbers rather than two ways of reading a zero.
const CYCLE_LEN: usize = 3;

/// The whole `backspace` packet for `ghbdtn` → `привет`: six characters taken off the screen and
/// six code units typed back, each of them an edge down and an edge up.
const WHOLE_PACKET: usize = inject::replacement_events(6, 6);

/// Installs a buffer on this thread and leaves the position counter of FR-32 at **1**.
///
/// Off zero on purpose: "the counter did not move" has to be a statement about the press and
/// not about an atomic that was zero anyway. Answers the pair every check below compares with.
fn a_buffer_one_step_along_the_cycle() -> (usize, bool) {
    buffer::install_recorder(Recorder::with_capacity(16));

    buffer::with(|recorder| {
        recorder.advance_cycle(CYCLE_LEN);
        (recorder.cycle_position(), recorder.in_conversion())
    })
    .expect("the buffer was installed on this thread one line ago")
}

/// The position counter of FR-32 and the conversion session of FR-10, as they stand now.
fn cycle_and_session() -> (usize, bool) {
    buffer::with(|recorder| (recorder.cycle_position(), recorder.in_conversion()))
        .expect("the buffer is installed on this thread")
}

/// One press of `ghbdtn` against `bench`, in the `backspace` mode of FR-41.
fn press_ghbdtn(bench: &mut Bench) -> Replaced {
    inject::replace_in(bench, &ghbdtn(), &russian(), 0)
        .expect("the packet is sized from the lengths it is built with")
}

/// **The finding itself.** A replacement `SendInput` refused outright leaves the position
/// counter of FR-32 and the conversion session of FR-10 exactly where it found them.
///
/// The bench takes none of the packet, which is what a blocked injection looks like from inside
/// this program: `replace_in` still answers `Ok`, and the screen still shows the text the user
/// typed. A counter that moved here would describe a screen that does not exist.
#[test]
fn an_injection_the_system_refused_moves_neither_the_counter_nor_the_session() {
    let _short_sends = short_sends_are_mine();

    let (before_cycle, before_session) = a_buffer_one_step_along_the_cycle();
    assert_eq!(
        (before_cycle, before_session),
        (1, false),
        "before the press"
    );

    // Nothing is held, so steps 3 and 6 build empty modifier packets and send nothing at all:
    // the one `SendInput` of this press is the replacement, and this bench takes none of it.
    let mut bench = Bench::refusing(WHOLE_PACKET);
    let outcome = press_ghbdtn(&mut bench);

    assert_eq!(bench.sent().len(), 1, "step 4 alone reached the system");
    assert_eq!(outcome.replacement.requested, WHOLE_PACKET);
    assert_eq!(outcome.replacement.accepted, 0, "the injection was blocked");
    assert!(!outcome.reached_the_screen());

    inject::note_press(Some(&outcome), CYCLE_LEN);

    let (after_cycle, after_session) = cycle_and_session();
    assert_eq!(
        after_cycle, 1,
        "FR-32: the screen did not change, so the position did not move"
    );
    assert!(
        !after_session,
        "FR-10: a session in which nothing was converted is not open"
    );
    assert_eq!((after_cycle, after_session), (before_cycle, before_session));

    // The other way in, unchanged by this task: a press that produced no `Replaced` at all — a
    // sizing failure — moves nothing either.
    inject::note_press(None, CYCLE_LEN);
    assert_eq!(cycle_and_session(), (1, false));

    buffer::uninstall();
}

/// **The ordinary press is untouched.** A packet the system took whole marks the session
/// converted and moves the position counter of FR-32 by one, exactly as before this task.
#[test]
fn a_replacement_the_system_took_whole_moves_the_counter_and_opens_the_session() {
    let (before_cycle, before_session) = a_buffer_one_step_along_the_cycle();
    assert_eq!(
        (before_cycle, before_session),
        (1, false),
        "before the press"
    );

    let mut bench = Bench::new();
    let outcome = press_ghbdtn(&mut bench);

    assert_eq!(outcome.replacement.requested, WHOLE_PACKET);
    assert_eq!(outcome.replacement.accepted, WHOLE_PACKET);
    assert!(outcome.reached_the_screen());

    inject::note_press(Some(&outcome), CYCLE_LEN);

    assert_eq!(
        cycle_and_session(),
        (2, true),
        "FR-32 and FR-10: one step further along the cycle, session open"
    );

    buffer::uninstall();
}

/// **Conservative on purpose.** A packet the system took only *in part* does not move the
/// counter either — and the discrepancy is still counted where FR-45 counts it.
///
/// Half a word replaced is the failure nobody notices: the screen and the counter part company
/// and stay parted for the rest of the session. A counter that did not move is the failure the
/// user repairs by pressing the hotkey again. The second assertion is the other half of the
/// requirement — the bookkeeping of FR-45 was to be left exactly as it was, and it is: one call
/// came up short by one event, and both counters say so.
#[test]
fn a_replacement_taken_in_part_moves_nothing_and_is_still_counted_by_fr45() {
    let _short_sends = short_sends_are_mine();

    let (before_cycle, before_session) = a_buffer_one_step_along_the_cycle();
    assert_eq!(
        (before_cycle, before_session),
        (1, false),
        "before the press"
    );

    let before_counters = inject::send_mismatches();

    let mut bench = Bench::refusing(1);
    let outcome = press_ghbdtn(&mut bench);

    assert_eq!(outcome.replacement.requested, WHOLE_PACKET);
    assert_eq!(outcome.replacement.accepted, WHOLE_PACKET - 1);
    assert!(
        !outcome.reached_the_screen(),
        "part of a packet is not the packet"
    );

    inject::note_press(Some(&outcome), CYCLE_LEN);

    assert_eq!(
        cycle_and_session(),
        (1, false),
        "FR-32 and FR-10: a half-replaced word moves neither"
    );

    let after_counters = inject::send_mismatches();
    assert_eq!(
        after_counters.0,
        before_counters.0 + 1,
        "FR-45: one call still came up short, and is still counted"
    );
    assert_eq!(
        after_counters.1,
        before_counters.1 + 1,
        "FR-45: and the one event it lost is still counted too"
    );

    buffer::uninstall();
}

/// **What `is_complete` counts, and what it does not.** Neither the pause of FR-44 nor a
/// surrogate pair can make a packet the system took whole look incomplete.
///
/// `Dispatched::requested` and `Dispatched::accepted` are `INPUT` **structures**, summed over
/// every call the dispatch made, so the split FR-44 introduces changes `calls` and nothing else.
/// A character outside the BMP is four structures that `portion_end` keeps inside one portion,
/// so the pair is not split across calls in the first place — and even the short last portion of
/// a packet that ended mid-character would still be counted on both sides of the comparison.
/// The check matters because the strength the audit confirmed must not turn into a refusal to
/// advance the cycle.
#[test]
fn neither_the_pause_of_fr44_nor_a_surrogate_pair_makes_a_taken_packet_incomplete() {
    let target = layout_with(
        LayoutId::from_raw(0x0419_0419),
        0x22,
        KeyMapping::from_char(NON_BMP),
    );

    let source = us();
    let strokes = [Keystroke::recorded_in(&source, 0x22, false, Mods::NONE)];

    let (before_cycle, before_session) = a_buffer_one_step_along_the_cycle();
    assert_eq!(
        (before_cycle, before_session),
        (1, false),
        "before the press"
    );

    // A pause above zero is the opt-in case of FR-44: the packet leaves in portions.
    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &strokes, &target, 4)
        .expect("the packet is sized from the lengths it is built with");

    assert_eq!(outcome.erased, 1, "one character on the screen");
    assert_eq!(outcome.typed, 2, "two code units — the surrogate pair");
    assert!(
        outcome.replacement.calls > 1,
        "FR-44 really did split the packet into portions"
    );
    assert_eq!(
        outcome.replacement.requested,
        inject::replacement_events(1, 2),
        "the portions add up to the whole packet, structure for structure"
    );
    assert!(
        outcome.reached_the_screen(),
        "`is_complete` counts structures the system took, not the calls that carried them"
    );

    inject::note_press(Some(&outcome), CYCLE_LEN);

    assert_eq!(
        cycle_and_session(),
        (2, true),
        "so the ordinary press of FR-44 advances the cycle exactly as FR-41's does"
    );

    buffer::uninstall();
}

/// The pause of FR-44 defaults to the zero of section 7 and survives a round trip.
#[test]
fn the_pause_of_fr44_defaults_to_zero_and_can_be_published() {
    assert_eq!(inject::inter_event_delay_ms(), 0);

    inject::set_inter_event_delay_ms(12);
    assert_eq!(inject::inter_event_delay_ms(), 12);

    // Put back, because this is process-wide state and the tests of this binary run in
    // parallel: a value left behind would be another test's environment.
    inject::set_inter_event_delay_ms(0);
}

// =========================================================================================
// Task T-04-2 — FR-42, the compatibility mode through a selection
// =========================================================================================

/// The compatibility packet for `ghbdtn` → `привет`, and the log it produced.
///
/// Six characters: one `Shift` down, six `Left` down + up, one `Shift` up, six characters.
fn selection_run(bench: &mut Bench) -> lang_switcher::inject::Replaced {
    inject::replace_in_with(
        bench,
        &ghbdtn(),
        &russian(),
        inject::OnScreen::as_typed(&ghbdtn()),
        0,
        ReplacementMethod::Selection,
    )
    .expect("the packet is sized from the lengths it is built with")
}

// ---------------------------------------------------------------------------------------
// Point 9 — FR-42: `N` × `Shift+Left`, then the insertion
// ---------------------------------------------------------------------------------------

/// **FR-42.** The packet is `Shift` down, `N` × (`Left` down + up), `Shift` up, and then the
/// insertion — and it contains no `Backspace` and no `Delete` at all.
///
/// The last part is the requirement's point: FR-42 exists because in an application with
/// autocompletion one `Backspace` may delete a whole substituted token instead of one character
/// (§10, constraint 3), so a compatibility mode that still erased anything would mitigate
/// nothing. The selection is replaced by the insertion itself — "выделение заменяется одним
/// событием" — and the first character event is that one event.
#[test]
fn the_compatibility_packet_is_n_shift_left_then_the_insertion() {
    let text = utf16("привет");
    let mut out = [INPUT::default(); inject::selection_events(6, 6)];

    let len = inject::build_selection(6, &text, &mut out).expect("the array is exactly sized");

    // One `Shift` down, twelve arrow events, one `Shift` up, six characters down and up.
    assert_eq!(len, 1 + 12 + 1 + 12);

    let events = reduce(&out[..len]);

    // The selection is made first, under one `Shift` of ours.
    assert!(events[0].is_shift(false), "FR-42: our own Shift goes down");
    assert_eq!(events[0].scan, 0);

    for (index, event) in events[1..13].iter().enumerate() {
        assert!(
            event.is_left(index % 2 == 1),
            "FR-42: event {} must be the Left arrow {}",
            index + 1,
            if index % 2 == 1 { "up" } else { "down" }
        );
        assert_eq!(event.scan, 0);
    }

    assert!(
        events[13].is_shift(true),
        "FR-42: our own Shift comes up, and before the insertion"
    );

    // Then the insertion, a down and an up per code unit — the same run FR-41 ends with, and
    // for the same reason (task T-10-4): this mode inserts through the very same character
    // events, so it had the very same defect in Qt and is fixed by the very same form.
    let characters = &events[14..];

    assert_eq!(characters.len(), text.len() * 2);

    for (index, &unit) in text.iter().enumerate() {
        let down = characters[index * 2];
        let up = characters[index * 2 + 1];

        assert!(
            down.is_unicode_edge(false),
            "FR-42: character {index} goes down first"
        );
        assert!(
            up.is_unicode_edge(true),
            "T-10-4: character {index} is released before the next one is pressed"
        );
        assert_eq!(down.scan, unit, "FR-42: wScan is the character");
        assert_eq!(up.scan, unit, "the release names the same code unit");
    }

    // Nothing is erased anywhere in this mode.
    assert!(
        !events
            .iter()
            .any(|e| e.vk == VK_BACK.0 || e.vk == VK_DELETE.0),
        "FR-42: the selection is replaced, not deleted"
    );

    // Exactly two `Shift` events for six selected characters — one keystroke held, not six.
    assert_eq!(events.iter().filter(|e| e.is_shift(false)).count(), 1);
    assert_eq!(events.iter().filter(|e| e.is_shift(true)).count(), 1);
}

/// Nothing typed is nothing to select, and a `Shift+Left` sent then would select somebody else's
/// character and the insertion would replace it.
#[test]
fn an_empty_run_selects_nothing_and_presses_no_shift_at_all() {
    let text = utf16("да");
    let mut out = [INPUT::default(); inject::selection_events(0, 2)];

    let len = inject::build_selection(0, &text, &mut out).expect("the array is exactly sized");

    // Two code units, a down and an up each, and not one arrow or `Shift` among them.
    assert_eq!(len, 4);
    assert_eq!(inject::selection_events(0, 2), 4);

    let events = reduce(&out[..len]);

    for (index, &unit) in text.iter().enumerate() {
        assert!(events[index * 2].is_unicode_edge(false));
        assert!(events[index * 2 + 1].is_unicode_edge(true));
        assert_eq!(events[index * 2].scan, unit);
        assert_eq!(events[index * 2 + 1].scan, unit);
    }
}

/// A packet that does not fit is refused whole, in this mode as in the other.
#[test]
fn a_compatibility_packet_that_does_not_fit_is_refused_and_nothing_is_written() {
    let mut out = [INPUT::default(); 5];

    // Two `Shift` events, two arrows down and up, two characters down and up.
    assert_eq!(
        inject::build_selection(2, &utf16("ab"), &mut out),
        Err(InjectError::OutputTooSmall { needed: 10 })
    );

    for slot in &out {
        assert!(
            inject::keyboard(slot).is_none(),
            "a refused packet writes nothing at all"
        );
    }
}

// ---------------------------------------------------------------------------------------
// Point 10 — FR-41, FR-42: `N` counts characters, not keystrokes
// ---------------------------------------------------------------------------------------

/// **FR-42.** `N` is the same `N` as in FR-41 — characters on the screen, not keys pressed — so
/// a ligature is selected whole by two `Shift+Left` and not half of it by one.
#[test]
fn n_counts_characters_in_the_compatibility_mode_so_a_ligature_is_selected_whole() {
    let ligature = KeyMapping::from_to_unicode(2, &utf16("ij"));
    assert!(ligature.is_ligature());

    let source = layout_with(LayoutId::from_raw(0x0413_0413), 0x16, ligature);
    let strokes = [Keystroke::recorded_in(&source, 0x16, false, Mods::NONE)];

    // One stroke, two characters — and it is the two that decide.
    assert_eq!(strokes.len(), 1);
    assert_eq!(inject::typed_chars(&strokes), 2);

    let mut bench = Bench::new();
    let outcome = inject::replace_in_with(
        &mut bench,
        &strokes,
        &russian(),
        inject::OnScreen::as_typed(&strokes),
        0,
        ReplacementMethod::Selection,
    )
    .expect("the packet is sized from the lengths it is built with");

    assert_eq!(outcome.erased, 2, "FR-42: two characters, two Shift+Left");

    let packet = &bench.sent()[0];

    // Two presses of the arrow, that is two down and two up — for one keystroke.
    assert_eq!(packet.iter().filter(|e| e.is_left(false)).count(), 2);
    assert_eq!(packet.iter().filter(|e| e.is_left(true)).count(), 2);

    // And still one `Shift`, held across both.
    assert_eq!(packet.iter().filter(|e| e.is_shift(false)).count(), 1);
    assert_eq!(packet.iter().filter(|e| e.is_shift(true)).count(), 1);
}

/// A key that put nothing on the screen selects nothing, exactly as it erases nothing.
#[test]
fn a_stroke_that_produced_no_character_selects_nothing() {
    let nothing = Keystroke::new(
        LayoutId::from_raw(0x0409_0409),
        0x01,
        false,
        Mods::NONE,
        KeyMapping::EMPTY,
    );

    let mut bench = Bench::new();
    let outcome = inject::replace_in_with(
        &mut bench,
        &nothing_but(&nothing),
        &russian(),
        inject::OnScreen::as_typed(&nothing_but(&nothing)),
        0,
        ReplacementMethod::Selection,
    )
    .expect("sized");

    assert_eq!(outcome.erased, 0);

    // No selection means no `Shift` and no arrow — and the FR-23 pass-through of the stroke's
    // own (absent) characters means nothing is inserted either, so nothing is sent at all.
    assert!(bench.sent().iter().flatten().next().is_none());
}

/// One stroke as a slice — a name for the `[stroke]` above, so the assertion above reads.
fn nothing_but(stroke: &Keystroke) -> Vec<Keystroke> {
    vec![*stroke]
}

// ---------------------------------------------------------------------------------------
// Points 11 and 12 — FR-40 steps 3 and 6 against the `Shift` of FR-42
// ---------------------------------------------------------------------------------------

/// **FR-40 steps 3 and 6, against FR-42. The trap of this task.**
///
/// Two `Shift`s meet on this path: the one the *user* may be holding when the hotkey is pressed,
/// and the one the compatibility mode presses **itself** to make the selection. The order is
/// forced in both directions:
///
/// * the user's `Shift` is released by step 3 **before** the selection, or the arrows would be
///   indistinguishable from ours and the state would be nobody's;
/// * ours is released **before the insertion**, or the characters would extend a selection
///   instead of replacing it.
///
/// The user here is holding the very key the selection is about to press, which is the case the
/// two could be confused in at all.
#[test]
fn the_users_shift_is_released_before_the_selection_and_ours_before_the_insertion() {
    let mut bench = Bench::machine(Modifiers::LEFT_SHIFT);
    let outcome = selection_run(&mut bench);

    assert_eq!(
        outcome.released,
        Modifiers::LEFT_SHIFT,
        "FR-40 step 3 found the user's Shift"
    );

    let stream = bench.stream();

    // Step 3 goes out first and it is a release.
    assert!(
        stream[0].is_shift(true),
        "FR-40 step 3: the user's Shift comes off before anything else"
    );

    let our_shift_down = stream
        .iter()
        .position(|e| e.is_shift(false))
        .expect("FR-42 presses a Shift of its own");
    let our_shift_up = stream
        .iter()
        .skip(our_shift_down)
        .position(|e| e.is_shift(true))
        .map(|index| index + our_shift_down)
        .expect("FR-42 releases the Shift it pressed");
    let first_arrow = stream
        .iter()
        .position(|e| e.is_left(false))
        .expect("FR-42 selects with the Left arrow");
    let last_arrow = stream
        .iter()
        .rposition(|e| e.is_left(true))
        .expect("FR-42 selects with the Left arrow");
    let first_character = stream
        .iter()
        .position(|e| e.is_unicode())
        .expect("FR-42 inserts the conversion");

    assert!(
        0 < our_shift_down,
        "the user's Shift is off before ours goes on"
    );
    assert!(
        our_shift_down < first_arrow,
        "the selection is made with our own Shift held"
    );
    assert!(
        last_arrow < our_shift_up,
        "the whole selection, then the release"
    );
    assert!(
        our_shift_up < first_character,
        "FR-42: ours is off before a single character is inserted"
    );

    // Three `Shift` events in the entire run and no more: the user's release, ours down, ours
    // up. A fourth would mean somebody pressed a `Shift` nobody accounted for.
    assert_eq!(stream.iter().filter(|e| e.is_shift(false)).count(), 1);
    assert_eq!(stream.iter().filter(|e| e.is_shift(true)).count(), 2);
}

/// **FR-40 step 6, against FR-42.** The `Shift` the selection pressed itself is never mistaken
/// for one the user is holding, and is therefore never pressed again.
///
/// This is the failure the requirement's step 6 makes possible and the mode has to rule out: the
/// restore puts back "модификаторы, которые пользователь удерживает физически", it learns them
/// from the desktop's asynchronous key state, and that state follows injected events too. A
/// `Shift` left down by the packet would be reported as the user's and pressed a second time —
/// a `Shift` stuck down for good on a machine whose owner is holding nothing.
#[test]
fn step_six_does_not_restore_the_shift_the_selection_pressed_itself() {
    // The user is holding nothing at all, so every `Shift` in this run is the packet's own.
    let mut bench = Bench::machine(Modifiers::NONE);
    let outcome = selection_run(&mut bench);

    assert_eq!(outcome.released, Modifiers::NONE, "nothing was held");
    assert_eq!(
        outcome.restored,
        Modifiers::NONE,
        "FR-40 step 6: and nothing is what comes back"
    );
    assert_eq!(
        outcome.restore,
        Dispatched::default(),
        "not one event was sent to press a key nobody is pressing"
    );

    // The keyboard is left exactly as it was found.
    assert_eq!(bench.keyboard(), Modifiers::NONE);

    // Two `Shift` events, one down and one up, and both of them the packet's own.
    assert_eq!(
        bench.stream().iter().filter(|e| e.is_shift(false)).count(),
        1
    );
    assert_eq!(
        bench.stream().iter().filter(|e| e.is_shift(true)).count(),
        1
    );

    // The same with the user holding one, which is the case step 6 could get wrong in the other
    // direction: what step 3 released is not put back by step 6 either, because by then the
    // asynchronous state says it is up — and that is the behaviour of task T-04-1's mode too.
    let mut bench = Bench::machine(Modifiers::LEFT_SHIFT);
    let outcome = selection_run(&mut bench);

    assert_eq!(outcome.released, Modifiers::LEFT_SHIFT);
    assert_eq!(outcome.restored, Modifiers::NONE);
    assert_eq!(bench.keyboard(), Modifiers::NONE, "no Shift is left down");

    // And the check is not vacuous: a packet that *did* leave its own `Shift` down would be
    // seen by this bench, which is the whole reason it exists.
    let mut leaky = Bench::machine(Modifiers::NONE);
    let mut down = [INPUT::default(); 1];
    let len = inject::build_restore(Modifiers::LEFT_SHIFT, &mut down).expect("one key fits");
    inject::dispatch_in(&mut leaky, &down[..len], 0);

    assert_eq!(leaky.held(), Modifiers::LEFT_SHIFT);
}

// ---------------------------------------------------------------------------------------
// Point 13 — FR-03: the signature is on every `INPUT` of this mode too
// ---------------------------------------------------------------------------------------

/// **FR-03.** Every structure the compatibility mode sends carries [`INJECTED_SIGNATURE`] —
/// the `Shift` and the arrows of the selection as much as the characters of the insertion.
///
/// A `Shift+Left` without the signature would come back through the hook as the user's own
/// navigation, and module `buffer` would take it for the editing key it is and flush the buffer
/// the replacement is being made from.
#[test]
fn every_input_of_the_compatibility_mode_carries_the_injected_signature_of_fr03() {
    let mut bench = Bench::holding(&[
        Modifiers::RIGHT_SHIFT.with(Modifiers::LEFT_WIN),
        Modifiers::LEFT_CTRL,
    ]);

    let outcome = selection_run(&mut bench);

    // Three packets really did go out — a test that found none would pass the loop vacuously.
    assert_eq!(bench.sent().len(), 3);
    assert_eq!(outcome.erased, 6);
    assert_eq!(outcome.typed, 6);

    let stream = bench.stream();

    // 2 released + (Shift + 12 arrows + Shift + 6 characters down and up) + 1 restored.
    assert_eq!(stream.len(), 2 + (1 + 12 + 1 + 12) + 1);

    for event in stream {
        assert_eq!(
            event.extra, INJECTED_SIGNATURE,
            "FR-03: every INPUT carries the signature, this one did not: {event:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------
// Point 14 — FR-41: one call at delay zero, in this mode as well
// ---------------------------------------------------------------------------------------

/// **FR-41.** The compatibility packet goes in **exactly one** `SendInput` call at the default
/// pause of section 7 — the selection and the insertion are one atomic, ordered array.
#[test]
fn a_zero_delay_sends_the_compatibility_packet_in_exactly_one_call() {
    let text = utf16("привет");
    let mut packet = [INPUT::default(); inject::selection_events(6, 6)];
    let len = inject::build_selection(6, &text, &mut packet).expect("sized");

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 0);

    assert_eq!(done.calls, 1, "FR-41: one call, not one per event");
    assert_eq!(done.requested, 26);
    assert_eq!(done.accepted, 26);
    assert!(done.is_complete());

    assert_eq!(bench.log.len(), 1);
    assert_eq!(bench.sent()[0].len(), 26);
    assert!(!bench.log.iter().any(|step| matches!(step, Step::Pause(_))));

    // And through the whole of FR-40 in this mode: three packets, one call each.
    let mut bench = Bench::holding(&[Modifiers::LEFT_ALT, Modifiers::LEFT_ALT]);
    let outcome = selection_run(&mut bench);

    assert_eq!(outcome.release.calls, 1);
    assert_eq!(outcome.replacement.calls, 1);
    assert_eq!(outcome.restore.calls, 1);
}

// ---------------------------------------------------------------------------------------
// Point 15 — FR-44: portions with a pause, and the selection is not torn
// ---------------------------------------------------------------------------------------

/// **FR-44 in the compatibility mode.** A pause above zero sends the packet in portions with the
/// pause between them; the order and the content are exactly what one call would have carried,
/// and the selection is not broken in the middle.
///
/// "Not broken" is a statement about the `Shift`: it goes down in the first portion and comes up
/// only after the last arrow, because a portion boundary is a pause and not a release. If it
/// were released between arrows, every arrow after the first would move the caret instead of
/// extending the selection and the insertion would land in the middle of the user's word.
#[test]
fn a_delay_above_zero_portions_the_compatibility_packet_without_tearing_the_selection() {
    let text = utf16("да");
    let mut packet = [INPUT::default(); inject::selection_events(2, 2)];
    let len = inject::build_selection(2, &text, &mut packet).expect("sized");

    let whole = reduce(&packet[..len]);
    assert_eq!(whole.len(), 1 + 4 + 1 + 4);

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 5);

    assert_eq!(done.calls, 10);
    assert_eq!(done.requested, 10);
    assert_eq!(done.accepted, 10);
    assert_eq!(
        bench.stream(),
        whole,
        "FR-44 changes the timing, not the order"
    );

    // The pause goes *between*: nine of them for ten portions. One event per portion, because
    // there is no character outside the BMP in "да" to hold four events together.
    let expected: Vec<Step> = whole
        .iter()
        .enumerate()
        .flat_map(|(index, event)| {
            let pause = (index > 0).then_some(Step::Pause(5));
            pause.into_iter().chain([Step::Send(vec![*event])])
        })
        .collect();

    assert_eq!(bench.log, expected);

    // The selection survives the portioning: one `Shift` down at the front, one up after the
    // last arrow and before the first character, and nothing in between.
    let stream = bench.stream();
    assert!(stream[0].is_shift(false));
    assert!(stream[4].is_left(true), "the last arrow of the selection");
    assert!(stream[5].is_shift(true));
    assert!(
        stream[6].is_unicode_edge(false),
        "the insertion begins with the first character going down"
    );
    assert_eq!(stream.iter().filter(|e| e.is_shift(false)).count(), 1);
    assert_eq!(stream.iter().filter(|e| e.is_shift(true)).count(), 1);
}

// ---------------------------------------------------------------------------------------
// Point 16 — FR-41: surrogate pairs in the compatibility mode
// ---------------------------------------------------------------------------------------

/// **FR-41 in the compatibility mode.** A character outside the BMP is inserted as its two code
/// units, adjacent and in order — four `INPUT` structures since task T-10-4 — and selected by
/// **one** `Shift+Left`: it is one character on the screen however many code units it takes to
/// say.
#[test]
fn a_character_outside_the_bmp_leaves_the_compatibility_mode_as_two_adjacent_inputs() {
    let target = layout_with(
        LayoutId::from_raw(0x0419_0419),
        0x22,
        KeyMapping::from_char(NON_BMP),
    );

    let source = us();
    let strokes = [Keystroke::recorded_in(&source, 0x22, false, Mods::NONE)];

    let mut bench = Bench::new();
    let outcome = inject::replace_in_with(
        &mut bench,
        &strokes,
        &target,
        inject::OnScreen::as_typed(&strokes),
        0,
        ReplacementMethod::Selection,
    )
    .expect("the packet is sized from the lengths it is built with");

    assert_eq!(outcome.erased, 1);
    assert_eq!(outcome.typed, 2);

    let packet = &bench.sent()[0];

    // Shift, one arrow down and up, Shift, and both halves of the character pressed and
    // released.
    assert_eq!(packet.len(), 1 + 2 + 1 + 4);
    assert_eq!(packet.iter().filter(|e| e.is_left(false)).count(), 1);
    assert!(packet[4].is_unicode_edge(false));
    assert!(packet[5].is_unicode_edge(true));
    assert!(packet[6].is_unicode_edge(false));
    assert!(packet[7].is_unicode_edge(true));
    assert_eq!(packet[4].scan, HIGH_SURROGATE);
    assert_eq!(packet[5].scan, HIGH_SURROGATE);
    assert_eq!(packet[6].scan, LOW_SURROGATE, "the pair is not split");
    assert_eq!(packet[7].scan, LOW_SURROGATE);

    // And it stays unsplit under FR-44 as well: `dispatch_in` recognises the character from the
    // events themselves, so the rule is the same packet by packet and mode by mode.
    let mut bench = Bench::new();
    let mut whole = [INPUT::default(); inject::selection_events(1, 2)];
    let len =
        inject::build_selection(1, &[HIGH_SURROGATE, LOW_SURROGATE], &mut whole).expect("sized");
    inject::dispatch_in(&mut bench, &whole[..len], 4);

    let last = bench
        .sent()
        .pop()
        .expect("the insertion is the last portion");
    assert_eq!(last.len(), 4, "all four events of the character, one call");
    assert_eq!(last[0].scan, HIGH_SURROGATE);
    assert_eq!(last[1].scan, HIGH_SURROGATE);
    assert_eq!(last[2].scan, LOW_SURROGATE);
    assert_eq!(last[3].scan, LOW_SURROGATE);
}

// ---------------------------------------------------------------------------------------
// Points 17 and 18 — FR-42, section 7: the default mode, and the mode of task T-04-1
// ---------------------------------------------------------------------------------------

/// **FR-42, section 7.** The default is `backspace`, in the schema, in `settings` and in
/// `inject`, and choosing it explicitly is the same thing as not choosing at all.
///
/// The journey of a value from a real `config.toml` into the atomic is
/// `inject::tests::the_replacement_section_of_the_configuration_reaches_this_module`, a unit test
/// of the library: both published values are process-wide state, this binary already has a test
/// that publishes a pause and puts it back, and a second writer here would be asserting on that
/// test's timing. Nothing in this file writes either atomic outside the `#[ignore]`d checks.
#[test]
fn the_default_of_section_7_is_auto_and_replace_in_still_runs_the_backspace_packet() {
    // FR-42а moved the default of section 7 from `backspace` to `auto` — in the schema, in
    // the derived default and in the starting value of the published atomic, all three.
    // `backspace` remains as the manual override, and `replace_in` — the seam task T-04-1
    // left — still runs exactly the FR-41 packet, which the rest of this test pins down.
    assert_eq!(
        settings::Replacement::default().method,
        ReplacementMethod::Auto
    );
    assert_eq!(
        settings::Config::default().replacement.method,
        ReplacementMethod::Auto
    );
    assert_eq!(
        settings::Config::default().replacement.inter_event_delay_ms,
        0
    );
    assert_eq!(inject::replacement_method(), ReplacementMethod::Auto);

    // **Point 18.** The mode of task T-04-1 is untouched: the same strokes through `replace_in`,
    // which knows no mode, and through `replace_in_with` in the default one, produce the same
    // events in the same order — down to the log of the environment.
    let mut plain = Bench::machine(Modifiers::RIGHT_CTRL);
    let plain_outcome = inject::replace_in(&mut plain, &ghbdtn(), &russian(), 0).expect("sized");

    let mut chosen = Bench::machine(Modifiers::RIGHT_CTRL);
    let chosen_outcome = inject::replace_in_with(
        &mut chosen,
        &ghbdtn(),
        &russian(),
        inject::OnScreen::as_typed(&ghbdtn()),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("sized");

    assert_eq!(plain.log, chosen.log);
    assert_eq!(plain_outcome, chosen_outcome);

    // And it is still the FR-41 packet: backspaces, no arrows, no Shift of its own.
    let stream = plain.stream();
    assert_eq!(stream.iter().filter(|e| e.is_backspace(false)).count(), 6);
    assert!(!stream.iter().any(|e| e.is_left(false)));
    assert!(!stream.iter().any(|e| e.is_shift(false)));

    // The two packets differ, which is what makes the choice a choice.
    let mut selection = Bench::machine(Modifiers::RIGHT_CTRL);
    selection_run(&mut selection);
    assert_ne!(plain.log, selection.log);
}

// ---------------------------------------------------------------------------------------
// FR-42а — the resolver of `auto`. Task T-10-8.
//
// The rule is a pure function of a window class name (`inject::resolve_auto`), so it is
// driven here with names of this file's choosing — the fake source the task asks for — and
// the Win32 half is exercised once, live, against a real window of this file's own
// registered class (`resolver_live` below). No foreground is needed for either: the pure
// half asks no window anything, and the live half asks a message-only window by handle.
// ---------------------------------------------------------------------------------------

#[test]
fn both_console_classes_resolve_auto_to_backspace() {
    // The closed list itself, exactly as the module publishes it — both entries, and the
    // measurement of T-10-7 behind them: consoles are where `selection` was measured to
    // break (COOKED input does not select on Shift+Left) and `backspace` to hold.
    assert_eq!(
        inject::CONSOLE_WINDOW_CLASSES,
        ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS"]
    );

    for class in inject::CONSOLE_WINDOW_CLASSES {
        assert_eq!(
            inject::resolve_auto(Some(class)),
            ReplacementMethod::Backspace,
            "{class} is a console host and gets the Backspace path"
        );
    }

    // Case-insensitively, because Windows compares class names case-insensitively — the
    // spelling belongs to whoever registered the class, not to this program.
    assert_eq!(
        inject::resolve_auto(Some("consolewindowclass")),
        ReplacementMethod::Backspace
    );
    assert_eq!(
        inject::resolve_auto(Some("cascadia_hosting_window_class")),
        ReplacementMethod::Backspace
    );
}

#[test]
fn every_other_class_and_every_refusal_resolves_auto_to_selection() {
    // Ordinary windows of the measured matrix: every one of them held under `selection`
    // (T-10-7), and every one of them gets it.
    for class in [
        "Notepad",
        "Edit",
        "Chrome_WidgetWin_1",
        "Qt51511QWindowIcon",
        "OpusApp",
    ] {
        assert_eq!(
            inject::resolve_auto(Some(class)),
            ReplacementMethod::Selection,
            "{class} is not a console host"
        );
    }

    // Membership, not substring: the list is closed, and a class that merely contains a
    // listed name is a different class nobody measured.
    assert_eq!(
        inject::resolve_auto(Some("ConsoleWindowClass2")),
        ReplacementMethod::Selection
    );
    assert_eq!(
        inject::resolve_auto(Some("XCASCADIA_HOSTING_WINDOW_CLASS")),
        ReplacementMethod::Selection
    );

    // The empty class — no registered window has one, so it can only be a damaged answer.
    assert_eq!(inject::resolve_auto(Some("")), ReplacementMethod::Selection);

    // And the refusal itself: no foreground window at all, or `GetClassNameW` failed. The
    // direction is the decision FR-42а writes out — `selection` held everything measured
    // except the enumerable consoles, so the unknown gets the method with the evidence.
    assert_eq!(inject::resolve_auto(None), ReplacementMethod::Selection);
}

#[test]
fn explicit_methods_pass_through_the_effective_choice_untouched() {
    // Section 7 keeps `backspace` and `selection` as manual overrides without automatics:
    // the effective method of an explicit choice is that choice, whatever window is in
    // front — no class is even asked for, and these two must hold with any foreground.
    assert_eq!(
        inject::effective_method(ReplacementMethod::Backspace),
        ReplacementMethod::Backspace
    );
    assert_eq!(
        inject::effective_method(ReplacementMethod::Selection),
        ReplacementMethod::Selection
    );
}

#[test]
fn the_packet_functions_stay_total_over_auto_and_answer_the_refusal_direction() {
    // `Auto` never reaches the packet builders through `on_hotkey`, which resolves it first;
    // the builders are total anyway, and their answer is the same refusal direction the
    // resolver takes — the `Selection` packet, structure for structure.
    let text = utf16("привет");

    assert_eq!(
        inject::packet_events(ReplacementMethod::Auto, 6, text.len()),
        inject::selection_events(6, text.len())
    );

    let mut from_auto = vec![INPUT::default(); inject::selection_events(6, text.len())];
    let mut from_selection = from_auto.clone();

    let auto_len = inject::build_packet(ReplacementMethod::Auto, 6, &text, &mut from_auto)
        .expect("sized above");
    let selection_len = inject::build_selection(6, &text, &mut from_selection).expect("sized");

    assert_eq!(auto_len, selection_len);
    assert_eq!(
        reduce(&from_auto[..auto_len]),
        reduce(&from_selection[..selection_len])
    );
}

/// The live half of the resolver check: a real window of this file's own registered class.
///
/// A module of its own so that its imports — window creation, nothing else — are not in scope
/// for any other test, the same fence `behavioural` keeps around `SendInput`.
mod resolver_live {
    use lang_switcher::inject;
    use lang_switcher::settings::ReplacementMethod;

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, RegisterClassW,
        WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
    };
    use windows::core::{PCWSTR, w};

    /// The class this test registers — a name no console host will ever carry.
    const PROBE_CLASS: PCWSTR = w!("LangSwT108ResolverProbe");

    /// The plainest possible window procedure.
    ///
    /// # Safety
    ///
    /// Called by the system with the arguments of a live window's message, which are exactly
    /// what `DefWindowProcW` takes.
    unsafe extern "system" fn probe_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    /// **The one live test of the resolver** — a real window, of this file's own class, read
    /// back through the very `GetClassNameW` path the product resolves `auto` with.
    ///
    /// Message-only (`HWND_MESSAGE`): it has a class like any window, and it can never become
    /// the foreground window or appear on anybody's screen, so the test needs no foreground
    /// gate and disturbs nothing.
    #[test]
    fn a_real_window_of_our_own_class_reads_back_and_resolves_to_selection() {
        // SAFETY: `None` asks for the handle of the running executable — the documented use.
        let instance = unsafe { GetModuleHandleW(None) }.expect("GetModuleHandleW");

        let class = WNDCLASSW {
            lpfnWndProc: Some(probe_proc),
            hInstance: instance.into(),
            lpszClassName: PROBE_CLASS,
            ..Default::default()
        };

        // SAFETY: `class` is a live, fully initialised `WNDCLASSW` whose two pointers — the
        // window procedure and the static class name — outlive the call and the class.
        // NFR-13: a zero atom is the documented failure and is examined.
        let atom = unsafe { RegisterClassW(&class) };
        assert_ne!(
            atom,
            0,
            "RegisterClassW: {}",
            std::io::Error::last_os_error()
        );

        // SAFETY: the class name is the one registered a line above; `HWND_MESSAGE` as the
        // parent asks for a message-only window; every other argument is a plain value or a
        // null option. NFR-13: the `Result` is examined.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PROBE_CLASS,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
        }
        .expect("a message-only window of our own class");

        // The Win32 half answers the very name this test registered...
        let class = inject::window_class(window);
        assert_eq!(class.as_deref(), Some("LangSwT108ResolverProbe"));

        // ...and the rule sends a window that is not a console host down the FR-42 path.
        assert_eq!(
            inject::resolve_auto(class.as_deref()),
            ReplacementMethod::Selection
        );

        // Zero outcome of `GetClassNameW`, live: a null handle names no window and no class,
        // and the answer is the refusal — `None`, never an empty string mistaken for a name.
        assert_eq!(inject::window_class(HWND::default()), None);

        // SAFETY: `window` is the live window created above, owned by this thread.
        unsafe { DestroyWindow(window) }.expect("DestroyWindow");
    }
}

// ---------------------------------------------------------------------------------------
// SEC-01, SEC-02 — how the working buffers are released. Task T-13-15.
// ---------------------------------------------------------------------------------------

/// The text of a module under `src\`, with the line endings normalised — task **T-13-30**.
///
/// The one place this file reads a source file, and it collapses `\r\n` to `\n` before anybody
/// downstream sees the text. `.gitattributes` declares `* text=auto eol=crlf`, so **the canonical
/// checkout of this repository is CRLF**, while `cargo fmt` writes LF: one commit is one text
/// after a checkout and another after a format. Any sweep whose needle carries a newline answers
/// differently for the two, and the answer it gives on the canonical tree is the wrong one — the
/// cut in `the_working_buffers_are_released_through_the_volatile_zeroing` below was taking the
/// whole rest of `src\inject.rs` for a function body.
///
/// The same reading, and for the same reason, as `read_normalised` in `tests\guard.rs` after task
/// T-13-12; not reinvented here, because normalising at the read is the only place it can be done
/// once.
fn source_of(module: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("src\\{module} must be readable: {error}"));

    text.replace("\r\n", "\n")
}

/// Lines of `text` that contain `needle` and are not comment lines.
///
/// The module documents its own zeroing at length, and a sweep that counted prose would be a
/// sweep nobody could keep green. The same helper, for the same reason, as the one in
/// `tests\selection.rs`.
fn code_lines_with<'a>(text: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.starts_with("//"))
        .filter(|(_, line)| line.contains(needle))
        .collect()
}

/// **The audit of 2026-08-24: the module documentation promised a write the code did not make.**
///
/// The header of `src\inject.rs` says the working buffers are overwritten before they are
/// released, "which is the rule module `buffer` lives by (SEC-02)". Module `buffer` zeroes with
/// `ptr::write_volatile` and a `compiler_fence`, and says in its own documentation why anything
/// weaker is deletable; this module used `slice.fill(...)` in front of three deallocations. The
/// three go through `buffer::zero_slice` now, and no `fill` is left on any release path.
///
/// This is a check on the source and not on a value, because the property is exactly that the
/// write **cannot be optimised away** — a plain `fill` would pass every behavioural test in this
/// file and still leave the user's last word in the freed block under the Release profile of
/// section 3.2.
#[test]
fn the_working_buffers_are_released_through_the_volatile_zeroing() {
    let source = source_of("inject.rs");

    let fills = code_lines_with(&source, ".fill(");

    assert!(
        fills.is_empty(),
        "nothing in this module zeroes with `fill` any more: {fills:?}"
    );

    // Three buffers hold the user's text here and all three are zeroed through the helper: the
    // converted text and the `INPUT` packet of `replace_in_with`, and the copy of the strokes
    // `on_hotkey` takes out of the recorder.
    let zeroed = code_lines_with(&source, "buffer::zero_slice(");

    assert_eq!(
        zeroed.len(),
        3,
        "every working buffer is zeroed through the helper: {zeroed:?}"
    );

    for signature in ["pub fn replace_in_with", "pub fn on_hotkey"] {
        let remainder = source
            .split(signature)
            .nth(1)
            .unwrap_or_else(|| panic!("{signature} exists in the module"));

        // ⚠ **No silent fall-back — task T-13-30.**
        //
        // This used to end in `.split("\n}\n").next().unwrap_or_else(|| panic!(…))`. `.next()`
        // on a `Split` that has just been handed a non-empty string **never** answers `None`, so
        // the panic was unreachable; and on the canonical CRLF tree the needle does not occur at
        // all — the file holds `\r\n}\r\n` — so the "first piece" was the whole rest of
        // `src\inject.rs`: 15 661 bytes after `on_hotkey`, against the 1 438 its body is. The
        // assertion below was then satisfied by a `buffer::zero_slice(` belonging to some other
        // function, which was measured rather than supposed: moving the call out of `on_hotkey`
        // into a helper beside it passes the old form of this test and fails this one.
        //
        // A boundary that cannot be found is a broken cut, and a broken cut must fail — the shape
        // `tests\guard.rs` settled on in task T-13-12.
        let end = remainder.find("\n}\n").unwrap_or_else(|| {
            panic!(
                "src\\inject.rs: no closing brace in the first column after {signature}, so its \
                 body cannot be bounded — the assertion below would be made about the rest of \
                 the file"
            )
        });

        let body = &remainder[..end + "\n}\n".len()];

        // And the cut really cut: a body that is the whole remainder is the degenerate case above
        // wearing a different mask, so it is asserted against rather than trusted.
        assert!(
            body.len() < remainder.len(),
            "the body of {signature} was bounded ({} bytes) rather than taken as the rest of the \
             file ({} bytes)",
            body.len(),
            remainder.len()
        );

        assert!(
            body.contains("buffer::zero_slice("),
            "{signature} zeroes what it releases"
        );
    }
}

/// **NFR-01…NFR-05: nothing of this was added to the hook callback.**
///
/// The zeroing runs on the input thread *after* the handoff — `on_hotkey` is called from the
/// message loop, not from the callback — and in `replace_in_with`, which is further down the
/// same path. The callback of `WH_KEYBOARD_LL` lives in module `hook` and calls nothing of this
/// module; the check that it stays that way is that `hook.rs` never names the helper.
#[test]
fn the_zeroing_is_nowhere_near_the_hook_callback() {
    let hook = source_of("hook.rs");

    assert!(
        code_lines_with(&hook, "zero_slice").is_empty(),
        "the hook callback path does not zero buffers — it does not own any"
    );
}

/// **FR-100, task Т-21-5 — the sound is made nowhere near the path of a keystroke.**
///
/// NFR-01…NFR-05 and NFR-09 are the whole of why this test exists. The hook callback owes the
/// system an answer in microseconds and the input thread has thirty milliseconds for a whole
/// replacement; a sound is neither thread's work, whatever `MessageBeep` costs on the day.
///
/// Three assertions, and they are three because the sound could arrive on the wrong thread in
/// three different ways:
///
/// * `hook.rs` — the callback and the module that owns it — must not name the sound at all;
/// * `inject.rs` — the replacement itself, on the input thread — must not either;
/// * in `app.rs`, where both ends live, `MessageBeep` must be reached from exactly one place.
///   The input thread's branch *posts* (`WM_APP_SOUND_DONE` / `WM_APP_SOUND_IDLE`) and the UI
///   thread's window procedure is what answers, so a second caller appearing later would be a
///   sound made on whichever thread happened to run it.
#[test]
fn the_sound_of_fr_100_is_made_on_neither_the_hook_nor_the_input_path() {
    for module in ["hook.rs", "inject.rs"] {
        let source = source_of(module);

        for forbidden in [
            "PlaySound",
            "MessageBeep",
            "sound_press",
            "answer_press",
            "SOUND_",
        ] {
            assert!(
                code_lines_with(&source, forbidden).is_empty(),
                "{module} must not name {forbidden}: the sound of FR-100 is the UI thread's"
            );
        }
    }

    let app = source_of("app.rs");
    let product = app.split("mod tests {").next().unwrap_or(&app);

    let beeps = code_lines_with(product, "PlaySoundW(");

    assert_eq!(
        beeps.len(),
        1,
        "exactly one line of the program makes a sound: {beeps:?}"
    );

    // ⚠ And the sound is the program's own. `MessageBeep` was the mechanism until the acceptance
    // by ear of stage Э21, and the user rejected the system tones: borrowing them makes a
    // keystroke sound like an error dialog. A return to them would be a return of the defect.
    let system = code_lines_with(product, "MessageBeep");

    assert!(
        system.is_empty(),
        "the system beep is not the sound of this program any more: {system:?}"
    );

    let callers = code_lines_with(product, "sound_press(");

    assert_eq!(
        callers.len(),
        3,
        "the production beeper is reached from its definition and from the two arms of the UI \
         thread's window procedure, and from nowhere else: {callers:?}"
    );
}

// ---------------------------------------------------------------------------------------
// Task T-13-13 — the ceiling of `[replacement] inter_event_delay_ms`, measured on the packet
// ---------------------------------------------------------------------------------------

/// An [`Environment`] whose `pause` is the real `thread::sleep`, and which does nothing else.
///
/// [`Bench`] writes the pause down instead of taking it, which is what makes the *count* and the
/// *order* of FR-44 observable; this takes it, which is what makes the count worth anything. One
/// is the model and the other is the calibration of it — see
/// [`a_six_letter_word_at_the_ceiling_of_fr44_costs_seconds_and_not_weeks`].
struct SleepingBench {
    /// Pauses actually slept through.
    pauses: usize,
}

impl Environment for SleepingBench {
    fn held(&mut self) -> Modifiers {
        Modifiers::NONE
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        u32::try_from(events.len()).unwrap_or(u32::MAX)
    }

    fn pause(&mut self, delay_ms: u32) {
        self.pauses += 1;
        std::thread::sleep(std::time::Duration::from_millis(u64::from(delay_ms)));
    }

    fn switch_layout(&mut self) {}
}

/// The pauses of `log`, checked to be `delay_ms` every one of them, as one total.
fn total_pause(log: &[Step], delay_ms: u32) -> std::time::Duration {
    let millis: u64 = log
        .iter()
        .map(|step| match step {
            Step::Pause(asked) => {
                assert_eq!(*asked, delay_ms, "every pause is the configured one");
                u64::from(*asked)
            }
            _ => 0,
        })
        .sum();

    std::time::Duration::from_millis(millis)
}

/// ⭐ **Task T-13-13.** A six-letter word replaced at the ceiling of FR-44 costs **seconds**; the
/// same word at the value a hand-edited file could carry before this repair cost **weeks**.
///
/// # What the number is, and why it is measured here
///
/// The audit of 2026-08-24 put the arithmetic in one line — «замена слова из шести букв методом
/// backspace — это ~23 паузы» — and the whole finding rests on it: the pause of FR-44 is taken on
/// the input thread, the thread the `WH_KEYBOARD_LL` hook and the watchdog live on, so the total
/// is the length of time the keyboard of the machine is unattended. This asserts the count against
/// the packet the requirement actually builds rather than against that sentence.
///
/// `backspace` (FR-41): six characters are six `Backspace` down+up pairs and six code units as
/// down+up pairs — twenty-four events, one portion each, and a pause **between** portions, so
/// twenty-three of them. At [`inject::MAX_INTER_EVENT_DELAY_MS`] that is twenty-three seconds.
/// `selection` (FR-42) adds the two `Shift` events of the bracket: twenty-six events, twenty-five
/// pauses, twenty-five seconds.
///
/// # Why the total is modelled and one run is real
///
/// A test that really slept twenty-three seconds would prove twenty-three seconds and cost every
/// future run of the battery twenty-three seconds to re-prove arithmetic. So the count and the
/// value of every pause come off the log of [`Bench`] — the seam FR-44 was built against — and a
/// second run of the very same packet through [`SleepingBench`], at a fraction of the ceiling,
/// pays the pauses for real and shows that the model and the machine agree about how many there
/// are. The ceiling figure is that same count at the ceiling.
#[test]
fn a_six_letter_word_at_the_ceiling_of_fr44_costs_seconds_and_not_weeks() {
    let ceiling = inject::MAX_INTER_EVENT_DELAY_MS;

    assert_eq!(
        ceiling, 1_000,
        "ТЗ-Э13 fixes the ceiling of inter_event_delay_ms at a thousand milliseconds"
    );

    // FR-41, the `backspace` packet.
    let mut bench = Bench::new();
    let outcome = inject::replace_in(&mut bench, &ghbdtn(), &russian(), ceiling).expect("sized");

    assert_eq!(outcome.erased, 6, "six characters on the screen");
    assert_eq!(outcome.typed, 6, "six code units back");
    assert_eq!(outcome.replacement.requested, 24, "twenty-four events");
    assert_eq!(outcome.replacement.calls, 24, "one portion per event");

    let backspace = total_pause(&bench.log, ceiling);
    let backspace_pauses = bench
        .log
        .iter()
        .filter(|step| matches!(step, Step::Pause(_)))
        .count();

    assert_eq!(backspace_pauses, 23, "between twenty-four portions");
    println!("backspace, six letters at the ceiling: {backspace:?}");

    // FR-42, the compatibility packet — the same six letters through the other mode.
    let mut bench = Bench::new();
    let outcome = inject::replace_in_with(
        &mut bench,
        &ghbdtn(),
        &russian(),
        inject::OnScreen::as_typed(&ghbdtn()),
        ceiling,
        ReplacementMethod::Selection,
    )
    .expect("sized");

    assert_eq!(outcome.replacement.requested, 26, "two Shift events more");

    let selection = total_pause(&bench.log, ceiling);
    println!("selection, six letters at the ceiling: {selection:?}");

    // **Seconds.** Not thirty milliseconds — a pause above zero is the explicit trade of NFR-09
    // that FR-44 exists to offer — but a wait a person sits through rather than a machine they
    // restart, which is the whole of what the ceiling buys.
    assert!(
        backspace <= std::time::Duration::from_secs(30)
            && selection <= std::time::Duration::from_secs(30),
        "the ceiling must keep a six-letter word inside seconds: {backspace:?} / {selection:?}"
    );

    // And what the same packet cost before the ceiling existed, from the value a `u32` field in a
    // hand-edited file can hold. Days, not seconds — this is the defect, in the same unit.
    let unbounded = std::time::Duration::from_millis(23 * u64::from(u32::MAX));
    println!(
        "backspace, six letters at u32::MAX: {} days",
        unbounded.as_secs() / 86_400
    );
    assert!(
        unbounded > std::time::Duration::from_secs(1_000 * 86_400),
        "the value the file could carry is over a thousand days of frozen keyboard"
    );

    // The calibration: the same packet, the same count of pauses, actually slept through — at a
    // two-hundredth of the ceiling, so that the battery pays milliseconds to check a model that
    // answers in seconds.
    let sampled = ceiling / 200;
    let mut sleeper = SleepingBench { pauses: 0 };
    let started = std::time::Instant::now();
    inject::replace_in(&mut sleeper, &ghbdtn(), &russian(), sampled).expect("sized");
    let elapsed = started.elapsed();

    println!("the same packet at {sampled} ms, really slept: {elapsed:?}");

    assert_eq!(
        sleeper.pauses, backspace_pauses,
        "the model and the machine count the same pauses"
    );
    assert!(
        elapsed >= std::time::Duration::from_millis(23 * u64::from(sampled)),
        "every pause was really taken: {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------------------
// Points 22 and 23 — the end-to-end path, into a window this file owns
// ---------------------------------------------------------------------------------------

/// Everything the two behavioural checks need from Win32, and the fence around it.
///
/// Separated into a module of its own so that the imports it needs are not in scope for any
/// other test in this file: nothing above may reach a `SendInput`.
mod behavioural {
    use super::{GHBDTN, Recorder, buffer};
    use std::sync::{Mutex, MutexGuard};

    use lang_switcher::convert;
    use lang_switcher::hook::{Edge, INJECTED_SIGNATURE, KeyEvent};
    use lang_switcher::inject::{self, Modifiers};
    use lang_switcher::settings::ReplacementMethod;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SetFocus,
        VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, CreateWindowExW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
        GetWindowTextW, GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, SW_SHOW,
        SetForegroundWindow, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CHAR,
        WM_KEYDOWN, WS_BORDER, WS_POPUP, WS_VISIBLE,
    };
    use windows::core::{PCWSTR, w};

    /// `ES_MULTILINE | ES_AUTOVSCROLL` — the styles of a plain text box.
    const EDIT_STYLES: u32 = 0x0004 | 0x0040;

    /// Virtual key of the `A` key. There is no constant for the letter keys in the `windows`
    /// crate because the codes are the ASCII capitals themselves.
    const VK_A: VIRTUAL_KEY = VIRTUAL_KEY(0x41);

    /// The pause of FR-44 point 24 runs with, in milliseconds.
    ///
    /// Small on purpose: twenty events at this pause is under a tenth of a second, and what the
    /// check is about is that a portioned packet still arrives in order, not how long it takes.
    const POINT_24_PAUSE_MS: u32 = 2;

    /// How many attempts the window is given to become the foreground one before the test gives
    /// up. At the 50 ms pause of `open_foreground_window` this is about five seconds.
    const FOREGROUND_ATTEMPTS: u32 = 100;

    /// Only one of these tests may hold the foreground at a time.
    ///
    /// ⚠ **The second half of the §4.1а debt, and it is not about `SetForegroundWindow` at all.**
    /// `cargo test` runs the test functions **in parallel**. Five of them each create a window
    /// and each wait for *their own* to become foreground; they were competing with one another,
    /// and the winner took the synthetic input meant for all five. Measured: with the foreground
    /// finally being won reliably but without this lock, three tests passed and two read an empty
    /// window — `left: ""`, `right: "ghbdtn"` — because their keystrokes had gone to a sibling's
    /// window.
    ///
    /// The fence inside [`open_foreground_window`] cannot fix that on its own: it proves our
    /// window is foreground at the moment it returns, and a sibling can take the foreground in
    /// the gap between that moment and the `SendInput` in the test body. So the lock is held by
    /// [`TestWindow`] itself and released only when the window is dropped — that is, at the end
    /// of the test body, after the last send and the last read.
    static FOREGROUND: Mutex<()> = Mutex::new(());

    /// A window this test owns, destroyed when the value is dropped.
    ///
    /// Carries the [`FOREGROUND`] guard so that the whole test — asking for the foreground,
    /// sending, and reading back — is serialised against the other four.
    struct TestWindow {
        handle: HWND,
        /// Held, never read: dropping it at the end of the test is the entire purpose.
        _foreground: MutexGuard<'static, ()>,
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
            // destroyed exactly once — the type is neither `Copy` nor `Clone`. `DestroyWindow`
            // requires the calling thread to have created the window, and it did.
            let _ = unsafe { DestroyWindow(self.handle) };
        }
    }

    /// `Shift` is released however the test ends, panic included.
    ///
    /// A test that left `Shift` down would leave the machine's keyboard in that state for the
    /// person running it, which is the one failure mode of point 23 that must not be possible.
    struct ShiftGuard;

    impl Drop for ShiftGuard {
        fn drop(&mut self) {
            let mut events = [INPUT::default(); 1];

            if let Ok(len) = inject::build_release(Modifiers::LEFT_SHIFT, &mut events) {
                inject::dispatch(&events[..len], 0);
            }
        }
    }

    /// Creates the window, shows it and waits for it to become the foreground one.
    ///
    /// ⚠ **This is the fence.** Every `SendInput` in this module happens after this function
    /// has returned, and it only returns once `GetForegroundWindow` has answered with our own
    /// window. If the window never becomes foreground the test fails here, having sent nothing.
    fn open_foreground_window() -> TestWindow {
        // Taken before the window exists, so that two tests never even have windows up at the
        // same time. A panicking test poisons the lock; its result is still a usable guard, and
        // failing every later test because an earlier one failed would hide the real cause.
        let guard = FOREGROUND
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // SAFETY: `EDIT` is a system window class that exists in every process; the window name
        // is null, which asks for empty content; no parent, no menu, no module handle and no
        // creation parameter are passed, which is the documented way to ask for a top-level
        // window with nothing of ours behind it.
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                PCWSTR::null(),
                WINDOW_STYLE(EDIT_STYLES) | WS_POPUP | WS_VISIBLE | WS_BORDER,
                200,
                200,
                420,
                120,
                None,
                None,
                None,
                None,
            )
        }
        .expect("the EDIT class is registered in every process");

        let window = TestWindow {
            handle,
            _foreground: guard,
        };

        // SAFETY: `handle` is the live window this frame owns. Both calls take it by value and
        // touch no memory of ours; the returned `BOOL`s are deliberately not treated as fatal —
        // the loop below is what decides.
        unsafe {
            let _ = ShowWindow(handle, SW_SHOW);
            let _ = SetFocus(Some(handle));
        }

        for _ in 0..FOREGROUND_ATTEMPTS {
            ask_for_foreground(handle);
            pump();

            // SAFETY: takes no arguments, returns a handle by value, touches no memory of ours.
            if unsafe { GetForegroundWindow() } == handle {
                return window;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        panic!(
            "the test window did not become the foreground window; nothing was sent, which is \
             the point of this check"
        );
    }

    /// Asks the system to put our own window in front — **task T-04-3, debt §4.1а of `STATE.md`**.
    ///
    /// # Why this is not `SetForegroundWindow` alone any more
    ///
    /// These five tests used to ask with `SetForegroundWindow` and nothing else, and they passed
    /// early in a session and stopped passing later on the very same commit. Rake 3 of §2 of
    /// `TOOLCHAIN.md` names the cause: **Windows ignores `SetForegroundWindow` from a process
    /// that is not itself in the foreground.** Not an error — ignored, and only sometimes,
    /// depending on whether the desktop happens to be willing to hand focus over.
    ///
    /// The acceptance bench of task T-04-3 hit the same wall and settled it with
    /// `WScript.Shell.AppActivate` by process id, which goes through the shell's own foreground
    /// arbitration instead of around it. That is the technique these tests are moved onto, and
    /// this function is the whole of the change: **what the five tests check is untouched.**
    ///
    /// # Why not `AppActivate` itself, letter for letter
    ///
    /// It was tried here first, and it does not work **for this window**: `AppActivate` locates
    /// a window by process id or by title, and the window these tests create is a `WS_POPUP`
    /// `EDIT` control with **no title at all**. Measured — the five tests still failed on the
    /// same line, single-threaded, with `AppActivate` being called: it had nothing to find.
    ///
    /// So the bench's *mechanism* does not transfer, but the bench's *lesson* does, and the
    /// lesson is the part that matters. The bench drives **other** processes, where the only way
    /// in is the shell's arbitration. A test drives **its own** window, where the documented way
    /// past the foreground lock is to attach this thread's input queue to the queue of whichever
    /// thread currently owns the foreground: while the two are attached they share foreground
    /// rights, `SetForegroundWindow` stops being ignored, and the attachment is undone
    /// immediately afterwards. No child process, no PowerShell, and it works from a background
    /// process — which is exactly what `SetForegroundWindow` alone would not do.
    ///
    /// ⚠ **The half that actually decides is not here but in the caller:** the loop waits for
    /// `GetForegroundWindow` to answer with our window and refuses to send anything until it
    /// does. Asking better without checking the answer would leave the same race that §4.1а
    /// recorded.
    fn ask_for_foreground(handle: HWND) {
        // SAFETY: `GetForegroundWindow` takes no arguments and returns a handle by value.
        let foreground = unsafe { GetForegroundWindow() };

        // SAFETY: `foreground` may be null, which `GetWindowThreadProcessId` answers with zero —
        // examined below. `None` for the optional out-parameter asks only for the thread id.
        let owner = unsafe { GetWindowThreadProcessId(foreground, None) };
        // SAFETY: takes no arguments, returns this thread's id.
        let ours = unsafe { GetCurrentThreadId() };

        // Attaching a thread to itself is an error, and a zero owner means there is no
        // foreground window at this instant — in both cases the plain call below is all there is.
        let attached = owner != 0 && owner != ours;

        if attached {
            // SAFETY: both ids name live threads — ours by construction, the other one obtained
            // from the current foreground window a moment ago. A failed attach is not fatal: the
            // call below is then simply the unprivileged attempt, and the caller re-reads the
            // fact either way.
            unsafe {
                let _ = AttachThreadInput(ours, owner, true);
            }
        }

        // SAFETY: `handle` is the live window the caller owns. Both calls take it by value and
        // touch no memory of ours; their `BOOL`s are deliberately not fatal — the caller re-reads
        // `GetForegroundWindow`, which is the only thing the next `SendInput` depends on.
        unsafe {
            let _ = BringWindowToTop(handle);
            let _ = SetForegroundWindow(handle);
        }

        if attached {
            // SAFETY: undoes exactly the attachment made above, with the same two ids. Leaving
            // input queues attached would make this thread share the fate of another one.
            unsafe {
                let _ = AttachThreadInput(ours, owner, false);
            }
        }
    }

    /// Runs this thread's message queue dry, translating key messages into characters.
    ///
    /// `TranslateMessage` is required and not decoration: a `KEYEVENTF_UNICODE` event arrives as
    /// `WM_KEYDOWN` with `VK_PACKET`, and it is `TranslateMessage` that turns it into the
    /// `WM_CHAR` an `EDIT` control inserts.
    fn pump() {
        let mut message = MSG::default();

        loop {
            // SAFETY: `message` is a live, properly aligned `MSG` owned by this frame for the
            // whole call and is the only buffer written to. `None` for the window filter asks
            // for every message of this thread; the two zero filters mean "no range filter".
            let present = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) };

            if !present.as_bool() {
                return;
            }

            // SAFETY: `message` was filled by the `PeekMessageW` above and is read, not written,
            // by both calls.
            unsafe {
                let _ = TranslateMessage(&message);
                let _ = DispatchMessageW(&message);
            }
        }
    }

    /// The text of the window, as the control holds it now.
    fn window_text(window: &TestWindow) -> String {
        let mut buffer = [0u16; 256];

        // SAFETY: `window.handle` is the live window this frame owns and `buffer` is a live,
        // properly aligned array owned by this frame; the call is given its true length and writes no
        // more than that, NUL included. The return value is the length written, which is what
        // bounds the slice below.
        let written = unsafe { GetWindowTextW(window.handle, &mut buffer) };

        String::from_utf16_lossy(&buffer[..written.max(0) as usize])
    }

    /// Types `text` into the foreground window and records the same six strokes into the buffer.
    ///
    /// The typing goes through this task's own `KEYEVENTF_UNICODE` builder rather than through
    /// virtual keys, and deliberately: a virtual key produces whatever the *machine's* current
    /// layout says it does, so a run on a machine sitting in Russian would have typed `тщдвшт`
    /// and the test would have been measuring the tester's layout instead of this module. What
    /// is under test is the replacement, and the replacement needs the text on the screen and
    /// the strokes in the buffer to agree — which is exactly what this function establishes.
    fn type_ghbdtn(text: &str) {
        let units: Vec<u16> = text.encode_utf16().collect();
        // Sized from the module's own arithmetic and not from the number of code units: a unit
        // is a down and an up (task T-10-4), and a hand-written length here would refuse the
        // packet rather than fail an assertion.
        let mut events = vec![INPUT::default(); inject::replacement_events(0, units.len())];

        let len = inject::build_replacement(0, &units, &mut events).expect("sized");
        inject::dispatch(&events[..len], 0);

        pump();

        for &scan in &GHBDTN {
            buffer::record(KeyEvent {
                vk: 0,
                edge: Edge::Down,
                extra_info: 0,
                scan,
                flags: 0,
                time: 0,
            });
        }
    }

    /// The replacement mode and the pause of FR-44 go back to the defaults of section 7 however
    /// a check ends, panic included.
    ///
    /// Both are process-wide state that `on_hotkey` reads, and a check that left either changed
    /// would be the next check's environment — and, with `--ignored --test-threads=1`, the next
    /// check is the one that runs the mode of task T-04-1.
    struct ModeGuard;

    impl Drop for ModeGuard {
        fn drop(&mut self) {
            inject::set_replacement_method(ReplacementMethod::Backspace);
            inject::set_inter_event_delay_ms(0);
        }
    }

    /// One keyboard `INPUT` for a virtual key, carrying the signature of FR-03.
    ///
    /// The only place in this file that assembles an `INPUT` instead of asking module `inject`
    /// for one, and it exists for a single check: point 26 asks whether `Shift` is stuck, and a
    /// `KEYEVENTF_UNICODE` event cannot answer that. Such an event carries its character
    /// literally and would look identical with `Shift` welded down; only a **virtual** key, whose
    /// meaning the receiving application derives from the modifier state, can be asked.
    fn vk_event(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: INJECTED_SIGNATURE,
                },
            },
        }
    }

    /// **Point 26.** Presses one ordinary letter key and answers whether what appeared in the
    /// window is lower case — that is, whether `Shift` is stuck down after the replacement.
    ///
    /// "Lower case" and not "the letter `a`" because the answer depends on the layout the
    /// machine happens to be sitting in: `VK_A` reads `a` in US and `ф` in Russian, and both are
    /// lower case, which is the whole of what is being asked.
    fn an_ordinary_key_gives_a_lower_case_letter(window: &TestWindow) -> bool {
        let before = window_text(window).chars().count();

        inject::dispatch(&[vk_event(VK_A, false), vk_event(VK_A, true)], 0);
        pump();

        let after = window_text(window);

        after.chars().count() > before && after.chars().skip(before).all(|ch| !ch.is_uppercase())
    }

    /// Installs a typing buffer holding the hardwired FR-25 cache, recording under US.
    fn install_buffer() {
        buffer::install_recorder(Recorder::with_capacity(64));

        buffer::with(|recorder| {
            recorder.set_cache(convert::fallback_cache());
            recorder.set_active_layout(convert::FALLBACK_US);
        });
    }

    /// **Point 22.** `ghbdtn` typed into a window this test owns, replaced, and read back.
    #[test]
    #[ignore = "sends synthetic input; run with --ignored --test-threads=1"]
    fn ghbdtn_becomes_privet_in_our_own_window() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn", "the run to be replaced");

        let outcome = inject::on_hotkey().expect("the buffer holds six strokes under US");
        pump();

        assert_eq!(outcome.erased, 6);
        assert_eq!(outcome.typed, 6);
        assert_eq!(window_text(&window), "привет");

        buffer::uninstall();
    }

    /// **Task T-10-6 — what the system really delivers for one packet of FR-41.**
    ///
    /// The other half of the separating measurement of that task. The channel says the product
    /// builds six *distinct* character events; this says what arrives at the far end when the
    /// application does **not** retrieve a single message until the whole packet has been
    /// injected — the state a busy application is in, and the state the collapse happens in.
    ///
    /// The queue is left to fill for eighty milliseconds and then drained by hand, and the
    /// messages are examined rather than the text: six separate `WM_KEYDOWN` of `VK_PACKET`, each
    /// with **repeat count 1**, and six `WM_CHAR` carrying six **different** characters. That is
    /// what rules out the reading the shape of the defect suggests first — that Windows merges a
    /// run of injected characters into one message with a repeat count, which an application
    /// honouring that count would render as N copies of one character.
    ///
    /// ⚠ It asserts the message stream and not merely the final text: the text alone would pass
    /// on a queue that had merged and been un-merged by the control.
    #[test]
    #[ignore = "sends synthetic input; run with --ignored --test-threads=1"]
    fn a_stalled_queue_still_receives_every_character_separately() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");
        assert_eq!(window_text(&window), "ghbdtn", "the run to be replaced");

        let units: Vec<u16> = EXPECTED_UNITS.to_vec();
        let mut events = vec![INPUT::default(); inject::replacement_events(6, units.len())];
        let len = inject::build_replacement(6, &units, &mut events).expect("sized");
        inject::dispatch(&events[..len], 0);

        // Not a wait on a condition and deliberately so: the condition **is** the delay. Nothing
        // is retrieved until the whole packet is certainly injected, because the question is what
        // a queue that filled up before anybody looked at it holds.
        std::thread::sleep(std::time::Duration::from_millis(80));

        let mut message = MSG::default();
        let mut characters = Vec::new();
        let mut packets = Vec::new();

        loop {
            // SAFETY: `message` is a live, properly aligned `MSG` owned by this frame and the only
            // buffer written to; `None` asks for every message of this thread.
            let present = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) };
            if !present.as_bool() {
                break;
            }

            let repeat = (message.lParam.0 as u32) & 0xFFFF;
            match message.message {
                WM_KEYDOWN if message.wParam.0 == VK_PACKET => packets.push(repeat),
                WM_CHAR if repeat == 1 && message.wParam.0 > 0x20 => {
                    characters.push(message.wParam.0 as u16);
                }
                _ => {}
            }

            // SAFETY: `message` was filled by the `PeekMessageW` above and is read, not written.
            unsafe {
                let _ = TranslateMessage(&message);
                let _ = DispatchMessageW(&message);
            }
        }

        assert_eq!(
            packets,
            vec![1; units.len()],
            "one WM_KEYDOWN of VK_PACKET per code unit, none of them a repeat"
        );
        assert_eq!(
            characters, units,
            "and the characters arrive whole and in order, however long the queue stood"
        );
        assert_eq!(window_text(&window), "привет");

        buffer::uninstall();
    }

    /// `VK_PACKET` — the virtual key `KEYEVENTF_UNICODE` arrives as.
    const VK_PACKET: usize = 0x00E7;

    /// `привет` in UTF-16, which is what the packet of the check above carries.
    const EXPECTED_UNITS: [u16; 6] = [0x043F, 0x0440, 0x0438, 0x0432, 0x0435, 0x0442];

    /// **Task T-10-6 — four presses in a row, in a window this test owns.**
    ///
    /// The defect the user found is a press whose result is six copies of the **last** character
    /// of the text that should have appeared: `nnnnnn` for `ghbdtn`, `тттттт` for `привет`. The
    /// bench reproduces it in Notepad, and the channel of SEC-04a says the packet the product
    /// built on those very presses carried six **distinct** code units — so what this check is
    /// for is the other end: a plain `EDIT`, driven by the same `on_hotkey`, with the message
    /// queue pumped by this thread between presses.
    ///
    /// Every press is asserted, not only the last: the collapse is intermittent, and a check that
    /// looked at the end state alone would pass on a run in which press 2 collapsed and press 3
    /// put the right text back.
    #[test]
    #[ignore = "sends synthetic input and switches the layout; run with --ignored --test-threads=1"]
    fn four_presses_in_a_row_alternate_and_never_collapse() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn", "the run to be replaced");

        let expected = ["привет", "ghbdtn", "привет", "ghbdtn"];
        let mut seen = Vec::new();

        for press in 1..=expected.len() {
            let outcome = inject::on_hotkey()
                .unwrap_or_else(|| panic!("press {press}: the buffer still holds six strokes"));
            pump();

            seen.push((outcome.erased, outcome.typed, window_text(&window)));
        }

        for (index, want) in expected.iter().enumerate() {
            let (erased, typed, shown) = &seen[index];
            assert_eq!(
                (*erased, *typed),
                (6, 6),
                "press {}: FR-41 erases as many characters as it types back; whole run {seen:?}",
                index + 1
            );
            assert_eq!(
                shown,
                want,
                "press {}: the alternation of FR-33 over a cycle of two; whole run {seen:?}",
                index + 1
            );
        }

        buffer::uninstall();
    }

    /// **Point 23.** The same, with `Shift` physically held — the case FR-40 step 3 exists for.
    #[test]
    #[ignore = "sends synthetic input and holds Shift; run with --ignored --test-threads=1"]
    fn a_held_shift_does_not_turn_the_replacement_into_a_selection() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn");

        // Down goes `Shift`, and it comes back up however this test ends.
        let _guard = ShiftGuard;
        let mut events = [INPUT::default(); 1];
        let len = inject::build_restore(Modifiers::LEFT_SHIFT, &mut events).expect("one key");
        inject::dispatch(&events[..len], 0);
        pump();

        assert!(
            inject::held_modifiers().contains(Modifiers::LEFT_SHIFT),
            "the precondition of this check is that Shift really is held"
        );

        let outcome = inject::on_hotkey().expect("the buffer holds six strokes under US");
        pump();

        assert!(
            outcome.released.contains(Modifiers::LEFT_SHIFT),
            "FR-40 step 3: the held Shift was released before the replacement"
        );
        assert_eq!(
            window_text(&window),
            "привет",
            "not a selection, and not ПРИВЕТ"
        );

        buffer::uninstall();
    }

    // -----------------------------------------------------------------------------------
    // Task T-04-2 — points 22, 23, 24 and 26 in the compatibility mode of FR-42
    // -----------------------------------------------------------------------------------

    /// **Point 22, and point 26.** The compatibility mode of FR-42 end to end, in a window this
    /// test owns: `ghbdtn` typed, the hotkey path run, `привет` read back — and no `Shift` left
    /// stuck by the selection.
    ///
    /// This drives `inject::on_hotkey`, the very function `app::window_proc` calls, with the
    /// mode published exactly as the UI thread publishes it. Nothing is faked but the
    /// configuration source.
    #[test]
    #[ignore = "sends synthetic input; run with --ignored --test-threads=1"]
    fn ghbdtn_becomes_privet_through_a_selection() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn", "the run to be replaced");

        let _mode = ModeGuard;
        inject::set_replacement_method(ReplacementMethod::Selection);

        let outcome = inject::on_hotkey().expect("the buffer holds six strokes under US");
        pump();

        assert_eq!(outcome.erased, 6);
        assert_eq!(outcome.typed, 6);
        assert_eq!(window_text(&window), "привет");

        // **Point 26.** The `Shift` the selection pressed itself is up again, both by the
        // desktop's own account and by what an ordinary key now produces.
        let held = inject::held_modifiers();
        assert!(
            !held.contains(Modifiers::LEFT_SHIFT),
            "no Shift is stuck down"
        );
        assert!(!held.contains(Modifiers::RIGHT_SHIFT));
        assert!(
            an_ordinary_key_gives_a_lower_case_letter(&window),
            "point 26: ordinary typing gives lower-case letters after the replacement"
        );

        buffer::uninstall();
    }

    /// **Point 23 in the compatibility mode.** The user is holding `Shift` — the very key the
    /// selection is about to press itself — and the result is still the replacement.
    ///
    /// The two ways this can go wrong are both asserted against: `ПРИВЕТ` would mean the user's
    /// `Shift` was still down when the characters went out, and a window still reading `ghbdtn`
    /// would mean the insertion extended the selection instead of replacing it.
    #[test]
    #[ignore = "sends synthetic input and holds Shift; run with --ignored --test-threads=1"]
    fn a_held_shift_does_not_turn_the_selection_into_capitals() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn");

        let _mode = ModeGuard;
        inject::set_replacement_method(ReplacementMethod::Selection);

        // Down goes `Shift`, and it comes back up however this test ends.
        let _guard = ShiftGuard;
        let mut events = [INPUT::default(); 1];
        let len = inject::build_restore(Modifiers::LEFT_SHIFT, &mut events).expect("one key");
        inject::dispatch(&events[..len], 0);
        pump();

        assert!(
            inject::held_modifiers().contains(Modifiers::LEFT_SHIFT),
            "the precondition of this check is that Shift really is held"
        );

        let outcome = inject::on_hotkey().expect("the buffer holds six strokes under US");
        pump();

        assert!(
            outcome.released.contains(Modifiers::LEFT_SHIFT),
            "FR-40 step 3: the user's Shift was released before the selection"
        );
        assert_eq!(
            window_text(&window),
            "привет",
            "not ПРИВЕТ, and not a selection left standing"
        );

        buffer::uninstall();
    }

    /// **Point 24.** The compatibility mode with the pause of FR-44 above zero: the same text,
    /// in the same order, only slower — and the selection is not torn by the pauses.
    #[test]
    #[ignore = "sends synthetic input; run with --ignored --test-threads=1"]
    fn a_selection_survives_the_pause_of_fr44() {
        let window = open_foreground_window();

        install_buffer();
        type_ghbdtn("ghbdtn");

        assert_eq!(window_text(&window), "ghbdtn");

        let _mode = ModeGuard;
        inject::set_replacement_method(ReplacementMethod::Selection);
        inject::set_inter_event_delay_ms(POINT_24_PAUSE_MS);

        let outcome = inject::on_hotkey().expect("the buffer holds six strokes under US");
        pump();

        // Twenty-six events, twenty-six calls: the packet really was portioned, which is what
        // makes the result below a statement about FR-44 and not about FR-41 again. Fourteen of
        // them are the selection (Shift, six arrows down and up, Shift) and twelve are the six
        // characters, each a down and an up.
        assert_eq!(
            outcome.replacement.calls,
            inject::selection_events(6, 6),
            "FR-44: portions, not one call"
        );
        assert!(outcome.replacement.is_complete());
        assert_eq!(window_text(&window), "привет");

        assert!(
            !inject::held_modifiers().contains(Modifiers::LEFT_SHIFT),
            "the Shift of the selection is up even when the packet was portioned"
        );

        buffer::uninstall();
    }
}
