//! An in-memory ring log, optional dumping to a file.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: a ring journal in
//! memory, dumped to a file only when asked.
//!
//! Requirements this module covers: SEC-01, SEC-07, NFR-12.
//! Implemented by backlog tasks: T-06-4.
//!
//! # What this module is, before anything else
//!
//! This is the one part of a program that hooks the whole keyboard which is allowed to write
//! to disk. Three requirements draw the line and this module is built so that they hold by
//! construction rather than by agreement:
//!
//! - **SEC-01.** A keystroke is never written to disk — not to this journal, not to the
//!   configuration, not to a temporary file, not to a crash dump.
//! - **SEC-07.** The journal holds *events of the program* only. Characters, key codes and
//!   clipboard contents never enter it **at any level of detail**.
//! - **NFR-12.** Writing keystrokes to disk is absent completely.
//!
//! "At any level of detail" means there is no level at which it is permitted: no debug mode,
//! no environment variable and no feature gate turns it on. The mechanism that makes that
//! true is the *type of an entry*, not the discipline of the callers.
//!
//! ## Why an entry cannot carry a symbol
//!
//! An [`Event`] has four fields and not one of them is a place a character could go:
//!
//! | Field | Type | Where the value comes from |
//! |---|---|---|
//! | `ordinal` | `u64` | [`record`] itself, from the internal counter |
//! | `at_ms` | `u32` | [`record`] itself, from the internal clock |
//! | `operation` | [`Operation`] | an index into the closed table [`OPERATIONS`] |
//! | `code` | [`OsCode`] | an `HRESULT`, and only ever from a [`windows::core::Error`] |
//!
//! There is no `String`, no `&str`, no `char`, no `[u8]` and no caller-chosen integer. The
//! first two fields are produced inside [`record`], so a caller cannot put anything into them
//! at all. The last two are the entire caller-facing input to the journal, and both are
//! *narrowing* types:
//!
//! - [`Operation`] wraps a private `u16` that is an index into [`OPERATIONS`]. Its only
//!   public constructors are the associated constants and [`Operation::from_name`], which
//!   maps the infinitely many possible strings onto the finite table and answers
//!   [`Operation::UNLISTED`] for everything else — **keeping nothing of the text**. A name
//!   built out of a keystroke does not reach the journal as a name; it reaches it as the
//!   value "unlisted".
//! - [`OsCode`] wraps a private `i32` with no constructor that takes an integer. The only way
//!   to obtain one is [`OsCode::of`] from a real OS error, or [`OsCode::NONE`]. A scan code
//!   cannot be turned into an `OsCode`; there is no function that would do it.
//!
//! So the sentence "we do not put characters into the journal" is not a rule anybody has to
//! remember. `diag::record(Operation, OsCode)` is the only door into the ring, and neither
//! argument has room for a character.
//!
//! `buffer::Stroke` deliberately has neither `Debug` nor `Display`, which is what keeps a
//! stroke from being formatted anywhere at all; nothing here adds one, and nothing here needs
//! one, because no function in this module accepts a stroke, a key code or a buffer.
//!
//! # The write path — NFR-01 to NFR-05
//!
//! [`crate::app::report_non_critical`] is called from `Drop` implementations and from code
//! that sits next to the hook callback, so [`record`] must cost what the callback is allowed
//! to cost: **no allocation (NFR-03), no blocking primitive (NFR-04, section 6.3) and no
//! input-output (NFR-05)**.
//!
//! That is why the ring is a `static` array of atomics and not a `Vec` behind a `Mutex`, and
//! why an entry is four numbers rather than a formatted line. **Nothing is formatted while
//! recording.** Text appears only in [`render`], which runs on the UI thread at shutdown.
//!
//! Publication uses a sequence stamp rather than a lock: the writer takes a ticket with one
//! `fetch_add`, clears the stamp of the slot it landed on, stores the fields, and puts the
//! stamp back last. A reader that sees the stamp change under it discards that entry. The
//! writer never waits for anybody, which is the property a hook-adjacent path needs and a
//! mutex cannot give.
//!
//! That arrangement is a **seqlock**, and it is written in Boehm's canon rather than in the shape
//! that reads naturally: one `fence(Release)` in the writer and one `fence(Acquire)` in the
//! reader, each of them in a place where an ordering *on* the stamp store or load would not do the
//! job. [`Slot`] carries the citation and the reasoning; modules [`crate::guard`] and
//! [`crate::layouts`] hold the other two seqlocks of this program and say the same thing.
//!
//! # Threads — sections 6.1 and 6.3
//!
//! [`record`] is callable from every thread of section 6.1, the input thread included: it
//! touches process-wide statics through atomics and nothing else.
//!
//! [`dump_on_shutdown`] is not. Section 6.1 gives file input-output to the **UI thread** and
//! forbids it to the input thread outright, so that function is called from one place, on that
//! thread, once, as it leaves its message loop.
//!
//! # The file — section 7
//!
//! Off by default: `[diagnostics] log_enabled = false`. When it is false **no file is created
//! at all** — not an empty one, not a truncated one. The file lives next to the configuration,
//! in `%APPDATA%\Lang_Switcher\`, because section 7 states that `%ProgramFiles%` is not
//! writable by an ordinary user.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering, fence};
use std::time::Instant;

use windows::core::Error as WinError;

use crate::CONFIG_DIR_NAME;
use crate::settings;

// ---------------------------------------------------------------------------------------
// Size of the journal
// ---------------------------------------------------------------------------------------

/// Entries the ring holds before the oldest is evicted.
///
/// # Why this number
///
/// A [`Slot`] is twenty-four bytes, so the ring is `1024 * 24 = 24_576` bytes — **24 KiB, or
/// 0.29% of the 8 MB working set NFR-06 asks for**. That is the whole argument for the size:
/// the journal is charged against the memory ceiling permanently, from process start to
/// process end, and a quarter of a percent of it is a price the ceiling does not notice.
///
/// It is generous in the other direction too. Every entry is a *failure* of a Win32 call or a
/// life event of the journal itself; a healthy session produces two. A session that produced
/// a thousand and one has a problem whose first thousand entries are not the interesting ones,
/// which is exactly what a ring is for.
///
/// A power of two on purpose: the index of a slot is `ordinal % CAPACITY`, and the compiler
/// turns that into a mask instead of a division on the write path.
pub const CAPACITY: usize = 1024;

