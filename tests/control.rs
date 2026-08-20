//! Integration tests for task T-03-4 — the debug control channel of SEC-04a.
//!
//! The whole file is behind the `testing` feature, because the module under test is: SEC-04a
//! condition 1 puts the channel behind that gate and acceptance criterion 8 of section 13 of
//! SPEC checks that the shipped binary carries no trace of it. Under a plain `cargo test` this
//! file compiles to an empty test binary, which is the correct outcome and not an omission.
//!
//! # What is checked here, and what is not
//!
//! * Condition 2 of SEC-04a — **metadata only** — is checked against the exact set of keys the
//!   channel emits, not against the ones a test happens to look for: a key that appeared
//!   without being added to [`control::KEYS`] fails the test, which is what makes the check a
//!   review gate rather than a spot check.
//! * Condition 3 — **the owner of the current session and nobody else** — is checked by
//!   reading the descriptor back through the Win32 accessors and counting the entries of its
//!   DACL, including the `NULL`-DACL case, which is the way this is usually got wrong.
//! * Condition 4 — **read only** — is a property of the code: no `ReadFile`, an instance
//!   created with no inbound buffer at all, and an access mask that grants no write to anyone.
//!   The mask is asserted below, and so is the failure of a client that tries to open the
//!   channel for writing.
//! * The **length mirror**, on which the honesty of `buffer_len` rests: every path that changes
//!   the buffer must publish, and that is checked path by path against the list in
//!   `Ring::set_len`.
//! * The **cycle mirror** of task T-05-2a, on which the honesty of `cycle_position` rests, and
//!   which is checked the same way: the hotkey path that raises the counter (FR-32, FR-33) and
//!   the flush path that zeroes it (FR-34), each asserted against what the channel would report
//!   at that instant.
//! * The **server** of task T-03-4-2 — that a client of this user connects and is answered,
//!   that the answer tracks the buffer, that the channel takes no commands, and that the
//!   thread ends when it is asked to.
//!
//! # One test for everything that touches the pipe
//!
//! The channel is named after the **session**, so a process has one of them and two tests that
//! started a server would collide over the name. `cargo test` runs the tests of a binary in
//! parallel, so they are one test function instead — the same reason, and the same shape, as
//! the mirror test below.
//!
//! # Printing
//!
//! `Stroke` has no `Debug` (SEC-01, SEC-07), and nothing in this file formats one. The only
//! characters in the messages below are the string constants of this file.

#![cfg(feature = "testing")]

use std::fs::OpenOptions;
use std::io::Read;
use std::sync::{Mutex, PoisonError};

use lang_switcher::app;
use lang_switcher::buffer::{self, Recorder};
use lang_switcher::control;
use lang_switcher::convert::{self, Keystroke};
use lang_switcher::guard::Field;
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::inject::{self, Environment, Modifiers};
use lang_switcher::layouts::{
    KeyMapping, LayoutCache, LayoutId, LayoutMap, LayoutMapBuilder, Mods,
};
use lang_switcher::settings::ReplacementMethod;
use lang_switcher::watchdog;
use windows::Win32::System::Pipes::PIPE_REJECT_REMOTE_CLIENTS;
use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, VK_A, VK_BACK, VK_RETURN};

/// Serialises the tests that assert on the process-wide mirrors.
///
/// Each mirror is one atomic for the whole process and the typing buffer is a thread-local, so
/// two tests running in parallel would be asserting on each other's timing rather than on the
/// code. Each of the three holds this for its whole body; nothing else in the file touches a
/// mirror.
static MIRROR: Mutex<()> = Mutex::new(());

// -------------------------------------------------------------------------------------
// Condition 2 of SEC-04a — metadata only
// -------------------------------------------------------------------------------------

/// The payload is exactly the keys of the task table, one per line, and nothing else.
///
/// The assertion is on the **set**, so this test is what stops a later task from quietly
/// adding a key: growing the channel means growing [`control::KEYS`], and growing that means
/// somebody looked at what is being published.
#[test]
fn the_payload_is_exactly_the_documented_keys_one_per_line() {
    let text = control::render(&control::snapshot());

    let keys: Vec<&str> = text
        .lines()
        .map(|line| {
            line.split_once('=')
                .unwrap_or_else(|| panic!("every line is key=value, this one is not: {line:?}"))
                .0
        })
        .collect();

    assert_eq!(
        keys,
        control::KEYS.to_vec(),
        "the channel emits exactly the keys it documents, in that order"
    );

    // A line count that matched while a line was empty would pass the check above by
    // accident; `lines` ignores a trailing newline, so this pins the terminator too.
    assert!(text.ends_with('\n'), "the last line is terminated");
    assert_eq!(text.lines().count(), control::KEYS.len());
}

/// **SEC-01, SEC-07, condition 2.** Nothing but digits and the two words of FR-42 leaves here.
///
/// A character, a key code or a scan code would have to arrive as a value, so the values are
/// what this looks at: every one of them is a decimal number, except `replacement_method`,
/// which is one of the two words section 7 defines, and `watchdog_last_reason`, which is one of
/// the five words of `watchdog::Reason` — task **T-06-2**. There is no third shape for anything
/// to hide in, and each of the two word-valued keys is checked against its own closed list
/// rather than against "any word", which is what keeps the exception from becoming a hole.
///
/// Task **T-10-5** adds `active_layout`, and it is a third shape — a hexadecimal `HKL`. It is
/// held to a shape as closed as the two lists above: literally `0x` and exactly eight hex
/// digits, so nothing of variable length can ride in it, and an `HKL` carries nothing of a
/// keystroke in any case (module `switch` states the same thing about its own arguments).
///
/// Task **T-10-10** adds `field_state`, and it is no new shape at all — a fourth word-valued
/// key held against its own closed list, the four arms of `guard::Field`, built here from the
/// arms themselves rather than typed out. The exception stays an exception because every one
/// of these lists is closed and none of them is "any word".
#[test]
fn every_value_is_a_number_or_one_of_the_two_words_of_fr42() {
    let text = control::render(&control::snapshot());

    assert!(
        text.is_ascii(),
        "the payload is ASCII, so no decoded character can be riding in it"
    );

    let reasons: Vec<&str> = [
        watchdog::Reason::None,
        watchdog::Reason::Timer,
        watchdog::Reason::DesktopSwitch,
        watchdog::Reason::SessionChange,
        watchdog::Reason::PowerResume,
    ]
    .iter()
    .map(|reason| reason.name())
    .collect();

    let fields: Vec<&str> = [
        Field::Pending,
        Field::Ordinary,
        Field::Password,
        Field::Undetermined,
    ]
    .iter()
    .map(|field| field.name())
    .collect();

    for line in text.lines() {
        let (key, value) = line.split_once('=').expect("key=value");

        if key == "replacement_method" {
            assert!(
                value == "auto" || value == "backspace" || value == "selection",
                "replacement_method is one of the three words of section 7"
            );
            continue;
        }

        // **Task T-10-8.** The method the last replacement really ran — FR-42а resolves
        // `auto` per press, so the configured word above stopped answering which packet a
        // given press built. `none` is the state before the first replacement; `auto` is
        // deliberately NOT in this list, because a resolution cannot answer it.
        if key == "last_replacement_method" {
            assert!(
                value == "none" || value == "backspace" || value == "selection",
                "last_replacement_method is a resolved method or none, and {value:?} is neither"
            );
            continue;
        }

        if key == "watchdog_last_reason" {
            assert!(
                reasons.contains(&value),
                "watchdog_last_reason is one of {reasons:?}, and {value:?} is not"
            );
            continue;
        }

        // **Task T-10-10.** The third word-valued key, and held the same way: its own closed
        // list, built from the arms of `guard::Field` rather than written out here, so the
        // list of words and the list of arms cannot drift apart.
        if key == "field_state" {
            assert!(
                fields.contains(&value),
                "field_state is one of {fields:?}, and {value:?} is not"
            );
            continue;
        }

        if key == "active_layout" {
            let digits = value
                .strip_prefix("0x")
                .unwrap_or_else(|| panic!("active_layout is a hex HKL, and {value:?} is not"));

            assert!(
                digits.len() == 8 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "active_layout is 0x followed by exactly eight hex digits, {value:?} is not"
            );
            continue;
        }

        // **Task T-10-6.** The one compound value on the channel: three counts, `erase/units/
        // distinct`, describing the last replacement packet of FR-41. They travel together
        // because they are read together — see `control::LAST_REPLACEMENT` — and the shape is
        // asserted here so that a fourth field or a stray separator cannot appear unnoticed.
        if key == "last_replacement" {
            let fields: Vec<&str> = value.split('/').collect();
            assert_eq!(
                fields.len(),
                3,
                "last_replacement is erase/units/distinct, and {value:?} is not"
            );
            for field in fields {
                assert!(
                    !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit()),
                    "every field of last_replacement is a decimal count, {field:?} is not"
                );
            }
            continue;
        }

        // ⭐ **Task T-10-17.** The second compound value, and the same discipline as the first:
        // two hexadecimal `HKL`s separated by `/` — the layout the strokes were recorded under
        // and the one they were rendered into. Each half is held to literally `0x` and exactly
        // eight hex digits, the shape `active_layout` is held to above, so nothing of variable
        // length can ride in either of them. The flag beside it,
        // `last_replacement_changed`, needs no arm at all: it renders as `0` or `1` and falls
        // through to the decimal rule below, which is the tightest statement available about
        // a bit.
        if key == "last_replacement_direction" {
            let halves: Vec<&str> = value.split('/').collect();
            assert_eq!(
                halves.len(),
                2,
                "last_replacement_direction is from/to, and {value:?} is not"
            );
            for half in halves {
                let digits = half.strip_prefix("0x").unwrap_or_else(|| {
                    panic!("every half of last_replacement_direction is a hex HKL, {half:?} is not")
                });
                assert!(
                    digits.len() == 8 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "every half is 0x and exactly eight hex digits, {half:?} is not"
                );
            }
            continue;
        }

        assert!(
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
            "{key} is a plain decimal count, and {value:?} is not"
        );
    }
}

/// **A reserved key is missing from the payload, not answered with a zero.** SEC-06, and the
/// shape the future needs.
///
/// A bench that read `password_field=0` would record "no buffering in a password field" for a
/// build that cannot tell. Absence was the honest answer for as long as no task could tell.
///
/// The list held two keys until task **T-05-2a**, which published `cycle_position`, and one until
/// task **T-06-1**, which published `password_field`; task **T-06-3** made the two edits that
/// emptied it, this one and the constant, because they are one change and `assert_eq!` over two
/// arrays of different lengths is a compilation error rather than a failing test.
///
/// The assertion is still on the **whole array** and not on "contains", which is what keeps this a
/// review gate now that the array is empty: a key added to the reservation, or one left in it
/// after its task published it, fails here and somebody looks. That is what a reservation is for,
/// and it is why the constant outlives the last key that stood in it.
#[test]
fn the_reserved_keys_are_absent_rather_than_answered_with_a_zero() {
    /// Spelled out rather than written `[]` at the call site: `assert_eq!` needs the element type,
    /// and an empty literal has none to infer from.
    const NOTHING: [&str; 0] = [];

    let text = control::render(&control::snapshot());

    assert_eq!(
        control::RESERVED_KEYS,
        NOTHING,
        "every key SEC-04a reserved is published now"
    );

    // The two keys that left the reservation, each in the change that started emitting it. A key
    // in neither list would be one nothing answers and nothing reserves — which is the hole this
    // pair of assertions exists to keep shut.
    for published in ["cycle_position", "password_field"] {
        assert!(
            !control::RESERVED_KEYS.contains(&published),
            "{published} is published and is no longer reserved"
        );
        assert!(
            control::KEYS.contains(&published),
            "{published} is in the emitted key list instead"
        );
    }

    for reserved in control::RESERVED_KEYS {
        assert!(
            !text.contains(reserved),
            "{reserved} must not appear until its own task publishes it"
        );
        assert!(
            !control::KEYS.contains(&reserved),
            "{reserved} must not be in the emitted key list either"
        );
    }
}

