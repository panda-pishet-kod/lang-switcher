//! The host: the window of Windows a window of the shell lives in — its class, its frame, the
//! mouse, the keyboard, the theme, the scale of the screen and the language, all live, without a
//! blank frame and without making the window again.
//!
//! What the window shows is its [`Content`]'s: a description built anew from the content's state
//! ([`Content::describe`]) and what the content does with an [`Event`]. The host lays the
//! description out (`layout`, `fit`), draws it in one frame into one buffer (`canvas`,
//! `elements`) and turns the mouse and the keys into events.
//!
//! * **One frame.** No erasing of the background: the canvas paints every pixel, and the window is
//!   cloaked (`DWMWA_CLOAK`) until its first frame is drawn, so it never shows empty.
//! * **The frame of the window** is the system's: icon, title, minimise, close — no maximise; the
//!   frame is dragged between 620 × 500 and 1280 × 720 points (`WM_GETMINMAXINFO`, at the scale of
//!   the screen). A dark title bar by the documented attribute of DWM, and the colour `caption` on
//!   Windows 11; Windows 10 refuses that one and keeps its own.
//! * **On the task bar** like the windows before it: `WS_EX_APPWINDOW` (task T-21-4 — an owned
//!   window inherits its owner's absence from the bar).
//! * **The mouse:** under the mouse, pressed, and the action on release inside the same element.
//!   **The keys:** Tab and Shift+Tab through the elements in the order of the description, arrows
//!   inside a group (segments, sections), the space bar presses, Enter is the focused button or the
//!   main one, Esc closes (in a wizard — the content's «Cancel»). The keyboard ring shows only after
//!   a key. The strings carry no accelerator letters today (no `&` in `res\lang`), so none are read.
//! * **The text fields** are fields of Windows inside drawn wrappers (`field.rs`): placed by the
//!   layout, coloured through `WM_CTLCOLOREDIT`, their typing an `Edited` event; Tab, Shift+Tab, Esc
//!   and Enter pressed in a field come back here, so the keyboard ring goes between a field and the
//!   drawn elements both ways (T-120-18).
//! * **A list** is one stop of Tab: the arrows, Home, End, Page Up and Down select rows (and scroll
//!   the selected one into view), the space bar turns the check box of the selected row, the wheel
//!   scrolls the list under the mouse; a click on a row with a check box selects it and turns it.
//! * **Live:** a change of the theme of Windows or of the content's own setting, of the scale
//!   (`WM_DPICHANGED`, the rectangle the system offers) and of the language lays out and draws
//!   again in the same window.
//! * **A popup** ([`open_popup`], T-120-14): no caption and no button on the task bar, above the
//!   windows, the roundness (Windows 11) and the shadow of the system; it stands in the corner of
//!   the work area nearest the icons of the taskbar — measured: the program's icon the caller gives
//!   (`Shell_NotifyIconGetRect`), else the end of the taskbar (`SHAppBarMessage`) the interface of
//!   Windows reads to ([`corner`]) — and comes in from its side in 0,34 s, or stands at once when
//!   Windows' animation is off. It takes no focus, a click neither; Esc closes it when the keys
//!   reach it.
//! * Movement only where the description asks for it: a tick ([`Content::tick`], T-120-25) while the
//!   window is shown and Windows' animation is on, the frames of a picture, a popup coming in.
//!
//! FR-72: the host sends no message to another window at all.
//!
//! Knows nothing of what a window contains.

use crate::theme::{ThemeSetting, resolve, system_is_light, title_bar_is_dark};
use crate::ui::canvas::{Canvas, POINT_DPI, Rect};
use crate::ui::elements::{self, Look, State, hit_area, input_fill};
use crate::ui::field::{self, Fields, Spec};
use crate::ui::fit;
use crate::ui::layout::menu_width;
use crate::ui::layout::{
    Layout, ListView, Part, client_size, lay_out, least_window_size, window_size,
};
use crate::ui::scene::{ButtonKind, Event, Window};
use crate::ui::scene::{ListItem, Node, WindowKind};
use crate::ui::text::{Direction, Style, Text};
use crate::ui::text::{Measure, Wrap};
use crate::ui::tokens::MENU_MIN_WIDTH;
use crate::ui::tokens::motion;
use crate::ui::tokens::{CHECK, CONTROL_GAP, LIST_BAR};
use crate::ui::tokens::{
    CORNER_WIDTH, Color, DROPDOWN_GAP, DROPDOWN_ROWS, FIELD_PAD, FRAME_MIN, LIST_PAD, LIST_ROW_PAD,
    LIST_WHEEL_ROWS, MULTILINE_PAD, Rgb, Role, TYPE_AHEAD_MS, Theme, WINDOW_MAX, WINDOW_MIN,
    window_b,
};
use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use ::windows::Win32::Globalization::{
    GetLocaleInfoEx, GetUserDefaultUILanguage, LCIDToLocaleName, LOCALE_IREADINGLAYOUT,
    LOCALE_RETURN_NUMBER,
};
use ::windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_CAPTION_COLOR, DWMWA_CLOAK, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use ::windows::Win32::Graphics::Gdi::ClientToScreen;
use ::windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetDC, GetDeviceCaps, GetMonitorInfoW, HDC, InvalidateRect, LOGPIXELSY,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, MonitorFromRect, PAINTSTRUCT,
    ReleaseDC, ScreenToClient,
};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Controls::WM_MOUSELEAVE;
use ::windows::Win32::UI::Input::KeyboardAndMouse::VK_F4;
use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use ::windows::Win32::UI::Shell::{
    ABE_LEFT, ABE_RIGHT, ABM_GETTASKBARPOS, APPBARDATA, SHAppBarMessage,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_DBLCLKS, CS_DROPSHADOW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    EN_CHANGE, EN_KILLFOCUS, EN_SETFOCUS, GWLP_USERDATA, GetClientRect, GetCursorPos,
    GetWindowLongPtrW, GetWindowRect, HTCLIENT, HWND_TOPMOST, IDC_ARROW, IDC_HAND, KillTimer,
    LoadCursorW, MA_NOACTIVATE, MINMAXINFO, PostMessageW, RegisterClassExW,
    SPI_GETCLIENTAREAANIMATION, SW_SHOWNOACTIVATE, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOSIZE,
    SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursor, SetTimer, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, SystemParametersInfoW, WHEEL_DELTA, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_ERASEBKGND,
    WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS,
    WM_SETTINGCHANGE, WM_SIZE, WM_THEMECHANGED, WM_TIMER, WNDCLASSEXW, WS_CAPTION, WS_CLIPCHILDREN,
    WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP,
    WS_SYSMENU, WS_THICKFRAME,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, IsWindow, IsWindowVisible, WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_KILLFOCUS,
    WM_MOVE, WM_SYSKEYDOWN,
};
use ::windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, WM_NULL};
use ::windows::core::{BOOL, HSTRING, PCWSTR, Result as WinResult, w};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

/// What the window keeps: its host, borrowed by one message at a time. A message that arrives while
/// another is being handled (a size sent by `SetWindowPos`, a box the content opens) is answered
/// by Windows, so no two borrows of the host are ever live.
type Slot = RefCell<Box<dyn Handler>>;

/// What a window of the shell shows and does.
pub trait Content {
    /// The identifiers of the window's elements: an enumeration of its own (SEC-05).
    type Id: Copy + PartialEq + std::fmt::Debug + 'static;

    /// The description, built from the content's state now.
    fn describe(&self) -> Window<Self::Id>;
    /// What an element did; answers whether the window stays.
    fn event(&mut self, event: Event<Self::Id>) -> Reply;
    /// The title of the window.
    fn title(&self) -> String;
    /// The language of the program: its locale and whether it reads right to left.
    fn language(&self) -> (String, bool);
    /// The setting «Оформление».
    fn theme(&self) -> ThemeSetting;
    /// What Esc presses; none — Esc closes the window.
    fn escape(&self) -> Option<Self::Id> {
        None
    }
    /// The period of the window's tick, ms (T-120-25): while the window is open, shown and not
    /// minimised, and Windows moves what it shows, the content gets `Event::Tick` this often —
    /// a live example. None — no tick, the rule of «в покое — ноль».
    fn tick(&self) -> Option<u32> {
        None
    }
    /// The period of a timer of the content's own, ms (T-120-20): while the window is open the
    /// content gets `Event::Timer` this often, whatever Windows shows or moves — the pause of a
    /// double Shift, a count of seconds. Read again after every event; None — no timer.
    fn timer(&self) -> Option<u32> {
        None
    }
    /// The window opens at the least size of the passport (620 × 500) rather than at two thirds of
    /// the work area — a small window of a few lines (T-120-21 Б: the details for the author). The
    /// person may still make it larger.
    fn least(&self) -> bool {
        false
    }
}

/// What the content answers to an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Stay,
    Close,
}

/// What a test may read of a live host.
#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    /// The element with the focus, as its identifier prints.
    pub focus: Option<String>,
    /// Frames drawn so far.
    pub frames: u32,
    /// The background of the last frame.
    pub background: Option<Rgb>,
    pub scale: f32,
    /// The factor of the type size the window is laid out at.
    pub k: Option<f32>,
    /// The text fields of Windows in the window, by identifier.
    pub fields: Vec<(String, HWND)>,
    /// What the layout placed: the part, its identifier and its rectangle in client pixels.
    pub items: Vec<(String, Option<String>, Rect)>,
    /// The list of a choice field open under the window.
    pub dropdown: Option<HWND>,
    /// How many times the pictures of the window were drawn afresh.
    #[cfg(feature = "testing")]
    pub pictures_drawn: usize,
    /// How many fonts the fields of the window hold.
    #[cfg(feature = "testing")]
    pub field_fonts: usize,
}

/// The type-free face of a host, kept in the window.
trait Handler {
    fn handle(
        &mut self,
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT>;
    fn probe(&self) -> Probe;
    /// Sizes and places the window just made: an ordinary one on the monitor under the mouse, a
    /// popup in its corner.
    fn settle(&mut self, hwnd: HWND);
    /// Starts the popup's coming in, or puts it in its place at once when `moves` is false.
    fn appear(&mut self, hwnd: HWND, moves: bool);
    /// The description again and a new frame: the list of a choice field moved by the keys of its
    /// window.
    fn refresh(&mut self, hwnd: HWND);
    /// The menu opened from the keyboard: its first available item marked.
    fn mark_first(&mut self, hwnd: HWND);
}

/// Where a popup stands.
#[derive(Clone, Copy)]
enum Place {
    /// In the corner of the work area nearest the icons; the rectangle of the program's icon, when
    /// the caller measured it.
    Corner(Option<RECT>),
    /// Under a choice field (its rectangle on the screen), or over it where the work area ends: its
    /// list, at the type size `b` of the window it opens from, `rows` options in all, `width`
    /// points wide.
    Under {
        field: RECT,
        b: f32,
        rows: usize,
        width: f32,
    },
    /// The menu of the icon at the point `at` (where the icon was pressed), as the menus of Windows
    /// open from a point; `owner` is told when it goes (KB 135788).
    Menu { at: POINT, owner: Option<HWND> },
}

/// What a popup keeps: where it stands and how it comes in.
struct Popup {
    place: Place,
    /// Where the window stands once it has come in, screen pixels.
    at: (i32, i32),
    /// Its corner is on the right of the work area: it comes from the right.
    from_right: bool,
    /// When it began to come in; none once it stands.
    since: Option<Instant>,
}

/// The message the list of a choice field posts to the window it opens from: a row was clicked.
const WM_CHOSEN: u32 = WM_APP + 1;

/// The one element of the list of a choice field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropId {
    List,
}

