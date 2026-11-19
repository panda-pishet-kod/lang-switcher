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
/// in the language of the interface and runs until it is closed. With `LANGSW_TESTING_UI_PROBE`
/// naming a file it writes there how long the first frame took, in milliseconds.
#[cfg(feature = "testing")]
pub fn probe_main() -> std::process::ExitCode {
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, IsWindow, MSG, TranslateMessage,
    };

    let started = std::time::Instant::now();
    let about = About {
        language: settings::ui_language(),
        key: "Pause".to_owned(),
        theme: ThemeSetting::System,
    };
    let Ok(window) = crate::ui::host::open(about, None) else {
        return std::process::ExitCode::FAILURE;
    };
    let first_frame = started.elapsed().as_millis();
    if let Some(path) = std::env::var_os("LANGSW_TESTING_UI_PROBE") {
        let _ = std::fs::write(path, format!("first_frame_ms={first_frame}\n"));
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
    std::process::ExitCode::SUCCESS
}
