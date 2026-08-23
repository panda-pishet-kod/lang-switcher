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
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; T-11-10 (done) put the menu items
//! on `MF_OWNERDRAW` and paints them in the palette of FR-92а; T-11-21 (done) brought the
//! smoothing wave of tasks T-11-13…T-11-20 to this file — the check mark and the highlight
//! under the cursor are the smoothed figures of [`crate::settings`] and the entries are set
//! in its grey-antialiased face, so that one product no longer has two qualities of drawing;
//! T-11-22 (done) took the rest of the menu off the system — the two rules of FR-91 are drawn
//! by this file as well, the ground the entries stand on is the palette's rather than the
//! shell's ([`set_menu_background`]), and an entry is the height the mock-up gives it.
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
//! covers the about dialog (`settings::show_about_dialog`, task T-11-11), which is modal
//! for the same reason.
//!
//! The owner-draw state of task T-11-10 obeys the same rule from the other side: while
//! `TrackPopupMenuEx` runs its loop, `WM_MEASUREITEM` and `WM_DRAWITEM` re-enter this
//! module, so what they need lives in [`MENU_PAINT`] — a thread-local of its own, borrowed
//! only for the length of one message and never across the modal call, exactly like
//! [`UI_TRAY`] itself.
//!
//! # SEC-05
//!
//! Only the messages named in [`Tray::handle_message`] are handled; everything else is
//! answered with [`Reaction::Ignored`] and goes to `DefWindowProcW`. The callback message of
//! the icon can be posted by any process at the same integrity level, and all that achieves
//! is a menu on the screen — or, for the double click, the settings dialog of FR-92, which is
//! the same kind of thing: a window that changes nothing until the person in front of it
//! presses «Применить». Neither is a privileged action, neither writes anything anywhere, and
//! both are dismissed by pressing `Esc`. There is deliberately **no `WM_COMMAND` handler**: the menu is
//! tracked with `TPM_RETURNCMD`, so the chosen command comes back as the return value of
//! `TrackPopupMenuEx` instead of arriving as a message. A forged `WM_COMMAND` therefore
//! cannot pause the program or shut it down — no privileged action is reachable by message.
//!
//! Task T-11-10 (FR-92а) draws the menu items itself, which brings `WM_MEASUREITEM` and
//! `WM_DRAWITEM` to this window. Both are behind a gate: the thread-local [`MENU_PAINT`]
//! holds `Some` exactly while our menu is on the screen — set immediately before
//! `TrackPopupMenuEx`, cleared immediately after it returns — and outside that window both
//! messages are [`Reaction::Ignored`] like everything else foreign. The `itemData` of every
//! entry is a number and never a pointer — the command number for the five commands of FR-91,
//! the one reserved [`MENU_SEPARATOR_DATA`] for the two rules task T-11-22 also draws — so
//! nothing a forged message carries is dereferenced beyond the identifier check and the drawing
//! rectangle — the SEC-05 wording verbatim.
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

use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreateSolidBrush, DT_NOCLIP, DT_SINGLELINE, DT_VCENTER, DeleteObject,
    DrawTextW, FillRect, GetDC, GetTextExtentPoint32W, HBRUSH, HDC, HFONT, HGDIOBJ, LOGFONTW,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_SELECTED, ODT_MENU};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetSystemMetrics, HICON, HMENU,
    IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW, MENUINFO, MF_CHECKED, MF_OWNERDRAW, MF_SEPARATOR,
    MF_UNCHECKED, MIM_BACKGROUND, NONCLIENTMETRICSW, PostMessageW, RT_VERSION,
    RegisterWindowMessageW, SM_CXMENUCHECK, SM_CXSMICON, SM_CYMENU, SM_CYSMICON,
    SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetForegroundWindow, SetMenuInfo,
    SystemParametersInfoW, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP,
    WM_CONTEXTMENU, WM_DRAWITEM, WM_ENDSESSION, WM_LBUTTONDBLCLK, WM_MEASUREITEM, WM_NULL,
    WM_QUERYENDSESSION, WM_SETTINGCHANGE, WM_THEMECHANGED, WM_USER,
};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult, w};

use crate::settings::{self, Config};
use crate::{APP_NAME, app, theme};

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

/// Horizontal padding of an owner-drawn menu item: before the check column and after the
/// text — FR-92а, task T-11-10.
pub const MENU_H_PAD: i32 = 6;

/// Gap between the check column and the text of an owner-drawn menu item.
pub const MENU_CHECK_GAP: i32 = 4;

/// Vertical padding above and below the text of an owner-drawn menu entry, in **tenths of a
/// pixel of the mock-up** — FR-92а, task T-11-22.
///
/// ⚠ The number is the mock-up's own air and not one this file chose. The generator gives an
/// entry `$itemH = 38` of its pixels and sets it in `Segoe UI` 12,6 pt — which at the 140 % the
/// generator draws at is the 9 pt of the system menu face, the very face [`menu_item_logfont`]
/// asks for. `GetTextExtentPoint32W` measures that face 15 screen pixels tall at 96 DPI, and 15
/// screen pixels are 21 of the mock-up's; what is left of the entry is the air above and below
/// the line: (38 − 21) / 2 = **8,5 mock-up pixels**.
///
/// So the reference height enters as **padding** and does not replace the measurement:
/// [`menu_item_height`] still adds this to the text it measured, and a larger interface face
/// still makes a taller row — which a fixed 38 would not.
///
/// ⚠ Until this task the constant was `5` **screen** pixels, written by task T-11-10 before the
/// project had the discipline of mock-up lengths, and the entry came out 25 px against the
/// mock-up's 27,1.
pub const MENU_V_PAD_TENTHS: i32 = 85;

/// Height of the stripe one rule of FR-91 takes, in the pixels of the mock-up — the `$sepH = 11`
/// of the generator, task T-11-22.
pub const MENU_SEP_H: i32 = 11;

/// Inset of the line of a rule from the left and the right edge of the menu, in the pixels of
/// the mock-up — the `($mX+12)` and `($mX+$mW-12)` of
/// `$g.DrawLine($pen, ($mX+12), ($iy + $sepH/2), ($mX+$mW-12), ($iy + $sepH/2))`.
pub const MENU_SEP_INSET: i32 = 12;

/// Thickness of that line, in **screen** pixels — the one length of this menu that is not
/// stated in the pixels of the mock-up.
///
/// The generator strokes the line with `New-Object System.Drawing.Pen($T.Sep,[single]1)`, and a
/// pen of GDI+ straddles the path it follows: one pen pixel covers two rows of the picture and
/// measures as two — the reasoning [`crate::settings::BORDER_THICKNESS`] carries for the frames
/// of the dialog. One screen pixel is what that comes to at every scale these two windows live
/// at, and it is what the mock-up shows.
const MENU_SEP_THICKNESS: i32 = 1;

/// The `itemData` both rules of FR-91 carry — FR-92а, task T-11-22.
///
/// A number, exactly like the `itemData` of a command entry, and one no command of this file
/// equals: those are `0x1001`…`0x1005`. SEC-05 is untouched by it — the handler looks the
/// number up in what it already holds and dereferences nothing.
///
/// ⚠ A rule is appended with a command identifier of **zero** and carries this only as its
/// `itemData`, so `TrackPopupMenuEx` can never return it as a choice; the identifier and the
/// data are two different arguments of `AppendMenuW` and only the second one reaches the
/// drawing. Measured on this machine: the `WM_MEASUREITEM` and `WM_DRAWITEM` of a rule arrive
/// with `itemID` = 0 and `itemData` = this number.
pub const MENU_SEPARATOR_DATA: u32 = 0x10FF;

