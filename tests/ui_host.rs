//! The host of the interface shell, `src\ui\host.rs` (stage Э120, T-120-7), with nothing on the
//! screen: the window is made and never shown, and the tests send it the messages Windows would —
//! the keys, a change of theme, of scale, the question of its limits.

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{ButtonKind, Event, Node, Window, WindowKind};
use lang_switcher::ui::tokens::{Color, Role, Theme};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_END, VK_ESCAPE, VK_RETURN, VK_RIGHT, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, DispatchMessageW, GetClientRect, GetWindowRect, IsWindow, MINMAXINFO, MSG,
    PM_REMOVE, PeekMessageW, SendMessageW, WM_CLOSE, WM_DPICHANGED, WM_GETMINMAXINFO, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL, WM_PAINT, WM_THEMECHANGED,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Sections,
    Autostart,
    Sound,
    Theme,
    Site,
    Copy,
    Close,
}

/// A window with one of each kind the keys reach; `Sound` is unavailable.
struct Probe {
    events: Rc<RefCell<Vec<Event<Id>>>>,
    theme: Rc<Cell<ThemeSetting>>,
    segment: usize,
    section: usize,
}

impl Content for Probe {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Header {
                    eyebrow: None,
                    title: "Probe".to_owned(),
                    subtitle: None,
                },
                Node::Sections {
                    id: Id::Sections,
                    items: vec!["One".to_owned(), "Two".to_owned(), "Three".to_owned()],
                    current: self.section,
                },
                Node::Toggle {
                    id: Id::Autostart,
                    label: "Autostart".to_owned(),
                    hint: None,
                    on: true,
                    badge: None,
                },
                Node::Check {
                    id: Id::Sound,
                    label: "Sound".to_owned(),
                    on: false,
                    hint: None,
                },
                Node::Segments {
                    id: Id::Theme,
                    label: "Theme".to_owned(),
                    options: vec!["Light".to_owned(), "Dark".to_owned()],
                    selected: self.segment,
                },
                Node::Text(Role::Small, "A line".to_owned()),
                Node::Link {
                    id: Id::Site,
                    words: "The site".to_owned(),
                },
            ]),
            buttons: vec![
                Node::Button {
                    id: Id::Copy,
                    label: "Copy".to_owned(),
                    kind: ButtonKind::Quiet,
                    browser: false,
                },
                Node::Button {
                    id: Id::Close,
                    label: "Close".to_owned(),
                    kind: ButtonKind::Primary,
                    browser: false,
                },
            ],
            disabled: vec![Id::Sound],
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Selected(Id::Theme, index) => self.segment = index,
            Event::Selected(Id::Sections, index) => self.section = index,
            _ => {}
        }
        self.events.borrow_mut().push(event);
        Reply::Stay
    }

    fn title(&self) -> String {
        "Probe".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("en-US".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        self.theme.get()
    }
}

struct Open {
    hwnd: HWND,
    events: Rc<RefCell<Vec<Event<Id>>>>,
    theme: Rc<Cell<ThemeSetting>>,
}

impl Open {
    fn new() -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let theme = Rc::new(Cell::new(ThemeSetting::Light));
        let content = Probe {
            events: events.clone(),
            theme: theme.clone(),
            segment: 0,
            section: 0,
        };
        let hwnd = host::create(content, None).expect("the window of the shell");
        let open = Self {
            hwnd,
            events,
            theme,
        };
        open.send(WM_PAINT, 0, 0);
        open
    }

    fn send(&self, message: u32, wparam: usize, lparam: isize) -> isize {
        // SAFETY: a window of this thread; the message goes straight to its procedure.
        unsafe {
            SendMessageW(
                self.hwnd,
                message,
                Some(WPARAM(wparam)),
                Some(LPARAM(lparam)),
            )
        }
        .0
    }

    fn key(&self, key: u16) {
        self.send(WM_KEYDOWN, usize::from(key), 0);
    }

    fn focus(&self) -> Option<String> {
        host::probe(self.hwnd).expect("a live host").focus
    }

    /// Runs the messages posted to this thread: a close is posted.
    fn pump(&self) {
        let mut message = MSG::default();
        // SAFETY: the message queue of this thread; each message goes to its window.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: a message just taken from the queue.
            unsafe { DispatchMessageW(&message) };
        }
    }

    fn alive(&self) -> bool {
        // SAFETY: a handle by value.
        unsafe { IsWindow(Some(self.hwnd)) }.as_bool()
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        if self.alive() {
            self.send(WM_CLOSE, 0, 0);
        }
    }
}