/// Name of the dump file inside [`log_dir`].
pub const LOG_FILE_NAME: &str = "diag.log";

/// Bytes the journal occupies for the life of the process.
///
/// The whole of it: the ring is a `static`, so this memory is part of the image the loader
/// maps and is never allocated, grown or freed at run time.
pub const fn footprint_bytes() -> usize {
    size_of::<Slot>() * CAPACITY
}

// ---------------------------------------------------------------------------------------
// The type of an entry — SEC-01, SEC-07, NFR-12
// ---------------------------------------------------------------------------------------

/// An operating system error code, and nothing that is not one.
///
/// **SEC-07.** The private field has no public constructor that takes a number. [`OsCode::of`]
/// is the only way to obtain a non-zero value and it demands a [`windows::core::Error`], which
/// this program only ever produces from a Win32 call that failed. A scan code, a virtual key
/// or a character cannot be made into an `OsCode`: there is no function with that signature,
/// and adding one would be the change a reviewer is looking for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OsCode(i32);

impl OsCode {
    /// The absence of an error — `S_OK`. What the journal's own life events carry.
    pub const NONE: Self = Self(0);

    /// The `HRESULT` of a failed Win32 call.
    pub fn of(error: &WinError) -> Self {
        Self(error.code().0)
    }

    /// The code as the number the operating system gave, for a formatter.
    pub const fn raw(self) -> i32 {
        self.0
    }
}

/// What part of the program an [`Operation`] belongs to.
///
/// A grouping, so that a person reading the dump can see at a glance whether the trouble was
/// the hook, the tray or the layout. Every value is a word chosen at compile time; the list is
/// closed and is written out in [`Kind::name`] so that the words and the arms cannot drift
/// apart — the shape [`crate::watchdog::Reason::name`] uses for the same reason.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// The journal's own life: started, written, refused.
    Journal = 0,
    /// The process itself — the single-instance mutex, the module handle, the window class,
    /// the threads.
    Process = 1,
    /// Windows and messages.
    Window = 2,
    /// The keyboard hook and the subscriptions of the watchdog.
    Hook = 3,
    /// Switching the keyboard layout — the chain of FR-50.
    Layout = 4,
    /// The tray icon and its menu.
    Tray = 5,
    /// The debug control channel of SEC-04a, `testing` builds only.
    Channel = 6,
    /// An operation name that is not in [`OPERATIONS`]. **Nothing of the name is kept.**
    #[default]
    Unlisted = 7,
    /// The selection path and the clipboard — FR-60 to FR-65.
    Selection = 8,
}

impl Kind {
    /// The word the dump prints for this group.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Journal => "journal",
            Self::Process => "process",
            Self::Window => "window",
            Self::Hook => "hook",
            Self::Layout => "layout",
            Self::Tray => "tray",
            Self::Channel => "channel",
            Self::Unlisted => "unlisted",
            Self::Selection => "selection",
        }
    }
}

