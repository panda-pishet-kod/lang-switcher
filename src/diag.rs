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
//! | `code` | [`OsCode`] | an `HRESULT`, only ever from a [`windows::core::Error`] or from the Win32 number of an `io::Error` — of this module's own file write, or of the configuration file's inside a [`crate::settings::ConfigError`] |
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
//! - [`OsCode`] wraps a private `i32` with no constructor that takes an integer. The only ways
//!   to obtain one are [`OsCode::of`] from a real OS error, [`OsCode::of_io`] from the Win32
//!   number an `io::Error` of this module's own file write carries (task T-34-1),
//!   [`OsCode::of_config`] from the error of a configuration file operation, which hands its
//!   `io::Error` to `of_io` (task T-69-6), or [`OsCode::NONE`]. All three constructors take an
//!   error object and keep the number the operating system put into it and nothing else; a
//!   scan code cannot be turned into an `OsCode`, because there is no function that takes an
//!   integer, and `tests\diag.rs` keeps the second road inside this module and the third inside
//!   module `tray`, the one module that journals a configuration failure.
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
//! recording.** Text appears only in [`render_as`], which runs on the UI thread — at shutdown,
//! and when the button «Сохранить журнал» of the settings dialog asks for a dump (task T-34-3).
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
//! thread, once, as it leaves its message loop. Since task T-34-3 the file has a second writer —
//! [`write_to`], pressed as the button «Сохранить журнал» of the settings dialog — and it lives
//! on the same thread, because the dialog does.
//!
//! # The file — section 7
//!
//! Off by default: `[diagnostics] log_enabled = false`. When it is false **no file is created
//! at all** — not an empty one, not a truncated one. The file lives next to the configuration,
//! in `%APPDATA%\Lang_Switcher\`, because section 7 states that `%ProgramFiles%` is not
//! writable by an ordinary user. Whether it is written is decided by the setting **published
//! for this session** ([`set_log_enabled`], written by `app::publish_configuration`), not by
//! reading the file again at shutdown — task T-34-2.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering, fence};
use std::time::Instant;

use windows::Win32::Globalization::{GetTimeFormatEx, LOCALE_NAME_INVARIANT, TIME_FORMAT_FLAGS};
use windows::core::{Error as WinError, HRESULT, w};

use crate::CONFIG_DIR_NAME;

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
/// demands a [`windows::core::Error`], which this program only ever produces from a Win32 call
/// that failed; [`OsCode::of_io`] (task T-34-1) demands an `io::Error` and keeps the Win32
/// number it carries and nothing else — the road a refused write of the journal file takes;
/// [`OsCode::of_config`] (task T-69-6) demands a [`crate::settings::ConfigError`] and sends its
/// `io::Error` down that same road. A scan code, a virtual key or a character cannot be made into
/// an `OsCode`: there is no function with that signature, and adding one would be the change a
/// reviewer is looking for — which is why task T-69-6 bridged the configuration's errors with an
/// error object and not with a number (decision 129.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OsCode(i32);

impl OsCode {
    /// The absence of an error — `S_OK`. What the journal's own life events carry.
    pub const NONE: Self = Self(0);

    /// The `HRESULT` of a failed Win32 call.
    pub fn of(error: &WinError) -> Self {
        Self(error.code().0)
    }

    /// The `HRESULT` of a failed file operation of **this module** — task T-34-1, finding Н101.
    ///
    /// The second road into an `OsCode`, and it is as narrow as the first. An `io::Error` that
    /// came from a system call carries the Win32 number the system gave; `HRESULT::from_win32`
    /// turns it into the same `HRESULT` every other failure of this program is journalled
    /// under, and [`OsCode::of`] narrows that. **Nothing of the error's text survives** — not
    /// the message, not the kind, not a path. An `io::Error` this program built itself carries
    /// no system number and answers [`OsCode::NONE`]: there is no number to report and none is
    /// invented.
    ///
    /// **The one place the program reads the Win32 number of an `io::Error`** — task T-69-6,
    /// backlog line Э34-Б-3. The idiom was drawn first for a refused configuration write (task
    /// Т-22-9, in module `tray`) and then here (task T-34-1); since task T-69-6 the configuration
    /// comes here through [`OsCode::of_config`], and `tests\diag.rs` sweeps the sources for a
    /// second reading of that number. It also keeps every call of this function inside this file.
    pub fn of_io(error: &io::Error) -> Self {
        error
            .raw_os_error()
            .and_then(|code| u32::try_from(code).ok())
            .map_or(Self::NONE, |code| {
                Self::of(&WinError::from_hresult(HRESULT::from_win32(code)))
            })
    }

