//! The choice field of the shell (stage Э120, T-120-18 п. 4): its list in a popup of the shell under
//! the field — or over it where the work area ends — with the keys of the window it opens from,
//! the first letters typed, Enter, Esc, a click on a row and a click beside it. Nothing is on the
//! screen: the window is made and never shown, so its list is made and never shown either.

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{ButtonKind, Event, Node, Window, WindowKind};
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_END, VK_ESCAPE, VK_RETURN, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, IsWindow, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WM_CHAR, WM_CLOSE,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
};

#[path = "support/pressed.rs"]
mod pressed;
use pressed::press_through_the_loop_of_the_program;

/// The languages of the interface by their own names, as the list of the settings shows them.
const LANGUAGES: [&str; 8] = [
    "English",
    "Deutsch",
    "Français",
    "Română",
    "Русский",
    "Українська",
    "עברית",
    "日本語",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Language,
    Close,
}

struct Form {
    events: Rc<RefCell<Vec<Event<Id>>>>,
    selected: usize,
    options: Vec<String>,
}

impl Content for Form {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![Node::Choice {
                id: Id::Language,
                label: "Language".to_owned(),
                options: self.options.clone(),
                selected: self.selected,
            }]),
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
        if let Event::Selected(Id::Language, row) = event {
            self.selected = row;
        }
        self.events.borrow_mut().push(event);
        Reply::Stay
    }

    fn title(&self) -> String {
        "Choice".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("en-US".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Light
    }
}

struct Open {
    hwnd: HWND,
    events: Rc<RefCell<Vec<Event<Id>>>>,
}