/// The closed vocabulary of the journal.
///
/// One row per program event this journal is able to name, in the order the values of
/// [`Operation`] index them. **This table is the whole of what an entry can say.** It is fixed
/// at compile time and visible here in full; nothing this program reads from the keyboard, the
/// clipboard or a file can add a row to it or take a value outside it.
///
/// Row `0` is reserved for [`Operation::UNLISTED`] and must stay there: it is the answer
/// [`Operation::from_name`] gives to any name that is not below, and the answer
/// [`Operation::from_index`] gives to a slot it cannot decode.
///
/// Two names appear from more than one place — `PostMessageW` is posted both by `app` and by
/// method 1 of FR-50 in `switch`, and `MessageBoxW` by both `app` and `tray`. The counters
/// section of the dump separates them; the entry itself does not, and buying that separation
/// with a longer name is not worth widening what a name may contain.
static OPERATIONS: &[(&str, Kind)] = &[
    // The journal's own life.
    ("(unlisted)", Kind::Unlisted),
    ("journal started", Kind::Journal),
    ("journal written", Kind::Journal),
    ("journal write refused", Kind::Journal),
    // `app` — the process.
    ("CreateMutexW", Kind::Process),
    ("GetModuleHandleW", Kind::Process),
    ("RegisterClassExW", Kind::Process),
    ("UnregisterClassW", Kind::Process),
    ("CloseHandle", Kind::Process),
    ("TerminateProcess", Kind::Process),
    ("thread body", Kind::Process),
    // `app` — windows and messages.
    ("DestroyWindow", Kind::Window),
    ("PostMessageW", Kind::Window),
    ("MessageBoxW", Kind::Window),
    // `watchdog` — the hook and its subscriptions.
    ("RegisterRawInputDevices(RIDEV_REMOVE)", Kind::Hook),
    ("UnhookWinEvent", Kind::Hook),
    ("GetModuleHandleW (watchdog)", Kind::Hook),
    ("SetWindowsHookExW (watchdog)", Kind::Hook),
    ("WTSUnRegisterSessionNotification", Kind::Hook),
    ("KillTimer", Kind::Hook),
    ("KillTimer (fault)", Kind::Hook),
    // The debt task T-08-4 recorded and could not pay: this file was closed to it, so the
    // withdrawal of the FR-21 device notification counted its failure and could not name it.
    // `Kind::Hook` because the registration is one of the watchdog's subscriptions, which is
    // what that kind already is; **no new `Kind` is added**, for the reason every note above
    // gives — a variant of an enumeration is not a name.
    ("UnregisterDeviceNotification", Kind::Hook),
    // `switch` — the chain of FR-50.
    ("SystemParametersInfoW", Kind::Layout),
    ("ActivateKeyboardLayout", Kind::Layout),
    ("CoCreateInstance", Kind::Layout),
    ("ITfInputProcessorProfileMgr::ActivateProfile", Kind::Layout),
    // `tray`.
    ("RegisterWindowMessageW", Kind::Tray),
    ("Shell_NotifyIconW(NIM_ADD)", Kind::Tray),
    ("Shell_NotifyIconW(NIM_SETVERSION)", Kind::Tray),
    ("Shell_NotifyIconW(NIM_MODIFY)", Kind::Tray),
    ("Shell_NotifyIconW(NIM_DELETE)", Kind::Tray),
    ("DestroyMenu", Kind::Tray),
    ("Menu::build", Kind::Tray),
    ("SetForegroundWindow", Kind::Tray),
    ("PostMessageW(WM_NULL)", Kind::Tray),
    ("DestroyIcon", Kind::Tray),
    // `control` — SEC-04a, `testing` builds only.
    ("control::start", Kind::Channel),
    ("CloseHandle(token)", Kind::Channel),
    ("ConnectNamedPipe", Kind::Channel),
    ("DisconnectNamedPipe", Kind::Channel),
    ("write(control channel)", Kind::Channel),
    ("FlushFileBuffers", Kind::Channel),
    ("CloseHandle(pipe)", Kind::Channel),
    // `selection` — the clipboard and the selection path of §4.7.
    ("clipboard snapshot truncated", Kind::Selection),
    // `settings` and `tray` — the dialog of FR-92, autostart FR-93 and the string tables of
    // FR-94. Added by task T-08-2, which is the debt task T-08-1 recorded and could not pay:
    // this file was closed to it, so every refusal from the settings code went into the ring as
    // `Operation::UNLISTED` — the code of the failure with no name against it.
    //
    // The names are the Win32 calls themselves, which is the convention every row above
    // follows. They take existing kinds rather than a new one: a dialog control is a window
    // (`Kind::Window`) and the `Run` value and the resources of the image belong to the process
    // (`Kind::Process`). A `Kind::Settings` would read better in a dump and is deliberately not
    // added — the task opens this file for strings and for nothing else.
    ("SetDlgItemTextW", Kind::Window),
    ("SetWindowTextW", Kind::Window),
    ("CheckDlgButton", Kind::Window),
    ("CheckRadioButton", Kind::Window),
    ("GetDlgItem", Kind::Window),
    ("EndDialog", Kind::Window),
    ("DialogBoxParamW", Kind::Window),
    ("SetWindowLongPtrW", Kind::Window),
    ("ShellExecuteW", Kind::Window),
    ("RegSetValueExW", Kind::Process),
    ("RegCloseKey", Kind::Process),
    ("FindResourceExW", Kind::Process),
    ("LoadResource", Kind::Process),
    // The debt task T-08-2 found and had no mandate to pay — its report, section 18, problem 2.
    // Seven operations reached the journal as `Operation::UNLISTED`; **four of them are here and
    // three are deliberately not**, and the reason is a requirement, not an oversight.
    //
    // Kinds follow the same rule the rows above follow — the subject the call serves, out of the
    // kinds that already exist. **No new `Kind` is added**: task T-08-3 opens this file for
    // names, and a variant of an enumeration is not a name.
    //
    // * the three clipboard rows take `Kind::Selection`, which is "the selection path and the
    //   clipboard, FR-60 to FR-65", and that is exactly what they are. `AddClipboardFormatListener`
    //   lives in `app` and `RemoveClipboardFormatListener` in `selection`, but they are the two
    //   halves of one registration and must not read as two different subjects in a dump;
    // * `CloseHandle(process)` takes `Kind::Process`, the same kind the plain `CloseHandle` row
    //   above already has, because it is the same operation on the same kind of handle.
    //
    // ⚠ **The three the password-field probe of FR-71 reports under are absent on purpose.**
    // They are `CoCreateInstance(…)`, `QueryInterface(…)` and the timeout setter of the automation
    // interface, and every one of those names *contains* a UI Automation symbol. Acceptance point
    // 9 of FR-71 — enforced by `tests\guard.rs`, `no_ui_automation_name_occurs_anywhere_near_the_hook`
    // — requires that no such symbol occur anywhere under `src\` except in `src\guard.rs`, in code
    // or in prose alike, so that a future edit putting UI Automation on the hook's path would have
    // to write one of those symbols into `hook.rs` first. Adding the rows would put them here, and
    // spelling them in pieces to slip past the sweep would defeat a guard rather than satisfy it.
    // The three therefore stay `UNLISTED` until the owner of that requirement decides how a
    // journal name and that sweep are to live together. See the report of task T-08-3.
    ("AddClipboardFormatListener", Kind::Selection),
    ("RemoveClipboardFormatListener", Kind::Selection),
    ("CloseClipboard", Kind::Selection),
    ("CloseHandle(process)", Kind::Process),
    // The fate of the configuration file — task T-13-6, the finding "the outcome of reading
    // `config.toml` is thrown away". `settings::read_or_default` answers what became of the
    // file, `crate::tray` decides what may be done to it, and these five rows are how that
    // decision reaches a dump.
    //
    // ⚠ **Every one of them is a fact about the file and none of them is anything out of it.**
    // Not a line, not a fragment, not the name of a field this build has no name for, not the
    // line and column a parser stopped at. A configuration file can be edited by hand and
    // filled with anything at all — `ConfigError` is built on that very reasoning — so a row
    // here that could carry a shape of the file's contents would be the leak SEC-01 and SEC-07
    // forbid. These say *that* the file could not be read, *that* it was kept, *that* a save
    // was given up. A reader who needs to know why opens the file, which is where the text is
    // and the only place it is.
    //
    // `Kind::Process` for all five, and **no new `Kind` is added** — the rule every note above
    // follows. The configuration is state of the process: it is read once as the process
    // starts and written as it ends, and the second reader of it is this module itself, so a
    // `Kind::Tray` would be wrong for half of them and a `Kind::Settings` would be a new
    // variant, which is not a name.
    ("configuration file unreadable", Kind::Process),
    ("configuration file from a newer schema", Kind::Process),
    ("configuration file quarantined", Kind::Process),
    ("configuration file quarantine refused", Kind::Process),
    ("configuration save suppressed", Kind::Process),
    // Step 8 of FR-61 not made — task T-13-11, the note decision П-5 added to §4.7: «затирать
    // новую копию пользователя снимком недопустимо». `selection::restore_after` asks whether the
    // clipboard still holds this program's own write and puts nothing back when it does not, and
    // that refusal is a promise of the module not kept — so it may not be silent.
    //
    // ⚠ **The name is a fact about this program's own decision and carries nothing else.** Not
    // the content that was skipped over, not its size, not its formats, not the sequence numbers
    // that decided it — a number of the window station's counter is not the user's data, but it
    // is not this row's business either, and `OsCode::NONE` goes with the entry for the reason
    // «clipboard snapshot truncated» above passes it. SEC-01, SEC-07.
    //
    // `Kind::Selection` — «the selection path and the clipboard, FR-60 to FR-65» — and **no new
    // `Kind` is added**, the rule every note above follows.
    ("clipboard restore skipped", Kind::Selection),
    // The resumption FR-99 does not allow — task T-13-9, the finding "fail-safe is
    // irreversible and the icon lies after «Возобновить»". One row, appended at the end so
    // that no index above it moves: a slot already written carries a number, and renumbering
    // the table would change what a dump of an earlier run means.
    //
    // ⚠ **A fact and no value at all (SEC-01, SEC-07).** It says that a resumption arrived
    // while FR-99 held the program disarmed and was refused. It does not say what
    // `general.enabled` was, how many panics had run, which entry of the menu was chosen or
    // what the user had typed — and there is no branch here through which any of those could
    // reach the ring. The code beside it is always [`OsCode::NONE`]: this is an event of the
    // program, not the failure of a Win32 call.
    //
    // `Kind::Tray` and **no new `Kind`** — the rule every note above follows. Unlike the five
    // configuration rows, which are half the tray's and half the process's, this one belongs
    // to the tray whole: it is raised in `crate::tray::Tray::toggle_state`, it is about the
    // icon and the menu of FR-90 and FR-91, and the kind already exists.
    ("resume refused in fail-safe", Kind::Tray),
    // A millisecond field of section 7 taken down to its ceiling — task T-13-13, the finding
    // «`inter_event_delay_ms` не ограничен сверху: значение из файла способно усыпить поток
    // ввода». `app::publish_configuration` puts the three millisecond fields through their
    // ceilings on the way to the atomics the input and the UI thread read, and a publication that
    // had to take any of them down says so here. One row, appended at the end so that no index
    // above it moves: a slot already written carries a number, and renumbering the table would
    // change what a dump of an earlier run means.
    //
    // ⚠ **A fact and no value at all (SEC-01, SEC-07).** It says that a publication found a field
    // above its ceiling and published the ceiling instead. It does not say **which** field, what
    // the file asked for, what was published, or what the ceiling is — and there is no branch
    // through which any of those could reach the ring, because the caller passes no number at all.
    // The name is singular and field-less for exactly that reason: one entry per publication that
    // clamped anything, not one per field and not one per press. The code beside it is always
    // [`OsCode::NONE`] — this is a decision of the program, not the failure of a Win32 call.
    //
    // `Kind::Process` and **no new `Kind`**, the rule every note above follows: the value came out
    // of the configuration file, which is state of the process, and this is the kind the five
    // configuration rows of task T-13-6 already take.
    ("configuration field clamped to its ceiling", Kind::Process),
];