/// The check mark of FR-93 as the mock-up draws it — FR-92а, task T-11-21.
///
/// Read out of `scratchpad\chrome.ps1`, which is the generator the mock-up `ui-06-chrome.png`
/// was drawn by, and not measured off the picture: the menu arm strokes
/// `(PtF $bx ($by+1))`, `(PtF ($bx+4) ($by+5))`, `(PtF ($bx+11) ($by-6))` with
/// `New-Object System.Drawing.Pen($T.Fg,[single]2.1)`, where `$bx` is the left edge of the
/// mark and `$by` the vertical middle of the entry.
///
/// ⚠ The generator draws at 140 %, so every one of its pixels is 1,4 screen pixels — which is
/// exactly what [`crate::settings::scaled_tenths_offset`] divides by, and the reason these
/// literals are the generator's own numbers rather than numbers somebody has already divided.
///
/// The offsets are measured from the corner of [`menu_check_cell`] and not from `($bx, $by)`:
/// [`crate::settings::draw_check_mark`] clamps the tile it smooths in to the square it is
/// given, so the square has to hold the pen and the fading edge as well as the path. That is
/// the whole of [`MENU_CHECK_AIR`] — the three points below are the generator's, moved by it.
pub const MENU_CHECK_MARK: settings::CheckMark = settings::CheckMark {
    points_tenths: [(30, 100), (70, 140), (140, 30)],
    pen_tenths: 21,
};

/// Air around the path of [`MENU_CHECK_MARK`] inside the square it is drawn in, in the pixels
/// of the mock-up.
///
/// Not a number of the mock-up and not meant to be one: it is what keeps the tile of
/// [`crate::settings::draw_check_mark`] — which is clamped to this square — from cutting the
/// ends of the strokes off. That tile is `settings::stroke_bounds`, which reaches
/// `thickness / 2 + 1` **screen** pixels past the outermost point: two of them at 96 DPI, and
/// two mock-up pixels are only 1,43 of those. Three is the first number that covers it, and a
/// test walks four display scales to say so.
///
/// The air is added on all four sides, so it moves the path inside the square and not on the
/// screen: the mark lands on the same pixels it would have without it.
pub const MENU_CHECK_AIR: i32 = 3;

/// Side of the square [`MENU_CHECK_MARK`] is drawn in, in the pixels of the mock-up.
///
/// The path of the generator spans eleven of its pixels each way — `$bx … $bx+11` across and
/// `$by-6 … $by+5` down — and [`MENU_CHECK_AIR`] is added on each of the four sides.
pub const MENU_CHECK_CELL: i32 = 11 + 2 * MENU_CHECK_AIR;

/// Inset of the highlight under the cursor from the left and the right edge of the entry, in
/// the pixels of the mock-up — the `($mX+5)` and `($mW-10)` of
/// `FillR $g ($mX+5) $iy ($mW-10) $itemH $T.Hover 6`.
///
/// Left and right only: the generator gives the stripe the whole height of the entry.
pub const MENU_HOVER_INSET: i32 = 5;

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
    /// Handled, except that the settings dialog of FR-92 has to be opened.
    ///
    /// A variant for the same reason [`Reaction::ShowMenu`] is one: the dialog is modal, it runs
    /// a message loop that comes back into this module, and it must therefore not be opened
    /// while the tray is borrowed.
    ShowSettings,
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

    /// Whether `general.autostart` is set. The check mark of FR-91 shows this.
    ///
    /// What the *system* will actually do is [`settings::autostart_registered`], and the two are
    /// kept equal by [`Tray::set_autostart`] — FR-93. They are read separately on purpose: the
    /// settings dialog shows both, so that a value somebody removed from the registry by hand
    /// is visible rather than merely wrong.
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
    /// the menu is not a message. The measure/draw pair of the owner-drawn menu (FR-92а)
    /// is answered only while this program itself holds the menu on the screen — the gate
    /// of [`MENU_PAINT`]; at any other moment both are as foreign as anything else. The
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

            // FR-92а, task T-11-10: the measuring and the drawing of our own menu's
            // entries. Both are behind the SEC-05 gate — answered only while
            // [`MENU_PAINT`] says a menu of ours is on the screen, and `Ignored` at any
            // other moment, before anything of the message is dereferenced.
            WM_MEASUREITEM => measure_menu_item(lparam),
            WM_DRAWITEM => draw_menu_item(lparam),

            // FR-92а, task T-11-9: the two messages Windows broadcasts when the system
            // theme moves, received here because the hidden UI window is the one top-level
            // window of this process. The whole reaction is a call that may post one
            // private message to the open settings dialog — a closed dialog and a fixed
            // `theme != "system"` are both «nothing happens», decided in `settings`.
            // `Ignored`, not `Handled`: the broadcast goes on to `DefWindowProcW` like any
            // other message that is not ours to consume.
            //
            // SEC-05 verbatim: the string `lParam` names is compared against
            // `"ImmersiveColorSet"` and is used for nothing further. The reading is
            // careful — a null pointer is not ours, at most [`SETTING_NAME_CAP`] UTF-16
            // units are ever looked at, the comparison is exact — and nothing of the
            // message is kept: the copy below lives on this frame and dies with it.
            WM_SETTINGCHANGE => {
                // SAFETY: for the length of this delivery the system keeps the string the
                // broadcast names readable in this process, and the reader checks for null
                // and never looks past the terminator or the cap.
                let name = unsafe { setting_change_name(lparam) };

                settings::on_system_theme_message(name.as_deref());

                Reaction::Ignored
            }

            // `WM_THEMECHANGED` carries no string: it *is* the theme notification, by its
            // own name, so it enters the same decision as a `WM_SETTINGCHANGE` that named
            // the switch — one path after the string check, not two.
            WM_THEMECHANGED => {
                settings::on_system_theme_message(Some(settings::IMMERSIVE_COLOR_SET));

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
            WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT => Reaction::ShowMenu {
                x: coordinate(low_word(wparam.0)),
                y: coordinate(high_word(wparam.0)),
            },

            // A double click opens the settings dialog of FR-92, which is the convention every
            // resident program on this system follows. With the mouse this arm is not usually
            // reached — the first click of the pair has already opened the menu and the second
            // is consumed by the menu's own modal loop — so «Настройки…» in the menu stays the
            // documented way in, and this is the shortcut for whoever expects it to work.
            WM_LBUTTONDBLCLK => Reaction::ShowSettings,

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
    /// Saving is not optional here: the tray is the in-memory owner of the configuration and
    /// writes it out again at shutdown (FR-83), so a change kept only in the file would be
    /// overwritten by this copy on the way out.
    pub fn replace_config(&mut self, config: Config) {
        self.config = config;
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

/// The ceiling on how many UTF-16 units of a `WM_SETTINGCHANGE` string are ever looked at —
/// FR-92а, task T-11-9, the reading bounds of SEC-05.
///
/// `"ImmersiveColorSet"` is 17 units; 64 is that with room to spare, and a string still
/// running at 64 is one this program was never going to match.
const SETTING_NAME_CAP: usize = 64;

/// The string a `WM_SETTINGCHANGE` names, read within the bounds of SEC-05 — task T-11-9.
///
/// `None` for a null `lParam` (then it is not ours), for a string that shows no terminator
/// within [`SETTING_NAME_CAP`] units, and for units that are not UTF-16 — whatever either of
/// those is, it is not the one word this program compares against. The value lives on the
/// caller's frame and dies with the comparison it was read for: nothing of the message is
/// kept.
///
/// # Safety
///
/// `lparam` must be the `lParam` of a `WM_SETTINGCHANGE` delivered to this thread: for the
/// length of the delivery the system keeps the string it names readable in this process, and
/// the loop below reads one unit at a time, never past the terminator and never past the
/// cap.
unsafe fn setting_change_name(lparam: LPARAM) -> Option<String> {
    if lparam.0 == 0 {
        return None;
    }

    let text = lparam.0 as *const u16;
    let mut units = Vec::with_capacity(SETTING_NAME_CAP);

    for offset in 0..SETTING_NAME_CAP {
        // SAFETY: see above — every unit before `offset` was read and found non-zero, so
        // the read is behind the terminator and below the cap.
        let unit = unsafe { text.add(offset).read_unaligned() };

        if unit == 0 {
            return String::from_utf16(&units).ok();
        }

        units.push(unit);
    }

    None
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
        // Outside the borrow for the same reason: the dialog of FR-92 is modal.
        Reaction::ShowSettings => {
            let hwnd = with_tray(|tray| tray.hwnd)?;
            open_settings(hwnd);
            Some(LRESULT(0))
        }
    }
}

// ---------------------------------------------------------------------------------------
// The menu — FR-91
// ---------------------------------------------------------------------------------------

/// One command entry of FR-91 as the builder recorded it — task T-11-10.
///
/// With `MF_OWNERDRAW` the label no longer lives inside Windows' menu: this record is where
/// it lives instead, found by command number both by the drawing of [`MenuPaint`] and by
/// the tests that check FR-91 entry for entry. SEC-05 is why the lookup is by number: the
/// entry's `itemData` carries the command and nothing else, so no message can hand the
/// drawing a pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// The command identifier — the same number the entry carries as its `itemData`.
    pub command: u32,
    /// The label, out of the string table of the locale in force (FR-94).
    pub label: String,
    /// Whether the entry shows the check mark of FR-93.
    pub checked: bool,
}

