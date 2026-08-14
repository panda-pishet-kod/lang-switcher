//! The tray icon, the menu, handling `TaskbarCreated`.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-90 (the icon and its two states), FR-91 (the context
//! menu), FR-81 (`TaskbarCreated`), and the part of FR-83 that exists today — removing the
//! icon and saving the configuration. The other three actions of FR-83 belong elsewhere:
//! releasing the mutex is already done by [`crate::app`], unhooking arrives with T-03-1 and
//! wiping the buffer with T-03-2, and both have a marked place in [`Tray::shut_down`].
//! The list follows the backlog rather than the stub line this file used to carry —
//! decision R-17: FR-83 is the backlog's and the backlog names T-01-4.
//! Implemented by backlog tasks: T-01-4 (done); T-08-1 replaces the two marked stubs
//! (FR-92, FR-93).
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
//! `TrackPopupMenuEx` runs a modal message loop of its own: while the menu is up, messages
//! keep being dispatched to our window procedure, which re-enters this module. Holding the
//! thread-local `RefCell` across that call would turn any such message into a panic. So
//! [`Tray::handle_message`] never shows the menu itself — it answers [`Reaction::ShowMenu`],
//! and [`handle_ui_message`] shows the menu once the borrow has been dropped. The same rule
//! covers `MessageBoxW` in the about box, which is modal for the same reason.
//!
//! # SEC-05
//!
//! Only the messages named in [`Tray::handle_message`] are handled; everything else is
//! answered with [`Reaction::Ignored`] and goes to `DefWindowProcW`. The callback message of
//! the icon can be posted by any process at the same integrity level, and all that achieves
//! is a menu on the screen. There is deliberately **no `WM_COMMAND` handler**: the menu is
//! tracked with `TPM_RETURNCMD`, so the chosen command comes back as the return value of
//! `TrackPopupMenuEx` instead of arriving as a message. A forged `WM_COMMAND` therefore
//! cannot pause the program or shut it down — no privileged action is reachable by message.
//!
//! # SEC-01, SEC-07
//!
//! Nothing here ever sees a keystroke, a key code or the contents of the buffer, and nothing
//! may be added later that would: the tooltip is built from [`crate::APP_NAME`] and a state
//! word, the about box from that name and the version resource, and the menu from fixed
//! strings.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::PathBuf;

use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetSystemMetrics, HICON, HMENU,
    IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MessageBoxW, PostMessageW,
    RT_VERSION, RegisterWindowMessageW, SM_CXSMICON, SM_CYSMICON, SetForegroundWindow,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP, WM_CONTEXTMENU,
    WM_ENDSESSION, WM_LBUTTONDBLCLK, WM_NULL, WM_QUERYENDSESSION, WM_USER,
};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult, w};

use crate::settings::{self, Config};
use crate::{APP_NAME, app};

// ---------------------------------------------------------------------------------------
// The menu of FR-91 — labels and commands
// ---------------------------------------------------------------------------------------

/// First item of FR-91 while the program is active.
pub const LABEL_SUSPEND: &str = "Приостановить";

/// First item of FR-91 while the program is suspended.
pub const LABEL_RESUME: &str = "Возобновить";

/// Second item of FR-91. The last character is U+2026, one ellipsis and not three dots.
pub const LABEL_SETTINGS: &str = "Настройки…";

/// Third item of FR-91, the one carrying the check mark of `general.autostart`.
pub const LABEL_AUTOSTART: &str = "Запускать при входе в систему";

/// Fourth item of FR-91.
pub const LABEL_ABOUT: &str = "О программе";

/// Fifth item of FR-91.
pub const LABEL_EXIT: &str = "Выход";

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

/// Number of entries FR-91 puts in the menu: five commands and two separators.
///
/// Spelled out because it is the number a reader is most likely to get wrong: the block in
/// FR-91 has seven lines, two of which are rules.
pub const MENU_ENTRY_COUNT: i32 = 7;

