//! «От автора» on the shell — FR-103 (T-120-16 for the letters, T-120-17 for this window): the
//! head of «О программе» with «От автора» over it and no version; the pair of action cards — to
//! support the author (the page of support and the channel, in the browser) and to write to him
//! (the wizard); the card «Новости и обновления» — the two lines of the state of the feed, «Последние
//! письма» and the page of downloads, the switch «Сообщать об обновлениях программы», which acts at
//! once; «Закрыть». What it says is `letters::author_view`, as it was.

use crate::letters::{self, Action, AuthorView, Deed};
use crate::settings::{
    self, IDS_AUTHOR_CAPTION, IDS_CHANNEL_OPEN, IDS_CLOSE, IDS_FEEDBACK_PANEL, IDS_NEWS_DOWNLOAD,
    IDS_NEWS_LETTERS, IDS_NEWS_PANEL, IDS_SUPPORT_OPEN, IDS_SUPPORT_PANEL, IDS_UPDATES_SWITCH,
    IDS_WRITE_TO_AUTHOR, Language,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{Content, Reply};
use crate::ui::scene::{Art, Brand, ButtonKind, Event, Node, Role, Surface, Window, WindowKind};
use ::windows::Win32::Foundation::HWND;

/// The elements of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Support,
    Channel,
    Write,
    Letters,
    Download,
    /// «Сообщать об обновлениях программы».
    Switch,
    Close,
    /// The address under the panda: the page of the program.
    Site,
}

impl Id {
    /// What the button does — `None` for the switch, which writes a setting.
    fn action(self) -> Option<Action> {
        match self {
            Id::Support => Some(Action::OpenSupport),
            Id::Channel => Some(Action::OpenChannel),
            Id::Write => Some(Action::OpenWizard),
            Id::Letters => Some(Action::OpenLetters),
            Id::Download => Some(Action::OpenDownload),
            Id::Close => Some(Action::Close),
            Id::Site => Some(Action::OpenProgram),
            Id::Switch => None,
        }
    }

    /// A number of its own for each element — for the instrument of fitting.
    pub fn number(self) -> u16 {
        self as u16
    }
}

/// «От автора» in a language: what it says and the theme.
pub struct Author {
    pub view: AuthorView,
    pub language: Language,
    pub theme: ThemeSetting,
}

/// The name of the author's site under the panda, as in «О программе».
fn site() -> String {
    let page = letters::links::PROGRAM_URL;
    let rest = page.strip_prefix("https://").unwrap_or(page);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

impl Author {
    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }

    /// A button of the window; ↗ when it opens the browser (passport §6).
    fn button(&self, id: Id, label: u16, kind: ButtonKind) -> Node<Id> {
        Node::Button {
            id,
            label: self.line(label),
            kind,
            browser: id.action().is_some_and(Action::opens_browser),
        }
    }

    /// The card «Новости и обновления»: the state of the feed and of the version, the two
    /// buttons, the switch.
    fn news(&self) -> Node<Id> {
        let view = &self.view;
        let mut rows = vec![Node::Line(Role::Strong, self.line(IDS_NEWS_PANEL))];
        // ⚠ The two lines of the state, until the element «строка состояния» is in the scene (a
        // line to `e120-core`): plain sentences.
        for line in [&view.feed_about, &view.feed_state, &view.version_state] {
            if !line.is_empty() {
                rows.push(Node::Text(Role::Body, line.clone()));
            }
        }
        let mut buttons = Vec::new();
        // «Последние письма» while it has something to show; the page of downloads always — its
        // address is a constant of this build (task T-78-2).
        if view.letters {
            buttons.push(self.button(Id::Letters, IDS_NEWS_LETTERS, ButtonKind::Normal));
        }
        buttons.push(self.button(Id::Download, IDS_NEWS_DOWNLOAD, ButtonKind::Normal));
        rows.push(Node::Buttons(buttons));
        if let Some(on) = view.switch {
            rows.push(Node::Toggle {
                id: Id::Switch,
                label: self.line(IDS_UPDATES_SWITCH),
                hint: (!view.switch_note.is_empty()).then(|| view.switch_note.clone()),
                on,
                badge: None,
            });
        }
        if !view.file_only.is_empty() {
            rows.push(Node::Text(Role::Small, view.file_only.clone()));
        }
        Node::Surface(Surface::Block, Box::new(Node::Rows(rows)))
    }
}

