//! **Task T-95-6, решение 159.25** — the keyboard shortcut of the sticky keys of Windows while the
//! hotkey is the double press of `Shift`.
//!
//! Everything about the flags runs through [`sticky::follow_via`] with closures in place of
//! `SystemParametersInfoW`: **no test here touches the session of the person running it.** The
//! rest are sweeps of the source — where the shortcut is taken and every way out that gives it
//! back — and the names the journal has for it.

use std::cell::{Cell, RefCell};
use std::path::Path;

use lang_switcher::diag::{Kind, Operation};
use lang_switcher::sticky::{self, Change};
use windows::core::{Error as WinError, HRESULT, Result as WinResult};

/// `SKF_STICKYKEYSON` — sticky keys turned on.
const STICKYKEYSON: u32 = 0x1;

/// `SKF_HOTKEYACTIVE` — five presses of `Shift` turn sticky keys on and off: the shortcut.
const HOTKEYACTIVE: u32 = 0x4;

/// `SKF_CONFIRMHOTKEY` — the shortcut asks before it acts: the window the owner met (159б).
const CONFIRMHOTKEY: u32 = 0x8;

/// `SKF_HOTKEYSOUND` — a sound when the shortcut acts; one of the person's own settings.
const HOTKEYSOUND: u32 = 0x10;

/// The flags of a Windows nobody changed — **510**, measured on the owner's machine in the session
/// and in the profile alike before this task (`scratchpad-E95\sticky-baseline.log`, 2026-10-07).
const WINDOWS_DEFAULT: u32 = 0x1FE;

/// A session of the sticky keys: its flags, and every write made to them.
struct Session {
    flags: Cell<u32>,
    writes: RefCell<Vec<u32>>,
}

impl Session {
    fn new(flags: u32) -> Self {
        Self {
            flags: Cell::new(flags),
            writes: RefCell::new(Vec::new()),
        }
    }

    fn read(&self) -> impl FnOnce() -> WinResult<u32> + '_ {
        || Ok(self.flags.get())
    }

    fn write(&self) -> impl FnOnce(u32) -> WinResult<()> + '_ {
        |flags| {
            self.writes.borrow_mut().push(flags);
            self.flags.set(flags);
            Ok(())
        }
    }

    fn writes(&self) -> Vec<u32> {
        self.writes.borrow().clone()
    }
}

/// A read the step must not make — the system is not to be asked at all.
fn no_read() -> impl FnOnce() -> WinResult<u32> {
    || panic!("the system must not be asked about the sticky keys here")
}

/// A write the step must not make.
fn no_write() -> impl FnOnce(u32) -> WinResult<()> {
    |flags| panic!("nothing must be written here, and {flags:#x} was")
}

/// What the system answers when it refuses — `E_ACCESSDENIED`, any refusal would do.
fn refusal() -> WinError {
    WinError::from_hresult(HRESULT(0x8007_0005_u32 as i32))
}

// ---------------------------------------------------------------------------------------
// The rule of the flags
// ---------------------------------------------------------------------------------------

/// The decision itself: the double press published on a Windows nobody changed takes **the one
/// bit** of the shortcut — `SKF_CONFIRMHOTKEY`, the window's own setting, stays as it was — and owes
/// it back.
#[test]
fn the_double_press_turns_the_shortcut_off_and_owes_it_back() {
    let session = Session::new(WINDOWS_DEFAULT);

    let (owed, done) = sticky::follow_via(false, true, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::TurnedOff));
    assert!(owed, "the bit taken is owed back");
    assert_eq!(session.writes(), vec![WINDOWS_DEFAULT & !HOTKEYACTIVE]);
    assert_eq!(session.flags.get() & CONFIRMHOTKEY, CONFIRMHOTKEY);
}

