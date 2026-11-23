//! The tray icon, the menu, handling `TaskbarCreated`.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-90 (the icon and its two states), FR-91 (the context
//! menu), FR-81 (`TaskbarCreated`), and the part of FR-83 that exists today — removing the
//! icon and saving the configuration. The other three actions of FR-83 belong elsewhere:
//! releasing the mutex is already done by [`crate::app`], unhooking by [`crate::app`]'s window
//! procedure (task T-03-1), and wiping the buffer by `park_buffer` and by the drop of the
//! recorder the input thread owns (task T-03-2) — see [`Tray::shut_down`].
//! The list follows the backlog rather than the stub line this file used to carry —
//! decision R-17: FR-83 is the backlog's and the backlog names T-01-4.
//! Implemented by backlog tasks: T-01-4 (done); T-08-1 (done) filled the two marked stubs —
//! the menu now opens the settings dialog of FR-92, which lives in [`crate::settings`], and
//! the check mark of FR-93 now writes and removes the value under
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; the menu, drawn by this file on the
//! menu of Windows from T-11-10 to T-12-9, is a window of the shell since T-120-15
//! (`ui::windows::tray_menu`): the line of the state on top, icons, the key of the hotkey, the
//! switch of autostart.
//! T-13-6 (done) closed the finding of the audit of 2026-08-24: what [`settings::read_or_default`]
//! answers about the file is now acted on rather than dropped, so a configuration this build
//! could not read is kept before anything is written over it and one from a newer build is not
//! written to at all — see [`Tray::save_config`].
//! T-13-9 (done) closed a second finding of the same audit, the low one: while FR-99 holds the
//! program disarmed the resumption of FR-90 is refused rather than performed, so the icon can
//! no longer say "активна" over dead processing — [`resume_is_refused`], [`Tray::toggle_state`]
//! and the greyed first entry of [`Menu::build`].
//! T-13-14 (done) closed a third, the middle one: the menu of FR-91 was fully alive while the
//! modal dialog of FR-92 was on the screen, so a suspension or an autostart made from the tray
//! was silently undone by the next «Применить» — two editors of one configuration, which
//! section 6.3 does not allow. While the dialog is up the two entries that edit the
//! configuration are greyed ([`Menu::build`]) **and** refused ([`dispatch_command`]) — one
//! rule, [`dialog_locks_command`], read in both places.
//! T-43-4 (done) closed finding С21 of the audit of 2026-09-04 on the same rule: while the
//! settings dialog **or** the «О программе» window is up, «Настройки…» and «О программе» are
//! greyed and refused as well, the double click on the icon goes through the same door, and
//! `settings::show_about_dialog` turns round at its top — no modal window opens over another.
//!
//! # Where this lives — section 6.1
//!
//! Everything here runs on the UI thread and nowhere else. The tray is installed by
//! [`attach`] from `app::serve_window` right after that thread has created its window, and
//! it is reachable only through a thread-local, so a call from the input thread finds
//! nothing and does nothing rather than touching the shell from the hook path.
//!
//! The UI window is a hidden *top-level* window rather than a message-only one, which is
//! decision R-20 point 2 and exists for FR-81: `RegisterWindowMessage("TaskbarCreated")` is
//! broadcast, and a broadcast never reaches an `HWND_MESSAGE` window.
//!
//! # Why the menu is not shown while the tray is borrowed
//!
//! The menu of FR-91 is a window of the shell since task T-120-15 (`ui::windows::tray_menu`):
//! it reads the tray to build itself and, when its switch is pressed, changes the tray, so it is
//! never made while the thread-local `RefCell` is held. [`Tray::handle_message`] answers
//! [`Reaction::ShowMenu`], and [`handle_ui_message`] shows the menu once the borrow is dropped.
//! The command chosen in it runs after the menu is gone, on [`WM_APP_MENU_CHOSEN`]: «Настройки…»
//! runs a modal loop of its own, and that loop must not start inside a message of the menu.
//!
//! # SEC-05
//!
//! Only the messages named in [`Tray::handle_message`] are handled; everything else is
//! answered with [`Reaction::Ignored`] and goes to `DefWindowProcW`. The callback message of
//! the icon can be posted by any process at the same integrity level, and all that achieves
//! is a menu on the screen — or, for the double click, the settings dialog of FR-92, which is
//! the same kind of thing: a window that changes nothing until the person in front of it
//! presses «Применить». Neither is a privileged action, neither writes anything anywhere, and
//! both are dismissed by pressing `Esc`. There is deliberately **no `WM_COMMAND` handler**: the
//! command chosen in the menu is a number the menu of this thread writes down as it closes,
//! and [`WM_APP_MENU_CHOSEN`] carries nothing — a forged one finds no command and does nothing.
//! No privileged action is reachable by message.
//!
//! One menu at a time: while it is up, a press on the icon closes it ([`open_menu_window`]).
//!
//! # SEC-01, SEC-07
//!
//! Nothing here ever sees a keystroke, a key code or the contents of the buffer, and nothing
//! may be added later that would: the tooltip is built from [`crate::APP_NAME`] and a state
//! word, the about box from that name and the version resource, and the menu from fixed
//! strings.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetMonitorInfoW, LOGFONTW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    ReleaseDC,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_LARGE_ICON, NIIF_USER, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NIN_BALLOONUSERCLICK, NIN_SELECT, NOTIFYICON_VERSION_4,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetClassNameW, GetCursorPos, GetSystemMetrics, HICON, IMAGE_ICON, LR_DEFAULTCOLOR,
    LoadImageW, NONCLIENTMETRICSW, PostMessageW, RT_VERSION, RegisterWindowMessageW, SM_CXICON,
    SM_CXSMICON, SM_CYICON, SM_CYSMICON, SPI_GETNONCLIENTMETRICS, SYSTEM_METRICS_INDEX,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetForegroundWindow, SetTimer, SystemParametersInfoW,
    WM_APP, WM_CONTEXTMENU, WM_ENDSESSION, WM_LBUTTONDBLCLK, WM_QUERYENDSESSION, WM_SETTINGCHANGE,
    WM_THEMECHANGED, WM_TIMER, WM_USER,
};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult, w};

use crate::layouts::LayoutId;
use crate::settings::{self, Config, Quarantined, SavePolicy};
use crate::{APP_NAME, app, diag, theme};

// ---------------------------------------------------------------------------------------
// The shell, and the tests kept out of it — 208.236 п. 6 (208.226, 208.232)
// ---------------------------------------------------------------------------------------

/// **The one door of this module to `Shell_NotifyIconW`** — 208.236 п. 6.
///
/// Every call of this module is still written `Shell_NotifyIconW(NIM_…, &data)`; the name is
/// this function now and not the import, so that one place decides where the call goes. On a
/// thread a test has kept out of the shell — [`keep_this_thread_out_of_the_shell`] — the call
/// does not leave the process and answers success: the icon "is there" for every count and flag
/// of [`Tray`], and nothing is added to the notification area. Everywhere else, and always in
/// the build that ships ([`shell_may_be_left_out`]), it is the call of Windows itself.
///
/// Why at all: Windows remembers an icon without a GUID by the path of the image and its uID
/// (`reports\TRAY-DUPES-180.md`), so each folder of a target and each hash of a test image used
/// to leave a row of its own under «Другие значки области уведомлений» on the machine the tests
/// ran on — 27 of them on 2026-11-23 (208.226).
///
/// # Safety
///
/// The contract of `Shell_NotifyIconW`: `data` points at a live, fully initialised
/// `NOTIFYICONDATAW` whose `cbSize` describes it. It is only read, and not kept.
#[allow(non_snake_case)]
unsafe fn Shell_NotifyIconW(
    message: windows::Win32::UI::Shell::NOTIFY_ICON_MESSAGE,
    data: *const NOTIFYICONDATAW,
) -> windows::core::BOOL {
    if this_thread_is_out_of_the_shell() {
        return true.into();
    }

    SHELL_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));

    // SAFETY: the caller's contract, passed on unchanged.
    unsafe { windows::Win32::UI::Shell::Shell_NotifyIconW(message, data) }
}

thread_local! {
    /// Whether a test has kept this thread out of the shell — read only where
    /// [`shell_may_be_left_out`] lets it be.
    static OUT_OF_THE_SHELL: Cell<bool> = const { Cell::new(false) };

    /// The calls of this thread that went to the shell itself.
    static SHELL_CALLS: Cell<u32> = const { Cell::new(0) };
}

/// **Whether a build may keep a thread out of the shell** — 208.236 п. 6: a build with debug
/// assertions (both batteries are one) or with the feature `testing`; **never the build that
/// ships**, `--release` without `testing`, whose icon must reach the notification area whatever
/// asked otherwise.
///
/// `cfg(test)` cannot be the condition: `tests\tray.rs` links the library built without it. Nor
/// can `testing` alone — the plain battery runs without it, and half the runs would still leave
/// rows behind (the reasoning of [`settings::autostart_may_register`]). Pure and public for the
/// same reason as that function: the batteries are not the build that ships, and they check its
/// answer here.
pub const fn shell_may_be_left_out(debug_build: bool, testing: bool) -> bool {
    debug_build || testing
}

/// What [`shell_may_be_left_out`] answers for this very build.
const THIS_BUILD_MAY_LEAVE_THE_SHELL_OUT: bool =
    shell_may_be_left_out(cfg!(debug_assertions), cfg!(feature = "testing"));

/// Keeps the trays of this thread out of the shell (`true`) or sends them to it again (`false`)
/// — for the tests, which call it before they install a tray; nothing in the program does.
///
/// Per thread, because the tests of one image run side by side, and the few that are about the
/// shell itself go to it while the rest stay out. In the build that ships the request is held
/// and never read: [`this_thread_is_out_of_the_shell`] answers `false` there.
pub fn keep_this_thread_out_of_the_shell(out: bool) {
    OUT_OF_THE_SHELL.with(|flag| flag.set(out));
}

/// Whether the `Shell_NotifyIconW` of this thread stays in the process — see
/// [`keep_this_thread_out_of_the_shell`].
pub fn this_thread_is_out_of_the_shell() -> bool {
    THIS_BUILD_MAY_LEAVE_THE_SHELL_OUT && OUT_OF_THE_SHELL.with(Cell::get)
}

/// How many calls of this thread went to the shell itself — what a test reads to know that it
/// stayed out, or that it really reached the notification area.
pub fn shell_calls_on_this_thread() -> u32 {
    SHELL_CALLS.with(Cell::get)
}

// ---------------------------------------------------------------------------------------
// The menu of FR-91 — labels and commands
// ---------------------------------------------------------------------------------------

// ⚠ **The five labels of FR-91 are no longer literals here** — FR-94, task T-08-2. They are
// rows of the two string tables of `app.rc`, and [`Menu::build`] reads them out of the table of
// the locale `general.language` chose, exactly as the settings dialog reads its own. The
// identifiers live in `settings` with the rest of the interface vocabulary, because the table
// they index is one table and splitting its numbering across two modules is how numbering
// drifts.
//
// The ellipsis of «Настройки…» is one character, U+2026, in both locales; that is a property of
// the string table now and a test reads it back out of the built binary.

/// Command identifier of the first item of FR-91.
///
/// The values start well above the range Windows uses for its own dialog commands (`IDOK`
/// is 1, `IDHELP` is 9) so that a stray identifier cannot be mistaken for one of ours. Zero
/// is excluded by `TrackPopupMenuEx`, which reports "nothing was chosen" with a zero return.
pub const CMD_TOGGLE: u32 = 0x1001;

/// Command identifier of `Настройки…` — FR-92, task T-08-1.
pub const CMD_SETTINGS: u32 = 0x1002;

/// Command identifier of `Запускать при входе в систему` — FR-93, task T-08-1.
pub const CMD_AUTOSTART: u32 = 0x1003;

/// Command identifier of `О программе`.
pub const CMD_ABOUT: u32 = 0x1004;

/// Command identifier of `Выход`.
pub const CMD_EXIT: u32 = 0x1005;

/// Command identifier of the permanent entry `Написать автору…` — FR-91, task Т-32-4,
/// решение 101 п. 9. It stands above `О программе`.
pub const CMD_WRITE: u32 = 0x1006;

/// Command identifier of the temporary entry `Непрочитанное письмо…` — FR-91. Present only
/// while a news item of the feed is unread.
pub const CMD_UNREAD: u32 = 0x1007;

/// Command identifier of the temporary entry `Доступна версия X…` — FR-91. Present only while
/// the feed names a version newer than this one.
pub const CMD_UPDATE: u32 = 0x1008;

/// Command identifier of the way into «От автора» from the about window — FR-103.
///
/// A number of its own rather than [`CMD_WRITE`], although stage А sends both to the same
/// window: the two entries mean different things to a person (one says «write», the other says
/// «from the author»), and stage В gives the first one the wizard. A command that changed
/// meaning under an installed build would be exactly the trap `app.rc` warns about for control
/// identifiers.
pub const CMD_AUTHOR: u32 = 0x1009;

/// Command identifier of the temporary entry `Что нового в X…` — task T-119-7 (решения 208.93
/// п. 5, 208.94 п. 5). Present for a week after **any** update.
pub const CMD_WHATSNEW: u32 = 0x100A;

/// Number of entries FR-91 puts in the menu when nothing temporary is due: **six** commands and
/// two separators.
///
/// Spelled out because it is the number a reader is most likely to get wrong. ⚠ **Seven until
/// task Т-32-4**: FR-91's addition puts the permanent entry «Написать автору…» above «О
/// программе», and the two temporary entries of FR-101 stand above the first rule *only while
/// there is a reason for them* — so this is the count of the menu at rest, and
/// [`Menu::build`] answers a longer one when the feed has something to say.
pub const MENU_ENTRY_COUNT: i32 = 8;

// ---------------------------------------------------------------------------------------
// Private constants
// ---------------------------------------------------------------------------------------

/// Resource identifier of the "active" icon in `app.rc` — also the icon on the caption of every
/// window of the shell (`ui::host`, Б1 of AUDIT-180).
pub(crate) const IDI_APP_ACTIVE: u16 = 101;

/// Resource identifier of the "suspended" icon in `app.rc`.
const IDI_APP_PAUSED: u16 = 102;

/// The "active, and a letter is unread" icon of FR-90's third state — task Т-32-4.
const IDI_APP_ACTIVE_UNREAD: u16 = 103;

/// The "suspended, and a letter is unread" icon.
const IDI_APP_PAUSED_UNREAD: u16 = 104;

/// The darkbar set — the same four states without the plate, for a dark taskbar; FR-90, task
/// T-86-2, решение 145.1. The "active" icon of that set.
const IDI_TRAY_DARK_ACTIVE: u16 = 105;

/// The "suspended" icon of the darkbar set.
const IDI_TRAY_DARK_PAUSED: u16 = 106;

/// The "active, and a letter is unread" icon of the darkbar set.
const IDI_TRAY_DARK_ACTIVE_UNREAD: u16 = 107;

/// The "suspended, and a letter is unread" icon of the darkbar set.
const IDI_TRAY_DARK_PAUSED_UNREAD: u16 = 108;

/// Identifier of our one and only notify icon, unique within this window.
const TRAY_ICON_ID: u32 = 1;

/// The message the shell posts to our window on behalf of the icon.
///
/// `WM_APP + 1` is taken: it is the wake-up message of [`crate::app`]. A `WM_APP` message
/// means nothing outside the process that defines it, so a value that collides with nothing
/// of ours is all that is required.
const WM_APP_TRAY: u32 = WM_APP + 2;

/// `NIN_KEYSELECT`, which the `windows` crate metadata does not carry.
///
/// Spelled out from `shellapi.h`, where it is `WM_USER + 1`. It is what the shell sends when
/// the icon is chosen from the keyboard rather than with the mouse; leaving it out would make
/// the tray unreachable without a pointing device.
const NIN_KEYSELECT: u32 = WM_USER + 1;

/// Name of the broadcast message FR-81 is about.
const TASKBAR_CREATED: PCWSTR = w!("TaskbarCreated");

/// Offset of `VS_FIXEDFILEINFO` inside a `VS_VERSIONINFO` resource.
///
/// The header before it is fixed in size: `wLength`, `wValueLength` and `wType` are two
/// bytes each, `szKey` is the 16 UTF-16 units of `"VS_VERSION_INFO"` including its
/// terminator, and two bytes of padding bring 38 up to the four-byte boundary the structure
/// is required to start on.
const FIXED_FILE_INFO_OFFSET: usize = 40;

/// Length of the part of `VS_FIXEDFILEINFO` this module reads: signature, structure version
/// and the two version words.
const FIXED_FILE_INFO_READ: usize = 16;

/// `dwSignature` of `VS_FIXEDFILEINFO`. Verified rather than assumed: it is the only thing
/// that tells a real version resource from a buffer that merely has the right length.
const FIXED_FILE_INFO_SIGNATURE: u32 = 0xFEEF_04BD;

// ---------------------------------------------------------------------------------------
// The public surface
// ---------------------------------------------------------------------------------------

/// What the window procedure must do once the tray has looked at a message.
///
/// The menu is a variant rather than something [`Tray::handle_message`] does itself because
/// `TrackPopupMenuEx` is modal and must not run while the tray is borrowed. See the module
/// documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// None of ours. Hand the message to `DefWindowProcW` unchanged.
    Ignored,
    /// Fully handled; this is the value to return to Windows.
    Handled(LRESULT),
    /// Handled, except that the menu of FR-91 has to be shown at this point on the screen.
    ShowMenu {
        /// Screen x the menu opens at — **task T-41-3**: the point the message asked for,
        /// pulled onto the work area of the monitor nearest it.
        x: i32,
        /// Screen y, pulled the same way.
        y: i32,
        /// Whether this showing came from the keyboard (`NIN_KEYSELECT`) rather than the mouse.
        ///
        /// It decides whose point wins in [`show_menu`]: the mouse's showings follow the
        /// **cursor**, asked of the system there; the keyboard's follow the shell's own point,
        /// which is the icon's, because the cursor may be anywhere at all when a person opens
        /// the menu with Enter — on another monitor, in another program's window.
        from_the_keyboard: bool,
    },
    /// Handled, except that the settings dialog of FR-92 has to be opened.
    ///
    /// A variant for the same reason [`Reaction::ShowMenu`] is one: the dialog is modal, it runs
    /// a message loop that comes back into this module, and it must therefore not be opened
    /// while the tray is borrowed.
    ShowSettings,
    /// Handled, except that the list of layouts has to be opened — task T-119-3: a left click on
    /// the icon while it is the flag of FR-90б. Outside the borrow: the press reaches into explorer.
    OpenList,
}

// ---------------------------------------------------------------------------------------
// The resumption FR-99 does not allow — task T-13-9, point (б)
// ---------------------------------------------------------------------------------------

/// The journal name of a refused resumption — task T-13-9, one row of `diag`'s closed table.
///
/// **SEC-01, SEC-07.** A literal chosen at compile time, and it names the fact and nothing
/// beside it: a resumption arrived while FR-99 held the program disarmed, and it was refused.
/// Not what `general.enabled` was, not how many panics had run, not which entry of the menu was
/// chosen, and nothing whatever that was typed. [`diag::Operation::from_name`] narrows even
/// this onto the table of `src\diag.rs`, so a name that were not a row of it would keep none of
/// its text.
const RESUME_REFUSED_IN_FAIL_SAFE: &str = "resume refused in fail-safe";

/// Whether «Возобновить» has to be refused right now — **the rule of task T-13-9, point (б)**.
///
/// # What it is about
///
/// FR-99 disarms the program after the fourth consecutive panic inside the hook callback:
/// «весь ввод пропускается без обработки», and the program «сигнализирует состояние иконкой
/// в трее». The flag that says so, `hook::fail_safe`, is raised exactly once and is reset
/// nowhere — the audit of 2026-08-24 confirmed by grep that the codebase holds one write and
/// only reads besides. Until this task nothing stopped the user from then choosing
/// «Возобновить»: [`Tray::toggle_state`] set `general.enabled` back to true, the icon went
/// back to "активна" and `hook::set_active(true)` re-published it — while `hook::classify`
/// went on answering `PASS` to every stroke, hotkey included. The program looked alive and was
/// not, and there was no way back short of restarting the process.
///
/// That breaks the signalling FR-99 names in as many words and the distinguishability FR-90
/// requires of the two icon states, so the resumption is refused and the icon stays where
/// FR-99 put it.
///
/// # Only the one direction
///
/// `fail_safe && !enabled` and not `fail_safe` alone. Suspending stays allowed, and it has to:
/// the transition into the fail-safe state is itself a `toggle_state`, sent by
/// `hook::handle_input_message` on [`crate::hook::WM_APP_FAIL_SAFE`] **after** the flag is
/// already up. A blanket refusal would refuse the very move FR-99 asks for and leave the icon
/// saying "активна" — the same lie, arrived at from the other side.
///
/// # The way back is a restart, and that is the whole of it
///
/// Deliberately no reset of the flag. See the long note at [`Tray::toggle_state`] for why the
/// alternative — clearing `FAIL_SAFE` and the panic streak on an explicit resumption — was
/// weighed and not taken.
///
/// # Why a function of two arguments instead of a read of the atomic
///
/// The same reason `hook::classify` takes a `Mode`, and that module says it outright: the flag
/// has one writer, the fourth consecutive panic inside a callback only the system can call, so
/// a rule that read the atomic inside itself would be a rule no test could ever drive. The two
/// callers — [`Tray::toggle_state`] and [`Menu::build`] — read `hook::fail_safe()` and hand it
/// in; `tests\tray.rs` hands in both values and measures all four rows.
pub fn resume_is_refused(enabled: bool, fail_safe: bool) -> bool {
    fail_safe && !enabled
}

/// Whether an open modal window of this program takes `command` away from the menu — tasks
/// T-13-14 and T-43-4.
///
/// # The finding of task T-13-14
///
/// The audit of 2026-08-24 found the tray fully alive while the modal dialog of FR-92 is on
/// the screen: [`settings::show_dialog`] guards only against a *second copy of the window*, so
/// the icon can still be clicked, the menu still comes up, and «Приостановить» and «Запускать
/// при входе в систему» still change the live configuration and write the file and the
/// registry. The dialog meanwhile edits a **copy** taken when it was opened, and «Применить»
/// hands that copy back through [`apply_settings`], which replaces the configuration whole
/// ([`Tray::replace_config`]). Whatever the tray changed in between is gone — silently, with
/// the file and the `Run` key rewritten from the stale copy.
///
/// Section 6.3 gives the configuration one owner, and two editors of it at one time is what
/// this refuses. The answer is the simple and honest one the task asks for: while the dialog
/// is up, the two entries that edit the configuration are not the user's to choose.
///
/// # The finding of task T-43-4 — the two doors
///
/// The audit of 2026-09-04 (С21) found the other half of the same fact: the menu comes up from
/// under **any** modal window of this program, because a modal loop keeps dispatching to the
/// other windows of the thread. The settings dialog had a latch against a second copy of
/// itself; the «О программе» window of FR-92а had none. So «О программе» opened over the
/// settings dialog and over itself, and three things followed: the windows stacked, and closing
/// the inner one emptied the record the system theme is delivered through, leaving the outer one
/// on yesterday's palette; «ОК» and «Применить» of a settings dialog with «О программе» on top
/// only raised a flag, because its loop lay under a foreign one; and «Применить» with a change of
/// language left the help of the window above it in the old language.
///
/// The answer is the one the audit recommended — both doors shut: while either window is up,
/// «Настройки…» and «О программе» are not the user's to choose, and [`settings::show_about_dialog`]
/// turns round at its top as [`settings::show_dialog`] always has.
///
/// # Which commands, and by which window
///
/// * [`CMD_TOGGLE`] and [`CMD_AUTOSTART`] — by the **settings dialog** only, because those are
///   exactly the two menu commands that write into the configuration it is editing. «О
///   программе» edits nothing, and a suspension made while it is up is lost to nobody;
/// * [`CMD_SETTINGS`] and [`CMD_ABOUT`] — by **either** window: each opens a modal window of
///   its own, and a modal window opened over another is the whole of С21. Before task T-43-4
///   «Настройки…» was left alive here on the ground that [`settings::show_dialog`] answers a
///   second copy with nothing; that is still so, and the entry now says it by being grey;
/// * [`CMD_EXIT`] and every other entry — by **neither**: a program that cannot be closed
///   because a window is open is worse than either finding, and the letters of FR-101 open
///   modeless windows, which run no loop of their own for another window to be caught under.
///
/// # Two halves of one rule, and this is the rule
///
/// [`Menu::build`] appends the locked entries `MF_GRAYED | MF_DISABLED` — that is the half the
/// user sees and the half that keeps `TrackPopupMenuEx` from returning a locked command at all.
/// [`dispatch_command`] asks the same question again before it does anything — that is the
/// half that decides, and every road into a command goes through it: the menu, the double click
/// on the icon (task T-43-4 — until then it called [`open_settings`] past the door) and the
/// buttons of the letters. The greying is the *view* of this rule; the gate in the command
/// handler is the rule. Both read this one function, so there is nothing for the two to
/// disagree about.
///
/// # Why arguments rather than reads inside
///
/// The same shape and the same reason as [`resume_is_refused`] one function up: the decision
/// belongs to one place, the reading of the flags to the caller, and a rule that reached for a
/// thread-local of its own would be a rule the menu tests could not drive — they build menus
/// on a test thread that has no dialog and never will. [`show_menu`] and [`dispatch_command`]
/// read [`settings::dialog_is_open`] and [`settings::about_is_open`] and hand the answers in.
///
/// **SEC-05.** Both flags are thread-locals written only by the guards of the two windows; they
/// are not derived from any message, and no `wParam`, `lParam` or pointer takes part in this
/// decision (R-20). See [`settings::dialog_is_open`] and [`settings::about_is_open`].
pub fn dialog_locks_command(command: u32, dialog_open: bool, about_open: bool) -> bool {
    match command {
        CMD_TOGGLE | CMD_AUTOSTART => dialog_open,
        CMD_SETTINGS | CMD_ABOUT => dialog_open || about_open,
        _ => false,
    }
}

