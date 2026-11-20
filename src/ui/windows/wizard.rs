//! The wizard «Написать автору» on the shell — FR-104, T-120-19: five steps for a trouble and
//! three for an idea, the thanks and «Не удалось отправить», in the elements of passport §6 — the
//! choice cards, the field of the program with «Взять из активного окна» beside it, the radio
//! buttons, the list of the layout and the key, the segmented switch «Повторяется», the four boxes
//! of what is attached, the text of the appeal and the contact, the quiet buttons. What it gathers,
//! what it sends and where is `letters::Wizard` and `letters::report`, as they were: not a sign
//! of the appeal changes.

use crate::letters::report::{Field, Repeat, Trouble};
use crate::letters::{self, Action, Answer, Deed, Forward, Post, Step, Wizard};
use crate::settings::{
    self, IDS_CLOSE, IDS_WIZARD_AND_PRESSED, IDS_WIZARD_ATTACH_FOOT, IDS_WIZARD_ATTACH_JOURNAL,
    IDS_WIZARD_ATTACH_JOURNAL_SUB, IDS_WIZARD_ATTACH_LAYOUTS, IDS_WIZARD_ATTACH_MACHINE,
    IDS_WIZARD_ATTACH_SETTINGS, IDS_WIZARD_BACK, IDS_WIZARD_CANCEL, IDS_WIZARD_CAPTION,
    IDS_WIZARD_CAPTURE, IDS_WIZARD_CAPTURE_COUNT, IDS_WIZARD_CAPTURE_NOTE, IDS_WIZARD_CARD_IDEA,
    IDS_WIZARD_CARD_IDEA_SUB, IDS_WIZARD_CARD_NOTHING, IDS_WIZARD_CARD_NOTHING_SUB,
    IDS_WIZARD_CARD_WRONG, IDS_WIZARD_CARD_WRONG_SUB, IDS_WIZARD_CONTACT, IDS_WIZARD_COPY_ADDRESS,
    IDS_WIZARD_COPY_TEXT, IDS_WIZARD_EXPECTED, IDS_WIZARD_FIELD_NORMAL, IDS_WIZARD_FIELD_PASSWORD,
    IDS_WIZARD_FIELD_PASSWORD_SUB, IDS_WIZARD_FIELD_UNKNOWN, IDS_WIZARD_GOT, IDS_WIZARD_IDEA_HELPS,
    IDS_WIZARD_IDEA_WHAT, IDS_WIZARD_MAIL_ADDRESS, IDS_WIZARD_PROGRAM, IDS_WIZARD_RECEIVED,
    IDS_WIZARD_REPEAT, IDS_WIZARD_REPEAT_ALWAYS, IDS_WIZARD_REPEAT_ONCE,
    IDS_WIZARD_REPEAT_SOMETIMES, IDS_WIZARD_SAVE, IDS_WIZARD_SENT_TEXT, IDS_WIZARD_STEP,
    IDS_WIZARD_TYPED_IN, IDS_WIZARD_WHERE_FIELD, Language,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{Content, Reply};
use crate::ui::scene::{
    Art, ButtonKind, Event, FieldKind, MenuState, Node, Role, Window, WindowKind,
};
use ::windows::Win32::Foundation::HWND;

/// The elements of the wizard, of all its pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    /// The cards of «Что случилось?».
    Wrong,
    Nothing,
    Idea,
    /// «Программа» and «Взять из активного окна».
    Program,
    Capture,
    /// The kind of field.
    Normal,
    Password,
    Unknown,
    /// «Набрали в раскладке», «и нажали», «Ожидали», «Получили», «Повторяется».
    Layout,
    Key,
    Expected,
    Got,
    Repeat,
    /// The four boxes of «Что приложить?».
    Machine,
    Layouts,
    Settings,
    Journal,
    /// «Что предлагаете?» and «Чем это поможет?».
    Proposal,
    Helps,
    /// The text of the appeal, «Как с вами связаться», and the quiet buttons.
    Appeal,
    Contact,
    CopyText,
    CopyAddress,
    Save,
    /// «Открыть канал» of the thanks (FR-104, решение 199.10).
    Channel,
    /// The row of the window.
    Cancel,
    Back,
    Next,
}

impl Id {
    /// A number of its own for each element — for the instrument of fitting.
    pub fn number(self) -> u16 {
        self as u16
    }
}