/// Sticky keys turned on are the person's — the shortcut is how they turn them off again — and the
/// double press takes nothing then: no write, nothing owed.
#[test]
fn sticky_keys_turned_on_are_the_persons_and_are_left_alone() {
    let session = Session::new(WINDOWS_DEFAULT | STICKYKEYSON);

    let (owed, done) = sticky::follow_via(false, true, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(!owed);
    assert!(session.writes().is_empty(), "{:x?}", session.writes());
}

/// A shortcut already off — by the person or by another program — has nothing to give: nothing is
/// written, nothing is owed, and so another hotkey later gives nothing «back» either.
#[test]
fn a_shortcut_already_off_owes_nothing_and_gets_nothing_back() {
    let session = Session::new(WINDOWS_DEFAULT & !HOTKEYACTIVE);

    let (owed, done) = sticky::follow_via(false, true, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(!owed);
    assert!(session.writes().is_empty());

    let (owed, done) = sticky::follow_via(owed, false, no_read(), no_write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(!owed);
}

/// ⭐ **The controller's note on 159.25: give back only what was taken.** The sample of Microsoft
/// writes the whole saved structure back, which would turn off sticky keys the person turned on
/// while the program ran, and undo any setting they changed. Here the person turned sticky keys on
/// and the sound of the shortcut off in the meantime; the way out puts back the one bit and keeps
/// both of theirs.
#[test]
fn the_way_out_gives_back_the_bit_and_keeps_what_the_person_changed() {
    let session = Session::new(WINDOWS_DEFAULT);
    let (owed, _) = sticky::follow_via(false, true, session.read(), session.write());

    // The person, in «Параметры», while the double press is the hotkey.
    session
        .flags
        .set((session.flags.get() | STICKYKEYSON) & !HOTKEYSOUND);

    let (owed, done) = sticky::follow_via(owed, false, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::GivenBack));
    assert!(!owed);
    assert_eq!(
        session.flags.get(),
        (WINDOWS_DEFAULT | STICKYKEYSON) & !HOTKEYSOUND,
        "the shortcut is back, sticky keys stay on, the sound stays off: {:#x}",
        session.flags.get()
    );
}

/// A shortcut someone put back while it was owed — the person, or a game ending with the same
/// trick — is on already: nothing is written, and the debt is closed.
#[test]
fn a_shortcut_someone_put_back_is_not_written_again() {
    let session = Session::new(WINDOWS_DEFAULT);
    let (owed, _) = sticky::follow_via(false, true, session.read(), session.write());

    session.flags.set(session.flags.get() | HOTKEYACTIVE);
    let before = session.writes().len();

    let (owed, done) = sticky::follow_via(owed, false, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(!owed);
    assert_eq!(
        session.writes().len(),
        before,
        "no write for a bit already on"
    );
}

/// Forth and back on a Windows nobody touched in between returns its flags exactly — two writes.
#[test]
fn forth_and_back_returns_the_flags_of_the_session_exactly() {
    let session = Session::new(WINDOWS_DEFAULT);

    let (owed, _) = sticky::follow_via(false, true, session.read(), session.write());
    let (owed, done) = sticky::follow_via(owed, false, session.read(), session.write());

    assert_eq!(done.ok(), Some(Change::GivenBack));
    assert!(!owed);
    assert_eq!(session.flags.get(), WINDOWS_DEFAULT);
    assert_eq!(
        session.writes(),
        vec![WINDOWS_DEFAULT & !HOTKEYACTIVE, WINDOWS_DEFAULT]
    );
}

/// A second publication of the double press — another «Применить» — asks the system nothing, and
/// so does any publication of another hotkey with nothing owed: the shortcut is the system's
/// business only when this program has something to do to it.
#[test]
fn a_publication_with_nothing_to_do_asks_the_system_nothing() {
    let (owed, done) = sticky::follow_via(true, true, no_read(), no_write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(owed, "still owed: nothing was given back");

    let (owed, done) = sticky::follow_via(false, false, no_read(), no_write());

    assert_eq!(done.ok(), Some(Change::Nothing));
    assert!(!owed);
}

/// A refusal of the system leaves the debt as it was: a read refused takes nothing and gives nothing
/// back; a write refused on the way in owes nothing; a write refused on the way out still owes.
#[test]
fn a_refusal_of_the_system_leaves_the_debt_as_it_was() {
    let (owed, done) = sticky::follow_via(false, true, || Err(refusal()), no_write());
    assert!(done.is_err());
    assert!(!owed);

    let (owed, done) = sticky::follow_via(true, false, || Err(refusal()), no_write());
    assert!(done.is_err());
    assert!(owed);

    let (owed, done) = sticky::follow_via(false, true, || Ok(WINDOWS_DEFAULT), |_| Err(refusal()));
    assert!(done.is_err());
    assert!(!owed);

    let (owed, done) = sticky::follow_via(
        true,
        false,
        || Ok(WINDOWS_DEFAULT & !HOTKEYACTIVE),
        |_| Err(refusal()),
    );
    assert!(done.is_err());
    assert!(owed);
}

/// The two halves of the rule as functions of the flags alone — the same verdicts as above, at the
/// edges: `taken` leaves every other bit as it is, `given_back` only ever sets the one.
#[test]
fn the_rule_takes_and_gives_back_one_bit_and_nothing_else() {
    // `!STICKYKEYSON` — every bit set but sticky keys turned on.
    for flags in [0, HOTKEYACTIVE, WINDOWS_DEFAULT, !STICKYKEYSON] {
        let taken = sticky::taken(flags);

        if flags & HOTKEYACTIVE == 0 {
            assert_eq!(taken, None, "{flags:#x}: nothing to take");
        } else {
            assert_eq!(taken, Some(flags & !HOTKEYACTIVE), "{flags:#x}");
        }
    }

    assert_eq!(sticky::taken(WINDOWS_DEFAULT | STICKYKEYSON), None);
    assert_eq!(sticky::taken(u32::MAX), None, "sticky keys on: left alone");

    // `!HOTKEYACTIVE` — every bit set but the shortcut.
    for flags in [
        0,
        STICKYKEYSON,
        WINDOWS_DEFAULT & !HOTKEYACTIVE,
        !HOTKEYACTIVE,
    ] {
        assert_eq!(
            sticky::given_back(flags),
            Some(flags | HOTKEYACTIVE),
            "{flags:#x}"
        );
    }

    assert_eq!(sticky::given_back(WINDOWS_DEFAULT), None);
}

// ---------------------------------------------------------------------------------------
// The source: where the shortcut is taken, every way out, and the profile never written
// ---------------------------------------------------------------------------------------

/// A file of `src\` with its lines of comment taken out — a call turned into a comment is no call
/// (the lesson of the mutant M6 of task T-95-2: a sweep that read comments missed it).
fn code_of(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));

    text.replace("\r\n", "\n")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The body of the function that `signature` opens: from it to the first line that is a lone `}` at
/// the left margin.
fn body_of<'a>(code: &'a str, signature: &str) -> &'a str {
    let start = code
        .find(signature)
        .unwrap_or_else(|| panic!("`{signature}` must be in the source"));
    let rest = &code[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("the body of `{signature}` must end"));

    &rest[..end]
}

/// The controls of the two helpers above: a call turned into a comment is not found, and a body
/// ends at its own brace.
#[test]
fn the_sweeps_of_this_file_do_not_read_comments() {
    let sample =
        "fn a() {\n    // crate::sticky::give_back();\n    real();\n}\nfn b() {\n    x();\n}\n";
    let code: String = sample
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    assert!(!code.contains("give_back"));
    assert!(body_of(&code, "fn a()").contains("real();"));
    assert!(!body_of(&code, "fn a()").contains("x();"));
}

/// Where the shortcut is taken: the publication of the configuration follows the hotkey it has just
/// published — the start and every «Применить» go through it (`app::publish_configuration`).
#[test]
fn the_publication_of_the_configuration_takes_the_shortcut_after_the_hotkey() {
    let app = code_of("app.rs");
    let publish = body_of(&app, "pub fn publish_configuration(");

    let hotkey = publish
        .find("crate::hook::set_hotkey(")
        .expect("the publication publishes the hotkey");
    let follow = publish
        .find("crate::sticky::follow_hotkey(")
        .expect("the publication tells module `sticky` what the hotkey is (T-95-6)");

    assert!(
        follow > hotkey,
        "the shortcut follows the hotkey just published"
    );
}

/// ⭐ **Every way out** (the controller's note on 159.25, point 3): the ordinary end of the process
/// — the main thread, once every thread is joined; `WM_ENDSESSION` — the end of the session and the
/// uninstaller of 155.25, which posts it with `wParam` TRUE; FR-96 — after the hook is off and before
/// the process ends. Another hotkey published is the test above: `follow_hotkey(false)`.
#[test]
fn every_way_out_gives_the_shortcut_back() {
    let app = code_of("app.rs");

    let first = body_of(&app, "fn run_as_first_instance(");
    let joined = first
        .find("join_all(threads)")
        .expect("the main thread joins the threads");
    let given = first
        .find("crate::sticky::give_back();")
        .expect("the ordinary end gives the shortcut back (T-95-6)");
    assert!(given > joined, "given back once every thread is joined");

    let session = app
        .find("if message == WM_ENDSESSION && wparam.0 != 0 {")
        .expect("the window procedure handles the end of the session");
    let branch = &app[session..];
    let branch = &branch[..branch.find('}').expect("the branch ends")];
    assert!(
        branch.contains("crate::sticky::give_back();"),
        "WM_ENDSESSION gives the shortcut back: {branch}"
    );

    let hook = code_of("hook.rs");
    let emergency = body_of(&hook, "fn emergency_exit()");
    let unhooked = emergency
        .find("uninstall();")
        .expect("FR-96 takes the hook off first");
    let quietly = emergency
        .find("crate::sticky::give_back_quietly();")
        .expect("FR-96 gives the shortcut back (T-95-6)");
    let ended = emergency
        .find("TerminateProcess(")
        .expect("FR-96 ends the process");
    assert!(
        unhooked < quietly && quietly < ended,
        "the hook off, the shortcut back, then the end"
    );
}

/// NFR-01: the callback of a live hook never asks the system about the sticky keys — module `hook`
/// names module `sticky` once, in FR-96, where the hook is already off.
#[test]
fn the_callback_of_a_live_hook_never_touches_the_sticky_keys() {
    let hook = code_of("hook.rs");

    assert_eq!(
        hook.matches("sticky::").count(),
        1,
        "module `hook` names `sticky` once"
    );
    assert!(body_of(&hook, "fn emergency_exit()").contains("sticky::"));
}

/// ⭐ **The profile of the person is never written**: every `SystemParametersInfoW` of module
/// `sticky` passes no update flag — no `SPIF_UPDATEINIFILE`, no `SPIF_SENDCHANGE` — so the next logon
/// reads the person's own flags whatever this program did.
#[test]
fn the_profile_of_the_person_is_never_written() {
    let code = code_of("sticky.rs");
    let calls = code.matches("SystemParametersInfoW(").count();

    assert_eq!(calls, 2, "one read and one write");
    assert_eq!(
        code.matches("SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)")
            .count(),
        calls,
        "every call passes no update flag"
    );
    assert!(!code.contains("SPIF_UPDATEINIFILE"));
    assert!(!code.contains("SPIF_SENDCHANGE"));
    assert!(code.contains("SPI_SETSTICKYKEYS"));
}

// ---------------------------------------------------------------------------------------
// The journal
// ---------------------------------------------------------------------------------------

/// The journal has a name for each thing this program does to a setting of Windows, and for the
/// system's refusal: a fact and no value (SEC-01, SEC-07), in the group of the configuration.
#[test]
fn the_journal_names_the_shortcut_taken_given_back_and_refused() {
    for name in [
        "sticky keys shortcut turned off",
        "sticky keys shortcut given back",
        "SystemParametersInfoW (sticky keys)",
    ] {
        let operation = Operation::from_name(name);

        assert_ne!(operation, Operation::UNLISTED, "«{name}» is a row");
        assert_eq!(operation.name(), name);
        assert_eq!(operation.kind(), Kind::Process, "«{name}»");
    }
}