/// The context menu of FR-91, destroyed when this value is dropped.
///
/// Built afresh for every click rather than kept and patched: the first label and the check
/// mark depend on state that can change between two clicks, and a menu built from the state
/// it is about cannot show a stale one.
pub struct Menu {
    handle: HMENU,
    /// The command entries in menu order — the builder's own record of what it appended,
    /// and since task T-11-10 the only place the labels exist at all.
    items: Vec<MenuItem>,
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

        let mut menu = Self {
            handle,
            items: Vec::with_capacity(5),
        };

        // FR-94: every label out of the string table of the locale in force. The menu is built
        // afresh on every click, so it carries the locale published at start-up without any
        // state of its own.
        let first = settings::text(if enabled {
            settings::IDS_MENU_SUSPEND
        } else {
            settings::IDS_MENU_RESUME
        });

        menu.append_command(CMD_TOGGLE, &first, false)?;
        menu.append_separator()?;
        menu.append_command(
            CMD_SETTINGS,
            &settings::text(settings::IDS_MENU_SETTINGS),
            false,
        )?;
        menu.append_command(
            CMD_AUTOSTART,
            &settings::text(settings::IDS_MENU_AUTOSTART),
            autostart,
        )?;
        menu.append_separator()?;
        menu.append_command(CMD_ABOUT, &settings::text(settings::IDS_MENU_ABOUT), false)?;
        menu.append_command(CMD_EXIT, &settings::text(settings::IDS_MENU_EXIT), false)?;

