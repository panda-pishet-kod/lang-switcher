//! The letters on the shell — FR-101: «Привет», «Спасибо», «Новость» and «Обновление» are one
//! frame with different content (T-120-16), the parts of [`LetterPlan`] in its order: the head,
//! the notice, the lead, the example with the key, the paragraph, the author's entry, the numbered
//! list, the pair of action cards, the link, the line under them; the row of buttons, the main one
//! the rightmost. When a letter comes, how it is put off and marked read is `letters.rs`'s, as it
//! was.
//!
//! [`LetterView`] is the description alone, which the instruments of the shell build in any
//! language; [`Product`] is the window `letters::show_letter` opens, with what the program adds:
//! its owner, the entry it is about, its place in the list of open windows.

use crate::letters::{self, Action, Button, Deed, Letter, LetterPlan};
use crate::settings::Language;
use crate::theme::{KEY_PLACEHOLDER, ThemeSetting};
use crate::ui::host::{Content, Reply};
use crate::ui::scene::{
    Art, Brand, ButtonKind, Event, Node, OnPicture, Role, Surface, Window, WindowKind,
};
use ::windows::Win32::Foundation::HWND;

/// The elements of a letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    /// A button or the link: what it does. One element per action in a letter — no letter has
    /// two buttons doing the same.
    Do(Action),
    /// The address under the panda: the page of the program.
    Site,
}

impl Id {
    /// A number of its own for each element — for the instrument of fitting, which goes over the
    /// windows of every type at once.
    pub fn number(self) -> u16 {
        match self {
            Id::Do(action) => action as u16,
            Id::Site => u16::MAX,
        }
    }
}

/// A letter in a language: what it says, the name of the key it speaks of, the theme.
pub struct LetterView {
    pub letter: Letter,
    pub plan: LetterPlan,
    /// The name of the key, in place of [`KEY_PLACEHOLDER`].
    pub key: String,
    /// The frame of the demonstration of «Привет» (`letters::demo_frame`): the window counts it
    /// once it is given a tick (T-120-25); without one it stands at `letters::DEMO_STILL`, the
    /// final frame.
    pub tick: u32,
    pub language: Language,
    pub theme: ThemeSetting,
}

/// The picture of each letter: its own drawing of the panda.
fn art_of(letter: Letter) -> Art {
    match letter {
        Letter::Welcome => Art::Hello,
        Letter::Thanks => Art::Thanks,
        Letter::News(_) => Art::News,
        Letter::Update => Art::Update,
    }
}

