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
                link: Id::Site,
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