        Ok(menu)
    }

    /// The raw handle, for `TrackPopupMenuEx` and for the tests that read the menu back with
    /// `GetMenuItemCount`, `GetMenuState` and `GetMenuItemInfoW`.
    pub fn handle(&self) -> HMENU {
        self.handle
    }

    /// The command entries in menu order, labels and check marks included — the record the
    /// drawing of [`MenuPaint`] and the tests of FR-91 both read.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Appends one command entry as `MF_OWNERDRAW` — FR-92а, task T-11-10.
    ///
    /// The label no longer travels through `AppendMenuW`: it is recorded in [`Menu::items`],
    /// where [`MenuPaint`] finds it by command number at drawing time. What `AppendMenuW`
    /// gets as its last argument is the entry's `itemData`, and that is **the command
    /// number and nothing else** — SEC-05: a `WM_DRAWITEM` can be forged by any process of
    /// our integrity level, dereferencing a pointer taken out of one would be a hole, and a
    /// number a handler merely looks up is not.
    ///
    /// The check mark of FR-93 is still declared to Windows as `MF_CHECKED` — that is what
    /// keeps the entry's state readable from outside, `GetMenuState` and screen readers
    /// alike — and recorded in the entry, which is what the drawing paints by.
    fn append_command(&mut self, command: u32, label: &str, checked: bool) -> WinResult<()> {
        let mark = if checked { MF_CHECKED } else { MF_UNCHECKED };

        // SAFETY: `self.handle` is a live menu created by `CreatePopupMenu` and owned by
        // this value. `command` is one of our own non-zero identifiers. Under
        // `MF_OWNERDRAW` the last argument is not read as a string: it is carried verbatim
        // as the entry's `itemData`, and what is placed there is the command number — an
        // address-shaped value with no allocation behind it, which `without_provenance`
        // says in so many words, and which nothing ever dereferences (SEC-05). The crate
        // turns the `BOOL` into a `Result`, so NFR-13 is satisfied by the `?`.
        unsafe {
            AppendMenuW(
                self.handle,
                MF_OWNERDRAW | mark,
                usize::try_from(command).unwrap_or(0),
                PCWSTR(std::ptr::without_provenance(
                    usize::try_from(command).unwrap_or(0),
                )),
            )
        }?;

        self.items.push(MenuItem {
            command,
            label: label.to_owned(),
            checked,
        });

        Ok(())
    }

    /// Appends one of the two rules of FR-91, drawn by this program — task T-11-22.
    ///
    /// `MF_SEPARATOR | MF_OWNERDRAW`, and **both** flags are the point:
    ///
    /// * `MF_OWNERDRAW` is what brings `WM_MEASUREITEM` and `WM_DRAWITEM` for the rule, so the
    ///   stripe is the palette's and not the shell's engraved groove of FR-92а's «цена»;
    /// * `MF_SEPARATOR` is what keeps the entry a **rule** for everybody outside this program —
    ///   `GetMenuState` still answers `MF_SEPARATOR`, accessibility still reads a separator, and
    ///   the keyboard still steps over it.
    ///
    /// ⚠ That the pair works together at all is a measurement of task T-11-22 and not a reading
    /// of the documentation. On this machine both messages arrive for such an entry, the height
    /// the measurement returns is the height the system lays out, and `↓` walks the entries
    /// **round** it — 0 → 2 → 3 → 4. The alternative the task weighed,
    /// `MF_OWNERDRAW | MF_DISABLED | MF_GRAYED` without `MF_SEPARATOR`, was measured on the same
    /// menu in the same run and rejected: the keyboard **stops** on it, which would leave the
    /// cursor standing on an empty stripe.
    ///
    /// The identifier is zero — a rule is not a command and `TrackPopupMenuEx` must never
    /// return one — and the `itemData` is [`MENU_SEPARATOR_DATA`], which is what the drawing
    /// recognises the entry by.
    fn append_separator(&self) -> WinResult<()> {
        // SAFETY: as above. `self.handle` is a live menu owned by this value. Under
        // `MF_OWNERDRAW` the last argument is not read as a string: it is carried verbatim as
        // the entry's `itemData`, and what is placed there is a number with no allocation
        // behind it — `without_provenance` says so in as many words — which nothing ever
        // dereferences (SEC-05). The crate turns the `BOOL` into a `Result`, so NFR-13 is
        // satisfied by the return.
        unsafe {
            AppendMenuW(
                self.handle,
                MF_SEPARATOR | MF_OWNERDRAW,
                0,
                PCWSTR(std::ptr::without_provenance(
                    usize::try_from(MENU_SEPARATOR_DATA).unwrap_or(0),
                )),
            )
        }
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

// ---------------------------------------------------------------------------------------
// The drawing of the menu — FR-92а, task T-11-10
// ---------------------------------------------------------------------------------------

/// Everything `WM_MEASUREITEM` and `WM_DRAWITEM` need while the menu of FR-91 is on the
/// screen — and, by being the `Some` of [`MENU_PAINT`], the SEC-05 gate itself.
///
/// # Life of one showing — FR-92а
///
/// Built by [`show_menu`] immediately before `TrackPopupMenuEx` and dropped immediately
/// after it returns. The palette is resolved exactly once per showing — the setting out of
/// the configuration, the system switch read once — and the three brushes and the font live
/// exactly that long. `theme::Brushes` is deliberately not used: that set belongs
/// to the long-lived dialog of FR-92, while these die with the menu.
///
/// ⚠ Since task T-11-22 one of those brushes is handed to Windows as well — `SetMenuInfo`
/// takes [`MenuPaint::window_bg`] as the ground of the menu, and Windows paints with it until
/// the menu goes away. That is the same «exactly that long», which is why the brush is this
/// one and not a second one made for the purpose.
///
/// # Why a thread-local of its own — the rule of the module header
///
/// While `TrackPopupMenuEx` runs its modal loop, every message re-enters this module, so
/// the state the measure/draw handlers read must not sit behind the borrow of [`UI_TRAY`]
/// and must not require [`show_menu`] to hold any borrow across the modal call.
/// [`MENU_PAINT`] is borrowed for the length of one message and released, exactly like
/// [`UI_TRAY`] in [`handle_ui_message`], and nothing running under its borrow pumps
/// messages.
struct MenuPaint {
    /// The entries of the menu being shown — labels found by the command number that
    /// arrives as `itemData`.
    items: Vec<MenuItem>,
    /// The palette of this showing, resolved once — FR-92а.
    palette: &'static theme::Palette,
    /// The menu face of [`menu_item_logfont`], created for this showing.
    font: HFONT,
    /// Brush of [`theme::Palette::window_bg`] — the background of an entry at rest, and,
    /// since task T-11-22, the ground of the whole menu: it is this brush that
    /// [`set_menu_background`] hands to `SetMenuInfo`, so the ground under the entries and
    /// the ground between them are one brush and not two.
    window_bg: HBRUSH,
    /// Brush of [`theme::Palette::hover_bg`] — the fill of the rounded stripe under the
    /// entry the cursor is on (`ODS_SELECTED`).
    hover_bg: HBRUSH,
    /// Brush of [`theme::Palette::panel_border`] — the line of a rule, task T-11-22.
    panel_border: HBRUSH,
}

thread_local! {
    /// The state of the menu now on the screen — `Some` exactly from just before
    /// `TrackPopupMenuEx` until just after it returns. This *is* the gate SEC-05 asks
    /// for: [`measure_menu_item`] and [`draw_menu_item`] answer [`Reaction::Ignored`]
    /// while it holds `None`, before anything of the message is dereferenced.
    static MENU_PAINT: RefCell<Option<MenuPaint>> = const { RefCell::new(None) };
}

impl MenuPaint {
    /// Creates the paint state of one showing, every handle checked.
    ///
    /// `None` when the non-client metrics cannot be read or any GDI object is refused —
    /// the same contract as `theme::Brushes::new`, for the same reasons: NFR-13 does not
    /// allow painting with a handle nobody looked at, and nothing goes to the journal
    /// because these calls do not promise a last-error code. The caller shows the menu
    /// anyway — unpainted rows over no menu at all.
    fn new(items: Vec<MenuItem>, palette: &'static theme::Palette) -> Option<Self> {
        // NFR-13: `None` — the non-client metrics were refused. Without them there is no
        // menu face to measure with, and a guessed face would mislabel every measurement
        // that follows. Task T-11-21: the face that comes back is already the smoothed
        // one — see [`menu_item_logfont`].
        let face = menu_item_logfont()?;

        // SAFETY: `face` is a live local of this frame, read only for the length of the
        // call. The handle, if valid, becomes the property of the value below and is
        // destroyed exactly once — in `Drop`, or right here on the partially failed branch.
        let font = unsafe { CreateFontIndirectW(&raw const face) };

        // SAFETY: all three calls take a colour by value, read no memory of ours and return
        // a handle; every handle is examined below, and the valid ones are owned exactly as
        // the font is.
        //
        // ⚠ There is no pen here since task T-11-21. The check mark is drawn by
        // [`crate::settings::draw_check_mark`], which owns a pen for exactly the one call —
        // a smoothed stroke is drawn twice, once enlarged and once not, with two different
        // thicknesses, so a pen made once for the showing could not have served it.
        let (window_bg, hover_bg, panel_border) = unsafe {
            (
                CreateSolidBrush(palette.window_bg),
                CreateSolidBrush(palette.hover_bg),
                CreateSolidBrush(palette.panel_border),
            )
        };

        let handles: [HGDIOBJ; 4] = [
            font.into(),
            window_bg.into(),
            hover_bg.into(),
            panel_border.into(),
        ];

        // NFR-13: every handle is examined before anybody paints with it.
        if handles.iter().any(HGDIOBJ::is_invalid) {
            for handle in handles.into_iter().filter(|handle| !handle.is_invalid()) {
                // SAFETY: `handle` came from one of the successful creations above, was
                // handed to nobody, and this loop is the only thing that frees it — the
                // failed constructor returns `None` and no `Drop` will run.
                let deleted = unsafe { DeleteObject(handle) };

                // NFR-13: examined — loudly where the tests run, in words here; the
                // reasoning of `theme::Brushes::new`, which faces the same cleanup.
                debug_assert!(
                    deleted.as_bool(),
                    "DeleteObject refused an object just made"
                );
            }

            return None;
        }

        Some(Self {
            items,
            palette,
            font,
            window_bg,
            hover_bg,
            panel_border,
        })
    }

    /// The entry `command` names, if the menu being shown has one.
    ///
    /// `None` for any other number — which is the whole of what a forged `itemData` can
    /// achieve while the gate is up (SEC-05): a lookup that finds nothing.
    fn item(&self, command: u32) -> Option<&MenuItem> {
        self.items.iter().find(|item| item.command == command)
    }

    /// Measures one label with the menu font — the arithmetic of `WM_MEASUREITEM`.
    ///
    /// Width is the text plus the check column on the left and the paddings around both;
    /// height is [`menu_item_height`] — the text as measured plus the air the mock-up leaves
    /// above and below it (task T-11-22). The width includes the check column on purpose,
    /// even though some Windows versions add room of their own around owner-drawn entries: a
    /// menu a few pixels wider than the minimum reads fine, a clipped label does not.
    fn measure(&self, label: &str) -> (u32, u32) {
        let units: Vec<u16> = label.encode_utf16().collect();
        let mut extent = SIZE::default();
        let mut dpi = settings::SCREEN_DPI;

        // SAFETY: `None` asks for a DC of the screen, which needs no window of ours; a
        // valid handle is released below, on this same thread, as `ReleaseDC` requires.
        let dc = unsafe { GetDC(None) };

        if !dc.is_invalid() {
            // The message carries no device context — an entry is measured before there is
            // a menu window to measure it on — so the DPI comes off the same screen DC the
            // text is measured with, which is the one [`measure_dpi`] would open anyway.
            dpi = settings::dc_dpi(dc);

            // SAFETY: `dc` is the live DC just obtained and `self.font` is the live font
            // this value owns; the previous selection is restored below, before the DC is
            // released.
            let previous = unsafe { SelectObject(dc, self.font.into()) };

            // SAFETY: `units` is owned by this frame and only read; `extent` is a live
            // local the call writes one `SIZE` into, and it keeps neither pointer.
            let measured = unsafe { GetTextExtentPoint32W(dc, &units, &mut extent) };

            // NFR-13: examined. A refused measurement falls into the metric fallback
            // below rather than into a zero-height menu row.
            if !measured.as_bool() {
                extent = SIZE::default();
            }

            // SAFETY: restores what was selected before; both handles are live — the DC
            // until the release below, the old object because the DC still refers to it.
            let _ = unsafe { SelectObject(dc, previous) };

            // SAFETY: the DC came from the `GetDC` above, on this same thread. NFR-13:
            // nothing useful can be done about a refused release, and it is not made
            // fatal — the DC was ours for one measurement.
            let _ = unsafe { ReleaseDC(None, dc) };
        }

        let height = if extent.cy > 0 {
            menu_item_height(extent.cy, dpi)
        } else {
            // The screen DC or the measurement was refused. `SM_CYMENU` is the system's
            // own row height; nineteen is that metric at 100% scale.
            //
            // SAFETY: reads a system-wide metric, takes no pointer, touches no memory of
            // ours.
            let metric = unsafe { GetSystemMetrics(SM_CYMENU) };

            if metric > 0 { metric } else { 19 }
        };

        let width = MENU_H_PAD + check_column() + MENU_CHECK_GAP + extent.cx + MENU_H_PAD;

        // The conversions cannot fail on real coordinates; zero on the impossible branch
        // gives Windows a degenerate entry rather than this thread a panic.
        (
            u32::try_from(width).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
        )
    }

    /// Measures one rule of FR-91 — the `WM_MEASUREITEM` of an entry that carries no text,
    /// task T-11-22.
    ///
    /// Height is [`MENU_SEP_H`] of the mock-up's pixels through the display scale. Width is
    /// the least the line itself needs — its two insets — and deliberately not the width of
    /// the menu: the width of a menu is the width of its widest entry, and a rule that
    /// claimed the labels' width would be measuring the menu by its rules.
    ///
    /// ⚠ The rule is nevertheless *drawn* across the whole menu, because Windows lays every
    /// entry of a menu out at the width of the widest. Measured on this machine at 96 DPI: the
    /// two rules asked for 18 pixels and came back to `WM_DRAWITEM` as rectangles 214 wide,
    /// the width of the labels — which is why [`menu_separator_line`] takes its insets from
    /// the rectangle it is given and not from anything this function returns.
    fn measure_rule(&self) -> (u32, u32) {
        let dpi = measure_dpi();
        let inset = settings::scaled(MENU_SEP_INSET, dpi);
        let height = settings::scaled(MENU_SEP_H, dpi);

        // As above: zero on the impossible branch rather than a panic on the UI thread.
        (
            u32::try_from(2 * inset).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
        )
    }

    /// Paints one rule of FR-91 — the drawing half of a `MF_SEPARATOR | MF_OWNERDRAW` entry,
    /// task T-11-22.
    ///
    /// The stripe is the ground `window_bg`, and across its vertical middle lies the line of
    /// [`menu_separator_line`] in `panel_border` — `$T.Bg` and `$T.Sep` of the generator. A
    /// rule is never `ODS_SELECTED`, which is measured and not assumed: the keyboard steps
    /// over it and the pointer cannot highlight it, so there is no second state to paint.
    ///
    /// Reads of `structure` are limited to `hDC` and `rcItem` — the SEC-05 list.
    fn draw_rule(&self, structure: &DRAWITEMSTRUCT) {
        let dc = structure.hDC;
        let rect = structure.rcItem;
        let dpi = settings::dc_dpi(dc);

        // SAFETY: `dc` and `rect` came with the message; while the gate is up they describe
        // an entry of our menu being painted, and the call only writes pixels into that DC.
        // `self.window_bg` is a live brush this value owns.
        let filled = unsafe { FillRect(dc, &rect, self.window_bg) };

        // NFR-13: examined in the only way available — a refused fill leaves the stripe
        // unpainted for one frame and nothing here can repair it. The reasoning of task
        // T-11-1: a refused GDI call is a `debug_assert!` and not an event of SEC-07.
        debug_assert!(filled != 0, "FillRect refused a live brush");

        let line = menu_separator_line(&rect, dpi);

        // SAFETY: as above — the message's DC and a rectangle computed from its own
        // `rcItem`; `self.panel_border` is a live brush this value owns.
        let drawn = unsafe { FillRect(dc, &line, self.panel_border) };

        // NFR-13: examined, as above.
        debug_assert!(drawn != 0, "FillRect refused a live brush");
    }

    /// Paints one entry — the drawing half of `WM_DRAWITEM`. FR-92а: the ground is
    /// `window_bg`, and the entry under the cursor carries the rounded `hover_bg` stripe of
    /// [`menu_hover_rect`] over it; text `text`, under the cursor `sel_fg`; the check mark
    /// is [`MenuPaint::draw_check_mark`]. There are no disabled entries in the menu of
    /// FR-91, so no third state is painted.
    ///
    /// Reads of `structure` are limited to `hDC`, `rcItem` and `itemState` — the SEC-05
    /// list; the entry itself was already found by [`MenuPaint::item`].
    fn draw(&self, structure: &DRAWITEMSTRUCT, item: &MenuItem) {
        let dc = structure.hDC;
        let rect = structure.rcItem;
        let selected = structure.itemState.0 & ODS_SELECTED.0 != 0;

        // The DPI of the device the entry is being painted on — every mock-up length below
        // goes through it. The menu window is a real window of the screen, so its DC
        // answers the scale of the monitor the menu came up on (NFR-13: a refusal is the
        // 100 % look, which `settings::dc_dpi` decides).
        let dpi = settings::dc_dpi(dc);

        // SAFETY: `dc` and `rect` came with the message; while the gate is up they
        // describe an entry of our menu being painted, and every call below only writes
        // pixels into that DC — the most a forged message can buy is a drawing on its own
        // DC. `self.window_bg` is a live brush this value owns.
        let filled = unsafe { FillRect(dc, &rect, self.window_bg) };

        // NFR-13: examined in the only way available — a refused fill leaves the row
        // unpainted for one frame and nothing here can repair it.
        debug_assert!(filled != 0, "FillRect refused a live brush");

        // Task T-11-21: the highlight is a **figure on top of the ground** and no longer
        // the ground itself, because the mock-up rounds it and holds it off the edges —
        // `FillR $g ($mX+5) $iy ($mW-10) $itemH $T.Hover 6`. `paint_rounded` smooths the
        // four corners the same way every rounded figure of the dialog is smoothed; the
        // outline is handed the fill's own colour, so the figure has a fill and no frame.
        if selected {
            settings::paint_rounded(
                dc,
                &menu_hover_rect(&rect, dpi),
                settings::scaled(settings::CORNER_RADIUS, dpi),
                self.palette.hover_bg,
                self.hover_bg,
                dpi,
            );
        }

        // SAFETY: `self.font` is live for the whole showing; the previous selection is
        // restored at the end of this function, while the DC is still the message's.
        let previous_font = unsafe { SelectObject(dc, self.font.into()) };

        // SAFETY: a state call on the message's DC — a mode by value, no memory of ours.
        let _ = unsafe { SetBkMode(dc, TRANSPARENT) };

        let colour = if selected {
            self.palette.sel_fg
        } else {
            self.palette.text
        };

        // SAFETY: as above — a colour by value.
        let _ = unsafe { SetTextColor(dc, colour) };

        let mut text_rect = RECT {
            left: rect.left + MENU_H_PAD + check_column() + MENU_CHECK_GAP,
            top: rect.top,
            right: rect.right - MENU_H_PAD,
            bottom: rect.bottom,
        };

        let mut units: Vec<u16> = item.label.encode_utf16().collect();

        // SAFETY: `units` and `text_rect` are owned by this frame for the whole call and
        // written by nobody else; the flags ask for one vertically centred line. NFR-13:
        // a zero return would mean nothing was drawn, and there is nothing to do about it
        // that the next paint will not do better.
        let _ = unsafe {
            DrawTextW(
                dc,
                &mut units,
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_NOCLIP,
            )
        };

        if item.checked {
            self.draw_check_mark(dc, &rect, dpi);
        }

        // SAFETY: restores the font that was selected when the message arrived.
        let _ = unsafe { SelectObject(dc, previous_font) };
    }

    /// The check mark of FR-93 — the smoothed figure of the mock-up, task T-11-21.
    ///
    /// Drawn by hand rather than by `DrawFrameControl(DFC_MENU, DFCS_MENUCHECK)`, because
    /// that call paints the system's mark in the system's colours whatever the palette of
    /// FR-92а says — the exact thing task T-11-10 existed to stop. Since task T-11-21 the
    /// hand is [`crate::settings::draw_check_mark`] and not this file's own two lines: the
    /// figure is [`MENU_CHECK_MARK`], the square is [`menu_check_cell`], and the smoothing
    /// is the one supersampling this program has — no second copy of it lives here.
    ///
    /// The ink is [`theme::Palette::text`] in both states, which is what it was before this
    /// task and what the mock-up draws (`$T.Fg`, on the highlighted row as on any other):
    /// the palette is task T-11-10's and is not touched here.
    fn draw_check_mark(&self, dc: HDC, rect: &RECT, dpi: i32) {
        let cell = menu_check_cell(rect, check_column(), dpi);

        settings::draw_check_mark(dc, &cell, self.palette.text, MENU_CHECK_MARK, dpi);
    }
}

impl Drop for MenuPaint {
    fn drop(&mut self) {
        let handles: [HGDIOBJ; 4] = [
            self.font.into(),
            self.window_bg.into(),
            self.hover_bg.into(),
            self.panel_border.into(),
        ];

        for handle in handles {
            // SAFETY: every handle came from a successful creation in `new` and is freed
            // exactly once: the type is neither `Copy` nor `Clone`, the fields are
            // private and never reassigned, and `drop` runs once. The menu the objects
            // painted is gone — `show_menu` drops this value only after `TrackPopupMenuEx`
            // has returned.
            let deleted = unsafe { DeleteObject(handle) };

            // NFR-13: examined — loudly where the tests run, in words here; the same
            // reasoning as `theme::Brushes`' own `Drop`, which faces the same refusal.
            debug_assert!(deleted.as_bool(), "DeleteObject refused an owned object");
        }
    }
}

/// Width of the check column of an owner-drawn menu entry — `SM_CXMENUCHECK`.
///
/// Asked of the system so the column follows the display scale, like the icon sizes of
/// FR-90. Sixteen is the metric at 100% scale and stands in for a metric the system
/// refused (NFR-13: a zero would collapse the column).
fn check_column() -> i32 {
    // SAFETY: reads a system-wide metric, takes no pointer and touches no memory of ours.
    let metric = unsafe { GetSystemMetrics(SM_CXMENUCHECK) };

    if metric > 0 { metric } else { 16 }
}

/// The highlight under the cursor, in the pixels of an entry whose rectangle is `item` —
/// FR-92а, task T-11-21.
///
/// [`MENU_HOVER_INSET`] of the mock-up off the left and the right edge and the whole height of
/// the entry, exactly as the generator draws it. Pure, so the geometry is a table a test reads
/// without a menu on the screen; the rounding of the corners is [`crate::settings::paint_rounded`]
/// with [`crate::settings::CORNER_RADIUS`], which is the `6` the same call carries.
pub fn menu_hover_rect(item: &RECT, dpi: i32) -> RECT {
    let inset = settings::scaled(MENU_HOVER_INSET, dpi);

    RECT {
        left: item.left + inset,
        top: item.top,
        right: item.right - inset,
        bottom: item.bottom,
    }
}

/// Height of one entry of FR-91 whose text measures `text_height` pixels, at `dpi` — FR-92а,
/// task T-11-22.
///
/// The measurement of the text plus the mock-up's own air on both sides of it,
/// [`MENU_V_PAD_TENTHS`]. Two properties at once, and the task asked for both: the row lands on
/// the mock-up's `$itemH = 38` for the face the mock-up is drawn in — 15 + 2 × 6 = **27** screen
/// pixels at 96 DPI, which is `settings::scaled(38, 96)` to the pixel — and it still **grows
/// with the face**, because what the padding is added to is a measurement and not a constant.
///
/// Pure, for the reason [`menu_hover_rect`] is: a test reads it without a menu on the screen.
pub fn menu_item_height(text_height: i32, dpi: i32) -> i32 {
    text_height + 2 * settings::scaled_tenths(MENU_V_PAD_TENTHS, dpi)
}

/// The line of one rule of FR-91, inside the stripe whose rectangle is `item` — FR-92а, task
/// T-11-22.
///
/// [`MENU_SEP_INSET`] of the mock-up off the left and the right edge, [`MENU_SEP_THICKNESS`]
/// thick, lying on the vertical middle of the stripe — the `($iy + $sepH/2)` of the generator.
/// Pure, like [`menu_hover_rect`], and for the same reason: the geometry of a rule is a table a
/// test reads without a menu on the screen.
pub fn menu_separator_line(item: &RECT, dpi: i32) -> RECT {
    let inset = settings::scaled(MENU_SEP_INSET, dpi);
    let middle = (item.top + item.bottom) / 2;

    RECT {
        left: item.left + inset,
        top: middle,
        right: item.right - inset,
        bottom: middle + MENU_SEP_THICKNESS,
    }
}

/// The DPI the entries of FR-91 are measured at — task T-11-22.
///
/// `WM_MEASUREITEM` carries no device context: an entry is measured before there is a menu
/// window to measure it against. A DC of the screen answers instead, exactly as the text
/// measurement of [`MenuPaint::measure`] uses one, and a refused DC is the 100 % look
/// (NFR-13, the fallback [`crate::settings::dc_dpi`] itself names).
fn measure_dpi() -> i32 {
    // SAFETY: `None` asks for a DC of the screen, which needs no window of ours; a valid
    // handle is released below, on this same thread, as `ReleaseDC` requires.
    let dc = unsafe { GetDC(None) };

    if dc.is_invalid() {
        return settings::SCREEN_DPI;
    }

    let dpi = settings::dc_dpi(dc);

    // SAFETY: the DC came from the `GetDC` above, on this same thread. NFR-13: nothing useful
    // can be done about a refused release, and it is not made fatal — the DC was ours for one
    // reading.
    let _ = unsafe { ReleaseDC(None, dc) };

    dpi
}

/// The square the check mark of FR-93 is drawn in, for an entry whose rectangle is `item` and
/// a check column `column` pixels wide — FR-92а, task T-11-21.
///
/// [`MENU_CHECK_CELL`] mock-up pixels a side, centred on the check column that
/// [`check_column`] measures and on the middle of the entry — which is where the generator
/// puts it (`$by = $iy + $itemH/2`).
///
/// ⚠ The horizontal anchor is the column and **not** the `$mX + 16` of the generator, and this
/// is deliberate: the width of an entry, the left edge of its text and the column itself are
/// all built on `SM_CXMENUCHECK` (task T-11-10, and no part of this task), so a fixed mock-up
/// offset would walk into the text at a display scale where the metric grows and the offset
/// does not. At 96 DPI the two land within three pixels of each other. The **figure** — the
/// three points, the pen and the square around them — is the generator's own.
///
/// Pure, for the reason [`menu_hover_rect`] is.
pub fn menu_check_cell(item: &RECT, column: i32, dpi: i32) -> RECT {
    let side = settings::scaled(MENU_CHECK_CELL, dpi);
    let left = item.left + MENU_H_PAD + (column - side) / 2;
    let top = (item.top + item.bottom) / 2 - side / 2;

    RECT {
        left,
        top,
        right: left + side,
        bottom: top + side,
    }
}

/// The face the entries of FR-91 are measured and drawn in — FR-92а, task T-11-21.
///
/// `lfMenuFont` of `SPI_GETNONCLIENTMETRICS`, which is the face a menu of this system is set
/// in, put through [`crate::settings::antialiased_logfont`]: one field changed — the quality —
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

    Some(settings::antialiased_logfont(metrics.lfMenuFont))
}