/// **The two counts task T-06-1a added come from module `guard` and are the ones it holds** —
/// SEC-06, and the reason the pair exists at all.
///
/// `password_field` is one bit, and one bit cannot say *whose* window it describes. A bench
/// reading `password_field=0` beside a caret that has just moved into a password box cannot tell
/// "this field is ordinary" from "the program was never told the focus moved" — and those two
/// differ by exactly the thing SEC-06 is about. `focus_changes` and `password_probes` are what
/// separate them: the first is the input thread's end of the chain (`guard::note_focus_moved`),
/// the second the watcher thread's (`guard::run_pending_probe`), and a change that raises neither
/// is a caret the program did not follow.
///
/// Asserted against `guard::counters()` rather than against a literal, because what has to hold
/// is that the snapshot reports **these** counters and not two numbers that merely look like
/// them. Monotone counters cannot be pinned to an equality across two reads — the program keeps
/// running — so the assertion is that the snapshot never lags behind a reading taken before it
/// and never runs ahead of one taken after it.
#[test]
fn the_focus_and_probe_counts_are_the_ones_module_guard_holds() {
    let before = lang_switcher::guard::counters();
    let state = control::snapshot();
    let after = lang_switcher::guard::counters();

    assert!(
        state.focus_changes >= before.focus_changes && state.focus_changes <= after.focus_changes,
        "focus_changes is guard's own count of focus changes"
    );
    assert!(
        state.password_probes >= before.probes && state.password_probes <= after.probes,
        "password_probes is guard's own count of probes carried out"
    );

    // On the wire, in the documented place, as plain decimal counts. SEC-01 and SEC-07: what
    // leaves here is how many times something happened, never what was typed.
    let text = control::render(&state);

    for (key, value) in [
        ("focus_changes", state.focus_changes),
        ("password_probes", state.password_probes),
    ] {
        assert!(
            control::KEYS.contains(&key),
            "{key} is one of the documented keys"
        );
        assert!(
            text.contains(&format!("{key}={value}\n")),
            "{key} is rendered as the count the snapshot carries"
        );
    }
}

/// The two configuration values of FR-42 and FR-44 really do come from what was published.
///
/// This is the point of putting them on the channel at all: today there is no way to see from
/// outside the process that `config.toml` reached module `inject`, because the values live in
/// atomics. The publication path is module `inject`'s and is exercised there; what is checked
/// here is that the channel reports the published value rather than a constant.
#[test]
fn the_replacement_settings_on_the_channel_are_the_published_ones() {
    // The three words of section 7, `auto` of FR-42а included: the bench compares this key
    // against what it wrote into `config.toml`, and since task T-10-8 that may be `auto`.
    assert_eq!(control::method_name(ReplacementMethod::Auto), "auto");
    assert_eq!(
        control::method_name(ReplacementMethod::Backspace),
        "backspace"
    );
    assert_eq!(
        control::method_name(ReplacementMethod::Selection),
        "selection"
    );

    let state = control::snapshot();

    assert_eq!(
        state.inter_event_delay_ms,
        lang_switcher::inject::inter_event_delay_ms(),
        "FR-44: the channel reports the published pause"
    );
    assert_eq!(
        state.replacement_method,
        lang_switcher::inject::replacement_method(),
        "FR-42: the channel reports the published method"
    );
}

// -------------------------------------------------------------------------------------
// The name of the channel
// -------------------------------------------------------------------------------------

/// One prefix constant, and the session identifier after it.
///
/// The prefix is what acceptance criterion 8 of section 13 searches the shipped binary for, so
/// it has to be a single recognisable string rather than something assembled at run time.
#[test]
fn the_name_is_the_single_prefix_followed_by_the_session() {
    let name = control::pipe_name();

    assert!(
        name.starts_with(control::PIPE_NAME_PREFIX),
        "the name begins with the one constant the criterion searches for"
    );
    assert!(
        control::PIPE_NAME_PREFIX.starts_with(r"\\.\pipe\"),
        "a local named pipe, on this machine and not over the network"
    );

    let session = &name[control::PIPE_NAME_PREFIX.len()..];

    assert!(
        !session.is_empty() && session.bytes().all(|byte| byte.is_ascii_digit()),
        "the tail is the decimal session identifier, and {session:?} is not"
    );

    // Two calls in one process name one channel; a name that varied per call would give the
    // bench nothing to connect to.
    assert_eq!(name, control::pipe_name());
}

// -------------------------------------------------------------------------------------
// Condition 3 of SEC-04a — the owner of the current session, and nobody else
// -------------------------------------------------------------------------------------

/// **The DACL holds exactly one allow entry, on the SID of this process's owner.**
///
/// Every clause of the assertion answers a way of getting condition 3 wrong: no DACL at all,
/// a `NULL` DACL — which grants everyone everything and is the usual mistake — a DACL that is
/// present but not the one this code built, more than one entry, or an entry on somebody
/// else's SID. The rights are checked too: read and nothing else, which is condition 4
/// expressed where a client cannot argue with it.
#[test]
fn the_dacl_admits_the_owner_of_this_process_and_nobody_else() {
    let descriptor = control::OwnerOnly::for_this_process()
        .expect("the descriptor is built from this process's own token");

    let facts = descriptor.dacl_facts().expect("the DACL reads back");

    assert!(
        facts.present,
        "SEC-04a condition 3: the descriptor has a DACL"
    );
    assert!(
        !facts.null_dacl,
        "SEC-04a condition 3: a NULL DACL grants everyone everything and is forbidden"
    );
    assert!(
        facts.own_acl,
        "the attached DACL is the one this code built, by address"
    );
    assert_eq!(facts.ace_count, 1, "exactly one entry, of any kind");
    assert_eq!(
        facts.allow_aces_on_owner, 1,
        "and that entry allows the owner of this process"
    );
    assert_eq!(
        facts.granted,
        control::CHANNEL_RIGHTS,
        "the entry grants read and nothing else"
    );

    // The attributes really do point at the descriptor that was just examined, and the handle
    // is never inheritable: a channel that a child process could inherit would be a channel
    // that left this process.
    let attributes = descriptor.attributes();

    assert!(!attributes.lpSecurityDescriptor.is_null());
    assert!(!attributes.bInheritHandle.as_bool());
    assert_eq!(
        attributes.nLength as usize,
        size_of::<windows::Win32::Security::SECURITY_ATTRIBUTES>()
    );
}

// -------------------------------------------------------------------------------------
// The length mirror — the honesty of `buffer_len`
// -------------------------------------------------------------------------------------

/// Scan code and virtual key of `A`. Nothing here depends on which key it is.
const SCAN_A: u16 = 0x1E;

/// One ordinary key press.
fn press(vk: u16, scan: u16) {
    buffer::record(KeyEvent {
        vk,
        edge: Edge::Down,
        extra_info: 0,
        scan,
        flags: 0,
        time: 0,
    });
}

/// The length the channel would report right now.
fn mirrored() -> usize {
    control::snapshot().buffer_len
}

/// **Every path that changes the buffer publishes the new length.**
///
/// One test and not eight, deliberately: the mirror is a process-wide atomic and `cargo test`
/// runs the tests of a binary in parallel, so two tests asserting on it would be asserting on
/// each other's timing. This is the same reason `app`'s own counter test gives.
///
/// The list below is the list of writes to `Ring::len`, which is the whole of what can change
/// the length: the constructor, `push` — both storing and evicting — `pop`, `clear` and the
/// two writes of `retain_after`. `Recorder::set_capacity`, `buffer::install` and
/// `buffer::uninstall` reach the mirror through those, and are exercised as well because the
/// path from them to a publication runs through a `Drop` and is worth seeing work.
#[test]
fn every_path_that_changes_the_buffer_publishes_its_length() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    // `Ring::with_capacity`, through `install`. A fresh buffer is empty and says so.
    buffer::install_recorder(Recorder::with_capacity(4));
    assert_eq!(mirrored(), 0, "a new buffer publishes zero");

    // `Ring::push`, storing.
    for expected in 1..=4 {
        press(VK_A.0, SCAN_A);
        assert_eq!(
            mirrored(),
            expected,
            "a stored stroke publishes the new length"
        );
    }

    // `Ring::push`, evicting — FR-07. The length does not change, and the mirror must not
    // drift: a publication that only ran on the growing branch would be wrong from here on.
    press(VK_A.0, SCAN_A);
    assert_eq!(
        mirrored(),
        4,
        "an eviction leaves the length at the capacity"
    );
    assert_eq!(buffer::len(), 4, "and the buffer agrees");

    // `Ring::pop` — the `Backspace` row of FR-10.
    press(VK_BACK.0, 0x0E);
    assert_eq!(mirrored(), 3, "a Backspace publishes one less");

    // `Ring::clear` through a flushing key — the boundary row of FR-10.
    press(VK_RETURN.0, 0x1C);
    assert_eq!(mirrored(), 0, "a flush publishes zero");

    // `Ring::clear` through the public flush the asynchronous sources use.
    press(VK_A.0, SCAN_A);
    press(VK_A.0, SCAN_A);
    assert_eq!(mirrored(), 2);
    assert!(buffer::reset());
    assert_eq!(mirrored(), 0, "reset publishes zero");

    // `Ring::retain_after`, the partial arm — FR-12. Two strokes at tick 10, two at tick 30,
    // and an event at tick 20 removes the first pair and keeps the second.
    buffer::install_recorder(Recorder::with_capacity(8));
    for time in [10, 10, 30, 30] {
        buffer::record(KeyEvent {
            vk: VK_A.0,
            edge: Edge::Down,
            extra_info: 0,
            scan: SCAN_A,
            flags: 0,
            time,
        });
    }
    assert_eq!(mirrored(), 4);
    assert!(buffer::reset_up_to(20).is_some());
    assert_eq!(mirrored(), 2, "a partial flush publishes what survived");

    // `Ring::retain_after` reaching `reset` instead — the full-clearance arm of FR-12.
    assert!(buffer::reset_up_to(40).is_some());
    assert_eq!(mirrored(), 0, "a full clearance publishes zero");

    // `Recorder::set_capacity`: a new ring replaces the old one, and the old one is dropped
    // and zeroed (SEC-02). Whichever of the two publishes last, the answer is the truth.
    press(VK_A.0, SCAN_A);
    press(VK_A.0, SCAN_A);
    assert_eq!(mirrored(), 2);
    buffer::with(|recorder| recorder.set_capacity(16));
    assert_eq!(
        mirrored(),
        0,
        "a resize publishes the empty ring it left behind"
    );
    assert_eq!(buffer::len(), 0);

    // `buffer::uninstall`: the buffer is dropped, which zeroes it, and the mirror follows —
    // the same path the input thread takes when it ends.
    press(VK_A.0, SCAN_A);
    assert_eq!(mirrored(), 1);
    assert!(buffer::uninstall());
    assert_eq!(mirrored(), 0, "an uninstalled buffer publishes zero");
    assert!(!buffer::is_installed());

    // And once more with the rules mixed, comparing the published number against the buffer's
    // own answer after every step: the walk above takes the paths one at a time, and what the
    // acceptance bench of section 11.5 will rely on is that they never disagree in traffic.
    // Part of this test rather than a second one for the reason given above.
    buffer::install_recorder(Recorder::with_capacity(6));

    let script: [(u16, u16); 12] = [
        (VK_A.0, SCAN_A),
        (VK_A.0, SCAN_A),
        (VK_BACK.0, 0x0E),
        (VK_A.0, SCAN_A),
        (VK_A.0, SCAN_A),
        (VK_A.0, SCAN_A),
        (VK_RETURN.0, 0x1C),
        (VK_A.0, SCAN_A),
        (VK_BACK.0, 0x0E),
        (VK_BACK.0, 0x0E),
        (VK_A.0, SCAN_A),
        (VK_A.0, SCAN_A),
    ];

    for (vk, scan) in script {
        press(vk, scan);

        assert_eq!(
            buffer::len(),
            mirrored(),
            "the published length is the length of the buffer"
        );
    }

    buffer::uninstall();
    assert_eq!(mirrored(), 0);
}