/// What happened, as an index into [`OPERATIONS`].
///
/// **SEC-01, SEC-07 — this is the type the whole requirement rests on.** The field is private
/// and there is no constructor that takes an integer, so a value of this type is always one of
/// the rows above. [`Operation::from_name`] is the funnel every textual operation name goes
/// through, and it is a *narrowing*: a name that is not in the table becomes
/// [`Operation::UNLISTED`] and the text is dropped on the floor. That is what makes the
/// journal proof against a caller — present or future — that hands it a string built out of
/// something the user typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Operation(u16);

impl Operation {
    /// A name that is not in [`OPERATIONS`]. Nothing of the name survives.
    pub const UNLISTED: Self = Self(0);
    /// [`init`] ran.
    pub const JOURNAL_STARTED: Self = Self(1);
    /// The dump reached the file.
    pub const JOURNAL_WRITTEN: Self = Self(2);
    /// The dump did not reach the file — NFR-13, the result of the write was examined.
    pub const JOURNAL_WRITE_REFUSED: Self = Self(3);

    /// The row of [`OPERATIONS`] whose name is exactly `name`, or [`Operation::UNLISTED`].
    ///
    /// Allocation-free, lock-free and free of input-output, because it is on the write path:
    /// a walk over a `static` table comparing string slices, and comparing two `&str` starts
    /// by comparing their lengths.
    ///
    /// **The narrowing step of SEC-07.** Every string in the universe maps onto this finite
    /// table; everything outside it maps onto one value that carries no text.
    pub fn from_name(name: &str) -> Self {
        OPERATIONS
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, row)| row.0 == name)
            .and_then(|(index, _)| u16::try_from(index).ok())
            .map_or(Self::UNLISTED, Self)
    }

    /// The name of this operation, for the dump.
    pub fn name(self) -> &'static str {
        OPERATIONS
            .get(usize::from(self.0))
            .map_or(OPERATIONS[0].0, |row| row.0)
    }

    /// The group this operation belongs to.
    pub fn kind(self) -> Kind {
        OPERATIONS
            .get(usize::from(self.0))
            .map_or(Kind::Unlisted, |row| row.1)
    }

    /// The index stored in a slot.
    const fn index(self) -> u16 {
        self.0
    }

    /// The value a slot decoded to, or [`Operation::UNLISTED`] if it names no row.
    fn from_index(index: u32) -> Self {
        u16::try_from(index)
            .ok()
            .filter(|&index| usize::from(index) < OPERATIONS.len())
            .map_or(Self::UNLISTED, Self)
    }
}

