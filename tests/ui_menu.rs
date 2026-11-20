//! The menu of the icon on the shell (stage Э120, the core of T-120-15): its place from the point
//! of the icon, its layout — the head, the items at least 36 high, the lines, the ends of the
//! items, the mirror — and its keys. Nothing is on the screen: the menu is made and never shown.

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::layout::{Part, client_size, lay_out};
use lang_switcher::ui::scene::{Event, MenuState, Node, Trailing, Window, WindowKind};
use lang_switcher::ui::text::FixedAdvance;
use lang_switcher::ui::tokens::{MENU_ROW_MIN_HEIGHT, Role};
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_END, VK_ESCAPE, VK_RETURN, VK_SPACE, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, IsWindow, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WA_INACTIVE,
    WM_ACTIVATE, WM_CHAR, WM_CLOSE, WM_KEYDOWN, WM_PAINT,
};

const MEASURE: FixedAdvance = FixedAdvance(0.5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Pause,
    Settings,
    Feed,
    Letters,
    Exit,
}

fn item(id: Id, icon: &str, label: &str, trailing: Trailing, emphasis: bool) -> Node<Id> {
    Node::MenuItem {
        id,
        icon: icon.to_owned(),
        label: label.to_owned(),
        trailing,
        emphasis,
    }
}