/// The tray of one thread: the icon, its state, and the configuration behind it.
///
/// A type of its own rather than something hidden inside the thread-local, so that the
/// integration tests can drive it directly and so that task T-08-1 has something to attach
/// the settings dialog to.
pub struct Tray {
    /// The window the shell posts the callback message to, and the owner of the menu.
    hwnd: HWND,
    /// Size the icons were loaded at, from `SM_CXSMICON` and `SM_CYSMICON` — FR-90.
    icon_size: (i32, i32),
    /// The "active" icon.
    active: Icon,
    /// The "suspended" icon.
    paused: Icon,
    /// The "active, and a letter is unread" icon — FR-90's third state, task Т-32-4.
    active_unread: Icon,
    /// The "suspended, and a letter is unread" icon.
    paused_unread: Icon,
    /// The same four icons at [`large_icon_size`], for the balloon of [`Tray::announce`] —
    /// задача Т-33а-1. `None` when the system refused to load them, and then the balloon goes
    /// out exactly as it did before (NFR-13): a letter must not be lost over its picture.
    balloon: Option<BalloonIcons>,
    /// The four state icons of the darkbar set at [`small_icon_size`] — task T-86-2, решение
    /// 145.1: what the record carries while the taskbar is dark. `None` when the system refused
    /// to load them, and then the main set serves both taskbars, as it did before this task
    /// (NFR-13): a tray must not fail to stand over its second set.
    darkbar: Option<DarkbarIcons>,
    /// What `SystemUsesLightTheme` said the last time it was asked — at install, and on every
    /// `WM_SETTINGCHANGE` and `WM_THEMECHANGED` since ([`Tray::follow_taskbar_theme`]). The
    /// third answer of [`state_icon_for`].
    taskbar_is_light: bool,
    /// The configuration; `general.enabled` *is* the state of FR-90.
    config: Config,
    /// Where the configuration is written back to. `None` when `%APPDATA%` is not set.
    config_path: Option<PathBuf>,
    /// What [`settings::read_or_default`] said may be done to the file — task T-13-6.
    ///
    /// **The state of the decision, and it lives for the session.** It is set once, from the
    /// outcome of the one read this program performs, and every one of the four ways to
    /// [`Tray::save_config`] — [`Tray::toggle_state`], [`Tray::set_autostart`],
    /// [`Tray::replace_config`] and [`Tray::shut_down`] — goes through it. The tray is the
    /// right place for it because the tray is the only writer: `settings::write_to` has
    /// exactly one call site in the program and it is in this file, and the tray is created
    /// once, on the UI thread, and dropped as the process ends.
    ///
    /// The only transition is [`SavePolicy::QuarantineFirst`] to [`SavePolicy::Allowed`], and
    /// it happens after the damaged file has actually been moved aside.
    /// [`SavePolicy::Forbidden`] never changes: a file from a newer schema is still from a
    /// newer schema at shutdown. Nor does [`SavePolicy::NotRead`] (task T-55-6): the one read of
    /// the session has been made, and a file that could not be read is left alone until the
    /// next start reads it.
    save_policy: SavePolicy,
    /// What `config.toml` is known to hold — task **T-55-4**, finding Н27.
    ///
    /// `Some` only while the file on the disk is known to be exactly this configuration: read
    /// whole at the start ([`settings::ReadOutcome::Current`]) or written by [`Tray::save_config`]
    /// since. `None` for everything else — no file yet, a file that needed a migration, a file
    /// this build could not read, a file from a newer schema — and then the first save writes, as
    /// it always did. [`Tray::save_config`] leaves the file alone while the configuration equals
    /// this.
    on_disk: Option<Config>,
    /// What `RegisterWindowMessageW("TaskbarCreated")` returned — FR-81. Zero means the
    /// registration failed, and zero is also `WM_NULL`, so it must never be compared against.
    taskbar_created: u32,
    /// Whether the icon is believed to be in the notification area right now.
    icon_present: bool,
    /// How many times `NIM_ADD` has been issued. Acceptance point 12 reads this to see that
    /// `TaskbarCreated` really does add the icon a second time.
    add_calls: u32,
    /// How many times `NIM_MODIFY` has been issued by [`Tray::refresh_icon`] — task T-41-6.
    modify_calls: u32,
    /// ⭐ **How many of those the shell refused — the counter finding Н22 asks for, task
    /// T-41-6.**
    ///
    /// Before this task a refused update **cleared [`Tray::icon_present`]**, and updates go out
    /// on pause, on resume, on «Применить» and on every change of language: one refusal and the
    /// icon froze with yesterday's tooltip for the rest of the session. Worse, the same flag
    /// gates [`Tray::announce`], so one refusal also silenced **every letter of FR-101**.
    ///
    /// So the refusal is counted instead of believed. The flag goes on meaning the one thing it
    /// can honestly mean — «удалось ли добавить значок» — and this number is what a refused
    /// update leaves behind, beside its line in the journal.
    modify_failures: u32,
    /// How many times `NIM_DELETE` has been issued — task T-41-6, and what shows that the
    /// delete of FR-83 really goes out even when the flag says there is nothing to delete.
    delete_calls: u32,
    /// How many balloons of FR-101 have been handed to the shell — task T-41-6.
    announce_calls: u32,
    /// Whether [`Tray::shut_down`] has already run. The cleanup of FR-83 is reached from two
    /// directions and has to do its work exactly once.
    finished: bool,
    /// The flag of FR-90б while `general.tray_flag` is on — task T-119-2. `None` with the flag
    /// off: nothing of the flag exists then.
    flag: Option<FlagState>,
    /// The subscription to the list of layouts closing (task T-119-4), held only while the flag is
    /// on — [crate::flags::ListWatch].
    list_watch: Option<crate::flags::ListWatch>,
    /// What this tray last published to [`crate::flags::set_on`] — published only when it changes,
    /// so a tray with the flag off never writes the switch at all (it starts off).
    flag_published: bool,
    /// The last window in front that is not a part of the shell, as the re-reading of the flag saw
    /// it — where the focus goes back after a list opened by a click on the flag (task T-119-3). A
    /// value, compared and handed to `SetForegroundWindow`, never dereferenced; 0 = none yet.
    last_person: usize,
    /// The list of layouts the program opened by a click on the flag, until it is finished — task
    /// T-119-3. `None`: no list of the program's own; a list the person opened is not touched.
    list_session: Option<ListSession>,
}

/// A list of layouts opened by a click on the flag — task T-119-3, решение 208.164.
#[derive(Clone, Copy, Debug)]
struct ListSession {
    /// The window the person typed in before the click — [`Tray::last_person`] at that moment.
    target: usize,
    /// The layout the flag showed at the click.
    before: LayoutId,
    /// The layout read of the shell after the list closed — what the person chose; for the list of
    /// the program's own, the row chosen.
    chosen: LayoutId,
    /// Whether the list is the program's own (РУС/ENG hidden at the clock — part Б): its choice
    /// never reaches the window by itself, the program's switch gives it, a console included.
    own: bool,
    /// The looks of the gift left for a person's window not yet in front — `flags::gift_step`.
    gift_left: u8,
}

/// What the flag of FR-90б shows and the icon drawn for it — task T-119-2.
struct FlagState {
    /// The layout the flag stands for.
    layout: LayoutId,
    /// The face and the look the icon below was drawn for.
    drawn: Option<(crate::flags::Face, crate::flags::Look)>,
    /// The icon itself, destroyed with this value.
    icon: Option<Icon>,
}

impl Tray {
    /// Installs the tray on the window `hwnd`, with the configuration read from the standard
    /// path of section 7.
    pub fn install(hwnd: HWND, instance: HINSTANCE) -> WinResult<Self> {
        Self::install_at(hwnd, instance, settings::default_config_path())
    }

    /// Installs the tray with the configuration read from `config_path`.
    ///
    /// Separate from [`Tray::install`] so that a test can point the tray at a file of its own
    /// instead of at the configuration of whoever is running the test suite.
    pub fn install_at(
        hwnd: HWND,
        instance: HINSTANCE,
        config_path: Option<PathBuf>,
    ) -> WinResult<Self> {
        let icon_size = small_icon_size(Some(hwnd));
        let active = Icon::load(instance, IDI_APP_ACTIVE, icon_size)?;
        let paused = Icon::load(instance, IDI_APP_PAUSED, icon_size)?;
        // FR-90's third state, task Т-32-4 — loaded here with the other two so that the dot
        // appears the moment a letter arrives rather than after a load that could fail then.
        let active_unread = Icon::load(instance, IDI_APP_ACTIVE_UNREAD, icon_size)?;
        let paused_unread = Icon::load(instance, IDI_APP_PAUSED_UNREAD, icon_size)?;

        // Задача Т-33а-1: те же четыре значка ещё раз, крупным кадром — для шара уведомления.
        // ⚠ Без `?`: значок уведомления — украшение письма, а письмо — нет. Отказ загрузки
        // стоит одной записи в журнале и возвращает шару прежний вид (NFR-13).
        let balloon = BalloonIcons::load(Some(hwnd), instance);

        // Задача T-86-2, решение 145.1: набор без плиты — те же четыре состояния для тёмной
        // панели, тем же малым размером. ⚠ Без `?`, как и набор шара: трей, не вставший из-за
        // второго набора, хуже трея с плитой на тёмной панели (NFR-13). Переключатель панели
        // читается здесь один раз; дальше его перечитывают ветки сообщений темы.
        let darkbar = DarkbarIcons::load(instance, icon_size);
        let taskbar_is_light = theme::taskbar_is_light();

        let (config, save_policy, on_disk) = match config_path.as_deref() {
            Some(path) => {
                // Task T-55-5, finding Т8: the temporaries an interrupted write left beside the
                // file go first — before this process has written anything, so none is its own.
                if settings::remove_abandoned_temporaries(path).refused > 0 {
                    note_configuration(CONFIG_TEMPORARY_NOT_REMOVED);
                }

                let (config, outcome) = settings::read_or_default(path);

                // Решение 120.2, task T-55-7: one line of the journal for every number of the file
                // that was read as its default — taken right after the read, on the thread that
                // read.
                for _ in 0..settings::take_softened_fields() {
                    note_configuration(CONFIG_FIELD_DEFAULTED);
                }

                // **This is the place `read_or_default` returns a pair for** — task T-13-6.
                // The configuration half is usable whatever happened, which is what that
                // function exists for; the other half is the fate of the *file*, and it is a
                // decision this tray has to take, because this tray is the only thing in the
                // program that ever writes that file back. Dropping it here was the defect:
                // `save_config` below was unconditional then and FR-83 reached it on every exit,
                // so a file this build could not read was overwritten with defaults at the first
                // toggle or the first shutdown.
                let policy = SavePolicy::for_read(&outcome);

                // SEC-01, SEC-07: the journal is told *that* the file could not be read or
                // came from a newer build, and never a byte of what was in it — no line, no
                // fragment, not the name of a field this build has no name for. An ordinary
                // read says nothing at all: an event per healthy start-up is noise, and the
                // two entries below are only worth having because they are rare.
                match policy {
                    SavePolicy::Allowed => {}
                    SavePolicy::QuarantineFirst => note_configuration(CONFIG_UNREADABLE),
                    SavePolicy::Forbidden => note_configuration(CONFIG_NEWER_SCHEMA),
                    SavePolicy::NotRead => note_configuration(CONFIG_NOT_READ),
                }

                // Task T-55-4: only a file read whole is known to hold this very configuration.
                let on_disk =
                    matches!(outcome, Ok(settings::ReadOutcome::Current)).then(|| config.clone());

                (config, policy, on_disk)
            }
            // `%APPDATA%` is not set. A resident utility still has to run, and the defaults
            // of section 7 are a complete configuration, so this is not a reason to refuse.
            // Nothing was read and nothing will be written — `save_config` leaves on the same
            // `None` — so the policy is the one that changes nothing. Task T-55-9, finding Н67:
            // nothing was read, which is exactly a first run, so the language is the system's.
            None => (Config::for_a_first_run(), SavePolicy::Allowed, None),
        };

        // SAFETY: `TASKBAR_CREATED` is a NUL-terminated `'static` UTF-16 literal, so the
        // pointer the call reads through outlives the call by definition. The call has no
        // effect beyond returning the atom of a globally registered message name, and
        // registering the same name twice returns the same value rather than failing —
        // which is why FR-81's "once, when the window is created" is satisfied here.
        let taskbar_created = unsafe { RegisterWindowMessageW(TASKBAR_CREATED) };

        if taskbar_created == 0 {
            // NFR-13: the result is examined. Not fatal — the icon still works, it just will
            // not come back on its own if the shell restarts.
            app::report_non_critical("RegisterWindowMessageW", &WinError::from_thread());
        }

        let mut tray = Self {
            hwnd,
            icon_size,
            active,
            paused,
            active_unread,
            paused_unread,
            balloon,
            darkbar,
            taskbar_is_light,
            config,
            config_path,
            save_policy,
            on_disk,
            taskbar_created,
            icon_present: false,
            add_calls: 0,
            modify_calls: 0,
            modify_failures: 0,
            delete_calls: 0,
            announce_calls: 0,
            finished: false,
            flag: None,
            list_watch: None,
            flag_published: false,
            last_person: 0,
            list_session: None,
        };

        tray.add_icon();

        // FR-90б: a flag that was on when the program stopped is the icon from the start.
        tray.sync_flag_watch();
        if tray.flag_on() {
            tray.refresh_flag();
        }

        Ok(tray)
    }

    /// Whether the icon is the flag of the layout — `general.tray_flag`, FR-90б, task T-119-2.
    pub fn flag_on(&self) -> bool {
        self.config.general.tray_flag
    }

    /// Publishes the setting to the threads that tell the flag of a change and holds the
    /// subscription to the list of layouts closing exactly while the flag is on — task T-119-4.
    /// Off: the atomic says no and the subscription is gone (no new hook, no new thread).
    pub fn sync_flag_watch(&mut self) {
        let on = self.flag_on();

        if on != self.flag_published {
            crate::flags::set_on(on);
            self.flag_published = on;
        }

        if !on {
            self.list_watch = None;
        } else if self.list_watch.is_none() {
            self.list_watch = crate::flags::ListWatch::start();
        }
    }

    /// Whether the subscription to the list of layouts is held — for `tests\tray.rs`.
    pub fn watches_the_list(&self) -> bool {
        self.list_watch.is_some()
    }

    /// The layout the flag shows, while it is on — for `tests\tray.rs` and the tooltip.
    pub fn flag_layout(&self) -> Option<LayoutId> {
        self.flag.as_ref().map(|flag| flag.layout)
    }

    /// The handle the record carries now: the flag while it is on and drawn, the icon of FR-90
    /// otherwise — for `tests\tray.rs`.
    pub fn shown_icon(&self) -> HICON {
        self.flag_icon().unwrap_or_else(|| self.state_icon())
    }

    /// Re-reads the layout of the window the person types into (FR-52) and, if the flag shows
    /// another one, hands the shell the new flag — FR-90б. With the flag off: drops whatever the
    /// flag held and does nothing else, which is the guard of «флаг выключен — нового кода нет».
    ///
    /// A reading that cannot see (a classic console, FR-52's `blind`) or sees no window keeps the
    /// layout last known: the flag does not go blank between two windows.
    pub fn refresh_flag(&mut self) {
        let reading = crate::switch::read();

        // Task T-119-3: the window the focus goes back to after a list opened by a click on the
        // flag — the last one in front that is not a part of the shell. The click itself puts the
        // taskbar in front, and its re-reading must not take the person's window's place.
        if !reading.window.is_none() && !is_shell_window(reading.window.raw()) {
            self.last_person = reading.window.raw();
        }

        let layout = if reading.blind {
            // A classic console (FR-52): the layout the watcher found by the name of the input
            // indicator, when it has just found one (task T-119-4), else the layout last known.
            crate::flags::take_console_layout()
        } else {
            (reading.layout.raw() != 0).then_some(reading.layout)
        };

        self.show_flag_of(layout);
    }

    /// [`Tray::refresh_flag`] with the reading handed in — the seam `tests\tray.rs` drives.
    pub fn show_flag_of(&mut self, layout: Option<LayoutId>) {
        if !self.flag_on() {
            if self.flag.take().is_some() {
                self.refresh_icon();
            }
            return;
        }

        let known = layout.or_else(|| self.flag_layout());

        let Some(layout) = known else {
            return;
        };

        if self.flag_layout() == Some(layout)
            && self.flag.as_ref().is_some_and(|flag| flag.icon.is_some())
        {
            return;
        }

        match self.flag.as_mut() {
            Some(flag) => flag.layout = layout,
            None => {
                self.flag = Some(FlagState {
                    layout,
                    drawn: None,
                    icon: None,
                });
            }
        }

        self.refresh_icon();
    }

    /// The flag icon of the moment, drawn anew only when what it shows changed — the layout's
    /// face, the size, the pause, the dot, the taskbar. `None` with the flag off or refused by
    /// the system (NFR-13: the record then carries the icon of FR-90).
    fn prepare_flag(&mut self) {
        if !self.flag_on() {
            self.flag = None;
            return;
        }

        let look = crate::flags::Look {
            size: u32::try_from(self.icon_size.0).unwrap_or(16),
            paused: !self.enabled(),
            unread: self.has_unread(),
            dark_bar: !self.taskbar_is_light,
        };

        let Some(flag) = self.flag.as_mut() else {
            return;
        };

        let face = crate::flags::face_of(flag.layout);
        let wanted = (face, look);

        if flag.drawn.as_ref() == Some(&wanted) && flag.icon.is_some() {
            return;
        }

        flag.icon = crate::flags::icon(&wanted.0, look).map(|handle| Icon { handle });

        if flag.icon.is_none() {
            app::report_non_critical("CreateIconIndirect(flag)", &WinError::from_thread());
        }

        flag.drawn = Some(wanted);
    }

    /// The handle of the drawn flag, if the flag is on and drawn.
    fn flag_icon(&self) -> Option<HICON> {
        if !self.flag_on() {
            return None;
        }

        self.flag.as_ref()?.icon.as_ref().map(|icon| icon.handle)
    }

    /// Whether correction is armed — `general.enabled`, the state of FR-90.
    pub fn enabled(&self) -> bool {
        self.config.general.enabled
    }

    /// Whether `general.autostart` is set. The check mark of FR-91 shows this.
    ///
    /// What the *system* will actually do is [`settings::autostart_registered`], and the two are
    /// kept equal from both ends — FR-93: by [`Tray::set_autostart`] and «Применить» when a person
    /// changes the setting, and, since task T-55-1, by every start of the program, which makes the
    /// `Run` key agree with this value ([`attach_at_via`], решение 120.4 (б)). They are read
    /// separately on purpose: the settings dialog shows both, so that a value somebody removed
    /// from the registry by hand is visible rather than merely wrong.
    pub fn autostart(&self) -> bool {
        self.config.general.autostart
    }

    /// What the one read of the configuration said may be done to the file — the decision of task
    /// T-13-6, read out — task T-55-2.
    ///
    /// «Применить» and the check mark of FR-91 ask it before they touch `HKCU\…\Run`: the one rule
    /// of [`SavePolicy::lets_the_run_key_follow`] is a question about this value.
    pub fn save_policy(&self) -> SavePolicy {
        self.save_policy
    }

    /// The configuration this tray holds.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The size the icons were loaded at — `(SM_CXSMICON, SM_CYSMICON)`.
    pub fn icon_size(&self) -> (i32, i32) {
        self.icon_size
    }

    /// What the taskbar switch said the last time the tray asked — FR-90, task T-86-2.
    pub fn taskbar_is_light(&self) -> bool {
        self.taskbar_is_light
    }

    /// The icon the shell is handed in the record — `hIcon` of [`Tray::notify_data`], the very
    /// handle `NIM_ADD` and `NIM_MODIFY` carry. Task T-86-2: what a test compares, pixel for
    /// pixel, with the frame the resources hold.
    pub fn record_icon(&self) -> HICON {
        self.notify_data().hIcon
    }

    /// Whether the icon is believed to be in the notification area right now.
    ///
    /// "Believed": the only source is the return value of the last `Shell_NotifyIcon`, and
    /// the shell can take an icon away without telling anybody — which is FR-81, and is why
    /// this is a hint for the tests and for T-08-1 rather than a fact to branch the product
    /// on.
    pub fn icon_present(&self) -> bool {
        self.icon_present
    }

    /// How many times `NIM_ADD` has been issued since installation.
    ///
    /// Exists for FR-81: the only observable difference between "the message was handled"
    /// and "the message was swallowed" is that the icon gets added again.
    pub fn add_calls(&self) -> u32 {
        self.add_calls
    }

    /// How many `NIM_MODIFY` updates [`Tray::refresh_icon`] has issued — task T-41-6.
    ///
    /// The number the acceptance of finding Н22 reads: a refused update must not stop the
    /// **next** one going out, and the only observable difference between «обновления идут» and
    /// «значок замер» is this count moving.
    pub fn modify_calls(&self) -> u32 {
        self.modify_calls
    }

    /// How many of those the shell refused — see [`Tray::modify_failures`], task T-41-6.
    pub fn modify_failures(&self) -> u32 {
        self.modify_failures
    }

    /// How many `NIM_DELETE` removals have been issued — task T-41-6.
    ///
    /// FR-83 says the icon comes off at exit. Before this task the removal was skipped whenever
    /// the flag said the icon was not there — and the flag could say so while the shell still
    /// held an icon, which is how a dead icon was left behind in the notification area.
    pub fn delete_calls(&self) -> u32 {
        self.delete_calls
    }

    /// How many balloons of FR-101 have been handed to the shell — task T-41-6.
    pub fn announce_calls(&self) -> u32 {
        self.announce_calls
    }

    /// The message the shell posts on behalf of the icon.
    pub fn callback_message(&self) -> u32 {
        WM_APP_TRAY
    }

    /// The registered `TaskbarCreated` message — FR-81. Zero if registration failed.
    pub fn taskbar_created_message(&self) -> u32 {
        self.taskbar_created
    }