/// What the list of an open choice field holds — shared by the window it opens from, which takes
/// the keys, and the list's own popup, which takes the mouse.
struct DropState {
    options: Vec<String>,
    /// The value now: its row is ticked.
    current: usize,
    /// The row the keys are on.
    highlight: usize,
    first: usize,
    /// How many rows the popup shows.
    visible: usize,
    /// The row a click chose.
    chosen: Option<usize>,
    /// The first letters typed, and when the last of them came.
    typed: String,
    typed_at: Option<Instant>,
}

impl DropState {
    fn view(&self) -> ListView {
        ListView {
            selected: self.highlight,
            first: self.first,
            visible: self.visible,
            total: self.options.len(),
            checked: None,
        }
    }

    /// The keys on `row`, scrolled into view.
    fn go(&mut self, row: usize) {
        self.highlight = row.min(self.options.len().saturating_sub(1));
        self.first = self.view().first_showing(self.highlight);
    }

    /// A letter typed into the open list: the first row whose option begins with the letters typed
    /// so far — a pause of `TYPE_AHEAD_MS` starts a new word.
    fn type_letter(&mut self, letter: char) {
        let fresh = self
            .typed_at
            .is_none_or(|at| at.elapsed() >= Duration::from_millis(TYPE_AHEAD_MS));
        if fresh {
            self.typed.clear();
        }
        self.typed.extend(letter.to_lowercase());
        self.typed_at = Some(Instant::now());
        if let Some(row) = self
            .options
            .iter()
            .position(|option| option.to_lowercase().starts_with(&self.typed))
        {
            self.go(row);
        }
    }
}

/// The content of the list of a choice field: one drawn list filling its popup.
struct DropList {
    shared: Rc<RefCell<DropState>>,
    owner: HWND,
    locale: String,
    rtl: bool,
    theme: ThemeSetting,
}

impl Content for DropList {
    type Id = DropId;

    fn describe(&self) -> Window<DropId> {
        let state = self.shared.borrow();
        Window {
            kind: WindowKind::Bare,
            art: None,
            brand: None,
            body: Node::List {
                id: DropId::List,
                rows: state
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, label)| ListItem {
                        label: label.clone(),
                        check: None,
                        mark: index == state.current,
                    })
                    .collect(),
                selected: state.highlight,
                first: state.first,
            },
            buttons: Vec::new(),
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<DropId>) -> Reply {
        let mut state = self.shared.borrow_mut();
        match event {
            Event::Selected(_, row) => {
                state.chosen = Some(row);
                // SAFETY: a message to the window the list opened from, of this thread.
                let _ = unsafe { PostMessageW(Some(self.owner), WM_CHOSEN, WPARAM(0), LPARAM(0)) };
                Reply::Close
            }
            Event::Scrolled(_, first) => {
                state.first = first;
                Reply::Stay
            }
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        String::new()
    }

    fn language(&self) -> (String, bool) {
        (self.locale.clone(), self.rtl)
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// An open list of a choice field, as the window it opens from keeps it.
struct OpenDrop<I> {
    id: I,
    hwnd: HWND,
    shared: Rc<RefCell<DropState>>,
}

/// The timers of a window of the shell: the steps of a popup coming in, the frames of a picture.
const APPEAR_TIMER: usize = 1;
const PICTURE_TIMER: usize = 2;
const TICK_TIMER: usize = PICTURE_TIMER + 1;
const CONTENT_TIMER: usize = TICK_TIMER + 1;

/// The timers of ticks alive in the process (T-120-25): none once no window with a tick is open.
static TICK_TIMERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many windows of the shell tick now.
pub fn live_tick_timers() -> usize {
    TICK_TIMERS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The timers of the contents' own alive in the process (T-120-20).
static CONTENT_TIMERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many windows of the shell run a timer of their content now.
pub fn live_content_timers() -> usize {
    CONTENT_TIMERS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The period a window ticks at: what its content asks, while it is shown, not minimised, and
/// Windows moves what it shows; else none (the window shows its last frame).
pub fn tick_period(
    shown: bool,
    minimised: bool,
    animation: bool,
    asked: Option<u32>,
) -> Option<u32> {
    asked.filter(|_| shown && !minimised && animation)
}

/// Where a popup stands, its left-top in screen pixels: in the corner of the work area `work`
/// nearest the notification area `anchor` (the program's icon, or the end of the taskbar where the
/// icons are), `margin` from the edges — by Windows, not «the lower right corner» (passport §7).
pub fn corner(anchor: RECT, work: RECT, size: (i32, i32), margin: i32) -> (i32, i32) {
    let (w, h) = size;
    let right = anchor.left + anchor.right >= work.left + work.right;
    let bottom = anchor.top + anchor.bottom >= work.top + work.bottom;
    let x = if right {
        work.right - margin - w
    } else {
        work.left + margin
    };
    let y = if bottom {
        work.bottom - margin - h
    } else {
        work.top + margin
    };
    (
        x.clamp(work.left, (work.right - w).max(work.left)),
        y.clamp(work.top, (work.bottom - h).max(work.top)),
    )
}

/// Where the list of a choice field stands, its left-top in screen pixels: under the field
/// `field`, `gap` from it, or over it when the work area `work` ends below; inside the work area.
pub fn under(field: RECT, work: RECT, size: (i32, i32), gap: i32) -> (i32, i32) {
    let (w, h) = size;
    let below = field.bottom + gap;
    let y = if below + h <= work.bottom {
        below
    } else {
        field.top - gap - h
    };
    (
        field.left.clamp(work.left, (work.right - w).max(work.left)),
        y.clamp(work.top, (work.bottom - h).max(work.top)),
    )
}

/// The work area of the monitor `rect` stands on.
fn work_of(rect: &RECT) -> RECT {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: the monitor of a rectangle; the out parameter is a local.
    let _ = unsafe { GetMonitorInfoW(MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST), &mut info) };
    info.rcWork
}

/// Where the menu of the icon stands, its left-top in screen pixels: from the point `at` as the
/// menus of Windows open — its corner on the point, turned over to the side the work area `work`
/// has room on, and inside it.
pub fn menu_place(at: POINT, work: RECT, size: (i32, i32)) -> (i32, i32) {
    let (w, h) = size;
    let x = if at.x + w <= work.right {
        at.x
    } else {
        at.x - w
    };
    let y = if at.y + h <= work.bottom {
        at.y
    } else {
        at.y - h
    };
    (
        x.clamp(work.left, (work.right - w).max(work.left)),
        y.clamp(work.top, (work.bottom - h).max(work.top)),
    )
}

/// Opens `content` as the menu of the icon at the point `at` (screen pixels): a popup of the shell
/// that takes the foreground and the keys — as `TrackPopupMenu` asks of the window it opens from
/// (KB 135788: `SetForegroundWindow` before, an empty message to `owner` after). From the keyboard
/// (`keyboard`) its first available item is marked. It goes with Esc, a choice that closes, or when
/// it loses the foreground (a click beside, Win, Alt+Tab).
pub fn open_menu<C: Content + 'static>(
    content: C,
    owner: Option<HWND>,
    at: POINT,
    keyboard: bool,
) -> WinResult<HWND> {
    let popup = Popup {
        place: Place::Menu { at, owner },
        at: (0, 0),
        from_right: true,
        since: None,
    };
    let hwnd = make(content, None, Some(popup))?;
    if keyboard && let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok()) {
        host.mark_first(hwnd);
    }
    cloak(hwnd, true);
    // SAFETY: a live window of this thread; it takes the foreground before it shows (KB 135788).
    unsafe {
        let _ = SetForegroundWindow(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetFocus(Some(hwnd));
    }
    paint_now(hwnd);
    cloak(hwnd, false);
    Ok(hwnd)
}

/// Makes the menu of `content` at the point `at` and places it, without showing it: the tests send
/// it messages with nothing on the screen; `keyboard` marks its first available item.
pub fn create_menu<C: Content + 'static>(
    content: C,
    owner: Option<HWND>,
    at: POINT,
    keyboard: bool,
) -> WinResult<HWND> {
    let popup = Popup {
        place: Place::Menu { at, owner },
        at: (0, 0),
        from_right: true,
        since: None,
    };
    let hwnd = make(content, None, Some(popup))?;
    if keyboard && let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok()) {
        host.mark_first(hwnd);
    }
    Ok(hwnd)
}

/// Closes a window of the shell from outside — the menu of the icon at a second press of the icon.
pub fn close(hwnd: HWND) {
    close_later(hwnd);
}

/// Whether Windows moves what it shows («Показывать анимацию в Windows»).
fn animation_on() -> bool {
    let mut on = BOOL::from(true);
    // SAFETY: the out parameter is a local of the size the action writes.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        )
    };
    read.is_err() || on.as_bool()
}

/// Whether the interface of Windows reads right to left: its icons then stand at the left end of a
/// taskbar along the bottom or the top.
pub fn windows_reads_right_to_left() -> bool {
    // SAFETY: reads the language of the interface of the person.
    let language = u32::from(unsafe { GetUserDefaultUILanguage() });
    // SAFETY: asks the length first, then fills a local of that length.
    let length = unsafe { LCIDToLocaleName(language, None, 0) };
    let mut name = vec![0_u16; usize::try_from(length).unwrap_or(0)];
    // SAFETY: as above.
    if unsafe { LCIDToLocaleName(language, Some(&mut name), 0) } == 0 {
        return false;
    }
    // `LOCALE_RETURN_NUMBER`: the answer is a number in the first two places.
    let mut layout = [0_u16; 2];
    // SAFETY: the name is the terminated string just read; the answer goes into a local.
    let read = unsafe {
        GetLocaleInfoEx(
            PCWSTR(name.as_ptr()),
            LOCALE_IREADINGLAYOUT | LOCALE_RETURN_NUMBER,
            Some(&mut layout),
        )
    };
    read != 0 && layout[0] == 1
}

/// The notification area when the caller did not measure the icon: the end of the taskbar where
/// the icons stand (`SHAppBarMessage`), or — no taskbar — the end of the bottom of the work area
/// under the mouse.
fn notification_area() -> RECT {
    let at_start = windows_reads_right_to_left();
    let mut bar = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };
    // SAFETY: the structure is a local whose size it carries.
    if unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut bar) } != 0 {
        let rc = bar.rc;
        return match bar.uEdge {
            // A taskbar on a side: the icons at its bottom.
            ABE_LEFT | ABE_RIGHT => RECT {
                top: rc.bottom,
                ..rc
            },
            _ if at_start => RECT {
                right: rc.left,
                ..rc
            },
            _ => RECT {
                left: rc.right,
                ..rc
            },
        };
    }
    let mut cursor = POINT::default();
    // SAFETY: the out parameter is a local.
    let _ = unsafe { GetCursorPos(&mut cursor) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: the monitor nearest the mouse; the out parameter is a local.
    let _ = unsafe {
        GetMonitorInfoW(
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST),
            &mut info,
        )
    };
    let work = info.rcWork;
    let x = if at_start { work.left } else { work.right };
    RECT {
        left: x,
        top: work.bottom,
        right: x,
        bottom: work.bottom,
    }
}

struct Host<C: Content> {
    content: C,
    window: Window<C::Id>,
    layout: Option<Layout<C::Id>>,
    canvas: Option<Canvas>,
    text: Option<Text>,
    locale: String,
    rtl: bool,
    theme: Theme,
    scale: f32,
    hover: Option<usize>,
    pressed: Option<usize>,
    focus: Option<C::Id>,
    /// The keyboard ring shows: a key was pressed since the last click.
    cue: bool,
    tracking: bool,
    frames: u32,
    background: Option<Rgb>,
    /// What draws the pictures of the panda: `art.rs`, its pictures kept at their size and scale.
    pictures: Option<Box<dyn crate::ui::canvas::Pictures>>,
    /// The text fields of Windows inside the drawn wrappers.
    fields: Fields<C::Id>,
    /// What the wheel turned short of a whole notch.
    wheel: i32,
    /// How far the body of a small screen is scrolled, in points (208.198); 0 — at its top.
    scroll_y: f32,
    /// The window is a popup: no caption, in the corner, not taking the focus.
    popup: Option<Popup>,
    /// The list of a choice field open under it.
    drop: Option<OpenDrop<C::Id>>,
    /// A picture moves in the window: its timer runs.
    animating: bool,
    /// The period the tick timer runs at, while it runs.
    ticking: Option<u32>,
    /// The period the content's own timer runs at, while it runs.
    timing: Option<u32>,
}

