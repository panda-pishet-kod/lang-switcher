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
//! * **Live:** a change of the theme of Windows or of the content's own setting, of the scale
//!   (`WM_DPICHANGED`, the rectangle the system offers) and of the language lays out and draws
//!   again in the same window.
//! * No movement in the large windows.
//!
//! FR-72: the host sends no message to another window at all.
//!
//! Knows nothing of what a window contains.

use crate::theme::{ThemeSetting, resolve, system_is_light, title_bar_is_dark};
use crate::ui::canvas::{Canvas, POINT_DPI};
use crate::ui::elements::{self, Look, State, hit_area};
use crate::ui::fit;
use crate::ui::layout::{Layout, Part, client_size, window_size};
use crate::ui::scene::{ButtonKind, Event, Window};
use crate::ui::text::{Direction, Text};
use crate::ui::tokens::{Color, Rgb, Theme, WINDOW_MAX, WINDOW_MIN};
use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use ::windows::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_CLOAK, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use ::windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetDC, GetDeviceCaps, GetMonitorInfoW, InvalidateRect, LOGPIXELSY,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, PAINTSTRUCT, ReleaseDC,
};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Controls::WM_MOUSELEAVE;
use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_DOWN,
    VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA,
    GetClientRect, GetCursorPos, GetWindowLongPtrW, GetWindowRect, HTCLIENT, IDC_ARROW, IDC_HAND,
    LoadCursorW, MINMAXINFO, PostMessageW, RegisterClassExW, SW_SHOWNORMAL, SWP_NOACTIVATE,
    SWP_NOZORDER, SetCursor, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE, WM_DESTROY,
    WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE,
    WM_THEMECHANGED, WNDCLASSEXW, WS_CAPTION, WS_EX_APPWINDOW, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_THICKFRAME,
};
use ::windows::core::{BOOL, HSTRING, PCWSTR, Result as WinResult, w};
use std::cell::RefCell;
use std::sync::OnceLock;

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
}

/// The class of every window of the shell.
fn class() -> PCWSTR {
    static ATOM: OnceLock<u16> = OnceLock::new();
    let name = w!("LangSwitcher.Shell");

    ATOM.get_or_init(|| {
        // SAFETY: the module of this process; the class is registered once, its name a static.
        let instance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
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

/// Makes the window of `content` and places it, without showing it: the tests send it messages
/// with nothing on the screen.
pub fn create<C: Content + 'static>(content: C, owner: Option<HWND>) -> WinResult<HWND> {
    let title = HSTRING::from(content.title());
    let slot: Box<Slot> = Box::new(RefCell::new(Box::new(Host::new(content))));
    let raw = Box::into_raw(slot);

    // SAFETY: the class is registered; `raw` reaches `WM_NCCREATE`, which keeps it in the window,
    // and `WM_NCDESTROY` frees it. On a refusal no message carried it and it is freed here.
    let made = unsafe {
        CreateWindowExW(
            WS_EX_APPWINDOW,
            class(),
            &title,
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_THICKFRAME,
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
            place(hwnd);
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

/// Sizes the window by its monitor's work area and centres it there.
fn place(hwnd: HWND) {
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
    let size = window_size(area.0, area.1);
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
        let k = fit::k_for(&self.locale, size.class);
        self.layout = Some(fit::emergency(&self.window, size, k, self.rtl, text));
    }

    fn state_of(&self, index: usize, id: Option<C::Id>) -> State {
        let disabled = id.is_some_and(|id| self.window.disabled.contains(&id));
        State {
            hover: self.hover == Some(index),
            pressed: self.pressed == Some(index) && self.hover == Some(index),
            focus: self.cue && id.is_some() && id == self.focus,
            disabled,
        }
    }

    fn paint(&mut self, hwnd: HWND) {
        self.lay_out(hwnd);
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
        canvas.frame_with(pictures, |painter| {
            painter.clear(background);
            for (item, state) in layout.items.iter().zip(&states) {
                elements::draw(painter, &look, &item.part, item.rect, *state);
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
                let area = hit_area(&item.part, item.rect, state)?;
                (x >= area.x && y >= area.y && x < area.right() && y < area.bottom())
                    .then_some(index)
            })
    }

    /// The identifiers Tab goes through: the description's order, the unavailable left out.
    fn order(&self) -> Vec<C::Id> {
        self.layout.as_ref().map_or_else(Vec::new, |layout| {
            layout
                .focus_order()
                .into_iter()
                .filter(|id| !self.window.disabled.contains(id))
                .collect()
        })
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
            _ => Some(Event::Pressed(id)),
        }
    }

    /// The element of `id` a key presses: its first part that answers a press.
    fn index_of(&self, id: C::Id) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        layout.items.iter().position(|item| {
            item.id == Some(id) && hit_area(&item.part, item.rect, State::NORMAL).is_some()
        })
    }

    /// Hands `event` to the content and shows what it made of it.
    fn deliver(&mut self, hwnd: HWND, event: Event<C::Id>) {
        if self.content.event(event) == Reply::Close {
            close_later(hwnd);
            return;
        }
        self.rebuild();
        self.read_theme(hwnd);
        invalidate(hwnd);
    }

    fn key(&mut self, hwnd: HWND, key: u16) -> bool {
        // A layout dropped by a change waits for the next frame; the keys need it now.
        self.lay_out(hwnd);
        self.cue = true;
        let order = self.order();
        let at = self
            .focus
            .and_then(|focus| order.iter().position(|id| *id == focus));

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
                    Some(id) => self.deliver(hwnd, Event::Pressed(id)),
                    None => close_later(hwnd),
                }
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
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                // SAFETY: a live window; the structure is a local, ended below.
                unsafe { BeginPaint(hwnd, &mut paint) };
                self.paint(hwnd);
                // SAFETY: ends the paint begun above.
                let _ = unsafe { EndPaint(hwnd, &paint) };
                Some(LRESULT(0))
            }
            WM_SIZE => {
                let bits = lparam.0 as u32;
                let (w, h) = (u32::from(bits as u16), bits >> u16::BITS);
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(w.max(1), h.max(1));
                }
                self.layout = None;
                invalidate(hwnd);
                Some(LRESULT(0))
            }
            WM_GETMINMAXINFO => {
                // SAFETY: for this message `lparam` is the system's `MINMAXINFO`, writable.
                let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
                let (frame_w, frame_h) = frame_of(hwnd);
                let px = |points: f32| (points * self.scale).round() as i32;
                info.ptMinTrackSize = POINT {
                    x: px(WINDOW_MIN.w.0) + frame_w,
                    y: px(WINDOW_MIN.h.0) + frame_h,
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
            WM_LBUTTONDOWN => {
                let (x, y) = point_of(lparam);
                self.cue = false;
                self.lay_out(hwnd);
                self.pressed = self.hit(x, y);
                self.hover = self.pressed;
                if let Some(id) = self
                    .pressed
                    .and_then(|index| self.layout.as_ref()?.items[index].id)
                {
                    self.focus = Some(id);
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
                if let Some(event) = pressed
                    .filter(|index| released == Some(*index))
                    .and_then(|index| self.activation(index))
                {
                    self.deliver(hwnd, event);
                }
                Some(LRESULT(0))
            }
            WM_KEYDOWN => self.key(hwnd, wparam.0 as u16).then_some(LRESULT(0)),
            WM_DESTROY => {
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

    fn probe(&self) -> Probe {
        Probe {
            focus: self.focus.map(|id| format!("{id:?}")),
            frames: self.frames,
            background: self.background,
            scale: self.scale,
            k: self.layout.as_ref().map(|layout| layout.k),
        }
    }
}