/// Tab goes through every available element in the order of the description and comes back to the
/// first; the unavailable one is passed over.
#[test]
fn tab_goes_through_every_available_element_in_order_and_comes_back() {
    let open = Open::new();
    let expected = [
        "Sections",
        "Autostart",
        "Theme",
        "Site",
        "Copy",
        "Close",
        "Sections",
    ];

    let seen: Vec<String> = expected
        .iter()
        .map(|_| {
            open.key(VK_TAB.0);
            open.focus().unwrap_or_default()
        })
        .collect();

    assert_eq!(seen, expected, "the order of Tab");
}

#[test]
fn enter_presses_the_main_button_and_escape_closes_the_window() {
    let open = Open::new();
    open.key(VK_RETURN.0);
    assert_eq!(
        open.events.borrow().last(),
        Some(&Event::Pressed(Id::Close))
    );

    open.key(VK_ESCAPE.0);
    open.pump();
    assert!(!open.alive(), "Esc closes the window");
}

#[test]
fn arrows_move_the_segment_and_the_section() {
    let open = Open::new();
    open.key(VK_TAB.0);
    open.key(VK_DOWN.0);
    assert_eq!(
        open.events.borrow().last(),
        Some(&Event::Selected(Id::Sections, 1))
    );

    for _ in 0..10 {
        if open.focus().as_deref() == Some("Theme") {
            break;
        }
        open.key(VK_TAB.0);
    }
    assert_eq!(
        open.focus().as_deref(),
        Some("Theme"),
        "Tab reaches the segments"
    );
    open.key(VK_RIGHT.0);
    assert_eq!(
        open.events.borrow().last(),
        Some(&Event::Selected(Id::Theme, 1))
    );
    open.key(VK_RIGHT.0);
    assert_eq!(open.events.borrow().len(), 2, "no segment past the last");
}

/// A change of the palette draws a frame of the new background in the same window.
#[test]
fn the_window_is_laid_out_at_the_factor_of_the_table() {
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::client_size;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let open = Open::new();
    let mut client = RECT::default();
    // SAFETY: the out parameter is a local.
    unsafe { GetClientRect(open.hwnd, &mut client) }.expect("the client area");
    let probe = host::probe(open.hwnd).expect("a live host");
    let size = client_size(
        client.right as f32 / probe.scale,
        client.bottom as f32 / probe.scale,
    );

    assert_eq!(
        probe.k,
        Some(fit::k_for("en-US", size.class)),
        "k from fit::TABLE, class {:?}",
        size.class
    );
}

#[test]
fn a_change_of_theme_draws_the_new_background_in_the_same_window() {
    let open = Open::new();
    let before = host::probe(open.hwnd).expect("a live host");
    assert_eq!(before.background, Some(Color::Bg.get(Theme::Light)));

    open.theme.set(ThemeSetting::Dark);
    open.send(WM_THEMECHANGED, 0, 0);
    open.send(WM_PAINT, 0, 0);
    let after = host::probe(open.hwnd).expect("the same window, alive");

    assert_eq!(after.background, Some(Color::Bg.get(Theme::Dark)));
    assert!(after.frames > before.frames);
}

