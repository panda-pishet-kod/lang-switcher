//! «Последние письма» on the shell — FR-101: a card per letter the program keeps (the letter of
//! the update, when its row stands, and the three last news items), each with «Открыть письмо»
//! and, while it is unread, «Прочитано»; the line under them; «Закрыть». What the list holds is
//! `letters::list_entries`, as it was.

use crate::letters::{self, Entry, Letter};
use crate::settings::{
    self, IDS_CLOSE, IDS_LETTERS_CAPTION, IDS_LETTERS_FOOT, IDS_LETTERS_OPEN, IDS_LETTERS_SUB,
    IDS_LETTERS_TITLE, IDS_NEWS_READ_BUTTON, Language,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{Content, Reply};
use crate::ui::scene::{Art, Brand, ButtonKind, Event, Node, Role, Window, WindowKind};
use ::windows::Win32::Foundation::HWND;

/// The elements of the window: the two buttons of each card by the card's place in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Open(usize),
    Read(usize),
    Close,
    /// The address under the panda: the page of the program.
    Site,
}

impl Id {
    /// A number of its own for each element — for the instrument of fitting, which goes over the
    /// windows of every type at once.
    pub fn number(self) -> u16 {
        let place = |index: usize| u16::try_from(index).unwrap_or_default();
        match self {
            Id::Open(index) => place(index).saturating_add(place(index)),
            Id::Read(index) => place(index).saturating_add(place(index)).saturating_add(1),
            Id::Close => u16::MAX - 1,
            Id::Site => u16::MAX,
        }
    }
}

/// The list in a language.
pub struct LettersList {
    pub entries: Vec<Entry>,
    pub language: Language,
    pub theme: ThemeSetting,
}

/// The name of the author's site under the panda, as in «О программе».
fn site() -> String {
    let page = letters::links::PROGRAM_URL;
    let rest = page.strip_prefix("https://").unwrap_or(page);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

impl LettersList {
    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }

    /// The card of one letter: the date and whether it is read, the heading, the first line of
    /// what it said, the buttons.
    fn card(&self, index: usize, entry: &Entry) -> Node<Id> {
        let mut buttons = vec![Node::Button {
            id: Id::Open(index),
            label: self.line(IDS_LETTERS_OPEN),
            kind: ButtonKind::Normal,
            browser: false,
        }];
        if entry.unread {
            buttons.push(Node::Button {
                id: Id::Read(index),
                label: self.line(IDS_NEWS_READ_BUTTON),
                kind: ButtonKind::Normal,
                browser: false,
            });
        }
        Node::LetterCard {
            label: entry.meta.clone(),
            title: entry.title.clone(),
            words: entry.excerpt.clone(),
            buttons,
        }
    }
}

impl Content for LettersList {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        // The head of the mock-up (AUDIT-180b Н4, решение 208.227): the caption of the window as its
        // eyebrow, as «От автора» has it, then whose letters they are and what the list holds.
        let mut body = vec![Node::Header {
            eyebrow: Some(self.line(IDS_LETTERS_CAPTION)),
            title: self.line(IDS_LETTERS_TITLE),
            subtitle: Some(self.line(IDS_LETTERS_SUB)),
        }];
        // The cards half a block apart, as the mock-up of 208.136 п. 2 has them (`.lcards`: `.5em`).
        if !self.entries.is_empty() {
            body.push(Node::Rows(
                self.entries
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| self.card(index, entry))
                    .collect(),
            ));
        }
        body.push(Node::Text(Role::Small, self.line(IDS_LETTERS_FOOT)));
        Window {
            kind: WindowKind::Ordinary,
            // The panda of the news, as in the mock-up.
            art: Some(Art::News),
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: site(),
                link: Id::Site,
            }),
            body: Node::Column(body),
            buttons: vec![Node::Button {
                id: Id::Close,
                label: self.line(IDS_CLOSE),
                kind: ButtonKind::Primary,
                browser: false,
            }],
            disabled: Vec::new(),
        }
    }

    /// The pure description answers nothing: the actions are the program's ([`Product`]).
    fn event(&mut self, _event: Event<Id>) -> Reply {
        Reply::Stay
    }

    fn title(&self) -> String {
        self.line(IDS_LETTERS_CAPTION)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// The states the window shows, for the instrument of fitting: full — the letter of the update
/// and three news items, two of them unread — and empty.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let full = LettersList {
        entries: letters::sample_entries(language),
        language,
        theme: ThemeSetting::Light,
    };
    let empty = LettersList {
        entries: Vec::new(),
        language,
        theme: ThemeSetting::Light,
    };
    vec![
        ("letters list".to_owned(), full.describe()),
        ("letters list empty".to_owned(), empty.describe()),
    ]
}

/// The window `letters::open_list` opens: the description and what the program adds to it.
pub struct Product {
    pub list: LettersList,
    /// The program's window: the owner of this one and of the letters it opens.
    pub owner: HWND,
    /// The place of the window in the list of open windows of `letters`.
    pub _record: letters::ShellRecord,
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.list.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Pressed(Id::Close) => Reply::Close,
            // The letter of the card, by the road of the menu entry: shown and raised, the person
            // asked for it; nothing about it is recorded as a letter that came by itself.
            Event::Pressed(Id::Open(index)) => {
                if let Some(entry) = self.list.entries.get(index) {
                    let letter = entry.news.map_or(Letter::Update, Letter::News);
                    let item = letters::item_of(letter);
                    letters::show_letter(self.owner, letter, item.as_ref(), true);
                }
                Reply::Stay
            }
            // «Прочитано» of a card is «Прочитано» of its letter: the entry is read, the dot of
            // the icon follows, and the list is built again from what is stored — the card says
            // «прочитано» and loses the button.
            Event::Pressed(Id::Read(index)) => {
                let news = self.list.entries.get(index).and_then(|entry| entry.news);
                if let Some(id) = news {
                    letters::update_state(|state| letters::mark_read(state, id));
                    crate::tray::refresh_unread_mark();
                    self.list.entries = letters::list_entries_now();
                }
                Reply::Stay
            }
            Event::Pressed(Id::Site) => {
                letters::open_link(letters::links::program_url(self.list.language));
                Reply::Stay
            }
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        self.list.title()
    }

    fn language(&self) -> (String, bool) {
        self.list.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.list.theme()
    }
}