    /// The `HRESULT` of a failed operation on **the configuration file** — task T-69-6, backlog
    /// line Э34-Б-3, decision 129.3; the road task Т-22-9 drew in module `tray` as `os_code_of`.
    ///
    /// The third road into an `OsCode`, and as narrow as the other two: it takes an error object
    /// and never a number. [`crate::settings::ConfigError::Io`] carries the `io::Error` of a system
    /// call on the file, and its number goes the one way every such number goes in this program —
    /// [`OsCode::of_io`]. The two variants that are not system failures answer [`OsCode::NONE`]:
    ///
    /// * [`crate::settings::ConfigError::Serialize`] — this program's own value refusing to become
    ///   TOML. No call failed, and a number invented for it would be a claim about one that did;
    /// * [`crate::settings::ConfigError::Malformed`] — cannot arrive from a write at all, and it is
    ///   the one variant carrying something derived from the file's contents (the line and column a
    ///   parser stopped at). It is answered with no number for both reasons, and the second is the
    ///   one that would matter if the first ever stopped being true.
    ///
    /// ⚠ The `io::Error` inside the `Io` variant is a value any code could build out of any number,
    /// so `tests\diag.rs` keeps the calls of this function inside module `tray`, the one module
    /// that journals a configuration failure — as it keeps the calls of `of_io` inside this file.
    pub fn of_config(error: &crate::settings::ConfigError) -> Self {
        match error {
            crate::settings::ConfigError::Io(io) => Self::of_io(io),
            crate::settings::ConfigError::Malformed { .. }
            | crate::settings::ConfigError::Serialize => Self::NONE,
        }
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
    /// The debug control channel of SEC-04a — **`testing` builds only, and since task T-41-11
    /// that is true of the variant itself and not only of what it describes** (finding Н43).
    ///
    /// ⭐ The discriminants of this enum are **explicit**, which is the one thing this repair got
    /// for nothing: taking the variant away does not move `Unlisted` from 7 or `Selection` from 8,
    /// so nothing else in the vocabulary is disturbed.
    #[cfg(feature = "testing")]
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
            // Task T-41-11, finding Н43: the category goes with the seven rows it named. The
            // word «channel» is a trace of the channel too, and a trace in the shipped file was
            // the whole of the finding.
            #[cfg(feature = "testing")]
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
    // Task T-41-5, finding Н24: the name of FR-82 was held, and no window of this program's own
    // class answered anywhere in the session — so the name is somebody else's and this copy
    // started all the same. **The whole of the finding is that this used to happen in silence**:
    // the person saw a program that would not start, with no cause and no trace.
    ("single-instance name held by a stranger", Kind::Process),
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
    // Задача Т-33а-1: большой значок для шара уведомления. Отдельное имя от прочих загрузок
    // значка потому, что и последствие у отказа своё — уведомление уходит со старым, малым
    // значком, а не пропадает (NFR-13); журнал обязан различать эти два случая.
    ("LoadImageW(SM_CXICON)", Kind::Tray),
    // ⛔ **The seven rows of the channel stood here until task T-41-11 (finding Н43).** They are
    // now in [`CHANNEL_OPERATIONS`], at the end of the vocabulary and under the same feature as
    // the channel itself — `#[cfg]` cannot be put on an element of an array literal, which is
    // why they moved rather than merely got a gate.
    //
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
    // Task Т-31-2, решение 99.1(б): the two calls that put a rebuilt settings window back where
    // the previous one stood. `Kind::Window` like every row of this block, and for the same
    // reason — the subject of both is a window of this program.
    ("GetWindowRect", Kind::Window),
    ("SetWindowPos", Kind::Window),
    // Task T-41-3, finding Н44: the two questions the tray asks the system before it opens the
    // menu of FR-91 — where the cursor is, and where the desktop of that monitor ends. A refusal
    // of either is NFR-13 (the menu still opens, at the point the message asked for, pulled onto
    // the desktop or not), and a refusal nobody could see would be a menu in the wrong place with
    // no explanation.
    ("GetCursorPos", Kind::Window),
    ("GetMonitorInfoW", Kind::Window),
    ("ShellExecuteW", Kind::Window),
    // Task T-41-1, finding С52: an address the gate of `letters::links::is_allowed` turned away.
    // Beside `ShellExecuteW` and in its group, because it is the same subject seen from the
    // other side — what was **not** handed to the shell. Without this row the refusal would be
    // a silence, and a silence is what the finding asks to make visible.
    ("link refused", Kind::Window),
    ("RegSetValueExW", Kind::Process),
    ("RegCloseKey", Kind::Process),
    ("FindResourceExW", Kind::Process),
    // Task T-43-15, finding Н68: the copy of a dialog template the mirror of right-to-left
    // languages patches (`settings::compiled_template`) is found by the call without `Ex`, and
    // its refusal reached the ring unnamed. Beside its sibling and of its kind.
    ("FindResourceW", Kind::Process),
    ("LoadResource", Kind::Process),
    // Task Т-29-3, вопрос 95: the interface language of the user's Windows, asked once and only
    // when there is no configuration file yet. `Kind::Process` for the same reason the two
    // resource rows above take it — the subject is this process's own environment and not a
    // window of it — and no new `Kind` is added, which is the rule every block here follows.
    ("GetUserPreferredUILanguages", Kind::Process),
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
    // ⚠ **The three the password-field probe of FR-71 reported under were absent on purpose
    // from task T-08-3 until task T-34-4.** They were `CoCreateInstance(…)`, `QueryInterface(…)`
    // and the timeout setter of the automation interface, and every one of those names
    // *contains* a UI Automation symbol. Acceptance point 9 of FR-71 — enforced by
    // `tests\guard.rs`, `no_ui_automation_name_occurs_anywhere_near_the_hook` — requires that no
    // such symbol occur anywhere under `src\` except in `src\guard.rs`, in code or in prose
    // alike, so that a future edit putting UI Automation on the hook's path would have to write
    // one of those symbols into `hook.rs` first. Adding the rows under those names would have put
    // them here, and spelling them in pieces to slip past the sweep would have defeated a guard
    // rather than satisfied it. **Решение 117.3 (Э34) answered the question T-08-3 asked:** the
    // three carry neutral names — `password probe: client / interface / timeout`, at the end of
    // this table — that say which step of the probe refused and contain no symbol the sweep
    // looks for. The sweep is untouched.
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
    // starts and written as it ends, and until task T-34-2 the second reader of it was this
    // module itself (it now listens to the published setting instead — `set_log_enabled`), so a
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
    // A selection too big to read — finding №7 of the audit of 2026-08-31, decision 77 of the
    // user. Step 4 of FR-61 had no ceiling while the snapshot of FR-64 had one, so a selection of
    // any size at all was copied off the clipboard and decoded twice more on the way to step 6.
    // `selection::read_unicode_text` now asks `read_refuses_size` before it copies anything, and
    // a read that found a block above the four megabytes of FR-64 says so here. One row, appended
    // at the end so that no index above it moves: a slot already written carries a number, and
    // renumbering the table would change what a dump of an earlier run means.
    //
    // ⚠ **A fact and no value at all (SEC-01, SEC-07).** It says that a read found a block over
    // the ceiling and copied nothing. It does not say how big the block was, what the ceiling is,
    // which format it was in, or a single byte of what stood there — and there is no branch
    // through which any of those could reach the ring, because the caller passes no number at
    // all. The code beside it is always [`OsCode::NONE`]: this is a decision of the program, not
    // the failure of a Win32 call.
    //
    // `Kind::Selection` — «the selection path and the clipboard, FR-60 to FR-65» — and **no new
    // `Kind`**, the rule every note above follows.
    ("clipboard read oversized", Kind::Selection),
    // ⭐ **Task Т-22-6, finding м6 of the audit of 2026-09-01** — step 8 of FR-61 was due, was
    // **not** withheld by decision П-5, and did not put the snapshot back. One row, appended at
    // the end so that no index above it moves: a slot already written carries a number, and
    // renumbering the table would change what a dump of an earlier run means.
    //
    // ⚠ **This is the opposite of «clipboard restore skipped» above and must never be read as the
    // same event.** A skip is a decision — the clipboard had moved on and somebody's new copy was
    // protected on purpose. This is a failure: the restore was owed, was attempted, and the user's
    // clipboard did not come back. `selection::note_restore_failed` is the only writer, and a skip
    // returns before it can be reached.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07).** It says that a restore was attempted and did not
    // happen. It does not say what was lost, how large it was, which formats it held, or how many
    // entries the snapshot carried. The code beside it is the `HRESULT` of the Win32 call that
    // refused, taken from the error and with its message dropped unread — or [`OsCode::NONE`] on
    // the two paths where no system call failed at all.
    //
    // `Kind::Selection` — «the selection path and the clipboard, FR-60 to FR-65» — and **no new
    // `Kind`**, the rule every note above follows.
    ("clipboard restore failed", Kind::Selection),
    // ⭐ **Task Т-22-9, finding м12 of the audit of 2026-09-01** — the save was allowed by
    // `SavePolicy`, was attempted, and `config.toml` did not take it. The sixth configuration
    // event and the last one to get a row: the other five have had theirs since task T-13-6, and
    // this one — the only one that loses a choice the user has already been shown as applied —
    // carried `TODO(T-06-4)` and `let _ = error` instead. One row, appended at the end so that no
    // index above it moves.
    //
    // ⚠ **The name is the fact, and the code is the system's own number (SEC-01, SEC-07).** It
    // does not say which setting was being written, what the file held, where the file is, or how
    // far the write got. Unlike the five rows beside it this one **does** carry a code, and the
    // code has one source: the Win32 number of the `io::Error` the write refused with, taken
    // through `HRESULT::from_win32` and `OsCode::of`, with the error's text dropped unread. The
    // one variant of `settings::ConfigError` that carries anything derived from the file's
    // contents — the line and column of a parser — cannot arrive from a write and is answered
    // with no number at all; `tray::os_code_of` is where that is written down.
    //
    // `Kind::Process` — the group every other configuration event of this program is in — and
    // **no new `Kind`**, the rule every note above follows.
    ("configuration write failed", Kind::Process),
    // ⭐ **Task Т-25-2, finding м-Э24-2 of stage Э24** — a second instance found the mutex of
    // FR-82 taken and left **without** putting up the modal notification, because the deadline of
    // FR-97 was armed and therefore nobody was in front of the screen. One row, appended at the
    // end so that no index above it moves.
    //
    // ⚠ **Not a failure, and it must never be read as one.** Every other name in this table
    // records something that refused or went wrong; this one records a window deliberately not
    // shown, and it carries no code at all — no system call was made and none failed. It is the
    // line that replaces the window: the only trace a bench has that this launch was a second
    // instance rather than a start that never happened.
    //
    // ⚠ **Debug only.** Its one writer, `app::debug_timeout::notification_is_suppressed`, lives
    // behind `cfg(debug_assertions)`, so this row can never be written in the shipped product —
    // the row itself is harmless there, being a `&str` in a table the whole program shares.
    //
    // `Kind::Process` — the group `CreateMutexW` and the rest of the FR-82 path are already in —
    // and **no new `Kind`**, the rule every note above follows.
    ("FR-82 notification suppressed", Kind::Process),
    // ⭐ **Task Т-32-1, FR-101** — the three system calls the letters of the author make outside
    // a window. Appended at the end so that no index above them moves: a slot already written
    // carries a number, and renumbering the table would change what a dump of an earlier run
    // means.
    //
    // `GetDateFormatEx` is the clock: this program asks it for today's local date, because the
    // schedule of FR-101 counts in whole days and `GetTickCount` measures something else
    // entirely. The other two are the quiet moment of FR-101 — whether Windows is accepting
    // notifications at all, and how long the session has been still.
    //
    // ⚠ **A name and a code, and nothing of what was asked (SEC-01, SEC-07).** None of the
    // three carries a date, a state or a number of seconds into the ring; a refusal says which
    // call refused and what the system said about it, exactly as every Win32 row above does.
    //
    // `Kind::Process` for all three, and **no new `Kind`** — the rule every note above follows.
    // The subject of each is this process's own environment, which is the kind
    // `GetUserPreferredUILanguages` already takes.
    ("GetDateFormatEx", Kind::Process),
    ("SHQueryUserNotificationState", Kind::Process),
    ("GetLastInputInfo", Kind::Process),
    // ⭐ **Task Т-32-3, FR-101 and FR-103** — the three calls the modeless windows of the
    // letters make and this program had never made before: the two ways a modeless dialog is
    // created, and the timer that drives the demonstration of «Привет».
    //
    // `Kind::Window` for all three, and **no new `Kind`** — the rule every note above follows.
    // The subject of each is a window of this program, which is the kind the whole block of
    // `SetDlgItemTextW` and its neighbours already takes.
    // ⚠ `DestroyWindow` is **not** added here: it has been in the table since `app` first
    // created a window, and a second row with the same name would make the name ambiguous —
    // which the test `every_name_maps_back_to_its_own_row` says out loud.
    ("CreateDialogIndirectParamW", Kind::Window),
    ("CreateDialogParamW", Kind::Window),
    ("SetTimer", Kind::Window),
    // ⭐ **Task Т-32-4, FR-101** — the balloon that announces a letter. `Kind::Tray`, the group
    // every other `Shell_NotifyIcon` row of this program is in, and the name says which of the
    // four calls it was: the flag, not the operation, is what tells them apart.
    //
    // ⚠ **A name and a code, and not one word of the letter (SEC-01, SEC-07).** What refused
    // is a call to the shell; what the balloon would have said is a sentence of this program's
    // own string tables, and none of it goes into the ring.
    ("Shell_NotifyIconW(NIF_INFO)", Kind::Tray),
    // ⭐ **Task Т-32-6, FR-102 and SEC-03** — the one network operation this program has, and
    // the cryptography that decides whether to believe what it brought back. Appended at the
    // end so that no index above them moves.
    //
    // `Kind::Process` and **no new `Kind`**: SEC-03 makes the feed a property of the process —
    // «сетевая активность ограничена одной операцией» — and a `Kind::Feed` would be a new
    // variant where a name will do, which is the rule every note above follows.
    //
    // ⚠ **A name and a code, and nothing of the request or the answer (SEC-01, SEC-07).** Not
    // the address, not the status, not a byte of the document, not the reason a signature
    // failed beyond the fixed sentence. A dump of this program can say «the feed was read» and
    // «a feed was refused»; it can never say what was in it or where it came from.
    ("WinHttpOpen", Kind::Process),
    ("WinHttpConnect", Kind::Process),
    ("WinHttpSendRequest", Kind::Process),
    ("WinHttpReceiveResponse", Kind::Process),
    ("WinHttpReadData", Kind::Process),
    ("WinHttpOpenRequest", Kind::Process),
    ("BCryptOpenAlgorithmProvider", Kind::Process),
    ("BCryptImportKeyPair", Kind::Process),
    // ⚠ Written the moment the session is opened, **before** anything is sent — the one line
    // that says a request was made at all. Its absence is what proves `[letters] feed = false`
    // reaches the network never: a refusal further down leaves its own name, but a request that
    // failed at the socket would otherwise leave nothing, and «nothing» would prove nothing.
    ("feed request", Kind::Process),
    ("feed read ok", Kind::Process),
    ("feed signature line refused", Kind::Process),
    ("feed document refused", Kind::Process),
    ("feed from a newer schema", Kind::Process),
    ("feed signature did not verify", Kind::Process),
    ("feed answer oversized", Kind::Process),
    // Task T-34-3, finding С48: the clock of the dump header. `GetDateFormatEx` above already
    // answers the date; this is its sibling for the time of day, and a refusal says which call
    // refused and what the system said — never what time it was. Appended at the end for the
    // reason every row above gives: an index already written must keep its meaning.
    ("GetTimeFormatEx", Kind::Process),
    // Task T-34-4, finding Н95, решение 117.3: the three steps of the password-field probe of
    // FR-71 that used to reach the ring as `(unlisted)` — creating the client, asking it for the
    // bounded interface, setting its timeouts. Named by the step and not by the call, so that no
    // symbol of acceptance point 9 of FR-71 stands in this file (see the note above the
    // clipboard rows). `Kind::Process` — a COM object of this process, the kind the other
    // `CoCreateInstance` of this table does not share only because that one switches a layout.
    ("password probe: client", Kind::Process),
    ("password probe: interface", Kind::Process),
    ("password probe: timeout", Kind::Process),
    // ⭐ **Task T-55-1, решение 120.4 (б)** — autostart for real: every start of the program makes
    // `HKCU\…\Run` agree with `general.autostart`, and each of the two things it may do to the
    // value is one row. Appended at the end for the reason every row above gives: an index already
    // written must keep its meaning.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07).** Not the command that was written, not the path
    // of the image, not what the value said before — only that a start wrote the value or removed
    // it. A refusal has no row of its own here: it is reported under `RegSetValueExW` above, the
    // name the other two writers of the value already report under.
    //
    // `Kind::Process` — the value follows the configuration, which is state of the process, the
    // kind every configuration row takes — and **no new `Kind`**, the rule every note above follows.
    ("autostart registered at start", Kind::Process),
    ("autostart removed at start", Kind::Process),
    // ⭐ **Task T-55-2, решение 120.4 (ж)** — a change of autostart the person asked for and the
    // program withheld, because the session lives on a file from a newer schema: «Применить» and
    // the check mark of FR-91 do not ask the `Run` key then, and this row is how the withholding
    // reaches a dump instead of silence. The neighbour of «configuration save suppressed», and of
    // its shape. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not the state asked for, not the state kept.
    // `Kind::Process` and **no new `Kind`**, the rule every note above follows.
    ("autostart change suppressed", Kind::Process),
    // ⭐ **Task T-55-3, finding Н26** — `config.toml` carried the read-only attribute, the rename
    // of the atomic write was refused for it, and the write cleared the attribute once and landed.
    // A person may have set that attribute by hand, and the program changed it: a fact worth a
    // line. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not the path, not the attributes, not the
    // configuration. `Kind::Process` and **no new `Kind`**, the rule every note above follows.
    ("configuration read-only attribute cleared", Kind::Process),
    // ⭐ **Task T-55-5, finding Т8** — a temporary of an interrupted write of `config.toml` that the
    // start found beside the file and could not remove. Not a failure of the program's work — the
    // configuration is read all the same — and not silence either. Appended at the end so that no
    // index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not the name, not the process id in it, not the
    // count. `Kind::Process` and **no new `Kind`**, the rule every note above follows.
    ("configuration temporary not removed", Kind::Process),
    // ⭐ **Task T-55-6, решение 120.1** — `config.toml` is there and could not be read, twice:
    // held open by another program, closed to this account, a failing disk. Nothing is written and
    // nothing is moved for the session. Not «configuration file unreadable» above, which is now the
    // name of a file read whole and not understood — the quarantine; the two causes lead to two
    // different fates, and a dump must be able to tell them apart. Appended at the end so that no
    // index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not the path, not the system's answer, not a
    // byte of the file. `Kind::Process` and **no new `Kind`**, the rule every note above follows.
    ("configuration file not read", Kind::Process),
    // ⭐ **Task T-55-7, решение 120.2** — a number of `config.toml` that was not a whole number its
    // field can hold was read as the default of that field, and the rest of the file was read as it
    // is. One line per such field; before this task one such value sent the whole file to
    // quarantine. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not which field, not what stood there, not the
    // default it became. `Kind::Process` and **no new `Kind`**, the rule every note above follows.
    ("configuration field read as its default", Kind::Process),
    // ⭐ **Task T-38-7, finding Н16 of the audit of 2026-09-04, decision 121.3** — the clipboard
    // listed more formats than `selection::snapshot` walks, and the tail of the list was never
    // seen: the snapshot is partial in a way none of its counts shows. The neighbour of «clipboard
    // snapshot truncated» — that one is the budget of FR-64, this one the bound of the
    // enumeration — and of its shape. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not how many formats there were, not which, not
    // a byte of any of them. `Kind::Selection` and **no new `Kind`**, the rule every note above
    // follows.
    ("clipboard formats truncated", Kind::Selection),
    // ⭐ **Task T-38-8, finding С11 of the audit of 2026-09-04, decisions 121.2 and 121.3** — the
    // clipboard listed formats and `selection::snapshot` kept none of them: step 1 answered
    // success, and step 8 will have nothing to put back. The behaviour is not changed; this is the
    // trace of it. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not how many formats there were, not which, not
    // why none was kept. `Kind::Selection` and **no new `Kind`**, the rule every note above follows.
    ("clipboard snapshot saved nothing", Kind::Selection),
    // ⭐ **Task T-71-3, backlog line Э36-Б-3 (finding Н9 of stage Э36), decision 132.2** — a press
    // the hotkey capture of FR-94 refused. The reason stood under the field and nothing of it
    // reached a dump; task T-36-5 saw this door and had no mandate to open it. Recorded by
    // `settings::note_capture_refused`, where the refusal is carried out, beside the counter
    // `settings.capture_refusals`. Appended at the end so that no index above it moves.
    //
    // ⚠ **A fact and no value (SEC-01, SEC-07)** — not the key, not the modifiers, not which reason
    // it was: an entry has no field for any of them, and the person has read the reason on the
    // screen. `Kind::Window` — a refusal is the business of a dialog, like «link refused» — and
    // **no new `Kind`**, the rule every note above follows.
    ("hotkey capture refused", Kind::Window),
    // ⛔ **A row «tray foreground refused» stood here on `e73` alone (task T-73-1) and was taken
    // out again by task T-73-3, decision 134в.** It named a refusal of the tray's own foreground
    // gate; the live acceptance of `e73` recorded sixteen of them against the owner's own clicks,
    // the gate was removed, and nothing can produce the name any more. It is removed rather than
    // left standing because a name in this table is a promise that something writes it. Removing
    // the **last** row moves no index above it, which is why the row was appended there in the
    // first place. A refusal of the foreground is still journalled — by the system's own answer,
    // under `SetForegroundWindow` above.
];

/// The vocabulary of the debug channel — **finding Н43, task T-41-11**.
///
/// # Why these seven names live apart from the rest
///
/// The channel itself is behind `#[cfg(feature = "testing")]` and is absent from the shipped
/// program (SEC-04a condition 1, acceptance criterion 8 of section 13). Its seven **operation
/// names** were not: they stood in the middle of [`OPERATIONS`], which has no gate, and so they
/// went into the shipped binary as plain text. Nothing could use them there — there is no code
/// to report them — but anybody who ran a text search over the file would find the traces of a
/// mechanism the program says it does not have, and «следов канала не остаётся» would be an
/// inaccurate promise. **In an open release that is a conversation about trust**, which is the
/// whole of what finding Н43 is about.
///
/// Measured before the repair and not assumed: all seven were in
/// `target\release\LangSwitcher.exe` in UTF-8, seven of seven — journal
/// `scratchpad-E41\premise-P3-strings.log`.
///
/// # Why at the end, and what that costs
///
/// `#[cfg]` cannot be written on an element of an array literal, so a gate in place was not
/// available: the rows had to become a second array. They go **at the end** of the vocabulary
/// rather than the beginning because [`Operation`] is an **index** into it — a row inserted
/// above another moves that other one's number.
///
/// The cost is that in a build **with** the feature these seven numbers are now the highest ones
/// rather than numbers 5 to 11, and every row that used to sit below them moved down by seven.
/// That is harmless by construction: an index never leaves the process. It is produced by
/// [`Operation::from_name`], kept in the ring, and turned back into a name by
/// [`Operation::name`] at the moment the dump is written — and the dump is text. Nothing on disk,
/// in the configuration or in any protocol carries one.
///
/// The four published constants of [`Operation`] are numbers 0 to 3 and stand **above** the
/// removal, so they did not move at all.
#[cfg(feature = "testing")]
static CHANNEL_OPERATIONS: &[(&str, Kind)] = &[
    ("control::start", Kind::Channel),
    ("CloseHandle(token)", Kind::Channel),
    ("ConnectNamedPipe", Kind::Channel),
    ("DisconnectNamedPipe", Kind::Channel),
    ("write(control channel)", Kind::Channel),
    ("FlushFileBuffers", Kind::Channel),
    ("CloseHandle(pipe)", Kind::Channel),
];

/// Empty in the shipped program — see the documentation of the other half.
///
/// ⚠ **Empty and not absent.** The two halves are read through one pair of helpers
/// ([`row_at`] and [`row_count`]), so the code that walks the vocabulary is the same code in
/// both configurations and there is no second path to keep in step.
#[cfg(not(feature = "testing"))]
static CHANNEL_OPERATIONS: &[(&str, Kind)] = &[];

/// The row of the vocabulary at `index` — the core table first, the channel's rows after it.
///
/// One reader for both halves, so that [`Operation::name`], [`Operation::kind`] and
/// [`Operation::from_index`] cannot disagree about where the vocabulary ends.
fn row_at(index: usize) -> Option<&'static (&'static str, Kind)> {
    OPERATIONS
        .get(index)
        .or_else(|| CHANNEL_OPERATIONS.get(index.wrapping_sub(OPERATIONS.len())))
}