/// One entry of the journal, as a reader sees it.
///
/// Four numbers. **SEC-01, SEC-07: there is nowhere here to put a character or a scan code.**
/// `ordinal` and `at_ms` are produced by [`record`] and are not caller input at all; the other
/// two are the narrowing types described in the module documentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Event {
    /// Position in the total order of everything ever recorded. Gaps between consecutive
    /// entries of a snapshot are entries the ring has already evicted.
    pub ordinal: u64,
    /// Milliseconds between [`init`] and this entry.
    pub at_ms: u32,
    /// What happened.
    pub operation: Operation,
    /// The `HRESULT` that came with it, or [`OsCode::NONE`].
    pub code: OsCode,
}

impl Event {
    /// The group of [`Event::operation`].
    pub fn kind(self) -> Kind {
        self.operation.kind()
    }
}

// ---------------------------------------------------------------------------------------
// The ring — section 6.2, NFR-03, NFR-04, NFR-05, NFR-06
// ---------------------------------------------------------------------------------------

/// One entry as it is stored: twenty-four bytes of atomics, and no indirection anywhere.
///
/// `stamp` is both the publication marker and the ordinal. Zero means "never written, or being
/// written right now"; any other value is `ordinal + 1`. A reader that finds the same non-zero
/// stamp before and after reading the other three fields has read one consistent entry; one
/// that does not has been overtaken by a writer and skips the slot.
///
/// # The barriers are Boehm's, not the ones that read naturally
///
/// Hans-J. Boehm, *"Can seqlocks get along with programming language memory models?"*, MSPC 2012.
/// The canon that paper settles, and the form every seqlock in this program is written in — the
/// same words stand over [`crate::layouts::published`] and [`crate::guard::is_excluded_name`]:
///
/// * the **reader** loads the stamp, reads the fields with `Relaxed` loads, then executes
///   `fence(Acquire)` **before** the control load of the stamp, which may itself be `Relaxed`.
///   Putting `Acquire` *on* the control load instead is the trap the paper is about: acquire on a
///   load orders the operations that come **after** it, and what has to be pinned here is the
///   field loads that came **before** — nothing stops them from being sunk past an acquire load;
/// * the **writer** executes `fence(Release)` between clearing the stamp and storing the fields,
///   so that no field store can be hoisted above the zero that announces the write, and closes
///   with a `Release` store of the new stamp, which is what publishes the fields to a reader that
///   sees it.
///
/// **Neither fence was here until task T-13-16**, and the sentence above about one consistent
/// entry was a promise the memory model did not keep — which is what the audit of 2026-08-24
/// found: «заявленный инвариант … моделью памяти не обеспечен». On x86_64 the omission was
/// practically unobservable; section 3 admits an ARM64 build, where a load-load reordering is not
/// a thought experiment. Nothing heavier than the two fences was added, and nothing could be: a
/// fence is an ordering instruction and not a synchronisation primitive — it takes no lock, waits
/// for nobody and cannot block, which is what NFR-04 asks of everything [`record`] does. On x86_64
/// both compile to no instruction at all.
struct Slot {
    stamp: AtomicU64,
    at_ms: AtomicU32,
    operation: AtomicU32,
    code: AtomicI32,
}

impl Slot {
    /// A slot that has never been written. Every slot starts here, at load time.
    ///
    /// A `const fn` and not a `const` item: a constant whose type holds atomics would be
    /// copied at each mention rather than referred to, which is a mistake often enough that
    /// clippy refuses to let one be declared.
    const fn empty() -> Self {
        Self {
            stamp: AtomicU64::new(0),
            at_ms: AtomicU32::new(0),
            operation: AtomicU32::new(0),
            code: AtomicI32::new(0),
        }
    }
}

/// The journal.
///
/// **Section 6.2, NFR-06, and the reason the write path can promise NFR-03.** This is a
/// `static`, so its twenty-four kilobytes are part of the process image the loader maps: the
/// memory exists before `main` runs, is never grown, never moved and never freed. There is no
/// `Box`, no `Vec`, no `OnceLock<Vec<_>>` and no lazy first-touch anywhere in this module —
/// which is not a stylistic choice but the only arrangement compatible with a write path that
/// may not allocate. A journal that allocated on its first entry would allocate on a path that
/// is forbidden to.
static RING: [Slot; CAPACITY] = [const { Slot::empty() }; CAPACITY];

/// The ticket counter. Also the count of everything ever recorded.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// The instant [`init`] ran, which every `at_ms` is measured from.
///
/// A `OnceLock` and not a lock in the sense NFR-04 forbids: [`OnceLock::get`] never blocks —
/// it answers `None` while the cell is empty or being filled — and [`OnceLock::set`] is
/// reached exactly once, from [`init`], at start-up and on no hook-adjacent path. The same
/// pattern `control::note_start` uses for the same measurement.
static START: OnceLock<Instant> = OnceLock::new();

/// Starts the journal. Called once, at start-up, from [`crate::app::run`].
///
/// The ring itself needs no starting — see [`RING`]. What this fixes is the zero of the clock,
/// and it records the first entry so that a dump from a session in which nothing went wrong
/// still says when the session began.
pub fn init() {
    // Already set means `init` ran twice, which is a caller error and not a reason to fail:
    // the first zero is the truthful one and is kept.
    let _ = START.set(Instant::now());

    record(Operation::JOURNAL_STARTED, OsCode::NONE);
}