    /// Looks at one window message — the single place that decides what the tray reacts to.
    ///
    /// SEC-05: the list below is exhaustive and everything outside it is
    /// [`Reaction::Ignored`]. Nothing in it is privileged — the two entries that change
    /// anything, the state of FR-90 and the exit, are reachable only through the menu, and
    /// the menu is not a message. The
    /// theme pair of task T-11-9 is the last sentence of SEC-05 verbatim: the reaction is
    /// bounded to re-reading the system setting and repainting our own open dialog, and
    /// the message's content is not used past the comparison of one string.
    pub fn handle_message(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> Reaction {
        // FR-81. Tested before the `match` because the value is not a compile-time constant:
        // it is whatever `RegisterWindowMessageW` returned. Zero is excluded, or a failed
        // registration would turn every `WM_NULL` into a re-add.
        if self.taskbar_created != 0 && message == self.taskbar_created {
            self.readd_icon();
            return Reaction::Handled(LRESULT(0));
        }

        match message {
            // The callback of the icon, in the `NOTIFYICON_VERSION_4` convention: the event
            // is in the *low word of `lParam`* and the cursor position is in `wParam`. That
            // is the opposite of the old convention, where the event was the whole of
            // `lParam` and `wParam` held the identifier of the icon.
            WM_APP_TRAY => self.handle_icon_notification(wparam, lparam),

            // FR-83. `TRUE` is not a choice: a program has no business holding up the end of
            // a session, and refusing would only put a dialog in the user's way. Nothing is
            // cleaned up here — the session may still be cancelled, in which case
            // `WM_ENDSESSION` arrives with `FALSE` and the program carries on.
            WM_QUERYENDSESSION => Reaction::Handled(LRESULT(1)),

            // FR-83. `wParam` is `TRUE` only if the session really is ending. The cleanup
            // runs here and now, synchronously, because the system may terminate the process
            // shortly after this returns; it is the same function the ordinary exit calls,
            // so there is one cleanup path and not two.
            WM_ENDSESSION => {
                if wparam.0 != 0 {
                    self.shut_down();
                    app::request_shutdown();
                }

                Reaction::Handled(LRESULT(0))
            }

            // **FR-101, task Т-32-4 — the one clock the letters run on.** The first tick comes
            // ninety seconds after the start (NFR-08: nothing of this may be in the way of a
            // program starting), and every one after it an hour later (NFR-10: a program at
            // rest does nothing, and once an hour is as close to nothing as a schedule counted
            // in days can be).
            //
            // The reaction is `Ignored` so that the answer is worked out **outside** the borrow
            // of the tray: showing a letter reads the configuration through `with_tray`, and a
            // second borrow would panic. `handle_ui_message` is what runs it.
            //
            // SEC-05: a forged `WM_TIMER` buys the sender one run of a schedule that reads its
            // own state and a clock, and at worst one letter shown a few hours early.
            WM_TIMER if wparam.0 == LETTERS_TIMER => Reaction::Ignored,

            // **FR-90, task T-86-2, решение 145.2: the icon follows the taskbar.** The taskbar
            // switch is re-read and, if it moved, the record gets the icon of the other set. Nothing
            // of the message is read: a forged broadcast buys one reading of the switch and, while
            // the answer stands, no `NIM_MODIFY`. The windows of the shell — the settings and
            // «О программе» among them since stage Э120 — follow the system theme on their own,
            // so nothing is handed on from here any more.
            WM_SETTINGCHANGE | WM_THEMECHANGED => {
                self.follow_taskbar_theme();

                Reaction::Ignored
            }

            _ => Reaction::Ignored,
        }
    }

    /// The `WM_APP_TRAY` half of [`Tray::handle_message`].
    fn handle_icon_notification(&self, wparam: WPARAM, lparam: LPARAM) -> Reaction {
        match u32::from(low_word(unsigned(lparam.0))) {
            // A right click, the keyboard menu key, or Shift+F10 on the icon. FR-91 names
            // this one.
            //
            // A left click (`NIN_SELECT`) and Enter or Space (`NIN_KEYSELECT`) are answered
            // with the same menu. FR-91 is silent about them and the choice is argued in the
            // report of T-01-4; the short of it is that the menu is the entire interface of
            // the program at this stage, that showing it is idempotent and reversible, and
            // that toggling the state on a stray click would disarm a resident utility
            // without telling anybody.
            // ⭐ **Task T-41-3, finding Н44: the point is not believed.** Until this task the two
            // words of `wParam` went straight to `TrackPopupMenuEx`, and the sender was not
            // asked about. `WM_APP_TRAY` is posted like any other message, so any process at this
            // integrity level could name any coordinates at all — and the menu unrolled in the
            // middle of the screen, or past its edge, while the program pulled itself to the
            // front to do it. It is [`menu_point`] that takes the point back onto the desktop;
            // whose point it is at all is settled a moment later, in [`show_menu`].
            //
            // ⭐ **Task T-119-3, решение 208.88: the flag answers a left click as РУС/ENG does** —
            // with the list of layouts of Windows. Only the mouse's `NIN_SELECT` and only while the
            // icon is the flag; Enter and Space (`NIN_KEYSELECT`), the right click, the menu key and
            // Shift+F10 keep the menu, and so does the left click with the flag off (1.6.1).
            NIN_SELECT if self.flag_on() => Reaction::OpenList,
            notification @ (WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT) => {
                let (x, y) = menu_point((
                    coordinate(low_word(wparam.0)),
                    coordinate(high_word(wparam.0)),
                ));

                Reaction::ShowMenu {
                    x,
                    y,
                    from_the_keyboard: notification == NIN_KEYSELECT,
                }
            }

            // A double click opens the settings dialog of FR-92, which is the convention every
            // resident program on this system follows. With the mouse this arm is not usually
            // reached — the first click of the pair has already opened the menu and the second
            // is consumed by the menu's own modal loop — so «Настройки…» in the menu stays the
            // documented way in, and this is the shortcut for whoever expects it to work.
            WM_LBUTTONDBLCLK => Reaction::ShowSettings,

            // **FR-101, task Т-32-4 — the balloon was clicked.** The person is asking for the
            // letter the balloon announced, so the window comes up **with** the focus: this is
            // the case FR-101 excepts from «фокус не отбирают», because the focus was given
            // rather than taken.
            //
            // `Ignored` so that the work happens outside the borrow of the tray — showing a
            // letter reads the configuration through `with_tray` of its own.
            //
            // SEC-05: the event number is the whole of what is read out of the message, and a
            // forged one buys the sender one window of ours, holding a letter this program
            // itself composed.
            NIN_BALLOONUSERCLICK => Reaction::Ignored,

            // Every other notification of the icon: the rest of the balloon events, the hover
            // notifications of version 4, the raw button messages that accompany the ones
            // above. A `WM_APP` message has no default handling, so answering zero is the
            // whole of "ignore it".
            _ => Reaction::Handled(LRESULT(0)),
        }
    }

    /// Flips between "active" and "suspended" and writes the change to the configuration.
    ///
    /// The state of FR-90 is stored in `general.enabled`, so switching it and saving it are
    /// the same operation. There is no hook to arm or disarm yet — task T-03-1 reads this
    /// flag; until then the state is the icon, the tooltip and the file.
    ///
    /// # The one move this refuses — task T-13-9, point (б)
    ///
    /// **While FR-99 holds, the resumption is refused and nothing at all happens here**: no
    /// change in memory, no icon, no tooltip, no save. One event goes into the ring journal —
    /// [`RESUME_REFUSED_IN_FAIL_SAFE`], the fact and no value beside it — so that a refusal is
    /// visible in a dump instead of being a silence. The rule and the reasoning are at
    /// [`resume_is_refused`]; this is where it is enforced, because this method is the only
    /// way `general.enabled` ever moves — «Возобновить» in the menu reaches it through
    /// [`dispatch_command`], and nothing else in the program writes that field.
    ///
    /// The greying of the menu entry ([`Menu::build`]) is a second line and not this one: it
    /// stops the command being asked for, which is what makes the refusal visible instead of
    /// merely silent, but the menu is user interface and this is the rule.
    ///
    /// **Восстановление — только перезапуском процесса.** That is the accepted price and the
    /// alternative was weighed: resetting `hook::fail_safe` and the panic streak on an explicit
    /// resumption would give a way back inside the session, and it would also re-arm the
    /// callback that has just panicked four times in a row against unchanged causes — the
    /// stroke that panics, the layout that panics, the buffer state that panics. FR-99 counts
    /// «подряд», and the counter is put back to zero by every callback that returns normally,
    /// so a program resumed into the same cause spends four more panics arriving at the same
    /// place, and a user who keeps pressing «Возобновить» is a loop of exactly that. The panics
    /// are not free: each one unwinds through the most timing-critical function in the program
    /// (NFR-01: p99 under 100 µs), and FR-80 removes a hook that is too slow **silently**. A
    /// state that is honest and needs a restart is worth more than one that is reversible and
    /// oscillates. The choice is written up in the report of this task.
    ///
    /// # How this sits with the save policy of task T-13-6
    ///
    /// They never meet, and that is by construction rather than by luck. This refusal returns
    /// **before** anything is changed, so [`Tray::save_config`] is not reached at all: a
    /// refused resumption asks nothing of [`Tray::save_policy`], attempts no quarantine and
    /// puts no «configuration save suppressed» in the journal. The two also answer different
    /// questions and neither can shadow the other — T-13-6 decides the fate of the **file**
    /// (and may forbid writing it for the session, [`SavePolicy::Forbidden`]), while this
    /// decides whether the **state** moves at all. In the direction that is still allowed —
    /// the suspension FR-99 itself performs — the icon changes and `save_config` then decides
    /// the file's fate on its own terms, so a configuration from a newer schema is still left
    /// alone and the icon still tells the truth. Neither outcome depends on the other.
    ///
    /// # The three refusals on this road, in order — task T-13-14
    ///
    /// Since this task there are **three** places on the way from a click on «Приостановить» to
    /// a byte on the disk where the program can say no, and they stand one inside another,
    /// widest question first:
    ///
    /// 1. [`dispatch_command`] — [`dialog_locks_command`]: **may this editor edit at all?**
    ///    Asked before the tray is so much as borrowed, because it is about *who* is holding
    ///    the configuration and not about what is being asked of it (section 6.3). It is the
    ///    only one of the three that can refuse a move that is in every other way legal, so it
    ///    has to be outermost: put it any deeper and a second editor would already have started
    ///    work.
    /// 2. This method — [`resume_is_refused`]: **may the state move?** A property of the
    ///    transition and of FR-99, asked once the caller has been admitted. It cannot be moved
    ///    outward, because the same road is walked by `hook::handle_input_message` on
    ///    [`crate::hook::WM_APP_FAIL_SAFE`], which never passes through `dispatch_command` at
    ///    all — a gate that lived only in the menu handler would leave that road open.
    /// 3. [`Tray::save_config`] — [`Tray::save_policy`] of task T-13-6: **may the file be
    ///    written?** Innermost, because it is only reached when the state really has moved, and
    ///    because it must also stand in front of the three other savers
    ///    ([`Tray::set_autostart`], [`Tray::replace_config`], [`Tray::shut_down`]) that this
    ///    method knows nothing about.
    ///
    /// None of the three can shadow another: 1 and 2 return before anything changes, and 3
    /// decides only the fate of the file, after memory and the icon are already right. Each is
    /// asked of a different fact — a thread-local of the UI thread, an atomic FR-99 raises, and
    /// the outcome of the one read at start-up — and each is enforced at the one place that all
    /// of its roads pass through.
    pub fn toggle_state(&mut self) {
        if resume_is_refused(self.enabled(), crate::hook::fail_safe()) {
            // One event, and it is worth having precisely because it should be rare: the entry
            // that asks for this is appended `MF_GRAYED | MF_DISABLED` while FR-99 holds, so
            // `TrackPopupMenuEx` cannot return `CMD_TOGGLE` and the ordinary road here is shut.
            // A refusal reaching this line therefore says the resumption arrived some other
            // way, and a dump that shows it is what tells a reader the state was honest rather
            // than stuck. SEC-01 and SEC-07 are argued at the name; the code is
            // [`diag::OsCode::NONE`] because this is an event of the program and not the
            // failure of a Win32 call.
            diag::record(
                diag::Operation::from_name(RESUME_REFUSED_IN_FAIL_SAFE),
                diag::OsCode::NONE,
            );

            return;
        }

        self.config.general.enabled = !self.config.general.enabled;

        // ⭐ **Row 9 of the FR-10 table, task Т-13-7** — «Приостановка программы пользователем …
        // полный сброс + обнуление памяти». Before this task the row had no implementation: this
        // method changed the icon and the configuration and nothing else, `hook::set_active` was
        // one atomic store, and the audit of 2026-08-24 found the whole row wired to nothing.
        //
        // ⚠ **Honestly: this is a belt beside a brace, not the only thing holding the trousers
        // up.** Every real road to the menu entry runs over the ring on its way here — a click on
        // the tray icon is a mouse button (Raw Input → `watchdog::apply_flush`), and the keyboard
        // road is `Win+B`, arrows and `Enter`, every one of them a flush key of the LL-hook rows
        // — so in practice the buffer is already empty a moment before the state moves, and while
        // the program is suspended `hook::classify` answers `PASS` before `buffer::record` is
        // reached, so nothing is written into it either. The verdict of the audit finding says as
        // much and narrows its own consequence for this row. It is wired all the same, because
        // FR-10 states the rule directly and a requirement met only as a side effect of somebody
        // else's mechanism is a requirement that is one refactor away from not being met at all.
        // For row 8 — the session lock — the consequence was **not** covered by anything, and
        // there this is the repair rather than the insurance; see `watchdog::WIPING_SESSION_EVENTS`.
        //
        // **Only the "off" edge.** Resuming is not a flush row: FR-10 names «приостановка» and
        // not its undoing, and the ring the pause emptied is still empty. This is deliberately
        // the mirror of `hook::set_active`, which asks for a fresh `CapsLock` on the `false →
        // true` edge alone (task T-13-4): that one exists because a press may have been *missed*
        // during the pause, this one because a word may still be *held* going into it. They act
        // on opposite edges, they touch different state, and neither can undo the other.
        //
        // This is the UI thread and the ring is the input thread's (section 6.3), so what happens
        // here is one relaxed increment and one `PostMessageW` — see `watchdog::request_wipe`.
        // Placed before `save_config`, which writes a file: the wipe should not queue behind
        // disk I/O.
        if !self.config.general.enabled {
            crate::watchdog::request_wipe();
        }

        self.refresh_icon();
        self.save_config();
    }

    /// Records `general.autostart` and saves it — the file half of FR-93.
    ///
    /// The registry half is [`settings::set_autostart`] and is made to succeed *before* this is
    /// called: a file that claims the program starts with the session while the registry says
    /// otherwise is the one outcome worth avoiding, and the order is what avoids it.
    pub fn set_autostart(&mut self, autostart: bool) {
        self.config.general.autostart = autostart;
        self.save_config();
    }

    /// Takes the configuration the settings dialog of FR-92 produced.
    ///
    /// The whole structure is replaced rather than patched field by field, because the dialog
    /// starts from a copy of this very configuration and returns it with the sections of FR-92
    /// changed and everything else — `general.enabled`, `[buffer] capacity`, `schema_version` —
    /// exactly as it found them.
    ///
    /// Saving is not optional here: the tray is the in-memory owner of the configuration, and a
    /// change kept only in memory would reach the disk at shutdown (FR-83) or never. Since task
    /// T-55-4 a configuration equal to what the file holds is not written again — see
    /// [`Tray::save_config`].
    /// Whether the configuration in force is the one on the disk — false after a write that
    /// failed or was held back. Without `%APPDATA%` there is no file to agree with, and nothing is
    /// lost by that.
    pub fn config_saved(&self) -> bool {
        self.config_path.is_none() || self.on_disk.as_ref() == Some(&self.config)
    }

    pub fn replace_config(&mut self, config: Config) {
        self.config = config;
        // FR-90б, task T-119-2: the check box of the flag acts at once — a flag put on reads the
        // layout now, a flag taken off gives the icon of FR-90 back in the same update.
        self.sync_flag_watch();
        if self.flag_on() {
            self.refresh_flag();
        }
        self.refresh_icon();
        self.save_config();
    }

    /// Removes the icon and saves the configuration — the cleanup of FR-83.
    ///
    /// **This is the one cleanup path of the program.** Every way out reaches it:
    ///
    /// * `Выход` in the menu asks [`crate::app::request_shutdown`]; the UI thread leaves its
    ///   message loop, `serve_window` drops the [`Attachment`], and [`Drop`] calls this;
    /// * the FR-97 debug timeout requests the same shutdown and arrives the same way;
    /// * `WM_ENDSESSION` calls this directly, because the system may not leave the process
    ///   enough time to unwind, and then asks for the same shutdown so that the unwinding
    ///   happens anyway. The second visit finds `finished` set and does nothing.
    ///
    /// Idempotent by construction, which is what lets the two directions share it.
    pub fn shut_down(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;

        // Task T-119-4: the flag's subscription and switch leave with the tray.
        self.list_watch = None;
        if self.flag_published {
            crate::flags::set_on(false);
            self.flag_published = false;
        }

        // FR-83, "удаление иконки".
        self.remove_icon();

        // FR-83, "сохранение конфигурации".
        self.save_config();

        // FR-83, "снятие хуков" — already done, and deliberately not from here: task T-03-1
        // put the `UnhookWindowsHookEx` of `WM_ENDSESSION` into `app::window_proc`, which sees
        // the message before this cleanup is reached, because `src\tray.rs` was outside that
        // task's file scope. The code there is correct; only this note was stale.
        //
        // FR-83, "обнуление буфера" — already done, and deliberately not from here. The typing
        // buffer is a thread-local of the **input** thread (section 6.3), which this thread
        // cannot reach at all: `buffer::with` on the UI thread finds nothing. It is zeroed by
        // `buffer::reset` — the ring is overwritten, SEC-02 — on every flush of FR-10, by
        // `app::park_buffer` when the focus enters a password field (FR-70), and by the drop of
        // the recorder when the input thread leaves `serve_window`. Task T-03-2 put it there and
        // this note was the stale half of that move (documentation debt, section 4.3 of
        // STATE.md, paid by task T-08-1).
        //
        // FR-83, "освобождение мьютекса" — already done, and deliberately not from here: the
        // mutex is owned by `app::SingleInstance` and released by its `Drop` in
        // `run_as_first_instance`, after every thread has been joined. That ordering matters
        // and must not be moved here — releasing the name while a window of this instance is
        // still alive would let a second instance start on top of the first.
    }

    // -----------------------------------------------------------------------------------
    // The icon — FR-90
    // -----------------------------------------------------------------------------------

    /// The icon that matches the current state.
    ///
    /// FR-90's third state, task Т-32-4: the dot of FR-101 rides on **both** of the two states
    /// this program has always had, because they are independent — a person can pause the
    /// program with a letter unread. Task T-86-2 added the third question — the theme of the
    /// taskbar — and moved the whole rule into [`state_icon_for`], where a table can close it;
    /// this method only asks the three questions of itself and turns the answer into a handle.
    fn state_icon(&self) -> HICON {
        self.icon_of(state_icon_for(
            self.enabled(),
            self.has_unread(),
            self.taskbar_is_light,
        ))
    }

    /// The handle of one of the eight icons — the exhaustive twin of [`state_icon_for`].
    ///
    /// NFR-13: a darkbar icon whose set never loaded is answered with the main icon of the same
    /// state, so a dark taskbar shows the plate rather than nothing — the state the program was
    /// in before task T-86-2.
    fn icon_of(&self, which: StateIcon) -> HICON {
        match which {
            StateIcon::Active => self.active.handle,
            StateIcon::Paused => self.paused.handle,
            StateIcon::ActiveUnread => self.active_unread.handle,
            StateIcon::PausedUnread => self.paused_unread.handle,
            StateIcon::DarkbarActive => self
                .darkbar
                .as_ref()
                .map_or(self.active.handle, |set| set.active.handle),
            StateIcon::DarkbarPaused => self
                .darkbar
                .as_ref()
                .map_or(self.paused.handle, |set| set.paused.handle),
            StateIcon::DarkbarActiveUnread => self
                .darkbar
                .as_ref()
                .map_or(self.active_unread.handle, |set| set.active_unread.handle),
            StateIcon::DarkbarPausedUnread => self
                .darkbar
                .as_ref()
                .map_or(self.paused_unread.handle, |set| set.paused_unread.handle),
        }
    }

    /// Re-reads the taskbar switch and, if it moved, hands the shell the icon of the other set —
    /// FR-90, task T-86-2, решение 145.2. The `WM_SETTINGCHANGE` and `WM_THEMECHANGED` arms of
    /// [`Tray::handle_message`] call this **before** the gate of task T-13-20: that gate is about
    /// repainting the windows of FR-92а, and the icon has to follow the taskbar with every window
    /// closed.
    ///
    /// SEC-05: nothing of the message is read. A forged broadcast buys the sender one reading of
    /// the personalization switch and, while the answer stands, no `NIM_MODIFY` at all.
    fn follow_taskbar_theme(&mut self) {
        self.follow_taskbar_theme_with(theme::taskbar_is_light());
    }

    /// The same, with the answer handed in — the seam `tests\tray.rs` drives both ways without
    /// touching the registry, which belongs to whoever runs the tests.
    ///
    /// The same answer twice is nothing: no field moves and no update goes out. A changed answer
    /// is one `NIM_MODIFY` through [`Tray::refresh_icon`], which takes its icon from
    /// [`Tray::state_icon`] — the three questions asked afresh.
    pub fn follow_taskbar_theme_with(&mut self, taskbar_is_light: bool) {
        if self.taskbar_is_light == taskbar_is_light {
            return;
        }

        self.taskbar_is_light = taskbar_is_light;
        self.refresh_icon();
    }

    /// The same choice as [`Tray::state_icon`], but out of the large set — задача Т-33а-1.
    ///
    /// `None` means the large icons never loaded, and the caller then leaves the balloon as it
    /// was before this task (NFR-13).
    fn balloon_icon(&self) -> Option<HICON> {
        let large = self.balloon.as_ref()?;

        Some(match (self.enabled(), self.has_unread()) {
            (true, false) => large.active.handle,
            (false, false) => large.paused.handle,
            (true, true) => large.active_unread.handle,
            (false, true) => large.paused_unread.handle,
        })
    }

    /// Whether a news item of the feed is unread — the fact the dot of FR-90 shows and the
    /// entry «Непрочитанное письмо…» of FR-91 stands on.
    ///
    /// Read out of **this value's own** configuration and the feed the letters module holds,
    /// so that nothing here reaches for `with_tray` — the tray is already borrowed whenever
    /// this is called, and a second borrow would panic.
    fn has_unread(&self) -> bool {
        crate::letters::with_feed(|feed| {
            crate::letters::has_unread_news(&self.config.letters, feed)
        })
    }

    /// The descriptor every `Shell_NotifyIcon` call of this module is built from.
    fn notify_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            // The structure is versioned by its own size, so a wrong `cbSize` is rejected
            // wholesale rather than misread. It cannot overflow a `u32`; zero on the
            // impossible branch makes the call fail loudly instead of being interpreted.
            cbSize: u32::try_from(size_of::<NOTIFYICONDATAW>()).unwrap_or(0),
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            // `NIF_SHOWTIP` is not decoration: under `NOTIFYICON_VERSION_4` the standard
            // tooltip is suppressed unless it is asked for, so `NIF_TIP` on its own would
            // fill in `szTip` and never show it.
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: WM_APP_TRAY,
            hIcon: self.shown_icon(),
            ..Default::default()
        };

        // Task T-41-8: the tooltip goes through the same writer as the two balloon fields. It
        // used to have a copy of its own, and a copy is where the two would have drifted apart.
        // FR-90б, task T-119-2: with the flag on, the name of the layout comes first.
        let tip = match self.flag.as_ref().filter(|_| self.flag_on()) {
            Some(flag) => flag_tip(flag.layout, self.enabled()),
            None => icon_tip(self.enabled()),
        };
        write_field(&mut data.szTip, &tip);