/// The `WM_MEASUREITEM` of the menu of FR-91 — the measuring half of task T-11-10.
///
/// The gate first, everything else after: while [`MENU_PAINT`] holds `None` this program
/// has no menu on the screen, the message cannot be the system working on our behalf, and
/// the answer is [`Reaction::Ignored`] with nothing dereferenced (SEC-05). Past the gate,
/// the reads are `CtlType` and `itemData` — the latter only as a number to look up — and
/// the writes are `itemWidth` and `itemHeight`, which are the point of the message.
fn measure_menu_item(lparam: LPARAM) -> Reaction {
    MENU_PAINT.with(|slot| {
        let paint = slot.borrow();

        let Some(paint) = paint.as_ref() else {
            return Reaction::Ignored;
        };

        if lparam.0 == 0 {
            return Reaction::Ignored;
        }

        // SAFETY: the gate is up, so our menu is on the screen and this message is the
        // system asking about one of its entries — `lParam` is then the address of a live
        // `MEASUREITEMSTRUCT` for the length of the call, checked non-null above. The
        // reference does not outlive this closure.
        let structure = unsafe { &mut *(lparam.0 as *mut MEASUREITEMSTRUCT) };

        if structure.CtlType != ODT_MENU {
            return Reaction::Ignored;
        }

        // `itemData` is used as a number and nothing else — SEC-05.
        let Ok(data) = u32::try_from(structure.itemData) else {
            return Reaction::Ignored;
        };

        // The rules of FR-91 carry one reserved number and no label — task T-11-22.
        let (width, height) = if data == MENU_SEPARATOR_DATA {
            paint.measure_rule()
        } else {
            let Some(item) = paint.item(data) else {
                return Reaction::Ignored;
            };

            paint.measure(&item.label)
        };

        structure.itemWidth = width;
        structure.itemHeight = height;

        Reaction::Handled(LRESULT(1))
    })
}

