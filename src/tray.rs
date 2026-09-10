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
//! shell's ([`set_menu_background`]), and an entry is the height the mock-up gives it;
//! T-11-22 left one piece of the shell standing and T-12-9 (done) took that — the **frame**
//! of the popup window itself, square and grey `160,160,160` until then, is now the rounded
//! frame of the palette ([`install_menu_frame_hook`]), and the entry under the cursor is
//! written in the same ink as every other, as the mock-up writes it.
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
//! holds `Some` exactly while our own drawing of a menu is active — set immediately before
//! `TrackPopupMenuEx`, cleared immediately after it returns — and outside that window both
//! messages are [`Reaction::Ignored`] like everything else foreign. The `itemData` of every
//! entry is a number and never a pointer — the command number for the five commands of FR-91,
//! the one reserved [`MENU_SEPARATOR_DATA`] for the two rules task T-11-22 also draws — so
//! nothing a forged message carries is dereferenced beyond the identifier check and the drawing
//! rectangle — the SEC-05 wording verbatim.
//!
//! # Two thread-locals about the menu, and why they are two — task T-13-18
//!
//! [`MENU_PAINT`] and [`MENU_ON_SCREEN`] look alike and answer **different questions**. Neither
//! is the other's spare, and removing either brings back a defect the audit of 2026-08-24
//! found:
//!
//! * [`MENU_PAINT`] answers «**is our own drawing active?**» — is there paint state these two
//!   messages may read. It is the gate SEC-05 asks for, and it is `None` in the degraded
//!   showing where [`MenuPaint::new`] was refused: there the menu is up, the rows come up in
//!   the shell's own colours, and the two messages have nothing of ours to answer with.
//! * [`MENU_ON_SCREEN`] answers «**is a menu of ours on the screen?**» — the question
//!   [`show_menu`] has to ask of *itself* before it starts a second showing on top of a live
//!   one. It goes up on the first line of that function and comes down once
//!   `TrackPopupMenuEx` has returned, so it wraps the modal call together with everything the
//!   showing builds for it, and it is up for the ordinary showing and the degraded one alike
//!   — which is exactly why a check of `MENU_PAINT.is_some()` could not have taken its place.
//!
//! The re-entry is not hypothetical: the callback message is postable by any process at the
//! same integrity level (below), and while `TrackPopupMenuEx` pumps its modal loop every such
//! message is dispatched into [`handle_ui_message`]. Without the second flag the nested call
//! replaced [`MENU_PAINT`] — freeing, out from under Windows, the brush `SetMenuInfo` had been
//! given for the menu still on the screen — and then took the slot away, leaving the outer
//! showing with the SEC-05 gate down. With it, a second `WM_APP_TRAY` finds nothing: it does
//! not reach the tray, the shell, GDI or the menu of the live showing.
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

use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUNDSMALL, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreateSolidBrush, DT_NOCLIP, DT_SINGLELINE, DT_VCENTER, DeleteObject,
    DrawTextW, FillRect, GetDC, GetTextExtentPoint32W, HBRUSH, HDC, HFONT, HGDIOBJ, LOGFONTW,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_SELECTED, ODT_MENU};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_LARGE_ICON, NIIF_USER, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NIN_BALLOONUSERCLICK, NIN_SELECT, NOTIFYICON_VERSION_4,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CWPSTRUCT, CallNextHookEx, CreatePopupMenu, DestroyIcon, DestroyMenu,
    GetClassNameW, GetSystemMetrics, HHOOK, HICON, HMENU, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW,
    MENUINFO, MF_CHECKED, MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_OWNERDRAW, MF_SEPARATOR,
    MF_UNCHECKED, MIM_BACKGROUND, NONCLIENTMETRICSW, PostMessageW, RT_VERSION,
    RegisterWindowMessageW, SM_CXICON, SM_CXMENUCHECK, SM_CXSMICON, SM_CYICON, SM_CYMENU,
    SM_CYSMICON, SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetForegroundWindow,
    SetMenuInfo, SetTimer, SetWindowsHookExW, SystemParametersInfoW, TPM_LAYOUTRTL, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, UnhookWindowsHookEx, WH_CALLWNDPROC, WM_APP,
    WM_CONTEXTMENU, WM_DRAWITEM, WM_ENDSESSION, WM_LBUTTONDBLCLK, WM_MEASUREITEM, WM_NCCREATE,
    WM_NULL, WM_QUERYENDSESSION, WM_SETTINGCHANGE, WM_THEMECHANGED, WM_TIMER, WM_USER,
};
use windows::core::{Error as WinError, HRESULT, PCWSTR, Result as WinResult, w};

use crate::settings::{self, Config, Quarantined, SavePolicy};
use crate::{APP_NAME, app, diag, theme};

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

/// Resource identifier of the "active" icon in `app.rc`.
const IDI_APP_ACTIVE: u16 = 101;

/// Resource identifier of the "suspended" icon in `app.rc`.
const IDI_APP_PAUSED: u16 = 102;

/// The "active, and a letter is unread" icon of FR-90's third state — task Т-32-4.
const IDI_APP_ACTIVE_UNREAD: u16 = 103;

/// The "suspended, and a letter is unread" icon.
const IDI_APP_PAUSED_UNREAD: u16 = 104;

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
/// measures as two — the reasoning [`crate::theme::BORDER_THICKNESS`] carries for the frames
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
/// exactly what [`crate::theme::scaled_tenths_offset`] divides by, and the reason these
/// literals are the generator's own numbers rather than numbers somebody has already divided.
///
/// The offsets are measured from the corner of [`menu_check_cell`] and not from `($bx, $by)`:
/// [`crate::theme::draw_check_mark`] clamps the tile it smooths in to the square it is
/// given, so the square has to hold the pen and the fading edge as well as the path. That is
/// the whole of [`MENU_CHECK_AIR`] — the three points below are the generator's, moved by it.
pub const MENU_CHECK_MARK: theme::CheckMark = theme::CheckMark {
    points_tenths: [(30, 100), (70, 140), (140, 30)],
    pen_tenths: 21,
};

/// Air around the path of [`MENU_CHECK_MARK`] inside the square it is drawn in, in the pixels
/// of the mock-up.
///
/// Not a number of the mock-up and not meant to be one: it is what keeps the tile of
/// [`crate::theme::draw_check_mark`] — which is clamped to this square — from cutting the
/// ends of the strokes off. That tile is `theme::stroke_bounds`, which reaches
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