        data
    }

    /// Adds the icon — `NIM_ADD` — and puts it into the version 4 convention.
    ///
    /// NFR-13, and the one place in this program where a failed Win32 call is *expected*:
    /// the shell may not be up yet, in which case `Shell_NotifyIcon` fails and FR-81 brings
    /// us back here when `TaskbarCreated` arrives. Failing the process over it would mean a
    /// program that cannot start before the desktop does.
    fn add_icon(&mut self) {
        self.add_calls += 1;
        self.prepare_flag();

        let data = self.notify_data();

        // SAFETY: `data` is a fully initialised `NOTIFYICONDATAW` owned by this frame; its
        // `cbSize` describes it, its `hWnd` is the live window this tray was installed on,
        // and its `hIcon` is owned by `self.active` or `self.paused`, both of which outlive
        // the icon in the notification area because `Tray::drop` removes the icon before the
        // handles are destroyed. The call reads through the pointer and does not keep it.
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };

        if !added.as_bool() {
            self.icon_present = false;
            app::report_non_critical("Shell_NotifyIconW(NIM_ADD)", &WinError::from_thread());
            return;
        }

        self.icon_present = true;

        // FR-90. Version 4 is what makes the shell send `WM_CONTEXTMENU` and `NIN_SELECT`
        // with the cursor position in `wParam`; it has to be asked for after a successful
        // `NIM_ADD`, since before one there is no icon to set the version of.
        let mut versioned = data;
        versioned.Anonymous.uVersion = NOTIFYICON_VERSION_4;

        // SAFETY: the same descriptor and the same invariants as the call above; the only
        // field that differs is the union member, which is written here and never read.
        // Writing a union field is safe in Rust — it is reading one that is not.
        let set = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &versioned) };

        if !set.as_bool() {
            // NFR-13. Not fatal, but not silent either: without version 4 the callback
            // parameters swap meaning and the menu would come up in the wrong place.
            app::report_non_critical(
                "Shell_NotifyIconW(NIM_SETVERSION)",
                &WinError::from_thread(),
            );
        }
    }

    /// Announces a letter with the icon's own balloon — `NIF_INFO`, FR-101, task Т-32-4.
    ///
    /// **The first knock of every letter but «Привет».** The balloon is the system's own — its
    /// colours, its font and its place are Windows', ours are only the icon and the words — and
    /// clicking it opens the letter (`NIN_BALLOONUSERCLICK`). If it is not noticed, FR-101 says
    /// the window opens by itself at the next quiet moment, and that is the schedule's business
    /// rather than this method's.
    ///
    /// `NIIF_USER` with `NIF_ICON` asks the shell to show **this program's own icon** in the
    /// balloon rather than one of the three system glyphs: a letter from the author is neither
    /// a warning nor an error, and the icon a person will click is the icon they already know.
    ///
    /// The two fields are truncated by [`write_field`] rather than refused: `szInfo` holds 255
    /// UTF-16 units and `szInfoTitle` 63, and a sentence of the string tables in a language
    /// nobody measured must never be able to stop a letter being announced (NFR-13).
    ///
    /// # ⛔ Почему значок был размытым — задача Т-33а-1, находка глазом пользователя
    ///
    /// `NIIF_USER` без `NIIF_LARGE_ICON` означает «возьми `hIcon` этой записи», а `hIcon` здесь
    /// — значок области уведомлений, загруженный при [`small_icon_size`] (16 px при 100 %).
    /// Шар уведомления оболочка рисует ЗАМЕТНО КРУПНЕЕ, и 16 px в нём растягиваются: это и
    /// была «размытая иконка» на снимке пользователя. `NIIF_LARGE_ICON` говорит оболочке брать
    /// **`hBalloonIcon`**, и туда кладётся тот же ресурс, загруженный при [`large_icon_size`];
    /// кадры 32/48/64 в `.ico` для этого уже были — добавлять в ресурсы ничего не пришлось.
    ///
    /// Малый значок трея не меняется: `NIF_ICON` и `hIcon` остаются прежними, и в области
    /// уведомлений стоит ровно то, что стояло.
    fn announce(&mut self, title: &str, body: &str) {
        // ⚠ **Task T-41-6, finding Н22 — the early return stays, and what changed is the flag.**
        // No icon means no balloon: a balloon belongs to an icon and the shell has nowhere to
        // put one. What used to make this line a defect is that a refused *update* cleared the
        // flag — so one refusal silenced every letter of FR-101 for the rest of the session.
        // The refusal is counted now ([`Tray::modify_failures`]) and the flag is left alone, so
        // this return means what it says again.
        if !self.icon_present {
            return;
        }

        let mut data = self.notify_data();

        self.announce_calls += 1;

        data.uFlags |= NIF_INFO;

        data.dwInfoFlags = match self.balloon_icon() {
            Some(icon) => {
                data.hBalloonIcon = icon;
                NIIF_USER | NIIF_LARGE_ICON
            }
            // NFR-13: без большого значка уведомление уходит как прежде, а не молчит.
            None => NIIF_USER,
        };

        write_field(&mut data.szInfo, body);
        write_field(&mut data.szInfoTitle, title);

        // SAFETY: identical to the `NIM_MODIFY` of `refresh_icon` — the same locally owned
        // descriptor, the same live window, the same icon handles owned by `self`; the two
        // extra fields are arrays inside that descriptor.
        let shown = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };

        if !shown.as_bool() {
            // NFR-13. A balloon the shell refused is a letter that will open by itself at the
            // next quiet moment instead — the schedule does not depend on this.
            app::report_non_critical("Shell_NotifyIconW(NIF_INFO)", &WinError::from_thread());
        }
    }

    /// Updates the icon in place because the **letters** moved — task Т-32-4, FR-90.
    ///
    /// The public half is [`refresh_unread_mark`]; this is the method it reaches, and it exists
    /// because `refresh_icon` is private and the tray is the one owner of the notification area
    /// entry.
    pub fn refresh_state_icon(&mut self) {
        self.refresh_icon();
    }

    /// Updates the icon and the tooltip in place — `NIM_MODIFY`.
    fn refresh_icon(&mut self) {
        if !self.icon_present {
            // Nothing to modify: the shell was not up when we tried. FR-81 will add the icon
            // with the current state, which is this one.
            return;
        }

        self.prepare_flag();
        let data = self.notify_data();

        self.modify_calls += 1;

        // SAFETY: identical to the `NIM_ADD` above — the same locally owned descriptor, the
        // same live window, the same icon handles owned by `self`.
        let modified = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };

        if !modified.as_bool() {
            // ⭐ **Task T-41-6, finding Н22: the flag is NOT cleared here.** It used to be, and
            // that single line was three defects. Updates go out on pause, on resume, on
            // «Применить» and on every change of language, so one refusal froze the icon with
            // yesterday's tooltip for the rest of the session; the same flag gates
            // [`Tray::remove_icon`], so the icon was then left behind at exit; and since
            // task Т-32-4 it gates [`Tray::announce`] too, so one refusal also silenced **every
            // letter of FR-101**.
            //
            // The flag answers one question — «удалось ли добавить значок» — and a refused
            // *update* is not an answer to it. What a refusal leaves behind is this count and
            // the line below.
            self.modify_failures += 1;
            app::report_non_critical("Shell_NotifyIconW(NIM_MODIFY)", &WinError::from_thread());
        }
    }

    /// Removes the icon — `NIM_DELETE`.
    ///
    /// ⭐ **Task T-41-6, finding Н22: unconditional, and the flag is not asked.** FR-83 says the
    /// icon comes off when the program ends. The early return that used to stand here asked
    /// [`Tray::icon_present`] — and the flag could say «нет» while the shell still held an icon,
    /// which is exactly how a dead icon was left in the notification area after «Выход».
    ///
    /// A `NIM_DELETE` for an icon that is genuinely not there fails, and a failure here is
    /// NFR-13: one line in the journal, no crash, nothing else. That is a far cheaper thing to
    /// be wrong about than a ghost the person has to log out to get rid of.
    fn remove_icon(&mut self) {
        let data = self.notify_data();

        self.delete_calls += 1;

        // SAFETY: as above. `NIM_DELETE` reads only `cbSize`, `hWnd` and `uID`, all of which
        // are the ones the icon was added under, and the window is still alive: the
        // `Attachment` that owns this tray is dropped before the window is destroyed.
        let removed = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };

        self.icon_present = false;

        if !removed.as_bool() {
            // NFR-13. A shell that went away between the add and the delete is the ordinary
            // reason, and the icon went with it.
            app::report_non_critical("Shell_NotifyIconW(NIM_DELETE)", &WinError::from_thread());
        }
    }

    /// FR-81: puts the icon back after the shell has restarted.
    ///
    /// Delete before add, because `NIM_ADD` fails when the icon is already there. After a
    /// real restart of the shell it is the delete that fails and the add that succeeds;
    /// after a `TaskbarCreated` sent for any other reason it is the other way round. Both
    /// orders end with exactly one icon, which is what makes this safe to receive from
    /// anywhere — SEC-05: the message is a broadcast and anybody can send it.
    fn readd_icon(&mut self) {
        self.remove_icon();
        self.add_icon();

        // Task T-119-4: a new taskbar is most likely a new explorer — the subscription to its list
        // of layouts names a process that is gone; take it again.
        self.list_watch = None;
        self.sync_flag_watch();
    }

    // -----------------------------------------------------------------------------------
    // The configuration
    // -----------------------------------------------------------------------------------

    /// Writes the configuration back — section 7, and the "сохранение конфигурации" of FR-83.
    ///
    /// # The decision of task T-13-6, and where it is taken
    ///
    /// **All four ways to save arrive here** — [`Tray::toggle_state`],
    /// [`Tray::set_autostart`], [`Tray::replace_config`] and [`Tray::shut_down`] — and
    /// [`Tray::save_policy`], set from the outcome of the one read this program performs,
    /// stands in front of all four. That is the whole of the fix: there is one door and the
    /// decision is on it, so no path can be added later that walks round it.
    ///
    /// * [`SavePolicy::Allowed`] — the file was read whole; write it, exactly as before.
    /// * [`SavePolicy::QuarantineFirst`] — the bytes on the disk are text this build could not
    ///   parse. They are moved to `config.toml.bad` first, byte for byte, and only then is the
    ///   new file written. If they cannot be moved, nothing is written: the disk still holds
    ///   the only copy, and a save is never worth it.
    /// * [`SavePolicy::NotRead`] — the file is there and could not be read, twice (решение 120.1,
    ///   task T-55-6). Nothing is written and nothing is moved this session, for the reason of
    ///   `Forbidden` below and under the same journal line: what is on the disk is most likely the
    ///   person's whole configuration, and nothing this build holds is known to be it.
    /// * [`SavePolicy::Forbidden`] — the file came from a newer build. **Nothing is written,
    ///   ever, this session, and that includes the shutdown of FR-83.** FR-83 asks for
    ///   «сохранение конфигурации»; for this one file the way to save it is to leave it alone,
    ///   because everything this build could write back is a strict subset of what is in it —
    ///   see [`SavePolicy::Forbidden`], where that reasoning is written out in full against
    ///   the module's own promise at [`settings::ReadOutcome::FromNewerSchema`].
    ///
    /// # Only what changed — task T-55-4, finding Н27
    ///
    /// The file is printed afresh from memory whenever it is written, so a write is not free: a
    /// person's comments, the order of the lines and the fields of a later schema are gone after
    /// it. Until that task every one of the four ways wrote on every call, and the shutdown of
    /// FR-83 is one of them — a single start of the product turned a file a person had commented
    /// into a machine's. Now the tray remembers what the file holds ([`Tray::on_disk`]), and while
    /// the configuration equals it, **nothing is touched**: no temporary, no rename, no new time
    /// on the file. FR-83's «сохранение конфигурации» is honoured all the same — the configuration
    /// is on the disk already.
    ///
    /// The comparison is with what is on the disk, not with what was read at the start: a person
    /// who changes a setting and then changes it back has made two changes, and the second must
    /// reach the file as surely as the first. No file at all, a file that needed a migration and a
    /// file this build could not read leave nothing to compare with, and their first save writes,
    /// as it always did.
    fn save_config(&mut self) {
        let Some(path) = self.config_path.as_deref() else {
            // No `%APPDATA%`: there is nowhere to save to, and nothing was read from there
            // either, so the state simply does not outlive the session.
            return;
        };

        // Task T-55-4: the file already holds exactly this configuration.
        if self.on_disk.as_ref() == Some(&self.config) {
            return;
        }

        match self.save_policy {
            SavePolicy::Allowed => {}

            // Решение 120.1, task T-55-6: a file that could not be read is left alone for the
            // session for the reason a newer one is — nothing this build could write is known to
            // be what the person has on the disk.
            SavePolicy::Forbidden | SavePolicy::NotRead => {
                note_configuration(CONFIG_SAVE_SUPPRESSED);
                return;
            }

            SavePolicy::QuarantineFirst => match settings::quarantine(path) {
                Ok(Quarantined::Moved) => {
                    note_configuration(CONFIG_QUARANTINED);
                    // Done once. What is at `path` from here on is this program's own file.
                    self.save_policy = SavePolicy::Allowed;
                }
                Ok(Quarantined::NothingThere) => {
                    // The file went away between the read and now. There are no bytes to
                    // keep, so there is nothing to name in the journal and nothing to stop
                    // the write.
                    self.save_policy = SavePolicy::Allowed;
                }
                Err(_) => {
                    // NFR-13: the refusal is acted on — it is the reason nothing is written —
                    // rather than dropped. The direction is the one that keeps the file: the
                    // bytes on the disk are still the user's only copy, so the save is given
                    // up and the policy stays, which makes the next save try the move again.
                    //
                    // The error value itself does not reach the journal. It is text, and
                    // SEC-01 and SEC-07 keep text out — the same reasoning
                    // `diag::dump_on_shutdown` writes down for the `io::Error` of a refused
                    // dump. The event names the fact; the fact is what a reader needs.
                    note_configuration(CONFIG_QUARANTINE_REFUSED);
                    return;
                }
            },
        }

        match settings::write_to(path, &self.config) {
            Ok(outcome) => {
                // Task T-55-4: what was just written is what the file holds now.
                self.on_disk = Some(self.config.clone());

                // ⭐ **Task T-55-3, finding Н26.** The file carried the read-only attribute, and
                // the write cleared it once and landed. A person may have set that attribute by
                // hand, so the fact is named rather than taken silently.
                if outcome == settings::WriteOutcome::WrittenAfterClearingReadOnly {
                    note_configuration(CONFIG_READ_ONLY_CLEARED);
                }
            }
            Err(error) => {
                // A failed save must not take the process down: in the FR-83 case the program is
                // on its way out anyway, and in the toggle case the state is already right in
                // memory and on the screen.
                //
                // ⭐ **Task Т-22-9 — the debt of task T-06-4 closed, finding м12 of the audit of
                // 2026-09-01.** Not taking the process down is not the same as saying nothing: the
                // user pressed «Применить», the dialog closed, the setting is right on the screen
                // and wrong on the disk, and until this task the only configuration event of the
                // six that left no trace was this one — the one that loses the user's choice.
                note_configuration_failure(CONFIG_WRITE_FAILED, &error);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The configuration events of the journal — task T-13-6, SEC-01 and SEC-07
// ---------------------------------------------------------------------------------------

/// The read came back with text this build could not turn into a configuration.
const CONFIG_UNREADABLE: &str = "configuration file unreadable";

/// The file carries a `schema_version` this build does not know. Saving is off for the session.
const CONFIG_NEWER_SCHEMA: &str = "configuration file from a newer schema";

/// ⭐ **Решение 120.1, task T-55-6.** The file is there and could not be read, twice. Nothing is
/// written and nothing is moved for the session — [`SavePolicy::NotRead`].
const CONFIG_NOT_READ: &str = "configuration file not read";

/// ⭐ **Решение 120.2, task T-55-7.** A number of the file was not a whole number its field can
/// hold and was read as the default of that field; the rest of the file was read as it is.
const CONFIG_FIELD_DEFAULTED: &str = "configuration field read as its default";

/// The unreadable file was moved to `config.toml.bad` before the first write.
const CONFIG_QUARANTINED: &str = "configuration file quarantined";

/// It could not be moved, so nothing was written over it.
const CONFIG_QUARANTINE_REFUSED: &str = "configuration file quarantine refused";

/// A save was asked for and given up so that the file on the disk survives.
const CONFIG_SAVE_SUPPRESSED: &str = "configuration save suppressed";

/// ⭐ **Task Т-22-9.** The save was allowed, was attempted, and the file did not take it.
///
/// The sixth configuration event, and the last one to get a name: the other five have had rows
/// since task T-13-6, and this — the only one that loses a choice the user has already been shown
/// as applied — carried the open `TODO` of task T-06-4 and a dropped error instead.
const CONFIG_WRITE_FAILED: &str = "configuration write failed";

/// ⭐ **Task T-55-3, finding Н26.** The save landed only after the read-only attribute of
/// `config.toml` was cleared once — [`settings::WriteOutcome::WrittenAfterClearingReadOnly`].
const CONFIG_READ_ONLY_CLEARED: &str = "configuration read-only attribute cleared";

/// ⭐ **Task T-55-5, finding Т8.** The start found a temporary of an interrupted write beside the
/// file and could not remove it — [`settings::remove_abandoned_temporaries`].
const CONFIG_TEMPORARY_NOT_REMOVED: &str = "configuration temporary not removed";

/// ⭐ **Решение 120.4 (б), task T-55-1.** A start found `general.autostart = true` and no value of
/// this image under `HKCU\…\Run`, and wrote one — [`reconcile_autostart`].
const AUTOSTART_REGISTERED_AT_START: &str = "autostart registered at start";

/// The same start found `general.autostart = false` and a value under the name, and removed it.
const AUTOSTART_REMOVED_AT_START: &str = "autostart removed at start";

/// ⭐ **Решение 120.4 (ж), task T-55-2.** «Применить» or the check mark of FR-91 asked for a change
/// of autostart on a session that lives on a file from a newer schema, and the registry was not
/// asked — [`apply_settings_via`], [`toggle_autostart_via`].
const AUTOSTART_CHANGE_SUPPRESSED: &str = "autostart change suppressed";

/// Puts one configuration event into the ring: the fact, and nothing of the file.
///
/// **SEC-01, SEC-07.** The argument is one of the literals above, chosen at compile time,
/// and [`diag::Operation::from_name`] narrows even those onto the closed table of `src\diag.rs`
/// — a name that is not a row of it becomes `UNLISTED` and keeps none of its text. There is no
/// branch here through which a byte of the file, the name of a field this build does not know,
/// or the line and column a parser stopped at could reach the journal: the journal says *what
/// happened to the file* and never *what was in it*.
///
/// The code is always [`diag::OsCode::NONE`]. These are events of the program, not failures of
/// a Win32 call, and `OsCode` has no constructor that takes a number by design.
fn note_configuration(operation: &'static str) {
    diag::record(diag::Operation::from_name(operation), diag::OsCode::NONE);
}

/// [`note_configuration`] for an event that has a **system error** behind it — task **Т-22-9**.
///
/// **SEC-01, SEC-07.** The name is one of the literals above and is narrowed by
/// [`diag::Operation::from_name`] exactly as there. What is added is the code, and the code is the
/// number the operating system gave and nothing else: [`diag::OsCode::of_config`] takes it out of
/// the error and drops the error's text — the same road `diag::dump_on_shutdown` takes for the
/// `io::Error` of a refused dump, and the same line [`crate::app::report_non_critical`] draws for a
/// `windows::core::Error`.
///
/// ⭐ **Task T-69-6, backlog line Э34-Б-3 — one bridge, not two.** Until this task this module
/// read the Win32 number itself, in a function of its own (`os_code_of`, task Т-22-9), the same
/// five-step chain `diag::OsCode::of_io` repeats for the journal's file. The chain now lives in
/// `diag` alone, and the configuration reaches it with an error object — decision 129.3: a
/// constructor of `OsCode` out of a number is what SEC-07 forbids.
///
/// A path never reaches here, in any form. Neither does the line and column of a parser, which is
/// the one thing a [`settings::ConfigError`] carries that was derived from the file's contents —
/// see [`diag::OsCode::of_config`], where that variant is answered with no number at all.
fn note_configuration_failure(operation: &'static str, error: &settings::ConfigError) {
    diag::record(
        diag::Operation::from_name(operation),
        diag::OsCode::of_config(error),
    );
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// Ownership of the calling thread's tray.
///
/// The value carries nothing; what it owns is the entry in the thread-local. Dropping it
/// takes the [`Tray`] out and drops it, which is the cleanup path of FR-83, and that happens
/// *before* the window is destroyed because `app::serve_window` declares the window first
/// and Rust drops locals in reverse order of declaration.
pub struct Attachment {
    /// Makes the type neither `Send` nor `Sync`. The tray lives in one thread's local
    /// storage, so a value that claimed the right to cross threads would be a lie: dropping
    /// it elsewhere would clear the wrong thread's slot and leave the icon behind.
    _not_send: PhantomData<*const ()>,
}

impl Drop for Attachment {
    fn drop(&mut self) {
        detach();
    }
}

thread_local! {
    /// The tray of this thread, if it has one. Only the UI thread ever does.
    ///
    /// A thread-local rather than a pointer in `GWLP_USERDATA`: the window procedure has to
    /// reach this from a plain `extern "system"` function, and the alternative is a raw
    /// pointer whose lifetime nothing can check. `app` made the same choice for its shutdown
    /// flag, for the same reason.
    static UI_TRAY: RefCell<Option<Tray>> = const { RefCell::new(None) };
}

/// Installs the tray on the calling thread — the UI thread of section 6.1.
///
/// Called by `app::serve_window` immediately after the UI window has been created. Fails
/// only for reasons that cannot be transient: an icon missing from our own resources, or a
/// menu the window manager refused to create. A shell that is not up yet is **not** one of
/// them — that is FR-81's business, and it is reported rather than raised.
pub fn attach(hwnd: HWND, instance: HINSTANCE) -> WinResult<Attachment> {
    let attachment = attach_at(hwnd, instance, settings::default_config_path())?;

    // ⭐ Task T-119-9, решение 208.98: the feed kept by the last good read comes back before the
    // first tick and before the first menu — the product's own start only: the tests that attach
    // a tray of their own never read the `%APPDATA%` of the machine they run on.
    crate::letters::restore_feed();

    Ok(attachment)
}

/// The same, with the configuration read from `config_path` — task T-13-14.
///
/// Separate from [`attach`] for exactly the reason [`Tray::install_at`] is separate from
/// [`Tray::install`], and it is the missing other half of that pair: a test could already
/// build a tray of its own on a file under `%TEMP%`, but not one that [`with_tray`] can find,
/// and [`dispatch_command`] reaches the tray through [`with_tray`] and through nothing else.
/// Without this the gate of task T-13-14 could only be checked by reading the source, and its
/// acceptance asks for the value in memory and the bytes on the disk.
///
/// ⚠ `None`, or a path under `%APPDATA%`, is the product's business and not a test's: section
/// 7 puts the real file in `%APPDATA%\Lang_Switcher\config.toml`, and that file belongs to
/// whoever is running the tests.
///
/// Since task T-55-1 this is also where the product's autostart is carried out: it hands
/// [`attach_at_via`] the guard of решение 120.4 (е) and the real `Run` key — the value as
/// [`settings::autostart_value`] reads it and [`settings::set_autostart`] to write it — and nothing
/// else in the program does. A test or a bench that attaches through here is kept off that key by
/// the guard, which answers no for every image but the installed one.
pub fn attach_at(
    hwnd: HWND,
    instance: HINSTANCE,
    config_path: Option<PathBuf>,
) -> WinResult<Attachment> {
    attach_at_via(
        hwnd,
        instance,
        config_path,
        settings::this_build_may_register_autostart(),
        settings::autostart_value,
        settings::set_autostart,
    )
}

/// The same, with the registry half of решение 120.4 handed in — task T-55-1.
///
/// Installs the tray and, before the tray is put where [`with_tray`] finds it, makes the `Run` key
/// of FR-93 agree with the configuration the program is about to live by — [`reconcile_autostart`].
///
/// # Why the registry is three arguments
///
/// The shape and the reason of [`apply_settings_via`]: the branches are the whole point of the
/// reconciliation, and no test may drive them through the real `HKCU\…\CurrentVersion\Run` — that
/// key belongs to whoever runs the tests. `tests\tray.rs` hands in a guard and two closures that
/// count what they were asked; the product, through [`attach_at`], hands in
/// [`settings::this_build_may_register_autostart`], [`settings::autostart_value`] and
/// [`settings::set_autostart`], and nothing else does.
pub fn attach_at_via(
    hwnd: HWND,
    instance: HINSTANCE,
    config_path: Option<PathBuf>,
    this_build_may_register: bool,
    run_key_value: impl FnOnce() -> Option<String>,
    write_run_key: impl FnOnce(bool) -> WinResult<()>,
) -> WinResult<Attachment> {
    let tray = Tray::install_at(hwnd, instance, config_path)?;

    // Решение 120.4 (б): the configuration is the source of truth, and every start carries it out.
    reconcile_autostart(&tray, this_build_may_register, run_key_value, write_run_key);

    UI_TRAY.with(|slot| slot.replace(Some(tray)));

    // FR-101, task Т-32-4 — the clock of the letters, armed once, here: this is the moment the
    // program has a UI window, a configuration and an icon, and the first check is ninety
    // seconds away (NFR-08).
    start_letters_clock(hwnd);

    // Task T-119-6: the start after an important update owes the window «Обновлено» — planned here,
    // before the first tick of the letters moves the version seen; it waits for its quiet moment.
    let _ = crate::corner::plan_release_window(hwnd);

    Ok(Attachment {
        _not_send: PhantomData,
    })
}

/// Makes the `Run` key of FR-93 agree with the configuration the program starts on — **решение
/// 120.4 (б)**, task T-55-1.
///
/// # Why at every start
///
/// Until this task `general.autostart = true`, the default of section 7, was a check mark and
/// nothing more: the installer left its autostart task unticked, and the program wrote the value
/// only when somebody pressed «Применить» or chose the entry of FR-91 — after a reboot nothing
/// started, and autostart came on by an accident of the hand rather than by a decision. The
/// configuration is the source of truth (решение 120.4 (а)), and every start carries it out: `true`
/// meets no value of this image — the value is written; `false` meets a value — it is removed;
/// agreement — nothing is touched, not even written again.
///
/// **"A value of this image" is the command [`settings::autostart_command`] builds**, not any value
/// under the name. The name is this program's, but a value naming another image — a build that
/// once registered itself from a working tree, an older install somewhere else — starts that other
/// image at the next logon, which is not autostart of this program; it is written over.
///
/// # Three gates, in this order
///
/// 1. **Решение 120.4 (е) — `this_build_may_register`.** Only the installed release image says yes
///    ([`settings::autostart_may_register`]); a test binary or a debug build returns here, before
///    the registry is so much as read.
/// 2. **Решение 120.4 (ж) — a configuration a person stands behind.** A path to a file at all — a
///    session with no `%APPDATA%` read nothing, and its configuration is the defaults and nobody's
///    — and [`SavePolicy::lets_the_run_key_follow`] at [`settings::RunKeyMoment::Start`], the one
///    rule this shares with «Применить» and the check mark of FR-91.
/// 3. **Agreement** — the value is read, and written only when it disagrees.
///
/// # A refusal changes nothing in memory
///
/// Unlike [`apply_settings_via`], nothing steps back. There a person asked for a change the system
/// refused, and the file must not claim it; here nobody asked for anything — the file says what the
/// person chose and stays the source of truth. The refusal is reported under `RegSetValueExW`, the
/// name the other two writers of the value report under; the state line of the dialog goes on
/// showing the registry's own answer beside the check box; and the next start tries again.
///
/// Each action is one entry of the journal, and agreement is none: an event per healthy start is
/// noise — the reasoning [`Tray::install_at`] gives for the read of the configuration.
fn reconcile_autostart(
    tray: &Tray,
    this_build_may_register: bool,
    run_key_value: impl FnOnce() -> Option<String>,
    write_run_key: impl FnOnce(bool) -> WinResult<()>,
) {
    if !this_build_may_register {
        return;
    }

    if tray.config_path.is_none()
        || !tray
            .save_policy
            .lets_the_run_key_follow(settings::RunKeyMoment::Start)
    {
        return;
    }

    let wanted = tray.config.general.autostart;
    let value = run_key_value();

    let agrees = if wanted {
        value.is_some() && value == settings::autostart_command()
    } else {
        value.is_none()
    };

    if agrees {
        return;
    }

    match write_run_key(wanted) {
        Ok(()) => note_configuration(if wanted {
            AUTOSTART_REGISTERED_AT_START
        } else {
            AUTOSTART_REMOVED_AT_START
        }),
        Err(error) => app::report_non_critical("RegSetValueExW", &error),
    }
}

/// Takes the tray out of the calling thread and drops it, running the cleanup of FR-83.
fn detach() {
    // Taken out of the `RefCell` before being dropped, so that the borrow is over before
    // `Tray::drop` runs. `Shell_NotifyIcon` does not dispatch messages back to us, but a
    // cleanup path that would deadlock if it ever did is not one to leave lying around.
    let tray = UI_TRAY.with(|slot| slot.borrow_mut().take());
    drop(tray);
}

/// Runs `f` against the tray of the calling thread, if this thread has one.
///
/// `None` means "this thread is not the UI thread", which is the ordinary answer on the
/// input and watcher threads and is what keeps section 6.1 true by construction.
///
/// ⚠ `f` must not run a modal loop — no `TrackPopupMenuEx`, no `MessageBoxW`, no dialog. The
/// borrow is live for the whole call, and a modal loop dispatches messages back into
/// [`handle_ui_message`], which would borrow again and panic. Task T-08-1 has to observe
/// this when it adds the settings dialog.
pub fn with_tray<R>(f: impl FnOnce(&mut Tray) -> R) -> Option<R> {
    UI_TRAY.with(|slot| slot.borrow_mut().as_mut().map(f))
}

// ⛔ **`setting_change_name` stood here until task T-41-2 (finding В2), and it is not coming
// back.** It read the string a `WM_SETTINGCHANGE` names by dereferencing the pointer in
// `lParam` — bounded by a cap and a null check, and still a dereference of a number a stranger
// wrote. Twice the window it did that in was narrowed (tasks T-11-9 and T-13-20); the audit of
// 2026-09-04 asked instead for the question to be closed, and closing it cost nothing, because
// the one use the answer was ever put to was a comparison against a word this build owns.
//
// This note is left for the reader who comes looking for the function: after this task **no
// code of `tray.rs` dereferences anything that arrived on a message**, and `tests\tray.rs`
// sweeps the file to keep it so. Its cap and its test-only counter went with it.

/// The window-procedure entry point of the tray.
///
/// `Some` means the message was handled and that value has to be returned to Windows; `None`
/// means it is none of ours and belongs to `DefWindowProcW`.
pub fn handle_ui_message(message: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    // FR-101, task Т-32-4: the tick of the letters, answered **before** the tray is borrowed
    // and outside it — the schedule reads the configuration through `with_tray` of its own.
    if message == WM_TIMER && wparam.0 == LETTERS_TIMER {
        on_letters_tick();
        return Some(LRESULT(0));
    }

    // FR-90б, task T-119-4: the layout may have moved — read it now, and for `LATER` again after
    // each of the delays, by one-shot timers (NFR-10: no poll). Outside the borrow: `refresh_flag`
    // takes it. SEC-05: the messages carry nothing; a forged one buys a reading of the layout.
    if message == crate::flags::WM_APP_FLAG_NOW || message == crate::flags::WM_APP_FLAG_LATER {
        refresh_flag();

        if message == crate::flags::WM_APP_FLAG_LATER
            && let Some(hwnd) = with_tray(|tray| tray.hwnd)
        {
            for (timer, delay) in crate::flags::REREAD_TIMERS
                .into_iter()
                .zip(crate::flags::REREAD_MS)
            {
                // SAFETY: the live UI window and identifiers of this module; a timer that exists
                // is replaced, which is what a second `LATER` within the delay wants.
                let _ = unsafe { SetTimer(Some(hwnd), timer, delay, None) };
            }
        }

        return Some(LRESULT(0));
    }

    // FR-90б, task T-119-3: a list of layouts opened by a click on the flag — its close, the missing
    // indicator, and the three one-shot timers of `flags::LIST_TIMERS`. Outside the borrow: the
    // finish reads the layout and may switch it. SEC-05: the messages carry nothing.
    if message == crate::flags::WM_APP_LIST_CLOSED {
        list_closed();
        return Some(LRESULT(0));
    }

    if message == crate::flags::WM_APP_LIST_MISSING {
        list_missing();
        return Some(LRESULT(0));
    }

    // Task T-119-6: the window «Обновлено» waits for its quiet moment, once a second while it is
    // due; task T-119-5: the hint of the flag goes once the flag is on the taskbar. Outside the
    // borrow — the window in the corner reads the configuration through `with_tray` of its own.
    if message == WM_TIMER && wparam.0 == crate::corner::QUIET_TIMER {
        let hwnd = with_tray(|tray| tray.hwnd)?;
        let sound = with_tray(|tray| tray.config().feedback.sound).unwrap_or(false);
        crate::corner::quiet_tick(hwnd, sound);
        return Some(LRESULT(0));
    }

    if message == WM_TIMER && wparam.0 == crate::corner::HINT_TIMER {
        let hwnd = with_tray(|tray| tray.hwnd)?;
        crate::corner::hint_tick(hwnd, TRAY_ICON_ID);
        return Some(LRESULT(0));
    }

    // FR-82 since 1.8.0, task T-119-14: the settings a second start opened come forward.
    if message == WM_TIMER && wparam.0 == SECOND_START_TIMER {
        if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
            // SAFETY: the timer of this window this very message names; one-shot by this kill.
            let _ =
                unsafe { windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), wparam.0) };
        }

        bring_the_open_window_forward();
        return Some(LRESULT(0));
    }

    if message == WM_TIMER && crate::flags::LIST_TIMERS.contains(&wparam.0) {
        list_timer(wparam.0);
        return Some(LRESULT(0));
    }

    if message == WM_TIMER && crate::flags::REREAD_TIMERS.contains(&wparam.0) {
        if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
            // SAFETY: the timer of this window this very message names; one-shot by this kill.
            let _ =
                unsafe { windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), wparam.0) };
        }

        refresh_flag();

        // The last re-read: a console in front is blind to `switch::read` — ask the watcher for the
        // name of the input indicator (решение 208.142: once, about 300 ms after the event).
        if wparam.0 == crate::flags::REREAD_TIMERS[1]
            && crate::flags::is_on()
            && crate::switch::read().blind
        {
            let _ = app::post_to_watcher_thread(crate::flags::WM_APP_FLAG_CONSOLE);
        }

        return Some(LRESULT(0));
    }

    // FR-102, task Т-32-6: the feed thread has left something in its box. Outside every borrow
    // of the tray — taking it writes the configuration through `with_tray` of its own.
    //
    // SEC-05: the message carries nothing; a forged one finds an empty box.
    // Task T-120-15: the command chosen in the menu of the shell, once the menu is gone.
    if message == WM_APP_MENU_CHOSEN {
        run_chosen();
        return Some(LRESULT(0));
    }

    if message == crate::letters::WM_APP_FEED {
        crate::letters::take_feed();
        return Some(LRESULT(0));
    }

    // FR-101: the balloon was clicked. Outside the borrow for the same reason, and after the
    // tray has been asked — the icon's own bookkeeping runs first.
    if message == WM_APP_TRAY && u32::from(low_word(unsigned(lparam.0))) == NIN_BALLOONUSERCLICK {
        if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
            crate::letters::open_announced(hwnd);
        }

        return Some(LRESULT(0));
    }

    let reaction = with_tray(|tray| tray.handle_message(message, wparam, lparam))?;

    match reaction {
        Reaction::Ignored => None,
        Reaction::Handled(result) => Some(result),
        // Outside the borrow on purpose — see the module documentation.
        Reaction::ShowMenu {
            x,
            y,
            from_the_keyboard,
        } => {
            show_menu(x, y, from_the_keyboard);
            Some(LRESULT(0))
        }
        // Outside the borrow for the same reason: the dialog of FR-92 is modal.
        //
        // ⚠ Task T-43-4, finding С21: **through the door**, not past it. Until this task the arm
        // called `open_settings` itself, so a double click on the icon opened the settings dialog
        // over the «О программе» window the menu would have refused it over — the one road into
        // a command that did not ask [`dialog_locks_command`].
        Reaction::ShowSettings => {
            let hwnd = with_tray(|tray| tray.hwnd)?;
            dispatch_command(hwnd, CMD_SETTINGS);
            Some(LRESULT(0))
        }
        // Task T-119-3. Outside the borrow: the press reaches into explorer.
        Reaction::OpenList => {
            open_list();
            Some(LRESULT(0))
        }
    }
}

