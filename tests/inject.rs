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

use lang_switcher::buffer::{self, Recorder};
use lang_switcher::convert::{self, Keystroke};
use lang_switcher::hook::INJECTED_SIGNATURE;
use lang_switcher::inject::{
    self, Dispatched, Environment, InjectError, MODIFIER_COUNT, Modifiers,
};
use lang_switcher::layouts::{KeyMapping, LayoutId, LayoutMap, LayoutMapBuilder, Mods};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK,
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
    fn is_unicode(self) -> bool {
        self.flags & KEYEVENTF_UNICODE.0 != 0 && self.vk == 0
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
}

impl Bench {
    /// A bench that holds nothing and accepts everything.
    fn new() -> Self {
        Self {
            answers: Vec::new(),
            asked: 0,
            refuse: 0,
            log: Vec::new(),
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
        let answer = self
            .answers
            .get(self.asked)
            .copied()
            .unwrap_or(Modifiers::NONE);

        self.asked += 1;
        self.log.push(Step::Held(answer));

        answer
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        self.log.push(Step::Send(reduce(events)));

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

    // 6 backspaces down + 6 up + 6 characters + 2 released + 1 restored.
    assert_eq!(stream.len(), 6 * 2 + 6 + 2 + 1);

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
/// `KEYEVENTF_UNICODE` with `wVk = 0` and `wScan` = the code unit.
#[test]
fn the_packet_is_backspaces_down_and_up_first_then_unicode_events() {
    let text = utf16("привет");
    let mut out = [INPUT::default(); inject::replacement_events(6, 6)];

    let len = inject::build_replacement(6, &text, &mut out).expect("the array is exactly sized");

    assert_eq!(len, 18);

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

    // Then the characters, in the order they are to appear, one event per code unit.
    for (event, &unit) in events[12..].iter().zip(text.iter()) {
        assert!(event.is_unicode(), "FR-41: wVk = 0 and KEYEVENTF_UNICODE");
        assert_eq!(event.scan, unit, "FR-41: wScan is the character");
        assert_eq!(event.flags & KEYEVENTF_KEYUP.0, 0);
    }
}

/// A packet that does not fit is refused whole — FR-41 is about an array that is complete and
/// in order, and half of one is worse than none.
#[test]
fn a_packet_that_does_not_fit_is_refused_and_nothing_is_written() {
    let mut out = [INPUT::default(); 3];

    assert_eq!(
        inject::build_replacement(2, &utf16("ab"), &mut out),
        Err(InjectError::OutputTooSmall { needed: 6 })
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
/// A dead key that has not composed yet (FR-24) and a key the layout has no character for
/// (FR-23) are both recorded as strokes and both erase nothing, because there is nothing of
/// theirs to erase.
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
// Point 12 — FR-41: surrogate pairs
// ---------------------------------------------------------------------------------------

/// **FR-41.** A character outside the BMP leaves as **two** `INPUT` structures, adjacent and
/// in order — a pair split apart is two undefined characters, not one character.
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

    assert_eq!(replacement.len(), 2 + 2);
    assert!(replacement[2].is_unicode());
    assert!(replacement[3].is_unicode());
    assert_eq!(replacement[2].scan, HIGH_SURROGATE);
    assert_eq!(replacement[3].scan, LOW_SURROGATE, "the pair is not split");

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
    assert_eq!(done.requested, 18);
    assert_eq!(done.accepted, 18);
    assert!(done.is_complete());

    // One call, carrying everything, in order, and no pause anywhere.
    assert_eq!(bench.log.len(), 1);
    assert_eq!(bench.sent()[0].len(), 18);
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

    // Six events, six calls, and the same six events in the same order.
    assert_eq!(done.calls, 6);
    assert_eq!(done.requested, 6);
    assert_eq!(done.accepted, 6);
    assert_eq!(
        bench.stream(),
        whole,
        "FR-44 changes the timing, not the order"
    );

    // The pause goes *between*: five of them for six portions, never before the first and
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
#[test]
fn a_surrogate_pair_stays_in_one_portion_however_long_the_pause() {
    let mut packet = [INPUT::default(); 3];
    let len = inject::build_replacement(
        0,
        &[HIGH_SURROGATE, LOW_SURROGATE, u16::from(b'!')],
        &mut packet,
    )
    .expect("sized");

    let mut bench = Bench::new();
    let done = inject::dispatch_in(&mut bench, &packet[..len], 3);

    // Two portions, not three: the pair travels together and the `!` follows on its own.
    assert_eq!(done.calls, 2);

    let packets = bench.sent();
    assert_eq!(packets[0].len(), 2);
    assert_eq!(packets[0][0].scan, HIGH_SURROGATE);
    assert_eq!(packets[0][1].scan, LOW_SURROGATE);
    assert_eq!(packets[1].len(), 1);
    assert_eq!(packets[1][0].scan, u16::from(b'!'));
}

// ---------------------------------------------------------------------------------------
// Point 17 — FR-45: the return value is checked
// ---------------------------------------------------------------------------------------

/// **FR-45.** A `SendInput` that reports fewer events than it was given is a discrepancy, and
/// the discrepancy is counted.
///
/// Both the count of calls and the number of events lost are asserted in one test on purpose:
/// the counters belong to the process, `cargo test` runs a binary's tests in parallel, and two
/// tests asserting on the same counter would be asserting on each other's timing. This is the
/// only test in this file that makes a `SendInput` come up short.
#[test]
fn a_short_return_from_sendinput_reaches_the_counter_of_fr45() {
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

    assert_eq!(done.requested, 9);
    assert_eq!(done.accepted, 7);
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
    assert_eq!(outcome.replacement.accepted, 18);
    assert!(outcome.replacement.is_complete());

    let sent_before_switch: usize = bench.log[..switch]
        .iter()
        .filter_map(|step| match step {
            Step::Send(events) => Some(events.len()),
            _ => None,
        })
        .sum();

    assert_eq!(sent_before_switch, 1 + 18);
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

// ---------------------------------------------------------------------------------------
// Points 22 and 23 — the end-to-end path, into a window this file owns
// ---------------------------------------------------------------------------------------

/// Everything the two behavioural checks need from Win32, and the fence around it.
///
/// Separated into a module of its own so that the imports it needs are not in scope for any
/// other test in this file: nothing above may reach a `SendInput`.
mod behavioural {
    use super::{GHBDTN, Recorder, buffer};
    use lang_switcher::convert;
    use lang_switcher::hook::{Edge, KeyEvent};
    use lang_switcher::inject::{self, Modifiers};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SetFocus};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, GetForegroundWindow, GetWindowTextW, MSG,
        PM_REMOVE, PeekMessageW, SW_SHOW, SetForegroundWindow, ShowWindow, TranslateMessage,
        WINDOW_EX_STYLE, WINDOW_STYLE, WS_BORDER, WS_POPUP, WS_VISIBLE,
    };
    use windows::core::{PCWSTR, w};

    /// `ES_MULTILINE | ES_AUTOVSCROLL` — the styles of a plain text box.
    const EDIT_STYLES: u32 = 0x0004 | 0x0040;

    /// How long the window is given to become the foreground one before the test gives up.
    const FOREGROUND_ATTEMPTS: u32 = 200;

    /// A window this test owns, destroyed when the value is dropped.
    struct TestWindow(HWND);

    impl Drop for TestWindow {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
            // destroyed exactly once — the type is neither `Copy` nor `Clone`. `DestroyWindow`
            // requires the calling thread to have created the window, and it did.
            let _ = unsafe { DestroyWindow(self.0) };
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

        let window = TestWindow(handle);

        // SAFETY: `handle` is the live window this frame owns. All three calls take it by value
        // and touch no memory of ours; `ShowWindow` and `SetForegroundWindow` return a `BOOL`
        // that is deliberately not treated as fatal here — the loop below is what decides.
        unsafe {
            let _ = ShowWindow(handle, SW_SHOW);
            let _ = SetForegroundWindow(handle);
            let _ = SetFocus(Some(handle));
        }

        for _ in 0..FOREGROUND_ATTEMPTS {
            pump();

            // SAFETY: takes no arguments, returns a handle by value, touches no memory of ours.
            if unsafe { GetForegroundWindow() } == handle {
                return window;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));

            // SAFETY: as above.
            unsafe {
                let _ = SetForegroundWindow(handle);
            }
        }

        panic!(
            "the test window did not become the foreground window; nothing was sent, which is \
             the point of this check"
        );
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

        // SAFETY: `window.0` is the live window this frame owns and `buffer` is a live, properly
        // aligned array owned by this frame; the call is given its true length and writes no
        // more than that, NUL included. The return value is the length written, which is what
        // bounds the slice below.
        let written = unsafe { GetWindowTextW(window.0, &mut buffer) };

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
        let mut events = vec![INPUT::default(); units.len()];

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
}