// -------------------------------------------------------------------------------------
// The cycle mirror — the honesty of `cycle_position`, task T-05-2a
// -------------------------------------------------------------------------------------

/// The position the channel would report right now.
fn mirrored_cycle() -> usize {
    control::snapshot().cycle_position
}

/// **Both paths that move the counter of FR-32 publish it, and nothing else has to.**
///
/// The counter lives in a field of `Recorder`, which is a thread-local of the input thread
/// (section 6.3), so the number a client of the channel reads is the *mirror* and not the
/// counter. A mirror that lagged would be worse than no mirror: the acceptance bench of
/// section 11.5 takes it for the truth, and position 16 of the matrix of section 11.3 uses it
/// to tell "the text matched by luck" from "the program really came back to the start of the
/// cycle".
///
/// One test rather than several, for the reason the length mirror gives above: the mirror is a
/// process-wide atomic and `cargo test` runs the tests of a binary in parallel.
#[test]
fn every_path_that_moves_the_cycle_position_publishes_it() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    buffer::install_recorder(Recorder::with_capacity(8));

    // A known starting point, and the first assertion of FR-34 on the way to it: the flush
    // publishes the zero it leaves behind rather than merely holding it.
    assert!(buffer::reset());
    assert_eq!(mirrored_cycle(), 0, "a flush publishes the zero of FR-34");

    // `Recorder::advance_cycle` — the hotkey path. A cycle of two is the "Пара" of FR-30, and
    // FR-33 says the rollback is that cycle walked twice: 0 → 1 → 0.
    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(2)), Some(1));
    assert_eq!(
        mirrored_cycle(),
        1,
        "the hotkey path publishes the new position"
    );

    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(2)), Some(0));
    assert_eq!(
        mirrored_cycle(),
        0,
        "FR-33: the second press comes back to the start, and the channel says so"
    );

    // A cycle of three — FR-31 — walked past its end, so that a mirror published only on the
    // growing branch would be caught.
    for expected in [1usize, 2, 0, 1] {
        assert_eq!(
            buffer::with(|recorder| recorder.advance_cycle(3)),
            Some(expected)
        );
        assert_eq!(mirrored_cycle(), expected);
        assert_eq!(
            buffer::with(|recorder| recorder.cycle_position()),
            Some(mirrored_cycle()),
            "the published position is the position of the buffer"
        );
    }

    // FR-34 through a rule of the FR-10 table: a stroke, the counter off zero, a boundary key,
    // and the channel reports zero at once — not after the next press of the hotkey.
    press(VK_A.0, SCAN_A);
    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(3)), Some(2));
    assert_eq!(mirrored_cycle(), 2);

    press(VK_RETURN.0, 0x1C);
    assert_eq!(buffer::len(), 0, "the boundary key emptied the buffer");
    assert_eq!(buffer::with(|recorder| recorder.cycle_position()), Some(0));
    assert_eq!(
        mirrored_cycle(),
        0,
        "FR-34: the mirror does not lag behind the flush"
    );

    // And through the public flush the asynchronous sources of FR-10 arrive at.
    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(2)), Some(1));
    assert_eq!(mirrored_cycle(), 1);
    assert!(buffer::reset());
    assert_eq!(mirrored_cycle(), 0, "reset publishes zero");

    // `Backspace` is the one row of the table that is not a flush, so it moves the length and
    // must not move the position — a publication hung on the wrong path would show up here.
    press(VK_A.0, SCAN_A);
    press(VK_A.0, SCAN_A);
    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(4)), Some(1));
    press(VK_BACK.0, 0x0E);
    assert_eq!(buffer::len(), 1, "Backspace took one stroke out");
    assert_eq!(
        mirrored_cycle(),
        1,
        "and left the position where it was — FR-34 has nothing to say about it"
    );

    // **SEC-01, SEC-07.** What actually leaves the process is a decimal number on one line.
    // Nothing about the strokes that got the cycle there, and nothing about the layout the
    // position names.
    assert!(buffer::reset());
    assert_eq!(buffer::with(|recorder| recorder.advance_cycle(3)), Some(1));

    let text = control::render(&control::snapshot());
    let published = value_of(&text, "cycle_position");

    assert_eq!(published, "1", "the channel carries the position itself");
    assert!(
        published.bytes().all(|byte| byte.is_ascii_digit()),
        "and carries it as a plain decimal count and nothing else"
    );
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("cycle_position="))
            .count(),
        1,
        "on exactly one line"
    );

    buffer::uninstall();
}

// -------------------------------------------------------------------------------------
// The server — task T-03-4-2
// -------------------------------------------------------------------------------------

/// How many times a client retries a channel that is busy with somebody else.
const CONNECT_ATTEMPTS: u32 = 50;

/// Opens the channel the way any client of the acceptance bench would: **read only**, by name.
///
/// Retries a busy channel rather than failing: the instance is single and the server returns it
/// to the listening state between clients, so a client that arrives inside that window is early
/// rather than refused.
fn connect() -> std::io::Result<std::fs::File> {
    let name = control::pipe_name();
    let mut last = None;

    for _ in 0..CONNECT_ATTEMPTS {
        match OpenOptions::new().read(true).open(&name) {
            Ok(file) => return Ok(file),
            Err(error) => {
                last = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    Err(last.expect("the loop runs at least once"))
}

/// Connects, reads one answer to its end, and returns it.
///
/// The end of an answer is the server taking the connection away — it writes once, flushes and
/// disconnects — so a read that fails after bytes have arrived is the end of the message and
/// not a fault. A read that fails before any have is left to the assertions of the caller,
/// which check the content.
fn read_channel() -> String {
    let mut file = connect().expect("a client of the current user connects to the channel");
    let mut answer = Vec::new();
    let mut chunk = [0u8; 512];

    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => answer.extend_from_slice(&chunk[..read]),
            Err(_disconnected) => break,
        }
    }

    String::from_utf8(answer).expect("the payload is ASCII, so it is UTF-8")
}

/// The value of one key of an answer.
fn value_of<'a>(answer: &'a str, key: &str) -> &'a str {
    answer
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("the answer carries {key}"))
}

/// **The channel, end to end**: it is created, it admits this user, it answers with the
/// snapshot, it follows the buffer, it takes no commands, and it stops when it is asked to.
///
/// One test rather than six because the channel is named after the session: a process has one
/// of them, and `cargo test` would run six of these at once. The numbered points of the task's
/// acceptance list are marked in the body.
#[test]
fn the_channel_serves_the_owner_of_this_process_and_stops_when_asked() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    // Point 18, and it is asserted on the constants the server actually passes to
    // `CreateNamedPipeW` rather than on a copy of them: SEC-03 and NFR-11 forbid network
    // activity, and a named pipe without this flag is reachable over SMB.
    assert!(
        control::PIPE_MODE.contains(PIPE_REJECT_REMOTE_CLIENTS),
        "SEC-03, NFR-11: the channel refuses clients from other machines"
    );

    // Condition 4 of SEC-04a in the creation flags: the instance is outbound only and has no
    // inbound buffer, so there is nothing for a client to send through and nothing to read.
    assert_eq!(
        control::PIPE_OPEN_MODE,
        windows::Win32::Storage::FileSystem::PIPE_ACCESS_OUTBOUND,
        "SEC-04a condition 4: the server writes, the client reads, and there is no other way"
    );

    // The buffer this test will make the channel report on. Installed before the server so
    // that the first answer already has a length to carry.
    buffer::install_recorder(Recorder::with_capacity(16));

    let channel = control::start().expect("the channel comes up");

    // Point 16 and point 26: a client running as the current user connects and is answered,
    // and the answer is the documented snapshot — the same set of keys, in the same order,
    // that `render` is checked against above.
    let answer = read_channel();

    let keys: Vec<&str> = answer
        .lines()
        .map(|line| line.split_once('=').expect("key=value").0)
        .collect();

    assert_eq!(
        keys,
        control::KEYS.to_vec(),
        "point 26: the keys of the task table arrive over the channel"
    );
    assert!(answer.is_ascii() && answer.ends_with('\n'));

    for reserved in control::RESERVED_KEYS {
        assert!(
            !answer.contains(reserved),
            "point 22 holds on the wire too: {reserved} is absent, not zero"
        );
    }

    // Nothing is reserved any more, so the loop above is vacuous, and this is what carries its
    // meaning on the wire in its place: the two keys that left the reservation arrive **with a
    // value** rather than being dropped on the way out of the process. Absence used to be the
    // assertion; presence is its counterpart, and between them the wire is pinned in both
    // directions for every key the reservation has ever named.
    for published in ["cycle_position", "password_field"] {
        assert!(
            !value_of(&answer, published).is_empty(),
            "{published} left the reservation and arrives over the channel with a value"
        );
    }

    // Point 27. Six strokes into this thread's buffer, and the channel says six. The number
    // travels through the mirror of section 6.3, which is the only way a thread that is not
    // the input thread can learn it.
    for _ in 0..6 {
        press(VK_A.0, SCAN_A);
    }

    assert_eq!(buffer::len(), 6, "six strokes are in the buffer");
    assert_eq!(
        value_of(&read_channel(), "buffer_len"),
        "6",
        "point 27: the channel reports six"
    );

    // Point 28. Any rule of FR-10 — `Enter` is the boundary row — and the channel says zero.
    press(VK_RETURN.0, 0x1C);

    assert_eq!(buffer::len(), 0, "the flush emptied the buffer");
    assert_eq!(
        value_of(&read_channel(), "buffer_len"),
        "0",
        "point 28: the channel reports zero after a reset"
    );

    // Point 29, and condition 4 of SEC-04a. A client cannot obtain a writable handle at all:
    // the one allow entry of the DACL grants `GENERIC_READ` and the instance is outbound, so
    // the attempt is refused by the object rather than ignored by the server.
    assert!(
        OpenOptions::new()
            .write(true)
            .open(control::pipe_name())
            .is_err(),
        "point 29: the channel takes no writer"
    );
    assert!(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(control::pipe_name())
            .is_err(),
        "point 29: nor a reader that also wants to write"
    );

    // And the program is unchanged by having been asked: the channel still answers, with the
    // same keys.
    let after = read_channel();

    assert_eq!(
        after
            .lines()
            .map(|line| line.split_once('=').expect("key=value").0)
            .collect::<Vec<&str>>(),
        control::KEYS.to_vec(),
        "point 29: a refused writer changes nothing about the program"
    );

    // The thread ends when it is asked to, inside its grace period — the mechanism points 30
    // and 31 rest on, exercised here without a running program.
    assert!(
        channel.stop(),
        "the channel thread leaves `ConnectNamedPipe` and ends"
    );

    // And it really is gone: the name no longer resolves, so no thread is left holding it.
    assert!(
        OpenOptions::new()
            .read(true)
            .open(control::pipe_name())
            .is_err(),
        "a stopped channel leaves nothing listening"
    );

    buffer::uninstall();
}

// -------------------------------------------------------------------------------------
// Task T-08-3 — the two keys point 5 of its exclusion list needed
// -------------------------------------------------------------------------------------

