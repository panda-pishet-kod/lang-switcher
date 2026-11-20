//! The field of a key of the shell (stage Э120, T-120-20 core; T-120-18 «поле захвата клавиши»):
//! out of its capture it only shows the key — no stop of Tab, a press on it lands beside (104.3);
//! in its capture it holds the keyboard without a caret and every key goes to the content — Tab,
//! Esc, Enter, F10 too — and the loss of the keyboard comes with where it went. The window is made
//! and never shown.

#![cfg(feature = "testing")]

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{ButtonKind, Event, FieldKind, Node, Window, WindowKind};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, VK_ESCAPE, VK_F10, VK_RETURN, VK_TAB};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GUITHREADINFO, GetGUIThreadInfo, IsWindow, MSG, PM_REMOVE, PeekMessageW,
    SendMessageW, WM_CLOSE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_PAINT,
    WM_SYSKEYDOWN,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Key,
    Set,
    Close,
}

/// The section of the hotkey, in short: the field, «Set», «Close». «Set» arms the capture; Esc,
/// or the keyboard lost anywhere but to «Set», ends it (as `settings::CapturePair` does).
struct Hotkey {
    capturing: Rc<Cell<bool>>,
    events: Rc<RefCell<Vec<Event<Id>>>>,
}

impl Content for Hotkey {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Field {
                    id: Id::Key,
                    label: "Key".to_owned(),
                    value: "Pause".to_owned(),
                    hint: String::new(),
                    kind: FieldKind::Key {
                        capturing: self.capturing.get(),
                    },
                },
                Node::Button {
                    id: Id::Set,
                    label: "Set".to_owned(),
                    kind: ButtonKind::Normal,
                    browser: false,
                },
            ]),
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
        self.events.borrow_mut().push(event.clone());
        match event {
            Event::Pressed(Id::Set) => self.capturing.set(true),
            Event::KeyDown { vk, .. } if vk == VK_ESCAPE.0 => self.capturing.set(false),
            Event::FocusLost { to, .. } if to != Some(Id::Set) => self.capturing.set(false),
            Event::Pressed(Id::Close) => return Reply::Close,
            _ => {}
        }
        Reply::Stay
    }

    fn title(&self) -> String {
        "Hotkey".to_owned()
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
    capturing: Rc<Cell<bool>>,
    events: Rc<RefCell<Vec<Event<Id>>>>,
}

impl Open {
    fn new() -> Self {
        let capturing = Rc::new(Cell::new(false));
        let events = Rc::new(RefCell::new(Vec::new()));
        let hwnd = host::create(
            Hotkey {
                capturing: capturing.clone(),
                events: events.clone(),
            },
            None,
        )
        .expect("the window of the shell");
        send(hwnd, WM_PAINT, 0, 0);
        Self {
            hwnd,
            capturing,
            events,
        }
    }

    fn probe(&self) -> host::Probe {
        host::probe(self.hwnd).expect("a live host")
    }

    fn focus(&self) -> Option<String> {
        self.probe().focus
    }

    fn field(&self) -> HWND {
        self.probe()
            .fields
            .into_iter()
            .find(|(id, _)| id == "Key")
            .map(|(_, hwnd)| hwnd)
            .expect("the field of Windows of the key")
    }

    fn rect(&self, text: &str) -> Rect {
        self.probe()
            .items
            .into_iter()
            .find(|(part, _, _)| part.contains(text))
            .unwrap_or_else(|| panic!("no part «{text}»"))
            .2
    }

    fn press(&self, x: f32, y: f32) {
        send(self.hwnd, WM_LBUTTONDOWN, 0, point(x, y));
    }

    fn release(&self, x: f32, y: f32) {
        send(self.hwnd, WM_LBUTTONUP, 0, point(x, y));
    }

    fn click(&self, text: &str) {
        let r = self.rect(text);
        let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        self.press(x, y);
        self.release(x, y);
    }

    /// Arms the capture with «Set» and lets the frame give the keyboard to the field.
    fn arm(&self) {
        self.click("\"Set\"");
        send(self.hwnd, WM_PAINT, 0, 0);
        self.pump();
        assert!(self.capturing.get(), "«Set» armed no capture");
        self.events.borrow_mut().clear();
    }

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
            send(self.hwnd, WM_CLOSE, 0, 0);
        }
    }
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: a window of this thread; the message goes straight to its procedure.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))) }.0
}

fn point(x: f32, y: f32) -> isize {
    let (x, y) = (x.round() as i16 as u16, y.round() as i16 as u16);
    ((y as isize) << 16) | x as isize
}