/// The `WM_DRAWITEM` of the menu of FR-91 — the drawing half of task T-11-10.
///
/// The same gate, the same order: [`Reaction::Ignored`] with nothing dereferenced while
/// [`MENU_PAINT`] holds `None` (SEC-05). Past the gate, the reads are `CtlType`, `hDC`,
/// `rcItem`, `itemState` and `itemData` — the latter only as a number to look up.
fn draw_menu_item(lparam: LPARAM) -> Reaction {
    MENU_PAINT.with(|slot| {
        let paint = slot.borrow();

        let Some(paint) = paint.as_ref() else {
            return Reaction::Ignored;
        };

        if lparam.0 == 0 {
            return Reaction::Ignored;
        }

        // SAFETY: the gate is up, so our menu is on the screen and this message is the
        // system asking for one of its entries — `lParam` is then the address of a live
        // `DRAWITEMSTRUCT` for the length of the call, checked non-null above. The
        // reference does not outlive this closure.
        let structure = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };

        if structure.CtlType != ODT_MENU || structure.hDC.is_invalid() {
            return Reaction::Ignored;
        }

        // `itemData` is used as a number and nothing else — SEC-05.
        let Ok(data) = u32::try_from(structure.itemData) else {
            return Reaction::Ignored;
        };

        // The rules of FR-91 carry one reserved number and no label — task T-11-22.
        if data == MENU_SEPARATOR_DATA {
            paint.draw_rule(structure);

            return Reaction::Handled(LRESULT(1));
        }

        let Some(item) = paint.item(data) else {
            return Reaction::Ignored;
        };

        paint.draw(structure, item);

        Reaction::Handled(LRESULT(1))
    })
}