/// `fail_safe` and `consecutive_panics` are published, and they are a flag and a count.
///
/// # Why they exist at all
///
/// Task T-08-3 had to settle a claim of the form "FR-96 does not fire in state X". Five things
/// had to be excluded before the product could be blamed for that, and one of them was FR-99:
/// three panics in a row inside the callback disarm the program, and a disarmed program answers
/// `Outcome::PASS` to everything — which from outside looks exactly like a callback that was
/// never called. Neither the flag nor the streak is observable from outside the process by any
/// other means, which is the same argument decision Р-37 makes for `watchdog_recoveries`.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A bit and a count. The payload of a panic is dropped **unread** in `hook::guarded_decision`,
/// so there is nothing on this path that could carry a key code even in principle — which is
/// asserted below by parsing both values as numbers and refusing anything else.
#[test]
fn the_fail_safe_and_panic_keys_of_t_08_3_are_published_as_a_flag_and_a_count() {
    let text = control::render(&control::snapshot());

    let value = |key: &str| -> String {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("the channel does not publish {key}"))
            .to_owned()
    };

    let fail_safe = value("fail_safe");
    let panics = value("consecutive_panics");

    println!("fail_safe={fail_safe} consecutive_panics={panics}");

    // A flag is exactly one of two bytes. Anything else — a word, a name, a character — fails.
    assert!(
        fail_safe == "0" || fail_safe == "1",
        "fail_safe must be a flag, not {fail_safe:?}"
    );

    // A count is a decimal number and nothing else.
    assert!(
        panics.parse::<u32>().is_ok(),
        "consecutive_panics must be a count, not {panics:?}"
    );

    // Both are reviewed keys, which is what `KEYS` means, and neither is reserved.
    assert!(control::KEYS.contains(&"fail_safe"));
    assert!(control::KEYS.contains(&"consecutive_panics"));
    assert!(!control::RESERVED_KEYS.contains(&"fail_safe"));
    assert!(!control::RESERVED_KEYS.contains(&"consecutive_panics"));

    // They are appended, not inserted: twelve tasks' worth of readers diff these lines by eye.
    //
    // ⚠ Task **T-08-4** appended `device_changes` after them, which is the same rule applied
    // once more, so "last" and "one before last" is no longer what the rule says. What it says
    // is that these two sit at the positions they were appended at and that nothing has been
    // pushed in front of them — which is a stronger statement than the one it replaces, and one
    // that will not have to be rewritten by the next task that appends a key.
    assert_eq!(
        control::KEYS.iter().position(|key| *key == "fail_safe"),
        Some(18)
    );
    assert_eq!(
        control::KEYS
            .iter()
            .position(|key| *key == "consecutive_panics"),
        Some(19)
    );
}

// -------------------------------------------------------------------------------------
// Task T-08-4 — the key that shows FR-21 still being delivered
// -------------------------------------------------------------------------------------

/// `device_changes` is published, it is a count, and it says what `watchdog` says.
///
/// # Why it exists at all
///
/// Task T-08-4 took the keyboard entry out of the Raw Input registration, because a process that
/// holds one loses the whole low-level keyboard hook chain while a window of its own is in front —
/// which is what made FR-96 unreachable with the settings dialog of FR-92 open. That entry was
/// also half of the FR-21 delivery, so the delivery moved to `RegisterDeviceNotificationW` and the
/// real `WM_DEVICECHANGE`.
///
/// A replacement of a delivery mechanism has to be **shown** to deliver, and from outside the
/// process there is nothing else to look at: this number moving when a keyboard is plugged in is
/// the whole of the evidence. `cache_builds` is its other half — this counts the message, that
/// counts the rebuild it caused.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A count. The `WM_DEVICECHANGE` behind it carries a device name in its `lparam` and that name is
/// read nowhere in this program, which is asserted below by parsing the value as a number and
/// refusing anything else.
#[test]
fn the_device_change_key_of_t_08_4_is_published_as_a_count() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("device_changes="))
        .expect("the channel does not publish device_changes");

    println!("device_changes={published}");

    assert!(
        published.parse::<u32>().is_ok(),
        "device_changes must be a count, not {published:?}"
    );

    assert!(control::KEYS.contains(&"device_changes"));
    assert!(!control::RESERVED_KEYS.contains(&"device_changes"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task T-10-0
    // appended `background_skips` after this one, so "last" became a fixed position, the same
    // rewrite the fail_safe pair went through when this key arrived.
    assert_eq!(
        control::KEYS
            .iter()
            .position(|key| *key == "device_changes"),
        Some(20)
    );
}

/// The key mirrors `watchdog`, and is not a number of the channel's own making.
#[test]
fn the_device_change_key_mirrors_the_counter_of_the_watchdog() {
    let state = control::snapshot();

    assert_eq!(
        state.device_changes,
        lang_switcher::watchdog::counters().device_changes
    );

    // NFR-13, task T-08-4: `UnregisterDeviceNotification` refusals are counted rather than
    // journalled, because the vocabulary of `diag` has no name for that call and the task that
    // wrote it could not open `src\diag.rs`. A test process that never registered one must read
    // zero, and a non-zero value here would mean the count had been wired to something else.
    assert_eq!(
        lang_switcher::watchdog::counters().device_notice_failures,
        0
    );
}

/// The two new keys say what `hook` says, and not something of the channel's own.
#[test]
fn the_two_new_keys_mirror_the_state_of_the_hook() {
    let state = control::snapshot();

    assert_eq!(state.fail_safe, lang_switcher::hook::fail_safe());
    assert_eq!(
        state.consecutive_panics,
        lang_switcher::hook::consecutive_panics()
    );

    // A program that has not panicked reports no panics. This is the value task T-08-3 read in
    // the state under investigation, and the reason point 5 of its list came out "not the cause".
    assert!(!state.fail_safe);
    assert_eq!(state.consecutive_panics, 0);
}

// -------------------------------------------------------------------------------------
// Task T-10-0 — the key that makes the repair of the acceptance defect observable
// -------------------------------------------------------------------------------------

/// `background_skips` is published, it is a count, and it says what `watchdog` says.
///
/// # Why it exists at all
///
/// Task T-10-0 gated the flush of FR-10 on the event concerning the user's actual foreground
/// (decision Р-60): the idle focus churn of background processes used to reach `request_flush`
/// and erase what the user had typed — the defect that broke the acceptance session. The
/// product subscribes with `WINEVENT_SKIPOWNPROCESS`, so a storm staged in the product's own
/// process is invisible to it, and a test that staged one there would come out green having
/// checked nothing. The storm therefore comes from a **foreign** process, and this number
/// growing under it is the positive control that the storm was delivered and turned away —
/// while `window_flushes` standing still beside it is the repair itself.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A count. The event behind it is dropped whole; nothing of it is read but the handle
/// relation that decided it, which is asserted below by parsing the value as a number and
/// refusing anything else.
#[test]
fn the_background_skip_key_of_t_10_0_is_published_as_a_count() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("background_skips="))
        .expect("the channel does not publish background_skips");

    println!("background_skips={published}");

    assert!(
        published.parse::<u32>().is_ok(),
        "background_skips must be a count, not {published:?}"
    );

    assert!(control::KEYS.contains(&"background_skips"));
    assert!(!control::RESERVED_KEYS.contains(&"background_skips"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // T-10-0e appended `focus_repeats` after this one, so "last" became a fixed position,
    // the same rewrite `device_changes` went through when this key arrived.
    assert_eq!(
        control::KEYS
            .iter()
            .position(|key| *key == "background_skips"),
        Some(21)
    );
}

/// The key mirrors `watchdog`, and is not a number of the channel's own making.
#[test]
fn the_background_skip_key_mirrors_the_counter_of_the_watchdog() {
    let before = lang_switcher::watchdog::counters().background_skips;
    let state = control::snapshot();
    let after = lang_switcher::watchdog::counters().background_skips;

    // Monotone counters cannot be pinned to an equality across two reads — another test of
    // this binary may hold live subscriptions while this one runs — so the assertion is that
    // the snapshot never lags behind a reading taken before it and never runs ahead of one
    // taken after it, the same shape the focus-and-probe test uses for guard's counters.
    assert!(
        state.background_skips >= before && state.background_skips <= after,
        "background_skips is watchdog's own count of ignored background events"
    );
}

// -------------------------------------------------------------------------------------
// Task T-10-0e — the key that makes the frontmost-churn repair observable
// -------------------------------------------------------------------------------------

/// `focus_repeats` is published, it is a count, and it says what `watchdog` says.
///
/// # Why it exists at all
///
/// The gate of task T-10-0 turns away the churn of *background* processes, but the
/// frontmost window's own churn passes it legitimately — its root is the foreground — and
/// erased what the user had typed (VS Code, measured: 11 of 36 strokes over six rounds).
/// Task T-10-0e remembers the hwnd of the last focus event and skips the repeats. The
/// product subscribes with `WINEVENT_SKIPOWNPROCESS`, so the staged same-hwnd churn must
/// come from a foreign process, and this number growing under it is the positive control
/// that the churn was delivered and turned away — while `window_flushes` standing still
/// beside it is the repair itself.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A count. The handle behind it is compared as a value and dropped; nothing else of the
/// event is read at all — asserted below by parsing the value as a number and refusing
/// anything else.
#[test]
fn the_focus_repeat_key_of_t_10_0e_is_published_as_a_count() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("focus_repeats="))
        .expect("the channel does not publish focus_repeats");

    println!("focus_repeats={published}");

    assert!(
        published.parse::<u32>().is_ok(),
        "focus_repeats must be a count, not {published:?}"
    );

    assert!(control::KEYS.contains(&"focus_repeats"));
    assert!(!control::RESERVED_KEYS.contains(&"focus_repeats"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // T-10-0f appended `layout_probes` after this one, so "last" became a fixed position,
    // the same rewrite `background_skips` and `device_changes` each went through in turn.
    assert_eq!(
        control::KEYS.iter().position(|key| *key == "focus_repeats"),
        Some(22)
    );
}

/// The key mirrors `watchdog`, and is not a number of the channel's own making.
#[test]
fn the_focus_repeat_key_mirrors_the_counter_of_the_watchdog() {
    let before = lang_switcher::watchdog::counters().focus_repeats;
    let state = control::snapshot();
    let after = lang_switcher::watchdog::counters().focus_repeats;

    // The same bracket the background_skips mirror uses: a monotone counter cannot be
    // pinned to an equality across two reads while other tests of this binary run.
    assert!(
        state.focus_repeats >= before && state.focus_repeats <= after,
        "focus_repeats is watchdog's own count of skipped foreground repeats"
    );
}

// -------------------------------------------------------------------------------------
// Task T-10-0f — the key that makes the layout-probe repair observable
// -------------------------------------------------------------------------------------

/// `layout_probes` is published, it is a count, and it says what `watchdog` says.
///
/// # Why it exists at all
///
/// The probe behind `WM_APP_LAYOUT` is posted beside the `WM_APP_FLUSH` of its own focus
/// change, and answering that flush parks the typing buffer for the interval of FR-71 — so
/// before task T-10-0f every focus-change probe arrived at a gate that read the parked
/// buffer as "not the input thread" and was dropped whole: `layout_probes=1` against
/// `window_flushes=12` on the live run, felt as «первое нажатие моргает» (FR-26 converting
/// a stale direction «в себя»). The number was previously visible only in the file report,
/// that is, only after the process had exited; the repair is precisely this count growing
/// beside `focus_changes` on a live run, which is what moving it onto the channel makes
/// observable press by press.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A count of messages. Not a layout name, not a window, not a stroke — asserted below by
/// parsing the value as a number and refusing anything else.
#[test]
fn the_layout_probe_key_of_t_10_0f_is_published_as_a_count() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("layout_probes="))
        .expect("the channel does not publish layout_probes");

    println!("layout_probes={published}");

    assert!(
        published.parse::<u32>().is_ok(),
        "layout_probes must be a count, not {published:?}"
    );

    assert!(control::KEYS.contains(&"layout_probes"));
    assert!(!control::RESERVED_KEYS.contains(&"layout_probes"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // T-10-1 appended the four `callback_` keys after this one, so "last" became a fixed
    // position — the same rewrite `focus_repeats`, `background_skips` and `device_changes`
    // each went through in turn.
    assert_eq!(
        control::KEYS.iter().position(|key| *key == "layout_probes"),
        Some(23)
    );
}

/// The key mirrors `watchdog`, and is not a number of the channel's own making.
#[test]
fn the_layout_probe_key_mirrors_the_counter_of_the_watchdog() {
    let before = lang_switcher::watchdog::counters().layout_probes;
    let state = control::snapshot();
    let after = lang_switcher::watchdog::counters().layout_probes;

    // The same bracket the two mirrors above use: a monotone counter cannot be pinned to
    // an equality across two reads while other tests of this binary run.
    assert!(
        state.layout_probes >= before && state.layout_probes <= after,
        "layout_probes is watchdog's own count of answered layout probes"
    );
}

// -------------------------------------------------------------------------------------
// Task T-10-5 — the key that shows the stamp FR-26 takes its direction from
// -------------------------------------------------------------------------------------

/// `active_layout` is published, it is a hexadecimal `HKL`, and it closes the tail of `KEYS`.
///
/// # Why it exists at all
///
/// Every key added to this channel before it is a **count**: how many probes were answered,
/// how many caches were built, how many handoffs were made. FR-26 takes the direction of
/// every conversion from the *stamp* — `Recorder::active`, copied into every stroke at
/// `buffer::Recorder::record` and read back at `inject::take_press` — and a count of the
/// events that were supposed to refresh that stamp cannot say what value it ended up
/// holding. The acceptance session's «первое нажатие моргает» is exactly a stale stamp, and
/// task T-10-5 had to measure the moment it diverges from the layout the foreground window
/// really runs. Without this key that moment can only be inferred; with it, it is read.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A layout handle, in the shape module `switch` already argues for in the open: "an `HKL` is
/// an identifier of a layout, not a keystroke, and is fair game". Asserted below as `0x` plus
/// exactly eight hex digits — a closed shape nothing of variable length can ride in.
#[test]
fn the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("active_layout="))
        .expect("the channel does not publish active_layout");

    println!("active_layout={published}");

    let digits = published
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("active_layout must be a hex HKL, not {published:?}"));

    assert!(
        digits.len() == 8 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "active_layout must be 0x plus eight hex digits, not {published:?}"
    );

    assert!(control::KEYS.contains(&"active_layout"));
    assert!(!control::RESERVED_KEYS.contains(&"active_layout"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // **T-10-6** appended `last_replacement` after it, task **T-10-8** appended
    // `last_replacement_method` after that, and task **T-10-9** appended three more, each
    // rewriting this line, as its predecessors each rewrote theirs.
    //
    // ⚠ Written as a **fixed index** now, and not as `len() - n`: the tail-relative form made
    // every appending task rewrite an assertion about a key it did not touch, which is the
    // rake `device_changes` already walked into (task T-10-0). The position of this key is a
    // constant of the channel; the length of the channel is not.
    assert_eq!(
        control::KEYS.iter().position(|key| *key == "active_layout"),
        Some(28)
    );
}

/// The key mirrors what was published into it, and it is a register rather than a counter.
///
/// The two halves are what a mirror owes: a value written is the value read back, and a value
/// nobody wrote does not appear. `note_active_layout` is the one writer — module `app` calls
/// it from `publish_active_layout` and only when the stamp really landed in a recorder — so
/// driving it directly here is driving the whole path this key has.
#[test]
fn the_active_layout_key_mirrors_what_was_published_into_it() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    // The two layouts of the acceptance configuration, US and RU. Values, not handles: this
    // test opens no layout and asks the system nothing.
    for layout in [0x0409_0409_usize, 0x0419_0419_usize] {
        control::note_active_layout(layout);

        assert_eq!(
            control::snapshot().active_layout,
            layout,
            "the snapshot reports the stamp that was published into it"
        );

        assert!(
            control::render(&control::snapshot())
                .contains(&format!("active_layout={layout:#010x}")),
            "and renders it as the hex HKL a reader compares against a layout handle"
        );
    }

    // Unlike every other key of this channel it does not accumulate: a stamp is a register,
    // and the second publication replaces the first rather than adding to it.
    control::note_active_layout(0x0409_0409);
    assert_eq!(control::snapshot().active_layout, 0x0409_0409);
}

