//! The probe of the interface shell (stage Э120, T-120-9), testing build only: the window
//! «О программе» on the shell opened alone by `LangSwitcher.exe --ui-probe-about`, with the marks
//! the instruments of the stand read. Not a window description: it holds the instruments.

use crate::settings;
use crate::theme::ThemeSetting;
use crate::ui::windows::about::About;

/// The entry of the probe, testing build only: `LangSwitcher.exe --ui-probe-about` opens the window
/// in the language of the interface and runs until it is closed. For the instruments of the stand
/// (`LANGSW_TESTING_UI_PROBE` names the file the marks go to, each with the wall clock):
///
/// * `LANGSW_TESTING_UI_PROBE_WAIT_MS` — wait so long before asking for the window (`asked_unix_ms`),
///   so that the memory before it can be read;
/// * `LANGSW_TESTING_UI_PROBE_CLOSE_MS` — the window closes itself after so many milliseconds;
/// * `LANGSW_TESTING_UI_PROBE_LINGER_MS` — after the window is gone, stay so long at each step:
///   closed (`closed_unix_ms`, the pages by kind, the references the factory of Direct2D still had),
///   then after `HeapCompact` of the process heap (`compacted_unix_ms`);
/// * `LANGSW_TESTING_UI_PROBE_TWICE` — then open the window a second time and close it again
///   (`asked2_unix_ms`, `first_frame2_unix_ms`, `closed2_unix_ms`).
pub fn run_about() -> std::process::ExitCode {
    use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use ::windows::Win32::System::Memory::{GetProcessHeap, HEAP_NONE, HeapCompact};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, IsWindow, MSG, PostMessageW, TranslateMessage, WM_CLOSE,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let now = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis())
    };
    let millis = |name: &str| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_millis)
    };
    let report = std::env::var_os("LANGSW_TESTING_UI_PROBE");
    let mut lines = String::new();
    let mut mark = |line: String| {
        lines.push_str(&line);
        lines.push_str("\r\n");
        if let Some(path) = &report {
            let _ = std::fs::write(path, &lines);
        }
    };
    let linger = || {
        if let Some(linger) = millis("LANGSW_TESTING_UI_PROBE_LINGER_MS") {
            std::thread::sleep(linger);
        }
    };

    // One window: asked, drawn, closed by itself after CLOSE_MS, its loop run to the end.
    let show = |mark: &mut dyn FnMut(String), suffix: &str| -> bool {
        mark(format!("asked{suffix}_unix_ms={}", now()));
        let about = About {
            language: settings::ui_language(),
            key: "Pause".to_owned(),
            theme: ThemeSetting::System,
        };
        let Ok(window) = crate::ui::host::open(about, None) else {
            return false;
        };
        mark(format!("first_frame{suffix}_unix_ms={}", now()));
        if let Some(after) = millis("LANGSW_TESTING_UI_PROBE_CLOSE_MS") {
            let raw = window.0 as usize;
            std::thread::spawn(move || {
                std::thread::sleep(after);
                // SAFETY: a posted message to the probe's window; a window already gone ignores it.
                let _ = unsafe {
                    PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0))
                };
            });
        }
        let mut message = MSG::default();
        // SAFETY: the message loop of this thread; it ends when the window is gone.
        while unsafe { IsWindow(Some(window)) }.as_bool()
            && unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool()
        {
            // SAFETY: a message just taken from the queue.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        mark(format!("closed{suffix}_unix_ms={}", now()));
        true
    };

    if let Some(wait) = millis("LANGSW_TESTING_UI_PROBE_WAIT_MS") {
        std::thread::sleep(wait);
    }
    mark(format!("pages_before={}", pages()));
    if !show(&mut mark, "") {
        return std::process::ExitCode::FAILURE;
    }
    mark(format!("pages_after={}", pages()));
    mark(format!(
        "d2d_factory_refs_left={}",
        crate::ui::canvas::FACTORY_REFS_LEFT.load(std::sync::atomic::Ordering::Relaxed)
    ));
    linger();

    // SAFETY: the heap of this process; only free blocks are given back.
    let largest =
        unsafe { GetProcessHeap() }.map_or(0, |heap| unsafe { HeapCompact(heap, HEAP_NONE) });
    mark(format!(
        "compacted_unix_ms={} largest_free={largest}",
        now()
    ));
    mark(format!("pages_compacted={}", pages()));
    linger();

    if std::env::var_os("LANGSW_TESTING_UI_PROBE_TWICE").is_some() && show(&mut mark, "2") {
        mark(format!("pages_after2={}", pages()));
        linger();
    }
    std::process::ExitCode::SUCCESS
}