/// The name of the author's site under the panda — as in «О программе»: the host of the page of
/// the program, read out of `letters::links`.
fn site() -> String {
    let page = letters::links::PROGRAM_URL;
    let rest = page.strip_prefix("https://").unwrap_or(page);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

/// A button of the plan as an element: its kind, its sign ↗ when it opens the browser.
fn button(button: &Button, kind: ButtonKind) -> Node<Id> {
    Node::Button {
        id: Id::Do(button.action),
        label: button.label.clone(),
        kind,
        browser: button.action.opens_browser(),
    }
}

impl LetterView {
    /// A sentence with the name of the key in its place.
    fn keyed(&self, sentence: &str) -> String {
        sentence.replace(KEY_PLACEHOLDER, &self.key)
    }

    /// What the demonstration of «Привет» shows at this frame — `letters::demo_frame`, the same
    /// table the old window animated; `None` for a letter without it. The picture of the letter
    /// takes the word and the key from here once it has its anchors (T-120-25).
    pub fn demo(&self) -> Option<letters::DemoFrame> {
        self.plan
            .demo
            .then(|| letters::demo_frame(self.tick, self.plan.demo_pair))
    }

    /// The example of «Привет»: the word typed in the wrong layout, the key, the word it becomes.
    /// The line stands in the text at every width of the window; its words come into the picture
    /// when the picture has its anchors (T-120-25).
    fn example(&self) -> Node<Id> {
        let pair = self.plan.demo_pair;
        Node::Example {
            typed: pair.typed.to_owned(),
            key: self.key.clone(),
            fixed: pair.fixed.to_owned(),
        }
    }

    /// The demonstration at this frame: the word as much as it is typed on the screen of the laptop
    /// and the key, lit while it is down — over the picture in a wide window; in a narrow one the
    /// line of [`Self::example`] in the text instead (T-120-25 п. 4).
    fn live_example(&self) -> Node<Id> {
        let Some(frame) = self.demo() else {
            return self.example();
        };
        let keys = self
            .key
            .split(crate::settings::DOUBLE_TAP_JOINER)
            .map(str::to_owned)
            .collect();
        Node::OverPicture {
            over: vec![
                (
                    "screen",
                    OnPicture::Word {
                        word: frame.word_shown(),
                        fixed: frame.converted,
                    },
                ),
                (
                    "key",
                    OnPicture::Keys {
                        labels: keys,
                        pressed: frame.pressed,
                    },
                ),
            ],
            otherwise: Box::new(self.example()),
        }
    }

    /// The body of the letter, part by part.
    fn body(&self) -> Vec<Node<Id>> {
        let plan = &self.plan;
        let some = |words: &str| (!words.is_empty()).then(|| words.to_owned());
        let mut parts = vec![Node::Header {
            eyebrow: some(&plan.eyebrow),
            title: plan.title.clone(),
            subtitle: some(&plan.subtitle),
        }];

        if !plan.note.is_empty() {
            parts.push(Node::Surface(
                Surface::Note,
                Box::new(Node::Text(Role::Body, plan.note.clone())),
            ));
        }
        if !plan.lead.is_empty() {
            parts.push(Node::Text(Role::Lead, plan.lead.clone()));
        }
        if plan.demo {
            let mut example = vec![self.live_example()];
            if !plan.demo_caption.is_empty() {
                example.push(Node::Text(Role::Small, self.keyed(&plan.demo_caption)));
            }
            parts.push(Node::Rows(example));
        }
        if !plan.para.is_empty() {
            parts.push(Node::Text(Role::Body, plan.para.clone()));
        }

        // The author's words: under «Что нового в X» for a release, under the heading of the
        // entry for a news item.
        let mut entry = Vec::new();
        if !plan.whats_new.is_empty() {
            entry.push(Node::Line(Role::Label, plan.whats_new.clone()));
        } else if !plan.panel_title.is_empty() {
            entry.push(Node::Text(Role::Strong, plan.panel_title.clone()));
        }
        if !plan.panel_text.is_empty() {
            entry.push(Node::Text(Role::Body, plan.panel_text.clone()));
        }
        if !entry.is_empty() {
            parts.push(Node::Rows(entry));
        }

        if !plan.rows.is_empty() {
            let mut list = Vec::new();
            if !plan.panel.is_empty() {
                list.push(Node::Line(Role::Label, plan.panel.clone()));
            }
            let (numbered, plain): (Vec<_>, Vec<_>) =
                plan.rows.iter().partition(|row| !row.marker.is_empty());
            list.push(Node::Numbered(
                numbered.iter().map(|row| self.keyed(row.words())).collect(),
            ));
            list.extend(
                plain
                    .iter()
                    .map(|row| Node::Text(Role::Body, self.keyed(row.words()))),
            );
            parts.push(Node::Rows(list));
        }

        let mut cards = plan.cards.iter().map(|card| Node::Action {
            title: card.title.clone(),
            words: card.words.clone(),
            buttons: vec![button(&card.button, ButtonKind::Normal)],
        });
        // The pair stands side by side while a half is wide enough (the rule of the pair), one
        // under the other when it is not — the layout decides, the letter only says «a pair».
        match (cards.next(), cards.next()) {
            (Some(first), Some(second)) => {
                parts.push(Node::Pair(Box::new(first), Box::new(second)))
            }
            (Some(only), None) => parts.push(only),
            _ => {}
        }

        if let Some(link) = &plan.link {
            parts.push(Node::Link {
                id: Id::Do(link.action),
                words: link.label.clone(),
            });
        }
        if !plan.foot.is_empty() {
            parts.push(Node::Text(Role::Small, plan.foot.clone()));
        }
        parts
    }
}

impl Content for LetterView {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let plan = &self.plan;
        let mut buttons: Vec<Node<Id>> = [&plan.left, &plan.right]
            .into_iter()
            .flatten()
            .map(|plain| button(plain, ButtonKind::Normal))
            .collect();
        buttons.extend(
            plan.accent
                .iter()
                .map(|main| button(main, ButtonKind::Primary)),
        );
        Window {
            kind: WindowKind::Ordinary,
            art: Some(art_of(self.letter)),
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: site(),
                link: Id::Site,
            }),
            body: Node::Column(self.body()),
            buttons,
            // A button whose address is still a placeholder or not one of the program's own is
            // drawn and not pressed (task T-41-1, finding С52): it keeps its place and its word.
            disabled: plan
                .all_buttons()
                .filter(|button| !button.enabled)
                .map(|button| Id::Do(button.action))
                .collect(),
        }
    }

    /// The tick of the window moves the demonstration of «Привет» one frame on; the actions of the
    /// buttons are the program's ([`Product`]).
    fn event(&mut self, event: Event<Id>) -> Reply {
        if event == Event::Tick {
            self.tick = self.tick.wrapping_add(1);
        }
        Reply::Stay
    }

    /// «Привет» ticks with the frames of its demonstration (`letters::DEMO_TICK_MS`, T-120-25);
    /// every other letter stands still — the rule of «в покое — ноль».
    fn tick(&self) -> Option<u32> {
        self.plan.demo.then_some(letters::DEMO_TICK_MS)
    }

    fn title(&self) -> String {
        self.plan.caption.clone()
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }
}