/// The class of the windows of the shell: the ordinary one, or the popup's, which carries the
/// shadow of the system (on Windows 10 the only one there is).
fn class(popup: bool) -> PCWSTR {
    static ORDINARY: OnceLock<u16> = OnceLock::new();
    static POPUP: OnceLock<u16> = OnceLock::new();
    let (atom, name, style) = if popup {
        (
            &POPUP,
            w!("LangSwitcher.Shell.Popup"),
            CS_DBLCLKS | CS_DROPSHADOW,
        )
    } else {
        (&ORDINARY, w!("LangSwitcher.Shell"), CS_DBLCLKS)
    };

    atom.get_or_init(|| {
        // SAFETY: the module of this process; the class is registered once, its name a static.
        let instance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style,
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            // SAFETY: a system cursor.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: name,
            ..Default::default()
        };
        // SAFETY: the structure lives across the call; no background brush: the canvas paints.
        unsafe { RegisterClassExW(&class) }
    });
    name
}

/// Opens a window of the shell for `content`, owned by `owner`, centred on the work area of the
/// monitor under the mouse, at two thirds of it.
pub fn open<C: Content + 'static>(content: C, owner: Option<HWND>) -> WinResult<HWND> {
    let hwnd = create(content, owner)?;
    // Cloaked until the first frame is drawn: the window never shows empty.
    cloak(hwnd, true);
    // SAFETY: a live window of this thread.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };
    paint_now(hwnd);
    cloak(hwnd, false);
    Ok(hwnd)
}

/// Opens a window of the shell as [`open`] does, without taking the foreground: a letter the clock
/// of the program shows (as `show_letter(.., activate = false)` of the old windows).
pub fn open_inactive<C: Content + 'static>(content: C, owner: Option<HWND>) -> WinResult<HWND> {
    let hwnd = create(content, owner)?;
    cloak(hwnd, true);
    // SAFETY: a live window of this thread; shown without taking the focus.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    paint_now(hwnd);
    cloak(hwnd, false);
    Ok(hwnd)
}

/// Makes the window of `content` and places it, without showing it: the tests send it messages
/// with nothing on the screen.
pub fn create<C: Content + 'static>(content: C, owner: Option<HWND>) -> WinResult<HWND> {
    make(content, owner, None)
}

/// Opens `content` as a popup of the shell (T-120-14): no caption and no button on the task bar,
/// above the windows, without taking the focus, in the corner of the work area nearest the icons of
/// the taskbar; it comes in from its side in 0,34 s, or stands at once when Windows' animation is
/// off. `anchor` — the rectangle of the program's icon (`Shell_NotifyIconGetRect`), when the caller
/// has it; else the end of the taskbar where the icons are.
pub fn open_popup<C: Content + 'static>(content: C, anchor: Option<RECT>) -> WinResult<HWND> {
    let hwnd = create_popup(content, anchor)?;
    if let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok()) {
        host.appear(hwnd, animation_on());
    }
    cloak(hwnd, true);
    // SAFETY: a live window of this thread; shown without taking the focus.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    paint_now(hwnd);
    cloak(hwnd, false);
    Ok(hwnd)
}

/// Makes the popup of `content` and places it in its corner, without showing it.
pub fn create_popup<C: Content + 'static>(content: C, anchor: Option<RECT>) -> WinResult<HWND> {
    let popup = Popup {
        place: Place::Corner(anchor),
        at: (0, 0),
        from_right: true,
        since: None,
    };
    make(content, None, Some(popup))
}

fn make<C: Content + 'static>(
    content: C,
    owner: Option<HWND>,
    popup: Option<Popup>,
) -> WinResult<HWND> {
    let title = HSTRING::from(content.title());
    let is_popup = popup.is_some();
    let mut host = Host::new(content);
    host.popup = popup;
    let slot: Box<Slot> = Box::new(RefCell::new(Box::new(host)));
    let raw = Box::into_raw(slot);

    // The fields of Windows are children: the frame of the canvas goes round them.
    // A popup of its own stands above every window; the list of a choice field, above its owner.
    let (extended, style) = if is_popup {
        let above = if owner.is_some() {
            WS_EX_TOOLWINDOW
        } else {
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST
        };
        (above, WS_POPUP | WS_CLIPCHILDREN)
    } else {
        (
            WS_EX_APPWINDOW,
            WS_OVERLAPPED
                | WS_CAPTION
                | WS_SYSMENU
                | WS_MINIMIZEBOX
                | WS_THICKFRAME
                | WS_CLIPCHILDREN,
        )
    };
    // SAFETY: the class is registered; `raw` reaches `WM_NCCREATE`, which keeps it in the window,
    // and `WM_NCDESTROY` frees it. On a refusal no message carried it and it is freed here.
    let made = unsafe {
        CreateWindowExW(
            extended,
            class(is_popup),
            &title,
            style,
            0,
            0,
            0,
            0,
            owner,
            None,
            None,
            Some(raw.cast_const().cast()),
        )
    };
    match made {
        Ok(hwnd) => {
            if is_popup {
                // The roundness of the system (Windows 11); Windows 10 refuses the attribute.
                let round = DWMWCP_ROUND;
                // SAFETY: a documented attribute of DWM with a value of its size, a local.
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE,
                        (&raw const round).cast(),
                        size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                    )
                };
            }
            if let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok()) {
                host.settle(hwnd);
            }
            Ok(hwnd)
        }
        Err(error) => {
            // SAFETY: the box made above, which no window took.
            drop(unsafe { Box::from_raw(raw) });
            Err(error)
        }
    }
}

/// The slot the window keeps, or none before `WM_NCCREATE` and after `WM_NCDESTROY`.
fn slot_of<'a>(hwnd: HWND) -> Option<&'a Slot> {
    // SAFETY: the pointer `WM_NCCREATE` kept, alive until `WM_NCDESTROY` clears it; used on the
    // window's own thread only, within one message.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Slot;
    // SAFETY: as above.
    (!raw.is_null()).then(|| unsafe { &*raw })
}

/// What a test may read of the host of `hwnd`.
pub fn probe(hwnd: HWND) -> Option<Probe> {
    slot_of(hwnd)?.try_borrow().ok().map(|host| host.probe())
}

fn cloak(hwnd: HWND, on: bool) {
    let value = BOOL::from(on);
    // SAFETY: a documented attribute of DWM with a value of its size, a local.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            (&raw const value).cast(),
            size_of::<BOOL>() as u32,
        )
    };
}

/// The scale of the screen the window stands on: the DPI of its DC over a point's.
fn scale_of(hwnd: HWND) -> f32 {
    // SAFETY: the window's DC, released below; the call reads a property of the device.
    let dpi = unsafe {
        let dc = GetDC(Some(hwnd));
        let dpi = GetDeviceCaps(Some(dc), LOGPIXELSY);
        ReleaseDC(Some(hwnd), dc);
        dpi
    };
    if dpi > 0 { dpi as f32 / POINT_DPI } else { 1.0 }
}

/// The frame of the window around its client area, in pixels: what is added to a client size.
fn frame_of(hwnd: HWND) -> (i32, i32) {
    let (mut outer, mut inner) = (RECT::default(), RECT::default());
    // SAFETY: the out parameters are locals.
    let read = unsafe { GetWindowRect(hwnd, &mut outer).and(GetClientRect(hwnd, &mut inner)) };
    if read.is_err() {
        return (0, 0);
    }
    (
        (outer.right - outer.left) - (inner.right - inner.left),
        (outer.bottom - outer.top) - (inner.bottom - inner.top),
    )
}

/// Sizes the window by its monitor's work area — at its least size when the content asks — and
/// centres it there.
fn place(hwnd: HWND, least: bool) {
    let mut cursor = POINT::default();
    // SAFETY: the out parameter is a local.
    let _ = unsafe { GetCursorPos(&mut cursor) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: the monitor nearest the mouse; the out parameter is a local.
    let _ = unsafe {
        GetMonitorInfoW(
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST),
            &mut info,
        )
    };
    let work = info.rcWork;

    let scale = scale_of(hwnd);
    let area = (
        (work.right - work.left) as f32 / scale,
        (work.bottom - work.top) as f32 / scale,
    );
    let size = if least {
        least_window_size(area.0, area.1)
    } else {
        window_size(area.0, area.1)
    };
    let (frame_w, frame_h) = frame_of(hwnd);
    let (w, h) = (
        (size.w * scale).round() as i32 + frame_w,
        (size.h * scale).round() as i32 + frame_h,
    );
    let (x, y) = (
        work.left + (work.right - work.left - w) / 2,
        work.top + (work.bottom - work.top - h) / 2,
    );

    // SAFETY: a live window of this thread; plain values.
    let _ = unsafe { SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE) };
}

/// Draws a frame now, outside `WM_PAINT`: the first one, before the window is uncloaked.
fn paint_now(hwnd: HWND) {
    if let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok()) {
        let _ = host.handle(hwnd, WM_PAINT, WPARAM(0), LPARAM(0));
    }
}

/// Asks the window to close: posted, so the host is not borrowed when the window goes.
fn close_later(hwnd: HWND) {
    // SAFETY: a message to a live window of this thread, with no parameters.
    let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

/// The window procedure of the class: the host of the window answers, or Windows does.
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        // SAFETY: for `WM_NCCREATE` `lparam` is the window's `CREATESTRUCTW`, whose parameter is
        // the slot `create` made.
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        // SAFETY: a live window; the pointer is kept until `WM_NCDESTROY`.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize) };
    }

    match message {
        // Destroyed here, with no borrow of the host live: `WM_DESTROY` and `WM_NCDESTROY` arrive
        // inside this call.
        WM_CLOSE if slot_of(hwnd).is_some() => {
            // SAFETY: a live window of this thread.
            let _ = unsafe { DestroyWindow(hwnd) };
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            // SAFETY: as in `slot_of`; the window forgets the pointer first, then the slot made
            // by `create` is freed once.
            let raw = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut Slot;
            if !raw.is_null() {
                // SAFETY: as above.
                drop(unsafe { Box::from_raw(raw) });
            }
        }
        _ => {
            if let Some(mut host) = slot_of(hwnd).and_then(|slot| slot.try_borrow_mut().ok())
                && let Some(answer) = host.handle(hwnd, message, wparam, lparam)
            {
                return answer;
            }
        }
    }

    // SAFETY: the default answer for everything the host leaves alone.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

impl<C: Content> Host<C> {
    fn new(content: C) -> Self {
        let window = content.describe();
        let (locale, rtl) = content.language();
        Self {
            content,
            window,
            layout: None,
            canvas: None,
            text: None,
            locale,
            rtl,
            theme: Theme::Light,
            scale: 1.0,
            hover: None,
            pressed: None,
            focus: None,
            cue: false,
            tracking: false,
            frames: 0,
            background: None,
            pictures: Some(Box::new(crate::ui::art::Art::new())),
            fields: Fields::default(),
            wheel: 0,
            scroll_y: 0.0,
            popup: None,
            drop: None,
            animating: false,
            ticking: None,
            timing: None,
        }
    }

