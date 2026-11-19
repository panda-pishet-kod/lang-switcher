//! «О программе» on the shell — the window of the probe (stage Э120, T-120-9).
//!
//! The panel with the panda; over the title the line with the version (the only window that shows
//! it); the title; the line under it; the label «Как пользоваться» and its six items; the buttons
//! «От автора…» and «Закрыть». The strings are the program's own, from `res\lang`: nothing new.
//!
//! Until the owner's word on the probe it opens only in the testing build, by its own entry
//! ([`probe_main`]); the window the tray opens is the old one.

use crate::settings::{
    self, IDS_ABOUT_AUTHOR, IDS_ABOUT_CAPTION, IDS_ABOUT_HELP, IDS_ABOUT_HELP_1, IDS_ABOUT_HELP_2,
    IDS_ABOUT_HELP_3, IDS_ABOUT_HELP_4, IDS_ABOUT_HELP_5, IDS_ABOUT_HELP_6, IDS_ABOUT_LINE_1,
    IDS_ABOUT_VERSION, IDS_CLOSE, Language,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{Content, Reply};
use crate::ui::scene::{Art, Brand, ButtonKind, Event, Node, Role, Window, WindowKind};

/// The elements of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Author,
    Close,
}

/// «О программе» in a language, with the name of the key the help speaks of.
pub struct About {
    pub language: Language,
    pub key: String,
    pub theme: ThemeSetting,
}

impl About {
    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }
}

impl Content for About {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let help = [
            IDS_ABOUT_HELP_1,
            IDS_ABOUT_HELP_2,
            IDS_ABOUT_HELP_3,
            IDS_ABOUT_HELP_4,
            IDS_ABOUT_HELP_5,
            IDS_ABOUT_HELP_6,
        ];
        let version = self
            .line(IDS_ABOUT_VERSION)
            .replace("{0}", env!("CARGO_PKG_VERSION"));

        Window {
            kind: WindowKind::Ordinary,
            art: Some(Art::About),
            // ◆ The author and the site under the panda, as in the mock-up. ⚠ Before this window
            // reaches the product (T-120-13) the address moves to `letters::links`: criterion 4
            // finds network addresses in a Release image only there.
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: "panda-pishet-kod.dev".to_owned(),
            }),
            body: Node::Column(vec![
                Node::Header {
                    eyebrow: Some(version),
                    title: crate::APP_NAME.to_owned(),
                    subtitle: Some(self.line(IDS_ABOUT_LINE_1)),
                },
                Node::Rows(vec![
                    Node::Line(Role::Label, self.line(IDS_ABOUT_HELP)),
                    Node::Numbered(
                        help.iter()
                            .map(|id| self.line(*id).replace("{0}", &self.key))
                            .collect(),
                    ),
                ]),
            ]),
            buttons: vec![
                Node::Button {
                    id: Id::Author,
                    label: self.line(IDS_ABOUT_AUTHOR),
                    kind: ButtonKind::Normal,
                    browser: false,
                },
                Node::Button {
                    id: Id::Close,
                    label: self.line(IDS_CLOSE),
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
        self.line(IDS_ABOUT_CAPTION)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// The entry of the probe, testing build only: `LangSwitcher.exe --ui-probe-about` opens the window
/// in the language of the interface and runs until it is closed. For the instruments of the stand:
///
/// * `LANGSW_TESTING_UI_PROBE` — a file: the wall clock of the first finished frame,
///   `first_frame_unix_ms`, and of the window's end, `closed_unix_ms` (the stand compares them
///   with the moment it gave the command);
/// * `LANGSW_TESTING_UI_PROBE_WAIT_MS` — the process waits so long before it asks for the window,
///   so that its memory before the window can be read; `asked_unix_ms` is the moment it asks;
/// * `LANGSW_TESTING_UI_PROBE_CLOSE_MS` — the window closes itself after so many milliseconds;
/// * `LANGSW_TESTING_UI_PROBE_LINGER_MS` — the process stays so long after the window is gone, so
///   that its memory after closing can be read.
#[cfg(feature = "testing")]
pub fn probe_main() -> std::process::ExitCode {
    use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
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

    let about = About {
        language: settings::ui_language(),
        key: "Pause".to_owned(),
        theme: ThemeSetting::System,
    };
    if let Some(wait) = millis("LANGSW_TESTING_UI_PROBE_WAIT_MS") {
        std::thread::sleep(wait);
    }
    lines.push_str(&format!("asked_unix_ms={}\r\n", now()));
    let Ok(window) = crate::ui::host::open(about, None) else {
        return std::process::ExitCode::FAILURE;
    };
    lines.push_str(&format!("first_frame_unix_ms={}\r\n", now()));
    if let Some(path) = &report {
        let _ = std::fs::write(path, &lines);
    }

    if let Some(after) = millis("LANGSW_TESTING_UI_PROBE_CLOSE_MS") {
        let raw = window.0 as usize;
        std::thread::spawn(move || {
            std::thread::sleep(after);
            // SAFETY: a posted message to the probe's window; a window already gone ignores it.
            let _ =
                unsafe { PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
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

    lines.push_str(&format!("closed_unix_ms={}\r\n", now()));
    if let Some(path) = &report {
        let _ = std::fs::write(path, &lines);
    }
    if let Some(linger) = millis("LANGSW_TESTING_UI_PROBE_LINGER_MS") {
        std::thread::sleep(linger);
    }
    std::process::ExitCode::SUCCESS
}