// -------------------------------------------------------------------------------------
// Task T-10-6 — the key that tells a collapsed packet from a collapsed screen
// -------------------------------------------------------------------------------------

/// `last_replacement` is published, it is `erase/units/distinct`, and it closes the tail of
/// `KEYS`.
///
/// # Why it exists at all
///
/// The defect of task T-10-6 is a replacement of six different characters that reaches the screen
/// as six copies of one of them. Every key that existed said that press had gone perfectly:
/// `buffer_len=6`, `cycle_position` alternating, `send_mismatches=0`, `events_lost=0`. None of
/// them could separate «продукт построил схлопнутый пакет» from «продукт построил верный пакет,
/// а схлопнул его получатель», and those are defects of different modules — of `buffer` and
/// `convert` in the first case, of nothing this program owns in the second. `distinct` separates
/// them in one reading, and the answer it gave was **6**: the array handed to `SendInput` carried
/// six different characters on the very presses whose result on the screen was `тттттт`.
///
/// `erase` and `units` travel with it because the other half of the same report was «слово
/// стёрлось»: a packet whose erasure and insertion disagree is that report seen from inside.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// Three counts. Which characters the packet held is exactly what is not here, and «сколько среди
/// них различных» cannot be turned back into any of them. Asserted below as three decimal fields
/// separated by `/` — a closed shape nothing of variable length can ride in.
#[test]
fn the_last_replacement_key_of_t_10_6_is_published_as_three_counts() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("last_replacement="))
        .expect("the channel does not publish last_replacement");

    let fields: Vec<&str> = published.split('/').collect();
    assert_eq!(
        fields.len(),
        3,
        "last_replacement is erase/units/distinct, not {published:?}"
    );
    for field in &fields {
        assert!(
            !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit()),
            "every field is a decimal count, {field:?} is not"
        );
    }

    assert!(control::KEYS.contains(&"last_replacement"));
    assert!(!control::RESERVED_KEYS.contains(&"last_replacement"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // T-10-8 appended `last_replacement_method` after this one and task T-10-9 appended three
    // more, so "last" became a fixed position — the same rewrite every key of this tail has
    // gone through in turn, and it is written as a fixed index for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`.
    assert_eq!(
        control::KEYS
            .iter()
            .position(|key| *key == "last_replacement"),
        Some(29)
    );
}

/// The three counts mirror what was published, they do not accumulate, and they cannot tear.
///
/// The last property is the reason they share one atomic: they are compared **with each other**
/// — `erase` against `units`, `units` against `distinct` — so a reading that mixed one press with
/// another would be a wrong measurement rather than a stale one, which is not true of any other
/// key on this channel.
#[test]
fn the_last_replacement_key_mirrors_what_was_published_into_it() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    for (erase, units, distinct) in [(6, 6, 6), (6, 6, 1), (0, 3, 2)] {
        control::note_replacement(erase, units, distinct);

        let state = control::snapshot();
        assert_eq!(
            (
                state.replacement_erase,
                state.replacement_units,
                state.replacement_distinct
            ),
            (erase as u16, units as u16, distinct as u16),
            "the snapshot reports the shape that was published into it"
        );

        assert!(
            control::render(&state)
                .contains(&format!("last_replacement={erase}/{units}/{distinct}")),
            "and renders the three counts in that order"
        );
    }

    // A register, not a counter: the second publication replaces the first.
    control::note_replacement(2, 2, 2);
    assert_eq!(control::snapshot().replacement_erase, 2);

    // Each field saturates rather than wrapping into its neighbour — the ring of FR-07 holds 256
    // strokes, so nothing this program builds comes near it, and a value that wrapped would
    // corrupt the other two.
    control::note_replacement(usize::MAX, 1, 1);
    let state = control::snapshot();
    assert_eq!(state.replacement_erase, u16::MAX);
    assert_eq!(
        (state.replacement_units, state.replacement_distinct),
        (1, 1)
    );
}

/// **Step 5 of FR-40 reaches the stamp — the repair of task T-10-5, end to end on one thread.**
///
/// # What was broken
///
/// `inject::System::switch_layout` switched the foreground window and told nobody. None of the
/// four refresh paths of `app::publish_active_layout` fires for a switch made there — the focus
/// does not move, the user pressed no modifier, `WM_INPUTLANGCHANGE` cannot reach this process —
/// so `Recorder::active` kept the previous layout while the window ran the new one. Measured on
/// a live run as `active_layout=0x04090409` against a window on `0x04190419` with
/// `layout_probes` frozen across the press; felt as «первое нажатие моргает», because every
/// stroke of the next word was then stamped with the stale layout and FR-26 converted it into
/// the layout it was already typed in.
///
/// `app::note_layout_switched` is the far end of that repair. This drives it against a real
/// recorder installed on this thread and asserts both halves of what it owes: the stamp the
/// buffer will apply to the next stroke, and the mirror a run reads on the channel.
///
/// # What it must NOT do — FR-11, FR-32
///
/// A layout change is not a reason to throw away what the user typed, and the strokes already in
/// the ring must keep the layout each was recorded under — that is what makes the rollback of
/// FR-33 answer the same thing after this change as before it. Both are asserted below over a
/// buffer that is deliberately **not** empty when the switch is published.
#[test]
fn the_switch_of_fr40_step_five_reaches_the_stamp_without_touching_the_buffer() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    const US: LayoutId = LayoutId::from_raw(0x0409_0409);
    const RU: LayoutId = LayoutId::from_raw(0x0419_0419);

    buffer::install_recorder(Recorder::with_capacity(8));
    app::note_layout_switched(US);

    // Three strokes typed under US, exactly as a user would have them there when the hotkey
    // arrives: the switch of step 5 happens with a full buffer, never with an empty one.
    for _ in 0..3 {
        press(VK_A.0, SCAN_A);
    }

    assert_eq!(buffer::len(), 3);
    assert_eq!(
        buffer::with(|recorder| recorder.active_layout()),
        Some(US),
        "the stamp starts where the window was"
    );

    // Step 5 of FR-40, as `inject::System::switch_layout` now reports it.
    app::note_layout_switched(RU);

    assert_eq!(
        buffer::with(|recorder| recorder.active_layout()),
        Some(RU),
        "the stamp follows the switch the product itself made"
    );
    assert_eq!(
        control::snapshot().active_layout,
        RU.raw(),
        "and the channel says so, which is what made the defect measurable at all"
    );

    // FR-11: not one stroke was thrown away.
    assert_eq!(
        buffer::len(),
        3,
        "FR-11: a layout change does not flush the buffer, and this path has no reset on it"
    );

    // FR-32: the strokes keep the layout they were typed under, which is where
    // `inject::take_press` reads the direction of FR-26 from. If the publication reached them,
    // the rollback of FR-33 would answer a different layout from the second press onwards —
    // the very defect FR-32 warns about.
    for index in 0..3 {
        assert_eq!(
            buffer::with(|recorder| recorder.stroke(index).map(|stroke| stroke.hkl())),
            Some(Some(US)),
            "stroke {index} keeps the layout it was recorded under"
        );
    }

    // A new stroke, and only a new stroke, carries the new stamp.
    press(VK_A.0, SCAN_A);
    assert_eq!(
        buffer::with(|recorder| recorder.stroke(3).map(|stroke| stroke.hkl())),
        Some(Some(RU)),
        "the stroke made after the switch is the one that carries the new layout"
    );

    buffer::uninstall();
}

// -------------------------------------------------------------------------------------
// Task T-10-1 — the four keys of the callback-latency instrument, criterion 2 §13
// -------------------------------------------------------------------------------------