    /// The popup's size in points: the width of the window in the corner, the height its content
    /// takes (laid out twice: the type size follows the height).
    fn popup_size(&self) -> (f32, f32) {
        // The list of a choice field: as wide as the field, as many rows as it shows, at the type
        // size of its window (half a point to spare: the rows are counted by flooring).
        if let Some(Popup {
            place: Place::Menu { at, .. },
            ..
        }) = &self.popup
            && let Some(text) = self.text.as_ref()
        {
            // As wide as its longest line, between its least width and half the work area.
            let work = work_of(&RECT {
                left: at.x,
                top: at.y,
                right: at.x,
                bottom: at.y,
            });
            let half = (work.right - work.left) as f32 / self.scale / 2.0;
            let w =
                menu_width(&self.window, text).clamp(MENU_MIN_WIDTH.0, half.max(MENU_MIN_WIDTH.0));
            let first = lay_out(
                &self.window,
                client_size(w, WINDOW_MIN.h.0),
                1.0,
                self.rtl,
                text,
            );
            return (w, (WINDOW_MIN.h.0 + first.shortfall.h).max(1.0));
        }
        if let Some(Popup {
            place: Place::Under { b, rows, width, .. },
            ..
        }) = &self.popup
        {
            let style = Style::of(Role::Body, *b, WINDOW_MIN.h.0);
            let row = style.line_height() + LIST_ROW_PAD.v.at(*b) * 2.0;
            let shown = (*rows).clamp(1, DROPDOWN_ROWS.0 as usize) as f32;
            return (*width, LIST_PAD.at(*b) * 2.0 + row * shown + 0.5);
        }
        let w = CORNER_WIDTH.0;
        let mut h = WINDOW_MIN.h.0;
        if let Some(text) = self.text.as_ref() {
            for _ in 0..2 {
                let size = client_size(w, h);
                let k = fit::k_for(&self.locale, size.class);
                let layout = lay_out(&self.window, size, k, self.rtl, text);
                h = (h + layout.shortfall.h).max(1.0);
            }
        }
        (w, h)
    }