// ---------------------------------------------------------------------------------------
// Private constants
// ---------------------------------------------------------------------------------------

/// Resource identifier of the "active" icon in `app.rc`.
const IDI_APP_ACTIVE: u16 = 101;

/// Resource identifier of the "suspended" icon in `app.rc`.
const IDI_APP_PAUSED: u16 = 102;

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
        /// Screen x of the cursor, taken from `wParam` — the version 4 convention.
        x: i32,
        /// Screen y of the cursor, taken from `wParam`.
        y: i32,
    },
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
    /// The configuration; `general.enabled` *is* the state of FR-90.
    config: Config,
    /// Where the configuration is written back to. `None` when `%APPDATA%` is not set.
    config_path: Option<PathBuf>,
    /// What `RegisterWindowMessageW("TaskbarCreated")` returned — FR-81. Zero means the
    /// registration failed, and zero is also `WM_NULL`, so it must never be compared against.
    taskbar_created: u32,
    /// Whether the icon is believed to be in the notification area right now.
    icon_present: bool,
    /// How many times `NIM_ADD` has been issued. Acceptance point 12 reads this to see that
    /// `TaskbarCreated` really does add the icon a second time.
    add_calls: u32,
    /// Whether [`Tray::shut_down`] has already run. The cleanup of FR-83 is reached from two
    /// directions and has to do its work exactly once.
    finished: bool,
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
        let icon_size = small_icon_size();
        let active = Icon::load(instance, IDI_APP_ACTIVE, icon_size)?;
        let paused = Icon::load(instance, IDI_APP_PAUSED, icon_size)?;

        let config = match config_path.as_deref() {
            Some(path) => {
                let (config, outcome) = settings::read_or_default(path);
                // `read_or_default` has already substituted the defaults of section 7 for
                // anything it could not read, which is what it exists for; no decision is
                // left to the tray. Recording *why* a file failed to parse belongs to the
                // ring journal — TODO(T-06-4): hand `outcome` to `diag`.
                let _ = outcome;
                config
            }
            // `%APPDATA%` is not set. A resident utility still has to run, and the defaults
            // of section 7 are a complete configuration, so this is not a reason to refuse.
            None => Config::default(),
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
            config,
            config_path,
            taskbar_created,
            icon_present: false,
            add_calls: 0,
            finished: false,
        };

        tray.add_icon();

        Ok(tray)
    }

    /// Whether correction is armed — `general.enabled`, the state of FR-90.
    pub fn enabled(&self) -> bool {
        self.config.general.enabled
    }

    /// Whether `general.autostart` is set. The check mark of FR-91 shows this and nothing
    /// else; changing it is FR-93 and belongs to task T-08-1.
    pub fn autostart(&self) -> bool {
        self.config.general.autostart
    }

    /// The configuration this tray holds.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The size the icons were loaded at — `(SM_CXSMICON, SM_CYSMICON)`.
    pub fn icon_size(&self) -> (i32, i32) {
        self.icon_size
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
    /// the menu is not a message.
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
            WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT => Reaction::ShowMenu {
                x: coordinate(low_word(wparam.0)),
                y: coordinate(high_word(wparam.0)),
            },

            // A double click is deliberately not the settings dialog: the dialog is FR-92
            // and does not exist yet. With the mouse this arm is not even reached — the
            // first click of the pair has already opened the menu and the second is consumed
            // by the menu's own modal loop.
            // TODO(T-08-1): open the settings dialog of FR-92 from here.
            WM_LBUTTONDBLCLK => Reaction::Handled(LRESULT(0)),

            // Every other notification of the icon: balloon events, the hover notifications
            // of version 4, the raw button messages that accompany the ones above. A
            // `WM_APP` message has no default handling, so answering zero is the whole of
            // "ignore it".
            _ => Reaction::Handled(LRESULT(0)),
        }
    }

    /// Flips between "active" and "suspended" and writes the change to the configuration.
    ///
    /// The state of FR-90 is stored in `general.enabled`, so switching it and saving it are
    /// the same operation. There is no hook to arm or disarm yet — task T-03-1 reads this
    /// flag; until then the state is the icon, the tooltip and the file.
    pub fn toggle_state(&mut self) {
        self.config.general.enabled = !self.config.general.enabled;
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

        // FR-83, "удаление иконки".
        self.remove_icon();

        // FR-83, "сохранение конфигурации".
        self.save_config();

        // FR-83, "снятие хуков" — TODO(T-03-1): there is no `WH_KEYBOARD_LL` hook yet, and
        // this is where `UnhookWindowsHookEx` goes when there is one.
        //
        // FR-83, "обнуление буфера" — TODO(T-03-2): there is no keystroke buffer yet, and
        // this is where it is wiped (SEC-07) when there is one.
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
    fn state_icon(&self) -> HICON {
        if self.enabled() {
            self.active.handle
        } else {
            self.paused.handle
        }
    }

    /// Tooltip text — `APP_NAME` plus the state, and nothing else, ever (SEC-01, SEC-07).
    fn tip_text(&self) -> String {
        let state = if self.enabled() {
            "активна"
        } else {
            "приостановлена"
        };

        format!("{APP_NAME} — {state}")
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
            hIcon: self.state_icon(),
            ..Default::default()
        };

        write_tip(&mut data.szTip, &self.tip_text());

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

    /// Updates the icon and the tooltip in place — `NIM_MODIFY`.
    fn refresh_icon(&mut self) {
        if !self.icon_present {
            // Nothing to modify: the shell was not up when we tried. FR-81 will add the icon
            // with the current state, which is this one.
            return;
        }

        let data = self.notify_data();

        // SAFETY: identical to the `NIM_ADD` above — the same locally owned descriptor, the
        // same live window, the same icon handles owned by `self`.
        let modified = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };

        if !modified.as_bool() {
            self.icon_present = false;
            app::report_non_critical("Shell_NotifyIconW(NIM_MODIFY)", &WinError::from_thread());
        }
    }

    /// Removes the icon — `NIM_DELETE`.
    fn remove_icon(&mut self) {
        if !self.icon_present {
            return;
        }

        let data = self.notify_data();

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
    }

    // -----------------------------------------------------------------------------------
    // The configuration
    // -----------------------------------------------------------------------------------

    /// Writes the configuration back — section 7, and the "сохранение конфигурации" of FR-83.
    fn save_config(&self) {
        let Some(path) = self.config_path.as_deref() else {
            // No `%APPDATA%`: there is nowhere to save to, and nothing was read from there
            // either, so the state simply does not outlive the session.
            return;
        };

        if let Err(error) = settings::write_to(path, &self.config) {
            // A failed save must not take the process down: in the FR-83 case the program is
            // on its way out anyway, and in the toggle case the state is already right in
            // memory and on the screen. TODO(T-06-4): this belongs in the ring journal.
            let _ = error;
        }
    }
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
    let tray = Tray::install(hwnd, instance)?;

    UI_TRAY.with(|slot| slot.replace(Some(tray)));

    Ok(Attachment {
        _not_send: PhantomData,
    })
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