/// **Out of its capture the field only shows the key**: Tab goes round the window and never stops
/// in it, and a press on it gives it no keyboard.
#[test]
fn a_resting_field_of_a_key_is_no_stop_of_tab_and_takes_no_press() {
    let open = Open::new();
    let mut seen = Vec::new();
    for _ in 0..4 {
        send(open.hwnd, WM_KEYDOWN, usize::from(VK_TAB.0), 0);
        seen.push(open.focus());
    }
    assert!(
        seen.iter().all(|focus| focus.as_deref() != Some("Key")),
        "Tab stopped in the resting field: {seen:?}"
    );
    assert!(seen.contains(&Some("Set".to_owned())) && seen.contains(&Some("Close".to_owned())));

    let r = open.rect("Input");
    open.press(r.x + 2.0, r.y + 2.0);
    open.release(r.x + 2.0, r.y + 2.0);
    assert_ne!(
        open.focus().as_deref(),
        Some("Key"),
        "a press gave it the keyboard"
    );
    // SAFETY: reads the focus of this thread.
    assert_ne!(unsafe { GetFocus() }, open.field());
}

/// **In its capture the field holds the keyboard, without a caret, and every key goes to the
/// content** — Tab, Enter, F10 (a key with Alt's message) and the releases; none moves the focus
/// or closes the window.
#[test]
fn in_its_capture_every_key_goes_to_the_content() {
    let open = Open::new();
    open.arm();
    let field = open.field();
    assert_eq!(open.focus().as_deref(), Some("Key"));
    // SAFETY: reads the focus of this thread.
    assert_eq!(
        unsafe { GetFocus() },
        field,
        "the keyboard is not in the field"
    );
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: the structure is a local of the size it says.
    unsafe { GetGUIThreadInfo(0, &mut info) }.expect("the state of this thread");
    assert_ne!(info.hwndCaret, field, "a caret in the field of a key");

    send(field, WM_KEYDOWN, usize::from(VK_TAB.0), 0);
    send(field, WM_KEYDOWN, usize::from(VK_RETURN.0), 0);
    send(field, WM_SYSKEYDOWN, usize::from(VK_F10.0), 0);
    send(field, WM_KEYUP, usize::from(VK_TAB.0), 0);
    open.pump();

    assert_eq!(
        *open.events.borrow(),
        vec![
            Event::KeyDown {
                id: Id::Key,
                vk: VK_TAB.0
            },
            Event::KeyDown {
                id: Id::Key,
                vk: VK_RETURN.0
            },
            Event::KeyDown {
                id: Id::Key,
                vk: VK_F10.0
            },
            Event::KeyUp {
                id: Id::Key,
                vk: VK_TAB.0
            },
        ]
    );
    assert_eq!(
        open.focus().as_deref(),
        Some("Key"),
        "a key moved the focus"
    );
    assert!(open.alive());
}

/// **Esc goes to the capture, not to the window**: the content ends the capture and the window stays;
/// the keyboard goes on to the element after the field, and the ended capture reports no loss.
#[test]
fn esc_ends_the_capture_and_not_the_window() {
    let open = Open::new();
    open.arm();
    send(open.field(), WM_KEYDOWN, usize::from(VK_ESCAPE.0), 0);
    assert!(!open.capturing.get(), "Esc did not reach the capture");
    assert!(open.alive(), "Esc closed the window");

    send(open.hwnd, WM_PAINT, 0, 0);
    open.pump();
    assert_eq!(
        open.focus().as_deref(),
        Some("Set"),
        "the keyboard after the capture"
    );
    assert!(
        !open
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::FocusLost { .. })),
        "an ended capture reported a loss: {:?}",
        open.events.borrow()
    );
}

/// **The loss of the keyboard says where it went**: a press on «Set» — to «Set» (the capture goes
/// on, the press is a click of its own); a press beside the elements — to none.
#[test]
fn the_loss_of_the_keyboard_says_where_it_went() {
    let open = Open::new();
    open.arm();
    let set = open.rect("\"Set\"");
    open.press(set.x + set.w / 2.0, set.y + set.h / 2.0);
    open.pump();
    assert_eq!(
        open.events.borrow().first(),
        Some(&Event::FocusLost {
            id: Id::Key,
            to: Some(Id::Set)
        }),
        "{:?}",
        open.events.borrow()
    );
    open.release(set.x + set.w / 2.0, set.y + set.h / 2.0);
    open.pump();

    send(open.hwnd, WM_PAINT, 0, 0);
    open.pump();
    assert!(open.capturing.get());
    open.events.borrow_mut().clear();
    open.press(1.0, 1.0);
    open.release(1.0, 1.0);
    open.pump();
    assert_eq!(
        open.events.borrow().first(),
        Some(&Event::FocusLost {
            id: Id::Key,
            to: None
        }),
        "{:?}",
        open.events.borrow()
    );
}
