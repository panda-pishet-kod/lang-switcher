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

use lang_switcher::buffer::{self, Recorder};
use lang_switcher::control;
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::settings::ReplacementMethod;
use lang_switcher::watchdog;
use windows::Win32::System::Pipes::PIPE_REJECT_REMOTE_CLIENTS;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_A, VK_BACK, VK_RETURN};

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

    for line in text.lines() {
        let (key, value) = line.split_once('=').expect("key=value");

        if key == "replacement_method" {
            assert!(
                value == "backspace" || value == "selection",
                "replacement_method is one of the two words of section 7"
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

    // Appended, not inserted — the rule every key since task T-05-2a has followed.
    assert_eq!(control::KEYS[control::KEYS.len() - 1], "device_changes");
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