/// The window-procedure entry point of the tray.
///
/// `Some` means the message was handled and that value has to be returned to Windows; `None`
/// means it is none of ours and belongs to `DefWindowProcW`.
pub fn handle_ui_message(message: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let reaction = with_tray(|tray| tray.handle_message(message, wparam, lparam))?;

    match reaction {
        Reaction::Ignored => None,
        Reaction::Handled(result) => Some(result),
        // Outside the borrow on purpose — see the module documentation.
        Reaction::ShowMenu { x, y } => {
            show_menu(x, y);
            Some(LRESULT(0))
        }
    }
}

// ---------------------------------------------------------------------------------------
// The menu — FR-91
// ---------------------------------------------------------------------------------------

/// The context menu of FR-91, destroyed when this value is dropped.
///
/// Built afresh for every click rather than kept and patched: the first label and the check
/// mark depend on state that can change between two clicks, and a menu built from the state
/// it is about cannot show a stale one.
pub struct Menu {
    handle: HMENU,
}

impl Menu {
    /// Builds the menu of FR-91 for the given state.
    ///
    /// The composition is the block of FR-91 read top to bottom: the state item, a rule, the
    /// settings and autostart items, a rule, about and exit. Seven entries in all,
    /// [`MENU_ENTRY_COUNT`].
    pub fn build(enabled: bool, autostart: bool) -> WinResult<Self> {
        // SAFETY: takes no arguments and touches no memory of ours. The handle it returns is
        // owned by this value from here on and is destroyed exactly once, in `Drop`. The
        // crate turns a null handle into an error, so NFR-13 is satisfied by the `?`.
        let handle = unsafe { CreatePopupMenu()? };

        let menu = Self { handle };

        let first = if enabled { LABEL_SUSPEND } else { LABEL_RESUME };
        let autostart_mark = if autostart { MF_CHECKED } else { MF_UNCHECKED };

        menu.append_command(MF_STRING, CMD_TOGGLE, first)?;
        menu.append_separator()?;
        menu.append_command(MF_STRING, CMD_SETTINGS, LABEL_SETTINGS)?;
        menu.append_command(MF_STRING | autostart_mark, CMD_AUTOSTART, LABEL_AUTOSTART)?;
        menu.append_separator()?;
        menu.append_command(MF_STRING, CMD_ABOUT, LABEL_ABOUT)?;
        menu.append_command(MF_STRING, CMD_EXIT, LABEL_EXIT)?;

        Ok(menu)
    }

