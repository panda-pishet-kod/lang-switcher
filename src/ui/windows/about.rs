//! «О программе» on the shell — FR-92а.
//!
//! The panel with the panda; over the title the name of the program and its version (the only
//! window that shows the number); the title and the line under it; the label «Как пользоваться»
//! and its six items; the buttons «От автора…» and «Закрыть». The strings are the program's own,
//! from `res\lang`: nothing new.
//!
//! [`About`] is the description alone, which the instruments of the shell build in any language;
//! [`open`] is the window the menu of the icon opens, with the behaviour the program's window has
//! always had: one at a time, the commands of the menu it locks, the program's window for its
//! owner, a button on the task bar, Esc closes it.

use crate::settings::{
    self, AboutSession, Hotkey, IDS_ABOUT_AUTHOR, IDS_ABOUT_CAPTION, IDS_ABOUT_HELP,
    IDS_ABOUT_HELP_1, IDS_ABOUT_HELP_2, IDS_ABOUT_HELP_3, IDS_ABOUT_HELP_4, IDS_ABOUT_HELP_5,
    IDS_ABOUT_HELP_6, IDS_ABOUT_LINE_1, IDS_ABOUT_LINE_2, IDS_CLOSE, Language,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{self, Content, Reply};
use crate::ui::scene::{Art, Brand, ButtonKind, Event, Node, Role, Window, WindowKind};
use ::windows::Win32::Foundation::HWND;

/// The elements of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Author,
    Close,
    /// The address under the panda: the page of the program.
    Site,
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

/// The name of the author's site under the panda: the host of the program's page. Read out of
/// `letters::links`, because every network address of the program is declared there and the
/// acceptance of the release (criterion 4) looks for them in that one place.
fn site() -> String {
    let page = crate::letters::links::PROGRAM_URL;
    let rest = page.strip_prefix("https://").unwrap_or(page);
    rest.split('/').next().unwrap_or(rest).to_owned()
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
        // The version of the build is the version of the file: both come out of `Cargo.toml`,
        // and the release checks that the two agree (`tools\verify-version.ps1`).
        let eyebrow = format!("{} {}", crate::APP_NAME, env!("CARGO_PKG_VERSION"));

        Window {
            kind: WindowKind::Ordinary,
            art: Some(Art::About),
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: site(),
                link: Id::Site,
            }),
            body: Node::Column(vec![
                Node::Header {
                    eyebrow: Some(eyebrow),
                    title: self.line(IDS_ABOUT_LINE_1),
                    subtitle: Some(self.line(IDS_ABOUT_LINE_2)),
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
            Event::Pressed(Id::Site) => {
                let page = crate::letters::links::program_url(self.language);
                let _ = crate::settings::open_in_the_shell(page);
                Reply::Stay
            }
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

/// Every state the window shows — one — for the instrument of fitting.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let about = About {
        language,
        key: "Pause".to_owned(),
        theme: ThemeSetting::Light,
    };
    vec![("about".to_owned(), about.describe())]
}

/// The window the menu opens: the description and what the program adds to it.
struct Product {
    about: About,
    /// The program's window: the owner of this one and of «От автора», which opens from it.
    owner: HWND,
    /// The record [`settings::about_is_open`] reads. The host drops the content when the window
    /// is destroyed, and the record goes with it however the window was closed — Esc, the cross,
    /// a button, the end of the program.
    _session: AboutSession,
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.about.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            // FR-103: «От автора» opens on the program's window, not on this one, which closes.
            Event::Pressed(Id::Author) => {
                crate::letters::open_author(self.owner);
                Reply::Close
            }
            event => self.about.event(event),
        }
    }

    fn title(&self) -> String {
        self.about.title()
    }

    fn language(&self) -> (String, bool) {
        self.about.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.about.theme()
    }
}

/// Opens «О программе» on `owner`, the program's window, under the setting «Оформление» `theme`,
/// its help naming the key `hotkey` makes; answers the window, or none.
///
/// One window at a time: while «О программе» or the settings dialog is up nothing is made — the
/// menu greys the command for the same reason (`tray::dialog_locks_command`), and this is the
/// door behind it. A window Windows refused is journaled; the user is told by its absence.
pub fn open(owner: HWND, theme: ThemeSetting, hotkey: &Hotkey) -> Option<HWND> {
    if settings::about_is_open() || settings::dialog_is_open() {
        return None;
    }

    let product = Product {
        about: About {
            language: settings::ui_language(),
            key: settings::effective_hotkey_name(hotkey),
            theme,
        },
        owner,
        _session: AboutSession::open(),
    };

    #[cfg(feature = "testing")]
    let made = if testing::hidden() {
        host::create(product, Some(owner))
    } else {
        host::open(product, Some(owner))
    };
    #[cfg(not(feature = "testing"))]
    let made = host::open(product, Some(owner));

    match made {
        Ok(window) => {
            // Nothing posted is dispatched between the window's making and this line, so no
            // command of the menu can arrive while the window is up and the record still empty.
            AboutSession::record(window);
            Some(window)
        }
        Err(error) => {
            crate::app::report_non_critical("CreateWindowExW", &error);
            None
        }
    }
}

/// The door of the tests: [`open`] makes the window as the product makes it and leaves it hidden.
#[cfg(feature = "testing")]
pub mod testing {
    use std::cell::Cell;

    thread_local! {
        static HIDDEN: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn hidden() -> bool {
        HIDDEN.with(Cell::get)
    }

    /// Runs `f` with the window of [`super::open`] left hidden on this thread.
    pub fn hidden_while<R>(f: impl FnOnce() -> R) -> R {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                HIDDEN.with(|hidden| hidden.set(false));
            }
        }

        HIDDEN.with(|hidden| hidden.set(true));
        let _reset = Reset;
        f()
    }
}
