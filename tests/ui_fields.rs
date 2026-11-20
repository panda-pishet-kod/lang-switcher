//! The text fields and the drawn lists of the shell (stage Э120, T-120-18 пп. 1–3): the field of
//! Windows inside its drawn wrapper, the keyboard ring passed between the field and the drawn
//! elements both ways, the list with its keys, mouse, wheel and check boxes. Nothing is on the
//! screen: the window is made and never shown, and the tests send it — or its field, where the
//! keyboard would be — the messages Windows would.

use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{ButtonKind, Event, FieldKind, ListItem, Node, Window, WindowKind};
use lang_switcher::ui::tokens::{Color, Theme};
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, DeleteDC, GetBkColor, GetObjectW, GetTextColor, HGDIOBJ,
    LOGFONTW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardState, SetKeyboardState, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_RETURN,
    VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, ES_MULTILINE, ES_RIGHT, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, IsWindow,
    MSG, PM_REMOVE, PeekMessageW, SendMessageW, SetWindowTextW, WM_CLOSE, WM_CTLCOLOREDIT,
    WM_GETFONT, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL, WM_PAINT,
    WS_EX_RTLREADING,
};
use windows::core::HSTRING;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Name,
    Note,
    Layouts,
    Apply,
    Close,
}

/// A window with two fields, a list and two buttons.
struct Form {
    events: Rc<RefCell<Vec<Event<Id>>>>,
    name: String,
    note: String,
    note_kind: FieldKind,
    rows: usize,
    checks: Option<Vec<bool>>,
    selected: usize,
    first: usize,
    language: (String, bool),
    theme: ThemeSetting,
}