    /// The raw handle, for `TrackPopupMenuEx` and for the tests that read the menu back with
    /// `GetMenuItemCount` and `GetMenuStringW`.
    pub fn handle(&self) -> HMENU {
        self.handle
    }

    /// Appends one command item.
    fn append_command(&self, flags: MENU_ITEM_FLAGS, command: u32, label: &str) -> WinResult<()> {
        let label = wide(label);

        // SAFETY: `self.handle` is a live menu created by `CreatePopupMenu` and owned by this
        // value. `label` is a NUL-terminated UTF-16 buffer owned by this frame, neither moved
        // nor dropped until the call returns; `AppendMenuW` copies the string rather than
        // keeping the pointer. `command` is one of our own non-zero identifiers, which is
        // what `MF_STRING` requires of the third argument. The crate turns the `BOOL` into a
        // `Result`, so NFR-13 is satisfied by returning it.
        unsafe {
            AppendMenuW(
                self.handle,
                flags,
                usize::try_from(command).unwrap_or(0),
                PCWSTR(label.as_ptr()),
            )
        }
    }

    /// Appends one of the two rules of FR-91.
    fn append_separator(&self) -> WinResult<()> {
        // SAFETY: as above. A separator takes neither an identifier nor a string, which is
        // what the zero and the null pointer say; `MF_SEPARATOR` makes the call ignore both.
        unsafe { AppendMenuW(self.handle, MF_SEPARATOR, 0, PCWSTR::null()) }
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `CreatePopupMenu` and is destroyed exactly
        // once — this type is neither `Copy` nor `Clone`. The menu is not attached to any
        // window: `TrackPopupMenuEx` tracks a popup without taking ownership of it, and it
        // has returned by the time this runs.
        if let Err(error) = unsafe { DestroyMenu(self.handle) } {
            app::report_non_critical("DestroyMenu", &error);
        }
    }
}