// ---------------------------------------------------------------------------------------
// A click on the flag — task T-119-3, решения 208.88, 208.123, 208.164, 208.204
// ---------------------------------------------------------------------------------------

/// The window the person was in when the flag was clicked — the first window of the Z-order that is
/// visible, not cloaked by DWM, not a tool window, not a part of the shell and not this program's:
/// the click put the taskbar in front, and the window under it is the one the person typed in.
///
/// ⚠ **Stand T-119-3, Windows 11, 2026-11-20:** the window remembered by the re-readings of the flag
/// alone ([`Tray::last_person`]) was once Блокнот while Edge was in front — Edge came forward with no
/// re-reading the program saw — and the focus went back to Блокнот. The Z-order is asked at the
/// click; the remembered window is the fallback.
fn person_window_in_front() -> Option<usize> {
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::UI::WindowsAndMessaging::{
        GW_HWNDNEXT, GWL_EXSTYLE, GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowRect,
        GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOOLWINDOW,
    };

    let me = std::process::id();

    // SAFETY: `None` asks for the top of the desktop's Z-order; a null answer is checked.
    let mut window = unsafe { GetTopWindow(None) }.ok()?;

    for _ in 0..1024 {
        // SAFETY: window handles by value and locals; every call answers a value or a failure.
        let fit = unsafe {
            let mut cloaked = 0u32;
            let mut rect = RECT::default();
            let mut process = 0u32;

            let _ = DwmGetWindowAttribute(
                window,
                DWMWA_CLOAKED,
                (&raw mut cloaked).cast(),
                u32::try_from(size_of::<u32>()).unwrap_or(4),
            );
            let _ = GetWindowRect(window, &mut rect);
            GetWindowThreadProcessId(window, Some(&mut process));

            IsWindowVisible(window).as_bool()
                && cloaked == 0
                && GetWindowLongPtrW(window, GWL_EXSTYLE) & (WS_EX_TOOLWINDOW.0 as isize) == 0
                && rect.right > rect.left
                && rect.bottom > rect.top
                && process != me
        };

        if fit && !is_shell_window(window.0 as usize) {
            return Some(window.0 as usize);
        }

        // SAFETY: a window handle by value; the end of the Z-order answers a failure.
        window = unsafe { GetWindow(window, GW_HWNDNEXT) }.ok()?;
    }

    None
}

/// Whether the window a handle names is a part of the shell — the class through
/// [`crate::flags::is_shell_window_class`].
fn is_shell_window(raw: usize) -> bool {
    let mut name = [0u16; 64];

    // SAFETY: a window handle by value (a stale one answers 0) and a local buffer whose length is
    // passed.
    let len = unsafe { GetClassNameW(HWND(raw as *mut core::ffi::c_void), &mut name) };
    let len = usize::try_from(len).unwrap_or(0).min(name.len());

    crate::flags::is_shell_window_class(&String::from_utf16_lossy(&name[..len]))
}

/// Starts or replaces one of the one-shot timers of [`crate::flags::LIST_TIMERS`].
fn arm_list_timer(hwnd: HWND, which: usize) {
    // SAFETY: the live UI window and an identifier of this module; a timer that exists is
    // replaced, which is what a second close within the delay wants.
    let _ = unsafe {
        SetTimer(
            Some(hwnd),
            crate::flags::LIST_TIMERS[which],
            crate::flags::LIST_MS[which],
            None,
        )
    };
}

/// A left click on the flag: the list of layouts of Windows — решение 208.88, «как если бы нажали по
/// стандартному значку rus/eng». Windows 10 — `WM_LBUTTONDOWN`/`UP` posted to `InputIndicatorButton`
/// on this thread (решение 208.123), then the watcher puts the cursor on the selected row of the list
/// once it is up (решение 208.204); Windows 11 — UI Automation `Invoke` on the watcher thread. The
/// session remembers the window the person typed in and the layout the flag showed; whether the list
/// opened at all is looked at once, 1.5 s after the press.
fn open_list() {
    let Some(hwnd) = with_tray(|tray| {
        if !tray.flag_on() {
            return None;
        }

        if crate::flags::own_list_is_open() {
            return None;
        }

        tray.list_session = Some(ListSession {
            target: person_window_in_front().unwrap_or(tray.last_person),
            before: tray.flag_layout().unwrap_or_default(),
            chosen: LayoutId::default(),
            own: false,
            gift_left: crate::flags::GIFT_RETRIES,
        });

        Some(tray.hwnd)
    })
    .flatten() else {
        return;
    };

    let asked = if crate::flags::press_indicator_button() {
        // Windows 10, решение 208.204: the cursor onto the selected row of the list once it is
        // up — the list of a pressed button takes the arrows only then. A watcher that cannot be
        // asked leaves the cursor where it is; the list is open all the same.
        let _ = app::post_to_watcher_thread(crate::flags::WM_APP_LIST_CURSOR);
        true
    } else {
        app::post_to_watcher_thread(crate::flags::WM_APP_LIST_OPEN)
    };

    if !asked {
        list_missing();
        return;
    }

    arm_list_timer(hwnd, 0);
}

/// No list of Windows to open — РУС/ENG is hidden at the clock (the watcher found no indicator, or
/// the list did not come up within the look of [`crate::flags::LIST_MS`]): the list of the
/// program's own comes up beside the flag — task T-119-3, part Б. The session goes on as its own;
/// when even that window cannot be made, the session is dropped (NFR-13).
fn list_missing() {
    let Some((hwnd, before)) = with_tray(|tray| {
        let open = tray.list_session.as_mut()?;

        if open.own {
            return None;
        }

        open.own = true;
        Some((tray.hwnd, open.before))
    })
    .flatten() else {
        return;
    };

    // SAFETY: the timer of this window; a missing one is no error worth a word.
    let _ = unsafe {
        windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), crate::flags::LIST_TIMERS[0])
    };

    let shown = icon_rect(hwnd).is_some_and(|icon| crate::flags::show_own_list(icon, before));

    if !shown {
        with_tray(|tray| tray.list_session = None);
    }
}

/// The rectangle of the icon on the screen — `Shell_NotifyIconGetRect`; under a closed ^ the shell
/// answers the rectangle of ^ itself, which is where the list of the program's own then stands.
fn icon_rect(hwnd: HWND) -> Option<RECT> {
    use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};

    let icon = NOTIFYICONIDENTIFIER {
        cbSize: u32::try_from(size_of::<NOTIFYICONIDENTIFIER>()).unwrap_or(0),
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    };

    // SAFETY: a live local whose `cbSize` describes it; the answer is a value.
    unsafe { Shell_NotifyIconGetRect(&icon) }.ok()
}

/// The list of the program's own ended — called by module `flags` on this thread, outside its
/// borrow. A row chosen: the focus goes back to the person's window and, 120 ms later, the program's
/// switch of FR-50 gives the layout (a console too — the program's list is not Windows', nothing
/// reaches the window by itself). `Esc`: the focus goes back, nothing else. Away (a click
/// elsewhere, the link): the session ends and the focus stays where the person put it.
pub(crate) fn own_list_ended(end: crate::flags::OwnListEnd) {
    let Some((hwnd, session)) =
        with_tray(|tray| tray.list_session.map(|s| (tray.hwnd, s))).flatten()
    else {
        return;
    };

    let give_back = || {
        if session.target != 0 {
            // SAFETY: a window handle by value; a stale one is refused by the system, not misused.
            let _ = unsafe { SetForegroundWindow(HWND(session.target as *mut core::ffi::c_void)) };
        }
    };

    match end {
        crate::flags::OwnListEnd::Chosen(layout) => {
            with_tray(|tray| {
                if let Some(open) = tray.list_session.as_mut() {
                    open.chosen = layout;
                }
            });
            give_back();
            arm_list_timer(hwnd, 2);
        }
        crate::flags::OwnListEnd::Escaped => {
            with_tray(|tray| tray.list_session = None);
            give_back();
        }
        crate::flags::OwnListEnd::Away => {
            with_tray(|tray| tray.list_session = None);
        }
    }
}

/// The list closed (`flags::ListWatch`). For a list of the program's own the choice is read 60 ms
/// later; a list the person opened at the clock has no session and is left alone, and so has the
/// list of the program's own — it is no list of Windows.
fn list_closed() {
    let Some(hwnd) = with_tray(|tray| {
        tray.list_session
            .is_some_and(|open| !open.own)
            .then_some(tray.hwnd)
    })
    .flatten() else {
        return;
    };

    // SAFETY: the timer of this window; a missing one is no error worth a word.
    let _ = unsafe {
        windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), crate::flags::LIST_TIMERS[0])
    };

    arm_list_timer(hwnd, 1);
}

/// The layout of the taskbar's thread — what the person chose in a list the program opened: the
/// click on the flag put the taskbar in front, and the choice lands on its thread (T-119-1).
fn shell_layout() -> LayoutId {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};

    // SAFETY: a constant class name; a null answer is checked.
    let Ok(taskbar) = (unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }) else {
        return LayoutId::default();
    };

    // SAFETY: a live window handle by value; no process id asked for.
    let thread = unsafe { GetWindowThreadProcessId(taskbar, None) };

    if thread == 0 {
        return LayoutId::default();
    }

    // SAFETY: reads the layout of a thread by its identifier.
    LayoutId::from_raw(unsafe { GetKeyboardLayout(thread) }.0 as usize)
}

/// One of the one-shot timers of a list the program opened.
///
/// - **0, the look (1.5 s after the press):** no list on the screen and none closed meanwhile — the
///   indicator is hidden or did not answer; the session is dropped.
/// - **1, the choice (60 ms after the close):** the layout of the shell's thread is what was chosen;
///   the focus goes back to the person's window (`SetForegroundWindow`, no F24 — T-119-1, 60 of 60).
/// - **2, the look at the window (120 ms later):** where the choice did not arrive by itself (Edge),
///   the program's own switch of FR-50 gives it — [`crate::flags::layout_to_give`]; then the flag
///   reads the layout and the session ends.
fn list_timer(timer: usize) {
    let Some((hwnd, session)) =
        with_tray(|tray| tray.list_session.map(|s| (tray.hwnd, s))).flatten()
    else {
        return;
    };

    // SAFETY: the timer of this window this very message names; one-shot by this kill.
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), timer) };

    if timer == crate::flags::LIST_TIMERS[0] {
        if !session.own && !crate::flags::list_is_open() {
            list_missing();
        }

        return;
    }

    if timer == crate::flags::LIST_TIMERS[1] {
        let chosen = shell_layout();

        with_tray(|tray| {
            if let Some(open) = tray.list_session.as_mut() {
                open.chosen = chosen;
            }
        });

        if session.target != 0 {
            // SAFETY: a window handle by value; a stale one is refused by the system, not misused.
            let _ = unsafe { SetForegroundWindow(HWND(session.target as *mut core::ffi::c_void)) };
        }

        arm_list_timer(hwnd, 2);
        return;
    }

    let reading = crate::switch::read();

    // ⭐ Defect 3 of the stand (2026-11-20, Edge under ^): the gift waits for the person's window.
    match crate::flags::gift_step(reading.window.raw() == session.target, session.gift_left) {
        crate::flags::GiftStep::Wait { left } => {
            crate::diag::record(
                crate::diag::Operation::from_name("list gift skipped"),
                crate::diag::OsCode::NONE,
            );

            with_tray(|tray| {
                if let Some(open) = tray.list_session.as_mut() {
                    open.gift_left = left;
                }
            });

            arm_list_timer(hwnd, 2);
            return;
        }
        crate::flags::GiftStep::GiveUp => {
            crate::diag::record(
                crate::diag::Operation::from_name("list gift given up"),
                crate::diag::OsCode::NONE,
            );

            with_tray(|tray| tray.list_session = None);
            refresh_flag();
            return;
        }
        crate::flags::GiftStep::Now => {}
    }

    with_tray(|tray| tray.list_session = None);

    let give = if session.own {
        crate::flags::own_layout_to_give(session.chosen, reading.layout, reading.blind)
    } else {
        crate::flags::layout_to_give(
            session.before,
            session.chosen,
            reading.layout,
            reading.blind,
        )
    };

    if reading.window.raw() == session.target
        && let Some(layout) = give
    {
        // FR-50, the program's own switch — no sound: the person chose, the program completes.
        let sent = crate::switch::to(layout);

        // ⭐ Defect 3 of the stand (2026-11-20, cmd): a console reads blind (FR-52) and its flag comes
        // from the name of РУС/ENG — which is hidden whenever the list of the program's own is open
        // at all. The flag then stayed on the old layout; it shows the one the person chose, which
        // this program has just given to that very window.
        if crate::flags::flag_shows_the_given(session.own, reading.blind, &sent) {
            with_tray(|tray| tray.show_flag_of(Some(layout)));
            return;
        }
    }

    refresh_flag();
}

// ---------------------------------------------------------------------------------------
// The menu — FR-91
// ---------------------------------------------------------------------------------------

/// One command entry of FR-91 as the builder recorded it — the menu of the shell is described
/// from these, and the tests check FR-91 entry for entry against them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// The command identifier.
    pub command: u32,
    /// The label, out of the string table of the locale in force (FR-94).
    pub label: String,
    /// Whether the entry shows the check mark of FR-93.
    pub checked: bool,
    /// Whether the entry can be chosen — task T-13-9, point (б).
    ///
    /// True for every entry of FR-91 except one: «Возобновить» while FR-99 holds the program
    /// disarmed, which [`resume_is_refused`] decides and [`Menu::build`] appends as
    /// `MF_GRAYED | MF_DISABLED`. The record is here as well as in Windows because the drawing
    /// of task T-11-10 paints the entry itself and has to know which ink to use — and it takes
    /// that from **this** field rather than from the `ODS_GRAYED` of the message, so that
    /// nothing a forged `WM_DRAWITEM` carries decides how our menu looks (SEC-05).
    pub enabled: bool,
}

/// What the letters of FR-101 have to say in the menu right now — the two temporary entries of
/// FR-91's addition.
///
/// A value rather than two arguments, and it carries the **version** rather than a flag because
/// the entry names it: «Доступна версия 0.41.0…».
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pending {
    /// A news item of the feed is unread — FR-101, and the same fact the dot on the icon shows.
    pub unread: bool,
    /// The feed names this version, and it is newer than the one running.
    pub update: Option<String>,
    /// The version running, for a week after an update — the entry «Что нового в X…» (T-119-7).
    pub whats_new: Option<String>,
}

impl Pending {
    /// Whether any entry is due — what decides the rule under them.
    pub fn any(&self) -> bool {
        self.unread || self.update.is_some() || self.whats_new.is_some()
    }
}

/// The context menu of FR-91 — what it holds, line by line; the menu of the shell is described from it.
///
/// Built afresh for every click rather than kept and patched: the first label and the check
/// mark depend on state that can change between two clicks, and a menu built from the state
/// it is about cannot show a stale one.
pub struct Menu {
    /// The command entries in menu order — the builder's own record of what it appended,
    /// and since task T-11-10 the only place the labels exist at all.
    items: Vec<MenuItem>,
    /// The whole menu in order, rules included — what the menu of the shell is described from.
    entries: Vec<MenuEntry>,
}

/// One line of the menu of FR-91: a command or a rule between two groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item(MenuItem),
    Rule,
}

impl Menu {
    /// Builds the menu of FR-91 for the given state.
    ///
    /// The composition is the block of FR-91 read top to bottom: the state item, a rule, the
    /// settings and autostart items, a rule, about and exit. Seven entries in all,
    /// [`MENU_ENTRY_COUNT`].
    ///
    /// # `fail_safe` — task T-13-9, point (б)
    ///
    /// The third argument is `hook::fail_safe()`, handed in by [`show_menu`] rather than read
    /// here, for the reason [`resume_is_refused`] gives. It changes exactly one thing and only
    /// in one state: «Возобновить» is appended `MF_GRAYED | MF_DISABLED`, so
    /// `TrackPopupMenuEx` cannot return `CMD_TOGGLE` at all and the entry is written in
    /// [`theme::Palette::text_muted`] instead of `text`. The **composition** of FR-91 does not
    /// move: seven entries, the same five commands, the same two rules, the same labels. The
    /// entry stays in the menu, greyed rather than removed, because a user looking for the way
    /// back has to see that there is one and that it is not available — a menu that quietly
    /// lost a line would say nothing at all.
    ///
    /// # `dialog_open` — task T-13-14
    ///
    /// The fourth argument is [`settings::dialog_is_open`], handed in by [`show_menu`] for the
    /// same reason the third one is. On its own account it greys **two** entries and only while
    /// the modal dialog of FR-92 is on the screen: «Приостановить/Возобновить» and «Запускать
    /// при входе в систему», the two commands of FR-91 that edit the configuration the dialog
    /// is editing; with the fifth it greys the two doors as well (below). The rule is
    /// [`dialog_locks_command`], where the findings and the reasoning live; this is its visible
    /// half, and [`dispatch_command`] is the half that decides.
    ///
    /// The **composition** of FR-91 does not move here either: seven entries,
    /// [`MENU_ENTRY_COUNT`], the same five commands, the same two rules, the same labels.
    ///
    /// The two greyings compose rather than argue: the first entry is available only when
    /// neither FR-99 nor the open dialog takes it away, which is what the `&&` below says.
    ///
    /// # `about_open` — task T-43-4
    ///
    /// The fifth argument is [`settings::about_is_open`], handed in by [`show_menu`] beside the
    /// fourth. Together they grey the two **doors** — «Настройки…» and «О программе» — while
    /// either modal window is up, because a modal window opened over another is finding С21;
    /// the rule and its reasons are [`dialog_locks_command`]. «Выход» and «Написать автору…» stay
    /// available under both.
    pub fn build(
        enabled: bool,
        autostart: bool,
        fail_safe: bool,
        dialog_open: bool,
        about_open: bool,
        pending: &Pending,
    ) -> Self {
        Self::build_in(
            settings::ui_language(),
            enabled,
            autostart,
            fail_safe,
            dialog_open,
            about_open,
            pending,
        )
    }

    /// The same menu with its labels in `language` — the instrument of fitting goes over every
    /// language without touching the language of the interface.
    pub fn build_in(
        language: settings::Language,
        enabled: bool,
        autostart: bool,
        fail_safe: bool,
        dialog_open: bool,
        about_open: bool,
        pending: &Pending,
    ) -> Self {
        let mut menu = Self {
            items: Vec::with_capacity(8),
            entries: Vec::with_capacity(12),
        };

        // FR-91, task Т-32-4 — the two temporary entries of FR-101, in the top group and above
        // the first rule. They are here **only while there is a reason for them**: an unread
        // news item and a version newer than this one. Nothing else in this menu appears and
        // disappears, and that is deliberate — a person who has read the letter should not go
        // on being told there is one.
        if pending.unread {
            menu.append_command(
                CMD_UNREAD,
                &settings::text_in(language, settings::IDS_MENU_UNREAD),
                false,
                true,
            );
        }

        if let Some(version) = pending.update.as_deref() {
            menu.append_command(
                CMD_UPDATE,
                &settings::text_in(language, settings::IDS_MENU_UPDATE).replace("{0}", version),
                false,
                true,
            );
        }

        // Task T-119-7: a week after any update.
        if let Some(version) = pending.whats_new.as_deref() {
            menu.append_command(
                CMD_WHATSNEW,
                &settings::text_in(language, settings::IDS_MENU_WHATSNEW).replace("{0}", version),
                false,
                true,
            );
        }

        if pending.any() {
            menu.append_separator();
        }

        // FR-94: every label out of the string table of the locale in force. The menu is built
        // afresh on every click, so it carries the locale published at start-up without any
        // state of its own.
        let first = settings::text_in(
            language,
            if enabled {
                settings::IDS_MENU_SUSPEND
            } else if resume_is_refused(enabled, fail_safe) {
                // Task T-34-5, finding Н84: the entry is grey, and the label is the one carrier of
                // the reason — this menu has no tooltips (measured for the stage: not one `TTM_` in
                // this file), so «Возобновить» alone would be a grey word with no explanation.
                settings::IDS_MENU_RESUME_RESTART
            } else {
                settings::IDS_MENU_RESUME
            },
        );

        // Tasks T-13-9, T-13-14 and T-43-4: the four entries of FR-91 whose availability is not
        // constant. The first is taken away by either of two rules and needs both to be
        // silent; the other three by one — the same one, asked with both windows.
        let toggle_available = !resume_is_refused(enabled, fail_safe)
            && !dialog_locks_command(CMD_TOGGLE, dialog_open, about_open);
        let settings_available = !dialog_locks_command(CMD_SETTINGS, dialog_open, about_open);
        let autostart_available = !dialog_locks_command(CMD_AUTOSTART, dialog_open, about_open);
        let about_available = !dialog_locks_command(CMD_ABOUT, dialog_open, about_open);

        menu.append_command(CMD_TOGGLE, &first, false, toggle_available);
        menu.append_separator();
        menu.append_command(
            CMD_SETTINGS,
            &settings::text_in(language, settings::IDS_MENU_SETTINGS),
            false,
            settings_available,
        );
        menu.append_command(
            CMD_AUTOSTART,
            &settings::text_in(language, settings::IDS_MENU_AUTOSTART),
            autostart,
            autostart_available,
        );
        menu.append_separator();
        // FR-91's addition, task Т-32-4, решение 101 п. 9: the permanent entry stands **above**
        // «О программе», which is where the accepted mock-up puts it.
        menu.append_command(
            CMD_WRITE,
            &settings::text_in(language, settings::IDS_WRITE_TO_AUTHOR),
            false,
            true,
        );
        menu.append_command(
            CMD_ABOUT,
            &settings::text_in(language, settings::IDS_MENU_ABOUT),
            false,
            about_available,
        );
        menu.append_command(
            CMD_EXIT,
            &settings::text_in(language, settings::IDS_MENU_EXIT),
            false,
            true,
        );

        menu
    }