impl Content for Form {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let rows = (0..self.rows)
            .map(|row| ListItem {
                label: format!("Layout {row}"),
                check: self.checks.as_ref().map(|checks| checks[row]),
                mark: false,
            })
            .collect();
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Header {
                    eyebrow: None,
                    title: "Fields".to_owned(),
                    subtitle: None,
                },
                Node::Field {
                    id: Id::Name,
                    label: "Process".to_owned(),
                    value: self.name.clone(),
                    hint: "game.exe".to_owned(),
                    kind: FieldKind::Line,
                },
                Node::Field {
                    id: Id::Note,
                    label: "Note".to_owned(),
                    value: self.note.clone(),
                    hint: "What happened".to_owned(),
                    kind: self.note_kind,
                },
                Node::List {
                    id: Id::Layouts,
                    rows,
                    selected: self.selected,
                    first: self.first,
                },
            ]),
            buttons: vec![
                Node::Button {
                    id: Id::Apply,
                    label: "Apply".to_owned(),
                    kind: ButtonKind::Normal,
                    browser: false,
                },
                Node::Button {
                    id: Id::Close,
                    label: "Close".to_owned(),
                    kind: ButtonKind::Primary,
                    browser: false,
                },
            ],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        self.events.borrow_mut().push(event.clone());
        match event {
            Event::Edited(Id::Name, text) => self.name = text,
            Event::Edited(Id::Note, text) => self.note = text,
            Event::Selected(Id::Layouts, row) => self.selected = row,
            Event::Scrolled(Id::Layouts, first) => self.first = first,
            Event::RowChecked(Id::Layouts, row, on) => {
                if let Some(checks) = self.checks.as_mut() {
                    checks[row] = on;
                }
            }
            Event::Pressed(Id::Close) => return Reply::Close,
            // A window opened with a field of code: «Apply» makes it a field of text.
            Event::Pressed(Id::Apply) if self.note_kind == FieldKind::Code => {
                self.note_kind = FieldKind::Text;
            }
            _ => {}
        }
        Reply::Stay
    }

    fn title(&self) -> String {
        "Fields".to_owned()
    }

    fn language(&self) -> (String, bool) {
        self.language.clone()
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

struct Open {
    hwnd: HWND,
    events: Rc<RefCell<Vec<Event<Id>>>>,
}

impl Open {
    fn with(rows: usize, checks: bool, language: (&str, bool), theme: ThemeSetting) -> Self {
        Self::with_note(rows, checks, language, theme, FieldKind::Text)
    }

    /// The window with its field «Note» of `note_kind`.
    fn with_note(
        rows: usize,
        checks: bool,
        language: (&str, bool),
        theme: ThemeSetting,
        note_kind: FieldKind,
    ) -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let content = Form {
            events: events.clone(),
            name: String::new(),
            note: String::new(),
            note_kind,
            rows,
            checks: checks.then(|| (0..rows).map(|row| row % 2 == 0).collect()),
            selected: 0,
            first: 0,
            language: (language.0.to_owned(), language.1),
            theme,
        };
        let hwnd = host::create(content, None).expect("the window of the shell");
        let open = Self { hwnd, events };
        send(open.hwnd, WM_PAINT, 0, 0);
        open
    }

    fn new() -> Self {
        Self::with(20, false, ("en-US", false), ThemeSetting::Light)
    }

    fn probe(&self) -> host::Probe {
        host::probe(self.hwnd).expect("a live host")
    }

    fn focus(&self) -> Option<String> {
        self.probe().focus
    }

    fn field(&self, id: Id) -> HWND {
        let name = format!("{id:?}");
        self.probe()
            .fields
            .into_iter()
            .find(|(field, _)| *field == name)
            .map(|(_, hwnd)| hwnd)
            .unwrap_or_else(|| panic!("no field of Windows for {name}"))
    }

    /// A key where the keyboard is: in the focused field, else in the window.
    fn key(&self, key: u16) {
        let target = match self.focus() {
            Some(focus) if focus == "Name" => self.field(Id::Name),
            Some(focus) if focus == "Note" => self.field(Id::Note),
            _ => self.hwnd,
        };
        send(target, WM_KEYDOWN, usize::from(key), 0);
    }

    /// The same key with Shift held, for this thread only.
    fn key_with_shift(&self, key: u16) {
        let mut state = [0_u8; 256];
        // SAFETY: the key state of this thread, into and out of a local of the size asked.
        unsafe {
            let _ = GetKeyboardState(&mut state);
            let saved = state;
            state[usize::from(VK_SHIFT.0)] = 0x80;
            let _ = SetKeyboardState(&state);
            self.key(key);
            let _ = SetKeyboardState(&saved);
        }
    }

    fn events(&self) -> Vec<Event<Id>> {
        self.events.borrow_mut().drain(..).collect()
    }

    /// The parts the layout placed whose description contains `text`, with their rectangles.
    fn items(&self, text: &str) -> Vec<(String, Rect)> {
        // A layout dropped by an event is made again by the next frame.
        send(self.hwnd, WM_PAINT, 0, 0);
        self.probe()
            .items
            .into_iter()
            .filter(|(part, _, _)| part.contains(text))
            .map(|(part, _, rect)| (part, rect))
            .collect()
    }

    /// A click in the middle of the first part whose description contains `text`.
    fn click(&self, text: &str) {
        let (_, Rect { x, y, w, h }) = self.items(text).into_iter().next().unwrap_or_else(|| {
            panic!("no part «{text}» in the layout");
        });
        let at = point(x + w / 2.0, y + h / 2.0);
        send(self.hwnd, WM_LBUTTONDOWN, 0, at);
        send(self.hwnd, WM_LBUTTONUP, 0, at);
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

/// The red one before the task: in a window with fields, Tab from a field goes on to the next
/// element of the description — the field gives the keyboard ring back to the window.
#[test]
fn tab_from_a_field_goes_to_the_next_element() {
    let open = Open::new();
    open.key(VK_TAB.0);
    assert_eq!(open.focus().as_deref(), Some("Name"));
    open.key(VK_TAB.0);
    assert_eq!(
        open.focus().as_deref(),
        Some("Note"),
        "Tab pressed in the field Name did not leave it"
    );
}

/// Tab goes through both fields, the list and the buttons and comes back; Shift+Tab — the same
/// way back, out of a field too.
#[test]
fn tab_walks_the_fields_the_list_and_the_buttons_both_ways() {
    let open = Open::new();
    let order = ["Name", "Note", "Layouts", "Apply", "Close"];

    for expected in order.iter().chain(order.first()) {
        let from = open.focus();
        open.key(VK_TAB.0);
        assert_eq!(
            open.focus().as_deref(),
            Some(*expected),
            "Tab from {from:?} did not reach {expected}"
        );
    }
    for expected in order.iter().rev() {
        let from = open.focus();
        open.key_with_shift(VK_TAB.0);
        assert_eq!(
            open.focus().as_deref(),
            Some(*expected),
            "Shift+Tab from {from:?} did not reach {expected}"
        );
    }
}

/// The keys of the list select rows and give the event with the row; the end of a list of 20
/// scrolls so that the selected row is shown.
#[test]
fn the_keys_of_a_list_select_rows_and_scroll_to_them() {
    let open = Open::new();
    for _ in 0..3 {
        open.key(VK_TAB.0);
    }
    assert_eq!(open.focus().as_deref(), Some("Layouts"));

    open.key(VK_DOWN.0);
    assert_eq!(open.events(), [Event::Selected(Id::Layouts, 1)]);
    open.key(VK_UP.0);
    assert_eq!(open.events(), [Event::Selected(Id::Layouts, 0)]);

    open.key(VK_END.0);
    let events = open.events();
    assert_eq!(events[0], Event::Selected(Id::Layouts, 19));
    assert!(
        matches!(events.get(1), Some(Event::Scrolled(Id::Layouts, first)) if *first > 0),
        "the end of the list did not scroll: {events:?}"
    );
    send(open.hwnd, WM_PAINT, 0, 0);
    assert!(
        !open.items("ListRow { selected: true, index: 19").is_empty(),
        "row 19 is selected but not shown"
    );
    assert!(
        open.items("index: 0,").is_empty(),
        "row 0 is still shown after the end"
    );

    open.key(VK_HOME.0);
    assert_eq!(
        open.events(),
        [
            Event::Selected(Id::Layouts, 0),
            Event::Scrolled(Id::Layouts, 0)
        ]
    );
    open.key(VK_NEXT.0);
    let events = open.events();
    assert!(
        matches!(events.first(), Some(Event::Selected(Id::Layouts, row)) if *row > 1),
        "Page Down did not go a page: {events:?}"
    );
    assert!(
        !open.items("Scroll {").is_empty(),
        "20 rows and no scroll bar"
    );
}

/// A click on a row selects it; the wheel scrolls the list under the mouse.
#[test]
fn the_mouse_and_the_wheel_reach_the_rows() {
    let open = Open::new();
    open.click("ListRow { selected: false, index: 2");
    assert_eq!(open.events(), [Event::Selected(Id::Layouts, 2)]);
    assert_eq!(open.focus().as_deref(), Some("Layouts"));

    send(open.hwnd, WM_PAINT, 0, 0);
    let (_, Rect { x, y, w, h }) = open.items("List(").remove(0);
    let mut at = POINT {
        x: (x + w / 2.0) as i32,
        y: (y + h / 2.0) as i32,
    };
    // SAFETY: a window of this thread; the point is a local.
    let _ = unsafe { ClientToScreen(open.hwnd, &mut at) };
    let down = (-120_i16 as u16 as usize) << 16;
    send(
        open.hwnd,
        WM_MOUSEWHEEL,
        down,
        point(at.x as f32, at.y as f32),
    );
    assert_eq!(open.events(), [Event::Scrolled(Id::Layouts, 3)]);
}

/// A row with a check box: a click selects the row and turns its box, with the row and the value;
/// the space bar turns the box of the selected row.
#[test]
fn a_check_box_of_a_row_gives_the_row_and_its_value() {
    let open = Open::with(5, true, ("en-US", false), ThemeSetting::Light);
    open.click("ListRow { selected: false, index: 2");
    assert_eq!(
        open.events(),
        [
            Event::Selected(Id::Layouts, 2),
            Event::RowChecked(Id::Layouts, 2, false)
        ]
    );
    open.key(VK_SPACE.0);
    assert_eq!(open.events(), [Event::RowChecked(Id::Layouts, 2, true)]);
}

/// What is typed in a field reaches the window as an event with the text; Enter in a line field
/// presses the main button; Esc from a field closes the window.
#[test]
fn a_field_tells_the_window_what_is_typed_and_passes_enter_and_esc() {
    let open = Open::new();
    let name = open.field(Id::Name);
    // SAFETY: a field of this thread; the string lives across the call.
    let _ = unsafe { SetWindowTextW(name, &HSTRING::from("game.exe")) };
    assert_eq!(
        open.events(),
        [Event::Edited(Id::Name, "game.exe".to_owned())]
    );

    open.key(VK_TAB.0);
    open.key(VK_RETURN.0);
    assert_eq!(open.events(), [Event::Pressed(Id::Close)]);
    open.pump();
    assert!(!open.alive(), "Enter in the field did not press «Close»");

    let open = Open::new();
    open.key(VK_TAB.0);
    open.key(VK_ESCAPE.0);
    open.pump();
    assert!(!open.alive(), "Esc in the field did not close the window");
}

/// The field takes its colours from the table — in the dark theme too — and the direction of a
/// language read right to left.
#[test]
fn the_field_takes_the_colours_of_the_theme_and_the_direction_of_the_language() {
    let open = Open::with(3, false, ("he-IL", true), ThemeSetting::Dark);
    let name = open.field(Id::Name);

    // SAFETY: a DC of the screen made and deleted here.
    let dc = unsafe { CreateCompatibleDC(None) };
    send(open.hwnd, WM_CTLCOLOREDIT, dc.0 as usize, name.0 as isize);
    // SAFETY: as above.
    let (ink, fill) = unsafe { (GetTextColor(dc).0, GetBkColor(dc).0) };
    // SAFETY: as above.
    let _ = unsafe { DeleteDC(dc) };
    let colorref = |colour: Color| {
        let rgb = colour.get(Theme::Dark);
        u32::from(rgb.r) | (u32::from(rgb.g) << 8) | (u32::from(rgb.b) << 16)
    };
    assert_eq!(ink, colorref(Color::Ink), "the text of the field");
    assert_eq!(
        fill,
        colorref(Color::Surface),
        "the background of the field"
    );

    // SAFETY: a window of this thread.
    let (style, extended) = unsafe {
        (
            GetWindowLongPtrW(name, GWL_STYLE),
            GetWindowLongPtrW(name, GWL_EXSTYLE),
        )
    };
    assert_ne!(
        style & ES_RIGHT as isize,
        0,
        "a field in Hebrew is not aligned right"
    );
    assert_ne!(
        extended & WS_EX_RTLREADING.0 as isize,
        0,
        "a field in Hebrew does not read right to left"
    );
}

/// The face of the font a field of Windows is set in.
fn face_of(field: HWND) -> String {
    let font = send(field, WM_GETFONT, 0, 0);
    let mut logfont = LOGFONTW::default();
    // SAFETY: a font the shell made, alive while its field is; a local of the size given.
    let size = unsafe {
        GetObjectW(
            HGDIOBJ(font as *mut _),
            i32::try_from(std::mem::size_of::<LOGFONTW>()).expect("a small struct"),
            Some((&raw mut logfont).cast()),
        )
    };
    assert!(size > 0, "the font of the field");
    let name = &logfont.lfFaceName;
    let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..len])
}