/// The committed pages of this process by kind, in bytes — private, image, mapped — by walking the
/// address space with `VirtualQuery`: what a window leaves behind, by where it lives.
fn pages() -> String {
    use ::windows::Win32::System::Memory::{
        MEM_COMMIT, MEM_IMAGE, MEM_MAPPED, MEM_PRIVATE, MEMORY_BASIC_INFORMATION, VirtualQuery,
    };

    let (mut private, mut image, mut mapped) = (usize::MIN, usize::MIN, usize::MIN);
    let mut address = usize::MIN;
    loop {
        let mut region = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: a query of this process's own address space into a local.
        let answered = unsafe {
            VirtualQuery(
                Some(address as *const _),
                &mut region,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if answered == usize::MIN || region.RegionSize == usize::MIN {
            break;
        }
        if region.State == MEM_COMMIT {
            match region.Type {
                kind if kind == MEM_PRIVATE => private += region.RegionSize,
                kind if kind == MEM_IMAGE => image += region.RegionSize,
                kind if kind == MEM_MAPPED => mapped += region.RegionSize,
                _ => {}
            }
        }
        let Some(next) = (region.BaseAddress as usize).checked_add(region.RegionSize) else {
            break;
        };
        address = next;
    }
    format!("private:{private} image:{image} mapped:{mapped}")
}

/// Where the marks of a probe go: the file `LANGSW_TESTING_UI_PROBE` names, written whole each time.
fn report_to(lines: &str) {
    if let Some(path) = std::env::var_os("LANGSW_TESTING_UI_PROBE") {
        let _ = std::fs::write(path, lines);
    }
}

/// The code points of `text`, as the stand compares them (`U+0070 U+0440 …`).
fn code_points(text: &str) -> String {
    text.chars()
        .map(|c| format!("U+{:04X}", u32::from(c)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The wall clock in milliseconds.
fn unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// What the window of the fields' stand holds: the values typed, the row, the boxes.
#[derive(Default)]
struct FieldsState {
    name: String,
    note: String,
    selected: usize,
    first: usize,
    checks: Vec<bool>,
    /// The option of the choice field.
    language: usize,
    events: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldId {
    Name,
    Note,
    Layouts,
    Language,
    Apply,
    Close,
}

/// The window of the fields' stand: a line field, a text field, a list of 20 rows with check boxes
/// and two buttons, in Russian or — `LANGSW_TESTING_UI_PROBE_RTL=1` — in Hebrew, mirrored.
struct FieldsProbe {
    state: std::rc::Rc<std::cell::RefCell<FieldsState>>,
    rtl: bool,
}

/// The options of the choice field of the stand: languages by their own names.
const STAND_LANGUAGES: [&str; 8] = [
    "English",
    "Deutsch",
    "Français",
    "Română",
    "Русский",
    "Українська",
    "עברית",
    "日本語",
];

/// The rows of the list of the stand.
const STAND_ROWS: usize = 20;

/// The words of the fields' stand: Russian, or Hebrew for the mirror.
struct StandWords {
    title: &'static str,
    name: &'static str,
    note: &'static str,
    hint: &'static str,
    row: &'static str,
    language: &'static str,
    apply: &'static str,
    close: &'static str,
}

const RUSSIAN: StandWords = StandWords {
    title: "Поля",
    name: "Имя процесса",
    note: "Заметка",
    hint: "например game.exe",
    row: "Раскладка",
    language: "Язык",
    apply: "Применить",
    close: "Закрыть",
};

const HEBREW: StandWords = StandWords {
    title: "שדות",
    name: "שם תהליך",
    note: "הערה",
    hint: "למשל game.exe",
    row: "פריסה",
    language: "שפה",
    apply: "החל",
    close: "סגור",
};

impl crate::ui::host::Content for FieldsProbe {
    type Id = FieldId;

    fn describe(&self) -> crate::ui::scene::Window<FieldId> {
        use crate::ui::scene::{ButtonKind, FieldKind, ListItem, Node, Window, WindowKind};
        let state = self.state.borrow();
        let words = if self.rtl { &HEBREW } else { &RUSSIAN };
        let field = |id, label: &str, value: &str, kind| Node::Field {
            id,
            label: label.to_owned(),
            value: value.to_owned(),
            hint: words.hint.to_owned(),
            kind,
        };
        let button = |id, label: &str, kind| Node::Button {
            id,
            label: label.to_owned(),
            kind,
            browser: false,
        };
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Header {
                    eyebrow: Some("LANG SWITCHER · T-120-18".to_owned()),
                    title: words.title.to_owned(),
                    subtitle: None,
                },
                field(FieldId::Name, words.name, &state.name, FieldKind::Line),
                field(FieldId::Note, words.note, &state.note, FieldKind::Text),
                Node::List {
                    id: FieldId::Layouts,
                    rows: (0..STAND_ROWS)
                        .map(|index| ListItem {
                            label: format!("{} {}", words.row, index + 1),
                            check: state.checks.get(index).copied(),
                            mark: false,
                        })
                        .collect(),
                    selected: state.selected,
                    first: state.first,
                },
                Node::Choice {
                    id: FieldId::Language,
                    label: words.language.to_owned(),
                    options: STAND_LANGUAGES.map(str::to_owned).to_vec(),
                    selected: state.language,
                },
            ]),
            buttons: vec![
                button(FieldId::Apply, words.apply, ButtonKind::Normal),
                button(FieldId::Close, words.close, ButtonKind::Primary),
            ],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: crate::ui::scene::Event<FieldId>) -> crate::ui::host::Reply {
        use crate::ui::scene::Event;
        let mut state = self.state.borrow_mut();
        state.events += 1;
        match event {
            Event::Edited(FieldId::Name, text) => state.name = text,
            Event::Edited(FieldId::Note, text) => state.note = text,
            Event::Selected(FieldId::Layouts, row) => state.selected = row,
            Event::Selected(FieldId::Language, option) => state.language = option,
            Event::Scrolled(FieldId::Layouts, first) => state.first = first,
            Event::RowChecked(FieldId::Layouts, row, on) => {
                if let Some(check) = state.checks.get_mut(row) {
                    *check = on;
                }
            }
            Event::Pressed(FieldId::Close) => return crate::ui::host::Reply::Close,
            _ => {}
        }
        crate::ui::host::Reply::Stay
    }

    fn title(&self) -> String {
        "Lang Switcher — fields".to_owned()
    }

    fn language(&self) -> (String, bool) {
        let locale = if self.rtl { "he-IL" } else { "ru-RU" };
        (locale.to_owned(), self.rtl)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::System
    }
}

/// The entry of the fields' stand (T-120-18), testing build only: `LangSwitcher.exe
/// --ui-probe-fields` opens a window with two fields and a list and runs until it is closed. Every
/// 100 ms the whole state goes to the file `LANGSW_TESTING_UI_PROBE` names — the focus by the host,
/// the values typed as code points, the row, the background of the last frame, the layout of the
/// keyboard of this thread, the fields of Windows; the keys come from the stand, real ones.
pub fn run_fields() -> std::process::ExitCode {
    use ::windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetForegroundWindow, GetMessageW, IsWindow, MSG, SetTimer,
        TranslateMessage, WM_TIMER,
    };

    let state = std::rc::Rc::new(std::cell::RefCell::new(FieldsState {
        checks: (0..STAND_ROWS).map(|index| index % 2 == 0).collect(),
        ..FieldsState::default()
    }));
    let rtl = std::env::var("LANGSW_TESTING_UI_PROBE_RTL").is_ok_and(|value| value == "1");
    let content = FieldsProbe {
        state: state.clone(),
        rtl,
    };
    let Ok(window) = crate::ui::host::open(content, None) else {
        return std::process::ExitCode::FAILURE;
    };
    report_to(&format!(
        "opened_unix_ms={}\r\nwindow={}\r\n",
        unix_ms(),
        window.0 as usize
    ));

    // SAFETY: a timer of this thread, without a window: its ticks come to the loop below.
    let tick = unsafe { SetTimer(None, 0, 100, None) };
    let mut message = MSG::default();
    // SAFETY: the message loop of this thread; it ends when the window is gone.
    while unsafe { IsWindow(Some(window)) }.as_bool()
        && unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool()
    {
        if message.message == WM_TIMER && message.hwnd.is_invalid() && message.wParam.0 == tick {
            let Some(probe) = crate::ui::host::probe(window) else {
                continue;
            };
            let state = state.borrow();
            // SAFETY: reads the layout of this thread and the foreground window.
            let (layout, front) = unsafe { (GetKeyboardLayout(0), GetForegroundWindow()) };
            let fields: Vec<String> = probe
                .fields
                .iter()
                .map(|(id, hwnd)| format!("{id}:{}", hwnd.0 as usize))
                .collect();
            let checks: String = state
                .checks
                .iter()
                .map(|on| if *on { '1' } else { '0' })
                .collect();
            let lines = [
                format!("at_unix_ms={}", unix_ms()),
                format!("window={}", window.0 as usize),
                format!("focus={}", probe.focus.unwrap_or_default()),
                format!("name={}", code_points(&state.name)),
                format!("note={}", code_points(&state.note)),
                format!("selected={} first={}", state.selected, state.first),
                format!("checks={checks}"),
                format!("choice={}", state.language),
                format!("list_open={}", probe.dropdown.is_some()),
                format!("events={}", state.events),
                format!("frames={}", probe.frames),
                format!(
                    "background={}",
                    probe
                        .background
                        .map(|rgb| format!("#{:06X}", rgb.hex()))
                        .unwrap_or_default()
                ),
                format!("layout={:08X}", layout.0 as usize as u32),
                format!("foreground_is_window={}", front == window),
                format!("fields={}", fields.join(",")),
            ];
            report_to(&(lines.join("\r\n") + "\r\n"));
            continue;
        }
        // SAFETY: a message just taken from the queue.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    std::process::ExitCode::SUCCESS
}

/// What a popup of the stand shows: the shape of the window «Обновлено».
struct NoticeProbe;

impl crate::ui::host::Content for NoticeProbe {
    type Id = FieldId;

    fn describe(&self) -> crate::ui::scene::Window<FieldId> {
        use crate::ui::scene::{ButtonKind, Node, Role, Window, WindowKind};
        let button = |id, label: &str, kind| Node::Button {
            id,
            label: label.to_owned(),
            kind,
            browser: false,
        };
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(vec![
                Node::Line(Role::Eyebrow, "LANG SWITCHER".to_owned()),
                Node::Text(Role::Strong, "Обновлено до 1.8.0".to_owned()),
                Node::Text(
                    Role::Body,
                    "Флаг раскладки теперь стоит у часов. Щёлкните по нему, чтобы выбрать язык."
                        .to_owned(),
                ),
            ]),
            buttons: vec![
                button(FieldId::Close, "Закрыть", ButtonKind::Normal),
                button(FieldId::Apply, "Что нового", ButtonKind::Primary),
            ],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: crate::ui::scene::Event<FieldId>) -> crate::ui::host::Reply {
        match event {
            crate::ui::scene::Event::Pressed(_) => crate::ui::host::Reply::Close,
            _ => crate::ui::host::Reply::Stay,
        }
    }

    fn title(&self) -> String {
        "Lang Switcher — popup".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("ru-RU".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::System
    }
}

/// The entry of the popup's stand (T-120-14), testing build only: `LangSwitcher.exe
/// --ui-probe-popup` opens a popup in its corner — no anchor given, so measured by the taskbar —
/// and marks the foreground window before and after it shows, the popup's rectangle every tick of
/// its coming in (`x_<ms>=`), the work area, Windows' animation, and closes it after
/// `LANGSW_TESTING_UI_PROBE_CLOSE_MS`.
pub fn run_popup() -> std::process::ExitCode {
    use ::windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowRect, IsWindow, MSG,
        PostMessageW, SPI_GETCLIENTAREAANIMATION, SPI_GETWORKAREA,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetTimer, SystemParametersInfoW, TranslateMessage,
        WM_CLOSE, WM_TIMER,
    };

    let mut lines = String::new();
    let mut mark = |line: String| {
        lines.push_str(&line);
        lines.push_str("\r\n");
        report_to(&lines);
    };
    let rect_of = |hwnd: HWND| {
        let mut rect = RECT::default();
        // SAFETY: the out parameter is a local.
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        format!("{},{},{},{}", rect.left, rect.top, rect.right, rect.bottom)
    };

    let mut animation = ::windows::core::BOOL::from(true);
    let mut work = RECT::default();
    // SAFETY: out parameters that are locals of the sizes the actions write.
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut animation).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        );
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&raw mut work).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        );
    }
    // SAFETY: reads the foreground window.
    let before = unsafe { GetForegroundWindow() };
    mark(format!("asked_unix_ms={}", unix_ms()));
    mark(format!("animation={}", animation.as_bool()));
    mark(format!(
        "work_primary={},{},{},{}",
        work.left, work.top, work.right, work.bottom
    ));
    mark(format!("foreground_before={}", before.0 as usize));

    let Ok(popup) = crate::ui::host::open_popup(NoticeProbe, None) else {
        return std::process::ExitCode::FAILURE;
    };
    let shown = std::time::Instant::now();
    // SAFETY: reads the foreground window.
    let after = unsafe { GetForegroundWindow() };
    mark(format!("shown_unix_ms={}", unix_ms()));
    mark(format!("popup={}", popup.0 as usize));
    mark(format!("x_0={}", rect_of(popup)));
    mark(format!("foreground_after_show={}", after.0 as usize));
    mark(format!("focus_kept={}", after == before));

    if let Some(close) = std::env::var("LANGSW_TESTING_UI_PROBE_CLOSE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        let raw = popup.0 as usize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(close));
            // SAFETY: a posted message to the probe's window; a window already gone ignores it.
            let _ =
                unsafe { PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        });
    }

    // SAFETY: a timer of this thread, without a window: its ticks come to the loop below.
    let tick = unsafe { SetTimer(None, 0, 20, None) };
    let mut message = MSG::default();
    // SAFETY: the message loop of this thread; it ends when the popup is gone.
    while unsafe { IsWindow(Some(popup)) }.as_bool()
        && unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool()
    {
        if message.message == WM_TIMER && message.hwnd.is_invalid() && message.wParam.0 == tick {
            let ms = shown.elapsed().as_millis();
            if ms <= 700 {
                mark(format!("x_{ms}={}", rect_of(popup)));
            }
            continue;
        }
        // SAFETY: a message just taken from the queue.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    // SAFETY: reads the foreground window.
    let end = unsafe { GetForegroundWindow() };
    mark(format!("foreground_at_end={}", end.0 as usize));
    mark(format!("closed_unix_ms={}", unix_ms()));
    std::process::ExitCode::SUCCESS
}