/// Shows the menu of FR-91 at a point on the screen and carries out what was chosen.
///
/// Called with no borrow of the tray held: `TrackPopupMenuEx` runs a modal message loop that
/// dispatches back into our own window procedure.
fn show_menu(x: i32, y: i32) {
    let Some((hwnd, enabled, autostart)) =
        with_tray(|tray| (tray.hwnd, tray.enabled(), tray.autostart()))
    else {
        return;
    };

    let menu = match Menu::build(enabled, autostart) {
        Ok(menu) => menu,
        Err(error) => {
            app::report_non_critical("Menu::build", &error);
            return;
        }
    };

    // ⚠ Documented Windows behaviour, not superstition. A popup menu tracked by a window that
    // is not in the foreground does not go away when the user clicks somewhere else: it stays
    // on the screen until something else dismisses it. `SetForegroundWindow` here and the
    // empty `PostMessageW` below are the two halves of the published workaround.
    //
    // SAFETY: `hwnd` is the live window this tray was installed on; the call reads no memory
    // of ours.
    let foreground = unsafe { SetForegroundWindow(hwnd) };

    if !foreground.as_bool() {
        // NFR-13. The call fails when this process has no right to take the foreground — the
        // system grants that right to whoever the user last interacted with, and the user has
        // just clicked our icon, so in practice it succeeds. Not fatal when it does not: the
        // menu still appears, it may merely need a second click to be dismissed.
        app::report_non_critical("SetForegroundWindow", &WinError::from_thread());
    }

    // SAFETY: `menu.handle()` is a live popup menu owned by `menu` for the whole call — the
    // value is dropped at the end of this function, after `TrackPopupMenuEx` has returned,
    // and the call does not take ownership of it. `hwnd` is our live window, which is what
    // receives the menu's messages. `None` for `TPMPARAMS` is the documented way to ask for
    // no excluded rectangle. `TPM_RETURNCMD` makes the call *return* the chosen identifier
    // instead of posting `WM_COMMAND` — which is why this program has no `WM_COMMAND`
    // handler at all (SEC-05) — and `TPM_NONOTIFY` suppresses the notifications that would
    // otherwise accompany the choice.
    let chosen = unsafe {
        TrackPopupMenuEx(
            menu.handle(),
            (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0,
            x,
            y,
            hwnd,
            None,
        )
    };

    // The second half of the workaround: without a message arriving after the menu closes,
    // the window that tracked it can be left showing a menu that never repaints away.
    //
    // SAFETY: `hwnd` is our live window; `WM_NULL` is the message that does nothing by
    // definition, and both parameters are zero, so nothing is dereferenced on either side.
    if let Err(error) = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) } {
        app::report_non_critical("PostMessageW(WM_NULL)", &error);
    }

    // NFR-13 has nothing to check on `chosen`: under `TPM_RETURNCMD` the `BOOL` is not a
    // success flag but the identifier of the item chosen, and zero means the menu was
    // dismissed without a choice. The two are indistinguishable by design, and "the user
    // changed their mind" is not an error.
    if let Ok(command) = u32::try_from(chosen.0) {
        dispatch_command(hwnd, command);
    }
}