/// **A field of code** (T-120-19, the wizard's report): many lines, typed in as a field of text,
/// set in the monospace face of the role `code`; the other fields keep the face of `body`.
#[test]
fn a_field_of_code_is_set_in_the_monospace_face() {
    let mono = ["Cascadia Mono", "Consolas"];
    let open = Open::with_note(
        20,
        false,
        ("en-US", false),
        ThemeSetting::Light,
        FieldKind::Code,
    );
    let (note, name) = (open.field(Id::Note), open.field(Id::Name));
    let (note_face, name_face) = (face_of(note), face_of(name));
    assert!(
        mono.contains(&note_face.as_str()),
        "the field of code: {note_face}"
    );
    assert!(!mono.contains(&name_face.as_str()), "a line: {name_face}");
    // SAFETY: a window of this thread.
    let style = unsafe { GetWindowLongPtrW(note, GWL_STYLE) };
    assert_ne!(style & ES_MULTILINE as isize, 0, "many lines");
    drop(open);

    let open = Open::new();
    assert_eq!(
        face_of(open.field(Id::Note)),
        face_of(open.field(Id::Name)),
        "a field of text: the face of body"
    );
}

/// **The monospace font is let go** once no field of the window is of code. Counted as the fonts
/// the fields hold: GDI defers deleting a font still selected into a cached DC of an edit control,
/// so `GetObjectW` may answer for a font already deleted (measured: `DeleteObject` gave TRUE).
#[cfg(feature = "testing")]
#[test]
fn the_monospace_font_is_held_while_a_field_of_code_is_there() {
    let code = || {
        Open::with_note(
            20,
            false,
            ("en-US", false),
            ThemeSetting::Light,
            FieldKind::Code,
        )
    };
    let open = code();
    assert_eq!(open.probe().field_fonts, 2, "body and monospace");
    open.click("\"Apply\"");
    send(open.hwnd, WM_PAINT, 0, 0);
    let name_face = face_of(open.field(Id::Name));
    assert_eq!(
        face_of(open.field(Id::Note)),
        name_face,
        "a field of text now"
    );
    assert_eq!(
        open.probe().field_fonts,
        1,
        "the monospace font let go with the last field of code"
    );
}
