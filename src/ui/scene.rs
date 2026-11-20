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
}

impl Art {
    /// The name of the drawing in `res\art`.
    pub const fn name(self) -> &'static str {
        match self {
            Art::About => "about",
            Art::Avatar => "avatar",
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
        text: String,
        buttons: Vec<Node<I>>,
    },
    /// A line of settings with a switch at its end; the whole line is pressed.
    Toggle {
        id: I,
        label: String,
        hint: Option<String>,
        on: bool,
    },
    /// A check box before its label; the whole line is pressed.
    Check { id: I, label: String, on: bool },
    /// A radio button before its label; the whole line is pressed.
    Radio { id: I, label: String, on: bool },
    /// A choice card: a radio button, a title and a line under it; the whole card is pressed.
    ChoiceCard {
        id: I,
        title: String,
        text: String,
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
    /// A link to the site.
    Link { id: I, text: String },
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
    pub brand: Option<Brand>,
    pub body: Node<I>,
    pub buttons: Vec<Node<I>>,
    /// The elements unavailable now: drawn in the look of §5, answering no press, skipped by Tab.
    pub disabled: Vec<I>,
}

/// The line under the picture of the panel: the name in the main face, the address beside it in
/// the monospace one.
#[derive(Clone, Debug, PartialEq)]
pub struct Brand {
    pub name: String,
    pub address: String,
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
                text,
                buttons,
            } => Node::Action {
                title,
                text,
                buttons: all(buttons),
            },
            Node::Toggle {
                id,
                label,
                hint,
                on,
            } => Node::Toggle {
                id: f(id),
                label,
                hint,
                on,
            },
            Node::Check { id, label, on } => Node::Check {
                id: f(id),
                label,
                on,
            },
            Node::Radio { id, label, on } => Node::Radio {
                id: f(id),
                label,
                on,
            },
            Node::ChoiceCard {
                id,
                title,
                text,
                selected,
            } => Node::ChoiceCard {
                id: f(id),
                title,
                text,
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
            Node::Link { id, text } => Node::Link { id: f(id), text },
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
                text,
                buttons,
            } => {
                f(Role::Strong, title);
                f(Role::Body, text);
                buttons.iter_mut().for_each(|node| node.texts_mut(f));
            }
            Node::Toggle { label, hint, .. } => {
                f(Role::Body, label);
                if let Some(hint) = hint {
                    f(Role::Small, hint);
                }
            }
            Node::Check { label, .. } | Node::Radio { label, .. } => f(Role::Body, label),
            Node::ChoiceCard { title, text, .. } => {
                f(Role::Strong, title);
                f(Role::Small, text);
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
            Node::Link { text, .. } => f(Role::Link, text),
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
            brand: self.brand,
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

/// One line or several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Line,
    Text,
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