/// A scale changed while the program runs reaches the windows opened after it (stand of the wizard,
/// 21.11): the DPI is asked of Windows at each opening, not kept from the start of the process.
#[cfg(feature = "testing")]
#[test]
fn a_window_opened_after_a_change_of_scale_is_at_the_new_scale() {
    let at = |dpi: u32| {
        host::answer_dpi(Some(dpi));
        let open = Open::new();
        let scale = host::probe(open.hwnd).expect("alive").scale;
        let mut client = RECT::default();
        // SAFETY: the out parameter is a local.
        unsafe { GetClientRect(open.hwnd, &mut client) }.expect("the client rectangle");
        (scale, client.right, client.bottom)
    };
    let before = at(96);
    let after = at(144);
    host::answer_dpi(None);

    assert_eq!(
        (before.0, after.0),
        (1.0, 1.5),
        "the scale of a window opened at 96 DPI, then of one opened at 144"
    );
    assert!(
        after.1 > before.1 && after.2 > before.2,
        "the window opened after the change is larger: {before:?} → {after:?}"
    );
}

/// The frame is dragged between 900 × 540 and 1280 × 720 points: at 100 % and twice that at 200 %.
/// 900 × 540 is the least frame of a window of the shell since 208.198 — the reference it opens in;
/// until then it was 620 × 500.
#[test]
fn the_limits_of_the_frame_are_the_passports_at_the_scale_of_the_screen() {
    let open = Open::new();

    for (dpi, scale) in [(96usize, 1), (192, 2)] {
        let mut window = RECT::default();
        // SAFETY: the out parameter is a local.
        unsafe { GetWindowRect(open.hwnd, &mut window) }.expect("the window's rectangle");
        open.send(
            WM_DPICHANGED,
            dpi | (dpi << 16),
            (&raw const window) as isize,
        );
        assert_eq!(host::probe(open.hwnd).expect("alive").scale, scale as f32);

        let (mut outer, mut inner) = (RECT::default(), RECT::default());
        // SAFETY: the out parameters are locals.
        unsafe { GetWindowRect(open.hwnd, &mut outer).and(GetClientRect(open.hwnd, &mut inner)) }
            .expect("rectangles");
        let frame = (
            (outer.right - outer.left) - inner.right,
            (outer.bottom - outer.top) - inner.bottom,
        );

        let mut limits = MINMAXINFO::default();
        open.send(WM_GETMINMAXINFO, 0, (&raw mut limits) as isize);
        let client = |point: POINT| (point.x - frame.0, point.y - frame.1);

        assert_eq!(
            client(limits.ptMinTrackSize),
            (900 * scale, 540 * scale),
            "the least at {dpi} DPI"
        );
        assert_eq!(
            client(limits.ptMaxTrackSize),
            (1280 * scale, 720 * scale),
            "the largest at {dpi} DPI"
        );
    }
}

/// **A content that asks for it opens at the least size** (`Content::least`, T-120-21 Б): the client
/// area is 620 × 500 points at the scale of the screen, not two thirds of the work area.
#[test]
fn a_small_window_opens_at_620_by_500_points() {
    struct Small;

    impl Content for Small {
        type Id = u8;

        fn describe(&self) -> Window<u8> {
            Window {
                kind: WindowKind::Ordinary,
                art: None,
                brand: None,
                body: Node::Text(Role::Body, "Five lines for the author".to_owned()),
                buttons: Vec::new(),
                disabled: Vec::new(),
            }
        }

        fn event(&mut self, _: Event<u8>) -> Reply {
            Reply::Stay
        }

        fn title(&self) -> String {
            "Small".to_owned()
        }

        fn language(&self) -> (String, bool) {
            ("en-US".to_owned(), false)
        }

        fn theme(&self) -> ThemeSetting {
            ThemeSetting::Light
        }

        fn least(&self) -> bool {
            true
        }
    }

    let hwnd = host::create(Small, None).expect("the window of the shell");
    // SAFETY: a window of this thread; the message goes straight to its procedure.
    unsafe { SendMessageW(hwnd, WM_PAINT, None, None) };
    let scale = host::probe(hwnd).expect("alive").scale;
    let mut client = windows::Win32::Foundation::RECT::default();
    // SAFETY: the out parameter is a local.
    unsafe { GetClientRect(hwnd, &mut client) }.expect("the client rectangle");
    // SAFETY: as above.
    unsafe { SendMessageW(hwnd, WM_CLOSE, None, None) };

    let points = (client.right as f32 / scale, client.bottom as f32 / scale);
    assert!(
        (points.0 - 620.0).abs() <= 1.0 && (points.1 - 500.0).abs() <= 1.0,
        "the least window: {points:?} points at {scale}"
    );
}