/// How many rows the vocabulary has **in this configuration** — task T-41-11.
///
/// Differs by seven between the two builds, which is the point of the task and is why the bound
/// check of [`Operation::from_index`] asks this rather than `OPERATIONS.len()`.
fn row_count() -> usize {
    OPERATIONS.len() + CHANNEL_OPERATIONS.len()
}

/// Every row of the vocabulary, in the order the indices run.
fn rows() -> impl Iterator<Item = &'static (&'static str, Kind)> {
    OPERATIONS.iter().chain(CHANNEL_OPERATIONS)
}

/// What happened, as an index into the vocabulary.
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
        rows()
            .enumerate()
            .skip(1)
            .find(|(_, row)| row.0 == name)
            .and_then(|(index, _)| u16::try_from(index).ok())
            .map_or(Self::UNLISTED, Self)
    }

    /// The name of this operation, for the dump.
    pub fn name(self) -> &'static str {
        row_at(usize::from(self.0)).map_or(OPERATIONS[0].0, |row| row.0)
    }

    /// The group this operation belongs to.
    pub fn kind(self) -> Kind {
        row_at(usize::from(self.0)).map_or(Kind::Unlisted, |row| row.1)
    }

    /// The index stored in a slot.
    const fn index(self) -> u16 {
        self.0
    }

    /// The value a slot decoded to, or [`Operation::UNLISTED`] if it names no row.
    /// ⚠ The bound is [`row_count`] and **not** `OPERATIONS.len()`: since task T-41-11 the
    /// vocabulary is seven rows longer in a build with the channel, and a check against the core
    /// table alone would decode every one of the channel's own entries as «unlisted» in the very
    /// build the channel exists in.
    fn from_index(index: u32) -> Self {
        u16::try_from(index)
            .ok()
            .filter(|&index| usize::from(index) < row_count())
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
///
/// # ⚠ Two writers on one slot, and the price this program has agreed to pay — finding м8, task
/// Т-22-10
///
/// The seqlock above is written for **one writer and many readers**, and this program has four or
/// five threads that record. The ticket keeps them from choosing the same slot in the ordinary
/// case — [`NEXT`] is a fetch-add, so no two writers ever get the same ordinal — but it does not
/// keep two of them off one *slot*: a writer preempted between clearing the stamp and restoring it
/// is a writer whose slot can be reached again by whoever wraps the whole ring of [`CAPACITY`]
/// entries while it is stopped. The second writer's stamp then stands over a mixture of the two
/// entries' fields, and a reader has no way to tell — the stamp it checks before and after is the
/// same non-zero value, which is exactly what the canon says a whole entry looks like.
///
/// **The cost is accepted rather than repaired, by the user's decision of 2026-09-01 (question
/// 80).** What it takes to reach: 1024 records — the whole ring — inside the window between two
/// instructions of a preempted thread, and then a reader arriving before the ring turns again. The
/// program records single-figure numbers of events in an ordinary session; the ring is a
/// **diagnostic** journal that nothing in the product reads, no decision is taken from and no
/// requirement rests on; and one mixed line in a file the user is asked to attach to a bug report
/// is a smaller price than a lock on a path NFR-04 forbids to take one, or a second stamp on every
/// entry paid by every record for ever.
///
/// So this is documented and not fixed, and the documentation is the point: a reader of a dump who
/// meets an operation and a code that do not belong together has this paragraph to find, instead
/// of a mystery. Nothing else in the module changed for it — the code below is exactly as task
/// T-06-4 wrote it.
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
/// # Replaced whole, or not at all — task T-34-1, finding Н101
///
/// The dump goes to a temporary file beside the target, is flushed to the device, and is then
/// renamed over the target — the shape `settings::write_to` has for `config.toml`, and for the
/// same reason. A write that opened the target itself and then failed would leave the dump of
/// the previous run truncated or empty, which is the one copy a person still had of what went
/// wrong last time. Now a refusal at any step leaves that file exactly as it was, and the
/// temporary file is removed on both failing branches.
///
/// # A refusal is recorded here, with its code
///
/// Whatever step refused, the ring receives `journal write refused` **with the Win32 number
/// the system gave** ([`OsCode::of_io`]) — not `S_OK`, which is what stood here before and made
/// every refusal look like nothing at all. Recorded in this function rather than by its
/// callers, so that there is one place, and so that the error is still handed back for the
/// caller to act on (NFR-13).
///
/// **UI thread only** — section 6.1.
pub fn write_to(path: &Path) -> io::Result<()> {
    write_as(path, Session::Continues)
}

/// [`write_to`] with the state of the session spelled out in the header — task T-34-3.
///
/// [`dump_on_shutdown`] writes with [`Session::Ended`]; everything else that writes a dump does
/// so while the program runs and says so.
pub fn write_as(path: &Path, session: Session) -> io::Result<()> {
    let outcome = write_replacing(path, session);

    if let Err(error) = &outcome {
        record(Operation::JOURNAL_WRITE_REFUSED, OsCode::of_io(error));
    }

    outcome
}

/// The three steps of [`write_as`]: the folder, the temporary file, the rename.
fn write_replacing(path: &Path, session: Session) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path_for(path);

    // The removals below are the cleanup of a file this function created a moment ago. Their
    // result is not examined (NFR-13) for the reason `settings::write_to` gives: the error that
    // matters is the one being returned, and a `.tmp` that refused to go away is a stray file
    // beside the dump and nothing worse.
    if let Err(error) = write_and_sync(&temporary, render_as(session).as_bytes()) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    Ok(())
}