/// Carries out one menu command.
///
/// Reached only from [`show_menu`], that is, only from a menu the user opened. Nothing here
/// is reachable by sending this process a message — see SEC-05 in the module documentation.
fn dispatch_command(hwnd: HWND, command: u32) {
    match command {
        CMD_TOGGLE => {
            let _ = with_tray(Tray::toggle_state);
        }

        // FR-92, task T-08-1. Marked stub: the settings dialog is a `.rc` resource plus a
        // dialog procedure, neither of which exists yet, and a placeholder window invented
        // here would only have to be deleted again.
        // TODO(T-08-1): open the settings dialog of FR-92.
        CMD_SETTINGS => {}

        // FR-93, task T-08-1. Marked stub, and deliberately not implemented here: FR-93 is
        // autostart through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, that is, a
        // write to the registry, and task T-01-4 is not allowed to make one. The check mark
        // shows `general.autostart`; choosing the item changes nothing.
        // TODO(T-08-1): write the `Run` value and update `general.autostart`.
        CMD_AUTOSTART => {}

        // Outside any borrow of the tray: `MessageBoxW` is modal and pumps messages.
        CMD_ABOUT => show_about(hwnd),

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

// ---------------------------------------------------------------------------------------
// The about box
// ---------------------------------------------------------------------------------------

/// Shows the "О программе" window: the name and the version, and nothing else.
///
/// Russian, like the menu, and hard-wired for the same reason the menu labels are: FR-91
/// prints them in Russian, and FR-94 — the RU and EN resource strings — is task T-08-2,
/// which owns `app.rc`. Moving these strings into the resources belongs with the rest of
/// FR-94.
fn show_about(hwnd: HWND) {
    let version = file_version().map_or_else(
        || "версия недоступна".to_owned(),
        |(major, minor, build, revision)| format!("Версия {major}.{minor}.{build}.{revision}"),
    );

    let text = wide(&format!("{APP_NAME}\n{version}"));
    let caption = wide(APP_NAME);

    // SAFETY: both strings are NUL-terminated UTF-16 buffers owned by this frame, neither
    // moved nor dropped until the call returns, and `MessageBoxW` only reads through them.
    // `hwnd` is our live window and becomes the owner of the box, which is what keeps the box
    // in front of it. The call blocks and pumps messages, which is why it is made here and
    // not from inside `with_tray`.
    let result = unsafe {
        MessageBoxW(
            Some(hwnd),
            PCWSTR(text.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
        )
    };

    if result.0 == 0 {
        // NFR-13: zero is the documented failure value. Not fatal — nothing depends on the
        // box having been shown.
        app::report_non_critical("MessageBoxW", &WinError::from_thread());
    }
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

/// The size the shell wants a notification-area icon in — FR-90.
///
/// Asked of the system rather than fixed at 16: the value follows the display scale, and a
/// hard-wired 16 on a scaled monitor is stretched into a blurred icon. Both `.ico` files
/// carry 16/20/24/32 px frames, and `LoadImageW` picks the frame nearest what is asked for.
pub fn small_icon_size() -> (i32, i32) {
    // SAFETY: `GetSystemMetrics` reads a system-wide value, takes no pointer and touches no
    // memory of ours. It returns zero for an index the system does not know, which is why
    // the result is examined below rather than passed on (NFR-13): a zero would ask
    // `LoadImageW` for the resource's own size and quietly defeat the point of the call.
    let width = unsafe { GetSystemMetrics(SM_CXSMICON) };

    // SAFETY: as above.
    let height = unsafe { GetSystemMetrics(SM_CYSMICON) };

    // A metric that came back zero or negative is unusable. Sixteen is what the small-icon
    // metric is at 100% scale and is the safe fallback.
    let usable = |value: i32| if value > 0 { value } else { 16 };

    (usable(width), usable(height))
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

/// A NUL-terminated UTF-16 copy of `text`, for the Win32 calls that want one.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copies `text` into the fixed tooltip field, always leaving it NUL-terminated.
///
/// The field holds 128 UTF-16 units including the terminator, and what goes into it is a name
/// and a state word, so the truncating branch is unreachable in practice. It exists so that
/// no future edit can make the field overrun.
fn write_tip(field: &mut [u16; 128], text: &str) {
    let limit = field.len() - 1;
    let mut written = 0;

    for unit in text.encode_utf16().take(limit) {
        field[written] = unit;
        written += 1;
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
        write_tip(&mut field, "Lang Switcher — активна");

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
        write_tip(&mut field, &long);

        assert_eq!(field[127], 0, "the last unit must be the terminator");
        assert!(
            field[..127].iter().all(|unit| *unit == 0x044F),
            "the field must be filled with the text and nothing left over"
        );
    }

    #[test]
    fn the_small_icon_metric_is_a_usable_size() {
        let (width, height) = small_icon_size();

        assert!(
            width > 0 && height > 0,
            "SM_CXSMICON and SM_CYSMICON must give a usable size"
        );
    }
}