/// Whether the open settings dialog takes `command` away from the menu — task T-13-14.
///
/// # The finding
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
/// # Which two, and why not the other three
///
/// [`CMD_TOGGLE`] and [`CMD_AUTOSTART`] and no others, because those are exactly the two menu
/// commands that write into the configuration:
///
/// * [`CMD_SETTINGS`] leads into the dialog that is already open, and [`settings::show_dialog`]
///   answers `Ok(())` to it without opening anything — behaviour that already exists and is
///   deliberately left alone;
/// * [`CMD_ABOUT`] shows a window and changes nothing;
/// * [`CMD_EXIT`] must stay alive under every circumstance — a program that cannot be closed
///   because a window is open is worse than the finding.
///
/// # Two halves of one rule, and this is the rule
///
/// [`Menu::build`] appends the two entries `MF_GRAYED | MF_DISABLED` — that is the half the
/// user sees and the half that keeps `TrackPopupMenuEx` from returning either command at all.
/// [`dispatch_command`] asks the same question again before it does anything — that is the
/// half that decides. The greying is the *view* of this rule; the gate in the command handler
/// is the rule. Both read this one function, so there is nothing for the two to disagree
/// about.
///
/// # Why an argument rather than a read inside
///
/// The same shape and the same reason as [`resume_is_refused`] one function up: the decision
/// belongs to one place, the reading of the flag to the caller, and a rule that reached for a
/// thread-local of its own would be a rule the menu tests could not drive — they build menus
/// on a test thread that has no dialog and never will. [`show_menu`] and [`dispatch_command`]
/// read [`settings::dialog_is_open`] and hand the answer in.
///
/// **SEC-05.** The flag is a thread-local raised only by the guard of the open dialog; it is
/// not derived from any message, and no `wParam`, `lParam` or pointer takes part in this
/// decision (R-20). See [`settings::dialog_is_open`].
pub fn dialog_locks_command(command: u32, dialog_open: bool) -> bool {
    dialog_open && matches!(command, CMD_TOGGLE | CMD_AUTOSTART)
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
        // FR-90's third state, task Т-32-4 — loaded here with the other two so that the dot
        // appears the moment a letter arrives rather than after a load that could fail then.
        let active_unread = Icon::load(instance, IDI_APP_ACTIVE_UNREAD, icon_size)?;
        let paused_unread = Icon::load(instance, IDI_APP_PAUSED_UNREAD, icon_size)?;

        // Задача Т-33а-1: те же четыре значка ещё раз, крупным кадром — для шара уведомления.
        // ⚠ Без `?`: значок уведомления — украшение письма, а письмо — нет. Отказ загрузки
        // стоит одной записи в журнале и возвращает шару прежний вид (NFR-13).
        let balloon = BalloonIcons::load(instance);

        let (config, save_policy, on_disk) = match config_path.as_deref() {
            Some(path) => {
                // Task T-55-5, finding Т8: the temporaries an interrupted write left beside the
                // file go first — before this process has written anything, so none is its own.
                if settings::remove_abandoned_temporaries(path).refused > 0 {
                    note_configuration(CONFIG_TEMPORARY_NOT_REMOVED);
                }

                let (config, outcome) = settings::read_or_default(path);

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
            // `None` — so the policy is the one that changes nothing.
            None => (Config::default(), SavePolicy::Allowed, None),
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
            config,
            config_path,
            save_policy,
            on_disk,
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
            // careful — it happens only when there is somebody to hand the answer to
            // (task T-13-20, [`on_setting_change`]), a null pointer is not ours, at most
            // [`SETTING_NAME_CAP`] UTF-16 units are ever looked at, the comparison is
            // exact — and nothing of the message is kept: the copy lives on that frame
            // and dies with it.
            //
            // The setting is read out of `self` **here**, on the way in, because that is
            // the one cheap local fact this arm has and [`on_setting_change`] must have
            // before it looks at anything of the message.
            WM_SETTINGCHANGE => {
                on_setting_change(lparam, self.config().general.theme);

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
        // FR-90's third state, task Т-32-4: the dot of FR-101 rides on **both** of the two
        // states this program has always had, because they are independent — a person can pause
        // the program with a letter unread. Four icons, two questions, and neither of them
        // knows about the other.
        match (self.enabled(), self.has_unread()) {
            (true, false) => self.active.handle,
            (false, false) => self.paused.handle,
            (true, true) => self.active_unread.handle,
            (false, true) => self.paused_unread.handle,
        }
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
            hIcon: self.state_icon(),
            ..Default::default()
        };

        write_tip(&mut data.szTip, &icon_tip(self.enabled()));

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
        if !self.icon_present {
            return;
        }

        let mut data = self.notify_data();

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
/// number the operating system gave and nothing else: [`os_code_of`] takes it from
/// [`io::Error::raw_os_error`] and drops the error's text — which is the same line
/// `diag::dump_on_shutdown` draws for the `io::Error` of a refused dump, and the same one
/// [`crate::app::report_non_critical`] draws for a `windows::core::Error`.
///
/// A path never reaches here, in any form. Neither does the line and column of a parser, which is
/// the one thing a [`settings::ConfigError`] carries that was derived from the file's contents —
/// see [`os_code_of`], where that variant is answered with no number at all.
fn note_configuration_failure(operation: &'static str, error: &settings::ConfigError) {
    diag::record(diag::Operation::from_name(operation), os_code_of(error));
}

/// The system's own code for a failed configuration write — task **Т-22-9**.
///
/// [`diag::OsCode`] has no constructor that takes an integer by design (SEC-07), so the number
/// travels the one road there is: an `io::Error` that came from a system call carries the Win32
/// code, `HRESULT::from_win32` turns it into the same `HRESULT` every other failure in this
/// program is journalled under, and [`diag::OsCode::of`] narrows that.
///
/// The two variants that are not system failures answer [`diag::OsCode::NONE`]:
///
/// * [`settings::ConfigError::Serialize`] — this program's own value refusing to become TOML.
///   No call failed, and a number invented for it would be a claim about one that did;
/// * [`settings::ConfigError::Malformed`] — cannot arrive from a write at all, and it is the one
///   variant carrying something derived from the file's contents (the line and column a parser
///   stopped at). It is answered with no number for both reasons, and the second is the one that
///   would matter if the first ever stopped being true.
///
/// An `io::Error` with no `raw_os_error` — one this program's own code built — answers `NONE` for
/// the same reason: there is no system number to report.
fn os_code_of(error: &settings::ConfigError) -> diag::OsCode {
    match error {
        settings::ConfigError::Io(io) => io
            .raw_os_error()
            .and_then(|code| u32::try_from(code).ok())
            .map_or(diag::OsCode::NONE, |code| {
                diag::OsCode::of(&WinError::from_hresult(HRESULT::from_win32(code)))
            }),
        settings::ConfigError::Malformed { .. } | settings::ConfigError::Serialize => {
            diag::OsCode::NONE
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
    attach_at(hwnd, instance, settings::default_config_path())
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

/// The ceiling on how many UTF-16 units of a `WM_SETTINGCHANGE` string are ever looked at —
/// FR-92а, task T-11-9, the reading bounds of SEC-05.
///
/// `"ImmersiveColorSet"` is 17 units; 64 is that with room to spare, and a string still
/// running at 64 is one this program was never going to match.
const SETTING_NAME_CAP: usize = 64;

/// Whether the string a `WM_SETTINGCHANGE` names is worth reading at all — FR-92а, SEC-05,
/// task T-13-20.
///
/// Two facts line up, and both of them are this program's **own**: there is somebody to hand
/// the string to — one of the two windows of FR-92а is up — and the setting in force is
/// `system`, under which alone the system's switch has any say (FR-92а fixes the palette
/// outright under `light` and `dark`, and a broadcast then changes nothing whatever).
///
/// «Кому отдать» is *two* windows since task T-13-17, not one: the settings dialog of FR-92
/// ([`settings::dialog_is_open`]) **or** the «О программе» window ([`settings::about_is_open`]).
/// Asking only the first would leave the about window on yesterday's palette and undo that
/// task — the very repair the second name exists for.
///
/// The shape is [`resume_is_refused`]'s and [`dialog_locks_command`]'s, for the same reason:
/// the facts are read once by the caller and handed in, so the rule is a function of its
/// arguments and a table can close it. **SEC-05, R-20:** all three arguments are published
/// values of this process — two thread-locals of the UI thread and one field of the
/// configuration the tray holds. No `wParam`, no `lParam` and no pointer reaches this
/// decision, which is what makes it safe to take *before* the message is looked at.
pub fn setting_name_is_wanted(
    dialog_open: bool,
    about_open: bool,
    setting: theme::ThemeSetting,
) -> bool {
    (dialog_open || about_open) && setting == theme::ThemeSetting::System
}

/// The whole of the `WM_SETTINGCHANGE` arm of [`Tray::handle_message`] — FR-92а, SEC-05,
/// task T-13-20.
///
/// # The order of the checks *is* the repair
///
/// The audit of 2026-08-24 found this arm calling [`setting_change_name`] on **every**
/// `WM_SETTINGCHANGE` that reached the hidden window — a dereference of a foreign pointer
/// after one null check — while the only consumer of the answer, at a closed dialog, left on
/// its first line. `WM_SETTINGCHANGE` is *posted*, and a posted message is not marshalled:
/// the number in `lParam` is whatever the sender put there, so a forged one, or the merely
/// wrong one a third-party program is known to broadcast, was a read of an arbitrary address
/// and an access violation.
///
/// So the two cheap local facts are established **first**, on the lines below, and `lparam`
/// is not touched at all unless they both hold. Written the other way round — the string read
/// and the gate applied to the result — this function would be exactly the finding again,
/// with a comment on top.
///
/// `setting` is `[general].theme` as the tray holds it, copied out by the caller before this
/// call: the same value the settings dialog was opened with and the same one
/// [`show_about_dialog`](settings::show_about_dialog) was handed, since «Применить» is the
/// one road that moves it and it moves both at once.
///
/// # SEC-05
///
/// Nothing here believes the message. A string that gets read is compared against
/// [`settings::IMMERSIVE_COLOR_SET`] and used for nothing else; the reaction on the far side
/// of that comparison is [`settings::on_system_theme_message`], which asks *the system* what
/// the theme now is and repaints this program's own window. A gate that refuses costs two
/// thread-local reads and one comparison of an enum, and the message goes on to
/// `DefWindowProcW` as it always did.
fn on_setting_change(lparam: LPARAM, setting: theme::ThemeSetting) {
    // Task T-13-20. **Before** `lparam` is looked at, and that is the whole of the repair —
    // see above. `dialog_is_open` and `about_is_open` are thread-locals of this, the UI
    // thread; `setting` is already on this frame.
    if !setting_name_is_wanted(
        settings::dialog_is_open(),
        settings::about_is_open(),
        setting,
    ) {
        return;
    }

    // SAFETY: this is the window procedure of this program's own hidden window, inside the
    // delivery of the `WM_SETTINGCHANGE` whose `lParam` this is — the two preconditions the
    // reader states — and the gate above has just established that there is a window to hand
    // the answer to, which is the third. The reader checks for null and never looks past the
    // terminator or the cap.
    let name = unsafe { setting_change_name(lparam) };

    settings::on_system_theme_message(name.as_deref());
}

#[cfg(test)]
thread_local! {
    /// How many times [`setting_change_name`] has been **entered** on this thread — task
    /// T-13-20.
    ///
    /// Test-only, and counted on the reader's very first line — before its null check and
    /// long before any dereference — on purpose. What criterion 5 has to establish is
    /// «читателя не звали», and a counter placed after the null check would answer
    /// «читатель не разыменовывал», which is a different and weaker sentence: it is
    /// satisfied by a null `lParam` the gate had nothing to do with.
    static SETTING_NAME_READS: Cell<u32> = const { Cell::new(0) };
}

/// The string a `WM_SETTINGCHANGE` names, read within the bounds of SEC-05 — task T-11-9,
/// narrowed by task T-13-20.
///
/// ⚠ **This function dereferences a pointer that arrived on a window message, and it has no
/// way of checking that the pointer is real.** `WM_SETTINGCHANGE` is posted and is therefore
/// not marshalled: the number in `lParam` is whatever the sender wrote there, and a non-null
/// number naming nothing readable is an access violation and the end of the process. The null
/// check below is **necessary and not sufficient** — the finding behind task T-13-20 is
/// precisely that a non-null pointer can be a foreign one. There is one caller,
/// [`on_setting_change`], it establishes the preconditions below before it calls, and nothing
/// else in this program may call this at all.
///
/// `None` for a null `lParam` (then it is not ours), for a string that shows no terminator
/// within [`SETTING_NAME_CAP`] units, and for units that are not UTF-16 — whatever either of
/// those is, it is not the one word this program compares against. The value lives on the
/// caller's frame and dies with the comparison it was read for: nothing of the message is
/// kept.
///
/// # Safety
///
/// The pointer in `lparam` counts as fit to read only while **all** of the following hold,
/// and not one of them is checkable here:
///
/// 1. `lparam` is the `lParam` of a `WM_SETTINGCHANGE` that Windows **delivered to this
///    thread**, and the read happens *inside* that delivery — the call is on the stack of the
///    window procedure that received it. For exactly that long the system keeps the string
///    the broadcast names readable in this process; a number saved and read afterwards, or
///    read on another thread, has nothing keeping it alive.
/// 2. The message reached the window procedure of this program's own hidden window — the one
///    window of this process that reads this message (§6.2) — rather than being handed in by
///    a caller that made the number up.
/// 3. The gate of [`setting_name_is_wanted`] has just answered `true`. This one is not a
///    memory precondition and is a precondition all the same: the read is worth its risk only
///    when there is a window waiting for the answer, and calling without it is the finding of
///    the audit of 2026-08-24 restored.
///
/// Given those, the loop below reads one unit at a time, never past the terminator and never
/// past the cap.
unsafe fn setting_change_name(lparam: LPARAM) -> Option<String> {
    // Task T-13-20, the number criterion 5 reads: entries into the reader, counted before
    // anything else happens in it — see [`SETTING_NAME_READS`]. Compiled only into the
    // unit-test build; the product has no counter.
    #[cfg(test)]
    SETTING_NAME_READS.with(|count| count.set(count.get().saturating_add(1)));

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
    // FR-101, task Т-32-4: the tick of the letters, answered **before** the tray is borrowed
    // and outside it — the schedule reads the configuration through `with_tray` of its own.
    if message == WM_TIMER && wparam.0 == LETTERS_TIMER {
        on_letters_tick();
        return Some(LRESULT(0));
    }

    // FR-102, task Т-32-6: the feed thread has left something in its box. Outside every borrow
    // of the tray — taking it writes the configuration through `with_tray` of its own.
    //
    // SEC-05: the message carries nothing; a forged one finds an empty box.
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
}

impl Pending {
    /// Whether either entry is due — what decides the rule under them.
    pub fn any(&self) -> bool {
        self.unread || self.update.is_some()
    }
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
    /// same reason the third one is. It greys **two** entries and only while the modal dialog
    /// of FR-92 is on the screen: «Приостановить/Возобновить» and «Запускать при входе в
    /// систему», the two commands of FR-91 that edit the configuration the dialog is editing.
    /// The rule is [`dialog_locks_command`], where the finding and the reasoning live; this is
    /// its visible half, and [`dispatch_command`] is the half that decides.
    ///
    /// The **composition** of FR-91 does not move here either: seven entries,
    /// [`MENU_ENTRY_COUNT`], the same five commands, the same two rules, the same labels.
    /// «Настройки…» stays available and still leads into the window that is already up,
    /// «О программе» and «Выход» stay available because they change no configuration.
    ///
    /// The two greyings compose rather than argue: the first entry is available only when
    /// neither FR-99 nor the open dialog takes it away, which is what the `&&` below says.
    pub fn build(
        enabled: bool,
        autostart: bool,
        fail_safe: bool,
        dialog_open: bool,
        pending: &Pending,
    ) -> WinResult<Self> {
        // SAFETY: takes no arguments and touches no memory of ours. The handle it returns is
        // owned by this value from here on and is destroyed exactly once, in `Drop`. The
        // crate turns a null handle into an error, so NFR-13 is satisfied by the `?`.
        let handle = unsafe { CreatePopupMenu()? };

        let mut menu = Self {
            handle,
            items: Vec::with_capacity(8),
        };

        // FR-91, task Т-32-4 — the two temporary entries of FR-101, in the top group and above
        // the first rule. They are here **only while there is a reason for them**: an unread
        // news item and a version newer than this one. Nothing else in this menu appears and
        // disappears, and that is deliberate — a person who has read the letter should not go
        // on being told there is one.
        if pending.unread {
            menu.append_command(
                CMD_UNREAD,
                &settings::text(settings::IDS_MENU_UNREAD),
                false,
                true,
            )?;
        }

        if let Some(version) = pending.update.as_deref() {
            menu.append_command(
                CMD_UPDATE,
                &settings::format_text(settings::IDS_MENU_UPDATE, &[version]),
                false,
                true,
            )?;
        }

        if pending.any() {
            menu.append_separator()?;
        }

        // FR-94: every label out of the string table of the locale in force. The menu is built
        // afresh on every click, so it carries the locale published at start-up without any
        // state of its own.
        let first = settings::text(if enabled {
            settings::IDS_MENU_SUSPEND
        } else if resume_is_refused(enabled, fail_safe) {
            // Task T-34-5, finding Н84: the entry is grey, and the label is the one carrier of
            // the reason — this menu has no tooltips (measured for the stage: not one `TTM_` in
            // this file), so «Возобновить» alone would be a grey word with no explanation.
            settings::IDS_MENU_RESUME_RESTART
        } else {
            settings::IDS_MENU_RESUME
        });

        // Tasks T-13-9 and T-13-14: the two entries of FR-91 whose availability is not
        // constant. The first is taken away by either of two rules and needs both to be
        // silent; the second by one.
        let toggle_available = !resume_is_refused(enabled, fail_safe)
            && !dialog_locks_command(CMD_TOGGLE, dialog_open);
        let autostart_available = !dialog_locks_command(CMD_AUTOSTART, dialog_open);

        menu.append_command(CMD_TOGGLE, &first, false, toggle_available)?;
        menu.append_separator()?;
        menu.append_command(
            CMD_SETTINGS,
            &settings::text(settings::IDS_MENU_SETTINGS),
            false,
            true,
        )?;
        menu.append_command(
            CMD_AUTOSTART,
            &settings::text(settings::IDS_MENU_AUTOSTART),
            autostart,
            autostart_available,
        )?;
        menu.append_separator()?;
        // FR-91's addition, task Т-32-4, решение 101 п. 9: the permanent entry stands **above**
        // «О программе», which is where the accepted mock-up puts it.
        menu.append_command(
            CMD_WRITE,
            &settings::text(settings::IDS_WRITE_TO_AUTHOR),
            false,
            true,
        )?;
        menu.append_command(
            CMD_ABOUT,
            &settings::text(settings::IDS_MENU_ABOUT),
            false,
            true,
        )?;
        menu.append_command(
            CMD_EXIT,
            &settings::text(settings::IDS_MENU_EXIT),
            false,
            true,
        )?;

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
    ///
    /// `enabled` is declared the same way and for the same reason (task T-13-9): `MF_GRAYED`
    /// together with `MF_DISABLED` is what keeps `TrackPopupMenuEx` from ever returning the
    /// command and what `GetMenuState` and accessibility read, and the record in the entry is
    /// what the drawing chooses its ink by.
    fn append_command(
        &mut self,
        command: u32,
        label: &str,
        checked: bool,
        enabled: bool,
    ) -> WinResult<()> {
        let mark = if checked { MF_CHECKED } else { MF_UNCHECKED };

        // `MF_ENABLED` is zero, so the ordinary entry is unchanged bit for bit by this task.
        // Both flags of the other case matter: `MF_GRAYED` is what draws it as unavailable to
        // everybody outside this program, and `MF_DISABLED` is what makes the choice
        // impossible — `TrackPopupMenuEx` returns zero instead of the command.
        let availability = if enabled {
            MF_ENABLED
        } else {
            MF_GRAYED | MF_DISABLED
        };

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
                MF_OWNERDRAW | mark | availability,
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
            enabled,
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
/// ⚠ That «exactly that long» is kept true by [`MenuOnScreen`] since task T-13-18 — before it,
/// a second entry into [`show_menu`] freed this value's brushes in the middle of the showing
/// they were painting.
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
    /// Brush of [`theme::Palette::menu_hover_bg`] — the fill of the rounded stripe under the
    /// entry the cursor is on (`ODS_SELECTED`).
    ///
    /// ⚠ **Не [`theme::Palette::hover_bg`], которым светится кнопка.** У меню своё поле
    /// потому, что земля под ним другая: кнопка стоит на `button_bg`, запись меню — на
    /// `window_bg`, и в «Тумане» один и тот же цвет отстоял там на 21/17/13, а здесь
    /// на 3/1/0 — полосы попросту не было видно. Разные числа взяты, чтобы вид совпал;
    /// разбор — у самого поля в [`theme`].
    hover_bg: HBRUSH,
    /// Brush of [`theme::Palette::panel_border`] — the line of a rule, task T-11-22.
    panel_border: HBRUSH,
}

thread_local! {
    /// The paint state of the showing now on the screen — `Some` exactly from just before
    /// `TrackPopupMenuEx` until just after it returns. This *is* the gate SEC-05 asks
    /// for: [`measure_menu_item`] and [`draw_menu_item`] answer [`Reaction::Ignored`]
    /// while it holds `None`, before anything of the message is dereferenced.
    ///
    /// ⚠ **«Our drawing is active», not «a menu is on the screen».** The two part company on
    /// the degraded path, where [`MenuPaint::new`] was refused and the menu is shown anyway:
    /// this slot is then `None` for the whole of a showing that is very much on the screen.
    /// The other question is [`MENU_ON_SCREEN`]'s, and the module header sets the two out
    /// side by side.
    static MENU_PAINT: RefCell<Option<MenuPaint>> = const { RefCell::new(None) };

    /// Whether a menu of this program is on the screen right now — task T-13-18.
    ///
    /// The second line of defence, and the answer to a question [`MENU_PAINT`] cannot be
    /// asked. It wraps `TrackPopupMenuEx`: up from the first line of [`show_menu`], down
    /// once that call has returned — so it is up for the ordinary showing and for the
    /// degraded one where [`MenuPaint::new`] was refused alike. Read and written through
    /// [`MenuOnScreen`] and through nothing else, so that no path out of a showing —
    /// including an unwind — can leave it up.
    static MENU_ON_SCREEN: Cell<bool> = const { Cell::new(false) };
}

/// The right to show one menu, held for exactly as long as that menu is up — task T-13-18.
///
/// [`MenuOnScreen::raise`] answers `None` when a showing is already live on this thread, and
/// that `None` is the early exit of [`show_menu`]: the finding of the audit of 2026-08-24 is
/// that a second `WM_APP_TRAY`, dispatched to us by the modal loop of `TrackPopupMenuEx`
/// itself, used to walk straight into the middle of the live showing and drop its
/// [`MenuPaint`] — brushes, menu face and the ground brush Windows had been handed by
/// `SetMenuInfo`, all freed while the menu they painted was still on the screen.
///
/// ⚠ **The lowering is a `Drop` and not a call**, for the reason [`MenuFrameHook`] gives next
/// door: `TrackPopupMenuEx` runs a message loop of its own, and a flag left up by a path
/// nobody thought of — the early returns, a refused `Menu::build`, an unwind out of anything
/// the modal loop dispatched — would wedge the menu shut for the rest of the run. A bare
/// `set(true) … set(false)` pair around the call would survive neither.
///
/// Neither `Send` nor `Sync`: the flag belongs to the thread that raised it, exactly like
/// [`Attachment`] and for the same reason.
struct MenuOnScreen {
    _not_send: PhantomData<*const ()>,
}

impl MenuOnScreen {
    /// Claims the screen for one showing, or answers `None` because it is already claimed.
    fn raise() -> Option<Self> {
        if MENU_ON_SCREEN.with(Cell::get) {
            return None;
        }

        MENU_ON_SCREEN.with(|flag| flag.set(true));

        Some(Self {
            _not_send: PhantomData,
        })
    }
}

impl Drop for MenuOnScreen {
    fn drop(&mut self) {
        MENU_ON_SCREEN.with(|flag| flag.set(false));
    }
}

#[cfg(test)]
thread_local! {
    /// How many entries into [`show_menu`] have got **past** its first gate on this thread —
    /// task T-13-18.
    ///
    /// Test-only, and counted in [`show_menu`] on the far side of the gate rather than inside
    /// [`MenuOnScreen::raise`] on purpose. What the tests have to tell apart is «turned round
    /// at the gate» from «returned a line or two later for a reason of its own», and a counter
    /// living inside the gate would measure the mechanism instead of the line: a gate written
    /// the other way — a reading of [`MENU_PAINT`], which the degraded showing defeats — has
    /// to move this number where the test of that showing says it must not.
    static MENU_SHOWINGS: Cell<u32> = const { Cell::new(0) };

    /// Makes [`MenuPaint::new`] answer `None` — the seam of the degraded showing, task
    /// T-13-18. Test-only; see the branch it feeds.
    static REFUSE_MENU_PAINT: Cell<bool> = const { Cell::new(false) };
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
        // Task T-13-18, the seam of the degraded showing. GDI exhaustion is not something a
        // test can ask the system for, and the half of this task that matters most — a menu
        // on the screen with **no** paint state behind it — begins exactly here. Compiled
        // only into the unit-test build; the product has no branch to reach it.
        #[cfg(test)]
        if REFUSE_MENU_PAINT.with(Cell::get) {
            return None;
        }

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
        // [`crate::theme::draw_check_mark`], which owns a pen for exactly the one call —
        // a smoothed stroke is drawn twice, once enlarged and once not, with two different
        // thicknesses, so a pen made once for the showing could not have served it.
        let (window_bg, hover_bg, panel_border) = unsafe {
            (
                CreateSolidBrush(palette.window_bg),
                CreateSolidBrush(palette.menu_hover_bg),
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
        let mut dpi = theme::SCREEN_DPI;

        // SAFETY: `None` asks for a DC of the screen, which needs no window of ours; a
        // valid handle is released below, on this same thread, as `ReleaseDC` requires.
        let dc = unsafe { GetDC(None) };

        if !dc.is_invalid() {
            // The message carries no device context — an entry is measured before there is
            // a menu window to measure it on — so the DPI comes off the same screen DC the
            // text is measured with, which is the one [`measure_dpi`] would open anyway.
            dpi = theme::dc_dpi(dc);

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
        let inset = theme::scaled(MENU_SEP_INSET, dpi);
        let height = theme::scaled(MENU_SEP_H, dpi);

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
        let dpi = theme::dc_dpi(dc);

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
    /// `window_bg`, and the entry under the cursor carries the rounded `menu_hover_bg` stripe of
    /// [`menu_hover_rect`] over it; the text is `text` **whether the cursor is on the entry
    /// or not** (task T-12-9 — the stripe is the whole of what marks a hot row, as it is in
    /// the mock-up); the check mark is [`MenuPaint::draw_check_mark`]. There are no disabled
    /// entries in the menu of FR-91, so no third state is painted.
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
        // 100 % look, which `theme::dc_dpi` decides).
        let dpi = theme::dc_dpi(dc);

        // ⚠ **Один кадр вместо череды — task T-15-1б.** The surface the steps below are drawn
        // onto, and the same cure task T-15-1 gave the push buttons of the settings dialog. A
        // string of separate GDI calls straight into the DC of the message is not atomic, and DWM
        // samples the surface of a window sixty times a second. Measured on the стенд, thirty
        // hover transitions between two entries in each palette: an intermediate state reached the
        // screen in **30 of 30**, its bounding box inside this entry's own rectangle and holding
        // the highlight stripe **already drawn with the label not yet on it** — a blank
        // highlighted row for one frame, on the surface this program opens oftenest.
        //
        // `None` — GDI refused the surface (NFR-13). `target` is then the DC of the message itself
        // and every step below paints where it painted before this task, flicker and all.
        //
        // ⚠ `dpi` is taken one line above off the DC of the **message** and never off the buffer:
        // `GetDeviceCaps` of a memory surface answers for the memory and not for the window, and
        // every mock-up length below goes through it.
        //
        // ⚠ `rcItem` of a menu entry is **offset** inside the menu window — it is not a client
        // rectangle starting at the origin — and the buffer is built for exactly that: it moves
        // its own logical origin onto the corner of `rect`, so every step below names the same
        // rectangles and the same points it named before.
        //
        // SAFETY: `dc` came with the message and is painted into for the length of the send; the
        // buffer reads it for its colour depth and its face, writes to it only in the blit at the
        // foot of this body, and frees its own DC and bitmap when this frame ends.
        let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
        let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

        // ⚠ **Since task T-15-1б this erase carries a second duty: it is what grounds the buffer.**
        // The corner smoothing of the highlight below reads the ground of each corner back **out
        // of** the DC it draws into, and the pixels of a fresh `CreateCompatibleBitmap` are
        // undefined — so this fill, standing exactly where it stood, is what makes the buffer hold
        // under those corners the same colour the screen held there.
        //
        // SAFETY: `rect` came with the message; while the gate is up they describe an entry of our
        // menu being painted, and every call below only writes pixels into `target` — the most a
        // forged message can buy is a drawing on its own DC. `self.window_bg` is a live brush this
        // value owns.
        let filled = unsafe { FillRect(target, &rect, self.window_bg) };

        // NFR-13: examined in the only way available — a refused fill leaves the row
        // unpainted for one frame and nothing here can repair it.
        debug_assert!(filled != 0, "FillRect refused a live brush");

        // Task T-11-21: the highlight is a **figure on top of the ground** and no longer
        // the ground itself, because the mock-up rounds it and holds it off the edges —
        // `FillR $g ($mX+5) $iy ($mW-10) $itemH $T.Hover 6`. `paint_rounded` smooths the
        // four corners the same way every rounded figure of the dialog is smoothed; the
        // outline is handed the fill's own colour, so the figure has a fill and no frame.
        if selected {
            theme::paint_rounded(
                target,
                &menu_hover_rect(&rect, dpi),
                theme::scaled(theme::CORNER_RADIUS, dpi),
                self.palette.menu_hover_bg,
                self.hover_bg,
                dpi,
            );
        }

        // SAFETY: `self.font` is live for the whole showing; the previous selection is
        // restored at the end of this function, while `target` is still alive.
        let previous_font = unsafe { SelectObject(target, self.font.into()) };

        // SAFETY: a state call on the DC being painted — a mode by value, no memory of ours.
        let _ = unsafe { SetBkMode(target, TRANSPARENT) };

        // FR-92а, task T-12-9: **one ink for every entry, hot or not.** The mock-up hands
        // `$brFg` to every row it draws — `chrome.ps1:167` — and does not brighten the row
        // under the cursor; the highlight is the whole of what marks it. Until this task the
        // hot row was written in `sel_fg` instead, which on «Графите» is 240,242,244 against
        // the 228,231,234 of `text` — twelve levels of a difference the mock-up does not
        // have. («Туман» hid it: there `sel_fg` *is* `text`.) The ink of an entry is
        // therefore no longer a function of `selected` at all.
        //
        // **Task T-13-9 adds the one exception, and it is not the cursor**: an entry that
        // cannot be chosen is written in [`theme::Palette::text_muted`], the palette's own
        // quieter ink, so that the greyed «Возобновить» of FR-99 reads as greyed on both
        // palettes instead of merely refusing to respond. The value comes from **our own**
        // record of the entry and never from the `ODS_GRAYED` of the message: a forged
        // `WM_DRAWITEM` decides nothing about how this menu looks (SEC-05). No colour is
        // invented here — `text_muted` is a role `theme` already defines and this file already
        // has no say in.
        let ink = if item.enabled {
            self.palette.text
        } else {
            self.palette.text_muted
        };

        // SAFETY: as above — a colour by value.
        let _ = unsafe { SetTextColor(target, ink) };

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
                target,
                &mut units,
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_NOCLIP,
            )
        };

        if item.checked {
            self.draw_check_mark(target, &rect, dpi);
        }

        // SAFETY: restores the font that was selected into `target` above.
        let _ = unsafe { SelectObject(target, previous_font) };

        // ⚠ **The one moment any of the above becomes visible — task T-15-1б.** Nothing since the
        // erase has touched the window, so what DWM can sample is the entry as it was or the entry
        // as it now is, and there is no third state for the eye to catch.
        //
        // NFR-13: the answer is examined in words and dropped. A refused blit leaves the entry
        // showing the picture the window already had there — its previous state and never a hole —
        // and painting the body a second time into the DC of the message is the very flicker this
        // task removes.
        //
        // SAFETY: `dc` came with the message and is painted into for the length of the send;
        // `buffer` is this frame's own, and its DC and bitmap are freed as it goes out of scope on
        // this line.
        if let Some(buffer) = buffer {
            let _ = unsafe { buffer.blit(dc) };
        }
    }

    /// The check mark of FR-93 — the smoothed figure of the mock-up, task T-11-21.
    ///
    /// Drawn by hand rather than by `DrawFrameControl(DFC_MENU, DFCS_MENUCHECK)`, because
    /// that call paints the system's mark in the system's colours whatever the palette of
    /// FR-92а says — the exact thing task T-11-10 existed to stop. Since task T-11-21 the
    /// hand is [`crate::theme::draw_check_mark`] and not this file's own two lines: the
    /// figure is [`MENU_CHECK_MARK`], the square is [`menu_check_cell`], and the smoothing
    /// is the one supersampling this program has — no second copy of it lives here.
    ///
    /// The ink is [`theme::Palette::text`] in both states, which is what it was before this
    /// task and what the mock-up draws (`$T.Fg`, on the highlighted row as on any other):
    /// the palette is task T-11-10's and is not touched here.
    fn draw_check_mark(&self, dc: HDC, rect: &RECT, dpi: i32) {
        let cell = menu_check_cell(rect, check_column(), dpi);

        theme::draw_check_mark(dc, &cell, self.palette.text, MENU_CHECK_MARK, dpi);
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
            // painted is gone by now, `window_bg` is no longer a brush Windows may paint
            // with, and **that is true by construction** since task T-13-18: the only
            // `drop` of a value of this type that a showing can reach is the `take` in
            // [`show_menu`] on the line after `TrackPopupMenuEx` returned, because
            // [`MenuOnScreen`] turns back every second entry into [`show_menu`] before it
            // reaches the `replace` above.
            //
            // ⚠ Until that task this comment asserted the same thing and the code did not
            // hold it up: a `WM_APP_TRAY` dispatched by the modal loop re-entered
            // [`show_menu`], and its `replace` ran this `Drop` on the live showing's paint
            // state — `DeleteObject` on the ground brush `SetMenuInfo` had handed to a menu
            // still on the screen (audit of 2026-08-24). The invariant is now enforced where
            // it is relied on, and not merely written down here.
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
/// without a menu on the screen; the rounding of the corners is [`crate::theme::paint_rounded`]
/// with [`crate::theme::CORNER_RADIUS`], which is the `6` the same call carries.
pub fn menu_hover_rect(item: &RECT, dpi: i32) -> RECT {
    let inset = theme::scaled(MENU_HOVER_INSET, dpi);

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
/// pixels at 96 DPI, which is `theme::scaled(38, 96)` to the pixel — and it still **grows
/// with the face**, because what the padding is added to is a measurement and not a constant.
///
/// Pure, for the reason [`menu_hover_rect`] is: a test reads it without a menu on the screen.
pub fn menu_item_height(text_height: i32, dpi: i32) -> i32 {
    text_height + 2 * theme::scaled_tenths(MENU_V_PAD_TENTHS, dpi)
}

/// The line of one rule of FR-91, inside the stripe whose rectangle is `item` — FR-92а, task
/// T-11-22.
///
/// [`MENU_SEP_INSET`] of the mock-up off the left and the right edge, [`MENU_SEP_THICKNESS`]
/// thick, lying on the vertical middle of the stripe — the `($iy + $sepH/2)` of the generator.
/// Pure, like [`menu_hover_rect`], and for the same reason: the geometry of a rule is a table a
/// test reads without a menu on the screen.
pub fn menu_separator_line(item: &RECT, dpi: i32) -> RECT {
    let inset = theme::scaled(MENU_SEP_INSET, dpi);
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
/// (NFR-13, the fallback [`crate::theme::dc_dpi`] itself names).
fn measure_dpi() -> i32 {
    // SAFETY: `None` asks for a DC of the screen, which needs no window of ours; a valid
    // handle is released below, on this same thread, as `ReleaseDC` requires.
    let dc = unsafe { GetDC(None) };

    if dc.is_invalid() {
        return theme::SCREEN_DPI;
    }

    let dpi = theme::dc_dpi(dc);

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
    let side = theme::scaled(MENU_CHECK_CELL, dpi);
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

// =========================================================================================
// FR-92а, task T-12-9 — the frame of the popup window the menu lives in
// =========================================================================================

/// The class every popup menu of Windows is a window of — `#32768`, the reserved atom.
///
/// ⚠ The number in that name is a coincidence and not a connection: `EVENT_OBJECT_CREATE` is
/// 32768 as well, and the two have nothing to do with each other.
const MENU_WINDOW_CLASS: [u16; 6] = [
    b'#' as u16,
    b'3' as u16,
    b'2' as u16,
    b'7' as u16,
    b'6' as u16,
    b'8' as u16,
];

thread_local! {
    /// The frame colour the hook below hands the menu window, as the four bytes of a
    /// `COLORREF` — written just before the hook goes in and read only by the hook.
    ///
    /// A `Cell<u32>` and not a `RefCell`: the hook callback runs at a moment the system
    /// chooses, and a `RefCell` there could meet a borrow of its own and panic. A `Cell` of a
    /// number can neither block nor allocate nor fail.
    static MENU_FRAME_COLOUR: Cell<u32> = const { Cell::new(0) };
}

/// Asks DWM to round the corners of the popup the menu lives in and to draw its frame in the
/// palette — FR-92а, task T-12-9.
///
/// # Why a hook, and why this one
///
/// A popup menu is a **window** of the reserved class [`MENU_WINDOW_CLASS`], and until this
/// task it was the last piece of this program still wearing the shell's chrome: a square
/// 1 px frame of `160,160,160`, a colour belonging to no palette of this program and to no
/// literal of the mock-ups. The mock-up draws that frame rounded and in `Border`.
///
/// The two attributes that change it are documented and take an `HWND`, and there is no
/// documented way to *ask* for the popup's `HWND`: `WM_INITMENUPOPUP` carries an `HMENU`, and
/// nothing documented leads back from one to the other. A hook on **this thread** is what
/// gives it, and the thread id is passed for exactly that reason — with `hMod` `None`, the
/// pairing the documentation requires when the thread belongs to the calling process.
///
/// ⚠ **This is not a search for somebody else's window and must never become one.** The hook
/// sees the windows of this one thread and no others.
///
/// # Why `WH_CALLWNDPROC` and not `WH_CBT`
///
/// Measured, in that order, and written down because the answer is not the obvious one.
/// `WH_CBT` is the hook that exists to announce a window being created, and its
/// `HCBT_CREATEWND` does carry the popup's handle — but it arrives **before `WM_NCCREATE`**,
/// and at that moment DWM does not yet know the window: both attribute calls answer
/// `0x80070006` (`ERROR_INVALID_HANDLE`). Nor does `WH_CBT` offer a later moment to move them
/// to: over a whole showing it delivered codes 3, 4 and 7 only — create, **destroy**, and a
/// skipped key — with no `HCBT_ACTIVATE` and no `HCBT_SETFOCUS` for that window at all.
///
/// `WH_CALLWNDPROC` gives the first live moment there is: `WM_NCCREATE` itself, where both
/// calls answer `S_OK` — before the popup is shown, so nothing is ever seen unrounded.
///
/// A refused hook is survived — NFR-13, and the menu comes up in the shell's chrome rather
/// than not at all.
fn install_menu_frame_hook(palette: &'static theme::Palette) -> MenuFrameHook {
    MENU_FRAME_COLOUR.with(|slot| slot.set(palette.field_border.0));

    // SAFETY: the procedure is a `'static` function of this module and outlives the hook,
    // which is removed by [`remove_menu_frame_hook`] before `show_menu` returns. `None` for
    // the module handle is what the documentation requires when the thread id names a thread
    // of the current process, and the thread named is this one.
    let hook = unsafe {
        SetWindowsHookExW(
            WH_CALLWNDPROC,
            Some(menu_frame_hook_proc),
            None,
            GetCurrentThreadId(),
        )
    };

    // NFR-13: the result is examined right here and the failure is survived. There is nothing
    // to journal — the closed vocabulary of `diag` has no row for this call, and the only
    // consequence of a refusal is the system's own frame around a correctly painted menu.
    MenuFrameHook(hook.ok())
}

/// The hook of [`install_menu_frame_hook`] for exactly as long as it is wanted — task T-12-9.
///
/// ⚠ **The removal is a `Drop` and not a call, so that no path out of the showing can miss
/// it** — not the ordinary one, not the one where `TrackPopupMenuEx` failed, and not an
/// unwind. A hook left in place would go on being called for every message sent on this
/// thread for the rest of the run, long after there is any menu to decorate.
///
/// `None` inside means the system refused the hook; there is then nothing to take out.
struct MenuFrameHook(Option<HHOOK>);

impl Drop for MenuFrameHook {
    fn drop(&mut self) {
        let Some(hook) = self.0.take() else {
            return;
        };

        // NFR-13: examined here and deliberately dropped. A refused removal cannot be
        // repaired by a second attempt with the same handle, and `take` above means no second
        // attempt is reachable.
        //
        // SAFETY: `hook` came from the successful `SetWindowsHookExW` of
        // [`install_menu_frame_hook`], is taken out of the field so it cannot be removed
        // twice, and the procedure it names is still in this module's image.
        let _ = unsafe { UnhookWindowsHookEx(hook) };
    }
}

/// The `WH_CALLWNDPROC` callback — task T-12-9.
///
/// ⚠ **This runs at a moment the system chooses, inside somebody else's call, for every
/// message *sent* on this thread while the menu is up.** It allocates nothing, takes no lock,
/// touches no file and journals nothing. The whole of its work on the overwhelming majority
/// of calls is one integer comparison: everything below `WM_NCCREATE` is skipped before any
/// pointer of ours is read, and a window is created far more rarely than a message is sent.
unsafe extern "system" fn menu_frame_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        // SAFETY: at a non-negative code `lParam` names a `CWPSTRUCT` the system owns for the
        // length of this call — the documented shape of this hook's parameter, and the
        // negative codes, which carry no structure, are excluded above.
        let sent = unsafe { &*(lparam.0 as *const CWPSTRUCT) };

        // The first moment the window exists as far as DWM is concerned; see
        // [`install_menu_frame_hook`] for the measurement that says so.
        if sent.message == WM_NCCREATE {
            // Seven cells for six characters and the terminator the call writes; the buffer
            // is this frame's and nothing outlives the call.
            let mut class = [0u16; 7];

            // SAFETY: `sent.hwnd` is the window the system is delivering a message to, and
            // `class` is a live local of this frame; the call writes at most its own length
            // and returns how much it wrote.
            let length = unsafe { GetClassNameW(sent.hwnd, &mut class) };

            if length == MENU_WINDOW_CLASS.len() as i32
                && class[..MENU_WINDOW_CLASS.len()] == MENU_WINDOW_CLASS
            {
                apply_menu_frame(sent.hwnd);
            }
        }
    }

    // SAFETY: the chain must be continued whatever this callback did — the documented
    // obligation of every hook procedure. `None` asks the system for the next hook itself.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Hands the popup the two attributes of task T-12-9 — the rounded corner and the frame of
/// the palette.
///
/// * `DWMWA_WINDOW_CORNER_PREFERENCE` (33) := `DWMWCP_ROUNDSMALL` (3) — the small radius, the
///   one the documentation reserves for «auxiliary UI» such as a menu. ⚠ The documentation is
///   honest that this is a *hint* that «does not guarantee rounding», and that popup menus are
///   not rounded by default; that it does round *this* popup is measured, not assumed —
///   task T-12-9 took the corner apart pixel by pixel to say so.
/// * `DWMWA_BORDER_COLOR` (34) := [`theme::Palette::field_border`] — 58,64,72 on «Графит»
///   against the mock-up's `Border` 58,63,71, and 207,213,220 on «Туман» against 209,214,220:
///   inside the Δ ≤ 2 the stage's terms of reference accepted. Asking for it replaces the
///   whole classic frame, `160,160,160` and all.
fn apply_menu_frame(hwnd: HWND) {
    let preference = DWMWCP_ROUNDSMALL;

    // NFR-13: both results are examined right here and the failure survived — the attributes
    // are a request and nothing else of this program depends on the answer. Nothing is
    // journaled: this is called from a callback, and a menu wearing the shell's frame is a
    // menu that works.
    //
    // SAFETY: `hwnd` is the live popup. Each attribute pointer names a live local of this
    // frame and the size passed is exactly that local's; the call copies the value and keeps
    // no pointer once it returns.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const preference).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };

    let colour = MENU_FRAME_COLOUR.with(Cell::get);

    // SAFETY: as above — four bytes of a `COLORREF` by value.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&raw const colour).cast(),
            size_of::<u32>() as u32,
        )
    };
}

/// Shows the menu of FR-91 at a point on the screen and carries out what was chosen.
///
/// Called with no borrow of the tray held: `TrackPopupMenuEx` runs a modal message loop that
/// dispatches back into our own window procedure.
///
/// # One showing at a time — task T-13-18
///
/// The first thing here is [`MenuOnScreen::raise`], and a `None` from it is the whole of the
/// answer: while a menu of ours is up, a second `WM_APP_TRAY` — a second click that queued
/// before the menu took the mouse, or a forged message from a process at the same integrity
/// level (SEC-05) — turns round on this line, before the tray is borrowed, before the shell or
/// GDI is touched and before anything of the live showing is disturbed.
///
/// **Why the flag and not `MENU_PAINT.is_some()`.** Because of the branch below where
/// [`MenuPaint::new`] is refused: the menu is then shown with the shell's own drawing and
/// [`MENU_PAINT`] stays `None` for the whole showing, so a gate reading it would wave the
/// second entry through into exactly the state this task exists to prevent. The two answer
/// different questions and the module header sets them out side by side.
///
/// **Where the right is given up, and why there.** [`MenuOnScreen`] is dropped by name on the
/// line after the `take` of [`MENU_PAINT`] — that is, once `TrackPopupMenuEx` has returned and
/// the popup window is gone, and **before** [`dispatch_command`]. Before, and not at the end
/// of the function, because `dispatch_command` opens the modal dialog of FR-92, and a menu
/// opened over that dialog is a legitimate showing that task T-13-14 greys two entries of.
/// Holding the right across `dispatch_command` would refuse it and undo that task; releasing
/// it any earlier would reopen the window this one closes.
fn show_menu(x: i32, y: i32) {
    // Task T-13-18. Held from here to the `drop` below; every early return of this function
    // gives it up on the way out, and so does an unwind — see [`MenuOnScreen`].
    let Some(on_screen) = MenuOnScreen::raise() else {
        return;
    };

    // The number the acceptance of task T-13-18 reads — see [`MENU_SHOWINGS`] for why it is
    // counted here, past the gate, and not inside it. Test-only; the product has no counter.
    #[cfg(test)]
    MENU_SHOWINGS.with(|count| count.set(count.get().saturating_add(1)));

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

    // Tasks T-13-9 and T-13-14: both read once, here, and handed to the builder — the same
    // direction the two states above travel, and the reason [`resume_is_refused`] and
    // [`dialog_locks_command`] take arguments at all.
    //
    // ⚠ That the second of the two can be true at all is the finding this menu is built
    // against: `TrackPopupMenuEx` and `DialogBoxParamW` both run message loops of their own,
    // so the modal settings dialog does not stop the icon being clicked and does not stop this
    // function running underneath it. That is also why the gate of task T-13-18 is given up
    // before `dispatch_command` and not at the end of this function: this very line has to
    // stay reachable while the dialog that greys the two entries is up.
    let menu = match Menu::build(
        enabled,
        autostart,
        crate::hook::fail_safe(),
        settings::dialog_is_open(),
        // FR-101: what the letters have to say right now, read once, here, and handed to the
        // builder — the direction the three states above travel and for the same reason.
        &pending_now(),
    ) {
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
    //
    // ⚠ **That refusal is why this is not the gate of task T-13-18 as well.** On this branch
    // the slot below stays `None` for a whole showing that is on the screen, so «is our
    // drawing active?» and «is a menu of ours up?» give different answers, and the second
    // question is [`MenuOnScreen`]'s — raised at the top of this function, already up on both
    // paths through the `if`. `replace` here can therefore no longer land on a live showing's
    // paint state, which is the finding of the audit of 2026-08-24.
    if let Some(paint) = MenuPaint::new(menu.items().to_vec(), palette) {
        // FR-92а, task T-11-22: the ground **between** the entries — the padding the menu
        // keeps around them — is the palette's from here on. The brush is the one
        // [`MenuPaint`] already owns and lives exactly as long as the showing does, which is
        // the whole of what `SetMenuInfo` requires: it is set before `TrackPopupMenuEx` and
        // the brush is freed after it returns, when the value below is taken out and dropped.
        set_menu_background(menu.handle(), paint.window_bg);

        MENU_PAINT.with(|slot| slot.replace(Some(paint)));
    }

    // Task T-12-9: the frame of the popup **window** the menu will live in, asked for just
    // before the window exists and given up the moment it stops existing. See
    // [`install_menu_frame_hook`] for why this needs a hook at all.
    let frame_hook = install_menu_frame_hook(palette);

    // SAFETY: `menu.handle()` is a live popup menu owned by `menu` for the whole call — the
    // value is dropped at the end of this function, after `TrackPopupMenuEx` has returned,
    // and the call does not take ownership of it. `hwnd` is our live window, which is what
    // receives the menu's messages. `None` for `TPMPARAMS` is the documented way to ask for
    // no excluded rectangle. `TPM_RETURNCMD` makes the call *return* the chosen identifier
    // instead of posting `WM_COMMAND` — which is why this program has no `WM_COMMAND`
    // handler at all (SEC-05) — and `TPM_NONOTIFY` suppresses the notifications that would
    // otherwise accompany the choice.
    //
    // **`TPM_LAYOUTRTL` — вопрос 97, task Т-30-2.** The one flag that mirrors this menu, asked
    // for here and nowhere else. Measured on its own (`scratchpad-Э30\посылки-п1б.log`): with
    // the flag the popup came back carrying `WS_EX_LAYOUTRTL` and a mirrored DC, without it it
    // did not — so no process-wide `SetProcessDefaultLayout` is needed to get a mirrored menu,
    // and none is set. That matters beyond tidiness: a process-wide layout outlives this call
    // and would mirror the next window this program grows, by an inheritance nobody set.
    let mut flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;

    if crate::settings::ui_language().is_rtl() {
        flags |= TPM_LAYOUTRTL.0;
    }

    let chosen = unsafe { TrackPopupMenuEx(menu.handle(), flags, x, y, hwnd, None) };

    // Task T-12-9: out on **every** path, including the one where the call above failed —
    // `TrackPopupMenuEx` has returned by this line however it returned, and the popup window
    // it created, if it created one, is gone. Dropped here by name rather than left to the
    // end of the function so that the hook's life is visibly the modal call's and no longer.
    drop(frame_hook);

    // The gate comes down here, immediately after the return: from this line on the two
    // messages are foreign again (SEC-05) — in particular before `dispatch_command` below
    // opens anything modal. Taken out of the `RefCell` before being dropped, like
    // [`detach`] and for the same reason; the drop frees the three brushes and the menu face,
    // whose whole life is the one showing (FR-92а) — the ground brush `SetMenuInfo` was given
    // among them, and the menu it was given to is gone by this line.
    let paint = MENU_PAINT.with(|slot| slot.borrow_mut().take());
    drop(paint);

    // Task T-13-18: the second gate comes down here and not at the end of the function.
    // `TrackPopupMenuEx` has returned, the popup window is gone and the paint state of the
    // showing has been freed above, so there is no longer a menu of ours for a second entry to
    // walk into — while `dispatch_command` below opens the modal dialog of FR-92, underneath
    // which a menu **is** allowed to be shown (task T-13-14 greys two of its entries for
    // exactly that case). Dropped by name for the reason `frame_hook` is, one gate above.
    drop(on_screen);

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
///
/// # The gate of task T-13-14 — and why it is here as well as in the menu
///
/// While the modal dialog of FR-92 is on the screen, [`CMD_TOGGLE`] and [`CMD_AUTOSTART`] are
/// refused and **nothing at all happens**: no change in memory, no icon, no file, no registry.
/// [`Menu::build`] greys the same two entries, and that is not a duplicate of this line but its
/// other half — greying is what the user sees and what keeps `TrackPopupMenuEx` from returning
/// the command, and *this* is what decides. A menu is user interface; a command handler is the
/// door, and the door is where a lock belongs. The two read one rule,
/// [`dialog_locks_command`], so they cannot come apart.
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
    // Task T-13-14. One door in front of the whole table rather than a check inside two of
    // the arms: a gate at the top cannot be walked round by an arm added later, and it is the
    // shape that makes «which commands are locked» a property of the rule and of nothing else.
    // The three commands the rule does not name — «Настройки…», «О программе», «Выход» — reach
    // their arms exactly as before.
    if dialog_locks_command(command, settings::dialog_is_open()) {
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
        CMD_AUTOSTART => toggle_autostart(),

        // Outside any borrow of the tray: the about dialog is modal and pumps messages.
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

    Pending { unread, update }
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

/// Opens the settings dialog of FR-92 and applies whatever it produces.
///
/// Called with **no borrow of the tray held**: `DialogBoxParamW` runs a modal message loop that
/// dispatches back into this module, exactly as `TrackPopupMenuEx` does. The configuration the
/// dialog starts from is therefore *copied* out of the tray first, and what comes back arrives
/// through [`apply_settings`].
///
/// # The loop — решение 99.1(б), task Т-31-2
///
/// «Применить» that changed the **direction of writing** answers
/// [`settings::DialogOutcome::Reopen`] instead of closing: the mirror of task Т-30-2 is a style
/// of the template a window is created from, and there is no later moment to put it on. So the
/// window is built again, in the rectangle the previous one stood in, out of a configuration
/// that has already been written to the file — there is nothing to lose and nothing to carry
/// across.
///
/// **The loop stands here and not inside the dialog procedure**, at the very depth the first
/// showing was made from and with no borrow of the tray held — посылка П2 of the mandate,
/// measured before this was written (`scratchpad-Э31\посылки-п2.log`): two showings out of one
/// call, and `with_tray` answering between them.
///
/// It cannot spin: the second showing starts in the language the first one published, so
/// `language_switch` there answers `Unchanged` until a person picks another language and presses
/// «Применить» again. Every turn of this loop is a press of a button.
fn open_settings(hwnd: HWND) {
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

    let mut opening = settings::Opening::Fresh;

    loop {
        // Read afresh on every turn: the previous showing wrote its «Применить» through
        // `apply_settings` into this very tray, and the window that comes up next must start
        // from what is in force and not from a copy taken before it.
        let Some(config) = with_tray(|tray| tray.config().clone()) else {
            return;
        };

        let mut apply = apply_settings;

        match settings::show_dialog(hwnd, HINSTANCE(module.0), &config, &mut apply, opening) {
            Ok(settings::DialogOutcome::Closed) => return,
            Ok(settings::DialogOutcome::Reopen(at)) => opening = settings::Opening::Again(at),
            Err(error) => {
                // NFR-13. The dialog either came up or it did not, and if it did not the user is
                // told by the absence of a window; the reason goes to the journal.
                app::report_non_critical("DialogBoxParamW", &error);
                return;
            }
        }
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
///
/// The registry is the one destination of the three that can answer no, and what is done with
/// that answer is [`apply_settings_via`], one function down.
fn apply_settings(config: &Config) {
    apply_settings_via(config, settings::set_autostart);
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

    settings::set_ui_language(language);

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

/// Shows the "О программе" window: the name, the version, two lines of description, the
/// «Как пользоваться» panel of task Т-23-4 and an «ОК» — and nothing else. FR-92а, task
/// T-11-11: the window is the own-drawn dialog
/// `IDD_ABOUT` of [`crate::settings`], shown in the resolved palette, because the
/// `MessageBoxW` this function used to call cannot be repainted by any documented means.
///
/// Called in the same order the box was called: from [`dispatch_command`], reached only
/// through [`handle_ui_message`], with **no borrow of the tray held** — the dialog is modal
/// and pumps messages, exactly like `TrackPopupMenuEx` (the rule of the module
/// documentation). The theme setting is *copied* out of the tray first, in a borrow that
/// ends before the modal call begins — the same shape [`open_settings`] has.
fn show_about(hwnd: HWND) {
    // FR-92а, task Т-23-4: the help panel of решение 82.5 names the hotkey, so the key travels
    // beside the theme setting — copied out of the tray in **one** borrow that ends before the
    // modal call begins, exactly as the setting alone used to be.
    let Some((setting, key)) = with_tray(|tray| {
        (
            tray.config().general.theme,
            tray.config().hotkey.key.clone(),
        )
    }) else {
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
    match settings::show_about_dialog(hwnd, HINSTANCE(module.0), setting, file_version(), &key) {
        // FR-103, task Т-32-4: «От автора…» ended the modal window, and the modeless one is
        // opened **here** — after the modal call has returned, at the depth the about window
        // was shown from, and owned by this program's own window rather than by one that has
        // just closed.
        Ok(true) => crate::letters::open_author(hwnd),
        Ok(false) => {}
        Err(error) => {
            // NFR-13. The dialog either came up or it did not, and if it did not the user is
            // told by the absence of a window; the reason goes to the journal.
            app::report_non_critical("DialogBoxParamW", &error);
        }
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
    fn load(instance: HINSTANCE) -> Option<Self> {
        let size = large_icon_size();

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

/// The size the shell draws the icon of a **balloon** at — задача Т-33а-1.
///
/// `SM_CXICON` and not a fixed 32: like the small metric it follows the display scale — 32 px at
/// 100 %, 40 at 125 %, 48 at 150 %, — and the four `.ico` files carry 32, 48 and 64 px frames,
/// so `LoadImageW` has an exact frame to hand back at each of those and nothing is stretched.
pub fn large_icon_size() -> (i32, i32) {
    // SAFETY: as in `small_icon_size` — a system-wide read that takes no pointer. A zero would
    // ask `LoadImageW` for the resource's own size, so the result is examined (NFR-13).
    let width = unsafe { GetSystemMetrics(SM_CXICON) };

    // SAFETY: as above.
    let height = unsafe { GetSystemMetrics(SM_CYICON) };

    // Thirty-two is what the large-icon metric is at 100% scale and is the safe fallback.
    let usable = |value: i32| if value > 0 { value } else { 32 };

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
/// window uses («Lang Switcher — настройки»), and they are not translated either.
pub fn icon_tip(enabled: bool) -> String {
    let state = settings::text(if enabled {
        settings::IDS_TIP_ACTIVE
    } else {
        settings::IDS_TIP_PAUSED
    });

    format!("{APP_NAME} — {state}")
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

/// [`write_tip`] for a field of any length — the two balloon fields of FR-101, task Т-32-4.
///
/// `szInfo` is 256 units and `szInfoTitle` 64, and neither is the 128 of `szTip`; a generic
/// body is one place the truncation can be wrong instead of three. The rule is the same:
/// **cut, never refuse** — a sentence of a locale nobody measured must not be able to stop a
/// letter being announced (NFR-13), and it is cut at a UTF-16 unit, which is where the field
/// itself ends.
fn write_field<const N: usize>(field: &mut [u16; N], text: &str) {
    let limit = field.len().saturating_sub(1);
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
    use windows::Win32::Graphics::Gdi::GetObjectType;

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
    #[test]
    fn a_refused_write_reports_the_system_code_and_a_parser_position_reports_nothing() {
        /// `ERROR_ACCESS_DENIED`, and the `HRESULT` Win32 codes are journalled under.
        const ACCESS_DENIED: i32 = 5;
        const AS_HRESULT: i32 = 0x8007_0005_u32 as i32;

        let refused = settings::ConfigError::Io(std::io::Error::from_raw_os_error(ACCESS_DENIED));

        assert_eq!(
            os_code_of(&refused).raw(),
            AS_HRESULT,
            "the number the operating system gave, in the form every other failure is recorded in"
        );

        // An `io::Error` this program built itself carries no system number, and none is invented.
        let ours = settings::ConfigError::Io(std::io::Error::other("no system call failed here"));

        assert_eq!(os_code_of(&ours).raw(), diag::OsCode::NONE.raw());

        // SEC-01, SEC-07: the position a parser stopped at is a fact about the file's contents.
        for error in [
            settings::ConfigError::Malformed { at: Some((12, 34)) },
            settings::ConfigError::Malformed { at: None },
            settings::ConfigError::Serialize,
        ] {
            assert_eq!(
                os_code_of(&error).raw(),
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

    // -----------------------------------------------------------------------------------
    // The re-entry of `show_menu` — task T-13-18
    // -----------------------------------------------------------------------------------
    //
    // ⚠ These live here and not in `tests\tray.rs` because everything they read is private
    // to this module by design: [`show_menu`], [`MENU_PAINT`], [`MENU_ON_SCREEN`] and
    // [`MenuOnScreen`]. Publishing any of them so that an integration test could reach it
    // would widen the surface of SEC-05 for the sake of a test, which is the wrong trade.
    //
    // ⚠ **Nothing here puts a menu on the screen, and nothing here can.** Two of the three
    // tests call [`show_menu`] with the flag already up, so the call turns round on its first
    // line — before the tray, before the shell, before GDI. The third calls it with the flag
    // down, and on a test thread that has no tray it returns one line further on, at
    // [`with_tray`]. `TrackPopupMenuEx` is unreachable from all three, which is what keeps a
    // suite that must not run the product from hanging on a modal loop.

    /// The number the gate moves: entries into [`show_menu`] that got past its first line.
    fn showings() -> u32 {
        MENU_SHOWINGS.with(Cell::get)
    }

    /// The four GDI handles of a paint state, as plain numbers — identity, not liveness.
    fn handles_of(paint: &MenuPaint) -> [isize; 4] {
        [
            paint.font.0 as isize,
            paint.window_bg.0 as isize,
            paint.hover_bg.0 as isize,
            paint.panel_border.0 as isize,
        ]
    }

    /// What GDI says each of those four handles is right now — zero means «not an object of
    /// mine any more», which is precisely the dangling handle of the finding.
    fn kinds_of(paint: &MenuPaint) -> [u32; 4] {
        // SAFETY: `GetObjectType` takes a handle by value, reads no memory of ours and
        // writes none. It is documented to answer zero for a handle that is not a live GDI
        // object, which is the answer this asks for — the four handles are those of a paint
        // state this test owns and has not dropped.
        unsafe {
            [
                GetObjectType(paint.font.into()),
                GetObjectType(paint.window_bg.into()),
                GetObjectType(paint.hover_bg.into()),
                GetObjectType(paint.panel_border.into()),
            ]
        }
    }

    /// Empties [`MENU_PAINT`] the way [`show_menu`] does, so the thread leaves no GDI
    /// objects behind whichever way the test ended.
    fn clear_paint() {
        let paint = MENU_PAINT.with(|slot| slot.borrow_mut().take());
        drop(paint);
    }

    /// **Criterion 5 of task T-13-18.** A second `WM_APP_TRAY` over a live showing — the
    /// click that queued before the menu took the mouse, or the forged message SEC-05 admits
    /// is postable by any process at the same integrity level — turns round at the gate, and
    /// the paint state of the live showing is *the same object*, with its brushes still
    /// brushes.
    ///
    /// The finding of the audit of 2026-08-24 is exactly what the last of those assertions
    /// would catch: before this task the second entry ran
    /// `MENU_PAINT.with(|slot| slot.replace(Some(paint)))` on a live showing, whose `Drop`
    /// then called `DeleteObject` on the ground brush `SetMenuInfo` had handed to the menu
    /// still on the screen. `GetObjectType` answers zero for a handle in that state.
    #[test]
    fn a_second_show_menu_over_a_live_showing_turns_round_at_the_gate() {
        // The live showing, assembled in the order `show_menu` assembles it: the right to
        // the screen first, the paint state of that showing second.
        let showing = MenuOnScreen::raise().expect("the first showing must be given the screen");
        let paint = MenuPaint::new(Vec::new(), &theme::GRAPHITE)
            .expect("this thread must be able to make a menu face and three brushes");

        let before_handles = handles_of(&paint);
        let before_kinds = kinds_of(&paint);

        MENU_PAINT.with(|slot| slot.replace(Some(paint)));

        let before = showings();

        // The second entry. With the flag up this returns on its first line.
        show_menu(7, 11);

        let after = showings();

        let (after_handles, after_kinds) = MENU_PAINT.with(|slot| {
            let borrowed = slot.borrow();
            let paint = borrowed
                .as_ref()
                .expect("the live showing's paint state must still be in the slot");

            (handles_of(paint), kinds_of(paint))
        });

        println!("showings past the gate: {before} -> {after}");
        println!("handles: {before_handles:?} -> {after_handles:?}");
        println!(
            "GetObjectType (0 = not a live object; 6 = OBJ_FONT, 2 = OBJ_BRUSH): \
             {before_kinds:?} -> {after_kinds:?}"
        );

        clear_paint();
        drop(showing);

        assert_eq!(
            after, before,
            "the second entry must not get past the gate — the counter is incremented on \
             the line after it and nowhere else"
        );
        assert_eq!(
            after_handles, before_handles,
            "the paint state must be the very same object: a re-created one would carry \
             other handles"
        );
        assert_eq!(
            after_kinds, before_kinds,
            "and the same objects: this is the assertion the finding fails"
        );
        assert!(
            after_kinds.iter().all(|kind| *kind != 0),
            "the menu face and the three brushes must still be live GDI objects — the \
             ground brush among them is the one Windows is painting the live menu with"
        );
    }

    /// **Criterion 6 of task T-13-18 — the half the second flag exists for.**
    ///
    /// [`MenuPaint::new`] is refused through the seam, which is the degraded showing of
    /// NFR-13: the menu is on the screen, drawn by the shell, and [`MENU_PAINT`] holds
    /// `None` for the whole of it. The early exit the audit proposed — `MENU_PAINT.is_some()`
    /// — reads `false` in that state and would wave the second entry through. The flag does
    /// not.
    #[test]
    fn a_degraded_showing_without_paint_state_of_its_own_is_refused_a_second_entry_too() {
        let showing = MenuOnScreen::raise().expect("the degraded showing takes the screen too");

        REFUSE_MENU_PAINT.with(|flag| flag.set(true));

        // The refusal itself, through the branch `show_menu` takes: `if let Some(paint) =
        // MenuPaint::new(..)` simply does not fire, and the slot is never filled.
        let refused = MenuPaint::new(Vec::new(), &theme::GRAPHITE);
        let paint_gate = MENU_PAINT.with(|slot| slot.borrow().is_some());

        let before = showings();

        show_menu(7, 11);

        let after = showings();

        println!("MenuPaint::new refused: {}", refused.is_none());
        println!("MENU_PAINT.is_some() during the degraded showing: {paint_gate}");
        println!(
            "MENU_ON_SCREEN during the same: {}",
            MENU_ON_SCREEN.with(Cell::get)
        );
        println!("showings past the gate: {before} -> {after}");

        REFUSE_MENU_PAINT.with(|flag| flag.set(false));
        drop(showing);

        assert!(
            refused.is_none(),
            "the seam must produce the state NFR-13 describes — no paint state for this \
             showing"
        );
        assert!(
            !paint_gate,
            "and the SEC-05 gate is therefore down while a menu of ours is on the screen: \
             a gate reading it would have let the second entry through"
        );
        assert_eq!(
            after, before,
            "the second entry must be refused all the same — this is what the flag is for"
        );
    }

    /// **Criterion 7 of task T-13-18.** The right to the screen comes back on every path out,
    /// and the next legitimate showing gets in.
    ///
    /// The panic half is not decoration: `TrackPopupMenuEx` dispatches other people's
    /// messages into this program, a `debug_assert!` in that path unwinds in the build the
    /// tests run, and a flag left up by such an unwind would wedge the menu shut for the rest
    /// of the run. A bare `set(true) … set(false)` pair around the modal call fails exactly
    /// this test.
    #[test]
    fn the_right_to_the_screen_comes_back_on_every_path_out_including_a_panic() {
        let first = MenuOnScreen::raise().expect("the first showing must be given the screen");
        let while_up = MenuOnScreen::raise().is_none();

        drop(first);

        let after_drop = MENU_ON_SCREEN.with(Cell::get);

        // A legitimate showing, past the gate. On a thread with no tray it returns at
        // `with_tray` one line later, so nothing of the shell is touched — but the counter
        // has already moved, which is what says the gate let it through.
        let before = showings();
        show_menu(7, 11);
        let after = showings();
        let after_show = MENU_ON_SCREEN.with(Cell::get);

        // The unwinding path.
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _showing = MenuOnScreen::raise().expect("the guard must be raised here");

            panic!("T-13-18: a deliberate unwind out of a showing");
        }));
        let after_panic = MENU_ON_SCREEN.with(Cell::get);

        // And the menu is not wedged shut afterwards.
        let before_again = showings();
        show_menu(7, 11);
        let after_again = showings();

        println!("a second raise while the first is up refused: {while_up}");
        println!("flag after the drop: {after_drop}");
        println!("showings past the gate: {before} -> {after}, flag after: {after_show}");
        println!(
            "panic caught: {}, flag after: {after_panic}",
            unwound.is_err()
        );
        println!("showings past the gate after the panic: {before_again} -> {after_again}");

        assert!(while_up, "one showing at a time");
        assert!(!after_drop, "the Drop of the guard lowers the flag");
        assert_eq!(
            after,
            before + 1,
            "the next legitimate showing passes the gate"
        );
        assert!(!after_show, "and gives the right back on its way out");
        assert!(unwound.is_err(), "the panic must have been caught here");
        assert!(
            !after_panic,
            "an unwind out of a showing must lower the flag as well — this is why the \
             lowering is a Drop and not a call"
        );
        assert_eq!(
            after_again,
            before_again + 1,
            "and the menu is not wedged shut for the rest of the run"
        );
    }

    // -----------------------------------------------------------------------------------
    // `WM_SETTINGCHANGE`: the string is read only when there is somebody to hand it to —
    // task T-13-20
    // -----------------------------------------------------------------------------------
    //
    // ⚠ These live here and not in `tests\tray.rs` for the reason the three above do:
    // [`setting_change_name`], [`on_setting_change`] and the counter are private to this
    // module and are staying private. A reader that dereferences a pointer out of a window
    // message is the last thing in this program to publish for the sake of a test — that
    // would widen the very surface SEC-05 narrows. What `tests\tray.rs` checks instead is
    // the published half: the table of [`setting_name_is_wanted`] and the product's own
    // call sites.
    //
    // ⚠ Every thread-local these read — the counter, [`settings::dialog_is_open`],
    // [`settings::about_is_open`] — belongs to the thread running the test, so the four
    // tests below do not see each other however `cargo test` schedules them.

    /// The number the reader moves: entries into [`setting_change_name`] on this thread.
    fn reads() -> u32 {
        SETTING_NAME_READS.with(Cell::get)
    }

    /// The «мусорный ненулевой `lParam`» of criterion 5 — a number that is a pointer to
    /// nothing.
    ///
    /// Deliberately unreadable, and deliberately harmless in being so. Windows reserves the
    /// first 64 KiB of every process's address space — the null partition — and maps nothing
    /// there ever, so this number cannot alias a page of the test, of the runtime or of
    /// anybody else, and a read of it is an access violation and nothing subtler.
    ///
    /// That is the point of choosing it. The proof these tests offer is that execution
    /// **never reached** the read; a pointer that happened to survive being read would prove
    /// nothing at all about the gate.
    fn a_pointer_to_nothing() -> LPARAM {
        LPARAM(0x2A2A)
    }

    /// A legitimate `WM_SETTINGCHANGE` payload: the one word of FR-92а, NUL-terminated, on
    /// the caller's own frame.
    fn immersive_color_set() -> Vec<u16> {
        settings::IMMERSIVE_COLOR_SET
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    /// A handle that names no window — the record test of task T-13-17 uses the same trick
    /// and for the same reason: what is under test is the *record*, not the dialog manager.
    /// Nothing is ever sent to this number; `GetWindowLongPtrW` answers zero for it, which is
    /// where `on_system_theme_message` stops.
    fn a_handle_that_names_no_window() -> HWND {
        HWND(0x1357 as *mut std::ffi::c_void)
    }

    /// **Criterion 5 of task T-13-20 — «некому отдать».**
    ///
    /// The state the audit found: no window of FR-92а on the screen, and a `WM_SETTINGCHANGE`
    /// carrying a non-null `lParam` that names nothing. Before this task the arm read up to 64
    /// UTF-16 units from that address and handed the result to a consumer that left on its
    /// first line. Now the reader is not entered at all — and the counter says so on the line
    /// *before* the null check, so the assertion is «читателя не звали» and not the weaker
    /// «читатель ничего не разыменовал».
    ///
    /// That this test returns at all is the second half of the proof: the address is in the
    /// null partition, and a build that let execution through to the read would not fail this
    /// assertion — it would take the whole test process down with an access violation.
    #[test]
    fn a_setting_change_with_nobody_to_tell_never_looks_at_its_lparam() {
        assert!(
            !settings::dialog_is_open(),
            "this thread has no settings dialog"
        );
        assert!(
            !settings::about_is_open(),
            "and no «О программе» window either"
        );

        let wanted = setting_name_is_wanted(
            settings::dialog_is_open(),
            settings::about_is_open(),
            theme::ThemeSetting::System,
        );

        let before = reads();
        on_setting_change(a_pointer_to_nothing(), theme::ThemeSetting::System);
        let after = reads();

        println!("gate: {wanted}; reader entries: {before} -> {after}");

        assert!(!wanted, "with neither window up there is nobody to tell");
        assert_eq!(
            after, before,
            "and the reader must not have been entered — the finding of the audit of \
             2026-08-24 is exactly this call happening anyway"
        );
    }

    /// **Criterion 6 of task T-13-20 — «есть кому отдать».** The dialog is open and the theme
    /// is `system`: the string is read, exactly as before this task, and it comes out as the
    /// word FR-92а names.
    ///
    /// The flag is raised through [`settings::DialogSession`] — the product's own guard and
    /// the only thing that raises it — so what the gate reads here is what it reads in the
    /// running program. No window is behind the flag, so `on_system_theme_message` stops at
    /// its own empty record; that half of the road belongs to `tests\settings.rs`, which
    /// closes it by a table.
    #[test]
    fn an_open_dialog_under_the_system_theme_is_told_as_it_always_was() {
        let text = immersive_color_set();
        let lparam = LPARAM(text.as_ptr() as isize);

        let _open = settings::DialogSession::open();

        assert!(settings::dialog_is_open(), "the guard raised the flag");

        let before = reads();
        on_setting_change(lparam, theme::ThemeSetting::System);
        let after = reads();

        // The value itself, over the very buffer the arm was handed: the reading half of
        // task T-11-9 is unchanged, and this is where that is said.
        //
        // SAFETY: `text` is a NUL-terminated UTF-16 buffer owned by this frame and alive
        // across the call — a stronger guarantee than the delivery of a real broadcast gives
        // — and the gate above has just answered yes, which is the third precondition.
        let name = unsafe { setting_change_name(lparam) };

        println!("reader entries: {before} -> {after}; the string read: {name:?}");

        assert_eq!(
            after,
            before + 1,
            "the arm must have entered the reader exactly once"
        );
        assert_eq!(
            name.as_deref(),
            Some(settings::IMMERSIVE_COLOR_SET),
            "and the reading itself is what it was before this task"
        );
    }

    /// **Criterion 7 of task T-13-20 — тема задана вручную.** The dialog is open, so there is
    /// a window on the screen; but `[general].theme` is `light` or `dark`, and under those
    /// FR-92а fixes the palette outright. The system's switch has no say, nothing would be
    /// repainted whatever the string said, and so the string is not read.
    ///
    /// Driven with the same pointer to nothing as criterion 5: this branch has to refuse
    /// before the dereference too, not merely refuse afterwards.
    #[test]
    fn a_theme_fixed_by_hand_stops_the_reading_even_with_a_window_up() {
        let _open = settings::DialogSession::open();

        assert!(settings::dialog_is_open(), "a window *is* up for this test");

        let before = reads();

        for setting in [theme::ThemeSetting::Light, theme::ThemeSetting::Dark] {
            let wanted = setting_name_is_wanted(true, false, setting);

            on_setting_change(a_pointer_to_nothing(), setting);

            println!("{setting:?}: gate {wanted}");

            assert!(
                !wanted,
                "{setting:?} is fixed by FR-92а — the system has no say"
            );
        }

        let after = reads();

        println!("reader entries: {before} -> {after}");

        assert_eq!(
            after, before,
            "neither fixed theme may reach the reader — the message changes nothing under \
             them, so the pointer is a risk taken for no purpose at all"
        );
    }

    /// **Ловушка 3 of task T-13-20, driven and not merely tabled.** Task T-13-17 is in this
    /// tree, so «есть кому отдать» is «диалог открыт **или** about жив» — and the second
    /// disjunct has to carry the gate on its own.
    ///
    /// A gate written as `dialog_is_open()` alone passes every other test in this file and
    /// fails this one, which is precisely what it is here for: the about window would stop
    /// following the system theme, and task T-13-17's repair would be undone in silence.
    #[test]
    fn the_about_window_alone_is_somebody_to_tell() {
        let text = immersive_color_set();
        let lparam = LPARAM(text.as_ptr() as isize);

        let _about = settings::AboutSession::open();
        settings::AboutSession::record(a_handle_that_names_no_window());

        assert!(
            !settings::dialog_is_open(),
            "no settings dialog — the first disjunct is false"
        );
        assert!(settings::about_is_open(), "and the second one is true");

        let before = reads();
        on_setting_change(lparam, theme::ThemeSetting::System);
        let after = reads();

        println!("reader entries with only the about window up: {before} -> {after}");

        assert_eq!(
            after,
            before + 1,
            "the about window is somebody to tell, and a gate that asked only about the \
             settings dialog would have refused here"
        );
    }
}