/// Names the temporary file [`write_replacing`] writes before the rename, beside the target.
///
/// Beside it and not in `%TEMP%`, because a rename is atomic within one volume only and
/// `%TEMP%` may be on another. The process id keeps two instances of the program from
/// colliding on one name; within one process the dump is written by the UI thread alone
/// (section 6.1), so nothing finer is needed.
fn temporary_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_else(|| OsString::from(LOG_FILE_NAME), |name| name.to_os_string());
    name.push(format!(".{}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Writes `bytes` to `path` and flushes them to the device before returning.
///
/// The flush is the point: without it the rename can reach the disk ahead of the contents,
/// and a power cut leaves an empty dump where a whole one was promised.
fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Dumps the journal if the session asks for it. Called once, on the **UI thread**, as it
/// leaves its message loop.
///
/// # Why the setting is the published one and not the file — task T-34-2, finding С47
///
/// Until this task the decision was taken by reading `config.toml` again, here, at shutdown,
/// and a file that would not read meant «off». That answered the wrong question: the file is
/// what the disk holds, and the disk can hold something other than what the person set in this
/// session — a save that failed, a file edited by hand after start-up, a file damaged since.
/// The journal then went silent exactly when it had been switched on, or wrote when it had
/// been switched off. The setting every other module acts on is the one
/// `app::publish_configuration` publishes at start-up and on every «Применить» (section 6.3),
/// and this module now listens to the same publication: [`set_log_enabled`] is written there,
/// [`log_enabled`] is what this function reads, and the file is not opened again. The read is
/// on the UI thread, which is also where the write happens — section 6.1 — so nothing crosses
/// a thread.
///
/// # When it is off
///
/// `log_enabled = false` is the default of section 7 and the ordinary case. Then this function
/// creates nothing: no folder, no file, not an empty one. That is the difference between a
/// journal that is off and a journal that is on and had nothing to say.
pub fn dump_on_shutdown() {
    let Some(target) = log_path() else {
        return;
    };

    dump_on_shutdown_to(&target);
}

/// The published `[diagnostics] log_enabled` of this session — task T-34-2, finding С47.
static LOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Publishes `[diagnostics] log_enabled` of the configuration of this session — task T-34-2.
///
/// Called by `app::publish_configuration` at start-up and on every «Применить», which is how
/// every other setting reaches the module that acts on it (section 6.3). Written and read on
/// the UI thread alone, so `Relaxed` is all the store needs; an atomic rather than a `Cell`
/// because a `static` has to be one.
pub fn set_log_enabled(enabled: bool) {
    LOG_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether the journal of this session is to be written — the published setting.
pub fn log_enabled() -> bool {
    LOG_ENABLED.load(Ordering::Relaxed)
}

/// [`dump_on_shutdown`] with its target made explicit, for the tests. Answers whether the dump
/// reached the file.
///
/// The one input besides the target is [`log_enabled`] — the published setting of the session.
/// No path to a configuration file is taken, and none is read: that is the whole of task
/// T-34-2, and `tests\diag.rs` plants a broken file and a file that says the opposite of the
/// session to show that neither is consulted.
pub fn dump_on_shutdown_to(target: &Path) -> bool {
    if !log_enabled() {
        return false;
    }

    // Recorded before the dump is rendered so that the file says it was written.
    record(Operation::JOURNAL_WRITTEN, OsCode::NONE);

    if let Err(_recorded) = write_as(target, Session::Ended) {
        // NFR-13: the outcome is examined here as well as inside `write_to`, which has already
        // put the refusal into the ring with the code the system gave (task T-34-1). Nothing
        // more can be done with it on this path: the place a report would go is the file that
        // just refused, and this is the last write of the process. The `io::Error` itself goes
        // no further — its number is in the ring, and its text is text, which nothing in this
        // module keeps.
        return false;
    }

    true
}

// ---------------------------------------------------------------------------------------
// Rendering — the only place in this module where text exists
// ---------------------------------------------------------------------------------------

/// Whether the session was still running when a dump was taken — task T-34-3, finding С48.
///
/// The one word that tells the dump the button writes from the dump the previous run left:
/// until this task the two were the same text with a different uptime, and a person opening
/// the journal folder could not say which run they were reading about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Session {
    /// Written while the program runs — the button «Сохранить журнал» of the settings dialog.
    Continues,
    /// Written by [`dump_on_shutdown`], the last act of the UI thread.
    Ended,
}

impl Session {
    /// The word the header carries.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Continues => "continues",
            Self::Ended => "ended",
        }
    }
}