    /// Sizes the popup to its content and puts it in its corner — moved by what it still has to
    /// come in.
    fn place_popup(&mut self, hwnd: HWND) {
        let (w, h) = self.popup_size();
        let scale = self.scale;
        let px = |points: f32| (points * scale).round() as i32;
        let Some(popup) = self.popup.as_mut() else {
            return;
        };
        let size = (px(w), px(h));
        if let Place::Menu { at, .. } = popup.place {
            let work = work_of(&RECT {
                left: at.x,
                top: at.y,
                right: at.x,
                bottom: at.y,
            });
            popup.at = menu_place(at, work, size);
            // SAFETY: a live window of this thread; plain values.
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    popup.at.0,
                    popup.at.1,
                    size.0,
                    size.1,
                    SWP_NOACTIVATE,
                )
            };
            self.layout = None;
            self.fit_canvas(hwnd);
            return;
        }
        if let Place::Under { field, .. } = popup.place {
            let work = work_of(&field);
            popup.at = under(field, work, size, px(DROPDOWN_GAP.0));
            // SAFETY: a live window of this thread; plain values.
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    popup.at.0,
                    popup.at.1,
                    size.0,
                    size.1,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            self.layout = None;
            self.fit_canvas(hwnd);
            return;
        }
        let anchor = match popup.place {
            Place::Corner(anchor) => anchor.unwrap_or_else(notification_area),
            Place::Under { field, .. } => field,
            Place::Menu { at, .. } => RECT {
                left: at.x,
                top: at.y,
                right: at.x,
                bottom: at.y,
            },
        };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: the monitor of the anchor; the out parameter is a local.
        let _ = unsafe {
            GetMonitorInfoW(
                MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST),
                &mut info,
            )
        };
        let work = info.rcWork;
        popup.at = corner(anchor, work, size, px(motion::POPUP_MARGIN.0));
        popup.from_right = popup.at.0 * 2 + size.0 >= work.left + work.right;
        let left = popup
            .since
            .map_or(0.0, |since| motion::remaining(elapsed_ms(since), true));
        let x = popup.at.0 + slide_px(left, scale, popup.from_right);
        // SAFETY: a live window of this thread; plain values.
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                popup.at.1,
                size.0,
                size.1,
                SWP_NOACTIVATE,
            )
        };
        self.layout = None;
        self.fit_canvas(hwnd);
    }

    /// The canvas takes the client area now: the size a move of the host's own sends was answered
    /// by Windows while the host was borrowed.
    fn fit_canvas(&mut self, hwnd: HWND) {
        let mut client = RECT::default();
        // SAFETY: the out parameter is a local.
        let _ = unsafe { GetClientRect(hwnd, &mut client) };
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.resize(client.right.max(1) as u32, client.bottom.max(1) as u32);
        }
    }

    /// One step of the popup coming in; the timer stops when it stands.
    fn step_in(&mut self, hwnd: HWND) {
        let scale = self.scale;
        let Some(popup) = self.popup.as_mut() else {
            return;
        };
        let left = popup
            .since
            .map_or(0.0, |since| motion::remaining(elapsed_ms(since), true));
        let x = popup.at.0 + slide_px(left, scale, popup.from_right);
        // SAFETY: a live window of this thread; plain values.
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                popup.at.1,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        if left <= 0.0 {
            popup.since = None;
            // SAFETY: the window's own timer.
            let _ = unsafe { KillTimer(Some(hwnd), APPEAR_TIMER) };
        }
    }

    /// The window is the menu of the icon.
    fn is_menu(&self) -> bool {
        matches!(
            self.popup,
            Some(Popup {
                place: Place::Menu { .. },
                ..
            })
        )
    }

    /// The keys of the menu: the arrows round the available items, Home, End, Enter and the space
    /// bar on the marked one, Esc. None — not a key of the menu.
    fn menu_key(&mut self, hwnd: HWND, key: u16) -> Option<bool> {
        let order = self.order();
        let at = self
            .focus
            .and_then(|focus| order.iter().position(|id| *id == focus));
        let n = order.len();
        let next = match key {
            k if k == VK_DOWN.0 => at.map_or(0, |at| (at + 1) % n.max(1)),
            k if k == VK_UP.0 => {
                at.map_or(n.saturating_sub(1), |at| (at + n.max(1) - 1) % n.max(1))
            }
            k if k == VK_HOME.0 => 0,
            k if k == VK_END.0 => n.saturating_sub(1),
            k if k == VK_RETURN.0 || k == VK_SPACE.0 => {
                if let Some(event) = self
                    .focus
                    .and_then(|focus| self.index_of(focus))
                    .and_then(|index| self.activation(index))
                {
                    self.deliver(hwnd, event);
                }
                return Some(true);
            }
            // Esc asks the content first, as in every window (T-120-15: the menu tells Esc from the
            // loss of the foreground by it).
            k if k == VK_ESCAPE.0 => {
                match self.content.escape() {
                    Some(id) => {
                        self.deliver(hwnd, Event::Pressed(id));
                    }
                    None => close_later(hwnd),
                }
                return Some(true);
            }
            _ => return None,
        };
        if let Some(id) = order.get(next) {
            self.focus = Some(*id);
        }
        invalidate(hwnd);
        Some(true)
    }

    /// A letter in the menu: the next available item whose label begins with it, round.
    fn menu_letter(&mut self, letter: char) {
        let wanted: String = letter.to_lowercase().collect();
        let mut items: Vec<(C::Id, String)> = Vec::new();
        if let Node::Column(parts) = &self.window.body {
            for part in parts {
                if let Node::MenuItem { id, label, .. } = part
                    && !self.window.disabled.contains(id)
                {
                    items.push((*id, label.to_lowercase()));
                }
            }
        }
        let start = self
            .focus
            .and_then(|focus| items.iter().position(|(id, _)| *id == focus))
            .map_or(0, |at| at + 1);
        let n = items.len();
        if let Some((id, _)) = (0..n)
            .map(|step| &items[(start + step) % n])
            .find(|(_, label)| label.starts_with(&wanted))
        {
            self.focus = Some(*id);
            self.cue = true;
        }
    }

    /// The element at `index` is a choice field.
    fn is_choice(&self, index: usize) -> bool {
        self.layout.as_ref().is_some_and(|layout| {
            matches!(
                layout.items.get(index).map(|item| &item.part),
                Some(Part::Field { .. })
            )
        })
    }

    /// The focused element, when it is a choice field.
    fn focused_choice(&self) -> Option<usize> {
        self.focus
            .and_then(|focus| self.index_of(focus))
            .filter(|index| self.is_choice(*index))
    }

    /// Opens the list of the choice field at `index` under it, without taking the focus: the keys
    /// stay here and move its rows.
    fn open_drop(&mut self, hwnd: HWND, index: usize) {
        self.close_drop(hwnd, false);
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let Some(item) = layout.items.get(index) else {
            return;
        };
        let (Part::Field { .. }, Some(id)) = (&item.part, item.id) else {
            return;
        };
        let Some((options, current)) = self.window.options(&id) else {
            return;
        };
        let (options, rect, b) = (options.to_vec(), self.on_screen(index, item.rect), layout.b);
        let scale = self.scale;
        let px = |points: f32| (points * scale).round() as i32;
        let mut ends = [
            POINT {
                x: px(rect.x),
                y: px(rect.y),
            },
            POINT {
                x: px(rect.right()),
                y: px(rect.bottom()),
            },
        ];
        for end in &mut ends {
            // SAFETY: a live window; the point is a local.
            let _ = unsafe { ClientToScreen(hwnd, end) };
        }
        let field = RECT {
            left: ends[0].x,
            top: ends[0].y,
            right: ends[1].x,
            bottom: ends[1].y,
        };
        // As wide as the field, and wider when an option needs it: no option is cut.
        let widest = self.text.as_ref().map_or(0.0, |text| {
            let style = Style::of(Role::Body, b, WINDOW_MIN.h.0);
            options
                .iter()
                .map(|option| text.measure(option, style, Wrap::OneLine, f32::MAX).width)
                .fold(0.0, f32::max)
        });
        let rows = options.len();
        let around = LIST_PAD.at(b) * 2.0
            + LIST_ROW_PAD.h.at(b) * 2.0
            + CHECK.at(b)
            + CONTROL_GAP.at(b)
            + LIST_BAR.0
            + LIST_PAD.at(b);
        let width = (widest + around).max((field.right - field.left) as f32 / scale);
        let mut state = DropState {
            options,
            current,
            highlight: current,
            first: 0,
            visible: rows.clamp(1, DROPDOWN_ROWS.0 as usize),
            chosen: None,
            typed: String::new(),
            typed_at: None,
        };
        state.go(current);
        let shared = Rc::new(RefCell::new(state));
        let list = DropList {
            shared: shared.clone(),
            owner: hwnd,
            locale: self.locale.clone(),
            rtl: self.rtl,
            theme: self.content.theme(),
        };
        let popup = Popup {
            place: Place::Under {
                field,
                b,
                rows,
                width,
            },
            at: (0, 0),
            from_right: true,
            since: None,
        };
        let Ok(drop) = make(list, Some(hwnd), Some(popup)) else {
            return;
        };
        // Shown without taking the focus — under a window on the screen only: a window that is not
        // shown (the tests send it messages) keeps its list hidden too.
        // SAFETY: a handle by value.
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            cloak(drop, true);
            // SAFETY: a live window of this thread.
            let _ = unsafe { ShowWindow(drop, SW_SHOWNOACTIVATE) };
            paint_now(drop);
            cloak(drop, false);
        }
        self.drop = Some(OpenDrop {
            id,
            hwnd: drop,
            shared,
        });
    }

    /// Closes the open list: with the row the keys are on when `choose`, with the row a click
    /// chose, or with nothing; a new value goes to the content as the choice's `Selected`.
    fn close_drop(&mut self, hwnd: HWND, choose: bool) {
        let Some(drop) = self.drop.take() else {
            return;
        };
        // SAFETY: a handle by value.
        if unsafe { IsWindow(Some(drop.hwnd)) }.as_bool() {
            close_later(drop.hwnd);
        }
        let (row, current) = {
            let state = drop.shared.borrow();
            let row = if choose {
                Some(state.chosen.unwrap_or(state.highlight))
            } else {
                state.chosen
            };
            (row, state.current)
        };
        invalidate(hwnd);
        if let Some(row) = row.filter(|row| *row != current) {
            self.deliver(hwnd, Event::Selected(drop.id, row));
        }
    }

    /// Draws the open list again after the keys moved it.
    fn refresh_drop(&self) {
        if let Some(drop) = self.drop.as_ref()
            && let Some(mut list) = slot_of(drop.hwnd).and_then(|slot| slot.try_borrow_mut().ok())
        {
            list.refresh(drop.hwnd);
        }
    }

    /// The keys while a list is open: they move its rows, Enter takes the row, Esc and F4 close it,
    /// Tab closes it and goes on. Answers whether the key was taken.
    fn drop_key(&mut self, hwnd: HWND, key: u16) -> bool {
        let Some(drop) = self.drop.as_ref() else {
            return false;
        };
        let shared = drop.shared.clone();
        let view = shared.borrow().view();
        let last = view.total.saturating_sub(1);
        let page = view.visible.saturating_sub(1).max(1);
        let row = match key {
            k if k == VK_UP.0 => view.selected.saturating_sub(1),
            k if k == VK_DOWN.0 => (view.selected + 1).min(last),
            k if k == VK_HOME.0 => 0,
            k if k == VK_END.0 => last,
            k if k == VK_PRIOR.0 => view.selected.saturating_sub(page),
            k if k == VK_NEXT.0 => (view.selected + page).min(last),
            k if k == VK_RETURN.0 => {
                self.close_drop(hwnd, true);
                return true;
            }
            k if k == VK_ESCAPE.0 || k == VK_F4.0 => {
                self.close_drop(hwnd, false);
                return true;
            }
            k if k == VK_TAB.0 => {
                self.close_drop(hwnd, false);
                return false;
            }
            _ => return true,
        };
        shared.borrow_mut().go(row);
        self.refresh_drop();
        true
    }

    /// The tick timer runs while the content asks for it and the window is shown, not minimised,
    /// with Windows' animation on.
    fn keep_ticking(&mut self, hwnd: HWND) {
        let asked = self.content.tick();
        let wanted = if asked.is_some() {
            // SAFETY: plain questions about a window of this thread.
            let (shown, minimised) =
                unsafe { (IsWindowVisible(hwnd).as_bool(), IsIconic(hwnd).as_bool()) };
            #[allow(unused_mut)]
            let (mut shown, mut animation) = (shown, animation_on());
            // The testing build counts a hidden window as shown and moving, so that its timer can
            // be counted with nothing on the screen.
            #[cfg(feature = "testing")]
            if std::env::var_os("LANGSW_TESTING_UI_TICK_HIDDEN").is_some() {
                (shown, animation) = (true, true);
            }
            tick_period(shown, minimised, animation, asked)
        } else {
            None
        };
        if wanted == self.ticking {
            return;
        }
        // SAFETY: the window's own timer.
        unsafe {
            match (self.ticking, wanted) {
                (_, Some(period)) => {
                    SetTimer(Some(hwnd), TICK_TIMER, period, None);
                }
                (Some(_), None) => {
                    let _ = KillTimer(Some(hwnd), TICK_TIMER);
                }
                (None, None) => {}
            }
        }
        match (self.ticking.is_some(), wanted.is_some()) {
            (false, true) => {
                TICK_TIMERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            (true, false) => {
                TICK_TIMERS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
        self.ticking = wanted;
    }

    /// The content's own timer runs at the period it asks, while the window is open.
    fn keep_timing(&mut self, hwnd: HWND) {
        let wanted = self.content.timer();
        if wanted == self.timing {
            return;
        }
        // SAFETY: the window's own timer.
        unsafe {
            match wanted {
                Some(period) => {
                    SetTimer(Some(hwnd), CONTENT_TIMER, period, None);
                }
                None => {
                    let _ = KillTimer(Some(hwnd), CONTENT_TIMER);
                }
            }
        }
        match (self.timing.is_some(), wanted.is_some()) {
            (false, true) => {
                CONTENT_TIMERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            (true, false) => {
                CONTENT_TIMERS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
        self.timing = wanted;
    }

    /// The timer of the frames runs while a picture that moves is in the window.
    fn keep_picture_moving(&mut self, hwnd: HWND) {
        let moving = self.layout.as_ref().is_some_and(|layout| {
            layout
                .items
                .iter()
                .any(|item| matches!(item.part, Part::Taskbar(_)))
        });
        if moving == self.animating {
            return;
        }
        self.animating = moving;
        // SAFETY: the window's own timer.
        unsafe {
            if moving {
                SetTimer(Some(hwnd), PICTURE_TIMER, motion::PICTURE_FRAME_MS, None);
            } else {
                let _ = KillTimer(Some(hwnd), PICTURE_TIMER);
            }
        }
    }

    /// The theme by the content's setting and the system's.
    fn read_theme(&mut self, hwnd: HWND) {
        let palette = resolve(self.content.theme(), system_is_light());
        let dark = title_bar_is_dark(palette);
        self.theme = if dark { Theme::Dark } else { Theme::Light };

        let flag = BOOL::from(dark);
        let caption = Color::Caption.get(self.theme);
        let colour = u32::from(caption.r)
            | (u32::from(caption.g) << u8::BITS)
            | (u32::from(caption.b) << u16::BITS);
        // SAFETY: documented attributes of DWM with values of their sizes, locals. The colour of
        // the caption is refused by Windows 10, which keeps its own.
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&raw const flag).cast(),
                size_of::<BOOL>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_CAPTION_COLOR,
                (&raw const colour).cast(),
                size_of::<u32>() as u32,
            );
        }
    }

    /// The description again, from the content's state; the text again when the language changed.
    fn rebuild(&mut self) {
        self.window = self.content.describe();
        let (locale, rtl) = self.content.language();
        if locale != self.locale || rtl != self.rtl || self.text.is_none() {
            self.locale = locale;
            self.rtl = rtl;
            let direction = if rtl {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            };
            self.text = Text::new(&self.locale, direction).ok();
        }
        self.layout = None;
    }

    /// The layout for the client area now, made when there is none.
    fn lay_out(&mut self, hwnd: HWND) {
        if self.layout.is_some() {
            return;
        }
        let Some(text) = self.text.as_ref() else {
            return;
        };
        let mut client = RECT::default();
        // SAFETY: the out parameter is a local.
        let _ = unsafe { GetClientRect(hwnd, &mut client) };
        let size = client_size(
            client.right as f32 / self.scale,
            client.bottom as f32 / self.scale,
        );
        // k comes from the table of the language and class (T-120-11); the road of last resort
        // lowers it further only for content that still does not fit here.
        // The list of a choice field takes the type size of the window it opens from.
        let k = match &self.popup {
            Some(Popup {
                place: Place::Under { b, .. },
                ..
            }) => b / window_b(size.w, size.h, 1.0),
            _ => fit::k_for(&self.locale, size.class),
        };
        self.layout = Some(fit::emergency(&self.window, size, k, self.rtl, text));
        self.scroll_y = self.scroll_y.clamp(0.0, self.scroll_most());
        self.sync_fields(hwnd);
    }

    /// How far the body can scroll, in points: 0 where nothing scrolls (208.198).
    fn scroll_most(&self) -> f32 {
        self.layout
            .as_ref()
            .and_then(|layout| layout.scroll)
            .map_or(0.0, |body| (body.content - body.viewport.h).max(0.0))
    }

    /// How far the element at `index` stands from where the layout put it: the elements of the body
    /// of a small screen move by the offset, the head and the buttons do not (208.198).
    fn shift_of(&self, index: usize) -> f32 {
        match self.layout.as_ref().and_then(|layout| layout.scroll) {
            Some(body) if (body.start..body.end).contains(&index) => self.scroll_y,
            _ => 0.0,
        }
    }

    /// The rectangle of the element at `index` as it stands on the screen, in points.
    fn on_screen(&self, index: usize, rect: Rect) -> Rect {
        Rect::new(rect.x, rect.y - self.shift_of(index), rect.w, rect.h)
    }

    /// Whether the point `x`, `y` (points) is where the element at `index` can be seen: an element
    /// of the body only in its room.
    fn seen_at(&self, index: usize, x: f32, y: f32) -> bool {
        match self.layout.as_ref().and_then(|layout| layout.scroll) {
            Some(body) if (body.start..body.end).contains(&index) => {
                let room = body.viewport;
                x >= room.x && y >= room.y && x < room.right() && y < room.bottom()
            }
            _ => true,
        }
    }

    /// Whether `rect` (points, on the screen) lies whole in the room of the body, for the element at
    /// `index`; an element out of the body always does.
    fn room_holds(&self, index: usize, rect: Rect) -> bool {
        match self.layout.as_ref().and_then(|layout| layout.scroll) {
            Some(body) if (body.start..body.end).contains(&index) => {
                let room = body.viewport;
                rect.x >= room.x
                    && rect.y >= room.y
                    && rect.right() <= room.right()
                    && rect.bottom() <= room.bottom()
            }
            _ => true,
        }
    }

    /// Scrolls the body to `y` points from its top, kept within it; answers whether it moved.
    fn scroll_body(&mut self, hwnd: HWND, y: f32) -> bool {
        let y = y.clamp(0.0, self.scroll_most());
        if (y - self.scroll_y).abs() < f32::EPSILON {
            return false;
        }
        self.scroll_y = y;
        self.sync_fields(hwnd);
        invalidate(hwnd);
        true
    }

    /// Brings the element `id` into the room of the body, when it is one of the body's and stands
    /// out of it — Tab to an element the scroll hid (208.198).
    fn reveal(&mut self, hwnd: HWND, id: C::Id) {
        let (Some(index), Some(body)) = (
            self.index_of(id),
            self.layout.as_ref().and_then(|layout| layout.scroll),
        ) else {
            return;
        };
        if !(body.start..body.end).contains(&index) {
            return;
        }
        let Some(rect) = self.layout.as_ref().map(|layout| layout.items[index].rect) else {
            return;
        };
        let (top, bottom) = (rect.y - body.viewport.y, rect.bottom() - body.viewport.y);
        if top < self.scroll_y {
            self.scroll_body(hwnd, top);
        } else if bottom > self.scroll_y + body.viewport.h {
            self.scroll_body(hwnd, bottom - body.viewport.h);
        }
    }

    /// The fields of Windows where the layout put their wrappers, in the font and colours now.
    fn sync_fields(&mut self, hwnd: HWND) {
        let (Some(layout), Some(text)) = (self.layout.as_ref(), self.text.as_ref()) else {
            return;
        };
        let scale = self.scale;
        let px = |points: f32| (points * scale).round() as i32;
        let style = Style::of(Role::Body, layout.b, layout.size.h);
        let code_style = Style::of(Role::Code, layout.b, layout.size.h);
        let look = field::Look {
            face: text.main_face().to_owned(),
            px: px(style.size),
            weight: style.weight,
            code_face: text.mono_face().to_owned(),
            code_px: px(code_style.size),
            code_weight: code_style.weight,
            hint: Color::Ink3.get(self.theme),
        };
        let specs: Vec<Spec<'_, C::Id>> = layout
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let Part::Input {
                    multiline,
                    code,
                    rtl,
                    capture,
                    value,
                    hint,
                } = &item.part
                else {
                    return None;
                };
                let pad = if *multiline { MULTILINE_PAD } else { FIELD_PAD };
                let (pad_h, pad_v) = (pad.h.at(layout.b), pad.v.at(layout.b));
                let rect = self.on_screen(index, item.rect);
                let state = self.state_of(index, item.id);
                // A field of the body out of its room hides behind the edge (208.198).
                let shown = self.room_holds(index, rect);
                let ink = if state.disabled {
                    Color::Ink3
                } else {
                    Color::Ink
                };
                Some(Spec {
                    id: item.id?,
                    inner: RECT {
                        left: px(rect.x + pad_h),
                        top: px(rect.y + pad_v),
                        right: px(rect.right() - pad_h),
                        bottom: px(rect.bottom() - pad_v),
                    },
                    multiline: *multiline,
                    code: *code,
                    rtl: rtl.unwrap_or(self.rtl),
                    capture: *capture,
                    value,
                    hint,
                    disabled: state.disabled,
                    ink: ink.get(self.theme),
                    fill: input_fill(state).get(self.theme),
                    shown,
                })
            })
            .collect();
        self.fields.sync(hwnd, &specs, &look);
    }

    fn state_of(&self, index: usize, id: Option<C::Id>) -> State {
        let disabled = id.is_some_and(|id| self.window.disabled.contains(&id));
        // A field shows the keyboard after a click too: the caret is in it.
        let input = self.layout.as_ref().is_some_and(|layout| {
            matches!(
                layout.items.get(index).map(|item| &item.part),
                Some(Part::Input { .. })
            )
        });
        State {
            hover: self.hover == Some(index),
            pressed: self.pressed == Some(index) && self.hover == Some(index),
            focus: (self.cue || input) && id.is_some() && id == self.focus,
            disabled,
        }
    }

    /// The keyboard where the focus is: in the field of Windows that has it, else in the window.
    fn give_keyboard(&self, hwnd: HWND) {
        let in_field = self.focus.is_some_and(|id| self.fields.focus(id));
        if !in_field && self.fields.has_keyboard() {
            // SAFETY: the window of this thread takes the keyboard back from its field.
            let _ = unsafe { SetFocus(Some(hwnd)) };
        }
    }

    /// The list `id`'s view, when `id` is a list.
    fn list_view(&self, id: C::Id) -> Option<ListView> {
        self.layout
            .as_ref()?
            .items
            .iter()
            .find_map(|item| match &item.part {
                Part::List(view) if item.id == Some(id) => Some(*view),
                _ => None,
            })
    }

    /// The list under `x`, `y` (client pixels).
    fn list_at(&self, x: f32, y: f32) -> Option<(C::Id, ListView)> {
        let (x, y) = (x / self.scale, y / self.scale);
        self.layout
            .as_ref()?
            .items
            .iter()
            .enumerate()
            .find_map(|(index, item)| match (&item.part, item.id) {
                (Part::List(view), Some(id))
                    if self.seen_at(index, x, y)
                        && x >= item.rect.x
                        && y + self.shift_of(index) >= item.rect.y
                        && x < item.rect.right()
                        && y + self.shift_of(index) < item.rect.bottom()
                        && !self.window.disabled.contains(&id) =>
                {
                    Some((id, *view))
                }
                _ => None,
            })
    }

    /// Selects `row` of the list `id` and scrolls it into view.
    fn select_row(&mut self, hwnd: HWND, id: C::Id, view: ListView, row: usize) {
        if row != view.selected && !self.deliver(hwnd, Event::Selected(id, row)) {
            return;
        }
        let first = view.first_showing(row);
        if first != view.first {
            self.deliver(hwnd, Event::Scrolled(id, first));
        }
    }

    /// The keys of a focused list; whether the key was one of them.
    fn list_key(&mut self, hwnd: HWND, id: C::Id, view: ListView, key: u16) -> bool {
        let last = view.total.saturating_sub(1);
        let page = view.visible.saturating_sub(1).max(1);
        let row = match key {
            k if k == VK_UP.0 => view.selected.saturating_sub(1),
            k if k == VK_DOWN.0 => (view.selected + 1).min(last),
            k if k == VK_HOME.0 => 0,
            k if k == VK_END.0 => last,
            k if k == VK_PRIOR.0 => view.selected.saturating_sub(page),
            k if k == VK_NEXT.0 => (view.selected + page).min(last),
            k if k == VK_SPACE.0 => {
                if let Some(on) = view.checked {
                    self.deliver(hwnd, Event::RowChecked(id, view.selected, !on));
                }
                return true;
            }
            _ => return false,
        };
        if view.total > 0 {
            self.select_row(hwnd, id, view, row);
        }
        invalidate(hwnd);
        true
    }

    fn paint(&mut self, hwnd: HWND) {
        self.lay_out(hwnd);
        // The colours of the fields follow the state of their wrappers (the mouse, the keyboard).
        self.sync_fields(hwnd);
        self.follow_capture(hwnd);
        self.keep_picture_moving(hwnd);
        self.keep_ticking(hwnd);
        self.keep_timing(hwnd);
        if self.canvas.is_none() {
            let mut client = RECT::default();
            // SAFETY: the out parameter is a local.
            let _ = unsafe { GetClientRect(hwnd, &mut client) };
            self.canvas = Some(Canvas::for_window(
                hwnd,
                client.right.max(1) as u32,
                client.bottom.max(1) as u32,
                self.scale,
            ));
        }
        let background = Color::Bg.get(self.theme);
        let states: Vec<State> = match &self.layout {
            Some(layout) => layout
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| self.state_of(index, item.id))
                .collect(),
            None => Vec::new(),
        };
        let scroll_y = self.scroll_y;
        let pictures = self.pictures.as_deref_mut();
        let (Some(canvas), Some(layout), Some(text)) = (
            self.canvas.as_mut(),
            self.layout.as_ref(),
            self.text.as_ref(),
        ) else {
            return;
        };
        let look = Look {
            theme: self.theme,
            rtl: self.rtl,
            b: layout.b,
            h: layout.size.h,
            text,
        };
        let body = layout.scroll;
        let in_body =
            |index: usize| body.is_some_and(|body| (body.start..body.end).contains(&index));
        canvas.frame_with(pictures, |painter| {
            painter.clear(background);
            for (index, (item, state)) in layout.items.iter().zip(&states).enumerate() {
                if !in_body(index) {
                    elements::draw(painter, &look, &item.part, item.rect, *state);
                }
            }
            // 208.198: the body of a small screen, moved by the offset and shown in its room only;
            // the bar at its edge, as a list has.
            if let Some(body) = body {
                painter.clip_rect(body.viewport);
                let (items, shown) = (
                    &layout.items[body.start..body.end],
                    &states[body.start..body.end],
                );
                for (item, state) in items.iter().zip(shown) {
                    let r = item.rect;
                    let rect = Rect::new(r.x, r.y - scroll_y, r.w, r.h);
                    elements::draw(painter, &look, &item.part, rect, *state);
                }
                painter.unclip();
                let scroll = Part::Scroll {
                    first: scroll_y.round() as usize,
                    visible: body.viewport.h.round() as usize,
                    total: body.content.round() as usize,
                };
                elements::draw(painter, &look, &scroll, body.bar, State::NORMAL);
            }
        });
        self.frames += 1;
        self.background = Some(background);
    }

    /// The topmost element answering a press at `x`, `y` (client pixels).
    fn hit(&self, x: f32, y: f32) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        let (x, y) = (x / self.scale, y / self.scale);
        layout
            .items
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, item)| {
                let state = self.state_of(index, item.id);
                let area = hit_area(&item.part, self.on_screen(index, item.rect), state)?;
                (self.seen_at(index, x, y)
                    && x >= area.x
                    && y >= area.y
                    && x < area.right()
                    && y < area.bottom())
                .then_some(index)
            })
    }

    /// The identifiers Tab goes through: the description's order, the unavailable left out, and
    /// a field of a key out of its capture (it only shows the key, T-120-20).
    fn order(&self) -> Vec<C::Id> {
        let resting = self.resting_keys();
        self.layout.as_ref().map_or_else(Vec::new, |layout| {
            layout
                .focus_order()
                .into_iter()
                .filter(|id| !self.window.disabled.contains(id) && !resting.contains(id))
                .collect()
        })
    }

    /// The fields of a key out of their capture.
    fn resting_keys(&self) -> Vec<C::Id> {
        self.layout.as_ref().map_or_else(Vec::new, |layout| {
            layout
                .items
                .iter()
                .filter(|item| {
                    matches!(
                        item.part,
                        Part::Input {
                            capture: Some(false),
                            ..
                        }
                    )
                })
                .filter_map(|item| item.id)
                .collect()
        })
    }

    /// The field of a key that captures now, if any.
    fn capturing(&self) -> Option<C::Id> {
        self.layout
            .as_ref()?
            .items
            .iter()
            .find_map(|item| match item.part {
                Part::Input {
                    capture: Some(true),
                    ..
                } => item.id,
                _ => None,
            })
    }

    /// The keyboard follows the capture of a field of a key (T-120-20): a field that captures holds
    /// it; once the capture ends it goes to the element after the field in the order of Tab.
    /// After `sync_fields`, so that a field whose capture has ended reports no loss.
    fn follow_capture(&mut self, hwnd: HWND) {
        if let Some(id) = self.capturing() {
            if self.focus != Some(id) || !self.fields.has_keyboard() {
                self.focus = Some(id);
                self.fields.focus(id);
            }
            return;
        }
        let Some(focus) = self
            .focus
            .filter(|focus| self.resting_keys().contains(focus))
        else {
            return;
        };
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let all = layout.focus_order();
        let order = self.order();
        let after = all.iter().position(|id| *id == focus).unwrap_or(0);
        self.focus = all[after..]
            .iter()
            .chain(&all[..after])
            .find(|id| order.contains(id))
            .copied();
        self.give_keyboard(hwnd);
    }

    /// The event pressing the element at `index` makes.
    fn activation(&self, index: usize) -> Option<Event<C::Id>> {
        let layout = self.layout.as_ref()?;
        let item = layout.items.get(index)?;
        let id = item.id?;
        match &item.part {
            Part::Segment { index, .. } | Part::Section { index, .. } => {
                Some(Event::Selected(id, *index))
            }
            Part::Hit => {
                layout
                    .items
                    .iter()
                    .find_map(|other| match (&other.part, other.id == Some(id)) {
                        (Part::Toggle { on } | Part::Check { on }, true) => {
                            Some(Event::Toggled(id, !on))
                        }
                        (Part::Radio { .. }, true) => Some(Event::Toggled(id, true)),
                        _ => None,
                    })
            }
            Part::ListRow {
                index,
                check: Some(on),
                ..
            } => Some(Event::RowChecked(id, *index, !on)),
            Part::ListRow { index, .. } => Some(Event::Selected(id, *index)),
            // A field and the frame of a list take the focus and press nothing.
            // An item of the menu with a switch turns it; any other is pressed.
            Part::MenuRow => Some(
                layout
                    .items
                    .iter()
                    .find_map(|other| match (&other.part, other.id == Some(id)) {
                        (Part::Toggle { on }, true) => Some(Event::Toggled(id, !on)),
                        _ => None,
                    })
                    .unwrap_or(Event::Pressed(id)),
            ),
            // A field and the frame of a list take the focus and press nothing; a choice field opens
            // its list (`open_drop`).
            Part::Input { .. } | Part::List(_) | Part::Field { .. } => None,
            _ => Some(Event::Pressed(id)),
        }
    }

    /// The events a press of the element at `index` makes: a row with a check box is selected
    /// first, then its box turns.
    fn presses(&self, index: usize) -> Vec<Event<C::Id>> {
        let mut events = Vec::new();
        if let Some(layout) = self.layout.as_ref()
            && let Some(item) = layout.items.get(index)
            && let (
                Part::ListRow {
                    selected: false,
                    index: row,
                    check: Some(_),
                },
                Some(id),
            ) = (&item.part, item.id)
        {
            events.push(Event::Selected(id, *row));
        }
        events.extend(self.activation(index));
        events
    }

    /// The element of `id` a key presses: its first part that answers a press.
    fn index_of(&self, id: C::Id) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        layout.items.iter().position(|item| {
            item.id == Some(id) && hit_area(&item.part, item.rect, State::NORMAL).is_some()
        })
    }

    /// Hands `event` to the content and shows what it made of it; answers whether the window stays.
    fn deliver(&mut self, hwnd: HWND, event: Event<C::Id>) -> bool {
        if self.content.event(event) == Reply::Close {
            close_later(hwnd);
            return false;
        }
        self.keep_timing(hwnd);
        self.rebuild();
        self.read_theme(hwnd);
        if self.popup.is_some() {
            // What the popup shows may take another height.
            self.place_popup(hwnd);
        }
        invalidate(hwnd);
        true
    }

    fn key(&mut self, hwnd: HWND, key: u16) -> bool {
        // A layout dropped by a change waits for the next frame; the keys need it now.
        self.lay_out(hwnd);
        self.cue = true;
        // The menu takes its own keys.
        if self.is_menu()
            && let Some(handled) = self.menu_key(hwnd, key)
        {
            return handled;
        }
        // An open list of a choice field takes the keys first.
        if self.drop.is_some() && self.drop_key(hwnd, key) {
            return true;
        }
        // The space bar and F4 open the list of a focused choice field.
        if (key == VK_SPACE.0 || key == VK_F4.0)
            && let Some(index) = self.focused_choice()
        {
            self.open_drop(hwnd, index);
            return true;
        }
        let order = self.order();
        let at = self
            .focus
            .and_then(|focus| order.iter().position(|id| *id == focus));

        // A focused list takes its own keys: the rows, the pages, the ends, its check boxes.
        if let Some(id) = self.focus
            && let Some(view) = self.list_view(id)
            && self.list_key(hwnd, id, view, key)
        {
            return true;
        }

        match key {
            k if k == VK_TAB.0 => {
                // SAFETY: reads the state of a key for this thread.
                let back = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
                let next = match (at, back, order.len()) {
                    (_, _, 0) => None,
                    (None, false, _) => Some(0),
                    (None, true, n) => Some(n - 1),
                    (Some(at), false, n) => Some((at + 1) % n),
                    (Some(at), true, n) => Some((at + n - 1) % n),
                };
                self.focus = next.map(|next| order[next]);
                if let Some(focus) = self.focus {
                    self.reveal(hwnd, focus);
                }
                self.give_keyboard(hwnd);
            }
            k if [VK_LEFT.0, VK_RIGHT.0, VK_UP.0, VK_DOWN.0].contains(&k) => {
                let forward = (k == VK_RIGHT.0) != self.rtl || k == VK_DOWN.0;
                let forward = if k == VK_UP.0 { false } else { forward };
                if let Some(event) = self.focus.and_then(|focus| self.step(focus, forward)) {
                    self.deliver(hwnd, event);
                    return true;
                }
            }
            k if k == VK_SPACE.0 => {
                if let Some(event) = self
                    .focus
                    .and_then(|focus| self.index_of(focus))
                    .and_then(|index| self.activation(index))
                {
                    self.deliver(hwnd, event);
                    return true;
                }
            }
            k if k == VK_RETURN.0 => {
                let focused_button =
                    self.focus
                        .and_then(|focus| self.index_of(focus))
                        .filter(|index| {
                            self.layout.as_ref().is_some_and(|layout| {
                                matches!(layout.items[*index].part, Part::Button { .. })
                            })
                        });
                let main = || {
                    self.layout.as_ref()?.items.iter().position(|item| {
                        matches!(
                            item.part,
                            Part::Button {
                                kind: ButtonKind::Primary,
                                ..
                            }
                        ) && item
                            .id
                            .is_some_and(|id| !self.window.disabled.contains(&id))
                    })
                };
                if let Some(event) = focused_button
                    .or_else(main)
                    .and_then(|index| self.activation(index))
                {
                    self.deliver(hwnd, event);
                    return true;
                }
            }
            k if k == VK_ESCAPE.0 => {
                match self.content.escape() {
                    Some(id) => {
                        self.deliver(hwnd, Event::Pressed(id));
                    }
                    None => close_later(hwnd),
                }
                return true;
            }
            // 208.198: the body of a small screen — a room up or down, its top, its end.
            k if [VK_PRIOR.0, VK_NEXT.0, VK_HOME.0, VK_END.0].contains(&k)
                && self.scroll_most() > 0.0 =>
            {
                let room = self
                    .layout
                    .as_ref()
                    .and_then(|layout| layout.scroll)
                    .map_or(0.0, |body| body.viewport.h);
                let y = match k {
                    k if k == VK_PRIOR.0 => self.scroll_y - room,
                    k if k == VK_NEXT.0 => self.scroll_y + room,
                    k if k == VK_HOME.0 => 0.0,
                    _ => self.scroll_most(),
                };
                self.scroll_body(hwnd, y);
                return true;
            }
            _ => return false,
        }
        invalidate(hwnd);
        true
    }

    /// The next option of the group `id`, forward or back: a segment or a section.
    fn step(&self, id: C::Id, forward: bool) -> Option<Event<C::Id>> {
        let layout = self.layout.as_ref()?;
        // A closed choice field: the arrows take the next option, as the lists of Windows do.
        if layout
            .items
            .iter()
            .any(|item| item.id == Some(id) && matches!(item.part, Part::Field { .. }))
        {
            let (options, now) = self.window.options(&id)?;
            let next = if forward {
                now + 1
            } else {
                now.checked_sub(1)?
            };
            return (next < options.len()).then_some(Event::Selected(id, next));
        }
        let options: Vec<(usize, bool)> = layout
            .items
            .iter()
            .filter(|item| item.id == Some(id))
            .filter_map(|item| match &item.part {
                Part::Segment {
                    index, selected, ..
                } => Some((*index, *selected)),
                Part::Section { index, current, .. } => Some((*index, *current)),
                _ => None,
            })
            .collect();
        let now = options.iter().position(|(_, on)| *on)?;
        let next = if forward {
            now + 1
        } else {
            now.checked_sub(1)?
        };
        options
            .get(next)
            .map(|(index, _)| Event::Selected(id, *index))
    }
}