/// Puts one entry into the ring. **Callable from every thread of section 6.1.**
///
/// # What this function is allowed to cost — NFR-01 to NFR-05
///
/// Line by line, and this is the whole body:
///
/// 1. `elapsed_ms()` — one atomic load inside [`OnceLock::get`] and one `Instant::now`, which
///    on Windows is `QueryPerformanceCounter`. No allocation, no lock, no file, no handle.
/// 2. `NEXT.fetch_add` — one atomic read-modify-write. Wait-free: it never spins and never
///    yields, unlike the acquire of a mutex, which is what NFR-04 and section 6.3 forbid.
/// 3. the index — a remainder by a power of two, which the compiler emits as a mask.
/// 4. `&RING[..]` — an address inside a `static`. Nothing is allocated; the slot has existed
///    since the image was mapped.
/// 5. `slot.stamp.store(0, …)` — one atomic write of a `u64`: the announcement that the slot is
///    being written and must be skipped.
/// 6. `fence(Release)` — an **ordering instruction and not a synchronisation primitive**: it takes
///    no lock, waits for nobody, cannot block and cannot fail, so NFR-04 has nothing to object to.
///    On x86_64 it emits no instruction at all; on the optional ARM64 build of section 3 it is one
///    barrier. See [`Slot`] for why it is here and why an ordering on the store above would not do.
/// 7. four `store`s — atomic writes of a `u32`, a `u32`, an `i32` and the `u64` stamp that
///    publishes them, the last of them with `Release`.
///
/// No branch of it formats anything, and no branch of it can: the arguments are two `Copy`
/// numbers. **This is where "the journal does not format while recording" is enforced** — the
/// text of an entry does not exist until [`render`] builds it, on the UI thread, at shutdown.
///
/// # Publication
///
/// The stamp is cleared before the fields are written and restored after them, with `Release`,
/// and a `fence(Release)` stands **between** the clearing and the first field store — the writer's
/// half of the canon of [`Slot`]. Without it the `Relaxed` clearing may be reordered with the
/// field stores, and then "a zero stamp means the slot is being written" is not a fact any reader
/// can lean on. With it, a reader either sees the entry whole or sees that it must skip the slot.
///
/// **Why the fence and not `Release` on each field store.** Boehm's canon admits both; the fence
/// is chosen because it is one ordering instruction for all three stores instead of three, because
/// it puts the barrier where the reasoning is — between the announcement and what it announces,
/// rather than spread over the things announced — and because it is the shape module
/// [`crate::layouts`] is written in, so that the three seqlocks of this program read alike.
///
/// A writer that wraps onto a slot a reader is in the middle of does not wait for it — the reader
/// loses the entry, the writer loses nothing. That trade is the right way round: the write path is
/// the one with a deadline.
pub fn record(operation: Operation, code: OsCode) {
    let at_ms = elapsed_ms();

    // Relaxed is enough: the ticket only has to be unique, and the ordering that matters is
    // the one the stamp below publishes.
    let ordinal = NEXT.fetch_add(1, Ordering::Relaxed);

    let slot = &RING[(ordinal % CAPACITY as u64) as usize];

    slot.stamp.store(0, Ordering::Relaxed);

    // Boehm's fence, and it stands **between** the cleared stamp and the fields rather than as an
    // ordering on either one — see `Slot`. This is what keeps the three stores below from being
    // hoisted above the zero that is supposed to announce them.
    fence(Ordering::Release);

    slot.at_ms.store(at_ms, Ordering::Relaxed);
    slot.operation
        .store(u32::from(operation.index()), Ordering::Relaxed);
    slot.code.store(code.raw(), Ordering::Relaxed);
    slot.stamp.store(ordinal + 1, Ordering::Release);
}

/// Everything ever recorded, evicted entries included. The ring holds the last [`CAPACITY`]
/// of them.
pub fn recorded() -> u64 {
    NEXT.load(Ordering::Relaxed)
}

/// Milliseconds since [`init`], saturating at forty-nine days.
///
/// Zero before [`init`] has run, which is truthful rather than convenient: there is no zero to
/// measure from yet.
fn elapsed_ms() -> u32 {
    match START.get() {
        Some(start) => u32::try_from(start.elapsed().as_millis()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// The entries the ring holds right now, oldest first.
///
/// **Not for the write path.** This allocates a `Vec` and sorts it, which is exactly what
/// [`record`] may not do; it is meant for [`render`] and for the tests, both of which run
/// where allocation is allowed.
///
/// An entry a writer overtakes while this reads it is dropped rather than reported torn. The
/// three field loads are `Relaxed` and what orders them is the reader's half of the canon of
/// [`Slot`]: the `Acquire` load of the stamp above them and the `fence(Acquire)` below them,
/// before the control load.
pub fn snapshot() -> Vec<Event> {
    let mut events = Vec::with_capacity(CAPACITY);

    for slot in &RING {
        let stamp = slot.stamp.load(Ordering::Acquire);

        if stamp == 0 {
            // Never written, or a writer is inside it right now.
            continue;
        }

        let at_ms = slot.at_ms.load(Ordering::Relaxed);
        let operation = Operation::from_index(slot.operation.load(Ordering::Relaxed));
        let code = OsCode(slot.code.load(Ordering::Relaxed));

        // Boehm's fence, and it stands **before** the control load rather than inside it — see
        // `Slot`. This is what keeps the three field loads above from being sunk below the check
        // that is supposed to vouch for them.
        fence(Ordering::Acquire);

        if slot.stamp.load(Ordering::Relaxed) != stamp {
            // A writer wrapped onto this slot while the three fields above were read. The
            // entry is not reported rather than reported wrong.
            continue;
        }

        events.push(Event {
            ordinal: stamp - 1,
            at_ms,
            operation,
            code,
        });
    }

    events.sort_unstable_by_key(|event| event.ordinal);

    events
}

// ---------------------------------------------------------------------------------------
// The file — section 7, section 6.1
// ---------------------------------------------------------------------------------------

/// The folder the journal is written to: `%APPDATA%\Lang_Switcher\`.
///
/// `None` when `APPDATA` is not set, which does not happen in an interactive Windows session.
/// Built the same way [`crate::settings::default_config_path`] builds its own, and for the
/// same reason section 7 gives: `%ProgramFiles%` is not writable by an ordinary user, so the
/// program writes next to its configuration or not at all.
///
/// **This is the function the settings dialog of task T-08-1 calls** for its "open the journal
/// folder" button. It answers a folder rather than a file on purpose: the folder exists to be
/// shown whether or not a journal was ever written to it.
pub fn log_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| log_dir_in(Path::new(&app_data)))
}

/// Builds the journal folder inside an arbitrary application data directory.
///
/// Split out from [`log_dir`] so that the shape of the path can be asserted without an
/// environment and without touching the real `%APPDATA%` — the split
/// [`crate::settings::config_path_in`] already makes for the configuration.
pub fn log_dir_in(app_data: &Path) -> PathBuf {
    app_data.join(CONFIG_DIR_NAME)
}

/// The journal file, `%APPDATA%\Lang_Switcher\diag.log`.
pub fn log_path() -> Option<PathBuf> {
    log_dir().map(|dir| dir.join(LOG_FILE_NAME))
}

/// Builds the journal file path inside an arbitrary application data directory.
pub fn log_path_in(app_data: &Path) -> PathBuf {
    log_dir_in(app_data).join(LOG_FILE_NAME)
}

/// Writes the dump to `path`, creating the folder if it is not there.
///
/// The file is replaced, not appended to: the ring belongs to one run of the program, and a
/// journal that grew without bound would be a second way of filling somebody's disk.
///
/// **UI thread only** — section 6.1.
pub fn write_to(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, render())
}

