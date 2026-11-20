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
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_ESCAPE, VK_RETURN, VK_RIGHT, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetClientRect, GetWindowRect, IsWindow, MINMAXINFO, MSG, PM_REMOVE,
    PeekMessageW, SendMessageW, WM_CLOSE, WM_DPICHANGED, WM_GETMINMAXINFO, WM_KEYDOWN, WM_PAINT,
    WM_THEMECHANGED,
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

/// The frame is dragged between 620 × 500 and 1280 × 720 points: at 100 % and twice that at 200 %.
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
            (620 * scale, 500 * scale),
            "the least at {dpi} DPI"
        );
        assert_eq!(
            client(limits.ptMaxTrackSize),
            (1280 * scale, 720 * scale),
            "the largest at {dpi} DPI"
        );
    }
}