impl Content for Author {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let view = &self.view;
        let support = Node::Action {
            title: self.line(IDS_SUPPORT_PANEL),
            words: view.author_text.clone(),
            buttons: vec![
                self.button(Id::Support, IDS_SUPPORT_OPEN, ButtonKind::Normal),
                self.button(Id::Channel, IDS_CHANNEL_OPEN, ButtonKind::Normal),
            ],
        };
        let feedback = Node::Action {
            title: self.line(IDS_FEEDBACK_PANEL),
            words: view.feedback_text.clone(),
            buttons: vec![self.button(Id::Write, IDS_WRITE_TO_AUTHOR, ButtonKind::Normal)],
        };
        // A button whose address is still a placeholder is drawn and not pressed (П7, П8).
        let disabled = [
            (Id::Support, letters::links::SUPPORT_URL),
            (Id::Channel, letters::links::CHANNEL_URL),
            (Id::Download, letters::links::DOWNLOAD_URL),
        ]
        .into_iter()
        .filter(|(_, address)| letters::links::is_placeholder(address))
        .map(|(id, _)| id)
        .collect();
        Window {
            kind: WindowKind::Ordinary,
            art: Some(Art::Thanks),
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: site(),
                link: Id::Site,
            }),
            body: Node::Column(vec![
                // The head of «О программе» (task T-81-3): «От автора» over it, and no number — the
                // only window with a version over its title is «О программе».
                Node::Header {
                    eyebrow: Some(self.line(IDS_AUTHOR_CAPTION)),
                    title: view.line_1.clone(),
                    subtitle: Some(view.line_2.clone()),
                },
                Node::Pair(Box::new(support), Box::new(feedback)),
                self.news(),
            ]),
            buttons: vec![self.button(Id::Close, IDS_CLOSE, ButtonKind::Primary)],
            disabled,
        }
    }

    /// The pure description answers nothing: the actions are the program's ([`Product`]).
    fn event(&mut self, _event: Event<Id>) -> Reply {
        Reply::Stay
    }

    fn title(&self) -> String {
        self.line(IDS_AUTHOR_CAPTION)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// «От автора» as the instrument of fitting sees it: a machine whose feed has been read, with an
/// update waiting and letters to show — the fullest the window gets — and the switch on.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let author = Author {
        view: letters::author_view_sample(language),
        language,
        theme: ThemeSetting::Light,
    };
    vec![("author".to_owned(), author.describe())]
}

/// The window `letters::open_author` opens: the description and what the program adds to it.
pub struct Product {
    pub author: Author,
    /// The program's window: the owner of this one and of the windows its buttons open.
    pub owner: HWND,
    /// The place of the window in the list of open windows of `letters`.
    pub _record: letters::ShellRecord,
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.author.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            // The switch acts at once (208.169): the setting is written, and the window shows what
            // was written — FR-102, task T-70-1 (`AuthorView::follow_feed`).
            Event::Toggled(Id::Switch, on) => {
                letters::update_state(|state| state.release_window = on);
                self.author.view.follow_feed(&letters::state_now());
                Reply::Stay
            }
            Event::Pressed(id) => {
                let Some(action) = id.action() else {
                    return Reply::Stay;
                };
                let deed = Deed {
                    owner: self.owner,
                    news: None,
                    link: "",
                    language: self.author.language,
                };
                if letters::act(&deed, action) {
                    Reply::Close
                } else {
                    Reply::Stay
                }
            }
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        self.author.title()
    }

    fn language(&self) -> (String, bool) {
        self.author.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.author.theme()
    }
}