/// Dumps the journal if the configuration asks for it. Called once, on the **UI thread**, as
/// it leaves its message loop.
///
/// # Why the setting is read here and not published at start-up
///
/// Section 6.1 gives file input-output to the UI thread and this function runs on it, so the
/// decision and the write happen in one place, on the one thread allowed to do either. Reading
/// the setting at the moment it is acted on also means the answer is the one that is true at
/// shutdown.
///
/// # When it is off
///
/// `log_enabled = false` is the default of section 7 and the ordinary case. Then this function
/// creates nothing: no folder, no file, not an empty one. That is the difference between a
/// journal that is off and a journal that is on and had nothing to say.
pub fn dump_on_shutdown() {
    let Some(config_path) = settings::default_config_path() else {
        return;
    };

    let (config, outcome) = settings::read_or_default(&config_path);

    // **The second consumer of `read_or_default`, and it uses the outcome — task T-13-6.**
    //
    // What it decides here is narrow, and narrow is correct: **this function never writes the
    // configuration file.** It reads one flag out of it and writes a file of its own,
    // elsewhere. So not one of the decisions the outcome drives in `crate::tray` exists on
    // this path — there is nothing to move aside, nothing to forbid, and no way for this code
    // to damage a file it does not write. That is why dropping the value here was never a
    // danger, and it is still not written.
    //
    // What the outcome does decide is *whose* answer `log_enabled` is. A read that failed
    // hands back the defaults of section 7 rather than the file, so the flag is this program's
    // own default and not a setting anybody chose — and that default is `false`, which is the
    // direction that creates no file nobody asked for. A journal that refused to run because
    // the configuration would not parse would be worse than no journal; a journal that started
    // writing on a guess about an unreadable file would be worse still. The branch says so in
    // one place instead of leaving it to be re-derived from the default table.
    let log_enabled = match outcome {
        Ok(_) => config.diagnostics.log_enabled,
        Err(_) => false,
    };

    if !log_enabled {
        return;
    }

    let Some(target) = log_path() else {
        return;
    };

    // Recorded before the dump is rendered so that the file says it was written.
    record(Operation::JOURNAL_WRITTEN, OsCode::NONE);

    if write_to(&target).is_err() {
        // NFR-13: the result is examined rather than discarded. There is nowhere left to
        // report it — the place a report would go is the file that just refused — so it goes
        // into the ring, which the next dump of this process would carry. The `io::Error`
        // itself is dropped: it is text, and nothing in this module keeps text.
        record(Operation::JOURNAL_WRITE_REFUSED, OsCode::NONE);
    }
}

// ---------------------------------------------------------------------------------------
// Rendering — the only place in this module where text exists
// ---------------------------------------------------------------------------------------

/// The whole dump as text.
///
/// # What is in it, and what is deliberately not
///
/// Two sections. **Events** is the ring. **Counters** is a snapshot of the counters the
/// earlier tasks already keep and already publish — this module reads them through their
/// public accessors and keeps no counter of its own, because a second copy of a number is a
/// second number to be wrong.
///
/// The counters printed are the *failure and health* counters named by this task: the hook
/// going up and coming down, the recoveries of the watchdog, the layout cache that would not
/// build, each method of the chain of FR-50, the `SendInput` return-value discrepancies, the
/// refused `PostMessage`s.
///
/// `watchdog::counters` and `hook::hotkey_handoffs` are **left out on purpose**. They are
/// honest counts of program events and SEC-07 would permit them, but they are proportional to
/// how much the user typed and clicked, and a number proportional to typing has no business in
/// a file when SEC-01 is what this module exists to honour. Leaving them out costs the
/// diagnostic nothing: not one of them can distinguish a program that is working from one that
/// is not.
pub fn render() -> String {
    let mut out = String::with_capacity(8192);

    let _ = writeln!(out, "{} diagnostic journal", crate::APP_NAME);
    let _ = writeln!(out);
    row_usize(&mut out, "journal.capacity", CAPACITY);
    row_usize(&mut out, "journal.footprint_bytes", footprint_bytes());
    row_u64(&mut out, "journal.recorded", recorded());
    row_u32(&mut out, "journal.uptime_ms", elapsed_ms());

    let _ = writeln!(out);
    let _ = writeln!(out, "--- counters ---");
    let _ = writeln!(out);
    render_counters(&mut out);

    let _ = writeln!(out);
    let _ = writeln!(out, "--- events ---");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:>10}  {:>10}  {:<10}  {:<45}  code",
        "ordinal", "ms", "kind", "operation"
    );

    for event in snapshot() {
        let _ = writeln!(
            out,
            "{:>10}  {:>10}  {:<10}  {:<45}  0x{:08X}",
            event.ordinal,
            event.at_ms,
            event.kind().name(),
            event.operation.name(),
            event.code.raw()
        );
    }

    out
}

