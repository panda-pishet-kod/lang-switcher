//! The scene: a window **described as data**, and what the shell answers back.
//!
//! A window of the shell is a tree of [`Node`]s — a column, a pair of blocks, a header, a toggle, a
//! row of buttons — built anew from the window's own state whenever that state changes. The tree
//! says *what* is shown and in which order; where it stands, how large and in what colour is decided
//! by the layout and the table of styles, never by the description (`tests\ui_guard.rs`, rule 2).
//!
//! Every interactive node carries an identifier of the window's own type `I` — an enumeration of
//! that window (SEC-05: an identifier from elsewhere has no meaning here). The shell answers with an
//! [`Event`] naming it.
//!
//! The order of the nodes is the logical order: the order of reading and of the Tab key, the same
//! in a mirrored window (passport §7).
//!
//! Knows nothing of Win32.

/// The roles of text, as a window description names them: the vocabulary of the passport §2, not its
/// values (a description does not reach `tokens`, `testsI_guard.rs` rule 2).
pub use crate::ui::tokens::Role;

/// The look of a button: one per kind (passport §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Dark, one per window, the rightmost of its row; Enter.
    Primary,
    /// Light with a frame.
    Normal,
    /// No frame and no fill: small actions.
    Quiet,
}

/// The surfaces of §3 that hold other nodes and are not pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// `surface`, a frame of `line`.
    Block,
    /// `surface2`, a frame of `line`.
    Nested,
    /// `note`, a frame of `note_line`.
    Note,
}

/// The picture of a window: a drawing of `res\art`, drawn by `art.rs` (T-120-12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// The panda of «О программе».
    About,
    /// The panda of the windows in the corner and of the letters.
    Avatar,
    /// The pictures of the letters (T-120-16) and of the wizard (T-120-19).
    Hello,
    Thanks,
    News,
    Update,
    Wizard,
}

impl Art {
    /// The name of the drawing in `res\art`.
    pub const fn name(self) -> &'static str {
        match self {
            Art::About => "about",
            Art::Avatar => "avatar",
            Art::Hello => "hello",
            Art::Thanks => "thanks",
            Art::News => "news",
            Art::Update => "update",
            Art::Wizard => "wizard",
        }
    }
}

/// What kind of window: it decides the narrow look and the gap above the row of buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Ordinary,
    /// Narrow: no circle with the panda; the sections become a row of tabs.
    Settings,
    /// The row of buttons stands closer to the content.
    Wizard,
    /// One element filling the window, no paddings and no buttons: the list of a choice field.
    Bare,
    /// The window in the corner (passport §2–§3): its own sizes, the buttons in shares 1 : 2.
    Corner,
    /// The menu of the icon (passport §3, T-120-15): rows of items, its own sizes.
    Menu,
}

/// The state the head of the menu reports: a dot `accent`, `ink3` or `warn` and its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuState {
    Working,
    Paused,
    Restart,
}

/// What an item of the menu shows at its end.
#[derive(Clone, Debug, PartialEq)]
pub enum Trailing {
    None,
    /// The key of the action, in its frame.
    Key(String),
    /// A dot `accent`: something new behind the item.
    Dot,
    /// A switch: the item turns it, and the menu stays.
    Switch(bool),
}

