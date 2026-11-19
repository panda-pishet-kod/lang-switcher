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

use crate::ui::tokens::Role;

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
}

impl Art {
    /// The name of the drawing in `res\art`.
    pub const fn name(self) -> &'static str {
        match self {
            Art::About => "about",
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
    /// A label and a choice field.
    Choice { id: I, label: String, value: String },
    /// The sections of the settings: a column, or a row of tabs in a narrow window.
    Sections {
        id: I,
        items: Vec<String>,
        current: usize,
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
    /// The line under the picture: the name and its words.
    pub brand: Option<String>,
    pub body: Node<I>,
    pub buttons: Vec<Node<I>>,
    /// The elements unavailable now: drawn in the look of §5, answering no press, skipped by Tab.
    pub disabled: Vec<I>,
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
}