/// The milliseconds since `since`.
fn elapsed_ms(since: Instant) -> u32 {
    u32::try_from(since.elapsed().as_millis()).unwrap_or(u32::MAX)
}

/// The share `left` of the slide of a popup in pixels, towards the side it comes from.
fn slide_px(left: f32, scale: f32, from_right: bool) -> i32 {
    let px = (motion::APPEAR_SLIDE.0 * scale * left).round() as i32;
    if from_right { px } else { -px }
}

fn invalidate(hwnd: HWND) {
    // SAFETY: a live window; the whole client area, without erasing.
    let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
}

/// The client coordinates of a mouse message.
fn point_of(lparam: LPARAM) -> (f32, f32) {
    let bits = lparam.0 as u32;
    let x = bits as u16 as i16;
    let y = (bits >> u16::BITS) as u16 as i16;
    (f32::from(x), f32::from(y))
}

impl<C: Content> Handler for Host<C> {
    fn handle(
        &mut self,
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        match message {
            WM_NCCREATE => {
                self.scale = scale_of(hwnd);
                self.rebuild();
                self.read_theme(hwnd);
                None
            }
            WM_ERASEBKGND => Some(LRESULT(1)),
            // The next frame of a picture: the description again, from the content's clock.
            // The tick of the window: an event to its content, then one frame as for any other.
            WM_TIMER if wparam.0 == TICK_TIMER => {
                self.deliver(hwnd, Event::Tick);
                Some(LRESULT(0))
            }
            WM_TIMER if wparam.0 == CONTENT_TIMER => {
                self.deliver(hwnd, Event::Timer);
                Some(LRESULT(0))
            }
            WM_TIMER if wparam.0 == PICTURE_TIMER => {
                self.rebuild();
                invalidate(hwnd);
                Some(LRESULT(0))
            }
            WM_TIMER if wparam.0 == APPEAR_TIMER => {
                self.step_in(hwnd);
                Some(LRESULT(0))
            }
            // A click answers in a popup without taking the focus from the window the person types
            // in (FR-101, as the window in the corner of E119).
            WM_MOUSEACTIVATE if self.popup.is_some() && !self.is_menu() => {
                Some(LRESULT(MA_NOACTIVATE as isize))
            }
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                // SAFETY: a live window; the structure is a local, ended below.
                unsafe { BeginPaint(hwnd, &mut paint) };
                self.paint(hwnd);
                // SAFETY: ends the paint begun above.
                let _ = unsafe { EndPaint(hwnd, &paint) };
                Some(LRESULT(0))
            }
            // The frame is dragged: the pictures stretch what they have (art::Art::hold); let go — they
            // are drawn afresh at the size, once.
            WM_ENTERSIZEMOVE => {
                if let Some(pictures) = self.pictures.as_deref_mut() {
                    pictures.hold(true);
                }
                None
            }
            WM_EXITSIZEMOVE => {
                if let Some(pictures) = self.pictures.as_deref_mut() {
                    pictures.hold(false);
                }
                invalidate(hwnd);
                None
            }
            WM_SIZE => {
                self.close_drop(hwnd, false);
                // Minimised or brought back: the tick stops or goes on.
                self.keep_ticking(hwnd);
                let bits = lparam.0 as u32;
                let (w, h) = (u32::from(bits as u16), bits >> u16::BITS);
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(w.max(1), h.max(1));
                }
                self.layout = None;
                invalidate(hwnd);
                Some(LRESULT(0))
            }
            WM_GETMINMAXINFO if self.popup.is_none() => {
                // SAFETY: for this message `lparam` is the system's `MINMAXINFO`, writable.
                let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
                let (frame_w, frame_h) = frame_of(hwnd);
                let px = |points: f32| (points * self.scale).round() as i32;
                // 208.198: a window of the shell is not dragged under the reference it opens in,
                // 900 × 540; a small window (`Content::least`) keeps its own least, 620 × 500.
                let least = if self.content.least() {
                    WINDOW_MIN
                } else {
                    FRAME_MIN
                };
                info.ptMinTrackSize = POINT {
                    x: px(least.w.0) + frame_w,
                    y: px(least.h.0) + frame_h,
                };
                info.ptMaxTrackSize = POINT {
                    x: px(WINDOW_MAX.w.0) + frame_w,
                    y: px(WINDOW_MAX.h.0) + frame_h,
                };
                Some(LRESULT(0))
            }
            WM_DPICHANGED => {
                self.scale = f32::from(wparam.0 as u16) / POINT_DPI;
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.set_scale(self.scale);
                }
                self.layout = None;
                if self.popup.is_some() {
                    // A popup keeps its corner: sized again, not where the system offers.
                    self.place_popup(hwnd);
                    invalidate(hwnd);
                    return Some(LRESULT(0));
                }
                // SAFETY: for this message `lparam` is the rectangle the system offers.
                let offer = unsafe { *(lparam.0 as *const RECT) };
                // SAFETY: a live window of this thread; plain values.
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        offer.left,
                        offer.top,
                        offer.right - offer.left,
                        offer.bottom - offer.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
                // The size the move sends was answered by Windows (the host was borrowed): the canvas
                // takes the new client area here.
                let mut client = RECT::default();
                // SAFETY: the out parameter is a local.
                let _ = unsafe { GetClientRect(hwnd, &mut client) };
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(client.right.max(1) as u32, client.bottom.max(1) as u32);
                }
                invalidate(hwnd);
                Some(LRESULT(0))
            }
            WM_SETTINGCHANGE | WM_THEMECHANGED => {
                self.rebuild();
                self.read_theme(hwnd);
                invalidate(hwnd);
                None
            }
            WM_MOUSEMOVE => {
                if !self.tracking {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: the structure is a local naming this window.
                    self.tracking = unsafe { TrackMouseEvent(&mut track) }.is_ok();
                }
                let (x, y) = point_of(lparam);
                self.lay_out(hwnd);
                let hover = self.hit(x, y);
                if hover != self.hover {
                    self.hover = hover;
                    invalidate(hwnd);
                }
                // In the menu the mouse moves the mark the keys go on from, as in the menus of Windows.
                if self.is_menu()
                    && let Some(id) = hover.and_then(|index| self.layout.as_ref()?.items[index].id)
                {
                    self.focus = Some(id);
                }
                Some(LRESULT(0))
            }
            WM_MOUSELEAVE => {
                self.tracking = false;
                if self.hover.take().is_some() {
                    invalidate(hwnd);
                }
                Some(LRESULT(0))
            }
            WM_SETCURSOR if (lparam.0 as u32 as u16) == HTCLIENT as u16 => {
                let over_link = self
                    .hover
                    .and_then(|index| self.layout.as_ref()?.items.get(index))
                    .is_some_and(|item| matches!(item.part, Part::Link { .. }));
                if !over_link {
                    return None;
                }
                // SAFETY: a system cursor.
                unsafe { SetCursor(LoadCursorW(None, IDC_HAND).ok()) };
                Some(LRESULT(1))
            }
            // A press anywhere in the window closes an open list, and only that.
            WM_LBUTTONDOWN if self.drop.is_some() => {
                self.close_drop(hwnd, false);
                Some(LRESULT(0))
            }
            WM_LBUTTONDOWN => {
                let (x, y) = point_of(lparam);
                self.cue = false;
                self.lay_out(hwnd);
                self.pressed = self.hit(x, y);
                self.hover = self.pressed;
                // A field of a key out of its capture only shows the key: a press on it is beside.
                let resting = self.resting_keys();
                if let Some(id) = self
                    .pressed
                    .and_then(|index| self.layout.as_ref()?.items[index].id)
                    .filter(|id| !resting.contains(id))
                {
                    self.focus = Some(id);
                    self.give_keyboard(hwnd);
                } else if let Some(id) = self.capturing() {
                    // A press beside the elements: the keyboard stays where it is, and a capture
                    // hears of it as of a loss to none (the old window's «press beside»).
                    self.deliver(hwnd, Event::FocusLost { id, to: None });
                }
                // SAFETY: a live window of this thread.
                unsafe { SetCapture(hwnd) };
                invalidate(hwnd);
                Some(LRESULT(0))
            }
            WM_LBUTTONUP => {
                // SAFETY: releases the capture taken on the press.
                let _ = unsafe { ReleaseCapture() };
                let (x, y) = point_of(lparam);
                self.lay_out(hwnd);
                let released = self.hit(x, y);
                let pressed = self.pressed.take();
                invalidate(hwnd);
                if let Some(index) = pressed.filter(|index| released == Some(*index))
                    && self.is_choice(index)
                {
                    self.open_drop(hwnd, index);
                    return Some(LRESULT(0));
                }
                let events = pressed
                    .filter(|index| released == Some(*index))
                    .map_or_else(Vec::new, |index| self.presses(index));
                for event in events {
                    if !self.deliver(hwnd, event) {
                        break;
                    }
                }
                Some(LRESULT(0))
            }
            WM_MOUSEWHEEL => {
                // The point of the wheel is on the screen.
                let (x, y) = point_of(lparam);
                let mut point = POINT {
                    x: x as i32,
                    y: y as i32,
                };
                // SAFETY: a live window; the point is a local.
                let _ = unsafe { ScreenToClient(hwnd, &mut point) };
                self.lay_out(hwnd);
                let list = self.list_at(point.x as f32, point.y as f32);
                // 208.198: no list under the mouse — the body of a small screen scrolls.
                if list.is_none() && self.scroll_most() <= 0.0 {
                    return None;
                }
                let notch = WHEEL_DELTA as i32;
                self.wheel += i32::from((wparam.0 >> u16::BITS) as u16 as i16);
                let notches = self.wheel / notch;
                self.wheel -= notches * notch;
                if let Some((id, view)) = list {
                    let first = view.scrolled(-(notches as isize) * LIST_WHEEL_ROWS.0 as isize);
                    if first != view.first {
                        self.deliver(hwnd, Event::Scrolled(id, first));
                    }
                } else if let Some(layout) = self.layout.as_ref() {
                    // As many lines of the body a notch as rows of a list.
                    let line = Style::of(Role::Body, layout.b, layout.size.h).line_height();
                    let by = notches as f32 * line * LIST_WHEEL_ROWS.0;
                    self.scroll_body(hwnd, self.scroll_y - by);
                }
                Some(LRESULT(0))
            }
            WM_COMMAND => {
                // What a field of Windows tells its window.
                let control = HWND(lparam.0 as *mut _);
                let id = self.fields.id_of(control)?;
                match (wparam.0 >> u16::BITS) as u16 as u32 {
                    EN_CHANGE => {
                        self.deliver(hwnd, Event::Edited(id, field::text_of(control)));
                    }
                    EN_SETFOCUS => {
                        self.focus = Some(id);
                        invalidate(hwnd);
                    }
                    EN_KILLFOCUS => invalidate(hwnd),
                    _ => {}
                }
                Some(LRESULT(0))
            }
            // A key of a field of a key in its capture: at once, to the content.
            field::WM_FIELD_KEY => {
                let id = self.fields.id_of(HWND(lparam.0 as *mut _))?;
                let vk = wparam.0 as u16;
                let event = if wparam.0 >> u16::BITS == 0 {
                    Event::KeyDown { id, vk }
                } else {
                    Event::KeyUp { id, vk }
                };
                self.deliver(hwnd, event);
                Some(LRESULT(0))
            }
            // The field lost the keyboard in its capture: to this window's element now in focus, to
            // another field, or to none.
            field::WM_FIELD_LEFT => {
                let id = self.fields.id_of(HWND(lparam.0 as *mut _))?;
                let taking = HWND(wparam.0 as *mut _);
                let to = if taking == hwnd {
                    self.focus.filter(|focus| *focus != id)
                } else {
                    self.fields.id_of(taking)
                };
                self.deliver(hwnd, Event::FocusLost { id, to });
                Some(LRESULT(0))
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => self
                .fields
                .colours(HDC(wparam.0 as *mut _), HWND(lparam.0 as *mut _)),
            WM_SETFOCUS => {
                // The window activated again: the keyboard back to its field.
                if let Some(id) = self.focus {
                    self.fields.focus(id);
                }
                None
            }
            WM_KEYDOWN => self.key(hwnd, wparam.0 as u16).then_some(LRESULT(0)),
            // Alt+Down opens the list of a focused choice field.
            WM_SYSKEYDOWN if wparam.0 as u16 == VK_DOWN.0 => {
                self.lay_out(hwnd);
                let index = self.focused_choice()?;
                self.open_drop(hwnd, index);
                Some(LRESULT(0))
            }
            // The first letters typed into an open list.
            WM_CHAR if self.is_menu() => {
                if let Some(letter) =
                    char::from_u32(wparam.0 as u32).filter(|c| c.is_alphanumeric())
                {
                    self.menu_letter(letter);
                    invalidate(hwnd);
                }
                Some(LRESULT(0))
            }
            WM_CHAR if self.drop.is_some() => {
                if let Some(letter) = char::from_u32(wparam.0 as u32).filter(|c| !c.is_control())
                    && let Some(drop) = self.drop.as_ref()
                {
                    drop.shared.borrow_mut().type_letter(letter);
                    self.refresh_drop();
                }
                Some(LRESULT(0))
            }
            WM_CHOSEN => {
                self.close_drop(hwnd, false);
                Some(LRESULT(0))
            }
            // The window leaves the keys, or moves: an open list goes.
            WM_KILLFOCUS | WM_MOVE => {
                self.close_drop(hwnd, false);
                None
            }
            WM_ACTIVATE if wparam.0 as u16 as u32 == WA_INACTIVE => {
                self.close_drop(hwnd, false);
                // The menu does not hang about: it goes with the foreground.
                if self.is_menu() {
                    close_later(hwnd);
                }
                None
            }
            WM_DESTROY => {
                self.close_drop(hwnd, false);
                // No tick outlives its window (in rest — zero).
                if self.ticking.take().is_some() {
                    // SAFETY: the window's own timer.
                    let _ = unsafe { KillTimer(Some(hwnd), TICK_TIMER) };
                    TICK_TIMERS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                }
                if self.timing.take().is_some() {
                    // SAFETY: the window's own timer.
                    let _ = unsafe { KillTimer(Some(hwnd), CONTENT_TIMER) };
                    CONTENT_TIMERS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                }
                // The window the menu opened from is told it went (KB 135788).
                if let Some(Popup {
                    place:
                        Place::Menu {
                            owner: Some(owner), ..
                        },
                    ..
                }) = &self.popup
                {
                    // SAFETY: an empty message to a window of this thread.
                    let _ = unsafe { PostMessageW(Some(*owner), WM_NULL, WPARAM(0), LPARAM(0)) };
                }
                // The fields first, while they are still the window's children.
                self.fields.release();
                // The pictures first: `art.rs` keeps bitmaps with the render target, which holds
                // the factory — gone before the canvas, the factory dies with the canvas.
                self.pictures = None;
                self.canvas = None;
                self.layout = None;
                self.text = None;
                None
            }
            _ => None,
        }
    }

    fn settle(&mut self, hwnd: HWND) {
        if self.popup.is_some() {
            self.place_popup(hwnd);
        } else {
            place(hwnd, self.content.least());
        }
    }

    fn refresh(&mut self, hwnd: HWND) {
        self.rebuild();
        invalidate(hwnd);
    }

    fn mark_first(&mut self, hwnd: HWND) {
        self.lay_out(hwnd);
        self.focus = self.order().first().copied();
        self.cue = true;
    }

    fn appear(&mut self, hwnd: HWND, moves: bool) {
        let Some(popup) = self.popup.as_mut() else {
            return;
        };
        if !moves {
            return;
        }
        popup.since = Some(Instant::now());
        self.step_in(hwnd);
        // SAFETY: the window's own timer, stopped by `step_in` when the window stands.
        let _ = unsafe { SetTimer(Some(hwnd), APPEAR_TIMER, motion::FRAME_MS, None) };
    }

    fn probe(&self) -> Probe {
        Probe {
            focus: self.focus.map(|id| format!("{id:?}")),
            frames: self.frames,
            background: self.background,
            scale: self.scale,
            k: self.layout.as_ref().map(|layout| layout.k),
            fields: self
                .fields
                .all()
                .into_iter()
                .map(|(id, hwnd)| (format!("{id:?}"), hwnd))
                .collect(),
            dropdown: self.drop.as_ref().map(|drop| drop.hwnd),
            #[cfg(feature = "testing")]
            pictures_drawn: self
                .pictures
                .as_deref()
                .map_or(0, |pictures| pictures.drawn_afresh()),
            #[cfg(feature = "testing")]
            field_fonts: self.fields.fonts(),
            items: self
                .layout
                .iter()
                .flat_map(|layout| layout.items.iter().enumerate())
                .map(|(index, item)| {
                    let r = self.on_screen(index, item.rect);
                    let s = self.scale;
                    (
                        format!("{:?}", item.part),
                        item.id.map(|id| format!("{id:?}")),
                        Rect::new(r.x * s, r.y * s, r.w * s, r.h * s),
                    )
                })
                .collect(),
        }
    }
}