impl Open {
    fn with(options: Vec<String>) -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let content = Form {
            events: events.clone(),
            selected: 0,
            options,
        };
        let hwnd = host::create(content, None).expect("the window of the shell");
        send(hwnd, WM_PAINT, 0, 0);
        Self { hwnd, events }
    }

    fn new() -> Self {
        Self::with(LANGUAGES.map(str::to_owned).to_vec())
    }

    fn probe(&self) -> host::Probe {
        send(self.hwnd, WM_PAINT, 0, 0);
        host::probe(self.hwnd).expect("a live host")
    }

    fn key(&self, key: u16) {
        send(self.hwnd, WM_KEYDOWN, usize::from(key), 0);
    }

    fn type_text(&self, text: &str) {
        for letter in text.chars() {
            send(self.hwnd, WM_CHAR, letter as usize, 0);
        }
    }

    fn events(&self) -> Vec<Event<Id>> {
        self.events.borrow_mut().drain(..).collect()
    }

    /// Tab to the choice field where the focus is not on it, then the space bar: the list opens.
    fn open_list(&self) -> HWND {
        if self.probe().focus.as_deref() != Some("Language") {
            self.key(VK_TAB.0);
        }
        assert_eq!(self.probe().focus.as_deref(), Some("Language"));
        self.key(VK_SPACE.0);
        self.probe()
            .dropdown
            .expect("the space bar did not open the list")
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        if alive(self.hwnd) {
            send(self.hwnd, WM_CLOSE, 0, 0);
        }
        pump();
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

/// `MK_LBUTTON`: the left button down, in the `wParam` of a message of the mouse.
const MK_LBUTTON: usize = 0x0001;

fn point(x: f32, y: f32) -> isize {
    ((y.round() as i16 as u16 as isize) << 16) | (x.round() as i16 as u16 as isize)
}

/// The parts of the window `hwnd` whose description contains `text`.
fn items(hwnd: HWND, text: &str) -> Vec<(String, Rect)> {
    send(hwnd, WM_PAINT, 0, 0);
    host::probe(hwnd)
        .expect("a live host")
        .items
        .into_iter()
        .filter(|(part, _, _)| part.contains(text))
        .map(|(part, _, rect)| (part, rect))
        .collect()
}

fn click(hwnd: HWND, rect: Rect) {
    let at = point(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    send(hwnd, WM_LBUTTONDOWN, 0, at);
    send(hwnd, WM_LBUTTONUP, 0, at);
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

/// Under the field while the work area holds it; over the field where it ends; never out of it.
#[test]
fn the_list_stands_under_the_field_or_over_it_inside_the_work_area() {
    let work = rect(0, 0, 1920, 1040);
    let field = rect(100, 400, 400, 436);
    assert_eq!(host::under(field, work, (300, 200), 4), (100, 440));
    let low = rect(100, 900, 400, 936);
    assert_eq!(
        host::under(low, work, (300, 200), 4),
        (100, 696),
        "no room below: over the field"
    );
    let tall = host::under(low, work, (300, 1200), 4);
    assert_eq!(tall, (100, 0), "taller than the work area: from its top");
    let right = host::under(rect(1800, 400, 2100, 436), work, (300, 200), 4);
    assert_eq!(right.0, 1620, "kept inside on the right");
}

/// The first letters typed choose the row: «ру» is «Русский»; Enter takes it.
#[test]
fn typing_the_first_letters_chooses_the_row_and_enter_takes_it() {
    let open = Open::new();
    let list = open.open_list();
    assert!(
        !items(list, "ListRow { selected: true, index: 0").is_empty(),
        "the list opens on the value now"
    );
    assert!(
        !items(list, "Tick").is_empty(),
        "the value now is not ticked"
    );

    open.type_text("ру");
    assert!(
        !items(list, "ListRow { selected: true, index: 4").is_empty(),
        "«ру» did not reach «Русский»"
    );
    open.key(VK_RETURN.0);
    pump();
    assert_eq!(open.events(), [Event::Selected(Id::Language, 4)]);
    assert_eq!(open.probe().dropdown, None, "Enter left the list open");
    assert!(!alive(list), "the list's window outlived Enter");
}

/// The space bar that opens the list, taken the way the program takes it — its `WM_CHAR` comes
/// after the key down (blocker of 1.8.0, stand of 21.11): the list opens and the space is not
/// typed into it, so «ру» typed at once still reaches «Русский».
#[test]
fn the_space_bar_that_opens_the_list_types_nothing_into_it() {
    let open = Open::new();
    if open.probe().focus.as_deref() != Some("Language") {
        open.key(VK_TAB.0);
    }
    assert_eq!(open.probe().focus.as_deref(), Some("Language"));

    press_through_the_loop_of_the_program(open.hwnd, VK_SPACE.0, 0x39);
    let list = open
        .probe()
        .dropdown
        .expect("the space bar did not open the list");
    open.type_text("ру");
    assert!(
        !items(list, "ListRow { selected: true, index: 4").is_empty(),
        "«ру» after the space bar did not reach «Русский»"
    );
}

/// Esc closes the list without a choice, though the keys moved in it.
#[test]
fn esc_closes_the_list_without_a_choice() {
    let open = Open::new();
    let list = open.open_list();
    open.key(VK_DOWN.0);
    open.key(VK_END.0);
    open.key(VK_ESCAPE.0);
    pump();
    assert!(open.events().is_empty(), "Esc chose a row");
    assert!(!alive(list), "Esc left the list open");
    assert!(alive(open.hwnd), "Esc in the list closed the window");
}

/// A click on a row chooses it; a click beside the list, in the window, closes it with nothing.
#[test]
fn a_click_on_a_row_chooses_it_and_a_click_beside_closes() {
    let open = Open::new();
    let list = open.open_list();
    let (_, row) = items(list, "ListRow { selected: false, index: 2").remove(0);
    click(list, row);
    pump();
    assert_eq!(open.events(), [Event::Selected(Id::Language, 2)]);
    assert!(!alive(list), "the list stayed after the click");

    let list = open.open_list();
    let (_, button) = items(open.hwnd, "Button").remove(0);
    click(open.hwnd, button);
    pump();
    assert!(!alive(list), "a click beside the list left it open");
    assert!(
        open.events().is_empty(),
        "the click that closed the list pressed the button under it"
    );
}

/// 66 languages: the list shows some and scrolls; the end of it is reached by the keys; a closed
/// field takes the next option by the arrow.
#[test]
fn a_long_list_scrolls_and_a_closed_field_steps_by_the_arrows() {
    let open = Open::with((1..=66).map(|n| format!("Language {n:02}")).collect());
    open.key(VK_TAB.0);
    open.key(VK_DOWN.0);
    assert_eq!(open.events(), [Event::Selected(Id::Language, 1)]);

    let list = open.open_list();
    assert!(
        !items(list, "Scroll {").is_empty(),
        "66 rows and no scroll bar"
    );
    open.key(VK_END.0);
    assert!(
        !items(list, "ListRow { selected: true, index: 65").is_empty(),
        "the end of the list is not shown"
    );
    open.key(VK_RETURN.0);
    pump();
    assert_eq!(open.events(), [Event::Selected(Id::Language, 65)]);
}

/// The first row the bar of the list `hwnd` says is shown: `Scroll { first: N, .. }`.
fn first_shown(hwnd: HWND) -> usize {
    let (part, _) = items(hwnd, "Scroll {").remove(0);
    part.split("first: ")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|number| number.trim().parse().ok())
        .unwrap_or_else(|| panic!("no first row in «{part}»"))
}

/// ⭐ **Решение 208.235** — «прокрутка в выборе языка интерфейса не работает, только колесиком
/// мышкой за палочку не тянется»: the thumb of the bar is taken by the mouse — also beside the thin
/// bar, in its margin — and the rows follow it to the end of the list; let go, the list stays open
/// and nothing is chosen. A press on the bar beside the thumb goes a page.
#[test]
fn the_thumb_of_the_bar_is_dragged_by_the_mouse() {
    let open = Open::with((1..=66).map(|n| format!("Language {n:02}")).collect());
    let list = open.open_list();
    let (_, bar) = items(list, "Scroll {").remove(0);
    assert_eq!(first_shown(list), 0);

    // The thumb stands at the top: taken just left of the bar, in its margin.
    let x = bar.x - 2.0;
    send(list, WM_LBUTTONDOWN, MK_LBUTTON, point(x, bar.y + 2.0));
    send(
        list,
        WM_MOUSEMOVE,
        MK_LBUTTON,
        point(x, bar.y + bar.h / 2.0),
    );
    let middle = first_shown(list);
    assert!(
        middle > 0 && middle < 66,
        "the thumb half way down shows row {middle}"
    );
    // Far below the window: the end of the list, no further.
    send(
        list,
        WM_MOUSEMOVE,
        MK_LBUTTON,
        point(x, bar.bottom() + 400.0),
    );
    send(list, WM_LBUTTONUP, 0, point(x, bar.bottom() + 400.0));
    pump();
    assert!(alive(list), "letting go of the thumb closed the list");
    assert!(
        !items(list, "index: 65").is_empty(),
        "the thumb dragged to the end does not show the last row"
    );
    let end = first_shown(list);
    assert!(open.events().is_empty(), "the drag chose a row");

    // Beside the thumb, above it: a page back.
    send(list, WM_LBUTTONDOWN, MK_LBUTTON, point(x, bar.y + 1.0));
    send(list, WM_LBUTTONUP, 0, point(x, bar.y + 1.0));
    let paged = first_shown(list);
    assert!(
        paged < end && paged > 0,
        "a press above the thumb went from {end} to {paged}, not a page"
    );
    // The mouse moving with no thumb held scrolls nothing.
    send(list, WM_MOUSEMOVE, 0, point(x, bar.bottom() - 1.0));
    assert_eq!(first_shown(list), paged);
    send(list, WM_LBUTTONUP, 0, point(x, bar.bottom() - 1.0));
    pump();
    assert!(open.events().is_empty(), "the bar chose a row");
}