    /// The command entries in menu order, labels and check marks included.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// The whole menu in order, rules included — what the menu of the shell is described from.
    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    /// Appends one command entry. `enabled` false: the entry is shown unavailable and answers no
    /// press — the menu of the shell reads it, and [`dispatch_command`] asks the same rule again.
    fn append_command(&mut self, command: u32, label: &str, checked: bool, enabled: bool) {
        let item = MenuItem {
            command,
            label: label.to_owned(),
            checked,
            enabled,
        };
        self.entries.push(MenuEntry::Item(item.clone()));
        self.items.push(item);
    }

    /// Appends a rule between two groups.
    fn append_separator(&mut self) {
        self.entries.push(MenuEntry::Rule);
    }
}

/// What the menu of the shell shows now — task T-120-15: the menu of FR-91 and the line of the
/// state, read from the tray of this thread.
pub struct MenuNow {
    pub menu: Menu,
    /// Processing is on (FR-90).
    pub enabled: bool,
    /// FR-99 holds: processing cannot be resumed without a restart.
    pub fail_safe: bool,
    /// The hotkey as the help names it, beside «Приостановить» / «Возобновить».
    pub key: String,
    /// The setting «Оформление».
    pub theme: theme::ThemeSetting,
}
/// The configuration in force, out of the tray of this thread — what the window of the settings
/// opens over (task T-120-20). None without a tray.
pub fn config_now() -> Option<Config> {
    with_tray(|tray| tray.config().clone())
}

/// [`MenuNow`] out of the tray of this thread, or none without one.
pub fn menu_now() -> Option<MenuNow> {
    let (enabled, autostart, theme, hotkey) = with_tray(|tray| {
        (
            tray.enabled(),
            tray.autostart(),
            tray.config().general.theme,
            tray.config().hotkey.clone(),
        )
    })?;
    let fail_safe = crate::hook::fail_safe();
    let menu = Menu::build(
        enabled,
        autostart,
        fail_safe,
        settings::dialog_is_open(),
        settings::about_is_open(),
        &pending_now(),
    );

    Some(MenuNow {
        menu,
        enabled,
        fail_safe,
        key: settings::effective_hotkey_name(&hotkey),
        theme,
    })
}

/// The menu has chosen a command — task T-120-15. `WM_APP + 33`, the next free number of the
/// program-wide row (see [`settings::WM_APP_SYSTEM_THEME`] for the row): `+ 32` is the cursor of
/// the list of Windows 10 ([`crate::flags::WM_APP_LIST_CURSOR`]), which the two stages took at once.
///
/// The menu of the shell runs no loop of its own, so the command it chose cannot run inside the
/// menu's own message: «Настройки…» would open a modal window while the host of the menu is
/// borrowed. The menu writes the command into [`MENU_CHOSEN`] as it goes and posts this to the
/// program's window; the command runs here, once the menu is gone — the order of the menu of
/// Windows, which returned its command after it closed.
///
/// SEC-05: the message carries nothing. A forged one finds the record empty and does nothing; the
/// command is a number this thread wrote, and [`dispatch_command`] asks the lock of the windows
/// again.
pub const WM_APP_MENU_CHOSEN: u32 = WM_APP + 33;

/// How soon after the menu went without a choice a press on the icon is the press that closed it:
/// the click on the icon takes the foreground from the menu, the menu closes, and the callback of
/// the icon arrives after it — it must not open the menu again. Longer than a click, shorter than
/// a person's second one.
pub const MENU_REPRESS: Duration = Duration::from_millis(300);

thread_local! {
    /// The command chosen in the menu that has just gone, zero for none.
    static MENU_CHOSEN: Cell<u32> = const { Cell::new(0) };
    /// The menu of the shell while it is up, zero otherwise.
    static MENU_WINDOW: Cell<isize> = const { Cell::new(0) };
    /// When the menu last went without a choice. A monotonic clock: no difference of two ticks of
    /// a counter that turns over (the lesson of Э73) decides anything here.
    static MENU_CLOSED_AT: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Records the menu just made — `tray_menu::open` calls it, and nothing else.
pub fn menu_opened(menu: HWND) {
    MENU_WINDOW.with(|record| record.set(menu.0 as isize));
}

/// The menu is gone, by whatever road: forgets it, and has the command chosen in it, if one was,
/// run on `owner` once this message is over. Gone without a choice and not by Esc — the foreground
/// left it, as a press on the icon takes it — notes when; gone by Esc, the next press on the icon is
/// a new one however soon it comes (finding of the stand of T-120-15).
pub fn menu_closed(owner: HWND, chosen: Option<u32>, escaped: bool) {
    MENU_WINDOW.with(|record| record.set(0));
    MENU_CLOSED_AT.with(|at| at.set((chosen.is_none() && !escaped).then(Instant::now)));

    if let Some(command) = chosen {
        MENU_CHOSEN.with(|record| record.set(command));
        // SAFETY: a posted message with two zeros to the program's own window.
        if let Err(error) =
            unsafe { PostMessageW(Some(owner), WM_APP_MENU_CHOSEN, WPARAM(0), LPARAM(0)) }
        {
            MENU_CHOSEN.with(|record| record.set(0));
            app::report_non_critical("PostMessageW", &error);
        }
    }
}

/// The menu of the shell, while it is up on this thread.
pub fn open_menu_window() -> Option<HWND> {
    let raw = MENU_WINDOW.with(Cell::get);
    (raw != 0).then_some(HWND(raw as *mut std::ffi::c_void))
}

/// Whether a press on the icon at `now` is the press that has just closed the menu (`closed_at`).
pub fn closed_by_this_press(closed_at: Option<Instant>, now: Instant) -> bool {
    closed_at.is_some_and(|at| now.saturating_duration_since(at) <= MENU_REPRESS)
}

/// Runs the command [`menu_closed`] noted — the arm of [`WM_APP_MENU_CHOSEN`].
fn run_chosen() {
    let command = MENU_CHOSEN.with(|record| record.replace(0));
    if command == 0 {
        return;
    }
    if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
        dispatch_command(hwnd, command);
    }
}

/// The face the entries of FR-91 are measured and drawn in — FR-92а, task T-11-21.
///
/// `lfMenuFont` of `SPI_GETNONCLIENTMETRICS`, which is the face a menu of this system is set
/// in, put through [`crate::theme::smoothed_logfont`]: one field changed — the quality —
/// and not a byte else, so the entries keep the system's type face, size, weight and character
/// set and lose only the colour fringe of ClearType. **The metrics do not move.** That is not
/// an assumption: task T-11-20 measured the same substitution on the dialog's own face with
/// `GetTextMetricsW` (eight fields) and `GetTextExtentPoint32W` (eight strings) and found both
/// identical, and the two tests that hold it live in `tests\settings.rs`.
///
/// `None` when the non-client metrics cannot be read (NFR-13) — there is then no menu face to
/// modify, and guessing one would mislabel every measurement made with it.
pub fn menu_item_logfont() -> Option<LOGFONTW> {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: u32::try_from(size_of::<NONCLIENTMETRICSW>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: `metrics` is a live local whose `cbSize` describes it, which is what the call
    // checks before writing into the pointer, and the pointer is not kept.
    // `SPI_GETNONCLIENTMETRICS` reads system state and changes nothing; the zero update-flags
    // ask for no broadcast.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some((&raw mut metrics).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };

    // NFR-13: examined.
    if read.is_err() {
        return None;
    }

    Some(theme::smoothed_logfont(metrics.lfMenuFont))
}
// ---------------------------------------------------------------------------------------
// Where the menu opens, and whether it may take the foreground — task T-41-3, finding Н44
// ---------------------------------------------------------------------------------------

/// Pulls a point onto a work area — task T-41-3, finding Н44.
///
/// Pure, so that the table of `tests\tray.rs` can close it: the awkward cases are not the middle
/// of the screen but the edges, the far negative numbers a forged message names, and
/// [`i32::MAX`].
///
/// ⚠ A degenerate rectangle — one whose right edge is left of its left, which a display driver
/// coming back from sleep has been known to answer with — leaves the point alone rather than
/// панicking: `i32::clamp` requires `min <= max` and is documented to panic otherwise, and a
/// menu in the wrong place is a far better outcome than a process that ends.
pub fn clamp_to_work_area(point: (i32, i32), area: RECT) -> (i32, i32) {
    let pull = |value: i32, low: i32, high: i32| {
        if high < low {
            value
        } else {
            value.clamp(low, high)
        }
    };

    (
        pull(point.0, area.left, area.right),
        pull(point.1, area.top, area.bottom),
    )
}

/// The work area — the screen less the task bar — of the monitor nearest `point`.
///
/// `MONITOR_DEFAULTTONEAREST` answers a monitor for **every** point, including one far off every
/// screen, which is exactly the case this function exists for.
///
/// ⚠ **Not `GetSystemMetrics`, and that is the lesson of stage Э42:** this process declares
/// `PerMonitorV2`, and in such a process the screen metrics answer for 96 DPI whatever the
/// monitor really is, while `GetSystemMetricsForDpi` lives behind a feature this program does not
/// carry. A monitor's own rectangle has no such trouble — it is in physical pixels by
/// definition.
///
/// NFR-13: a refusal answers `None`, is journalled, and leaves the caller with the point it was
/// given.
fn work_area_around(point: (i32, i32)) -> Option<RECT> {
    // SAFETY: reads a system table, keeps nothing of ours, and is documented to answer a handle
    // for every point under `MONITOR_DEFAULTTONEAREST`.
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: point.0,
                y: point.1,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };

    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: the handle came from the call above; `info` is a live local of this frame whose
    // `cbSize` the call reads first, as it documents, and it is the only memory touched.
    if !unsafe { GetMonitorInfoW(monitor, &raw mut info) }.as_bool() {
        app::report_non_critical("GetMonitorInfoW", &WinError::from_thread());
        return None;
    }

    Some(info.rcWork)
}

/// The point the menu of FR-91 is answered with — task T-41-3, finding Н44.
///
/// The coordinates a `WM_APP_TRAY` names, taken back onto the desktop. For a real click they are
/// the cursor's and the pull changes nothing at all — **the ordinary click does not move by a
/// pixel**, which is the requirement this task was given. For a forged message they are whatever
/// the sender wrote, and the menu no longer opens in the void.
fn menu_point(asked: (i32, i32)) -> (i32, i32) {
    work_area_around(asked).map_or(asked, |area| clamp_to_work_area(asked, area))
}

/// Whose point the menu opens at — task T-41-3, finding Н44. Pure, and a table closes it.
///
/// `asked` is what the message named, already pulled onto the desktop by [`menu_point`].
/// `cursor` is what the system answered, or `None` if it would not answer.
///
/// The cursor wins whenever there is one and the showing came from the mouse: a message can name
/// any coordinates its sender likes, and where the pointer is is not a thing it can lie about.
/// **It is not a change for an ordinary click** — a real one puts the cursor exactly where the
/// shell's own point is.
///
/// From the keyboard the message's point wins instead, and that is not a concession: a person
/// pressing Enter on the icon may have left the pointer on another monitor entirely, and the
/// shell's point is the icon's — which is where they are looking.
pub fn menu_point_of(
    asked: (i32, i32),
    cursor: Option<(i32, i32)>,
    from_the_keyboard: bool,
) -> (i32, i32) {
    if from_the_keyboard {
        asked
    } else {
        cursor.unwrap_or(asked)
    }
}

/// The cursor, if the system will say where it is — task T-41-3.
///
/// NFR-13: a refusal is journalled and answers `None`, and the caller falls back on the point the
/// message named.
fn cursor_point() -> Option<(i32, i32)> {
    let mut point = POINT::default();

    // SAFETY: `point` is a live local of this frame and the only memory the call writes.
    if let Err(error) = unsafe { GetCursorPos(&raw mut point) } {
        app::report_non_critical("GetCursorPos", &error);
        return None;
    }

    Some((point.x, point.y))
}

/// Shows the menu of FR-91 at a point on the screen — a window of the shell since task T-120-15
/// (`ui::windows::tray_menu`). What is chosen in it runs once it has gone ([`WM_APP_MENU_CHOSEN`]).
///
/// # One menu at a time, and the second press closes it
///
/// While the menu is up a press on the icon closes it and opens nothing — the menu of Windows did
/// the same. A press that has just closed it by taking the foreground away arrives after it went;
/// it is recognised by how soon it came ([`closed_by_this_press`]) and opens nothing either.
///
/// # Whose point — task T-41-3, finding Н44
///
/// `x` and `y` are what the message asked for, already pulled onto the desktop by [`menu_point`].
/// For a showing that came from the **mouse** they are overruled by [`cursor_point`] — the place
/// the person really clicked — and a forged message cannot put the menu anywhere else
/// ([`menu_point_of`]). From the keyboard (the menu key, Shift+F10 on the icon) the first
/// available item is marked.
fn show_menu(x: i32, y: i32, from_the_keyboard: bool) {
    if let Some(open) = open_menu_window() {
        crate::ui::host::close(open);
        return;
    }
    if closed_by_this_press(MENU_CLOSED_AT.with(Cell::get), Instant::now()) {
        return;
    }

    let (x, y) = menu_point_of((x, y), cursor_point(), from_the_keyboard);

    let Some(hwnd) = with_tray(|tray| tray.hwnd) else {
        return;
    };

    let _ = crate::ui::windows::tray_menu::open(hwnd, POINT { x, y }, from_the_keyboard);
}

/// Carries out one menu command.
///
/// Reached from [`show_menu`], that is, from a menu the user opened; from the buttons of the
/// letters (`letters::perform`, «Открыть настройки»); and — since task T-43-4 — from the double
/// click on the icon, which until then opened the settings dialog past this door. Of the three,
/// only the last two can be set off by a message another process forges, and both only ever ask
/// for [`CMD_SETTINGS`] — the window the module documentation's SEC-05 already names as the one
/// thing a forged double click achieves. No command number is ever taken out of a message.
///
/// # The gate of tasks T-13-14 and T-43-4 — and why it is here as well as in the menu
///
/// While the modal dialog of FR-92 is on the screen, [`CMD_TOGGLE`] and [`CMD_AUTOSTART`] are
/// refused and **nothing at all happens**: no change in memory, no icon, no file, no registry.
/// While it or the «О программе» window is up, [`CMD_SETTINGS`] and [`CMD_ABOUT`] are refused
/// the same way — no second modal window over the first (finding С21). [`Menu::build`] greys the
/// same entries, and that is not a duplicate of this line but its other half — greying is what
/// the user sees and what keeps `TrackPopupMenuEx` from returning the command, and *this* is
/// what decides. A menu is user interface; a command handler is the door, and the door is where
/// a lock belongs. The two read one rule, [`dialog_locks_command`], so they cannot come apart.
///
/// **Public for the tests of task T-13-14**, which measure the refusal the way its acceptance
/// asks for — by the value in memory and by the bytes on the disk, not by an intention read out
/// of the source. That widens no attack surface: SEC-05 is about messages another process can
/// forge, and this is a Rust function of our own library that no message reaches.
///
/// # Nothing is journalled here, deliberately
///
/// Unlike the refusal of task T-13-9 one door further in, which records
/// [`RESUME_REFUSED_IN_FAIL_SAFE`]. The vocabulary of `src\diag.rs` is closed and this task
/// does not open it: a name that is not a row of that table comes back
/// [`diag::Operation::UNLISTED`] and carries nothing, so an event invented here would be an
/// event that says nothing. The refusal is also not an anomaly worth a dump — it is the
/// ordinary answer to a click on an entry that is greyed on the screen at that very moment.
pub fn dispatch_command(hwnd: HWND, command: u32) {
    // Tasks T-13-14 and T-43-4. One door in front of the whole table rather than a check inside
    // four of the arms: a gate at the top cannot be walked round by an arm added later, and it
    // is the shape that makes «which commands are locked» a property of the rule and of nothing
    // else. The commands the rule does not name — «Выход» and the letters' entries — reach their
    // arms exactly as before.
    let dialog_open = settings::dialog_is_open();
    let about_open = settings::about_is_open();

    if dialog_locks_command(command, dialog_open, about_open) {
        return;
    }

    match command {
        // FR-90. Task T-13-9: while FR-99 holds, this arm is unreachable — the entry was
        // appended `MF_GRAYED | MF_DISABLED`, so `TrackPopupMenuEx` answered zero rather than
        // this command — and [`Tray::toggle_state`] refuses the resumption a second time
        // whatever brought it here.
        CMD_TOGGLE => {
            let _ = with_tray(Tray::toggle_state);
        }

        // FR-92, task T-08-1. Outside any borrow of the tray, like the about box below and for
        // the same reason: the dialog is modal and pumps messages.
        CMD_SETTINGS => open_settings(hwnd),

        // FR-93, task T-08-1.
        #[cfg(not(feature = "testing"))]
        CMD_AUTOSTART => toggle_autostart(),
        #[cfg(feature = "testing")]
        CMD_AUTOSTART => toggle_autostart_for_tests(),

        // Outside any borrow of the tray, like the letters below: the window of FR-92а is made
        // while the arm runs.
        CMD_ABOUT => show_about(hwnd),

        // FR-101 and FR-103, task Т-32-4 — the three entries of the letters from the author.
        // Every one of them opens a **modeless** window, which returns at once and holds no
        // borrow of anything: unlike the two arms above, these do not pump a message loop of
        // their own.
        //
        // ⚠ The two entries part company in stage В, and that is what the two numbers were
        // kept apart for: «Написать автору…» opens the wizard of FR-104, «От автора» opens the
        // window of FR-103. Until Т-32-8 both went to the second one.
        CMD_WRITE => crate::letters::open_wizard(hwnd),
        CMD_AUTHOR => crate::letters::open_author(hwnd),

        // The two temporary entries: the unread letter and the newer version. Both are present
        // in the menu only while there is a reason for them (FR-91), and both open the letter
        // they are about.
        CMD_UNREAD | CMD_UPDATE => show_pending_letter(hwnd, command == CMD_UPDATE),

        // Task T-119-7: the window «Обновлено» once more if it has not been seen, else the page of
        // the changes on the site.
        CMD_WHATSNEW => crate::corner::whats_new(),

        // The ordinary way out. Nothing is cleaned up here: asking `app` to come down leads
        // the UI thread out of its message loop and into the one cleanup path of FR-83,
        // `Tray::shut_down`, by way of `Attachment::drop`. The same route the FR-97 timeout
        // takes.
        CMD_EXIT => app::request_shutdown(),

        // `TrackPopupMenuEx` returns zero when the menu was dismissed, and nothing else can
        // turn up here: the identifiers are ours and the menu is built two frames up.
        _ => {}
    }
}

/// The identifier of the timer the schedule of FR-101 runs on — the UI window's own, and
/// distinct from the watchdog's, which lives on the input window.
const LETTERS_TIMER: usize = 2;

/// How long after the start the first check of the letters happens — NFR-08, «ничего не делает
/// в первые секунды».
const LETTERS_FIRST_MS: u32 = 90_000;

/// And how often after that — NFR-10. Once an hour is as rare as a schedule counted in days can
/// afford to be, and the whole of what it does when nothing is due is compare four dates.
const LETTERS_EVERY_MS: u32 = 60 * 60 * 1000;

/// Arms the clock of FR-101 on the UI window — called once, when the tray is installed.
///
/// NFR-13: a refused timer costs the letters, not the program. It is journaled and the program
/// runs on without them.
pub fn start_letters_clock(hwnd: HWND) {
    // SAFETY: `hwnd` is the live UI window; `None` for the callback asks for `WM_TIMER` at that
    // window, which is what `handle_ui_message` answers.
    let started = unsafe { SetTimer(Some(hwnd), LETTERS_TIMER, LETTERS_FIRST_MS, None) };

    if started == 0 {
        app::report_non_critical("SetTimer", &WinError::from_thread());
    }
}

/// One tick of the schedule of FR-101 — the whole of what the letters do on their own.
///
/// Three steps and in this order, and none of them holds a borrow of the tray while the next
/// runs:
///
/// 1. the first tick re-arms the clock at the hourly period — the ninety seconds of NFR-08 are
///    a delay before the first check, not a period;
/// 2. the state is initialised if it has never been (the day this installation started
///    counting), and the file is written if that changed anything;
/// 3. `letters::due` is asked what is due, and if anything is, it is shown — announced by a
///    balloon first for every letter but «Привет» (FR-101).
fn on_letters_tick() {
    let Some(hwnd) = with_tray(|tray| tray.hwnd) else {
        return;
    };

    // The period after the first tick. `SetTimer` with an identifier that already exists
    // **replaces** the timer rather than making a second one, which is the documented way to
    // change a period and the reason this needs no state of its own.
    //
    // SAFETY: `hwnd` is the live UI window and the identifier is this module's own.
    let restarted = unsafe { SetTimer(Some(hwnd), LETTERS_TIMER, LETTERS_EVERY_MS, None) };

    if restarted == 0 {
        app::report_non_critical("SetTimer", &WinError::from_thread());
    }

    crate::letters::tick(hwnd);
}

/// What the letters of FR-101 have to say in the menu right now.
///
/// Read **outside** any borrow of the tray, because `letters::pending` takes one of its own to
/// read the configuration.
fn pending_now() -> Pending {
    let (unread, update) = crate::letters::pending();
    let state = crate::letters::state_now();
    let whats_new = crate::corner::whats_new_stands(state.updated_on, crate::letters::today())
        .then(crate::letters::version_string);

    Pending {
        unread,
        update,
        whats_new,
    }
}

/// Shows the letter one of the two temporary menu entries is about — FR-91, task Т-32-4.
fn show_pending_letter(hwnd: HWND, update: bool) {
    crate::letters::show_pending(hwnd, update);
}

/// Announces a letter with the balloon of the icon — FR-101, task Т-32-4. The public half of
/// [`Tray::announce`].
pub fn announce_letter(title: &str, body: &str) {
    with_tray(|tray| tray.announce(title, body));
}

/// Puts the dot of FR-90 on the icon, or takes it off — task Т-32-4.
///
/// Called when the answer may have moved: a news item was marked read, a read of the feed
/// brought new ones or dropped old ones. The icon is refreshed unconditionally, which costs one
/// `NIM_MODIFY` and cannot be wrong; deciding whether it moved would need a second copy of the
/// rule.
pub fn refresh_unread_mark() {
    with_tray(Tray::refresh_state_icon);
}

// ---------------------------------------------------------------------------------------
// The settings dialog — FR-92, and the autostart of FR-93
// ---------------------------------------------------------------------------------------

/// Opens the window of the settings of FR-92 — a window of the shell since task T-120-20
/// (`ui::windows::settings`), owned by the program's window `hwnd`. It is not modal and has no
/// «Применить»: every change it makes is put in force at once through [`apply_settings_now`]. One
/// at a time — while it or «О программе» is up the door turns round, and the menu greys the entry
/// ([`dialog_locks_command`]).
fn open_settings(hwnd: HWND) {
    let _ = crate::ui::windows::settings::open(hwnd);
}

/// Takes a configuration the dialog produced and makes it the program's — the «Применить» of
/// FR-92.
///
/// **Three destinations, and all three are the requirement.** The tray holds the configuration
/// in memory and writes the file (section 6.1: the UI thread is the one that may); `app`
/// publishes it into the modules that act on it, which is what makes a changed setting take
/// effect without a restart (rule R-52); and the registry is made to agree with
/// `general.autostart`, which is FR-93 and is the one part of the configuration that lives
/// outside the file.
///
/// The registry is the one destination of the three that can answer no, and what is done with
/// that answer is [`apply_settings_via`], one function down.
/// Puts `config` in force the way «Применить» did, and answers whether the file of the
/// configuration took it — the window of the settings applies every change at once by this
/// (task T-120-20) and steps the change back when the file refused it. None without a tray.
pub fn apply_settings_now(config: &Config) -> Option<bool> {
    with_tray(|_| ())?;
    apply_settings(config);
    with_tray(|tray| tray.config_saved())
}