/// A part of a window description.
#[derive(Clone, Debug, PartialEq)]
pub enum Node<I> {
    /// Blocks one under another, a block's gap apart; a [`Node::Fill`] among them takes what is left
    /// of the height.
    Column(Vec<Node<I>>),
    /// Lines one under another, a line's gap apart: the inside of a block.
    Rows(Vec<Node<I>>),
    /// Side by side at their own widths, wrapping to the next line when the width runs out.
    Row(Vec<Node<I>>),
    /// Two equal halves while each is at least 20 B wide; narrower — one under the other.
    Pair(Box<Node<I>>, Box<Node<I>>),
    /// Three equal columns by the same rule.
    Triple(Box<Node<I>>, Box<Node<I>>, Box<Node<I>>),
    /// Takes the rest of the height of its column.
    Fill(Box<Node<I>>),
    /// A surface with the padding of its kind around what it holds.
    Surface(Surface, Box<Node<I>>),
    /// The steps of a wizard: the current one is shown, the height is the tallest one's.
    Steps(usize, Vec<Node<I>>),
    /// The head of a window: the line over the title, the title, the line under it. In a narrow
    /// window the circle with the panda stands beside it.
    Header {
        eyebrow: Option<String>,
        title: String,
        subtitle: Option<String>,
    },
    /// A paragraph in a role, wrapped by words.
    Text(Role, String),
    /// One line in a role, cut with an ellipsis.
    Line(Role, String),
    /// A button; `browser` — it opens the browser and carries the sign ↗.
    Button {
        id: I,
        label: String,
        kind: ButtonKind,
        browser: bool,
    },
    /// A row of buttons, pressed to the right, the main one the rightmost; one under another, the
    /// main one on top, when they do not fit.
    Buttons(Vec<Node<I>>),
    /// A card with a title, a text and buttons under the text — beside it when the window is tight.
    Action {
        title: String,
        words: String,
        buttons: Vec<Node<I>>,
    },
    /// A line of settings with a switch at its end; the whole line is pressed. `badge` — the
    /// label «new» after its words (passport §6).
    Toggle {
        id: I,
        label: String,
        hint: Option<String>,
        on: bool,
        badge: Option<String>,
    },
    /// A check box before its label; the whole line is pressed.
    Check {
        id: I,
        label: String,
        on: bool,
        /// A line in `small` under the label (the wizard's «what to attach», T-120-19).
        hint: Option<String>,
    },
    /// The progress of a wizard (passport §6): `total` bars across, the ones passed and the current
    /// `accent`, the rest `line2`; mirrored with the window.
    Progress { total: usize, current: usize },
    /// A radio button before its label; the whole line is pressed.
    Radio { id: I, label: String, on: bool },
    /// A choice card: a radio button, a title and a line under it; the whole card is pressed.
    ChoiceCard {
        id: I,
        title: String,
        words: String,
        selected: bool,
    },
    /// A label and a segmented switch; the switch takes a line of its own when it does not fit
    /// beside the label, and becomes a choice field when it does not fit even there.
    Segments {
        id: I,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// A label and a choice field: the selected option shown, the list of them in a popup.
    Choice {
        id: I,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// The sections of the settings: a column, or a row of tabs in a narrow window.
    Sections {
        id: I,
        items: Vec<String>,
        current: usize,
    },
    /// A numbered list: the numbers in `accent` (§1), the items wrapped beside them.
    Numbered(Vec<String>),
    /// A text field of Windows in the wrapper of the shell, its label above it.
    Field {
        id: I,
        label: String,
        value: String,
        /// The hint an empty field shows.
        hint: String,
        kind: FieldKind,
    },
    /// A list drawn by the shell: rows with or without a check box, one selected, scrolled from
    /// `first` (the layout moves `first` so that the selected row is seen).
    List {
        id: I,
        rows: Vec<ListItem>,
        selected: usize,
        first: usize,
    },
    /// The header of the window in the corner: the program's icon, «LANG SWITCHER · `about`» and
    /// the ✕ that closes it.
    CornerHeader { about: String, close: I },
    /// The message of the window in the corner: the picture of the window beside a title, a text
    /// and a link.
    Message {
        title: String,
        words: String,
        link: Option<(I, String)>,
    },
    /// The picture of «Обновлено»: the strip of the taskbar with the person's flag, at one moment
    /// of its animation.
    Taskbar(Taskbar),
    /// The head of the menu: the program's icon, «LANG SWITCHER», and the state at the end.
    MenuHead { state: MenuState, words: String },
    /// An item of the menu: an icon of `resrticons`, its label, what it shows at its end;
    /// `emphasis` — the label bold and the icon and label `accent` (an unread letter).
    MenuItem {
        id: I,
        icon: String,
        label: String,
        trailing: Trailing,
        emphasis: bool,
    },
    /// The line between the groups of the menu.
    MenuRule,
    /// A line of status (passport §6): its dot — `accent` while the program works, `ink3` paused,
    /// `warn` waiting for a restart, as in the head of the menu — and its words in `body`.
    Status { state: MenuState, words: String },
    /// The example with a key (passport §6 «Клавиша»): the word typed, the key, the word fixed.
    Example {
        typed: String,
        key: String,
        fixed: String,
    },
    /// What lies over the picture of the panel, each element in a named place of the picture
    /// (`anchor-…` of `resrt`, T-120-25) — when the window has the panel; `otherwise` in the flow
    /// when it stands in one column. Over the picture it takes no room in the flow, and neither the
    /// picture nor it is mirrored.
    OverPicture {
        over: Vec<(&'static str, OnPicture)>,
        otherwise: Box<Node<I>>,
    },
    /// The card of a letter (passport §6): its label, title, one line of its text, its buttons.
    LetterCard {
        label: String,
        title: String,
        words: String,
        buttons: Vec<Node<I>>,
    },
    /// A link to the site.
    Link { id: I, words: String },
    /// A line across.
    Divider,
}

/// A whole window: its kind, its picture, its content and its row of buttons, which stands in the
/// same place on every step and every section.
#[derive(Clone, Debug, PartialEq)]
pub struct Window<I> {
    pub kind: WindowKind,
    pub art: Option<Art>,
    /// The line under the picture: a name and an address (passport §2, `brand`).
    pub brand: Option<Brand<I>>,
    pub body: Node<I>,
    pub buttons: Vec<Node<I>>,
    /// The elements unavailable now: drawn in the look of §5, answering no press, skipped by Tab.
    pub disabled: Vec<I>,
}

/// The line under the picture of the panel: the name in the main face, the address beside it in
/// the monospace one.
#[derive(Clone, Debug, PartialEq)]
pub struct Brand<I> {
    pub name: String,
    pub address: String,
    /// The address is a link to the page of the program (decision of the controller on C3 of
    /// T-120-13, as the name of the old «О программе», 139.2 п. 4): pressed, it is this element.
    pub link: I,
}

/// An element over the picture of the panel (T-120-25), in the colours of the picture.
#[derive(Clone, Debug, PartialEq)]
pub enum OnPicture {
    /// A word on the little screen, as much of it as is typed; `fixed` — the word after the press.
    /// It starts at the left of its place in any language and reads as it is typed.
    Word { word: String, fixed: bool },
    /// Keys side by side in the middle of their place; `pressed` — lit.
    Keys { labels: Vec<String>, pressed: bool },
}

impl<I> Node<I> {
    /// The same node with every identifier passed through `f` — for the instrument of fitting,
    /// which goes over the windows of every type at once.
    pub fn map<J>(self, f: &impl Fn(I) -> J) -> Node<J> {
        let all = |nodes: Vec<Node<I>>| nodes.into_iter().map(|node| node.map(f)).collect();
        let one = |node: Box<Node<I>>| Box::new(node.map(f));
        match self {
            Node::Column(nodes) => Node::Column(all(nodes)),
            Node::Rows(nodes) => Node::Rows(all(nodes)),
            Node::Row(nodes) => Node::Row(all(nodes)),
            Node::Pair(a, b) => Node::Pair(one(a), one(b)),
            Node::Triple(a, b, c) => Node::Triple(one(a), one(b), one(c)),
            Node::Fill(node) => Node::Fill(one(node)),
            Node::Surface(kind, node) => Node::Surface(kind, one(node)),
            Node::Steps(current, steps) => Node::Steps(current, all(steps)),
            Node::Header {
                eyebrow,
                title,
                subtitle,
            } => Node::Header {
                eyebrow,
                title,
                subtitle,
            },
            Node::Text(role, text) => Node::Text(role, text),
            Node::Line(role, text) => Node::Line(role, text),
            Node::Button {
                id,
                label,
                kind,
                browser,
            } => Node::Button {
                id: f(id),
                label,
                kind,
                browser,
            },
            Node::Buttons(nodes) => Node::Buttons(all(nodes)),
            Node::Action {
                title,
                words,
                buttons,
            } => Node::Action {
                title,
                words,
                buttons: all(buttons),
            },
            Node::Toggle {
                id,
                label,
                hint,
                on,
                badge,
            } => Node::Toggle {
                id: f(id),
                label,
                hint,
                on,
                badge,
            },
            Node::Check {
                id,
                label,
                on,
                hint,
            } => Node::Check {
                id: f(id),
                label,
                on,
                hint,
            },
            Node::Progress { total, current } => Node::Progress { total, current },
            Node::Radio { id, label, on } => Node::Radio {
                id: f(id),
                label,
                on,
            },
            Node::ChoiceCard {
                id,
                title,
                words,
                selected,
            } => Node::ChoiceCard {
                id: f(id),
                title,
                words,
                selected,
            },
            Node::Segments {
                id,
                label,
                options,
                selected,
            } => Node::Segments {
                id: f(id),
                label,
                options,
                selected,
            },
            Node::Choice {
                id,
                label,
                options,
                selected,
            } => Node::Choice {
                id: f(id),
                label,
                options,
                selected,
            },
            Node::Sections { id, items, current } => Node::Sections {
                id: f(id),
                items,
                current,
            },
            Node::Numbered(items) => Node::Numbered(items),
            Node::Field {
                id,
                label,
                value,
                hint,
                kind,
            } => Node::Field {
                id: f(id),
                label,
                value,
                hint,
                kind,
            },
            Node::List {
                id,
                rows,
                selected,
                first,
            } => Node::List {
                id: f(id),
                rows,
                selected,
                first,
            },
            Node::CornerHeader { about, close } => Node::CornerHeader {
                about,
                close: f(close),
            },
            Node::Message { title, words, link } => Node::Message {
                title,
                words,
                link: link.map(|(id, text)| (f(id), text)),
            },
            Node::Taskbar(picture) => Node::Taskbar(picture),
            Node::MenuHead { state, words } => Node::MenuHead { state, words },
            Node::MenuItem {
                id,
                icon,
                label,
                trailing,
                emphasis,
            } => Node::MenuItem {
                id: f(id),
                icon,
                label,
                trailing,
                emphasis,
            },
            Node::MenuRule => Node::MenuRule,
            Node::Status { state, words } => Node::Status { state, words },
            Node::Example { typed, key, fixed } => Node::Example { typed, key, fixed },
            Node::OverPicture { over, otherwise } => Node::OverPicture {
                over,
                otherwise: one(otherwise),
            },
            Node::LetterCard {
                label,
                title,
                words,
                buttons,
            } => Node::LetterCard {
                label,
                title,
                words,
                buttons: buttons.into_iter().map(|node| node.map(f)).collect(),
            },
            Node::Link { id, words } => Node::Link { id: f(id), words },
            Node::Divider => Node::Divider,
        }
    }

    /// Every string the node shows, with the role it is set in, to read or to change — for the
    /// instrument of fitting (the string that does not fit, the stand-in for a missing one).
    pub fn texts_mut(&mut self, f: &mut impl FnMut(Role, &mut String)) {
        match self {
            Node::Column(nodes)
            | Node::Rows(nodes)
            | Node::Row(nodes)
            | Node::Steps(_, nodes)
            | Node::Buttons(nodes) => {
                nodes.iter_mut().for_each(|node| node.texts_mut(f));
            }
            Node::Pair(a, b) => {
                a.texts_mut(f);
                b.texts_mut(f);
            }
            Node::Triple(a, b, c) => {
                a.texts_mut(f);
                b.texts_mut(f);
                c.texts_mut(f);
            }
            Node::Fill(node) | Node::Surface(_, node) => node.texts_mut(f),
            Node::Header {
                eyebrow,
                title,
                subtitle,
            } => {
                if let Some(eyebrow) = eyebrow {
                    f(Role::Eyebrow, eyebrow);
                }
                f(Role::Title, title);
                if let Some(subtitle) = subtitle {
                    f(Role::Small, subtitle);
                }
            }
            Node::Text(role, text) | Node::Line(role, text) => f(*role, text),
            Node::Button { label, .. } => f(Role::Button, label),
            Node::Action {
                title,
                words,
                buttons,
            } => {
                f(Role::Strong, title);
                f(Role::Body, words);
                buttons.iter_mut().for_each(|node| node.texts_mut(f));
            }
            Node::Toggle {
                label, hint, badge, ..
            } => {
                f(Role::Body, label);
                if let Some(hint) = hint {
                    f(Role::Small, hint);
                }
                if let Some(badge) = badge {
                    f(Role::Label, badge);
                }
            }
            Node::Check { label, hint, .. } => {
                f(Role::Body, label);
                if let Some(hint) = hint {
                    f(Role::Small, hint);
                }
            }
            Node::Radio { label, .. } => f(Role::Body, label),
            Node::Progress { .. } => {}
            Node::ChoiceCard { title, words, .. } => {
                f(Role::Strong, title);
                f(Role::Small, words);
            }
            Node::Segments { label, options, .. } => {
                f(Role::Body, label);
                options.iter_mut().for_each(|option| f(Role::Body, option));
            }
            Node::Choice { label, options, .. } => {
                f(Role::Body, label);
                options.iter_mut().for_each(|option| f(Role::Body, option));
            }
            Node::Sections { items, .. } | Node::Numbered(items) => {
                items.iter_mut().for_each(|item| f(Role::Body, item));
            }
            Node::Field {
                label, value, hint, ..
            } => {
                f(Role::FieldLabel, label);
                f(Role::Body, value);
                f(Role::Body, hint);
            }
            Node::List { rows, .. } => rows
                .iter_mut()
                .for_each(|row| f(Role::Body, &mut row.label)),
            Node::CornerHeader { about, .. } => f(Role::CornerHeader, about),
            Node::Message { title, words, link } => {
                f(Role::CornerTitle, title);
                f(Role::CornerText, words);
                if let Some((_, link)) = link {
                    f(Role::Link, link);
                }
            }
            Node::Taskbar(_) => {}
            Node::MenuHead { words, .. } => f(Role::Small, words),
            Node::MenuItem {
                label, trailing, ..
            } => {
                f(Role::Body, label);
                if let Trailing::Key(key) = trailing {
                    f(Role::Small, key);
                }
            }
            Node::MenuRule => {}
            Node::Status { words, .. } => f(Role::Body, words),
            Node::Example { typed, fixed, .. } => {
                f(Role::Code, typed);
                f(Role::Code, fixed);
            }
            // The words over the picture are sized by their place in it, not by a role.
            Node::OverPicture { otherwise, .. } => otherwise.texts_mut(f),
            Node::LetterCard {
                label,
                title,
                words,
                buttons,
            } => {
                f(Role::Label, label);
                f(Role::Strong, title);
                f(Role::Body, words);
                buttons.iter_mut().for_each(|node| node.texts_mut(f));
            }
            Node::Link { words, .. } => f(Role::Link, words),
            Node::Divider => {}
        }
    }
}

impl<I> Window<I> {
    /// The same window with every identifier passed through `f`.
    pub fn map<J>(self, f: &impl Fn(I) -> J) -> Window<J> {
        Window {
            kind: self.kind,
            art: self.art,
            brand: self.brand.map(|brand| Brand {
                name: brand.name,
                address: brand.address,
                link: f(brand.link),
            }),
            body: self.body.map(f),
            buttons: self.buttons.into_iter().map(|node| node.map(f)).collect(),
            disabled: self.disabled.into_iter().map(f).collect(),
        }
    }

    /// Every string the window shows, with its role: the body, then the row of buttons.
    pub fn texts_mut(&mut self, f: &mut impl FnMut(Role, &mut String)) {
        self.body.texts_mut(f);
        self.buttons.iter_mut().for_each(|node| node.texts_mut(f));
    }
}

/// One moment of the picture of the taskbar (the mock-up of «Обновлено»): the strip of the
/// taskbar with the flag of one of the person's two layouts and its code, the plate of the keys,
/// the cursor on its way to the flag, the list of the two layouts over the strip.
#[derive(Clone, Debug, PartialEq)]
pub struct Taskbar {
    /// The person's two layouts: the one in force and the next.
    pub layouts: [crate::layouts::LayoutId; 2],
    /// Their codes, as the flag shows them.
    pub codes: [String; 2],
    /// Which of the two the flag shows now.
    pub shown: usize,
    /// The plate «Alt + Shift» is up.
    pub plate: bool,
    /// The cursor, as the share of its way to the flag.
    pub cursor: Option<f32>,
    /// The list of the two layouts is open, its first row hovered when `hover`.
    pub list: bool,
    pub hover: bool,
    /// The icons of the taskbar stand at its left: the interface of Windows reads right to left —
    /// the picture follows Windows, not the language of the program (passport §7).
    pub icons_left: bool,
}

/// One line or several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Line,
    Text,
    /// A field of a key (T-120-20): it shows the key and is not typed in; `capturing` — it takes
    /// every key now (`Event::KeyDown`, `Event::KeyUp`, `Event::FocusLost`). Out of the capture it
    /// is no stop of Tab.
    Key {
        capturing: bool,
    },
}

/// A row of a list.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub label: String,
    /// The row's check box, where it has one.
    pub check: Option<bool>,
    /// The row is the value now (the list of a choice field): a tick at its end.
    pub mark: bool,
}

/// What the shell answers about a window's elements.
#[derive(Clone, Debug, PartialEq)]
pub enum Event<I> {
    /// A button or a link was pressed.
    Pressed(I),
    /// A switch or a check box took the value.
    Toggled(I, bool),
    /// An option, a segment or a section was chosen.
    Selected(I, usize),
    /// The text of a field changed.
    Edited(I, String),
    /// The check box of row `usize` of a list took the value.
    RowChecked(I, usize, bool),
    /// A list was scrolled to show row `usize` first.
    Scrolled(I, usize),
    /// The tick the window asked for (`host::Content::tick`, T-120-25).
    Tick,
    /// The timer of the content's own (`host::Content::timer`, T-120-20).
    Timer,
    /// A key went down in the field of a key `id` in its capture: its virtual code. Sent at once,
    /// inside the key's own message — the keys held with it are read now as they were then.
    KeyDown { id: I, vk: u16 },
    /// A key went up there.
    KeyUp { id: I, vk: u16 },
    /// The field of a key `id` in its capture lost the keyboard: to the element `to`, or to none —
    /// a press beside the elements, another window.
    FocusLost { id: I, to: Option<I> },
}

impl<I: PartialEq> Node<I> {
    /// The options of the choice `id` and the selected one: a choice field, or segments that may be
    /// drawn as one when they do not fit (passport §6).
    pub fn options(&self, id: &I) -> Option<(&[String], usize)> {
        match self {
            Node::Choice {
                id: own,
                options,
                selected,
                ..
            }
            | Node::Segments {
                id: own,
                options,
                selected,
                ..
            } if own == id => Some((options, *selected)),
            Node::Column(nodes) | Node::Rows(nodes) | Node::Row(nodes) | Node::Steps(_, nodes) => {
                nodes.iter().find_map(|node| node.options(id))
            }
            Node::Pair(a, b) => a.options(id).or_else(|| b.options(id)),
            Node::Triple(a, b, c) => a
                .options(id)
                .or_else(|| b.options(id))
                .or_else(|| c.options(id)),
            Node::Fill(node) | Node::Surface(_, node) => node.options(id),
            _ => None,
        }
    }
}

impl<I: PartialEq> Window<I> {
    /// The options of the choice `id` anywhere in the window, and the selected one.
    pub fn options(&self, id: &I) -> Option<(&[String], usize)> {
        self.body.options(id)
    }
}