/// Every element: what goes grey while the appeal is on its way (T-107-3: it cannot be sent
/// twice or changed under the request).
pub const ALL: &[Id] = &[
    Id::Wrong,
    Id::Nothing,
    Id::Idea,
    Id::Program,
    Id::Capture,
    Id::Normal,
    Id::Password,
    Id::Unknown,
    Id::Layout,
    Id::Key,
    Id::Expected,
    Id::Got,
    Id::Repeat,
    Id::Machine,
    Id::Layouts,
    Id::Settings,
    Id::Journal,
    Id::Proposal,
    Id::Helps,
    Id::Appeal,
    Id::Contact,
    Id::CopyText,
    Id::CopyAddress,
    Id::Save,
    Id::Channel,
    Id::Cancel,
    Id::Back,
    Id::Next,
];

/// The options of «Повторяется», in the order of the switch.
const REPEATS: &[Repeat] = &[Repeat::Always, Repeat::Sometimes, Repeat::Once];

/// The wizard in a language: where it is and what it holds, and the theme.
pub struct Page {
    pub wizard: Wizard,
    pub language: Language,
    pub theme: ThemeSetting,
}

impl Page {
    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }

    /// The string `id` with `{0}`, `{1}` … filled in.
    fn filled(&self, id: u16, arguments: &[&str]) -> String {
        let mut words = self.line(id);
        for (index, argument) in arguments.iter().enumerate() {
            words = words.replace(&format!("{{{index}}}"), argument);
        }
        words
    }

    fn button(&self, id: Id, label: u16, kind: ButtonKind) -> Node<Id> {
        Node::Button {
            id,
            label: self.line(label),
            kind,
            browser: id == Id::Channel,
        }
    }

    fn field(&self, id: Id, label: u16, value: &str, kind: FieldKind) -> Node<Id> {
        Node::Field {
            id,
            label: self.line(label),
            value: value.to_owned(),
            hint: String::new(),
            kind,
        }
    }

    /// Whether the page is «Не удалось отправить».
    fn failed(&self) -> bool {
        self.wizard.current() == Step::Preview && self.wizard.post() == Post::Failed
    }

    /// The line over the title: «Написать автору · шаг N из M» on a step of the road, the name of
    /// the window alone on the thanks and on «Не удалось отправить», which are no steps of it.
    fn eyebrow(&self) -> String {
        let caption = self.line(IDS_WIZARD_CAPTION);
        if self.wizard.current() == Step::Done || self.failed() {
            return caption;
        }
        let number = (self.wizard.number() + 1).to_string();
        let total = self.wizard.road().len().to_string();
        format!(
            "{caption} · {}",
            self.filled(IDS_WIZARD_STEP, &[&number, &total])
        )
    }

    fn what(&self) -> Node<Id> {
        let chosen = self.wizard.draft().trouble;
        let card = |id, trouble, title, words| Node::ChoiceCard {
            id,
            title: self.line(title),
            words: self.line(words),
            selected: chosen == trouble,
        };
        Node::Rows(vec![
            card(
                Id::Wrong,
                Trouble::WrongResult,
                IDS_WIZARD_CARD_WRONG,
                IDS_WIZARD_CARD_WRONG_SUB,
            ),
            card(
                Id::Nothing,
                Trouble::NothingHappened,
                IDS_WIZARD_CARD_NOTHING,
                IDS_WIZARD_CARD_NOTHING_SUB,
            ),
            card(
                Id::Idea,
                Trouble::Idea,
                IDS_WIZARD_CARD_IDEA,
                IDS_WIZARD_CARD_IDEA_SUB,
            ),
        ])
    }

    fn place(&self) -> Node<Id> {
        let draft = self.wizard.draft();
        let capture = if self.wizard.countdown() > 0 {
            self.filled(
                IDS_WIZARD_CAPTURE_COUNT,
                &[&self.wizard.countdown().to_string()],
            )
        } else {
            self.line(IDS_WIZARD_CAPTURE)
        };
        let radio = |id, field, label| Node::Radio {
            id,
            label: self.line(label),
            on: draft.field == field,
        };
        Node::Rows(vec![
            Node::Row(vec![
                self.field(
                    Id::Program,
                    IDS_WIZARD_PROGRAM,
                    &draft.program,
                    FieldKind::Line,
                ),
                Node::Button {
                    id: Id::Capture,
                    label: capture,
                    kind: ButtonKind::Normal,
                    browser: false,
                },
            ]),
            Node::Text(Role::Small, self.line(IDS_WIZARD_CAPTURE_NOTE)),
            Node::Line(Role::FieldLabel, self.line(IDS_WIZARD_WHERE_FIELD)),
            radio(Id::Normal, Field::Normal, IDS_WIZARD_FIELD_NORMAL),
            radio(Id::Password, Field::Password, IDS_WIZARD_FIELD_PASSWORD),
            Node::Text(Role::Small, self.line(IDS_WIZARD_FIELD_PASSWORD_SUB)),
            radio(Id::Unknown, Field::Unknown, IDS_WIZARD_FIELD_UNKNOWN),
        ])
    }

    fn did(&self) -> Node<Id> {
        let draft = self.wizard.draft();
        let layouts = self.wizard.layouts();
        let chosen = layouts
            .iter()
            .position(|name| *name == draft.layout)
            .unwrap_or_default();
        let repeat = REPEATS
            .iter()
            .position(|each| *each == draft.repeat)
            .unwrap_or_default();
        Node::Rows(vec![
            Node::Pair(
                Box::new(Node::Choice {
                    id: Id::Layout,
                    label: self.line(IDS_WIZARD_TYPED_IN),
                    options: layouts.to_vec(),
                    selected: chosen,
                }),
                Box::new(self.field(
                    Id::Key,
                    IDS_WIZARD_AND_PRESSED,
                    &draft.hotkey,
                    FieldKind::Key { capturing: false },
                )),
            ),
            Node::Pair(
                Box::new(self.field(
                    Id::Expected,
                    IDS_WIZARD_EXPECTED,
                    &draft.expected,
                    FieldKind::Line,
                )),
                Box::new(self.field(Id::Got, IDS_WIZARD_GOT, &draft.got, FieldKind::Line)),
            ),
            Node::Segments {
                id: Id::Repeat,
                label: self.line(IDS_WIZARD_REPEAT),
                options: vec![
                    self.line(IDS_WIZARD_REPEAT_ALWAYS),
                    self.line(IDS_WIZARD_REPEAT_SOMETIMES),
                    self.line(IDS_WIZARD_REPEAT_ONCE),
                ],
                selected: repeat,
            },
        ])
    }

    fn attach(&self) -> Node<Id> {
        let facts = self.wizard.facts();
        let attach = self.wizard.draft().attach;
        let entries = facts.journal_entries.to_string();
        let mut rows = Vec::new();
        for (id, on, label, value) in [
            (
                Id::Machine,
                attach.machine,
                IDS_WIZARD_ATTACH_MACHINE,
                letters::report::machine_line(facts),
            ),
            (
                Id::Layouts,
                attach.layouts,
                IDS_WIZARD_ATTACH_LAYOUTS,
                facts.layouts.clone(),
            ),
            (
                Id::Settings,
                attach.settings,
                IDS_WIZARD_ATTACH_SETTINGS,
                facts.settings.clone(),
            ),
            (
                Id::Journal,
                attach.journal,
                IDS_WIZARD_ATTACH_JOURNAL,
                self.filled(IDS_WIZARD_ATTACH_JOURNAL_SUB, &[&entries]),
            ),
        ] {
            // The line of what goes into the appeal stands under the label of its box.
            rows.push(Node::Check {
                id,
                label: self.line(label),
                on,
                hint: Some(value),
            });
        }
        rows.push(Node::Text(Role::Small, self.line(IDS_WIZARD_ATTACH_FOOT)));
        Node::Rows(rows)
    }

    fn idea(&self) -> Node<Id> {
        let draft = self.wizard.draft();
        Node::Rows(vec![
            self.field(
                Id::Proposal,
                IDS_WIZARD_IDEA_WHAT,
                &draft.idea,
                FieldKind::Text,
            ),
            self.field(
                Id::Helps,
                IDS_WIZARD_IDEA_HELPS,
                &draft.helps,
                FieldKind::Text,
            ),
        ])
    }

    /// The quiet buttons and the line of what the last of them did.
    fn actions(&self, ids: &[(Id, u16)]) -> Vec<Node<Id>> {
        let mut rows = vec![Node::Row(
            ids.iter()
                .map(|(id, label)| self.button(*id, *label, ButtonKind::Quiet))
                .collect(),
        )];
        if !self.wizard.said().is_empty() {
            rows.push(Node::Text(Role::Small, self.wizard.said().to_owned()));
        }
        rows
    }

    fn preview(&self) -> Node<Id> {
        // The text of the appeal in the role `code` (passport §2), taking what the page leaves, and
        // in the direction of the language it was built in — data for the author, not words of
        // the window (the C3 of the controller: a Russian or English appeal reads left to right in
        // a mirrored window too).
        let mut rows = vec![
            Node::Fill(Box::new(Node::Field {
                id: Id::Appeal,
                label: String::new(),
                value: self.wizard.appeal().to_owned(),
                hint: String::new(),
                kind: FieldKind::Code {
                    rtl: self.wizard.appeal_reads_right_to_left(),
                },
            })),
            self.field(
                Id::Contact,
                IDS_WIZARD_CONTACT,
                self.wizard.contact(),
                FieldKind::Line,
            ),
        ];
        rows.extend(self.actions(&[
            (Id::CopyText, IDS_WIZARD_COPY_TEXT),
            (Id::Save, IDS_WIZARD_SAVE),
        ]));
        Node::Column(rows)
    }

    fn not_sent(&self) -> Node<Id> {
        let mut rows = vec![Node::Text(
            Role::Body,
            self.filled(IDS_WIZARD_MAIL_ADDRESS, &[letters::links::MAIL_ADDRESS]),
        )];
        rows.extend(self.actions(&[
            (Id::CopyText, IDS_WIZARD_COPY_TEXT),
            (Id::CopyAddress, IDS_WIZARD_COPY_ADDRESS),
            (Id::Save, IDS_WIZARD_SAVE),
        ]));
        Node::Rows(rows)
    }

    fn sent(&self) -> Node<Id> {
        Node::Rows(vec![
            Node::Text(Role::Body, self.line(IDS_WIZARD_SENT_TEXT)),
            // The page stands only after the site answered with success (`Wizard::take_answer`), so
            // the line says so as a fact (passport §6, the line of the state).
            Node::Status {
                state: MenuState::Working,
                words: self.line(IDS_WIZARD_RECEIVED),
            },
            Node::Row(vec![self.button(
                Id::Channel,
                settings::IDS_CHANNEL_OPEN,
                ButtonKind::Quiet,
            )]),
        ])
    }

    /// The row of the window: «Отмена», «Назад» from the second step, and the main button — the
    /// words of the step (`letters::step_words`); on the thanks «Закрыть» alone.
    fn buttons(&self) -> Vec<Node<Id>> {
        let step = self.wizard.current();
        if step == Step::Done {
            return vec![self.button(Id::Next, IDS_CLOSE, ButtonKind::Primary)];
        }
        let words = letters::step_words(step, self.wizard.post());
        let mut row = vec![self.button(Id::Cancel, IDS_WIZARD_CANCEL, ButtonKind::Normal)];
        if self.wizard.number() > 0 && !self.failed() {
            row.push(self.button(Id::Back, IDS_WIZARD_BACK, ButtonKind::Normal));
        }
        row.push(self.button(Id::Next, words.next, ButtonKind::Primary));
        row
    }
}