/// The counter snapshot, taken through the public accessors of the modules that own them.
fn render_counters(out: &mut String) {
    let (unhook_failures, last_unhook_error) = crate::hook::unhook_failures();

    row_flag(out, "hook.installed", crate::hook::is_installed());
    row_flag(out, "hook.down", crate::watchdog::hook_down());
    row_flag(out, "hook.active", crate::hook::is_active());
    row_flag(out, "hook.fail_safe", crate::hook::fail_safe());
    row_u32(
        out,
        "hook.consecutive_panics",
        crate::hook::consecutive_panics(),
    );
    row_u32(out, "hook.post_failures", crate::hook::post_failures());
    row_u32(out, "hook.unhook_failures", unhook_failures);
    row_hex(out, "hook.last_unhook_error", last_unhook_error as i32);
    row_flag(
        out,
        "hook.emergency_terminate_failed",
        crate::hook::emergency_terminate_failed(),
    );

    let health = crate::watchdog::health();

    row_u32(out, "watchdog.recoveries", health.recoveries);
    row_u32(out, "watchdog.silent_removals", health.silent_removals);
    row_u32(out, "watchdog.absent_at_check", health.absent_at_check);
    row_u32(out, "watchdog.install_failures", health.install_failures);
    row_word(out, "watchdog.last_reason", health.last_reason.name());
    row_u32(out, "watchdog.last_gap_us", health.last_gap_us);
    row_u32(out, "watchdog.max_gap_us", health.max_gap_us);
    row_u32(out, "watchdog.liveness_ticks", health.liveness_ticks);
    row_u32(out, "watchdog.desktop_switches", health.desktop_switches);
    row_u32(out, "watchdog.session_changes", health.session_changes);
    row_u32(out, "watchdog.power_resumes", health.power_resumes);

    row_u32(
        out,
        "layouts.cache_failures",
        crate::app::layout_cache_failures(),
    );

    let selection = crate::layouts::selection_failures();

    row_u32(out, "layouts.ime_layout", selection.ime_layout);
    row_u32(out, "layouts.no_layouts", selection.no_layouts);
    row_u32(out, "layouts.origin_outside", selection.origin_outside);

    let (send_mismatches, events_lost) = crate::inject::send_mismatches();

    row_u32(out, "inject.send_mismatches", send_mismatches);
    row_u32(out, "inject.events_lost", events_lost);

    let failures = crate::switch::failures();

    row_u32(out, "switch.post_message", failures.post_message);
    row_u32(out, "switch.attach_activate", failures.attach_activate);
    row_u32(out, "switch.text_services", failures.text_services);
    row_u32(out, "switch.post_rejected", failures.post_rejected);
    row_u32(out, "switch.activate_rejected", failures.activate_rejected);
    row_u32(
        out,
        "switch.activate_profile_rejected",
        failures.activate_profile_rejected,
    );
    row_u32(out, "switch.detach_failed", failures.detach_failed);
    row_u32(out, "switch.ime_target", failures.ime_target);
    row_u32(out, "switch.no_target", failures.no_target);
    row_u32(out, "switch.no_foreground", failures.no_foreground);
    row_u32(out, "switch.exhausted", failures.exhausted);
    row_u32(out, "switch.scope_unreadable", failures.scope_unreadable);
}

/// Width of the name column of the counter section. One place, so the columns line up.
const NAME_WIDTH: usize = 34;

/// One counter row holding a `u32`.
fn row_u32(out: &mut String, name: &str, value: u32) {
    let _ = writeln!(out, "{:<width$}{}", name, value, width = NAME_WIDTH);
}

/// One counter row holding a `u64`.
fn row_u64(out: &mut String, name: &str, value: u64) {
    let _ = writeln!(out, "{:<width$}{}", name, value, width = NAME_WIDTH);
}

/// One counter row holding a `usize`.
fn row_usize(out: &mut String, name: &str, value: usize) {
    let _ = writeln!(out, "{:<width$}{}", name, value, width = NAME_WIDTH);
}

/// One counter row holding a flag.
fn row_flag(out: &mut String, name: &str, value: bool) {
    let _ = writeln!(out, "{:<width$}{}", name, value, width = NAME_WIDTH);
}

/// One counter row holding an error code.
fn row_hex(out: &mut String, name: &str, value: i32) {
    let _ = writeln!(out, "{:<width$}0x{:08X}", name, value, width = NAME_WIDTH);
}

/// One counter row holding one of a closed list of words.
///
/// The word comes from a `const fn` of the module that owns the value — never from anything
/// this program read.
fn row_word(out: &mut String, name: &str, value: &str) {
    let _ = writeln!(out, "{:<width$}{}", name, value, width = NAME_WIDTH);
}

// ---------------------------------------------------------------------------------------
// Tests that need neither a window nor a keyboard
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The four constants have to point at the four rows they name. They are indices written
    /// by hand, which is the one way this table can be got wrong.
    #[test]
    fn the_named_rows_are_where_the_constants_say() {
        assert_eq!(Operation::UNLISTED.name(), "(unlisted)");
        assert_eq!(Operation::JOURNAL_STARTED.name(), "journal started");
        assert_eq!(Operation::JOURNAL_WRITTEN.name(), "journal written");
        assert_eq!(
            Operation::JOURNAL_WRITE_REFUSED.name(),
            "journal write refused"
        );
    }

    /// Every row must be reachable by its own name, and no two rows may share one.
    #[test]
    fn every_name_maps_back_to_its_own_row() {
        for (index, (name, _kind)) in OPERATIONS.iter().enumerate().skip(1) {
            assert_eq!(
                Operation::from_name(name).index(),
                u16::try_from(index).unwrap(),
                "name {name} does not map back to its own row"
            );
        }
    }

    /// The row reserved for the unlisted case must stay at index zero.
    #[test]
    fn row_zero_is_the_unlisted_one() {
        assert_eq!(Operation::UNLISTED.index(), 0);
        assert_eq!(OPERATIONS[0].1, Kind::Unlisted);
    }

    /// A slot is twenty-four bytes and the ring is what [`footprint_bytes`] says it is.
    #[test]
    fn the_footprint_is_the_one_the_documentation_claims() {
        assert_eq!(size_of::<Slot>(), 24);
        assert_eq!(footprint_bytes(), 24_576);
    }

    /// The index of a slot must be a mask, which needs a power of two.
    #[test]
    fn the_capacity_is_a_power_of_two() {
        assert!(CAPACITY.is_power_of_two());
    }
}
