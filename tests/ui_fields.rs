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
use std::sync::{Mutex, MutexGuard, PoisonError};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, DeleteDC, GetBkColor, GetObjectW, GetTextColor, HGDIOBJ,
    LOGBRUSH, LOGFONTW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardState, SetKeyboardState, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT,
    VK_RETURN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, ES_MULTILINE, ES_RIGHT, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW,
    GetWindowTextW, IsWindow, MSG, PM_REMOVE, PeekMessageW, SendMessageW, SetWindowTextW, WM_CLOSE,
    WM_CTLCOLOREDIT, WM_GETFONT, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL, WM_PAINT,
    WS_EX_RTLREADING,
};
use windows::core::HSTRING;

#[path = "support/pressed.rs"]
mod pressed;
use pressed::press_through_the_loop_of_the_program;

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
            Event::Pressed(Id::Apply) if matches!(self.note_kind, FieldKind::Code { .. }) => {
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
    /// This test's turn: let go after the window is closed (`Drop` runs before the fields go).
    _turn: MutexGuard<'static, ()>,
}

/// The windows of these tests open one at a time. A window that gains the foreground has the key
/// state of its thread synced by Windows, and a Shift another test holds for its thread goes with
/// it: the windows of one test binary must not take the foreground from each other.
static ONE_WINDOW: Mutex<()> = Mutex::new(());

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
        let turn = ONE_WINDOW.lock().unwrap_or_else(PoisonError::into_inner);
        let hwnd = host::create(content, None).expect("the window of the shell");
        let open = Self {
            hwnd,
            events,
            _turn: turn,
        };
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

    /// The same key with Shift held (`shift`) or up, for this thread only; whether Shift was still
    /// so once the key was taken. Windows syncs the key state of a thread whose window gains the
    /// foreground with the keyboard of the machine — a window of another program closing is enough,
    /// and a Shift pressed there comes in — so a step whose Shift changed is no trial of the window.
    fn key_with_shift(&self, key: u16, shift: bool) -> bool {
        let mut state = [0_u8; 256];
        // SAFETY: the key state of this thread, into and out of a local of the size asked.
        unsafe {
            let _ = GetKeyboardState(&mut state);
            let saved = state;
            state[usize::from(VK_SHIFT.0)] = if shift { 0x80 } else { 0 };
            let _ = SetKeyboardState(&state);
            self.key(key);
            let held = GetKeyState(i32::from(VK_SHIFT.0)) < 0;
            let _ = SetKeyboardState(&saved);
            held == shift
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
    let order = ["Name", "Note", "Layouts", "Apply", "Close"];
    // A step whose Shift Windows changed meanwhile is no trial: the walk is made again in a fresh
    // window, three times at most.
    for _ in 0..3 {
        if walk(&Open::new(), &order) {
            return;
        }
    }
    panic!("Windows changed Shift in three walks: no trial of the window");
}

/// Tab through `order` and round to its first, then Shift+Tab the whole way back; false when
/// Windows changed the Shift of this thread in a step (no trial).
fn walk(open: &Open, order: &[&str]) -> bool {
    let forth = order.iter().chain(order.first()).map(|to| (to, false));
    let back = order.iter().rev().map(|to| (to, true));
    for (expected, shift) in forth.chain(back) {
        let from = open.focus();
        if !open.key_with_shift(VK_TAB.0, shift) {
            return false;
        }
        let keys = if shift { "Shift+Tab" } else { "Tab" };
        assert_eq!(
            open.focus().as_deref(),
            Some(*expected),
            "{keys} from {from:?} did not reach {expected}"
        );
    }
    true
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
    drop(open);

    let open = Open::new();
    open.key(VK_TAB.0);
    open.key(VK_ESCAPE.0);
    open.pump();
    assert!(!open.alive(), "Esc in the field did not close the window");
}

fn text_of(field: HWND) -> String {
    let mut buffer = [0_u16; 64];
    // SAFETY: a field of this thread, into a local of the length given.
    let length = unsafe { GetWindowTextW(field, &mut buffer) };
    String::from_utf16_lossy(&buffer[..usize::try_from(length).unwrap_or(0)])
}

/// The road of the program (blocker of 1.8.0, stand of 21.11): a key in a field of the shell goes
/// through the one loop of the program and is typed — its character comes of `TranslateMessage`
/// there. Enter in a field of one line presses the main button once, and its character is eaten.
#[test]
fn a_key_in_a_field_is_typed_through_the_loop_of_the_program() {
    let open = Open::new();
    let name = open.field(Id::Name);

    press_through_the_loop_of_the_program(name, VK_SPACE.0, 0x39);
    assert_eq!(
        text_of(name),
        " ",
        "space in the field, through the loop of the program"
    );
    assert_eq!(open.events(), [Event::Edited(Id::Name, " ".to_owned())]);

    press_through_the_loop_of_the_program(name, VK_RETURN.0, 0x1C);
    assert_eq!(
        open.events(),
        [Event::Pressed(Id::Close)],
        "Enter in the field, through the loop of the program: the main button, once"
    );
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
        FieldKind::Code { rtl: false },
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

/// **208.202 — a field of code whose text has a letter the monospace face lacks goes wholly in the
/// face of `body`.** Consolas has no Hebrew and no Arabic, and on Windows 10 the face put in their
/// place was markedly smaller (the stand of 21.11). The font follows the text as it is typed, both
/// ways: Latin — the monospace face; a Japanese letter, which no monospace face of the program has —
/// `body`; Hebrew — `body` under Consolas; Latin again — the monospace face again. Twice: in
/// Consolas, the face of Windows 10, and in the face of this system (Cascadia Mono has Hebrew).
///
/// The text is typed: a field of many lines tells no change of a text set from outside (`EN_CHANGE`
/// is not sent for `WM_SETTEXT`), and the window would put its own text back.
#[cfg(feature = "testing")]
#[test]
fn a_field_of_code_with_a_letter_its_monospace_face_lacks_goes_in_the_face_of_body() {
    use windows::Win32::UI::Controls::EM_SETSEL;
    use windows::Win32::UI::WindowsAndMessaging::WM_CHAR;

    const LATIN: &str = "Windows 10 22H2, 0409 040D";
    const JAPANESE: &str = "Windows 10 \u{2014} \u{65E5}\u{672C}\u{8A9E}";
    const HEBREW: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}, Windows 10";
    let mut seen = Vec::new();
    for answered in [Some("Consolas"), None] {
        host::answer_mono_face(answered);
        let open = Open::with_note(
            3,
            false,
            ("he-IL", true),
            ThemeSetting::Light,
            FieldKind::Code { rtl: true },
        );
        let (note, body) = (open.field(Id::Note), face_of(open.field(Id::Name)));
        let typed = |text: &str| {
            send(note, EM_SETSEL, 0, -1);
            for unit in text.encode_utf16() {
                send(note, WM_CHAR, usize::from(unit), 1);
            }
            send(open.hwnd, WM_PAINT, 0, 0);
            (text_of(note) == text, face_of(note))
        };
        let faces = [LATIN, JAPANESE, HEBREW, LATIN].map(typed);
        drop(open);
        seen.push((answered, body, faces));
    }
    host::answer_mono_face(None);
    println!("{seen:#?}");

    let mono = ["Cascadia Mono", "Consolas"];
    assert_eq!(seen[0].2[0].1, "Consolas", "the face answered");
    for (answered, body, [latin, japanese, hebrew, again]) in &seen {
        assert!(
            [latin, japanese, hebrew, again]
                .iter()
                .all(|(kept, _)| *kept),
            "the typing reached the window ({answered:?})"
        );
        assert!(!mono.contains(&body.as_str()), "body: {body}");
        assert!(
            mono.contains(&latin.1.as_str()),
            "Latin: the monospace face ({answered:?}), {}",
            latin.1
        );
        assert_eq!(
            japanese.1, *body,
            "a Japanese letter, which no monospace face of the program has ({answered:?})"
        );
        if latin.1 == "Consolas" {
            assert_eq!(hebrew.1, *body, "Hebrew under Consolas");
        }
        assert_eq!(
            again.1, latin.1,
            "Latin again: the monospace face again ({answered:?})"
        );
    }
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
            FieldKind::Code { rtl: false },
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

/// Whether the field of Windows `field` is set right to left: aligned right, read right to left.
fn right_to_left(field: HWND) -> (bool, bool) {
    // SAFETY: a window of this thread.
    let (style, extended) = unsafe {
        (
            GetWindowLongPtrW(field, GWL_STYLE),
            GetWindowLongPtrW(field, GWL_EXSTYLE),
        )
    };
    (
        style & ES_RIGHT as isize != 0,
        extended & WS_EX_RTLREADING.0 as isize != 0,
    )
}

/// **A field of code goes by the direction of its own text** (T-120-19, the decision of the
/// controller in C3): in a window in Hebrew a report gathered in Russian or English stays left to
/// right — the field of Windows is not aligned right and does not read right to left; a report in
/// a language read right to left goes as the window. The other fields go as the window.
#[test]
fn a_field_of_code_goes_by_the_direction_of_its_own_text() {
    for (rtl, expected) in [(false, (false, false)), (true, (true, true))] {
        let open = Open::with_note(
            3,
            false,
            ("he-IL", true),
            ThemeSetting::Light,
            FieldKind::Code { rtl },
        );
        assert_eq!(
            right_to_left(open.field(Id::Note)),
            expected,
            "a field of code with rtl {rtl} in a window in Hebrew"
        );
        assert_eq!(
            right_to_left(open.field(Id::Name)),
            (true, true),
            "a line in a window in Hebrew"
        );
    }
}

/// A window with one field that, while the host handles an edit of the field, asks the window for
/// the field's colours — as the field asks itself when Windows has it paint while the window is
/// busy (a font set with a redraw, the keyboard moved to it).
struct Busy {
    /// The window and its field, told by the test once both exist.
    pair: Rc<RefCell<Option<(HWND, HWND)>>>,
    /// What the window answered from inside the edit: the text, the background, the brush.
    answers: Rc<RefCell<Vec<(u32, u32, isize)>>>,
}

impl Content for Busy {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Field {
                id: Id::Name,
                label: "Process".to_owned(),
                value: String::new(),
                hint: String::new(),
                kind: FieldKind::Line,
            },
            buttons: Vec::new(),
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        if let (Event::Edited(Id::Name, _), Some((window, field))) = (&event, *self.pair.borrow()) {
            // SAFETY: a DC of the screen made and deleted here.
            let dc = unsafe { CreateCompatibleDC(None) };
            let brush = send(window, WM_CTLCOLOREDIT, dc.0 as usize, field.0 as isize);
            // SAFETY: as above.
            let (ink, fill) = unsafe { (GetTextColor(dc).0, GetBkColor(dc).0) };
            // SAFETY: as above.
            let _ = unsafe { DeleteDC(dc) };
            self.answers.borrow_mut().push((ink, fill, brush));
        }
        Reply::Stay
    }

    fn title(&self) -> String {
        "Busy".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("en-US".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Dark
    }
}

/// **AUDIT-180b п. 7 (решение 208.227)** — the field asks the window for its colours while the
/// window is busy with something of its own, and gets the colours of the theme, not the white of
/// Windows. Before, the answer came from the host, and a host already borrowed left the message to
/// `DefWindowProcW`: the brush of the system window, white in the dark theme — the field «Ожидали»
/// of the wizard white under its first line.
#[test]
fn a_field_gets_its_colours_while_the_window_is_busy() {
    let turn = ONE_WINDOW.lock().unwrap_or_else(PoisonError::into_inner);
    let pair = Rc::new(RefCell::new(None));
    let answers = Rc::new(RefCell::new(Vec::new()));
    let content = Busy {
        pair: pair.clone(),
        answers: answers.clone(),
    };
    let hwnd = host::create(content, None).expect("the window of the shell");
    send(hwnd, WM_PAINT, 0, 0);
    let field = host::probe(hwnd)
        .expect("a live host")
        .fields
        .into_iter()
        .find(|(name, _)| name == "Name")
        .map(|(_, field)| field)
        .expect("the field of Windows");
    *pair.borrow_mut() = Some((hwnd, field));

    // SAFETY: a field of this thread; the string lives across the call.
    let _ = unsafe { SetWindowTextW(field, &HSTRING::from("x")) };

    let colorref = |colour: Color| {
        let rgb = colour.get(Theme::Dark);
        u32::from(rgb.r) | (u32::from(rgb.g) << 8) | (u32::from(rgb.b) << 16)
    };
    let answered = answers.borrow().clone();
    let mut logbrush = LOGBRUSH::default();
    if let [(_, _, brush)] = answered[..] {
        // SAFETY: the brush the window answered with, read into a local of its size while the
        // window that owns it is still open.
        let _ = unsafe {
            GetObjectW(
                HGDIOBJ(brush as *mut _),
                std::mem::size_of::<LOGBRUSH>() as i32,
                Some((&raw mut logbrush).cast()),
            )
        };
    }
    send(hwnd, WM_CLOSE, 0, 0);
    drop(turn);

    let [(ink, fill, brush)] = answered[..] else {
        panic!("the edit was not delivered once: {answered:?}")
    };
    assert_eq!(
        fill,
        colorref(Color::Surface),
        "the background of the field"
    );
    assert_eq!(ink, colorref(Color::Ink), "the text of the field");
    assert_ne!(brush, 0, "the answer is not a brush");
    assert_eq!(
        logbrush.lbColor.0,
        colorref(Color::Surface),
        "the brush the field paints its background with"
    );
}

/// Whether `field` has its vertical scroll bar now, and the range and page Windows keeps for it.
fn scroll_bar_of(field: HWND) -> (bool, i32, u32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetScrollInfo, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_RANGE, WS_VSCROLL,
    };
    // SAFETY: a window of this thread; the out parameter is a local of its declared size.
    unsafe {
        let shown = GetWindowLongPtrW(field, GWL_STYLE) & WS_VSCROLL.0 as isize != 0;
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE,
            ..Default::default()
        };
        let _ = GetScrollInfo(field, SB_VERT, &mut info);
        (shown, info.nMax, info.nPage)
    }
}