impl Content for Page {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let step = self.wizard.current();
        let words = letters::step_words(step, self.wizard.post());
        let failed = self.failed();
        let body = match step {
            Step::What => self.what(),
            Step::Where => self.place(),
            Step::Did => self.did(),
            Step::Attach => self.attach(),
            Step::Idea => self.idea(),
            Step::Preview if failed => self.not_sent(),
            Step::Preview => Node::Fill(Box::new(self.preview())),
            Step::Done => self.sent(),
        };
        let art = if step == Step::Done {
            Art::Thanks
        } else if failed {
            Art::Update
        } else {
            Art::Wizard
        };
        let disabled = if matches!(self.wizard.post(), Post::Sending(_)) {
            ALL.to_vec()
        } else {
            Vec::new()
        };
        let mut column = vec![Node::Header {
            eyebrow: Some(self.eyebrow()),
            title: self.line(words.title),
            subtitle: words.note.map(|note| self.line(note)),
        }];
        // The progress of the road (passport §6) on its steps; the thanks and «Не удалось
        // отправить» are no steps of it.
        if step != Step::Done && !failed {
            column.push(Node::Progress {
                total: self.wizard.road().len(),
                current: self.wizard.number(),
            });
        }
        column.push(body);
        Window {
            // The row of the wizard: «Отмена» at the edge of its own, the main button at the
            // other (passport §6).
            kind: WindowKind::Wizard,
            art: Some(art),
            brand: None,
            body: Node::Column(column),
            buttons: self.buttons(),
            disabled,
        }
    }

    /// The pure description answers nothing: the actions are the program's ([`Product`]).
    fn event(&mut self, _event: Event<Id>) -> Reply {
        Reply::Stay
    }

    fn title(&self) -> String {
        self.line(IDS_WIZARD_CAPTION)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// The eight pages as the instrument of fitting and the pictures see them —
/// `letters::wizard_pages`.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    letters::wizard_pages()
        .into_iter()
        .map(|(name, wizard)| {
            let page = Page {
                wizard,
                language,
                theme: ThemeSetting::Light,
            };
            (name.to_owned(), page.describe())
        })
        .collect()
}