/// Shows the menu of FR-91 at a point on the screen and carries out what was chosen.
///
/// Called with no borrow of the tray held: `TrackPopupMenuEx` runs a modal message loop that
/// dispatches back into our own window procedure.
fn show_menu(x: i32, y: i32) {
    let Some((hwnd, enabled, autostart, theme_setting)) = with_tray(|tray| {
        (
            tray.hwnd,
            tray.enabled(),
            tray.autostart(),
            tray.config().general.theme,
        )
    }) else {
        return;
    };

    let menu = match Menu::build(enabled, autostart) {
        Ok(menu) => menu,
        Err(error) => {
            app::report_non_critical("Menu::build", &error);
            return;
        }
    };

    // FR-92а: the palette of this showing, resolved exactly once — the setting the
    // configuration holds, one reading of the system switch. It lives, with the brushes and
    // the menu face of [`MenuPaint`], until `TrackPopupMenuEx` returns.
    let palette = theme::resolve(theme_setting, theme::system_is_light());

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

    // The gate of SEC-05 goes up here, immediately before `TrackPopupMenuEx`: from the
    // next line until the take() below, `WM_MEASUREITEM` and `WM_DRAWITEM` are answered.
    // A refused `MenuPaint::new` (GDI exhaustion; nothing to report — see its
    // documentation) leaves the gate down and the menu is shown anyway: the rows come up
    // unpainted, but every command of FR-91 still works and `Esc` still dismisses.
    if let Some(paint) = MenuPaint::new(menu.items().to_vec(), palette) {
        // FR-92а, task T-11-22: the ground **between** the entries — the padding the menu
        // keeps around them — is the palette's from here on. The brush is the one
        // [`MenuPaint`] already owns and lives exactly as long as the showing does, which is
        // the whole of what `SetMenuInfo` requires: it is set before `TrackPopupMenuEx` and
        // the brush is freed after it returns, when the value below is taken out and dropped.
        set_menu_background(menu.handle(), paint.window_bg);

        MENU_PAINT.with(|slot| slot.replace(Some(paint)));
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

    // The gate comes down here, immediately after the return: from this line on the two
    // messages are foreign again (SEC-05) — in particular before `dispatch_command` below
    // opens anything modal. Taken out of the `RefCell` before being dropped, like
    // [`detach`] and for the same reason; the drop frees the three brushes and the menu face,
    // whose whole life is the one showing (FR-92а) — the ground brush `SetMenuInfo` was given
    // among them, and the menu it was given to is gone by this line.
    let paint = MENU_PAINT.with(|slot| slot.borrow_mut().take());
    drop(paint);

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

/// Gives the menu the ground of the palette instead of the shell's — FR-92а, task T-11-22.
///
/// `SetMenuInfo` with `MIM_BACKGROUND` is the documented way to say what a menu's own
/// background is painted with, and the background is the part of a menu that is not any of its
/// entries: the padding the menu keeps above the first entry, below the last and along both
/// sides. Until this task that padding was `COLOR_MENU` — light grey on both palettes, which
/// went unnoticed on «Туман» (237,239,242 against the system's 240,240,240) and was plain on
/// «Графит» (32,35,41 against the same 240,240,240).
///
/// ⚠ `brush` must outlive the showing: Windows keeps the handle in the menu and paints with it
/// until the menu goes away. The one brush that lives exactly that long is
/// [`MenuPaint::window_bg`], and this is called with that one and no other — a second brush of
/// the same colour would be a second thing to free at the right moment.
///
/// The frame around the menu and the shape of its corners are **not** this call's to give: they
/// are the non-client area of a window of class `#32768`, which this program does not own. What
/// remains of them after this call is the honest remainder of task T-11-22 and is written down
/// in the report of that task.
///
/// NFR-13: the result is examined. A refusal is not fatal and is not journalled — the menu
/// simply comes up on the system's ground, and by the precedent of task T-11-1 a refused
/// drawing call is a `debug_assert!` rather than an event of SEC-07.
fn set_menu_background(menu: HMENU, brush: HBRUSH) {
    let info = MENUINFO {
        // The structure is versioned by its own size, exactly like `NOTIFYICONDATAW`: a wrong
        // `cbSize` is rejected wholesale rather than misread.
        cbSize: u32::try_from(size_of::<MENUINFO>()).unwrap_or(0),
        fMask: MIM_BACKGROUND,
        hbrBack: brush,
        ..Default::default()
    };

    // SAFETY: `menu` is the live popup this frame owns and `info` is a live local of this
    // frame whose `cbSize` describes it; the call reads it and does not keep the pointer. The
    // brush it does keep is `MenuPaint::window_bg`, which outlives the showing — see above.
    let set = unsafe { SetMenuInfo(menu, &info) };

    debug_assert!(set.is_ok(), "SetMenuInfo refused a live menu and brush");
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

        // FR-92, task T-08-1. Outside any borrow of the tray, like the about box below and for
        // the same reason: the dialog is modal and pumps messages.
        CMD_SETTINGS => open_settings(hwnd),

        // FR-93, task T-08-1.
        CMD_AUTOSTART => toggle_autostart(),

        // Outside any borrow of the tray: the about dialog is modal and pumps messages.
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
// The settings dialog — FR-92, and the autostart of FR-93
// ---------------------------------------------------------------------------------------

/// Opens the settings dialog of FR-92 and applies whatever it produces.
///
/// Called with **no borrow of the tray held**: `DialogBoxParamW` runs a modal message loop that
/// dispatches back into this module, exactly as `TrackPopupMenuEx` does. The configuration the
/// dialog starts from is therefore *copied* out of the tray first, and what comes back arrives
/// through [`apply_settings`].
fn open_settings(hwnd: HWND) {
    let Some(config) = with_tray(|tray| tray.config().clone()) else {
        return;
    };

    // SAFETY: `None` asks for the handle of the file used to create the calling process, which
    // is the running executable — the module `app.rc` was linked into, and therefore the one
    // holding the dialog template. The handle is borrowed and must not be freed; nothing here
    // frees it.
    let module = match unsafe { GetModuleHandleW(PCWSTR::null()) } {
        Ok(module) => module,
        Err(error) => {
            app::report_non_critical("GetModuleHandleW", &error);
            return;
        }
    };

    let mut apply = apply_settings;

    if let Err(error) = settings::show_dialog(hwnd, HINSTANCE(module.0), &config, &mut apply) {
        // NFR-13. The dialog either came up or it did not, and if it did not the user is told
        // by the absence of a window; the reason goes to the journal.
        app::report_non_critical("DialogBoxParamW", &error);
    }
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
fn apply_settings(config: &Config) {
    // FR-93 first: if the registry refuses, the file is not made to claim otherwise.
    if let Err(error) = settings::set_autostart(config.general.autostart) {
        app::report_non_critical("RegSetValueExW", &error);
    }

    with_tray(|tray| tray.replace_config(config.clone()));

    app::publish_configuration(config);
}

/// The check mark of FR-91, made to do what FR-93 says.
///
/// The registry is written **first** and the file only if that succeeded: the check mark
/// describes what the system will do at the next logon, and a file that disagrees with the
/// system would make the tray show a state that is not true.
fn toggle_autostart() {
    let Some(wanted) = with_tray(|tray| !tray.autostart()) else {
        return;
    };

    if let Err(error) = settings::set_autostart(wanted) {
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

/// Shows the "О программе" window: the name, the version, two lines of description and an
/// «ОК» — and nothing else. FR-92а, task T-11-11: the window is the own-drawn dialog
/// `IDD_ABOUT` of [`crate::settings`], shown in the resolved palette, because the
/// `MessageBoxW` this function used to call cannot be repainted by any documented means.
///
/// Called in the same order the box was called: from [`dispatch_command`], reached only
/// through [`handle_ui_message`], with **no borrow of the tray held** — the dialog is modal
/// and pumps messages, exactly like `TrackPopupMenuEx` (the rule of the module
/// documentation). The theme setting is *copied* out of the tray first, in a borrow that
/// ends before the modal call begins — the same shape [`open_settings`] has.
fn show_about(hwnd: HWND) {
    let Some(setting) = with_tray(|tray| tray.config().general.theme) else {
        return;
    };

    // SAFETY: `None` asks for the handle of the file used to create the calling process,
    // which is the running executable — the module `app.rc` was linked into, and therefore
    // the one holding the dialog template. The handle is borrowed and must not be freed;
    // nothing here frees it.
    let module = match unsafe { GetModuleHandleW(PCWSTR::null()) } {
        Ok(module) => module,
        Err(error) => {
            app::report_non_critical("GetModuleHandleW", &error);
            return;
        }
    };

    // The version travels the same road it always did: out of the `VERSIONINFO` resource
    // of the running executable by [`file_version`], never out of a literal.
    if let Err(error) =
        settings::show_about_dialog(hwnd, HINSTANCE(module.0), setting, file_version())
    {
        // NFR-13. The dialog either came up or it did not, and if it did not the user is
        // told by the absence of a window; the reason goes to the journal.
        app::report_non_critical("DialogBoxParamW", &error);
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