/// The four `callback_` keys are published, they are numbers, and they close the tail of
/// `KEYS` together, in the order samples → p50 → p99 → max.
///
/// # Why they exist at all
///
/// Criterion 2 of §13 requires the callback's latency **measured** — `QueryPerformanceCounter`
/// over at least 10 000 keystrokes — and the duration of a hook callback is not observable
/// from outside the process even in principle, which is the argument of решение Р-37 that put
/// `watchdog_recoveries` on this channel. Position 23 of §11.3 reads `callback_samples` to
/// know the sample was taken, and judges the three durations against NFR-01 (p99 < 100 000 нс)
/// and NFR-02 (max < 1 000 000 нс).
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// A count of invocations and three durations. Nothing about any stroke enters the instrument
/// — the probe starts before the callback reads the stroke and its drop knows only the clock.
/// Asserted below by parsing every value as a number and refusing anything else.
#[test]
fn the_callback_latency_keys_of_t_10_1_are_published_as_numbers() {
    let text = control::render(&control::snapshot());

    for key in [
        "callback_samples",
        "callback_p50_ns",
        "callback_p99_ns",
        "callback_max_ns",
    ] {
        let prefix = format!("{key}=");
        let published = text
            .lines()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .unwrap_or_else(|| panic!("the channel does not publish {key}"));

        println!("{key}={published}");

        assert!(
            published.parse::<u64>().is_ok(),
            "{key} must be a number, not {published:?}"
        );

        assert!(control::KEYS.contains(&key));
        assert!(!control::RESERVED_KEYS.contains(&key));
    }

    // Appended, not inserted — the rule every key since task T-05-2a has followed. The four
    // arrive together: one instrument, one block in the diff. They closed the list until task
    // T-10-5 appended `active_layout` after them, so "last" became a fixed position — the same
    // rewrite `layout_probes`, `focus_repeats`, `background_skips` and `device_changes` each
    // went through in turn.
    assert_eq!(
        &control::KEYS[24..28],
        &[
            "callback_samples",
            "callback_p50_ns",
            "callback_p99_ns",
            "callback_max_ns",
        ]
    );
}

/// The keys mirror the instrument in `hook`, and are not numbers of the channel's own making.
#[test]
fn the_callback_latency_keys_mirror_the_instrument_of_the_hook() {
    let before = lang_switcher::hook::callback_latency().samples;
    let state = control::snapshot();
    let after = lang_switcher::hook::callback_latency().samples;

    // The same bracket every mirror in this file uses: the sample count is monotone and
    // cannot be pinned to an equality across two reads while other tests of this binary run.
    assert!(
        state.callback_samples >= before && state.callback_samples <= after,
        "callback_samples is the instrument's own count of measured invocations"
    );

    // The percentile pair is ordered by construction — both are read off one histogram with
    // one rank rule. (p99 against the exact maximum is deliberately NOT asserted: the
    // percentiles are upper bounds of their cells and the maximum is exact, so a run whose
    // every sample sits in one cell reports p99 one cell above max — documented in
    // `hook::CallbackLatency`.)
    assert!(
        state.callback_p50_ns <= state.callback_p99_ns,
        "the median cannot exceed the 99th percentile of the same histogram"
    );

    // No instrument has run in this test process — no hook is ever installed here (the
    // header of tests\hook.rs says why) — so the published reading is the empty one: zeros
    // across all four keys, not garbage and not a stale cell.
    if state.callback_samples == 0 {
        assert_eq!(
            (
                state.callback_p50_ns,
                state.callback_p99_ns,
                state.callback_max_ns
            ),
            (0, 0, 0),
            "an empty instrument must publish zeros, not remnants"
        );
    }
}

// -------------------------------------------------------------------------------------
// Task T-10-8 — the key that shows which packet FR-42а chose for a given press
// -------------------------------------------------------------------------------------

/// `last_replacement_method` is published, it is one of a closed set of words, and it closes
/// the tail of `KEYS`.
///
/// # Why it exists at all
///
/// FR-42а made the configured method a *rule*: `replacement_method=auto` says how the choice
/// is made and nothing about what any given press chose. The live evidence of task T-10-8 —
/// «в Блокноте пошёл путь выделения, в cmd — путь Backspace» — is a statement about that
/// per-press choice, and without this key it is a statement nothing outside the process can
/// verify: both packets erase and retype the same characters, and `last_replacement`'s three
/// counts are deliberately packet-shape only.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// One word of a closed set about the program's own decision. The window class the decision
/// was made from is exactly what is NOT here — asserted below by refusing every value outside
/// the three words.
#[test]
fn the_last_replacement_method_key_of_t_10_8_is_published_as_a_closed_word() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("last_replacement_method="))
        .expect("the channel does not publish last_replacement_method");

    println!("last_replacement_method={published}");

    assert!(
        published == "none" || published == "backspace" || published == "selection",
        "last_replacement_method is a resolved method or none, not {published:?}"
    );

    assert!(control::KEYS.contains(&"last_replacement_method"));
    assert!(!control::RESERVED_KEYS.contains(&"last_replacement_method"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed. Task
    // T-10-9 appended three more after this one, so this is a fixed position now, for the
    // reason given at `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`.
    assert_eq!(
        control::KEYS
            .iter()
            .position(|key| *key == "last_replacement_method"),
        Some(30)
    );
}

// -------------------------------------------------------------------------------------
// Task T-10-9 — the three flush counters that only the file sink used to answer
// -------------------------------------------------------------------------------------

/// `window_flushes`, `full_clears` and `strokes_removed` are published, they are decimal
/// counts, and they close the tail of `KEYS` in that order.
///
/// # Why they exist at all
///
/// All three live in `watchdog::Counters` and were published through the file report of
/// `LANGSW_TESTING_REPORT` **only** — a sink the main thread writes *after the process has
/// exited*. Defect D of the final acceptance session («как только я открыл проводник
/// переключение перестало работать везде») is a question about a process that has to keep
/// running across the step under investigation, so the file sink cannot answer it even in
/// principle, and these three are exactly the numbers that separate the two candidate
/// mechanisms: strokes that reach the buffer and are erased by flushes move all three, and
/// strokes that never reach the buffer leave all three standing while `buffer_len` stays at
/// zero. The same standing `background_skips`, `focus_repeats` and `layout_probes` were moved
/// onto this channel for by tasks T-10-0, T-10-0e and T-10-0f.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// Three counts. `strokes_removed` counts *removals of* strokes and carries nothing of any
/// stroke — no scan code, no character, no layout — which is asserted below by refusing every
/// value that is not a bare decimal.
#[test]
fn the_three_flush_keys_of_t_10_9_are_published_as_decimal_counts_at_the_tail() {
    let text = control::render(&control::snapshot());

    for key in ["window_flushes", "full_clears", "strokes_removed"] {
        let published = text
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("the channel does not publish {key}"));

        println!("{key}={published}");

        assert!(
            !published.is_empty() && published.bytes().all(|byte| byte.is_ascii_digit()),
            "{key} must be a decimal count, not {published:?}"
        );

        assert!(control::KEYS.contains(&key));
        assert!(!control::RESERVED_KEYS.contains(&key));
    }

    // Appended in that order, and at the end — the rule every key since task T-05-2a has
    // followed. Written against fixed indices for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`.
    assert_eq!(
        &control::KEYS[31..34],
        &["window_flushes", "full_clears", "strokes_removed"]
    );
}

/// The three keys mirror `watchdog::counters()` and are not invented here.
///
/// The channel is a mirror of the module that owns the numbers (decision Р-28: one source,
/// two sinks), so the assertion that matters is not "some number appears" but "the number that
/// appears is the one `watchdog` holds". Read twice around the snapshot, because the program
/// keeps running: the published value has to lie between the two live reads, monotone counters
/// being what they are.
#[test]
fn the_three_flush_keys_of_t_10_9_mirror_the_watchdog_counters() {
    let before = watchdog::counters();
    let state = control::snapshot();
    let after = watchdog::counters();

    for (published, low, high) in [
        (
            state.window_flushes,
            before.window_flushes,
            after.window_flushes,
        ),
        (state.full_clears, before.full_clears, after.full_clears),
        (
            state.strokes_removed,
            before.strokes_removed,
            after.strokes_removed,
        ),
    ] {
        assert!(
            (low..=high).contains(&published),
            "the channel publishes what watchdog holds: {published} is outside {low}..={high}"
        );
    }
}

// -------------------------------------------------------------------------------------
// Task T-10-10 — the gate's own state, the blind spot the three flush counters left
// -------------------------------------------------------------------------------------

/// `field_state` is published, it is one of the four closed words of `guard::Field::name`,
/// and it closes the tail of `KEYS`.
///
/// # Why it exists at all
///
/// `password_field` — the flag SEC-06 requires — is `0` for **two different situations**:
/// `Field::Ordinary`, where buffering is on, and `Field::Pending`, where the focus has moved,
/// no verdict exists yet, buffering is off and `app::park_buffer` has already emptied the
/// buffer through `buffer::reset()`. That path goes *past* the flush counters, so
/// `strokes_removed` does not move either, and on the channel the two situations read
/// identically: `password_field=0`, `buffer_len=0`, `strokes_removed` standing. That is the
/// first of the four rows of the break-down table of task T-10-9 — «нажатия не доходят до
/// буфера, потому что ворота держат буфер снятым» — and it was the one row of four that a
/// **live** process could not be asked about directly. `window_flushes`, `full_clears` and
/// `strokes_removed` separated «буфер стирают сбросы» from «в буфер ничего не попадает»; they
/// cannot say why nothing is going in.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// **A word, and a word out of a closed list.** This is the state of the gate, never the
/// content of the field — the contents of a password field are read nowhere in this program.
/// The assertion below is membership of the four words of `guard::Field::name` and nothing
/// weaker: any fifth value, however harmless-looking, fails it.
#[test]
fn the_field_state_key_of_t_10_10_is_one_of_four_closed_words_at_the_tail() {
    let text = control::render(&control::snapshot());

    let published = text
        .lines()
        .find_map(|line| line.strip_prefix("field_state="))
        .expect("the channel does not publish field_state");

    println!("field_state={published}");

    assert!(
        ["pending", "ordinary", "password", "undetermined"].contains(&published),
        "field_state must be one of the four words of guard::Field::name, not {published:?}"
    );

    assert!(control::KEYS.contains(&"field_state"));
    assert!(!control::RESERVED_KEYS.contains(&"field_state"));

    // Appended, not inserted — the rule every key since task T-05-2a has followed, written
    // against a fixed index for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`.
    assert_eq!(control::KEYS[34], "field_state");
}

/// Every one of the four states of `guard::Field` renders as its own word, and `password_field`
/// keeps saying what it said.
///
/// The channel is a mirror (decision Р-28), so the assertion that matters is not "some word
/// appears" but "the word that appears is the one the state names". Driven over a `Snapshot`
/// taken live and then re-pointed at each arm in turn, because the four arms cannot all be
/// produced on a real gate inside one test: `Field::Pending` exists only for the few
/// milliseconds between a focus change and the verdict of its probe.
///
/// The second half is the point of the key. `password_field` is `0` for **both** `Ordinary`
/// and `Pending`, and this asserts that it still is — the flag of SEC-06 is not restated by
/// the new key and not replaced by it. `field_state` is what tells those two apart.
#[test]
fn the_field_state_key_names_each_of_the_four_states_and_leaves_the_flag_alone() {
    let mut state = control::snapshot();

    for (field, word) in [
        (Field::Pending, "pending"),
        (Field::Ordinary, "ordinary"),
        (Field::Password, "password"),
        (Field::Undetermined, "undetermined"),
    ] {
        state.field_state = field;

        assert_eq!(field.name(), word, "guard names the state {word}");
        assert!(
            control::render(&state).contains(&format!("field_state={word}\n")),
            "the channel prints the word guard gives it for {word}"
        );
    }

    // The blind spot itself, asserted rather than described: the flag cannot tell these two
    // apart, and the new key can.
    for field in [Field::Pending, Field::Ordinary] {
        state.field_state = field;
        state.password_field = false;
        assert!(
            control::render(&state).contains("password_field=0\n"),
            "the flag of SEC-06 reads 0 for {} exactly as it did before",
            field.name()
        );
    }
    assert_ne!(
        Field::Pending.name(),
        Field::Ordinary.name(),
        "and the new key is what separates them"
    );
}

/// The key mirrors what `guard::field()` holds and is not invented by the channel.
///
/// Read twice around the snapshot: unlike the counters this is not monotone, so the bracket
/// used elsewhere in this file does not apply. What is asserted instead is the property that
/// does hold — the published word is the word of *some* state `guard` was in, and if the gate
/// did not move across the three reads it is the word of that state exactly.
#[test]
fn the_field_state_key_mirrors_what_guard_holds() {
    let before = lang_switcher::guard::field();
    let state = control::snapshot();
    let after = lang_switcher::guard::field();

    if before == after {
        assert_eq!(
            state.field_state, before,
            "the channel publishes the state guard holds"
        );
    }

    assert!(
        ["pending", "ordinary", "password", "undetermined"].contains(&state.field_state.name()),
        "and it is a word of the closed list either way"
    );
}