/// ⭐ **The host holds the pictures while the frame is dragged** (the memory of 2026-11-20): three new
/// sizes between WM_ENTERSIZEMOVE and WM_EXITSIZEMOVE draw the picture of the panel afresh not once;
/// let go, it is drawn afresh once at its size.
#[cfg(feature = "testing")]
#[test]
fn the_host_holds_the_pictures_while_the_frame_is_dragged() {
    use lang_switcher::settings::Language;
    use lang_switcher::ui::windows::about::About;
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
    };

    let hwnd = host::create(
        About {
            language: Language::Ru,
            key: "Pause".to_owned(),
            theme: ThemeSetting::Light,
        },
        None,
    )
    .expect("the window of the shell");
    let send = |message: u32| {
        // SAFETY: a window of this thread; the message goes straight to its procedure.
        unsafe { SendMessageW(hwnd, message, None, None) };
    };
    let size = |w: i32, h: i32| {
        // SAFETY: a window of this thread; plain values.
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        send(WM_PAINT);
    };
    let drawn = || host::probe(hwnd).expect("alive").pictures_drawn;

    size(1100, 700);
    let first = drawn();
    assert!(first >= 1, "the picture of the panel is drawn");

    send(WM_ENTERSIZEMOVE);
    for (w, h) in [(1150, 720), (1200, 740), (1250, 760)] {
        size(w, h);
    }
    assert_eq!(drawn(), first, "held while the frame is dragged");

    send(WM_EXITSIZEMOVE);
    send(WM_PAINT);
    assert_eq!(drawn(), first + 1, "let go: drawn afresh once");
    send(WM_CLOSE);
}

/// A tall content in a small window (208.198): a switch at the top of the body, forty lines, the
/// link of the site at its end, and «Close» in the row of buttons. A small window (`least`) opens at
/// 620 × 500 points — under the least frame, a small screen, so its body scrolls.
struct Tall {
    events: Rc<RefCell<Vec<Event<Id>>>>,
}

impl Content for Tall {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let mut body = vec![
            Node::Header {
                eyebrow: None,
                title: "Tall".to_owned(),
                subtitle: None,
            },
            Node::Toggle {
                id: Id::Autostart,
                label: "Autostart".to_owned(),
                hint: None,
                on: true,
                badge: None,
            },
        ];
        body.extend((0..40).map(|row| Node::Text(Role::Body, format!("Line {row}"))));
        body.push(Node::Link {
            id: Id::Site,
            words: "The site".to_owned(),
        });
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(body),
            buttons: vec![Node::Button {
                id: Id::Close,
                label: "Close".to_owned(),
                kind: ButtonKind::Primary,
                browser: false,
            }],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        self.events.borrow_mut().push(event);
        Reply::Stay
    }

    fn title(&self) -> String {
        "Tall".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("en-US".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Light
    }

    fn least(&self) -> bool {
        true
    }
}

/// The rectangle, in client pixels, of the element `id` as the window shows it now.
fn rect_of(hwnd: HWND, id: &str) -> lang_switcher::ui::canvas::Rect {
    // SAFETY: a window of this thread; a frame lays it out.
    unsafe { SendMessageW(hwnd, WM_PAINT, None, None) };
    host::probe(hwnd)
        .expect("a live host")
        .items
        .into_iter()
        .find(|(_, item, _)| item.as_deref() == Some(id))
        .map(|(_, _, rect)| rect)
        .unwrap_or_else(|| panic!("the element {id}"))
}

