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
use windows::Win32::System::Pipes::PIPE_REJECT_REMOTE_CLIENTS;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_A, VK_BACK, VK_RETURN};

/// Serialises the two tests that assert on the process-wide length mirror.
///
/// The mirror is one atomic for the whole process and the typing buffer is a thread-local, so
/// two tests running in parallel would be asserting on each other's timing rather than on the
/// code. Each of the two holds this for its whole body; nothing else in the file touches the
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
/// which is one of the two words section 7 defines. There is no third shape for anything to
/// hide in.
#[test]
fn every_value_is_a_number_or_one_of_the_two_words_of_fr42() {
    let text = control::render(&control::snapshot());

    assert!(
        text.is_ascii(),
        "the payload is ASCII, so no decoded character can be riding in it"
    );

    for line in text.lines() {
        let (key, value) = line.split_once('=').expect("key=value");

        if key == "replacement_method" {
            assert!(
                value == "backspace" || value == "selection",
                "replacement_method is one of the two words of section 7"
            );
            continue;
        }

        assert!(
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
            "{key} is a plain decimal count, and {value:?} is not"
        );
    }
}

/// **The two reserved keys are missing, not zero.** SEC-06, and the shape the future needs.
///
/// A bench that read `password_field=0` would record "no buffering in a password field" for a
/// build that cannot tell. Absence is the honest answer until task T-06-1 (`password_field`)
/// and task T-05-2 (`cycle_position`) arrive.
#[test]
fn the_reserved_keys_are_absent_rather_than_answered_with_a_zero() {
    let text = control::render(&control::snapshot());

    assert_eq!(
        control::RESERVED_KEYS,
        ["password_field", "cycle_position"],
        "the reserved list names the two keys SEC-04a reserves"
    );

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