/// The whole dump as text, taken while the program runs — [`render_as`] with
/// [`Session::Continues`].
pub fn render() -> String {
    render_as(Session::Continues)
}

/// The wall clock at the moment of the dump: `YYYY-MM-DD HH:MM:SS`, local time — task T-34-3.
///
/// Local time and not UTC, because the person reading the dump lives in the calendar of the
/// machine; the invariant locale and a fixed picture, so that the digits and the calendar are
/// this file's and not the user's. `None` when either clock refused, which NFR-13 answers by
/// naming the refusal in the ring (`letters::today` names its own) and printing `(unknown)`.
fn wall_clock() -> Option<String> {
    let date = crate::letters::today()?;
    let time = local_time()?;

    Some(format!("{date} {time}"))
}

/// The time of day by the clock of this machine, `HH:MM:SS` in local time.
fn local_time() -> Option<String> {
    // `HH:MM:SS` and the terminator: nine units is the exact answer, and the buffer is asked
    // for more so that a longer answer is truncated rather than refused.
    let mut buffer = [0u16; 32];

    // SAFETY: the locale name and the picture are static NUL-terminated literals of this
    // image; the buffer is a live local of this frame and its length is what the call is told;
    // `None` for the time is the documented «the current local time». The call writes into the
    // buffer and reads nothing else of ours.
    let written = unsafe {
        GetTimeFormatEx(
            LOCALE_NAME_INVARIANT,
            TIME_FORMAT_FLAGS(0),
            None,
            w!("HH':'mm':'ss"),
            Some(&mut buffer),
        )
    };

    if written <= 0 {
        crate::app::report_non_critical("GetTimeFormatEx", &WinError::from_thread());
        return None;
    }

    // The count includes the terminator, which is not part of the text.
    let units = usize::try_from(written).ok()?.saturating_sub(1);

    String::from_utf16(buffer.get(..units)?).ok()
}