/// Every letter as the instrument of fitting sees it: built in `language`, the letters out of
/// the feed for `letters::sample_entry`, «Спасибо» with its snooze.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let item = letters::sample_entry();
    let context = |snooze_offered| letters::PlanContext {
        version: "1.7.0".to_owned(),
        hotkey: String::new(),
        snooze_offered,
        item: Some(&item),
    };
    [
        ("letter hello", Letter::Welcome),
        ("letter thanks", Letter::Thanks),
        ("letter news", Letter::News(item.id)),
        ("letter update", Letter::Update),
    ]
    .into_iter()
    .map(|(name, letter)| {
        let view = LetterView {
            letter,
            plan: letters::plan_in(language, letter, &context(true)),
            key: "Pause".to_owned(),
            tick: letters::DEMO_STILL,
            language,
            theme: ThemeSetting::Light,
        };
        (name.to_owned(), view.describe())
    })
    .collect()
}

/// The window `letters::show_letter` opens: the description and what the program adds to it.
pub struct Product {
    pub view: LetterView,
    /// The program's window: the owner of this one and of the windows its buttons open.
    pub owner: HWND,
    /// The identifier of the entry a «Новость» is about — what «Прочитано» marks.
    pub news: Option<u64>,
    /// The link the entry carried, for «Читать на сайте».
    pub link: String,
    /// The place of the window in the list of open windows of `letters`; it leaves the list when
    /// the host drops the content, however the window was closed.
    pub _record: letters::ShellRecord,
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.view.describe()
    }

    fn tick(&self) -> Option<u32> {
        self.view.tick()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Pressed(Id::Do(action)) => {
                let deed = Deed {
                    owner: self.owner,
                    news: self.news,
                    link: &self.link,
                    language: self.view.language,
                };
                if letters::act(&deed, action) {
                    Reply::Close
                } else {
                    Reply::Stay
                }
            }
            // The address under the panda opens the page of the program in the language of the
            // window — through the gate of `letters::open_link`, as in «О программе».
            Event::Pressed(Id::Site) => {
                letters::open_link(letters::links::program_url(self.view.language));
                Reply::Stay
            }
            event => self.view.event(event),
        }
    }

    fn title(&self) -> String {
        self.view.title()
    }

    fn language(&self) -> (String, bool) {
        self.view.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.view.theme()
    }
}