fn apply_settings(config: &Config) {
    let flag_was_on = with_tray(|tray| tray.flag_on()).unwrap_or(false);

    apply_settings_in_force(config);

    // Task T-119-5: the flag was just turned on — once per turning on, and only while it stands
    // under ^, its hint comes up by the notification area.
    if !flag_was_on
        && config.general.tray_flag
        && let Some(hwnd) = with_tray(|tray| tray.hwnd)
    {
        crate::corner::show_flag_hint(hwnd, TRAY_ICON_ID);
    }
}

/// The same, with the registry half of FR-93 handed in — task T-13-24.
///
/// # The finding
///
/// The audit of 2026-08-24 read the comment in the body below — «FR-93 first: if the registry
/// refuses, the file is not made to claim otherwise» — and then the line under it. There was
/// no early return and nothing else in its place, so [`Tray::replace_config`] went on to save
/// the very `general.autostart` the registry had just refused. From that moment the file, the
/// check mark of FR-91 and the dialog of FR-92 all claimed an autostart the system does not
/// have, and at the next logon nothing started. The invariant is this module's own and is
/// written out at [`Tray::set_autostart`]: a file that claims the program starts with the
/// session while the registry says otherwise is the one outcome worth avoiding.
///
/// # One field steps back — the operation does not
///
/// A bare `return` is the obvious repair and the wrong one. «Применить» carries *everything*
/// the user changed in the dialog — the hotkey, the layouts, the exclusions, the language, the
/// theme, the timings — and a refusal from the `Run` key of FR-93 says nothing whatever about
/// any of them. Abandoning the whole operation because one destination of three answered no
/// would trade a small lie for a large silent loss, and the user would have no way to tell
/// that the rest of the dialog had been thrown away.
///
/// So exactly one field steps back. `general.autostart` returns to the value the program is
/// living by, read out of the tray **before** anything is written, because
/// [`Tray::replace_config`] is what overwrites it; every other field is stored and published
/// exactly as the user left it. Both halves of what the file then says are true: the autostart
/// the registry really holds, and every other setting that was asked for.
///
/// The twin [`toggle_autostart`] does return on the same refusal, and the two do not disagree:
/// that path carries *only* the autostart, so skipping its one field and giving up its whole
/// operation are the same act.
///
/// # The disagreement is still shown
///
/// Nothing here hides anything. The refusal goes into the journal through
/// [`crate::app::report_non_critical`] — the one event that was already there, not a second
/// one — and the dialog goes on showing the registry's own answer, [`settings::autostart_registered`],
/// beside the state of the file: the two are read separately on purpose, so that a
/// disagreement is visible rather than hidden. What changes is only that the **file** stops
/// claiming what is not so.
///
/// # And the save policy of task T-13-6 — where the two meet since task T-55-2
///
/// For the **file** they still decide different questions. This function decides **what** is
/// stored; [`Tray::save_config`] decides **whether** the file may be written at all, on its own
/// facts — the outcome of the one read this program performs — and it decides that afterwards and
/// underneath [`Tray::replace_config`].
///
/// For the **registry** they meet once, in front of the write: the one rule of решение 120.4 (ж),
/// [`SavePolicy::lets_the_run_key_follow`] at [`settings::RunKeyMoment::Request`], shared with the
/// start of the program and with [`toggle_autostart_via`]. Before that task this door wrote the
/// `Run` key whatever the file was, and under [`SavePolicy::Forbidden`] — a file from a newer build,
/// not written this session — «Применить» made the registry and that file disagree for good. Now
/// the registry is not asked at all there, and `general.autostart` steps back exactly as it does on
/// a refusal: every other change of the dialog is still taken into memory, and a change of
/// autostart the person asked for is withheld and says so in the journal
/// ([`AUTOSTART_CHANGE_SUPPRESSED`]). Under [`SavePolicy::QuarantineFirst`] the registry **does**
/// follow: the person is choosing now, and their choice is what the file will carry once the
/// unreadable bytes are moved aside. That is the one difference from the start of the program,
/// where the same policy leaves the registry alone — the configuration then is defaults nobody
/// chose.
///
/// # Why the registry write is an argument
///
/// The same shape and the same reason as [`resume_is_refused`] and [`dialog_locks_command`]:
/// the refusing branch is the whole point of this function, and no test may drive it by making
/// the real `HKCU\…\CurrentVersion\Run` refuse — that key belongs to whoever is running the
/// tests, and FR-93 gives this program exactly one value in it. `tests\tray.rs` hands in a
/// closure that answers `Err`; the product hands in [`settings::set_autostart`] and nothing
/// else does.
pub fn apply_settings_via(config: &Config, write_run_key: impl FnOnce(bool) -> WinResult<()>) {
    // Read before anything moves, because `replace_config` at the foot of this function is what
    // overwrites it: `general.autostart` as the program is living by it right now — the value
    // the file already carries and the value the `Run` key was last made to agree with. `None`
    // is a thread with no tray of its own, and then there is nothing to store and nothing to
    // keep.
    let in_force = with_tray(|tray| tray.autostart());

    // Решение 120.4 (ж), task T-55-2: the one rule, asked on request — see the section on the save
    // policy above. A thread with no tray of its own follows, as this door always did.
    let may_follow = with_tray(|tray| {
        tray.save_policy()
            .lets_the_run_key_follow(settings::RunKeyMoment::Request)
    })
    .unwrap_or(true);

    let mut stored = config.clone();

    // FR-93 first: if the registry refuses, the file is not made to claim otherwise.
    let steps_back = if !may_follow {
        // Not asked at all. A change the person did ask for is withheld, and the journal is told.
        if in_force.is_some_and(|in_force| in_force != config.general.autostart) {
            note_configuration(AUTOSTART_CHANGE_SUPPRESSED);
        }

        true
    } else if let Err(error) = write_run_key(config.general.autostart) {
        // NFR-13: the refusal is used, not swallowed. It is reported, and it is the reason the
        // one field below steps back to what is true.
        app::report_non_critical("RegSetValueExW", &error);

        true
    } else {
        false
    };

    if steps_back && let Some(in_force) = in_force {
        stored.general.autostart = in_force;
    }

    with_tray(|tray| tray.replace_config(stored.clone()));

    // FR-94, решение 99, task Т-31-1: the interface locale of the **running** process follows
    // the configuration that has just been stored and written. Not a field like the others —
    // it is published into a module rather than acted on by one — which is why it stands here
    // beside `publish_configuration` and not inside it, and why the start-up publication of
    // `app` goes through the very same body. One body, not two.
    adopt_ui_language();

    app::publish_configuration(&stored);
}

/// Publishes the interface locale of the configuration the tray is living by — FR-94,
/// решение 99, task Т-31-1.
///
/// **The one place a running process learns what language it speaks**, and the whole of what
/// «без перезапуска» means in this program: `settings::text` reads the string table afresh on
/// every call, the menu of FR-91 is built afresh on every showing, and both windows are built
/// afresh on every opening — so the moment this atomic moves, everything opened afterwards is
/// in the new locale. What is already on the screen is the business of [`crate::settings`]:
/// the open settings window relabels or reopens itself (task Т-31-2), and the icon's tooltip
/// is brought along here.
///
/// **Why it reads the tray instead of taking an argument.** Section 6.1 makes the tray the
/// in-memory owner of the configuration, and both callers stand immediately after that owner
/// has been given the value in force — [`apply_settings_via`] after `replace_config`, `app`
/// after `attach`. Reading the owner rather than a copy is what makes it impossible for the
/// published locale and the stored one to disagree.
///
/// `None` — a thread with no tray — publishes nothing, which is the right answer on the input
/// and watcher threads: the locale belongs to the UI thread's configuration (section 6.1).
pub fn adopt_ui_language() {
    let Some(language) = with_tray(|tray| tray.config().general.language) else {
        return;
    };

    // Stage E108: a value of the closed set the build does not carry speaks English
    // (`Language::in_force`); the file keeps its word.
    settings::set_ui_language(language.in_force());

    // FR-90, решение 99.2: the tooltip is «Lang Switcher — {состояние}», and the state word is
    // a string of the locale. `NIM_MODIFY` in a second borrow, after the first has ended —
    // the discipline of this module, and cheap: the shell is told the icon it already has.
    with_tray(|tray| tray.refresh_icon());

    // FR-101, task Т-32-3: a letter or «От автора» that is open right now follows the language
    // too, by the two mechanisms of решение 99.1 — refilled where the direction of the script
    // is the same, rebuilt where it changed. Outside every borrow of the tray: rebuilding a
    // window reads the configuration through `with_tray` of its own.
    if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
        crate::letters::language_changed(hwnd);
    }
}

/// The check mark of FR-91, made to do what FR-93 says.
///
/// The registry is written **first** and the file only if that succeeded: the check mark
/// describes what the system will do at the next logon, and a file that disagrees with the
/// system would make the tray show a state that is not true.
fn toggle_autostart() {
    toggle_autostart_via(settings::set_autostart);
}

/// The same under the `testing` feature: the writer a test put in for its thread, else the registry.
#[cfg(feature = "testing")]
fn toggle_autostart_for_tests() {
    match testing::run_key_writer() {
        Some(write) => toggle_autostart_via(write),
        None => toggle_autostart(),
    }
}

/// «Применить» and the window of the settings put a configuration in force with the real write
/// of the `Run` key of FR-93 in hand.
#[cfg(not(feature = "testing"))]
fn apply_settings_in_force(config: &Config) {
    apply_settings_via(config, settings::set_autostart);
}

/// The same under the `testing` feature: the writer a test put in for its thread, else the registry
/// (the stands run the `testing` image as the product).
#[cfg(feature = "testing")]
fn apply_settings_in_force(config: &Config) {
    match testing::run_key_writer() {
        Some(write) => apply_settings_via(config, write),
        None => apply_settings_via(config, settings::set_autostart),
    }
}

/// What the tests put in the place of the write of the `Run` key — finding of task T-120-20: the
/// window of the settings puts every change in force at once, and a test of it that went the
/// product's road wrote the `Run` key of the machine the tests ran on with the path of the test
/// image. A thread that attached a tray for a test hands in its own writer here, and no test
/// touches that key.
#[cfg(feature = "testing")]
pub mod testing {
    use super::WinResult;
    use std::cell::Cell;

    /// A writer of the `Run` key: the value asked for, and whether it was written.
    pub type RunKeyWriter = fn(bool) -> WinResult<()>;

    thread_local! {
        static RUN_KEY: Cell<Option<RunKeyWriter>> = const { Cell::new(None) };
    }

    /// The `Run` key of this thread is written through `write` from now on.
    pub fn write_run_key_with(write: RunKeyWriter) {
        RUN_KEY.with(|cell| cell.set(Some(write)));
    }

    pub(super) fn run_key_writer() -> Option<RunKeyWriter> {
        RUN_KEY.with(Cell::get)
    }
}

/// The same, with the registry write handed in — task T-55-2.
///
/// # The second door, and the same rule
///
/// Before that task the menu entry wrote the `Run` key whatever the session's file was, and a
/// session on a file from a newer schema — a file not written at all this session — made the
/// registry and that file disagree for good at one click. The entry now asks the one rule of
/// решение 120.4 (ж), [`SavePolicy::lets_the_run_key_follow`] at
/// [`settings::RunKeyMoment::Request`], exactly as «Применить» does in [`apply_settings_via`]: under
/// [`SavePolicy::Forbidden`] the registry is not asked, the check mark stays where it was, and the
/// refusal is one line of the journal ([`AUTOSTART_CHANGE_SUPPRESSED`]) — never silence.
///
/// # Why the registry write is an argument
///
/// The reason [`apply_settings_via`] gives: no test may drive this door through the real
/// `HKCU\…\CurrentVersion\Run`. `tests\tray.rs` hands in a closure that counts; the menu hands in
/// [`settings::set_autostart`] through [`toggle_autostart`], and nothing else does.
pub fn toggle_autostart_via(write_run_key: impl FnOnce(bool) -> WinResult<()>) {
    let Some((wanted, may_follow)) = with_tray(|tray| {
        (
            !tray.autostart(),
            tray.save_policy()
                .lets_the_run_key_follow(settings::RunKeyMoment::Request),
        )
    }) else {
        return;
    };

    if !may_follow {
        // Решение 120.4 (ж): the file is not written this session, so neither is the registry.
        note_configuration(AUTOSTART_CHANGE_SUPPRESSED);
        return;
    }

    if let Err(error) = write_run_key(wanted) {
        // NFR-13. Nothing is recorded anywhere: the state the user asked for was not reached,
        // and the check mark stays where it was.
        app::report_non_critical("RegSetValueExW", &error);
        return;
    }

    with_tray(|tray| tray.set_autostart(wanted));
}

// ---------------------------------------------------------------------------------------
// The about box
// ---------------------------------------------------------------------------------------

/// Shows the «О программе» window of FR-92а — the window of the shell (`ui::windows::about`),
/// owned by the program's window `hwnd`. It is not modal: it returns at once, and the menu stays
/// locked against a second copy and the settings dialog for as long as it is up
/// ([`dialog_locks_command`] reads [`settings::about_is_open`]).
///
/// The theme setting and the hotkey are *copied* out of the tray in one borrow that ends before
/// the window is made: the help names the key, modifiers included (task T-93-1).
fn show_about(hwnd: HWND) {
    let Some((setting, key)) =
        with_tray(|tray| (tray.config().general.theme, tray.config().hotkey.clone()))
    else {
        return;
    };

    let _ = crate::ui::windows::about::open(hwnd, setting, &key);
}

/// The four-part file version of the running executable, from its `VERSIONINFO` resource.
///
/// Read from the resource rather than from `CARGO_PKG_VERSION` because that is what the about
/// box is supposed to show: `app.rc` is where the version of the shipped file is declared,
/// and a second copy of the number in the Rust source is a second place for it to be wrong.
/// `None` when the resource is absent or is not a version resource — which is what a binary
/// built without `app.rc` looks like, and is not a failure.
pub fn file_version() -> Option<(u16, u16, u16, u16)> {
    parse_fixed_file_version(version_resource()?)
}

/// The bytes of the `VS_VERSIONINFO` resource of the running executable.
fn version_resource() -> Option<&'static [u8]> {
    // SAFETY: `None` asks for the handle of the file used to create the calling process,
    // which is the running executable and cannot be unloaded under us. The handle is borrowed
    // and must not be freed; nothing here frees it.
    let module: HMODULE = unsafe { GetModuleHandleW(PCWSTR::null()) }.ok()?;

    // SAFETY: both "strings" are integer resource identifiers in the `MAKEINTRESOURCE` form
    // — a value below 65536 stored in the pointer, which is how the resource loader is told
    // to look up by identifier rather than by name. `RT_VERSION` is the crate's own constant
    // of exactly that shape, and `1` is the identifier `app.rc` gives the version block.
    // Nothing is dereferenced as a string.
    let found = unsafe { FindResourceW(Some(module), resource_id(1), RT_VERSION) };

    // NFR-13. Not worth reporting: a binary legitimately built without `app.rc` has no
    // version resource, and the about box says so rather than failing.
    if found.0.is_null() {
        return None;
    }

    // SAFETY: `module` and `found` are the pair just established — the resource was found in
    // this very module — which is the precondition of the call. It returns zero for a
    // resource it cannot size, which is checked below.
    let size = unsafe { SizeofResource(Some(module), found) };

    // SAFETY: the same pair, and the same precondition. `LoadResource` on a module of the
    // running process returns a handle to a block inside the mapped image; it is not freed
    // and must not be.
    let block = unsafe { LoadResource(Some(module), found) }.ok()?;

    // SAFETY: `block` came from the successful `LoadResource` directly above.
    let start = unsafe { LockResource(block) };

    if start.is_null() || size == 0 {
        return None;
    }

    let length = usize::try_from(size).ok()?;

    // SAFETY: `start` points at the first byte of a resource of exactly `size` bytes inside
    // the mapped image of this module, so the range is one allocated object, properly aligned
    // for `u8`, fully initialised, and never written to by anybody — resource data of a
    // loaded module is read-only. The lifetime is `'static` because the image of the running
    // executable stays mapped for the life of the process.
    Some(unsafe { std::slice::from_raw_parts(start.cast::<u8>(), length) })
}

/// Reads `dwFileVersionMS` and `dwFileVersionLS` out of a `VS_VERSIONINFO` resource.
///
/// Split out from [`version_resource`], and public, so that the layout arithmetic — the one
/// part of this that can be wrong without any Win32 call failing — can be checked both
/// against a buffer built by hand and against the bytes `rc.exe` really produced. The
/// integration tests do the second: `embed-resource` links `app.rc` into the binary targets
/// of this crate and not into the test executables, so a test cannot read its own version
/// resource and reads the shipped `LangSwitcher.exe` instead.
///
/// Every access is bounds-checked and the signature is verified, so a buffer that is not a
/// version resource yields `None` rather than a number read out of the middle of something
/// else.
pub fn parse_fixed_file_version(resource: &[u8]) -> Option<(u16, u16, u16, u16)> {
    let fixed =
        resource.get(FIXED_FILE_INFO_OFFSET..FIXED_FILE_INFO_OFFSET + FIXED_FILE_INFO_READ)?;

    let field = |at: usize| -> u32 {
        u32::from_le_bytes([fixed[at], fixed[at + 1], fixed[at + 2], fixed[at + 3]])
    };

    if field(0) != FIXED_FILE_INFO_SIGNATURE {
        return None;
    }

    // Each of the two words packs two of the four parts, the higher one first.
    let split = |packed: u32| -> (u16, u16) {
        (
            u16::try_from(packed >> 16).unwrap_or(0),
            u16::try_from(packed & 0xFFFF).unwrap_or(0),
        )
    };

    // `dwFileVersionMS` is at offset 8 of the structure and `dwFileVersionLS` at 12.
    let (major, minor) = split(field(8));
    let (build, revision) = split(field(12));

    Some((major, minor, build, revision))
}

// ---------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------

/// An icon loaded from our resources, destroyed when this value is dropped.
struct Icon {
    handle: HICON,
}

impl Icon {
    /// Loads icon `id` at `size` from the resources of `instance` — FR-90.
    fn load(instance: HINSTANCE, id: u16, size: (i32, i32)) -> WinResult<Self> {
        // SAFETY: `instance` is the module handle of this process, whose resources carry the
        // identifier asked for; the "name" is an integer identifier in `MAKEINTRESOURCE`
        // form, so nothing is dereferenced as a string. `LR_DEFAULTCOLOR` and the absence of
        // `LR_SHARED` mean the call creates a handle this value now owns and has to destroy,
        // which `Drop` does exactly once. The crate already turns a null result into an
        // error, so NFR-13 is satisfied by the `?`.
        let handle = unsafe {
            LoadImageW(
                Some(instance),
                resource_id(id),
                IMAGE_ICON,
                size.0,
                size.1,
                LR_DEFAULTCOLOR,
            )?
        };

        Ok(Self {
            handle: HICON(handle.0),
        })
    }
}

/// The four state icons at [`large_icon_size`] — the balloon's set, задача Т-33а-1.
///
/// A type of its own so that the whole set is either there or absent: a balloon that showed the
/// large «active» icon and the small «paused» one would be worse than one that showed neither.
struct BalloonIcons {
    /// The "active" icon, large.
    active: Icon,
    /// The "suspended" icon, large.
    paused: Icon,
    /// The "active, and a letter is unread" icon, large.
    active_unread: Icon,
    /// The "suspended, and a letter is unread" icon, large.
    paused_unread: Icon,
}

impl BalloonIcons {
    /// Loads all four, or none — NFR-13.
    ///
    /// The failure is journaled once and is not an error of the tray: everything the tray does
    /// works without these, and only the picture in the balloon is poorer for it.
    fn load(hwnd: Option<HWND>, instance: HINSTANCE) -> Option<Self> {
        let size = large_icon_size(hwnd);

        let load = |id: u16| match Icon::load(instance, id, size) {
            Ok(icon) => Some(icon),
            Err(error) => {
                app::report_non_critical("LoadImageW(SM_CXICON)", &error);
                None
            }
        };

        Some(Self {
            active: load(IDI_APP_ACTIVE)?,
            paused: load(IDI_APP_PAUSED)?,
            active_unread: load(IDI_APP_ACTIVE_UNREAD)?,
            paused_unread: load(IDI_APP_PAUSED_UNREAD)?,
        })
    }
}

/// The four state icons of the darkbar set at [`small_icon_size`] — FR-90, task T-86-2, решение
/// 145.1: the glyph alone, no plate, for a dark taskbar.
///
/// A type of its own for the reason [`BalloonIcons`] is one: the whole set is either there or
/// absent, so the tray never mixes a plate-less «active» with a plated «paused».
struct DarkbarIcons {
    /// The "active" icon, without the plate.
    active: Icon,
    /// The "suspended" icon, without the plate.
    paused: Icon,
    /// The "active, and a letter is unread" icon, without the plate.
    active_unread: Icon,
    /// The "suspended, and a letter is unread" icon, without the plate.
    paused_unread: Icon,
}

impl DarkbarIcons {
    /// Loads all four at `size`, or none — NFR-13.
    ///
    /// Journaled once under a name of its own and not an error of the tray: with this set absent
    /// the main set serves both taskbars, which is exactly the program before task T-86-2.
    fn load(instance: HINSTANCE, size: (i32, i32)) -> Option<Self> {
        let load = |id: u16| match Icon::load(instance, id, size) {
            Ok(icon) => Some(icon),
            Err(error) => {
                app::report_non_critical("LoadImageW(SM_CXSMICON, darkbar)", &error);
                None
            }
        };

        Some(Self {
            active: load(IDI_TRAY_DARK_ACTIVE)?,
            paused: load(IDI_TRAY_DARK_PAUSED)?,
            active_unread: load(IDI_TRAY_DARK_ACTIVE_UNREAD)?,
            paused_unread: load(IDI_TRAY_DARK_PAUSED_UNREAD)?,
        })
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `LoadImageW` without `LR_SHARED`, so it is
        // ours to destroy, and it is destroyed exactly once — this type is neither `Copy` nor
        // `Clone`. The icon is no longer in the notification area: the fields of `Tray` are
        // dropped after `Tray::drop` has run, and that removes it.
        if let Err(error) = unsafe { DestroyIcon(self.handle) } {
            app::report_non_critical("DestroyIcon", &error);
        }
    }
}

/// The DPI of the monitor `hwnd` is on — task T-42-10, finding 124б.2.
///
/// The one road this file takes to the display scale, and the same one the drawing takes:
/// `theme::dc_dpi` of a DC belonging to the **window**. `GetDpiForWindow` would say it in one
/// call, but it lives in the `Win32_UI_HiDpi` feature and the list of features of section 3.2
/// is closed (SEC-03) — and this road is not a workaround, it is what every painted pixel of
/// this program already goes through.
///
/// ⚠ `None` is a caller that has **no window yet** — [`crate::settings::CaptionIcons`] loads the
/// caption icon before `WM_INITDIALOG`, because a resource does not need a window and a window
/// does not exist yet. The screen DC then answers 96 in this process, which is the size that
/// caller asked for before this task and no worse than it was.
///
/// NFR-13: a DC that could not be got leaves the caller at 96 DPI, which is the 100 % look.
fn window_dpi(hwnd: Option<HWND>) -> i32 {
    // SAFETY: `hwnd` is a live window of this process or `None` for the screen; the DC is
    // released below on the same thread, as `ReleaseDC` requires.
    let dc = unsafe { GetDC(hwnd) };

    if dc.is_invalid() {
        return 96;
    }

    let dpi = crate::theme::dc_dpi(dc);

    // SAFETY: the DC was taken from this same window a line above.
    unsafe { ReleaseDC(hwnd, dc) };

    dpi
}

/// One system metric at the scale of the monitor `hwnd` is on — task T-42-10.
///
/// # ⛔ Why the metric has to be scaled by hand
///
/// The manifest declares this process **`PerMonitorV2`** (section 8.1), and in such a process
/// `GetSystemMetrics` answers for the DPI of the **thread's awareness context**, which is 96 —
/// there is no single DPI a per-monitor process could be asked about, which is exactly why
/// `GetSystemMetricsForDpi` exists. Measured on this machine: `GetSystemMetrics(SM_CXSMICON)`
/// answers **16** where 125 % needs **20**, and `GetSystemMetrics(SM_CXICON)` answers 32 where
/// it needs 40.
///
/// So the icon of FR-90 was loaded at 16 px and stretched to 20 by the shell, and the owner saw
/// it on the live product: «Ярлык в трее размытый и не четкий» (решение 124б.2). The `.ico`
/// files carry a 20 px frame — nobody was asking for it.
///
/// `GetSystemMetricsForDpi` is not reachable: it lives in the `Win32_UI_HiDpi` feature, and the
/// feature list is closed (SEC-03, §2 of the задание). The arithmetic is the same one the
/// drawing does: the metric at 96 DPI, scaled to the window's.
pub(crate) fn metric_at_window_dpi(
    hwnd: Option<HWND>,
    index: SYSTEM_METRICS_INDEX,
    at_96: i32,
) -> i32 {
    metric_at_dpi(index, at_96, window_dpi(hwnd))
}

/// One system metric at `dpi` — the body of [`metric_at_window_dpi`], for a caller that knows the
/// DPI of its window already (the windows of the shell ask `GetDpiForWindow`).
fn metric_at_dpi(index: SYSTEM_METRICS_INDEX, at_96: i32, dpi: i32) -> i32 {
    // SAFETY: `GetSystemMetrics` reads a system-wide value, takes no pointer and touches no
    // memory of ours. It returns zero for an index the system does not know, which is why the
    // result is examined rather than passed on (NFR-13): a zero would ask `LoadImageW` for the
    // resource's own size and quietly defeat the point of the call.
    let measured = unsafe { GetSystemMetrics(index) };

    // The documented value at 100 % is the fallback for a refused metric.
    let base = if measured > 0 { measured } else { at_96 };

    (base * dpi + 48) / 96
}