// -------------------------------------------------------------------------------------
// Task T-10-11 — the selection path, which had not one number on this channel
// -------------------------------------------------------------------------------------

/// The three `clipboard_` keys are published, they are decimal counts, and they close the tail
/// of `KEYS`.
///
/// # Why they exist at all
///
/// FR-42а resolves the configured `auto` **to `selection` for every non-console window** — a
/// Блокнот, a Telegram, a Word, a Chrome, a VS Code — so the selection path of FR-60/FR-61 is
/// the path almost everything the user types goes through, and it replaces a word by selecting
/// it and passing it through the clipboard. Until this task that path published **not one
/// number**. The break-down table of task T-10-9 has four rows, and its fourth — «замена
/// отказывает» — could be read only as `last_replacement=0/0/0`: the statement that a
/// replacement did not run, never the reason. These three are the reason:
/// `clipboard_refusals` is the selection path refusing outright, `clipboard_close_failures` is
/// the one failure that **lasts** — a clipboard this process opened and could not close refuses
/// everybody afterwards, which is the shape «переключение перестало работать везде» has — and
/// `clipboard_retries` is the gradient between the two.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// Three counts of program events, asserted below by refusing every value that is not a bare
/// decimal. Not the text of the clipboard, not its format, not its size: `selection::snapshot`
/// and `selection::restore` publish nothing of what they carry, and none of these numbers is
/// proportional to it or can be turned back into it.
#[test]
fn the_three_clipboard_keys_of_t_10_11_are_published_as_decimal_counts_at_the_tail() {
    let text = control::render(&control::snapshot());

    for key in [
        "clipboard_refusals",
        "clipboard_close_failures",
        "clipboard_retries",
    ] {
        let published = text
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("the channel does not publish {key}"));

        println!("{key}={published}");

        assert!(
            !published.is_empty() && published.bytes().all(|byte| byte.is_ascii_digit()),
            "{key} must be a decimal count, not {published:?}"
        );

        assert!(control::KEYS.contains(&key));
        assert!(!control::RESERVED_KEYS.contains(&key));
    }

    // Appended in that order — the rule every key since task T-05-2a has followed, written
    // against fixed indices for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`.
    //
    // ⚠ The length assertion that used to stand here has moved to
    // `the_five_restamp_keys_of_t_10_15_separate_every_outcome_in_one_snapshot`: it travels with
    // the key that currently **closes** the list, so that a task which appends one is the task
    // that edits it. Task T-10-15 appended five, so these three no longer close anything.
    assert_eq!(
        &control::KEYS[35..38],
        &[
            "clipboard_refusals",
            "clipboard_close_failures",
            "clipboard_retries"
        ]
    );
}

/// The three keys mirror `selection::counters()` and are not invented here.
///
/// The same assertion the flush keys of T-10-9 are held to, for the same reason (decision Р-28:
/// one source, two sinks): what matters is not that a number appears but that the number which
/// appears is the one `selection` holds. Read twice around the snapshot, because the program
/// keeps running and all three are monotone counters — the published value has to lie between
/// the two live reads.
///
/// ⚠ **What this test does not prove, said plainly.** On a quiet process every one of the
/// thirteen counters of `selection::Counters` reads zero, so a channel that mirrored the wrong
/// field of the right struct would pass here. The bracket is a mirror check, not a pairing
/// check. What pins the pairing is the run itself: the key that has to be right during the
/// experiment is `clipboard_refusals`, and its meaning is read off a live product whose
/// selection path is running — no unit test on an idle process can stand in for that, and this
/// one does not pretend to.
#[test]
fn the_three_clipboard_keys_of_t_10_11_mirror_the_selection_counters() {
    let before = lang_switcher::selection::counters();
    let state = control::snapshot();
    let after = lang_switcher::selection::counters();

    for (name, published, low, high) in [
        (
            "clipboard_refusals",
            state.clipboard_refusals,
            before.open_refusals,
            after.open_refusals,
        ),
        (
            "clipboard_close_failures",
            state.clipboard_close_failures,
            before.close_failures,
            after.close_failures,
        ),
        (
            "clipboard_retries",
            state.clipboard_retries,
            before.open_retries,
            after.open_retries,
        ),
    ] {
        assert!(
            (low..=high).contains(&published),
            "the channel publishes what selection holds: {name} is {published}, outside {low}..={high}"
        );
    }
}

/// The key mirrors what `inject::on_hotkey` publishes into it, a register and not a counter —
/// and a value no resolution can produce is recorded as `none` rather than invented.
#[test]
fn the_last_replacement_method_key_mirrors_what_was_published_into_it() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    for (method, word) in [
        (ReplacementMethod::Backspace, "backspace"),
        (ReplacementMethod::Selection, "selection"),
    ] {
        control::note_replacement_method(method);

        let state = control::snapshot();
        assert_eq!(
            state.last_replacement_method, word,
            "the snapshot reports the resolved method that was published into it"
        );
        assert!(
            control::render(&state).contains(&format!("last_replacement_method={word}\n")),
            "and renders it as that word"
        );
    }

    // `Auto` cannot be the outcome of a resolution — `inject::effective_method` never returns
    // it — and if a defect ever delivered it here anyway, the honest answer is "nothing
    // decidable ran", not a decision the program did not make.
    control::note_replacement_method(ReplacementMethod::Auto);
    assert_eq!(control::snapshot().last_replacement_method, "none");
}

// -------------------------------------------------------------------------------------
// Task T-10-15 — the five outcomes of the repair of defect E, which were one reading
// -------------------------------------------------------------------------------------

/// The layouts of the synthetic cache below. The same values `tests\buffer.rs` uses, and for the
/// same reason: a test states its premise instead of depending on what the keyboard of the
/// machine running it happens to be doing.
const EN: LayoutId = LayoutId::from_raw(0x0409_0409);
/// The other half of the cache.
const RU: LayoutId = LayoutId::from_raw(0x0419_0419);
/// A layout the cache of FR-20 does **not** hold — the premise of the silent refusal.
const NOT_IN_CACHE: LayoutId = LayoutId::from_raw(0x0407_0407);

/// A cache of FR-20 holding exactly [`EN`] and [`RU`], with one key on each.
///
/// One key is enough: nothing here decodes anything, and what the refusal turns on is
/// `LayoutCache::contains`, which is a question about the *set of layouts* and not about the
/// contents of a map.
fn two_layout_cache() -> LayoutCache {
    let map_of = |layout: LayoutId, character: char| {
        let mut builder = LayoutMapBuilder::new(layout);
        builder.set(SCAN_A, false, Mods::NONE, KeyMapping::from_char(character));
        builder.finish()
    };

    LayoutCache::from_maps(vec![map_of(EN, 'a'), map_of(RU, 'ф')]).expect("two non-empty maps")
}

/// The probe of a machine really running [`RU`] — the layout the cache **does** hold.
fn probe_russian() -> LayoutId {
    RU
}

/// The probe of a machine really running [`EN`] — the layout already stamped.
fn probe_english() -> LayoutId {
    EN
}

/// No foreground window at all: the desktop is switching, or this is the secure desktop.
fn probe_nothing() -> LayoutId {
    LayoutId::default()
}

/// ⭐ The probe of a machine whose real layout the cache has no map for.
fn probe_uncached() -> LayoutId {
    NOT_IN_CACHE
}

/// The change in the five counts, as a difference — the reading this channel is diffed for.
fn restamp_delta(before: [u32; 5], after: [u32; 5]) -> [i64; 5] {
    core::array::from_fn(|index| i64::from(after[index]) - i64::from(before[index]))
}

/// Installs a recorder with the cache of FR-20, [`EN`] stamped, and the given probe.
fn recorder_with(probe: buffer::LayoutProbe) {
    let mut recorder = Recorder::with_capacity(8);
    recorder.set_cache(two_layout_cache());
    recorder.set_active_layout(EN);
    recorder.stamp_layout_with(probe);
    buffer::install_recorder(recorder);
}

/// ⭐ **Every outcome of `Recorder::restamp` is a different reading of one snapshot** — the whole
/// of criterion 9 of task T-10-15, driven through the real recorder rather than through
/// `note_restamp`.
///
/// # Why this is the test and five calls of `note_restamp` are not
///
/// The publisher is trivial and could not be wrong in an interesting way. What could — and what
/// three repairs in a row were misled by — is the **pairing**: which branch of the repair
/// publishes which count. A test that called the publisher five times would assert that five
/// atomics can be incremented, and would go on passing if the branches were wired to each other's
/// counters. So every arm below builds the *premise of the branch* — a recorder with or without a
/// probe; a probe answering a layout the cache holds, does not hold, or already stamps — presses
/// a key through `buffer::record`, and reads the channel.
///
/// # ⚠ The negative half of each arm
///
/// Each arm asserts the whole vector of five and not only its own entry, so a branch that
/// published *two* counts, or published somebody else's, fails here. That is the same demand the
/// instrument of this task's experiment is held to: a reading is worth something only when the
/// values it must **not** take are pinned as well.
#[test]
fn the_five_restamp_keys_of_t_10_15_are_paired_with_the_branches_that_publish_them() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    // ---- outcome 1: not called at all, because the ring is not empty ---------------------
    // The recorder has both a probe and a cache, so the *only* thing keeping the branch shut is
    // the ring — which is what this outcome means.
    recorder_with(probe_russian);

    press(VK_A.0, SCAN_A);
    assert_eq!(
        buffer::with(|recorder| recorder.len()),
        Some(1),
        "the first stroke of the word is in the ring, so the next one is not a first stroke"
    );

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [1, 0, 0, 0, 0],
        "a stroke that is not the first of a word moves restamp_skips and nothing else"
    );

    // ---- outcome 2: called, and there is no probe ----------------------------------------
    buffer::install_recorder(Recorder::with_capacity(8));

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [0, 1, 0, 0, 0],
        "a recorder without a probe moves restamp_no_probe and nothing else"
    );

    // ---- outcome 3: the probe answers with the layout already stamped ---------------------
    recorder_with(probe_english);

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [0, 0, 1, 0, 0],
        "a stamp that already agrees moves restamp_unchanged and nothing else"
    );
    assert_eq!(
        buffer::with(|recorder| recorder.active_layout()),
        Some(EN),
        "and leaves the stamp exactly where it was"
    );

    // ---- outcome 3 again: no foreground window at all -------------------------------------
    // The secure desktop and the interval of a desktop switch both answer this way, and the
    // experiment of this task walks straight through them — so the arm is measured, not assumed.
    recorder_with(probe_nothing);

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [0, 0, 1, 0, 0],
        "an empty answer is the same outcome and is counted as one"
    );

    // ---- outcome 4: ⭐ the silent refusal — the cache has no map for what the probe read ---
    recorder_with(probe_uncached);

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [0, 0, 0, 1, 0],
        "a layout the cache of FR-20 does not hold moves restamp_uncached and nothing else"
    );
    assert_eq!(
        buffer::with(|recorder| recorder.active_layout()),
        Some(EN),
        "and the stamp stays stale — which is the defect this key was added to make visible"
    );

    // ---- outcome 5: the repair doing its work ---------------------------------------------
    recorder_with(probe_russian);

    let before = control::snapshot().restamp;
    press(VK_A.0, SCAN_A);
    assert_eq!(
        restamp_delta(before, control::snapshot().restamp),
        [0, 0, 0, 0, 1],
        "a layout the cache holds moves restamp_accepted and nothing else"
    );
    assert_eq!(
        buffer::with(|recorder| recorder.active_layout()),
        Some(RU),
        "and the stamp really was corrected"
    );

    // Leave a plain recorder behind — the shape every other test of this binary installs for
    // itself anyway, and one that holds neither a probe nor a cache of ours.
    buffer::install_recorder(Recorder::with_capacity(8));
}