/// The whole dump as text.
///
/// # What is in it, and what is deliberately not
///
/// A header — the shape of the ring, the uptime, and since task T-34-3 the wall clock of the
/// moment and whether the session still ran — and two sections. **Events** is the ring.
/// **Counters** is a snapshot of the counters the
/// earlier tasks already keep and already publish — this module reads them through their
/// public accessors and keeps no counter of its own, because a second copy of a number is a
/// second number to be wrong.
///
/// The counters printed are the *failure and health* counters named by this task: the hook
/// going up and coming down, the recoveries of the watchdog, the layout cache that would not
/// build, each method of the chain of FR-50, the `SendInput` return-value discrepancies, the
/// refused `PostMessage`s. Task T-34-4 (finding Н95) added the fourteen counters of the
/// password-field guard (`guard.*`) and the fifteen of the clipboard path (`selection.*`),
/// through the readers those two modules publish; task T-38-7 (finding Н16) added a sixteenth,
/// `selection.format_list_truncations`, and task T-38-8 (finding С11) a seventeenth,
/// `selection.snapshots_saved_nothing`. Task T-71-3 (backlog line Э36-Б-3) added
/// `settings.capture_refusals` — presses a hotkey capture refused: a deliberate act in one field of
/// one dialog and not typing, which is why it is not the kind of number the paragraph below keeps
/// out.
///
/// `watchdog::counters` and `hook::hotkey_handoffs` are **left out on purpose**. They are
/// honest counts of program events and SEC-07 would permit them, but they are proportional to
/// how much the user typed and clicked, and a number proportional to typing has no business in
/// a file when SEC-01 is what this module exists to honour. Leaving them out costs the
/// diagnostic nothing: not one of them can distinguish a program that is working from one that
/// is not.
pub fn render_as(session: Session) -> String {
    let mut out = String::with_capacity(8192);

    let _ = writeln!(out, "{} diagnostic journal", crate::APP_NAME);
    let _ = writeln!(out);
    row_usize(&mut out, "journal.capacity", CAPACITY);
    row_usize(&mut out, "journal.footprint_bytes", footprint_bytes());
    row_u64(&mut out, "journal.recorded", recorded());
    row_u32(&mut out, "journal.uptime_ms", elapsed_ms());
    row_word(
        &mut out,
        "journal.taken_at",
        wall_clock().as_deref().unwrap_or("(unknown)"),
    );
    row_word(&mut out, "journal.session", session.word());

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
    // Task T-36-4, finding Н40: the presses among those failures that were the user's own and
    // reached nobody. Beside the sum rather than instead of it — the sum keeps the meaning every
    // earlier dump gave it.
    row_u32(out, "hook.lost_hotkeys", crate::hook::lost_hotkeys());
    row_u32(out, "hook.unhook_failures", unhook_failures);
    row_hex(out, "hook.last_unhook_error", last_unhook_error as i32);
    row_flag(
        out,
        "hook.emergency_terminate_failed",
        crate::hook::emergency_terminate_failed(),
    );
    // Task T-71-3, backlog line Э36-Б-3: presses the hotkey capture of the settings dialog refused.
    // Next to the hook's rows and not among them — the second number here that means something to
    // a person, after `hook.lost_hotkeys`, and one that belongs to the dialog.
    row_u32(
        out,
        "settings.capture_refusals",
        crate::settings::capture_refusals(),
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
    // Task T-41-13, finding Н2: `WM_APP_FLUSH` messages that found nothing waiting. ⚠ Forged ones
    // and honestly coalesced ones both land here and the number does not separate them — see
    // `watchdog::FLUSHES_WITHOUT_REQUEST`. What it buys is that a flood stops being invisible.
    row_u32(
        out,
        "watchdog.flushes_without_request",
        health.flushes_without_request,
    );
    row_u32(out, "watchdog.session_changes", health.session_changes);
    row_u32(out, "watchdog.power_resumes", health.power_resumes);
    // Task T-37-1, finding Н3: requests to put the hook back whose post reached no window. The
    // reason is taken back rather than left armed, and this is where the loss shows.
    row_u32(out, "watchdog.rehook_posts_lost", health.rehook_posts_lost);
    // Task T-37-1, finding С23: focus changes whose `WM_APP_FLUSH` reached no window. Each of them
    // was played by the next message of the input window instead, so the number is of focus
    // changes that came late, never of focus changes lost.
    row_u32(out, "watchdog.flush_posts_lost", health.flush_posts_lost);
    // Task T-69-1, backlog line Э37-Б-1: wipes of rows 8 and 9 of FR-10 — a session lock, a pause
    // from the tray — whose `WM_APP_WIPE` reached no window. Each was played by the next message of
    // the input window instead, so the number is of wipes that came late, never of wipes lost.
    row_u32(out, "watchdog.wipe_posts_lost", health.wipe_posts_lost);

    row_u32(
        out,
        "layouts.cache_failures",
        crate::app::layout_cache_failures(),
    );

    let selection = crate::layouts::selection_failures();

    row_u32(out, "layouts.ime_layout", selection.ime_layout);
    row_u32(out, "layouts.no_layouts", selection.no_layouts);
    row_u32(out, "layouts.origin_outside", selection.origin_outside);
    row_u32(out, "layouts.too_many_layouts", selection.too_many_layouts);

    let (send_mismatches, events_lost) = crate::inject::send_mismatches();

    row_u32(out, "inject.send_mismatches", send_mismatches);
    row_u32(out, "inject.events_lost", events_lost);

    // Task T-39-8, finding Н118: the scope of FR-51, read here — on the thread that renders the
    // dump, the UI thread — and before the counters are taken, so that a read that fails is
    // already in `switch.scope_unreadable` below. Never on the switching path.
    let scope = crate::switch::scope();
    let failures = crate::switch::failures();

    // ⚠ Six rows left this section in task Т-14-4 — `attach_activate`, `text_services`,
    // `activate_rejected`, `activate_profile_rejected`, `detach_failed` and `exhausted`. They
    // counted methods 2 and 3 of the old chain of FR-50 and the end of that chain; the user struck
    // both methods out of the requirement (question 63 of `DECISIONS.md`), so the counters behind
    // them no longer exist. `switch.sent_unconfirmed` is the row that arrived in their place: the
    // addendum of FR-52, a switch sent to a window no verdict can be taken about.
    row_u32(out, "switch.post_message", failures.post_message);
    row_u32(out, "switch.post_rejected", failures.post_rejected);
    row_u32(out, "switch.sent_unconfirmed", failures.sent_unconfirmed);
    row_u32(out, "switch.ime_target", failures.ime_target);
    row_u32(out, "switch.no_target", failures.no_target);
    row_u32(out, "switch.no_foreground", failures.no_foreground);
    row_u32(out, "switch.scope_unreadable", failures.scope_unreadable);
    row_word(out, "switch.scope", scope.name());

    // Task T-34-4, finding Н95: the fourteen counters of the password-field guard of FR-70…FR-73
    // and the fifteen of the clipboard path of FR-60…FR-65, through the public readers both
    // modules already publish; tasks T-38-7 and T-38-8 added a sixteenth and a seventeenth of the
    // clipboard path, and tasks T-37-3 and T-37-6 a fifteenth and a sixteenth of the guard. Counts
    // and nothing else — SEC-07 allows a number and forbids a name of what was observed, and none
    // of these carries one. `tests\diag.rs` builds this list a second time
    // by taking the two structures apart field by field, so a counter added there and not here is
    // a compile error rather than a silent hole.
    let guard = crate::guard::counters();
    row_u32(out, "guard.focus_changes", guard.focus_changes);
    row_u32(out, "guard.probes", guard.probes);
    row_u32(out, "guard.stale_verdicts", guard.stale_verdicts);
    row_u32(out, "guard.password_verdicts", guard.password_verdicts);
    row_u32(out, "guard.ordinary_verdicts", guard.ordinary_verdicts);
    row_u32(
        out,
        "guard.undetermined_verdicts",
        guard.undetermined_verdicts,
    );
    row_u32(out, "guard.level1_verdicts", guard.level1_verdicts);
    row_u32(out, "guard.level2_verdicts", guard.level2_verdicts);
    row_u32(out, "guard.level2_timeouts", guard.level2_timeouts);
    row_u32(out, "guard.level3_failures", guard.level3_failures);
    row_u32(out, "guard.excluded_verdicts", guard.excluded_verdicts);
    row_u32(out, "guard.exclusions", guard.exclusions);
    row_u32(out, "guard.exclusions_refused", guard.exclusions_refused);
    row_u32(
        out,
        "guard.exclusion_read_retries",
        guard.exclusion_read_retries,
    );
    // Task T-37-3, finding С5: verdicts refused because the control they were determined for lost
    // the focus while the probe ran. Apart from `guard.stale_verdicts`, which is the refusal by
    // generation — a dump that summed the two would hide which race a machine is losing.
    row_u32(out, "guard.mismatched_verdicts", guard.mismatched_verdicts);
    // Task T-37-6, decision 126г: probes asked for because the user's desktop came back. One UAC
    // prompt answered without a focus change is one here; nought after one is the gap still open.
    row_u32(
        out,
        "guard.desktop_return_probes",
        guard.desktop_return_probes,
    );

    let clipboard = crate::selection::counters();
    row_u32(out, "selection.opens", clipboard.opens);
    row_u32(out, "selection.closes", clipboard.closes);
    row_u32(out, "selection.close_failures", clipboard.close_failures);
    row_u32(out, "selection.open_retries", clipboard.open_retries);
    row_u32(out, "selection.open_refusals", clipboard.open_refusals);
    row_u32(
        out,
        "selection.wrong_thread_refusals",
        clipboard.wrong_thread_refusals,
    );
    row_u32(out, "selection.updates", clipboard.updates);
    row_u32(out, "selection.own_updates", clipboard.own_updates);
    row_u32(out, "selection.foreign_updates", clipboard.foreign_updates);
    row_u32(out, "selection.truncations", clipboard.truncations);
    row_u32(
        out,
        "selection.format_list_truncations",
        clipboard.format_list_truncations,
    );
    row_u32(
        out,
        "selection.snapshots_saved_nothing",
        clipboard.snapshots_saved_nothing,
    );
    row_u32(out, "selection.refused_formats", clipboard.refused_formats);
    row_u32(out, "selection.handle_formats", clipboard.handle_formats);
    row_u32(
        out,
        "selection.listener_remove_failures",
        clipboard.listener_remove_failures,
    );
    row_u32(out, "selection.restore_skips", clipboard.restore_skips);
    row_u32(
        out,
        "selection.restore_failures",
        clipboard.restore_failures,
    );

    // ⭐ **The seven counters of the selection path itself.** Everything above this line counts
    // the **clipboard** — opens, formats, restores. These count the **press**: what the path did
    // with it, and which of the eight roads of `wants_selection_path` and of the eight steps of
    // FR-61 it went down. They are `selection::PathCounters`, a second structure of the same
    // module, and until this task not one of them reached a dump: the reader above takes
    // `selection::counters()` apart and there was no reader for this one.
    //
    // ⚠ **What the hole cost, said plainly.** A press that comes back as a single click — the
    // idle tone of FR-100 — can mean four different things: step 3 timed out and there was no
    // selection (`no_selection`), the path refused for one of the reasons of `Refusal`
    // (`refusals`), the window in front was a console or a window of our own thread
    // (`console_refusals`, `own_window_refusals`), or the path was never entered because the
    // typing buffer was not empty — in which case none of the seven moves at all. Without these
    // rows a dump could not tell those apart, and the difference is the whole diagnosis.
    // `late_copies` is the fifth answer and the one a person could never guess: the application
    // answered the `Ctrl+C` of step 2 **after** step 3 had given up, so the press was read as «no
    // selection» although there was one — the race of tasks T-13-11 and T-38-4.
    //
    // SEC-01 and SEC-07: seven counts of this program's own decisions. Not one of them is
    // derived from what was selected, copied or typed, and none can be turned back into it.
    let path = crate::selection::path_counters();
    row_u32(out, "selection.handovers", path.handovers);
    row_u32(out, "selection.conversions", path.conversions);
    row_u32(out, "selection.no_selection", path.no_selection);
    row_u32(out, "selection.refusals", path.refusals);
    row_u32(out, "selection.console_refusals", path.console_refusals);
    row_u32(
        out,
        "selection.own_window_refusals",
        path.own_window_refusals,
    );
    row_u32(out, "selection.late_copies", path.late_copies);
}

/// Width of the name column of the counter section. One place, so the columns line up.
// 34 until task T-34-4, when `selection.listener_remove_failures` — thirty-four characters —
// arrived and its value stood glued to its name: a name as wide as the column leaves no gap.
// Two wider than the longest name, and the test of the counters reads every row by «name,
// then whitespace», so the next name to outgrow the column is caught there.
const NAME_WIDTH: usize = 36;

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