/// The small and the big icon of the system at `dpi`: the two frames of the icon on the caption of
/// a window of the shell (`ui::host`, Б1 of AUDIT-180) — the sizes [`small_icon_size`] and
/// [`large_icon_size`] answer, at the DPI of that window.
pub(crate) fn caption_icon_sizes(dpi: u32) -> [(i32, i32); 2] {
    let dpi = i32::try_from(dpi).unwrap_or(96);
    [
        (
            metric_at_dpi(SM_CXSMICON, 16, dpi),
            metric_at_dpi(SM_CYSMICON, 16, dpi),
        ),
        (
            metric_at_dpi(SM_CXICON, 32, dpi),
            metric_at_dpi(SM_CYICON, 32, dpi),
        ),
    ]
}

/// The size the shell wants a notification-area icon in — FR-90.
///
/// Asked of the system and **scaled to the window's monitor** ([`metric_at_window_dpi`]): the
/// value follows the display scale, and a 16 px frame on a scaled monitor is stretched into a
/// blurred icon. The four `.ico` files carry 16/20/24/32/48/64 px frames, and `LoadImageW`
/// picks the frame nearest what is asked for — 16 at 100 %, 20 at 125 %, 24 at 150 %, 32 at
/// 200 %, every one of them an exact frame.
pub fn small_icon_size(hwnd: Option<HWND>) -> (i32, i32) {
    (
        metric_at_window_dpi(hwnd, SM_CXSMICON, 16),
        metric_at_window_dpi(hwnd, SM_CYSMICON, 16),
    )
}

/// The size the shell draws the icon of a **balloon** at — задача Т-33а-1.
///
/// `SM_CXICON` and not a fixed 32, and scaled to the window's monitor for the reason
/// [`metric_at_window_dpi`] gives: 32 px at 100 %, 40 at 125 %, 48 at 150 %, 64 at 200 %. The
/// four `.ico` files carry 32, 40, 48 and 64 px frames, so nothing is stretched at any of them.
pub fn large_icon_size(hwnd: Option<HWND>) -> (i32, i32) {
    (
        metric_at_window_dpi(hwnd, SM_CXICON, 32),
        metric_at_window_dpi(hwnd, SM_CYICON, 32),
    )
}

/// Which of the eight icons of FR-90 the record carries — task T-86-2, решение 145.1.
///
/// Four states, each in two sets: the main set (a dark plate, a white glyph) for a light taskbar
/// and for everything that is not the tray, and the darkbar set (the glyph alone, no plate) for a
/// dark taskbar, where the plate melted into the bar (finding 144б). An enumeration and not a
/// bare resource number so that [`Tray::icon_of`] can be exhaustive: a ninth icon is a compile
/// error, not a fallthrough.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateIcon {
    /// Working, nothing unread — the main set.
    Active,
    /// Suspended — the main set.
    Paused,
    /// Working, a letter unread — the main set.
    ActiveUnread,
    /// Suspended, a letter unread — the main set.
    PausedUnread,
    /// Working — the darkbar set.
    DarkbarActive,
    /// Suspended — the darkbar set.
    DarkbarPaused,
    /// Working, a letter unread — the darkbar set.
    DarkbarActiveUnread,
    /// Suspended, a letter unread — the darkbar set.
    DarkbarPausedUnread,
}

impl StateIcon {
    /// The resource of `app.rc` this icon is loaded from.
    pub fn resource(self) -> u16 {
        match self {
            Self::Active => IDI_APP_ACTIVE,
            Self::Paused => IDI_APP_PAUSED,
            Self::ActiveUnread => IDI_APP_ACTIVE_UNREAD,
            Self::PausedUnread => IDI_APP_PAUSED_UNREAD,
            Self::DarkbarActive => IDI_TRAY_DARK_ACTIVE,
            Self::DarkbarPaused => IDI_TRAY_DARK_PAUSED,
            Self::DarkbarActiveUnread => IDI_TRAY_DARK_ACTIVE_UNREAD,
            Self::DarkbarPausedUnread => IDI_TRAY_DARK_PAUSED_UNREAD,
        }
    }
}

/// The rule of the icon — FR-90, task T-86-2: three answers in, one of eight icons out.
///
/// The shape of [`setting_name_is_wanted`], for the same reason: the facts are read once by the
/// caller — `general.enabled`, the feed of the letters, the taskbar switch the tray holds — and
/// handed in, so the rule is a function of its arguments and an eight-row table in
/// `tests\tray.rs` closes it. The first two questions are the table [`Tray::state_icon`] has
/// answered since task Т-32-4; the third is this task, and it chooses the set.
pub fn state_icon_for(enabled: bool, unread: bool, taskbar_light: bool) -> StateIcon {
    match (enabled, unread, taskbar_light) {
        (true, false, true) => StateIcon::Active,
        (false, false, true) => StateIcon::Paused,
        (true, true, true) => StateIcon::ActiveUnread,
        (false, true, true) => StateIcon::PausedUnread,
        (true, false, false) => StateIcon::DarkbarActive,
        (false, false, false) => StateIcon::DarkbarPaused,
        (true, true, false) => StateIcon::DarkbarActiveUnread,
        (false, true, false) => StateIcon::DarkbarPausedUnread,
    }
}

/// A resource identifier in the `MAKEINTRESOURCE` form the resource loader expects.
///
/// The identifier goes *into* the pointer rather than being pointed at: a value below 65536
/// tells the loader to look the resource up by number and not by name, and nothing ever
/// dereferences it. `without_provenance` says exactly that — an address with no allocation
/// behind it — which is both what Win32 means here and what stops the value from being
/// mistaken for a pointer that could be read.
fn resource_id(id: u16) -> PCWSTR {
    PCWSTR(std::ptr::without_provenance(usize::from(id)))
}

/// The tooltip of the icon — `APP_NAME` plus the state of the **program**, and nothing else,
/// ever (SEC-01, SEC-07). FR-90, решение 99.2, task Т-31-4.
///
/// A free function and not a method, so that a test can walk all fourteen locales against both
/// states without a tray, an icon and a window; [`Tray::notify_data`] is the one caller in the
/// product and hands in `self.enabled()`.
///
/// **The name is not translated** — decision on question 7 — and the state word is, which is the
/// whole of this task: until решение 99.2 both words were Russian literals here, in all fourteen
/// locales, and neither Э28 nor Э30 caught it because the fitting stand measures windows and not
/// the notification area.
///
/// The em dash and the two spaces around it are the same joiner the caption of the settings
/// window uses («Lang Switcher — настройки»). Since stage L10N, task T-L10N-10 (решения 200.10 и
/// 200.10а) the form is a string, [`settings::IDS_TIP_FORMAT`] — «{0} — {1}» in every table, so
/// a language may give its own order and joiner; the name stays untranslated as `{0}`.
pub fn icon_tip(enabled: bool) -> String {
    let state = settings::text(if enabled {
        settings::IDS_TIP_ACTIVE
    } else {
        settings::IDS_TIP_PAUSED
    });

    settings::format_text(settings::IDS_TIP_FORMAT, &[APP_NAME, &state])
}

/// The tooltip while the icon is the flag — FR-90б, task T-119-2: the name of the layout first
/// (the name the lists of the settings window give it — the language in its own words, вопрос 92,
/// and the identifier of решение 82.1 behind it when two layouts of the session share the name,
/// which is how two layouts of one country, one flag, are told apart), then [`icon_tip`] on the
/// line below.
pub fn flag_tip(layout: LayoutId, enabled: bool) -> String {
    let session = crate::layouts::enumerate().unwrap_or_default();
    let name = match session.iter().position(|one| *one == layout) {
        Some(at) => settings::layout_labels(&session, &session).swap_remove(at),
        None => settings::layout_labels(&[layout], &session)
            .pop()
            .unwrap_or_default(),
    };

    format!("{name}\n{}", icon_tip(enabled))
}

/// Re-reads the layout for the flag — FR-90б, task T-119-2. Called on the UI thread after a press
/// of the program's own (the sound of FR-100 is told of every press, so the flag follows the same
/// message) and, with the subscriptions of T-119-4, after the layout may have moved. Nothing at
/// all with the flag off.
pub fn refresh_flag() {
    with_tray(|tray| {
        if tray.flag_on() {
            tray.refresh_flag();
        }
    });
}

/// FR-82 since 1.8.0 — task T-119-14, решение 208.148: the program was started a second time in
/// this session. The settings window comes up — through the door of the menu,
/// [`dispatch_command`] — or, when it (or the «О программе» window that locks it) is already up,
/// that window comes forward. On the UI thread, outside every borrow: the dialog is modal.
///
/// ⚠ **The dialog comes up above the window in front but not in front of it** (stand T-119-14,
/// Windows 11, 2026-11-20: the settings on top, the foreground still Блокнот) — a modal dialog
/// activates itself and does not take the foreground. So a one-shot timer of the UI window
/// ([`SECOND_START_TIMER`]), served by the dialog's own message loop, brings it forward while the
/// right the second start gave is still good.
pub fn show_settings_for_a_second_start() {
    if bring_the_open_window_forward() {
        return;
    }

    if let Some(hwnd) = with_tray(|tray| tray.hwnd) {
        // SAFETY: the live UI window and an identifier of this module; one-shot by the kill in
        // `handle_ui_message`.
        let _ = unsafe { SetTimer(Some(hwnd), SECOND_START_TIMER, SECOND_START_MS, None) };
        dispatch_command(hwnd, CMD_SETTINGS);
    }
}

/// The timer of [`show_settings_for_a_second_start`] — an identifier of the UI window, after the
/// flag's `27`…`29` (`flags::LIST_TIMERS`).
pub const SECOND_START_TIMER: usize = 30;

/// Its delay, in milliseconds: the dialog is made before its loop serves the first timer.
const SECOND_START_MS: u32 = 150;

/// The settings window or the «О программе» window of this thread forward, if one is up.
fn bring_the_open_window_forward() -> bool {
    let Some(open) = settings::open_window() else {
        return false;
    };

    // SAFETY: a live window of this thread by value; the second start gave this process the right
    // to the foreground (`AllowSetForegroundWindow`), and a refusal leaves it where it is.
    let _ = unsafe { SetForegroundWindow(open) };
    true
}

/// Copies `text` into one of the fixed fields of the notification area, always NUL-terminated
/// and **never cut through a surrogate pair** — tasks Т-32-4 and T-41-8.
///
/// The three fields are `szTip` (128 units), `szInfo` (256) and `szInfoTitle` (64), and one
/// generic body is one place the truncation can be wrong instead of three. The rule is
/// **cut, never refuse** — a sentence of a locale nobody measured must not be able to stop a
/// letter being announced (NFR-13).
///
/// # Finding Т11 — where the cut may not fall
///
/// These fields are arrays of UTF-16 **units**, not of characters. Everything outside the Basic
/// Multilingual Plane — an emoji, a rare ideograph — is written as a **surrogate pair**, two
/// units that mean one character only together, and a cut that falls between them would hand the
/// shell the leading half alone: not a character, and not valid UTF-16 either. So when the last
/// unit written is a leading half (`0xD800..=0xDBFF`) the terminator goes one place earlier and
/// that half is given back.
///
/// ⚠ **Only the leading half.** A *trailing* half as the last unit means the pair fitted whole,
/// and stepping back there would throw away a character that was perfectly well placed — which
/// is what the second control of the acceptance is for.
///
/// For `szTip` the branch is insurance: «Lang Switcher — активна» is short in all fourteen
/// locales. For the two balloon fields it is not — they carry the text of a letter, of whatever
/// length its author wrote.
fn write_field<const N: usize>(field: &mut [u16; N], text: &str) {
    let limit = field.len().saturating_sub(1);
    let mut written = 0;

    for unit in text.encode_utf16().take(limit) {
        field[written] = unit;
        written += 1;
    }

    // Task T-41-8, finding Т11. Three lines, and the whole of the repair: the pair is either
    // whole or not here at all.
    if written > 0 && (0xD800..=0xDBFF).contains(&field[written - 1]) {
        written -= 1;
    }

    field[written] = 0;
}

/// The bits of a signed message parameter, so its words can be taken apart.
fn unsigned(value: isize) -> usize {
    usize::from_ne_bytes(value.to_ne_bytes())
}

/// The low 16 bits of a message parameter.
fn low_word(value: usize) -> u16 {
    u16::try_from(value & 0xFFFF).unwrap_or(0)
}

/// The next 16 bits of a message parameter.
fn high_word(value: usize) -> u16 {
    u16::try_from((value >> 16) & 0xFFFF).unwrap_or(0)
}

/// A screen coordinate packed into 16 bits of a message parameter.
///
/// Signed, and that is not pedantry: a monitor placed left of or above the primary one has
/// negative screen coordinates, and reading the word as unsigned puts the menu somewhere
/// around x = 65000 — on the wrong screen, or on none.
fn coordinate(word: u16) -> i32 {
    i32::from(i16::from_le_bytes(word.to_le_bytes()))
}

#[cfg(test)]
mod tests {

    use super::*;

    /// A `VS_VERSIONINFO` prefix carrying the given two version words.
    fn version_resource_bytes(most: u32, least: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; FIXED_FILE_INFO_OFFSET];
        bytes.extend_from_slice(&FIXED_FILE_INFO_SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes()); // dwStrucVersion
        bytes.extend_from_slice(&most.to_le_bytes());
        bytes.extend_from_slice(&least.to_le_bytes());
        bytes
    }

    /// ⭐ **Task Т-22-9, finding м12 of the audit of 2026-09-01 — the refused save leaves the
    /// system's own number and nothing else.**
    ///
    /// Every variant of [`settings::ConfigError`] is driven, because the mapping is where SEC-01
    /// and SEC-07 are decided: one variant carries a Win32 code, which is a number the operating
    /// system gave and is what the journal is for, and one carries the line and column a parser
    /// stopped at — a value derived from the **contents** of somebody's file, which may not reach
    /// the ring in any form.
    ///
    /// Since task T-69-6 the mapping is `diag::OsCode::of_config`, the one bridge; the test stays
    /// here, in the module that journals a configuration failure, and drives it unchanged.
    #[test]
    fn a_refused_write_reports_the_system_code_and_a_parser_position_reports_nothing() {
        /// `ERROR_ACCESS_DENIED`, and the `HRESULT` Win32 codes are journalled under.
        const ACCESS_DENIED: i32 = 5;
        const AS_HRESULT: i32 = 0x8007_0005_u32 as i32;

        let refused = settings::ConfigError::Io(std::io::Error::from_raw_os_error(ACCESS_DENIED));

        assert_eq!(
            diag::OsCode::of_config(&refused).raw(),
            AS_HRESULT,
            "the number the operating system gave, in the form every other failure is recorded in"
        );

        // An `io::Error` this program built itself carries no system number, and none is invented.
        let ours = settings::ConfigError::Io(std::io::Error::other("no system call failed here"));

        assert_eq!(
            diag::OsCode::of_config(&ours).raw(),
            diag::OsCode::NONE.raw()
        );

        // SEC-01, SEC-07: the position a parser stopped at is a fact about the file's contents.
        for error in [
            settings::ConfigError::Malformed { at: Some((12, 34)) },
            settings::ConfigError::Malformed { at: None },
            settings::ConfigError::Serialize,
        ] {
            assert_eq!(
                diag::OsCode::of_config(&error).raw(),
                diag::OsCode::NONE.raw(),
                "{error:?} is not a system failure and must contribute no number"
            );
        }
    }

    /// ⭐ **Task Т-22-9.** A real refusal of the real writer reaches the ring under the row of the
    /// closed table, carrying the system's code.
    ///
    /// # How the refusal is staged
    ///
    /// A file is created, and `settings::write_to` is then asked to write **inside** it. The
    /// parent of the target is a file, so `create_dir_all` refuses before a single byte is
    /// written and the answer is a real `ConfigError::Io` with a real Win32 code — which is what
    /// distinguishes this from the mapping test above: nothing here is a value the test invented.
    ///
    /// The whole staging lives in this process's own temporary directory and is removed at the
    /// end. `config.toml` of the user is not reachable from here and is not touched.
    #[test]
    fn a_refused_write_of_the_real_writer_lands_in_the_journal() {
        let blocker = std::env::temp_dir().join(format!(
            "langsw-t-22-9-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        std::fs::write(&blocker, b"not a directory").expect("the temporary file must be creatable");

        let target = blocker.join("config.toml");

        let before = diag::recorded();

        let error = settings::write_to(&target, &settings::Config::default())
            .expect_err("a write inside a file cannot succeed");

        note_configuration_failure(CONFIG_WRITE_FAILED, &error);

        let _ = std::fs::remove_file(&blocker);

        let named = diag::Operation::from_name(CONFIG_WRITE_FAILED);

        assert_ne!(
            named,
            diag::Operation::UNLISTED,
            "Т-22-9: the row must be in the closed table, or the event keeps none of its name"
        );

        let entry = diag::snapshot()
            .into_iter()
            .find(|event| event.ordinal >= before && event.operation == named)
            .expect("the refusal must be in the ring");

        assert_eq!(entry.kind(), diag::Kind::Process);
        assert_ne!(
            entry.code.raw(),
            diag::OsCode::NONE.raw(),
            "a write refused by the file system carries the system's own number"
        );
    }

    #[test]
    fn the_four_parts_of_the_version_come_out_in_order() {
        let bytes = version_resource_bytes(0x0001_0002, 0x0003_0004);

        assert_eq!(parse_fixed_file_version(&bytes), Some((1, 2, 3, 4)));
    }

    #[test]
    fn a_buffer_without_the_signature_is_refused() {
        let mut bytes = version_resource_bytes(0x0000_0001, 0);
        bytes[FIXED_FILE_INFO_OFFSET] ^= 0xFF;

        assert_eq!(parse_fixed_file_version(&bytes), None);
    }

    #[test]
    fn a_buffer_too_short_for_the_structure_is_refused() {
        let bytes = version_resource_bytes(0, 0);

        for length in [0, FIXED_FILE_INFO_OFFSET, FIXED_FILE_INFO_OFFSET + 15] {
            assert_eq!(
                parse_fixed_file_version(&bytes[..length]),
                None,
                "a {length}-byte buffer must not be read as a version resource"
            );
        }
    }

    #[test]
    fn a_coordinate_left_of_the_primary_monitor_stays_negative() {
        // The trap of the version 4 convention is not only which parameter holds the
        // position, but that both halves of it are signed.
        assert_eq!(coordinate(low_word(0xFFF6)), -10);
        assert_eq!(coordinate(low_word(1920)), 1920);
    }

    #[test]
    fn the_halves_of_a_message_parameter_are_taken_apart_the_right_way_round() {
        let packed = 0x0064_0032usize;

        assert_eq!(low_word(packed), 0x0032);
        assert_eq!(high_word(packed), 0x0064);
        assert_eq!(unsigned(-1isize), usize::MAX);
    }

    #[test]
    fn a_tooltip_is_always_terminated() {
        let mut field = [0xFFFFu16; 128];
        write_field(&mut field, "Lang Switcher — активна");

        let end = field.iter().position(|unit| *unit == 0).unwrap();
        assert_eq!(
            String::from_utf16_lossy(&field[..end]),
            "Lang Switcher — активна"
        );
    }

    #[test]
    fn an_over_long_tooltip_is_truncated_rather_than_overrunning() {
        let mut field = [0xFFFFu16; 128];
        let long = "я".repeat(500);
        write_field(&mut field, &long);

        assert_eq!(field[127], 0, "the last unit must be the terminator");
        assert!(
            field[..127].iter().all(|unit| *unit == 0x044F),
            "the field must be filled with the text and nothing left over"
        );
    }

    /// **Finding Т11, task T-41-8 — a fixed field is never cut through a surrogate pair.**
    ///
    /// The three fields of the notification area are arrays of UTF-16 **units**, and what goes
    /// into them is cut to fit. A character outside the Basic Multilingual Plane — an emoji, a
    /// rare ideograph — is two units, and a cut that falls between them leaves the shell the
    /// leading half alone: not a character, and not valid UTF-16 either.
    ///
    /// ⚠ **Driven on all three fields, because only one of the three is unreachable.** `szTip`
    /// carries «Lang Switcher — активна», short in all fourteen locales, so for the tooltip this
    /// is insurance. `szInfo` and `szInfoTitle` carry the **text of a letter**, of whatever length
    /// its author wrote, and for them the branch fires on real data.
    ///
    /// The boundary is odd in all three (127, 255, 63) and an emoji is two units, so a string of
    /// nothing but emoji lands the cut between the halves by construction — no arithmetic to get
    /// wrong.
    #[test]
    fn a_fixed_field_is_never_cut_through_a_surrogate_pair() {
        /// One emoji — U+1F600, two UTF-16 units: `0xD83D` then `0xDE00`.
        const FACE: &str = "😀";

        fn check<const N: usize>(name: &str) {
            let mut field = [0xFFFFu16; N];
            let limit = N - 1;
            let text = FACE.repeat(N);

            write_field(&mut field, &text);

            let end = field
                .iter()
                .position(|unit| *unit == 0)
                .expect("the field must carry a terminator");

            let decoded = String::from_utf16(&field[..end]);

            println!(
                "{name}: field {N}, limit {limit}, written {end}, last unit {:#06X}, decoded {:?}",
                if end == 0 { 0 } else { field[end - 1] },
                decoded.as_ref().map(|text| text.chars().count())
            );

            assert!(
                decoded.is_ok(),
                "{name}: what the shell is handed must be valid UTF-16 — the cut fell between \
                 the halves of a pair and left the leading one behind"
            );

            let decoded = decoded.unwrap_or_default();

            assert!(
                !decoded.contains('\u{FFFD}'),
                "{name}: and it must carry no replacement character either"
            );
            assert_eq!(
                decoded.chars().count(),
                limit / 2,
                "{name}: as many whole emoji as fit, and the odd unit dropped"
            );
            assert_eq!(
                end,
                limit - 1,
                "{name}: exactly one unit short of the limit — the leading half stepped back"
            );
        }

        check::<128>("szTip");
        check::<256>("szInfo");
        check::<64>("szInfoTitle");

        // The control: a boundary that does **not** fall on a pair is cut exactly as before,
        // unit for unit. Without this half the repair could have been «шагать назад всегда».
        let mut field = [0xFFFFu16; 128];
        write_field(&mut field, &"я".repeat(500));

        assert_eq!(
            field[127], 0,
            "the terminator is in the last unit as before"
        );
        assert!(
            field[..127].iter().all(|unit| *unit == 0x044F),
            "and all 127 units before it are the text — nothing was given back"
        );

        // And the other control: a pair that fits **whole** is not touched. The last written
        // unit is then a trailing half, and stepping back there would throw away a character
        // that was perfectly well placed.
        let mut field = [0xFFFFu16; 5];
        write_field(&mut field, FACE);

        assert_eq!(
            (field[0], field[1], field[2]),
            (0xD83D, 0xDE00, 0),
            "a whole pair with room to spare is written whole"
        );
    }

    #[test]
    fn the_small_icon_metric_is_a_usable_size() {
        let (width, height) = small_icon_size(None);

        assert!(
            width > 0 && height > 0,
            "SM_CXSMICON and SM_CYSMICON must give a usable size"
        );
    }

    /// **Task T-42-10, finding 124б.2** — the size of an icon follows the scale of the monitor,
    /// and the frames the `.ico` files carry are exactly the sizes it takes.
    ///
    /// ⛔ The defect this closes: in a `PerMonitorV2` process `GetSystemMetrics` answers for
    /// 96 DPI whatever the monitor does, so the tray icon was loaded at 16 px and stretched to
    /// 20 by the shell. The owner saw it on the live product — «Ярлык в трее размытый и не
    /// четкий».
    ///
    /// The measurement is of the arithmetic, not of this machine's scale: `metric_at_window_dpi`
    /// multiplies a 96-DPI metric by the window's DPI, and the values it must produce at the
    /// five scales are the frames that exist in the resources.
    #[test]
    fn an_icon_size_follows_the_scale_and_lands_on_a_frame_that_exists() {
        // The frames all eight `.ico` files carry — `res\langswitcher-*.ico`, nineteen since
        // task T-85-2 and eight files since task T-86-2 — checked by `tools\verify-icons.ps1`
        // (решение 145.6: this list had stood at the ten of task T-42-11).
        let frames = [
            16, 20, 24, 28, 30, 32, 36, 40, 45, 48, 54, 56, 60, 63, 64, 72, 80, 96, 256,
        ];

        // What the arithmetic of `metric_at_window_dpi` gives for the two metrics at the five
        // scales the fitting stand of the контур measures.
        let scaled = |at_96: i32, dpi: i32| (at_96 * dpi + 48) / 96;

        for (dpi, percent) in [(96, 100), (120, 125), (144, 150), (168, 175), (192, 200)] {
            let small = scaled(16, dpi);
            let large = scaled(32, dpi);

            println!("{percent}%: значок трея {small} px, значок шара {large} px");

            for (what, size) in [("трея", small), ("шара", large)] {
                assert!(
                    frames.contains(&size),
                    "at {percent}% the icon {what} is asked for at {size} px, and no frame of \
                     that size exists — the shell would stretch the nearest one and the icon \
                     would be blurred, which is finding 124б.2"
                );
            }
        }

        // The three sizes the задача had to add frames for, spelled out: 125 % of the small
        // metric is 20, 175 % of it is 28, and 125 % of the large one is 40. ⚠ The middle one
        // is why this test exists at all — the first draft of the fix assumed 30 and the test
        // said 28, which is what the arithmetic really gives.
        assert_eq!(scaled(16, 120), 20, "125 % of the small metric is 20 px");
        assert_eq!(scaled(16, 168), 28, "175 % of the small metric is 28 px");
        assert_eq!(scaled(32, 120), 40, "125 % of the large metric is 40 px");
    }
}