/// The five keys are published, they are decimal counts, and they close the tail of `KEYS`.
///
/// # Why five and not one
///
/// The repair of defect E has five ways to end and **four of them leave the stamp exactly as it
/// was**, so `active_layout` — the only key that said anything about the stamp — reads the same
/// for all four. Task T-10-15 was given a defect three repairs in a row had missed, and the
/// instrument it started from could not tell «перештамповка не звалась» from «звалась и отказала,
/// потому что карты раскладки нет в кэше FR-20». Those are different defects in different modules
/// and they now differ by one line of one snapshot.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// Five counts of the program's own decisions, asserted below by refusing every value that is not
/// a bare decimal. Not a character, not a scan code, not a stroke — and deliberately not the
/// layout that was refused either.
#[test]
fn the_five_restamp_keys_of_t_10_15_separate_every_outcome_in_one_snapshot() {
    let text = control::render(&control::snapshot());

    for key in [
        "restamp_skips",
        "restamp_no_probe",
        "restamp_unchanged",
        "restamp_uncached",
        "restamp_accepted",
    ] {
        let published = text
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("the channel does not publish {key}"));

        println!("{key}={published}");

        assert!(
            !published.is_empty() && published.bytes().all(|byte| byte.is_ascii_digit()),
            "{key} must be a decimal count, not {published:?}"
        );

        assert!(control::KEYS.contains(&key));
        assert!(!control::RESERVED_KEYS.contains(&key));
    }

    // Appended in that order, and at the end — the rule every key since task T-05-2a has
    // followed, written against fixed indices for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`. The length
    // assertion travels with the last-appended key: it is the check that no key reached the
    // channel without a review, and its place is beside the key that currently closes the
    // list, so that a task which adds one is the task that edits it. Task **T-10-17** appended
    // two more after these five, so the length now travels with
    // `the_two_replacement_outcome_keys_of_t_10_17_close_the_channel`.
    assert_eq!(
        &control::KEYS[38..43],
        &[
            "restamp_skips",
            "restamp_no_probe",
            "restamp_unchanged",
            "restamp_uncached",
            "restamp_accepted"
        ]
    );
}

// -------------------------------------------------------------------------------------
// ⭐ Task T-10-17 — the two keys that say whether the replacement returned what it took,
// and in which direction it went
// -------------------------------------------------------------------------------------

/// An [`Environment`] that swallows everything — the replacement is driven for what it
/// **publishes**, not for what it sends.
///
/// Deliberately the smallest possible: `tests\inject.rs` owns the bench that inspects packets, and
/// a second copy of it here would be a second thing to keep in step. What these tests need is a
/// real call of `inject::replace_in_with` — the one place the packet is formed and therefore the
/// one place the two new keys are published — with nothing reaching the machine.
struct Silent;

impl Environment for Silent {
    fn held(&mut self) -> Modifiers {
        Modifiers::NONE
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        // FR-45: the honest answer of a system that accepted everything, so `send_mismatches`
        // stays where it was and no other key of this channel moves under these tests.
        u32::try_from(events.len()).unwrap_or(u32::MAX)
    }

    fn pause(&mut self, _delay_ms: u32) {}
}

/// The hardwired US half of the FR-25 table — the layout the strokes below are recorded under.
fn fallback_us() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_US).expect("the FR-25 table carries US")
}

/// The hardwired Russian half of the FR-25 table.
fn fallback_russian() -> LayoutMap {
    convert::fallback_map(convert::FALLBACK_RUSSIAN).expect("the FR-25 table carries Russian")
}

/// The six strokes of `ghbdtn` as the buffer records them under US — `привет` in Russian, the
/// example section 1 of SPEC opens with and the six characters the live protocol of defect E is
/// made of.
fn six_strokes_under_us() -> Vec<Keystroke> {
    let source = fallback_us();

    [0x22, 0x23, 0x30, 0x20, 0x14, 0x31]
        .iter()
        .map(|&scan| Keystroke::recorded_in(&source, scan, false, Mods::NONE))
        .collect()
}

/// What the channel says about the last replacement, as one tuple.
fn replacement_reading() -> (u16, u16, u16, bool, u32, u32) {
    let state = control::snapshot();

    (
        state.replacement_erase,
        state.replacement_units,
        state.replacement_distinct,
        state.last_replacement_changed,
        state.last_replacement_from,
        state.last_replacement_to,
    )
}

/// ⭐ **The instrument, shown in both of its states — task T-10-17, requirement 2.**
///
/// # What was missing and why this is the test
///
/// The live protocol of defect E reads `last_replacement=6/6/6` on **all thirty-four** presses of
/// a text the person watched not change, with the stamp demonstrably correct (`restamp_unchanged`
/// twenty-three times, `restamp_accepted` never). `6/6/6` is the same reading for a press that
/// rewrote six characters and for a press that handed back the six it took: the shape of a packet
/// says how much moved, never whether anything did. Four repairs in a row were guesses because
/// this reading did not exist.
///
/// A flag shown in one state is not a flag that has been checked — it is a constant nobody has
/// caught yet. So both circles are driven here through the real `inject::replace_in_with`, the one
/// place a packet is formed, and the whole tuple is asserted each time so that a publication
/// landing in the wrong key colours the test:
///
/// | Circle | Target | What the packet does | `changed` | direction |
/// |---|---|---|---|---|
/// | conversion | Russian | `ghbdtn` becomes `привет` | **`true`** | `0x04090409` to `0x04190419` |
/// | identity | US, the layout the strokes were typed in | `ghbdtn` stays `ghbdtn` | **`false`** | `0x04090409` to `0x04090409` |
///
/// The second circle is not a contrivance: it is exactly what `Cycle::target` answers whenever
/// `step % len == 0` — the rollback of FR-32 and FR-33, which the docblock of that function
/// describes as reproducing the original text code unit for code unit. Twelve of the person's
/// thirty-four presses were that press, and until these two keys nothing on the channel could say
/// so.
#[test]
fn the_two_replacement_outcome_keys_of_t_10_17_read_both_ways() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    let strokes = six_strokes_under_us();
    let us = LayoutId::from_raw(0x0409_0409);
    let russian = LayoutId::from_raw(0x0419_0419);
    let hkl = |id: LayoutId| u32::try_from(id.raw()).expect("an HKL is a thirty-two bit word");

    // ---- circle 1: the replacement really rewrites the word -------------------------------
    inject::replace_in_with(
        &mut Silent,
        &strokes,
        &fallback_russian(),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("the packet is sized from the lengths it is built with");

    assert_eq!(
        replacement_reading(),
        (6, 6, 6, true, hkl(us), hkl(russian)),
        "six off, six back, six of them different, the insertion differs, and the direction is \
         the one FR-26 read out of the strokes"
    );

    // ---- circle 2: the replacement returns what it took ------------------------------------
    //
    // The same six strokes rendered into the layout they were typed under — the identity
    // `Cycle::target` produces on the rollback press.
    inject::replace_in_with(
        &mut Silent,
        &strokes,
        &fallback_us(),
        0,
        ReplacementMethod::Backspace,
    )
    .expect("the packet is sized from the lengths it is built with");

    assert_eq!(
        replacement_reading(),
        (6, 6, 6, false, hkl(us), hkl(us)),
        "the shape is the same 6/6/6 the protocol of the person shows, and the two new keys are \
         what separate this press from the one above it"
    );

    // ⚠ The two circles differ **only** in the target, and the three counts that existed before
    // this task are identical across them. That equality is the whole argument for the keys:
    // without it a reader could claim `distinct` already answered the question.
    let state = control::snapshot();
    assert_eq!(
        (
            state.replacement_erase,
            state.replacement_units,
            state.replacement_distinct
        ),
        (6, 6, 6)
    );
}

/// The pair is a register and not a counter, the halves cannot tear, and each of them saturates
/// rather than truncating into the other.
///
/// Driven through the publisher directly, as
/// `the_last_replacement_key_mirrors_what_was_published_into_it` is: the test above proves the
/// product publishes the right values, this one proves the storage carries whatever it is given.
#[test]
fn the_replacement_direction_of_t_10_17_mirrors_what_was_published_into_it() {
    let _serialised = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);

    for (changed, from, to) in [
        (true, 0x0409_0409_usize, 0x0419_0419_usize),
        (false, 0x0419_0419, 0x0419_0419),
        (true, 0, 0x0413_0413),
    ] {
        control::note_replacement_outcome(changed, from, to);

        let state = control::snapshot();
        assert_eq!(
            (
                state.last_replacement_changed,
                state.last_replacement_from,
                state.last_replacement_to
            ),
            (changed, from as u32, to as u32),
            "the snapshot reports the outcome that was published into it"
        );

        let text = control::render(&state);
        assert!(
            text.contains(&format!(
                "last_replacement_direction={:#010x}/{:#010x}",
                from as u32, to as u32
            )),
            "and renders the two halves in the order from, then to"
        );
        assert!(
            text.contains(&format!("last_replacement_changed={}", u8::from(changed))),
            "and the flag as one bit"
        );
    }

    // A register: the second publication replaces the first, it does not accumulate.
    control::note_replacement_outcome(false, 0x0409_0409, 0x0409_0409);
    let state = control::snapshot();
    assert!(!state.last_replacement_changed);
    assert_eq!(
        (state.last_replacement_from, state.last_replacement_to),
        (0x0409_0409, 0x0409_0409)
    );

    // Each half saturates rather than wrapping into its neighbour. No keyboard layout handle is
    // wider than thirty-two bits, so this is unreachable in the product; a half that wrapped
    // would quietly report a **different layout**, which is the one failure this pair must never
    // have.
    control::note_replacement_outcome(true, usize::MAX, 0x0419_0419);
    let state = control::snapshot();
    assert_eq!(state.last_replacement_from, u32::MAX);
    assert_eq!(state.last_replacement_to, 0x0419_0419);
}

/// The two keys are published, they close the tail of `KEYS`, and the length assertion lives here
/// now.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// One bit and two layout handles. The bit is the *comparison* of two texts and neither text nor
/// any part of one is published; the handles are of exactly the kind `active_layout` has published
/// since task T-10-5 — "an `HKL` is an identifier of a layout, not a keystroke". Both shapes are
/// held closed by `every_value_is_a_number_or_one_of_the_two_words_of_fr42`: a bit falls under the
/// decimal rule, and each half of the direction must be `0x` and exactly eight hex digits, so
/// nothing of variable length can ride in either.
#[test]
fn the_two_replacement_outcome_keys_of_t_10_17_close_the_channel() {
    let text = control::render(&control::snapshot());

    let changed = text
        .lines()
        .find_map(|line| line.strip_prefix("last_replacement_changed="))
        .expect("the channel does not publish last_replacement_changed");
    assert!(
        changed == "0" || changed == "1",
        "last_replacement_changed is one bit, not {changed:?}"
    );

    let direction = text
        .lines()
        .find_map(|line| line.strip_prefix("last_replacement_direction="))
        .expect("the channel does not publish last_replacement_direction");
    let halves: Vec<&str> = direction.split('/').collect();
    assert_eq!(
        halves.len(),
        2,
        "last_replacement_direction is from/to, not {direction:?}"
    );

    for key in ["last_replacement_changed", "last_replacement_direction"] {
        assert!(control::KEYS.contains(&key));
        assert!(!control::RESERVED_KEYS.contains(&key));
    }

    // Appended, not inserted — the rule every key since task T-05-2a has followed, written
    // against fixed indices for the reason given at
    // `the_active_layout_key_of_t_10_5_is_published_as_a_hex_layout_handle`. The length
    // assertion travels with the last-appended key, so a task that adds one is the task that
    // edits it; it moved here from
    // `the_five_restamp_keys_of_t_10_15_separate_every_outcome_in_one_snapshot`.
    assert_eq!(
        &control::KEYS[43..45],
        &["last_replacement_changed", "last_replacement_direction"]
    );
    assert_eq!(control::KEYS.len(), 45);
}
