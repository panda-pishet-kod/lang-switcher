//! The popup of the shell (stage Э120, T-120-14 пп. 1 и 5): its place by Windows — the corner of
//! the work area nearest the icons of the taskbar, on rectangles, with nothing on the screen; its
//! coming in and Windows' animation switched off; its kind of window; the keyboard. The window is
//! made and never shown.

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{ButtonKind, Event, Node, Window, WindowKind};
use lang_switcher::ui::tokens::motion::{self, APPEAR_MS};
use lang_switcher::ui::tokens::{CORNER_WIDTH, Role};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, GetWindowRect, IsWindow,
    MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WM_CLOSE, WM_KEYDOWN,
    WM_MOUSEACTIVATE, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const W: i32 = 364;
const H: i32 = 180;
const MARGIN: i32 = 12;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

/// A screen of 1920 × 1080; the taskbar 48 high or 62 wide.
#[test]
fn the_corner_is_the_one_nearest_the_icons_for_every_place_of_the_taskbar() {
    let cases = [
        (
            "taskbar at the bottom, icons on the right",
            rect(1700, 1032, 1920, 1080),
            rect(0, 0, 1920, 1032),
            (1920 - MARGIN - W, 1032 - MARGIN - H),
        ),
        (
            "taskbar at the top",
            rect(1700, 0, 1920, 48),
            rect(0, 48, 1920, 1080),
            (1920 - MARGIN - W, 48 + MARGIN),
        ),
        (
            "taskbar on the left",
            rect(0, 900, 62, 1080),
            rect(62, 0, 1920, 1080),
            (62 + MARGIN, 1080 - MARGIN - H),
        ),
        (
            "taskbar on the right",
            rect(1858, 900, 1920, 1080),
            rect(0, 0, 1858, 1080),
            (1858 - MARGIN - W, 1080 - MARGIN - H),
        ),
        (
            "taskbar at the bottom, icons on the left (Windows in Hebrew)",
            rect(0, 1032, 220, 1080),
            rect(0, 0, 1920, 1032),
            (MARGIN, 1032 - MARGIN - H),
        ),
        (
            "a second monitor to the left of the first, icons on the right",
            rect(-220, 1032, 0, 1080),
            rect(-1920, 0, 0, 1032),
            (-MARGIN - W, 1032 - MARGIN - H),
        ),
    ];
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(name, anchor, work, expected)| {
            let got = host::corner(*anchor, *work, (W, H), MARGIN);
            (got != *expected).then(|| format!("{name}: {got:?}, expected {expected:?}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "the place is wrong:\n{}",
        wrong.join("\n")
    );
}

/// A window larger than the work area stays inside it.
#[test]
fn the_popup_stays_inside_the_work_area() {
    let work = rect(0, 0, 300, 200);
    let (x, y) = host::corner(rect(280, 200, 300, 240), work, (W, H), MARGIN);
    assert_eq!((x, y), (0, 200 - MARGIN - H));
}

/// The coming in: the whole slide at the start, less and less, none from 0,34 s; with Windows'
/// animation off — none at all.
#[test]
fn the_popup_comes_in_and_stands_at_once_without_animation() {
    assert_eq!(motion::remaining(0, false), 0.0);
    assert_eq!(motion::remaining(APPEAR_MS / 2, false), 0.0);

    assert!((motion::remaining(0, true) - 1.0).abs() < 1e-3);
    let mut before = 1.0_f32;
    for ms in (0..APPEAR_MS).step_by(10) {
        let now = motion::remaining(ms, true);
        assert!(now <= before + 1e-4, "the slide went back at {ms} ms");
        before = now;
    }
    assert_eq!(motion::remaining(APPEAR_MS, true), 0.0);
    // The curve of the mock-up comes most of the way early: past half of it by a quarter of time.
    assert!(motion::remaining(APPEAR_MS / 4, true) < 0.5);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Open,
    Close,
}

struct Notice;

impl Content for Notice {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Line(Role::Eyebrow, "Lang Switcher".to_owned()),
                Node::Text(Role::Body, "Updated to 1.8.0".to_owned()),
            ]),
            buttons: vec![
                Node::Button {
                    id: Id::Close,
                    label: "Close".to_owned(),
                    kind: ButtonKind::Normal,
                    browser: false,
                },
                Node::Button {
                    id: Id::Open,
                    label: "What is new".to_owned(),
                    kind: ButtonKind::Primary,
                    browser: false,
                },
            ],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Pressed(Id::Close) => Reply::Close,
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        "Notice".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("en-US".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Light
    }
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: a window of this thread; the message goes straight to its procedure.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))) }.0
}

fn pump() {
    let mut message = MSG::default();
    // SAFETY: the message queue of this thread; each message goes to its window.
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: a message just taken from the queue.
        unsafe { DispatchMessageW(&message) };
    }
}

fn alive(hwnd: HWND) -> bool {
    // SAFETY: a handle by value.
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

fn work_of(anchor: RECT) -> RECT {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: the monitor of a rectangle; the out parameter is a local.
    let _ = unsafe {
        GetMonitorInfoW(
            MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST),
            &mut info,
        )
    };
    info.rcWork
}

/// The popup is a window with no caption and no button on the task bar, above the others; it
/// stands in the corner of the anchor's work area — on the left for icons on the left — at the
/// width of the window in the corner; a click does not take the focus; Esc closes it.
#[test]
fn the_popup_is_a_window_in_the_corner_that_takes_no_focus() {
    // An anchor at the bottom left of the monitor of the point (0, 0): the icons on the left.
    let primary = work_of(rect(0, 0, 1, 1));
    let anchor = rect(
        primary.left,
        primary.bottom,
        primary.left + 1,
        primary.bottom + 1,
    );
    let hwnd = host::create_popup(Notice, Some(anchor)).expect("the popup");
    let scale = host::probe(hwnd).expect("a live host").scale;

    // SAFETY: a window of this thread.
    let (style, extended) = unsafe {
        (
            GetWindowLongPtrW(hwnd, GWL_STYLE) as u32,
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32,
        )
    };
    assert_ne!(style & WS_POPUP.0, 0, "not a popup");
    assert_eq!(style & WS_CAPTION.0, 0, "a popup with a caption");
    assert_ne!(
        extended & WS_EX_TOOLWINDOW.0,
        0,
        "a popup with a button on the task bar"
    );
    assert_eq!(
        extended & WS_EX_APPWINDOW.0,
        0,
        "a popup with a button on the task bar"
    );
    assert_ne!(extended & WS_EX_TOPMOST.0, 0, "a popup under other windows");

    let mut at = RECT::default();
    // SAFETY: the out parameter is a local.
    let _ = unsafe { GetWindowRect(hwnd, &mut at) };
    let margin = (motion::POPUP_MARGIN.0 * scale).round() as i32;
    let width = (CORNER_WIDTH.0 * scale).round() as i32;
    assert_eq!(
        at.right - at.left,
        width,
        "the width of the window in the corner"
    );
    assert_eq!(at.left, primary.left + margin, "the icons are on the left");
    assert_eq!(at.bottom, primary.bottom - margin, "the taskbar is below");

    assert_eq!(
        send(hwnd, WM_MOUSEACTIVATE, 0, 0),
        MA_NOACTIVATE as isize,
        "a click takes the focus"
    );

    send(hwnd, WM_KEYDOWN, usize::from(VK_ESCAPE.0), 0);
    pump();
    let closed = !alive(hwnd);
    if !closed {
        send(hwnd, WM_CLOSE, 0, 0);
    }
    assert!(closed, "Esc did not close the popup");
}