/// A menu: the head; pause with its key; settings, unavailable; a line; the feed with a switch; an
/// unread letter; exit.
fn menu(feed: bool) -> Window<Id> {
    Window {
        kind: WindowKind::Menu,
        art: None,
        brand: None,
        body: Node::Column(vec![
            Node::MenuHead {
                state: MenuState::Working,
                words: "работает".to_owned(),
            },
            item(
                Id::Pause,
                "pause",
                "Пауза",
                Trailing::Key("Pause".to_owned()),
                false,
            ),
            item(Id::Settings, "gear", "Настройки", Trailing::None, false),
            Node::MenuRule,
            item(Id::Feed, "star", "Лента", Trailing::Switch(feed), false),
            item(Id::Letters, "mail", "Письмо от автора", Trailing::Dot, true),
            Node::MenuRule,
            item(Id::Exit, "exit", "Выход", Trailing::None, false),
        ]),
        buttons: Vec::new(),
        disabled: vec![Id::Settings],
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

/// From the point of the icon, as the menus of Windows: over a taskbar below, under one at the top,
/// beside one on a side; always inside the work area.
#[test]
fn the_menu_opens_from_the_point_of_the_icon_inside_the_work_area() {
    let size = (320, 300);
    let cases = [
        (
            "taskbar at the bottom",
            POINT { x: 1800, y: 1060 },
            rect(0, 0, 1920, 1032),
            (1480, 732),
        ),
        (
            "taskbar at the top",
            POINT { x: 1800, y: 20 },
            rect(0, 48, 1920, 1080),
            (1480, 48),
        ),
        (
            "taskbar on the left",
            POINT { x: 30, y: 1000 },
            rect(62, 0, 1920, 1080),
            (62, 700),
        ),
        (
            "taskbar on the right",
            POINT { x: 1890, y: 1000 },
            rect(0, 0, 1858, 1080),
            (1538, 700),
        ),
    ];
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(name, at, work, expected)| {
            let got = host::menu_place(*at, *work, size);
            (got != *expected).then(|| format!("{name}: {got:?}, expected {expected:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The items are at least 36 high; the switch, the dot and the key stand at the end of their items;
/// the unread letter is emphasised; in Hebrew the icons are at the right and the ends at the left.
#[test]
fn the_items_of_the_menu_and_their_ends() {
    for rtl in [false, true] {
        let window = menu(false);
        let first = lay_out(&window, client_size(320.0, 400.0), 1.0, rtl, &MEASURE);
        let layout = lay_out(
            &window,
            client_size(320.0, 400.0 + first.shortfall.h),
            1.0,
            rtl,
            &MEASURE,
        );
        let rows: Vec<_> = layout
            .items
            .iter()
            .filter(|item| matches!(item.part, Part::MenuRow))
            .collect();
        assert_eq!(rows.len(), 5, "five items");
        for row in &rows {
            assert!(row.rect.h >= MENU_ROW_MIN_HEIGHT.0, "an item under 36");
        }
        let of = |id: Id, wanted: fn(&Part) -> bool| {
            layout
                .items
                .iter()
                .find(|item| item.id == Some(id) && wanted(&item.part))
                .map(|item| item.rect)
                .unwrap_or_else(|| panic!("{id:?}"))
        };
        let icon = of(Id::Feed, |part| matches!(part, Part::MenuIcon { .. }));
        let switch = of(Id::Feed, |part| matches!(part, Part::Toggle { .. }));
        if rtl {
            assert!(
                switch.right() < icon.x,
                "Hebrew: the switch is not at the left"
            );
        } else {
            assert!(icon.right() < switch.x, "the switch is not at the end");
        }
        of(Id::Letters, |part| matches!(part, Part::Dot(_)));
        of(Id::Letters, |part| {
            matches!(
                part,
                Part::Text {
                    role: Role::MenuEmphasis,
                    ..
                }
            )
        });
        of(Id::Pause, |part| {
            matches!(
                part,
                Part::Text {
                    role: Role::Small,
                    ..
                }
            )
        });
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item.part, Part::Divider))
                .count(),
            2,
            "two lines"
        );
    }
}

struct Menu {
    events: Rc<RefCell<Vec<Event<Id>>>>,
    feed: bool,
    escape: Option<Id>,
}

impl Content for Menu {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        menu(self.feed)
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        self.events.borrow_mut().push(event.clone());
        match event {
            Event::Toggled(Id::Feed, on) => {
                self.feed = on;
                Reply::Stay
            }
            _ => Reply::Close,
        }
    }

    fn title(&self) -> String {
        "Menu".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("ru-RU".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Light
    }

    fn escape(&self) -> Option<Id> {
        self.escape
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

fn open(keyboard: bool) -> (HWND, Rc<RefCell<Vec<Event<Id>>>>) {
    open_with(keyboard, None)
}

fn open_with(keyboard: bool, escape: Option<Id>) -> (HWND, Rc<RefCell<Vec<Event<Id>>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let content = Menu {
        events: events.clone(),
        feed: false,
        escape,
    };
    let hwnd =
        host::create_menu(content, None, POINT { x: 1800, y: 1000 }, keyboard).expect("the menu");
    send(hwnd, WM_PAINT, 0, 0);
    (hwnd, events)
}

fn mark(hwnd: HWND) -> Option<String> {
    host::probe(hwnd).expect("a live host").focus
}

/// From the keyboard the first item is marked; the arrows go round the available items, past the
/// unavailable one and the lines; a letter goes to its item; Enter presses the marked one.
#[test]
fn the_keys_go_round_the_available_items() {
    let (hwnd, events) = open(true);
    assert_eq!(
        mark(hwnd).as_deref(),
        Some("Pause"),
        "the first available item"
    );
    send(hwnd, WM_KEYDOWN, usize::from(VK_DOWN.0), 0);
    assert_eq!(
        mark(hwnd).as_deref(),
        Some("Feed"),
        "past the unavailable item and the line"
    );
    send(hwnd, WM_KEYDOWN, usize::from(VK_END.0), 0);
    assert_eq!(mark(hwnd).as_deref(), Some("Exit"));
    send(hwnd, WM_KEYDOWN, usize::from(VK_DOWN.0), 0);
    assert_eq!(mark(hwnd).as_deref(), Some("Pause"), "round from the last");
    send(hwnd, WM_KEYDOWN, usize::from(VK_UP.0), 0);
    assert_eq!(mark(hwnd).as_deref(), Some("Exit"), "round from the first");
    send(hwnd, WM_CHAR, 'п' as usize, 0);
    assert_eq!(
        mark(hwnd).as_deref(),
        Some("Pause"),
        "the letter of «Пауза»"
    );
    send(hwnd, WM_CHAR, 'н' as usize, 0);
    assert_eq!(
        mark(hwnd).as_deref(),
        Some("Pause"),
        "«Настройки» is unavailable"
    );

    send(hwnd, WM_CHAR, 'л' as usize, 0);
    assert_eq!(mark(hwnd).as_deref(), Some("Feed"));
    send(hwnd, WM_KEYDOWN, usize::from(VK_SPACE.0), 0);
    assert_eq!(
        events.borrow_mut().drain(..).collect::<Vec<_>>(),
        [Event::Toggled(Id::Feed, true)]
    );
    assert!(alive(hwnd), "the switch closed the menu");

    send(hwnd, WM_KEYDOWN, usize::from(VK_UP.0), 0);
    send(hwnd, WM_KEYDOWN, usize::from(VK_RETURN.0), 0);
    pump();
    assert_eq!(
        events.borrow_mut().drain(..).collect::<Vec<_>>(),
        [Event::Pressed(Id::Pause)]
    );
    assert!(!alive(hwnd), "a choice that closes left the menu");
}

/// The menu goes when it loses the foreground, and when it is closed from outside.
#[test]
fn the_menu_goes_with_the_foreground_and_from_outside() {
    let (hwnd, _) = open(false);
    send(hwnd, WM_ACTIVATE, WA_INACTIVE as usize, 0);
    pump();
    let gone = !alive(hwnd);
    if !gone {
        send(hwnd, WM_CLOSE, 0, 0);
    }
    assert!(gone, "the menu hangs about without the foreground");

    let (hwnd, _) = open(false);
    host::close(hwnd);
    pump();
    let gone = !alive(hwnd);
    if !gone {
        send(hwnd, WM_CLOSE, 0, 0);
    }
    assert!(gone, "host::close did not close the menu");
}

/// **Esc in the menu asks the content first, as in every window** (T-120-15): a content that names
/// what Esc presses hears it as that press — so it tells Esc from the loss of the foreground; one
/// that names nothing has the menu closed, as before.
#[test]
fn esc_in_the_menu_asks_the_content_first() {
    let (hwnd, events) = open_with(true, Some(Id::Exit));
    send(hwnd, WM_KEYDOWN, usize::from(VK_ESCAPE.0), 0);
    assert_eq!(
        events.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![Event::Pressed(Id::Exit)],
        "Esc is what the content names"
    );
    pump();

    let (hwnd, events) = open(true);
    send(hwnd, WM_KEYDOWN, usize::from(VK_ESCAPE.0), 0);
    pump();
    assert!(events.borrow().is_empty(), "nothing named: no event");
    assert!(!alive(hwnd), "and the menu is closed");
}