/// A press of the mouse at the middle of `rect`, client pixels.
fn click_at(hwnd: HWND, rect: lang_switcher::ui::canvas::Rect) {
    let (x, y) = (
        (rect.x + rect.w / 2.0) as i32,
        (rect.y + rect.h / 2.0) as i32,
    );
    let point = LPARAM((((y & 0xFFFF) << 16) | (x & 0xFFFF)) as isize);
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
        // SAFETY: a press to a window of this thread.
        unsafe { SendMessageW(hwnd, message, Some(WPARAM(1)), Some(point)) };
    }
}

fn open_tall() -> (HWND, Rc<RefCell<Vec<Event<Id>>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let hwnd = host::create(
        Tall {
            events: events.clone(),
        },
        None,
    )
    .expect("the window of the shell");
    (hwnd, events)
}

/// **208.198 — the mouse finds the body where it shows.** The link at the end of a tall body lies
/// under the room at first; End scrolls the body to its end, and a click at where the link now shows
/// presses it. The wheel moves the body too.
///
/// ⚠ Control: before End the link is under the row of buttons — out of the room.
#[test]
fn a_click_presses_the_element_of_a_scrolled_body_where_it_shows() {
    let (hwnd, events) = open_tall();
    let close = rect_of(hwnd, "Close");
    let before = rect_of(hwnd, "Site");
    assert!(
        before.y >= close.y,
        "the control: the link is out of the room at first: {before:?}, the button {close:?}"
    );

    // One notch of the wheel down moves the body up.
    let mut origin = POINT::default();
    // SAFETY: a window of this thread; the point is a local.
    let _ = unsafe { ClientToScreen(hwnd, &mut origin) };
    let point = LPARAM((((origin.y + 100) << 16) | ((origin.x + 100) & 0xFFFF)) as isize);
    // SAFETY: as above; WHEEL_DELTA down in the high word.
    unsafe {
        SendMessageW(
            hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM((-120i16 as u16 as usize) << 16)),
            Some(point),
        )
    };
    let wheeled = rect_of(hwnd, "Site");

    // SAFETY: a key to a window of this thread.
    unsafe { SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(usize::from(VK_END.0))), None) };
    let after = rect_of(hwnd, "Site");
    click_at(hwnd, after);
    let pressed = events.borrow().contains(&Event::Pressed(Id::Site));
    println!("before {before:?}; wheeled {wheeled:?}; after End {after:?}; pressed {pressed}");
    // SAFETY: a window of this thread.
    let _ = unsafe { DestroyWindow(hwnd) };

    assert!(wheeled.y < before.y, "the wheel moves the body up");
    assert!(
        after.bottom() <= close.y,
        "End brings the end of the body into the room"
    );
    assert!(pressed, "the click at where the link shows presses it");
}

/// **208.198 — Tab brings a hidden element into view.** Tab goes to the switch at the top and then to
/// the link at the end of the body, under the room: the body scrolls until the link shows.
#[test]
fn tab_to_an_element_the_scroll_hid_brings_it_into_view() {
    let (hwnd, _events) = open_tall();
    let close = rect_of(hwnd, "Close");
    let hidden = rect_of(hwnd, "Site");
    for _ in 0..2 {
        // SAFETY: a key to a window of this thread.
        unsafe { SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(usize::from(VK_TAB.0))), None) };
    }
    let shown = rect_of(hwnd, "Site");
    let focus = host::probe(hwnd).expect("a live host").focus;
    println!("hidden {hidden:?}; after Tab, Tab {shown:?}; focus {focus:?}");
    // SAFETY: a window of this thread.
    let _ = unsafe { DestroyWindow(hwnd) };

    assert_eq!(focus.as_deref(), Some("Site"), "Tab, Tab reaches the link");
    assert!(
        hidden.y >= close.y,
        "the control: the link was out of the room"
    );
    assert!(
        shown.y >= 0.0 && shown.bottom() <= close.y,
        "the link shows in the room: {shown:?}, the button {close:?}"
    );
}