/// The window `letters::open_wizard` opens: the description and what the program does with it.
pub struct Product {
    pub page: Page,
    /// The program's window: the owner of this one and the owner of the clipboard it writes.
    pub owner: HWND,
    /// The place of the window in the list of open windows of `letters`.
    pub _record: letters::ShellRecord,
}

impl Product {
    /// What a press, a choice or a box does to the wizard.
    fn choose(&mut self, id: Id, on: bool) {
        let wizard = &mut self.page.wizard;
        match id {
            Id::Wrong => wizard.choose(Trouble::WrongResult),
            Id::Nothing => wizard.choose(Trouble::NothingHappened),
            Id::Idea => wizard.choose(Trouble::Idea),
            Id::Normal => wizard.set_field(Field::Normal),
            Id::Password => wizard.set_field(Field::Password),
            Id::Unknown => wizard.set_field(Field::Unknown),
            Id::Machine | Id::Layouts | Id::Settings | Id::Journal => {
                let index = [Id::Machine, Id::Layouts, Id::Settings, Id::Journal]
                    .iter()
                    .position(|box_id| *box_id == id)
                    .unwrap_or_default();
                wizard.attach(index, on);
            }
            _ => {}
        }
    }
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.page.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        let language = self.page.language;
        let wizard = &mut self.page.wizard;
        match event {
            Event::Pressed(Id::Cancel) => return Reply::Close,
            Event::Pressed(Id::Next) => match wizard.forward() {
                Forward::Close => return Reply::Close,
                Forward::Send => {
                    wizard.send(language);
                }
                Forward::Moved => {}
            },
            Event::Pressed(Id::Back) => wizard.back(),
            Event::Pressed(Id::Capture) => {
                wizard.start_capture();
            }
            Event::Pressed(Id::CopyText) => wizard.copy_appeal(self.owner),
            Event::Pressed(Id::CopyAddress) => wizard.copy_address(self.owner),
            Event::Pressed(Id::Save) => wizard.save(),
            Event::Pressed(Id::Channel) => {
                let deed = Deed {
                    owner: self.owner,
                    news: None,
                    link: "",
                    language,
                };
                letters::act(&deed, Action::OpenChannel);
            }
            Event::Pressed(id) => self.choose(id, true),
            Event::Toggled(id, on) => self.choose(id, on),
            Event::Selected(Id::Repeat, index) => {
                if let Some(repeat) = REPEATS.get(index) {
                    wizard.set_repeat(*repeat);
                }
            }
            Event::Selected(Id::Layout, index) => wizard.choose_layout(index),
            Event::Edited(id, value) => match id {
                Id::Program => wizard.answer(Answer::Program, value),
                Id::Expected => wizard.answer(Answer::Expected, value),
                Id::Got => wizard.answer(Answer::Got, value),
                Id::Proposal => wizard.answer(Answer::Idea, value),
                Id::Helps => wizard.answer(Answer::Helps, value),
                Id::Appeal => wizard.edit_appeal(value),
                Id::Contact => wizard.edit_contact(value),
                _ => {}
            },
            Event::Timer => {
                if wizard.countdown() > 0 {
                    wizard.second(letters::wizard_window().unwrap_or(self.owner));
                }
                wizard.take_answer();
            }
            _ => {}
        }
        Reply::Stay
    }

    fn title(&self) -> String {
        self.page.title()
    }

    fn language(&self) -> (String, bool) {
        self.page.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.page.theme
    }

    fn timer(&self) -> Option<u32> {
        self.page.wizard.timer()
    }
}

impl Drop for Product {
    /// The answers go to `letters`, which keeps them only for a window that closed for a change of
    /// the language and opens again in the new one.
    fn drop(&mut self) {
        letters::wizard_closed(std::mem::replace(
            &mut self.page.wizard,
            letters::wizard_left(),
        ));
    }
}