/// **AUDIT-180b Н2 (решение 208.227)** — a text field shows the scroll bar of Windows only while its
/// text does not fit: none in an empty field or with a line typed, there with more lines than its
/// height holds — and working, its range beyond the page —, gone again when the text is cleared.
/// Red before: the bar of every text field was always there, light in the dark theme.
#[test]
fn a_text_field_shows_its_scroll_bar_only_while_its_text_does_not_fit() {
    use windows::Win32::UI::WindowsAndMessaging::WM_CHAR;

    let open = Open::with(3, false, ("ru-RU", false), ThemeSetting::Dark);
    let note = open.field(Id::Note);
    assert!(!scroll_bar_of(note).0, "an empty field has a scroll bar");

    send(note, WM_CHAR, usize::from(b'a'), 0);
    assert!(
        !scroll_bar_of(note).0,
        "a field with one letter has a scroll bar"
    );

    let long: String = (1..=60).map(|line| format!("line {line}\r\n")).collect();
    // SAFETY: a field of this thread; the string lives across the call.
    let _ = unsafe { SetWindowTextW(note, &HSTRING::from(long.as_str())) };
    let (shown, range, page) = scroll_bar_of(note);
    assert!(shown, "sixty lines and no scroll bar");
    assert!(
        range >= page as i32,
        "the bar does not scroll: range {range}, page {page}"
    );

    // SAFETY: as above.
    let _ = unsafe { SetWindowTextW(note, &HSTRING::from("")) };
    assert!(
        !scroll_bar_of(note).0,
        "the bar stays after the text is cleared"
    );
    send(open.hwnd, WM_CLOSE, 0, 0);
}
