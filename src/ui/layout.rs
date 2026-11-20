//! Layout: a window description, the window's size and a measure of text → the rectangle of every
//! element. Pure: no window, no Direct2D, no colour (`tests\ui_guard.rs` holds it to the numbers 0,
//! 1, 2 and 0.5 — every other one comes from the style table).
//!
//! What it decides, all by the passport (§3, §7):
//! * **the size and class of a window** — two thirds of the work area, between 620 × 500 and
//!   1280 × 720 points; a work area smaller than that — the whole of it, and only then scrolling;
//! * **one column or two** — the panel with the picture at 680 points and wider; a narrow window
//!   puts the circle with the panda beside its header (not in the settings);
//! * **the containers** — column, rows, wrapping row, pair and triple, fill, surfaces, the steps of a
//!   wizard (as tall as the tallest), the row of buttons in one place on every step;
//! * **the rules of wrapping** — a pair falls to a column under 20 B a half; buttons that do not fit
//!   stand one under another, the main one on top; a segmented switch takes a line of its own, then
//!   becomes a choice field; and when the content still does not fit the height, the buttons of an
//!   action card move beside its text;
//! * **the mirror** — one step at the very end, the same for every window: every rectangle reflected
//!   in the vertical axis of the window. The order of the elements — the order of Tab — is the
//!   description's and does not change.
//!
//! How much does not fit is answered, not repaired: picking the factor k is `fit.rs`'s.

use crate::ui::canvas::Rect;
use crate::ui::scene::{Art, ButtonKind, FieldKind, ListItem, Node, Surface, Window, WindowKind};
use crate::ui::scene::{MenuState, OnPicture, Trailing};
use crate::ui::text::{Align, Measure, Style, Wrap};
use crate::ui::tokens::shape::CHEVRON;
use crate::ui::tokens::{
    self, BLOCK_GAP, BLOCK_PAD, BLOCK_ROW_GAP, BRAND_FROM_BOTTOM, BRAND_FROM_EDGE, BUTTON_GAP,
    BUTTON_MIN_HEIGHT, BUTTON_PAD, BUTTON_PRIMARY_WEIGHT, BUTTON_ROW_ABOVE, CHECK, CHOICE_PAD,
    CLASS_LARGE_FROM, CLASS_ORDINARY_FROM, CONTENT_MAX_WIDTH, CONTENT_PAD, CONTROL_GAP, Em,
    FIELD_LABEL_GAP, FIELD_PAD, FIELD_TEXT_LINES, HEADER_SUBTITLE_GAP, HEADER_TITLE_GAP, LINE,
    LIST_BAR, LIST_PAD, LIST_ROW_PAD, LIST_VISIBLE_ROWS, MULTILINE_PAD, NARROW_ART, NARROW_PAD,
    NESTED_PAD, NOTE_PAD, PAIR_MIN_HALF, PANEL_ART, PANEL_SHARE, Role, SECTION_GAP, SECTION_PAD,
    SEGMENT_PAD, SETTINGS_ART, TOGGLE, TOGGLE_MIN, TWO_COLUMNS_FROM, WINDOW_MAX, WINDOW_MIN,
    WINDOW_OPEN_MIN, WINDOW_SHARE, WIZARD_BUTTON_ROW_ABOVE,
};
use crate::ui::tokens::{ART_RING, BADGE_GAP, TYPE_FLOOR};
use crate::ui::tokens::{BRAND_GAP, BRAND_NAME_WEIGHT, Face};
use crate::ui::tokens::{
    CORNER_ART, CORNER_BUTTON, CORNER_BUTTON_GAP, CORNER_BUTTON_MIN_HEIGHT, CORNER_GAP,
    CORNER_HEADER_ROW, CORNER_ICON, CORNER_ICON_GAP, CORNER_MAIN_SHARE, CORNER_PAD, CORNER_PICTURE,
    CORNER_SECOND_SHARE, CORNER_TITLE_GAP,
};
use crate::ui::tokens::{KEY_BOTTOM, KEY_PAD};
use crate::ui::tokens::{
    MENU_B, MENU_DOT, MENU_HEAD_PAD, MENU_ICON, MENU_ICON_COLUMN, MENU_ITEM_GAP, MENU_ITEM_PAD,
    MENU_PAD, MENU_ROW_MIN_HEIGHT, MENU_RULE_PAD, MENU_STATE_DOT, MENU_STATE_GAP,
};
use crate::ui::tokens::{
    ON_PICTURE_KEY, ON_PICTURE_KEY_GAP, ON_PICTURE_KEY_LABEL, ON_PICTURE_KEY_MIN,
    ON_PICTURE_KEY_PAD, ON_PICTURE_WEIGHT, ON_PICTURE_WORD, ON_PICTURE_WORD_INSET,
};
use crate::ui::tokens::{ROW_FIELD_MIN, SETTINGS_PANEL_GAP, SETTINGS_PANEL_PAD, STATUS_DOT};
use crate::ui::tokens::{WIZARD_STEP, WIZARD_STEP_GAP};

/// The class of a window by its width: the table of type sizes has a factor per class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Narrower than two columns: one column.
    Narrow,
    /// Two columns, below the largest width.
    Ordinary,
    /// At the largest width.
    Large,
}

/// The size of a window, in points at 100 %.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowSize {
    pub w: f32,
    pub h: f32,
    pub class: Class,
    /// The work area is smaller than the least window: the window takes all of it, and only here
    /// its content may scroll.
    pub scrolls: bool,
}

fn class_of(w: f32, h: f32) -> Class {
    let reaches = |from: tokens::SizePt| w >= from.w.0 && h >= from.h.0;
    if reaches(CLASS_LARGE_FROM) {
        Class::Large
    } else if reaches(CLASS_ORDINARY_FROM) {
        Class::Ordinary
    } else {
        Class::Narrow
    }
}

/// The size and class of a window whose client area is `w` × `h` points: the window as it is,
/// after the person dragged its frame.
pub fn client_size(w: f32, h: f32) -> WindowSize {
    WindowSize {
        w,
        h,
        class: class_of(w, h),
        scrolls: w < WINDOW_MIN.w.0 || h < WINDOW_MIN.h.0,
    }
}

/// The least window of the passport, 620 × 500, while the work area holds it — a small window of a
/// few lines (`host::Content::least`); a work area smaller still gives the window all of it, as
/// [`window_size`] does.
pub fn least_window_size(area_w: f32, area_h: f32) -> WindowSize {
    if area_w < WINDOW_MIN.w.0 || area_h < WINDOW_MIN.h.0 {
        return window_size(area_w, area_h);
    }
    WindowSize {
        w: WINDOW_MIN.w.0,
        h: WINDOW_MIN.h.0,
        class: class_of(WINDOW_MIN.w.0, WINDOW_MIN.h.0),
        scrolls: false,
    }
}

/// The window for a work area of `area_w` × `area_h` points (passport §3).
pub fn window_size(area_w: f32, area_h: f32) -> WindowSize {
    if area_w < WINDOW_MIN.w.0 || area_h < WINDOW_MIN.h.0 {
        return WindowSize {
            w: area_w,
            h: area_h,
            class: class_of(area_w, area_h),
            scrolls: true,
        };
    }

    // Two thirds of the work area, but at opening no less than 900 × 540 while the area holds it
    // (208.168), within the limits of the frame.
    let side = |area: f32, open: f32, least: f32, most: f32| {
        (area * WINDOW_SHARE.0)
            .max(open.min(area))
            .clamp(least, most)
    };
    let w = side(area_w, WINDOW_OPEN_MIN.w.0, WINDOW_MIN.w.0, WINDOW_MAX.w.0);
    let h = side(area_h, WINDOW_OPEN_MIN.h.0, WINDOW_MIN.h.0, WINDOW_MAX.h.0);

    WindowSize {
        w,
        h,
        class: class_of(w, h),
        scrolls: false,
    }
}

/// The state a status line reports (passport §6: the dot `accent`, `ink3` or `warn`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Working,
    Idle,
    Restart,
}

/// What a placed rectangle shows. The elements draw each kind one way (T-120-6).
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    /// The panel with the picture.
    Panel,
    /// A picture of the panda in its circle.
    Art(Art),
    Surface(Surface),
    Text {
        role: Role,
        text: String,
        wrap: Wrap,
        align: Align,
    },
    Button {
        kind: ButtonKind,
        label: String,
    },
    /// The line over the window's row of buttons.
    ButtonRule,
    /// The track of a switch.
    Toggle {
        on: bool,
    },
    /// The box of a check box.
    Check {
        on: bool,
    },
    /// The track of a segmented switch.
    Track,
    Segment {
        index: usize,
        selected: bool,
        label: String,
    },
    /// The box of a choice field.
    Field {
        value: String,
    },
    Section {
        index: usize,
        current: bool,
        label: String,
    },
    /// A link: its words in `role` — `link`, or `brand` for the address under the picture.
    Link {
        text: String,
        role: Role,
    },
    Divider,
    /// Where a press counts for the element: the whole line of a switch or a check box.
    Hit,
    /// A radio button.
    Radio {
        on: bool,
    },
    /// The body of a choice card: the whole card is pressed.
    Card {
        selected: bool,
    },
    /// The wrapper of a text field of Windows: the field itself is an `EDIT` the host places inside
    /// it; the value or, when it is empty, the hint is drawn under it too — for the first frame
    /// and for the pictures made in memory.
    Input {
        multiline: bool,
        /// A field of a key: whether it captures now (`FieldKind::Key`).
        capture: Option<bool>,
        value: String,
        hint: String,
    },
    /// The frame of a list, with what the keys of the list need.
    List(ListView),
    /// A row of a list: its number among all the rows and its check box, where it has one.
    ListRow {
        selected: bool,
        index: usize,
        check: Option<bool>,
    },
    /// The tick of the row that is the value now, in the list of a choice field.
    Tick,
    /// The program's icon, in the header of the window in the corner.
    AppIcon,
    /// The ✕ that closes the window in the corner.
    Cross,
    /// An icon of the menu (`resrticons`), `accent` when the item is emphasised.
    MenuIcon {
        name: String,
        emphasis: bool,
    },
    /// The picture of the taskbar in «Обновлено», at one moment.
    Taskbar(crate::ui::scene::Taskbar),
    /// The thin scroll bar of a list that has more rows than it shows.
    Scroll {
        first: usize,
        visible: usize,
        total: usize,
    },
    /// One bar of the progress of a wizard.
    Progress {
        done: bool,
    },
    /// A key: its name in a frame.
    Key {
        label: String,
    },
    /// The dot of a status line.
    Dot(Status),
    /// The label «new».
    Badge {
        text: String,
    },
    /// A row of the tray menu.
    MenuRow,
    /// The name in the line under the picture: the main face, the weight and colour of a name.
    BrandName {
        text: String,
    },
    /// The number of an item of a numbered list.
    Number {
        text: String,
    },
    /// A word over the picture of the panel (T-120-25): the monospace face at `size` points, in the
    /// colours of the picture; `fixed` — the word after the press.
    PictureWord {
        word: String,
        size: f32,
        fixed: bool,
    },
    /// A key over the picture of the panel: its name at `size` points in the colours of the
    /// picture; `pressed` — lit.
    PictureKey {
        label: String,
        size: f32,
        pressed: bool,
    },
}

/// What the keys and the wheel of a list need of it: the selected row, the first row shown, how many
/// are shown and how many there are, and the check box of the selected row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListView {
    pub selected: usize,
    pub first: usize,
    pub visible: usize,
    pub total: usize,
    pub checked: Option<bool>,
}

impl ListView {
    /// The first row to show so that `row` is seen, the view moved as little as it can.
    pub fn first_showing(self, row: usize) -> usize {
        if row < self.first {
            row
        } else if row >= self.first + self.visible {
            row + 1 - self.visible
        } else {
            self.first
        }
    }

    /// The first row after scrolling by `rows` (back when negative), inside the list.
    pub fn scrolled(self, rows: isize) -> usize {
        let last = self.total.saturating_sub(self.visible);
        self.first.saturating_add_signed(rows).min(last)
    }
}

impl Part {
    /// Whether the part lies over the picture of the panel (T-120-25): it moves with the picture
    /// and is not mirrored.
    pub fn lies_over_picture(&self) -> bool {
        matches!(self, Part::PictureWord { .. } | Part::PictureKey { .. })
    }

    /// The name of the kind, for the messages of the tests.
    pub fn name(&self) -> &'static str {
        match self {
            Part::Panel => "panel",
            Part::Art(_) => "picture",
            Part::Surface(_) => "surface",
            Part::Text { .. } => "text",
            Part::Button { .. } => "button",
            Part::ButtonRule => "button rule",
            Part::Toggle { .. } => "toggle",
            Part::Check { .. } => "check box",
            Part::Track => "segment track",
            Part::Segment { .. } => "segment",
            Part::Field { .. } => "choice field",
            Part::Section { .. } => "section",
            Part::Link { .. } => "link",
            Part::Divider => "divider",
            Part::Hit => "hit area",
            Part::Radio { .. } => "radio button",
            Part::Card { .. } => "choice card",
            Part::Input { .. } => "text field",
            Part::List(_) => "list",
            Part::Scroll { .. } => "list scroll bar",
            Part::Tick => "tick",
            Part::AppIcon => "program icon",
            Part::Cross => "close cross",
            Part::Taskbar(_) => "taskbar picture",
            Part::MenuIcon { .. } => "menu icon",
            Part::ListRow { .. } => "list row",
            Part::Progress { .. } => "wizard progress",
            Part::Key { .. } => "key",
            Part::Dot(_) => "status dot",
            Part::Badge { .. } => "badge",
            Part::MenuRow => "menu row",
            Part::BrandName { .. } => "brand name",
            Part::Number { .. } => "list number",
            Part::PictureWord { .. } => "word over the picture",
            Part::PictureKey { .. } => "key over the picture",
        }
    }
}

/// One element of a laid-out window.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed<I> {
    pub part: Part,
    pub rect: Rect,
    /// The element's identifier, where it is pressed or chosen.
    pub id: Option<I>,
}

/// How many points the content lacks, across and down; zero or less — it fits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shortfall {
    pub w: f32,
    pub h: f32,
}

impl Shortfall {
    pub fn fits(self) -> bool {
        self.w <= 0.0 && self.h <= 0.0
    }
}

/// A step of the order of giving way (passport §3а), in its order: taken one after another, each
/// only while the window does not fit at the type size of its table, the same for every window.
/// After the last there is none: the window scrolls and the instrument of fitting fails it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Concession {
    /// A row of buttons that does not fit stands in two rows, the main one in the lower at its
    /// edge; not fitting so either — in a column across the width, the main one on top.
    Buttons,
    /// A pair stands in a column when a half is narrower than its widest unbreakable content (a
    /// button, a word), even when the half is not narrower than its least.
    Pair,
    /// In a pair standing side by side, the buttons of a card stand beside its text — when this
    /// makes it lower.
    Card,
    /// A narrow window loses the circle with the panda in its header.
    Circle,
    /// The label of a button wider than its place goes on to a second line; the button grows.
    Wrap,
}

impl Concession {
    /// The steps, in their order.
    pub const ALL: &'static [Concession] = &[
        Concession::Buttons,
        Concession::Pair,
        Concession::Card,
        Concession::Circle,
        Concession::Wrap,
    ];
    /// The steps the factor of the table is measured with (T-120-11): the one that was there
    /// before the order of §3а. The others give way only at the factor of the table.
    pub const TABLE: &'static [Concession] = &[Concession::Card];

    fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A laid-out window.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout<I> {
    pub size: WindowSize,
    /// The type size B it was laid out at.
    pub b: f32,
    pub k: f32,
    pub mirrored: bool,
    pub items: Vec<Placed<I>>,
    pub shortfall: Shortfall,
    /// The last step of the order of giving way the window came to (§3а); none — it fitted as it
    /// was.
    pub step: Option<Concession>,
}

impl<I: Copy + PartialEq> Layout<I> {
    /// The identifiers in the order of the Tab key: the description's order, once each, in either
    /// direction of the window.
    pub fn focus_order(&self) -> Vec<I> {
        let mut order: Vec<I> = Vec::new();
        for id in self.items.iter().filter_map(|item| item.id) {
            if !order.contains(&id) {
                order.push(id);
            }
        }
        order
    }

    /// Where `straight`'s element `item` stands in this layout when it is mirrored (passport §7):
    /// reflected in the vertical axis of the window — or, for what lies over the picture of the
    /// panel (T-120-25), moved with the picture and not reflected in it.
    pub fn reflection_of(&self, straight: &Layout<I>, item: &Placed<I>) -> Rect {
        let r = item.rect;
        let picture = |layout: &Layout<I>| {
            layout
                .items
                .iter()
                .find(|placed| matches!(placed.part, Part::Art(_)))
                .map(|placed| placed.rect)
        };
        match (
            item.part.lies_over_picture(),
            picture(straight),
            picture(self),
        ) {
            (true, Some(from), Some(to)) => Rect::new(r.x + to.x - from.x, r.y, r.w, r.h),
            _ => Rect::new(self.size.w - r.x - r.w, r.y, r.w, r.h),
        }
    }
}

/// `window` at `size` and the factor `k`, its text measured by `measure`; `rtl` — the language of
/// the program reads right to left and the window is mirrored.
pub fn lay_out<I: Copy>(
    window: &Window<I>,
    size: WindowSize,
    k: f32,
    rtl: bool,
    measure: &dyn Measure,
) -> Layout<I> {
    lay_out_with(window, size, k, rtl, measure, Concession::ALL)
}

/// [`lay_out`] with only the steps `allowed` of the order of giving way (§3а) — the instrument of
/// fitting measures the factor of its table with [`Concession::TABLE`].
pub fn lay_out_with<I: Copy>(
    window: &Window<I>,
    size: WindowSize,
    k: f32,
    rtl: bool,
    measure: &dyn Measure,
    allowed: &[Concession],
) -> Layout<I> {
    // The window in the corner keeps its sizes (§2): its buttons' role at their 13,5 points.
    let b = if window.kind == WindowKind::Corner {
        CORNER_BUTTON.0 / Role::Button.style().size.0
    } else if window.kind == WindowKind::Menu {
        MENU_B.0
    } else {
        tokens::window_b(size.w, size.h, k)
    };
    let mut place = Placer {
        b,
        h: size.h,
        one_column: size.w < TWO_COLUMNS_FROM.0,
        kind: window.kind,
        art: window.art,
        picture: None,
        sections_at: None,
        given: 0,
        in_pair: false,
        measure,
        out: Vec::new(),
        rtl,
        over: 0.0,
    };

    let mut shortfall = place.window(window, size);
    // The order of giving way (§3а): the big windows only — the window in the corner and the menu
    // take the height they need, the list of a choice field is the list.
    let mut step = None;
    let gives_way = !matches!(
        window.kind,
        WindowKind::Corner | WindowKind::Menu | WindowKind::Bare
    );
    for concession in Concession::ALL {
        if shortfall.fits() || !gives_way {
            break;
        }
        if !allowed.contains(concession) {
            continue;
        }
        place.given |= concession.bit();
        place.out.clear();
        place.over = 0.0;
        shortfall = place.window(window, size);
        step = Some(*concession);
    }

    let mut items = place.out;
    if rtl {
        mirror(&mut items, size.w);
    }

    Layout {
        size,
        b,
        k,
        mirrored: rtl,
        items,
        shortfall,
        step,
    }
}

/// The width the menu `window` needs (T-120-15): its widest item or head on one line, and its
/// paddings; the window keeps it between `MENU_MIN_WIDTH` and half the work area (the host's).
pub fn menu_width<I: Copy>(window: &Window<I>, measure: &dyn Measure) -> f32 {
    let place: Placer<'_, I> = Placer {
        b: MENU_B.0,
        h: WINDOW_MIN.h.0,
        one_column: true,
        kind: window.kind,
        art: None,
        picture: None,
        sections_at: None,
        given: 0,
        in_pair: false,
        measure,
        out: Vec::new(),
        rtl: false,
        over: 0.0,
    };
    let parts: Vec<&Node<I>> = match &window.body {
        Node::Column(children) => children.iter().collect(),
        other => vec![other],
    };
    let widest = parts
        .into_iter()
        .map(|part| match part {
            Node::MenuItem {
                label,
                trailing,
                emphasis,
                ..
            } => place.menu_item_width(label, trailing, *emphasis),
            Node::MenuHead { words, .. } => place.menu_head_width(words),
            _ => 0.0,
        })
        .fold(0.0, f32::max);
    widest + MENU_PAD.0 * 2.0
}

/// The rule of the pair (passport §3): `count` equal columns stand side by side in `width` points at
/// the type size `b` while each is at least 15 B wide (208.168), the blocks' gap between them. A pair
/// lives in the column of text, at most 40 B: up to 19,5 B a half.
pub fn columns_fit(count: usize, width: f32, b: f32) -> bool {
    let count = count.max(1) as f32;
    let each = (width - BLOCK_GAP.at(b) * (count - 1.0)) / count;
    each >= PAIR_MIN_HALF.at(b)
}

/// The mirror of passport §7: every rectangle reflected in the vertical axis of a window `width`
/// points wide. The pictures move with their rectangles and are not turned (the canvas draws them
/// as they are); text reads from its own edge by the direction of the language.
pub fn mirror<I>(items: &mut [Placed<I>], width: f32) {
    for item in items {
        item.rect.x = width - item.rect.x - item.rect.w;
    }
}

/// The dot of a line of status for the state of the program.
fn status_of(state: MenuState) -> Status {
    match state {
        MenuState::Working => Status::Working,
        MenuState::Paused => Status::Idle,
        MenuState::Restart => Status::Restart,
    }
}

/// What places the nodes of one window.
struct Placer<'m, I> {
    b: f32,
    /// The language reads right to left: the window is mirrored at the end (the arrow of an example
    /// points the way it reads).
    rtl: bool,
    /// The window's height: the brand line is sized by it.
    h: f32,
    /// The window stands in one column: the circle beside the header, the sections as tabs.
    one_column: bool,
    kind: WindowKind,
    art: Option<Art>,
    /// The circle the picture of the panel is drawn in, once it is placed: what lies over the
    /// picture is placed by it.
    picture: Option<Rect>,
    /// The column under the picture of the panel of the settings — left, top, width: where their
    /// sections stand in a wide window.
    sections_at: Option<(f32, f32, f32)>,
    /// The steps of the order of giving way taken (`Concession::bit`).
    given: u8,
    /// The node is placed in a pair standing side by side (§3а, step 3).
    in_pair: bool,
    measure: &'m dyn Measure,
    out: Vec<Placed<I>>,
    /// The widest overflow met: a run of text wider than its room.
    over: f32,
}

impl<I: Copy> Placer<'_, I> {
    fn em(&self, value: Em) -> f32 {
        value.at(self.b)
    }

    /// Whether the step `concession` of the order of giving way is taken.
    fn gives(&self, concession: Concession) -> bool {
        self.given & concession.bit() != 0
    }

    /// Whether `node` placed `w` wide runs over its room: a word or a button wider than it.
    fn overflows_at(&mut self, node: &Node<I>, w: f32) -> bool {
        let (kept, over) = (self.out.len(), self.over);
        self.over = 0.0;
        self.node(node, 0.0, 0.0, w, None);
        let overflows = self.over > 0.0;
        self.out.truncate(kept);
        self.over = over;
        overflows
    }

    /// The height `place` takes, placed and taken away again.
    fn trial(&mut self, place: impl FnOnce(&mut Self) -> f32) -> f32 {
        let (kept, over) = (self.out.len(), self.over);
        let h = place(self);
        self.out.truncate(kept);
        self.over = over;
        h
    }

    fn style(&self, role: Role) -> Style {
        Style::of(role, self.b, self.h)
    }

    fn push(&mut self, part: Part, rect: Rect, id: Option<I>) -> usize {
        self.out.push(Placed { part, rect, id });
        self.out.len() - 1
    }

    /// The width of `text` on one line, unbounded.
    fn width_of(&self, style: Style, text: &str) -> f32 {
        self.measure
            .measure(text, style, Wrap::OneLine, f32::MAX)
            .width
    }

    /// The height `node` takes in `w`, placed nowhere.
    fn height(&mut self, node: &Node<I>, w: f32, slot: Option<f32>) -> f32 {
        let (kept, over) = (self.out.len(), self.over);
        let h = self.node(node, 0.0, 0.0, w, slot);
        self.out.truncate(kept);
        self.over = over;
        h
    }

    /// The whole window; answers what does not fit.
    fn window(&mut self, window: &Window<I>, size: WindowSize) -> Shortfall {
        if window.kind == WindowKind::Corner {
            return self.corner_window(window, size);
        }
        if window.kind == WindowKind::Menu {
            return self.menu_window(window, size);
        }
        // One element filling the window: the list of a choice field in its popup.
        if window.kind == WindowKind::Bare {
            let h = self.node(&window.body, 0.0, 0.0, size.w, Some(size.h));
            return Shortfall {
                w: self.over,
                h: h - size.h,
            };
        }
        // One column under the width of two (208.168: 880), whatever the class of type sizes.
        let narrow = size.w < TWO_COLUMNS_FROM.0;
        let (mut area_x, mut area_w) = (0.0, size.w);
        let mut brand_line: Vec<Placed<I>> = Vec::new();

        if let (Some(art), false) = (window.art, narrow) {
            let panel = size.w * PANEL_SHARE.0;
            self.push(Part::Panel, Rect::new(0.0, 0.0, panel, size.h), None);
            let circle = match window.kind {
                WindowKind::Settings => SETTINGS_ART,
                _ => PANEL_ART,
            };
            let d = circle.at(size.w, size.h);
            // The settings: the picture at the top of the panel and their sections under it (the
            // mock-up's `.panel.setp`); any other window: the picture in the middle of the panel.
            let at = if window.kind == WindowKind::Settings {
                let (pad_v, pad_h) = (self.em(SETTINGS_PANEL_PAD.v), self.em(SETTINGS_PANEL_PAD.h));
                let at = Rect::new((panel - d) / 2.0, pad_v, d, d);
                self.sections_at = Some((
                    pad_h,
                    at.bottom() + self.em(SETTINGS_PANEL_GAP),
                    panel - pad_h * 2.0,
                ));
                at
            } else {
                Rect::new((panel - d) / 2.0, (size.h - d) / 2.0, d, d)
            };
            self.push(Part::Art(art), at, None);
            self.picture = Some(at);

            if let Some(brand) = &window.brand {
                let style = self.style(Role::Brand);
                let edge = self.em(BRAND_FROM_EDGE);
                let line = style.line_height();
                let at = Rect::new(
                    edge,
                    size.h - self.em(BRAND_FROM_BOTTOM) - line,
                    panel - edge * 2.0,
                    line,
                );
                // The name in the main face, the address beside it in the monospace one.
                let name_style = Style {
                    face: Face::Main,
                    weight: BRAND_NAME_WEIGHT,
                    ..style
                };
                let name_w = self.width_of(name_style, &brand.name).min(at.w);
                let gap = BRAND_GAP.0 * style.size;
                let room = (at.w - name_w - gap).max(0.0);
                // Placed after the content and the buttons: the link of the address is the last
                // stop of Tab, the order of reading (passport §7).
                brand_line = vec![
                    Placed {
                        part: Part::BrandName {
                            text: brand.name.clone(),
                        },
                        rect: Rect::new(at.x, at.y, name_w, line),
                        id: None,
                    },
                    Placed {
                        part: Part::Link {
                            text: brand.address.clone(),
                            role: Role::Brand,
                        },
                        rect: Rect::new(
                            at.x + name_w + gap,
                            at.y,
                            self.width_of(style, &brand.address).min(room),
                            line,
                        ),
                        id: Some(brand.link),
                    },
                ];
            }
            (area_x, area_w) = (panel, size.w - panel);
        }

        let pad = if narrow { NARROW_PAD } else { CONTENT_PAD };
        let column = (area_w - self.em(pad.h) * 2.0).min(self.em(CONTENT_MAX_WIDTH));
        let x = area_x + (area_w - column) / 2.0;
        let (top, bottom) = (self.em(pad.v), size.h - self.em(pad.v));

        // The row of buttons stands at the bottom, the same on every step; the body takes the rest.
        let mut floor = bottom;
        let mut row = None;
        if !window.buttons.is_empty() {
            let height = {
                let (kept, over) = (self.out.len(), self.over);
                let h = self.buttons(&window.buttons, 0.0, 0.0, column);
                self.out.truncate(kept);
                self.over = over;
                h
            };
            let above = match window.kind {
                WindowKind::Wizard => WIZARD_BUTTON_ROW_ABOVE,
                _ => BUTTON_ROW_ABOVE,
            };
            let rule = bottom - height - self.em(above) - LINE.0;
            row = Some((rule, bottom - height));
            floor = rule - self.em(BLOCK_GAP);
        }

        let room = floor - top;
        let natural = self.height(&window.body, column, None);
        // The height of the room reaches a column, for its fills; nothing else stretches to it.
        let fills = matches!(&window.body, Node::Column(children) if children.iter().any(|child| matches!(child, Node::Fill(_))));
        // What does not fit is measured with the fills at their least (T-120-20): a list that takes
        // the room left shows as many rows as there is room for, at least one — its natural six rows
        // are no need of the window.
        let least = if fills {
            self.height(&window.body, column, Some(0.0))
        } else {
            natural
        };
        let slot = matches!(window.body, Node::Column(_)).then_some(room);
        // ◆ Content that does not fill the room stands in the middle of it, as the mock-ups centre
        // their content (`align-content: center`); a window with a fill takes the room from the top.
        let top = if fills || natural >= room {
            top
        } else {
            top + (room - natural) / 2.0
        };
        self.node(&window.body, x, top, column, slot);

        if let Some((rule, buttons)) = row {
            self.push(Part::ButtonRule, Rect::new(x, rule, column, LINE.0), None);
            self.window_buttons(window, x, buttons, column);
        }
        self.out.extend(brand_line);

        Shortfall {
            w: self.over,
            h: least - room,
        }
    }

    /// Places `node` at `x`, `y` in `w`; `slot` is the height it must take, where it has one.
    /// Answers the height it took.
    fn node(&mut self, node: &Node<I>, x: f32, y: f32, w: f32, slot: Option<f32>) -> f32 {
        match node {
            Node::Column(children) => self.column(children, x, y, w, slot, self.em(BLOCK_GAP)),
            Node::Rows(children) => self.column(children, x, y, w, slot, self.em(BLOCK_ROW_GAP)),
            Node::Row(children) => self.row(children, x, y, w),
            Node::Pair(left, right) => self.columns(&[left, right], x, y, w, slot),
            Node::Triple(a, b, c) => self.columns(&[a, b, c], x, y, w, slot),
            Node::Fill(child) => {
                let h = self.node(child, x, y, w, slot);
                slot.map_or(h, |slot| slot.max(h))
            }
            Node::Surface(kind, child) => self.surface(*kind, child, x, y, w, slot),
            Node::Steps(current, steps) => {
                let tallest = steps
                    .iter()
                    .map(|step| self.height(step, w, None))
                    .fold(0.0, f32::max);
                let h = slot.map_or(tallest, |slot| slot.max(tallest));
                if let Some(step) = steps.get(*current) {
                    self.node(step, x, y, w, Some(h));
                }
                h
            }
            Node::Header {
                eyebrow,
                title,
                subtitle,
            } => self.header(eyebrow.as_deref(), title, subtitle.as_deref(), x, y, w),
            Node::Text(role, text) => self.text(*role, text, x, y, w, Wrap::Words),
            Node::Line(role, text) => self.text(*role, text, x, y, w, Wrap::OneLine),
            Node::Button { .. } => {
                let (bw, bh) = self.fitted_button(node, w);
                self.button(node, Rect::new(x, y, bw, bh));
                bh
            }
            Node::Buttons(buttons) => self.buttons(buttons, x, y, w),
            Node::Action {
                title,
                words,
                buttons,
            } => self.action(title, words, buttons, x, y, w),
            Node::Toggle {
                id,
                label,
                hint,
                on,
                badge,
            } => {
                let track = (
                    self.em(TOGGLE.w).max(TOGGLE_MIN.w.0),
                    self.em(TOGGLE.h).max(TOGGLE_MIN.h.0),
                );
                self.control(
                    *id,
                    (label, badge.as_deref()),
                    hint.as_deref(),
                    Part::Toggle { on: *on },
                    track,
                    false,
                    x,
                    y,
                    w,
                )
            }
            Node::Progress { total, current } => {
                let (h, gap) = (self.em(WIZARD_STEP), self.em(WIZARD_STEP_GAP));
                let count = (*total).max(1);
                let each = ((w - gap * (count - 1) as f32) / count as f32).max(0.0);
                for index in 0..count {
                    self.push(
                        Part::Progress {
                            done: index <= *current,
                        },
                        Rect::new(x + (each + gap) * index as f32, y, each, h),
                        None,
                    );
                }
                h
            }
            Node::Check {
                id,
                label,
                on,
                hint,
            } => {
                let side = CHECK.at(self.b);
                self.control(
                    *id,
                    (label, None),
                    hint.as_deref(),
                    Part::Check { on: *on },
                    (side, side),
                    true,
                    x,
                    y,
                    w,
                )
            }
            Node::Radio { id, label, on } => {
                let side = CHECK.at(self.b);
                self.control(
                    *id,
                    (label, None),
                    None,
                    Part::Radio { on: *on },
                    (side, side),
                    true,
                    x,
                    y,
                    w,
                )
            }
            Node::ChoiceCard {
                id,
                title,
                words,
                selected,
            } => self.choice_card(*id, title, words, *selected, x, y, w),
            Node::Segments {
                id,
                label,
                options,
                selected,
            } => self.segments(*id, label, options, *selected, x, y, w),
            Node::Choice {
                id,
                label,
                options,
                selected,
            } => self.choice(*id, label, options, *selected, x, y, w),
            // In the panel of a wide window of the settings they take no room in the flow.
            Node::Sections { id, items, current } => match self.sections_at {
                Some((left, top, width)) => {
                    self.sections(*id, items, *current, left, top, width);
                    0.0
                }
                None => self.sections(*id, items, *current, x, y, w),
            },
            Node::Field {
                id,
                label,
                value,
                hint,
                kind,
            } => self.field(*id, label, value, hint, *kind, x, y, w),
            Node::List {
                id,
                rows,
                selected,
                first,
            } => self.list(*id, rows, *selected, *first, x, y, w, slot),
            Node::CornerHeader { about, close } => self.corner_header(about, *close, x, y, w),
            Node::MenuHead { state, words } => self.menu_head(*state, words, x, y, w),
            Node::MenuItem {
                id,
                icon,
                label,
                trailing,
                emphasis,
            } => self.menu_item(*id, icon, label, trailing, *emphasis, x, y, w),
            Node::Example { typed, key, fixed } => self.example(typed, key, fixed, x, y, w),
            Node::OverPicture { over, otherwise } => match (self.art, self.picture) {
                (Some(art), Some(circle)) => {
                    for (anchor, element) in over {
                        self.over_picture(art, circle, anchor, element);
                    }
                    0.0
                }
                _ => self.node(otherwise, x, y, w, slot),
            },
            Node::LetterCard {
                label,
                title,
                words,
                buttons,
            } => self.letter_card(label, title, words, buttons, x, y, w),
            Node::MenuRule => {
                let (pad_h, pad_v) = (MENU_RULE_PAD.w.0, MENU_RULE_PAD.h.0);
                self.push(
                    Part::Divider,
                    Rect::new(x + pad_h, y + pad_v, w - pad_h * 2.0, LINE.0),
                    None,
                );
                pad_v * 2.0 + LINE.0
            }
            Node::Status { state, words } => {
                // The dot in the middle of the first line, the words after it.
                let (dot, gap) = (self.em(STATUS_DOT), self.em(CONTROL_GAP));
                let line = self.style(Role::Body).line_height();
                self.push(
                    Part::Dot(status_of(*state)),
                    Rect::new(x, y, dot, line),
                    None,
                );
                let words = self.text(
                    Role::Body,
                    words,
                    x + dot + gap,
                    y,
                    (w - dot - gap).max(0.0),
                    Wrap::Words,
                );
                words.max(line)
            }
            Node::Taskbar(picture) => {
                let h = CORNER_PICTURE.0;
                self.push(Part::Taskbar(picture.clone()), Rect::new(x, y, w, h), None);
                h
            }
            Node::Message { title, words, link } => {
                self.message(title, words, link.as_ref(), x, y, w)
            }
            Node::Link { id, words } => {
                let style = self.style(Role::Link);
                let width = self.width_of(style, words).min(w);
                let h = style.line_height();
                self.push(
                    Part::Link {
                        text: words.clone(),
                        role: Role::Link,
                    },
                    Rect::new(x, y, width, h),
                    Some(*id),
                );
                h
            }
            Node::Numbered(items) => self.numbered(items, x, y, w),
            Node::Divider => {
                self.push(Part::Divider, Rect::new(x, y, w, LINE.0), None);
                LINE.0
            }
        }
    }

    /// The window in the corner (passport §3): its paddings, its parts one under another
    /// `CORNER_GAP` apart; answers how much higher it must be (the popup takes its height from it).
    fn corner_window(&mut self, window: &Window<I>, size: WindowSize) -> Shortfall {
        let pad = CORNER_PAD;
        let (x, w) = (pad.left.0, size.w - pad.left.0 - pad.right.0);
        let mut y = pad.top.0;
        let parts: Vec<&Node<I>> = match &window.body {
            Node::Column(children) => children.iter().collect(),
            other => vec![other],
        };
        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                y += CORNER_GAP.0;
            }
            y += self.node(part, x, y, w, None);
        }
        Shortfall {
            w: self.over,
            h: y + pad.bottom.0 - size.h,
        }
    }

    /// The header of the window in the corner: the icon, the words, the ✕ at the end.
    fn corner_header(&mut self, about: &str, close: I, x: f32, y: f32, w: f32) -> f32 {
        let row = CORNER_HEADER_ROW.0;
        let icon = CORNER_ICON.0;
        let gap = CORNER_ICON_GAP.0;
        self.push(
            Part::AppIcon,
            Rect::new(x, y + (row - icon) / 2.0, icon, icon),
            None,
        );
        let style = self.style(Role::CornerHeader);
        let words_x = x + icon + gap;
        let words_w = (w - icon - gap * 2.0 - row).max(0.0);
        self.push(
            Part::Text {
                role: Role::CornerHeader,
                text: about.to_owned(),
                wrap: Wrap::OneLine,
                align: Align::Start,
            },
            Rect::new(
                words_x,
                y + (row - style.line_height()) / 2.0,
                words_w,
                style.line_height(),
            ),
            None,
        );
        self.push(
            Part::Cross,
            Rect::new(x + w - row, y, row, row),
            Some(close),
        );
        row
    }

    /// The message of the window in the corner: its picture in a circle at the start, the title,
    /// the text and the link beside it.
    fn message(
        &mut self,
        title: &str,
        text: &str,
        link: Option<&(I, String)>,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let circle = CORNER_ART.0;
        let (mut column_x, mut column_w) = (x, w);
        if let Some(art) = self.art {
            self.push(Part::Art(art), Rect::new(x, y, circle, circle), None);
            column_x += circle + CORNER_GAP.0;
            column_w -= circle + CORNER_GAP.0;
        }
        let mut h = self.text(Role::CornerTitle, title, column_x, y, column_w, Wrap::Words);
        h += CORNER_TITLE_GAP.0;
        h += self.text(
            Role::CornerText,
            text,
            column_x,
            y + h,
            column_w,
            Wrap::Words,
        );
        if let Some((id, label)) = link {
            let style = self.style(Role::Link);
            let line = style.line_height();
            let width = self.width_of(style, label).min(column_w);
            self.push(
                Part::Link {
                    text: label.clone(),
                    role: Role::Link,
                },
                Rect::new(column_x, y + h, width, line),
                Some(*id),
            );
            h += line;
        }
        h.max(circle)
    }

    /// The buttons of the window in the corner: the second one share at the start, the main two
    /// at the end; one under another, the main first, when a label does not fit its share.
    fn corner_buttons(&mut self, buttons: &[Node<I>], x: f32, y: f32, w: f32) -> f32 {
        let main = buttons.iter().find(|node| {
            matches!(
                node,
                Node::Button {
                    kind: ButtonKind::Primary,
                    ..
                }
            )
        });
        let second = buttons.iter().find(|node| {
            matches!(
                node,
                Node::Button {
                    kind: ButtonKind::Normal,
                    ..
                }
            )
        });
        let height =
            |this: &Self, node: &Node<I>| this.button_size(node).1.max(CORNER_BUTTON_MIN_HEIGHT.0);
        let gap = CORNER_BUTTON_GAP.0;
        let (one, two) = (CORNER_SECOND_SHARE.0, CORNER_MAIN_SHARE.0);
        match (second, main) {
            (Some(second), Some(main)) => {
                let unit = (w - gap) / (one + two);
                let (second_w, main_w) = (unit * one, unit * two);
                let fits =
                    self.button_size(second).0 <= second_w && self.button_size(main).0 <= main_w;
                let h = height(self, main).max(height(self, second));
                if fits {
                    self.button(second, Rect::new(x, y, second_w, h));
                    self.button(main, Rect::new(x + second_w + gap, y, main_w, h));
                    h
                } else {
                    let (main_h, second_h) = (height(self, main), height(self, second));
                    self.button(main, Rect::new(x, y, w, main_h));
                    self.button(second, Rect::new(x, y + main_h + gap, w, second_h));
                    main_h + gap + second_h
                }
            }
            (None, Some(only)) | (Some(only), None) => {
                let h = height(self, only);
                self.button(only, Rect::new(x, y, w, h));
                h
            }
            (None, None) => 0.0,
        }
    }

    /// The menu of the icon (passport §3, T-120-15): its paddings, its head, items and lines one
    /// under another; answers how much higher it must be (the popup takes its height from it).
    fn menu_window(&mut self, window: &Window<I>, size: WindowSize) -> Shortfall {
        let pad = MENU_PAD.0;
        let (x, w) = (pad, size.w - pad * 2.0);
        let mut y = pad;
        let parts: Vec<&Node<I>> = match &window.body {
            Node::Column(children) => children.iter().collect(),
            other => vec![other],
        };
        for part in parts {
            y += self.node(part, x, y, w, None);
        }
        Shortfall {
            w: self.over,
            h: y + pad - size.h,
        }
    }

    /// The head of the menu: the icon and «LANG SWITCHER» at the start, the state at the end.
    fn menu_head(&mut self, state: MenuState, words: &str, x: f32, y: f32, w: f32) -> f32 {
        let (pad_h, pad_v) = (MENU_HEAD_PAD.w.0, MENU_HEAD_PAD.h.0);
        let name = self.style(Role::CornerHeader);
        let small = self.style(Role::Small);
        let icon = CORNER_ICON.0;
        let row = icon.max(name.line_height()).max(small.line_height());
        let middle = |h: f32| y + pad_v + (row - h) / 2.0;
        self.push(
            Part::AppIcon,
            Rect::new(x + pad_h, middle(icon), icon, icon),
            None,
        );
        let words_w = self.width_of(small, words);
        let words_x = x + w - pad_h - words_w;
        let dot = MENU_STATE_DOT.0;
        let dot_x = words_x - MENU_STATE_GAP.0 - dot;
        let name_x = x + pad_h + icon + CORNER_ICON_GAP.0;
        self.push(
            Part::Text {
                role: Role::CornerHeader,
                text: crate::APP_NAME.to_owned(),
                wrap: Wrap::OneLine,
                align: Align::Start,
            },
            Rect::new(
                name_x,
                middle(name.line_height()),
                (dot_x - CORNER_ICON_GAP.0 - name_x).max(0.0),
                name.line_height(),
            ),
            None,
        );
        self.push(
            Part::Dot(status_of(state)),
            Rect::new(dot_x, middle(dot), dot, dot),
            None,
        );
        self.push(
            Part::Text {
                role: Role::Small,
                text: words.to_owned(),
                wrap: Wrap::OneLine,
                align: Align::Start,
            },
            Rect::new(
                words_x,
                middle(small.line_height()),
                words_w,
                small.line_height(),
            ),
            None,
        );
        row + pad_v * 2.0
    }

    /// The width the head of the menu needs on one line.
    fn menu_head_width(&self, words: &str) -> f32 {
        let name = self.width_of(self.style(Role::CornerHeader), crate::APP_NAME);
        let words = self.width_of(self.style(Role::Small), words);
        MENU_HEAD_PAD.w.0 * 2.0
            + CORNER_ICON.0
            + CORNER_ICON_GAP.0 * 2.0
            + name
            + MENU_STATE_DOT.0
            + MENU_STATE_GAP.0
            + words
    }

    /// The width an item of the menu needs on one line: paddings, icon, label, its end.
    fn menu_item_width(&self, label: &str, trailing: &Trailing, emphasis: bool) -> f32 {
        let role = if emphasis {
            Role::MenuEmphasis
        } else {
            Role::Body
        };
        let label = self.width_of(self.style(role), label);
        let end = match trailing {
            Trailing::None => 0.0,
            Trailing::Key(key) => MENU_ITEM_GAP.0 + self.width_of(self.style(Role::Small), key),
            Trailing::Dot => MENU_ITEM_GAP.0 + MENU_DOT.0,
            Trailing::Switch(_) => MENU_ITEM_GAP.0 + self.em(TOGGLE.w).max(TOGGLE_MIN.w.0),
        };
        MENU_ITEM_PAD.0 * 2.0 + MENU_ICON_COLUMN.0 + MENU_ITEM_GAP.0 + label + end
    }

    /// An item of the menu: the row that is pressed, the icon, the label, the end.
    #[allow(clippy::too_many_arguments)]
    fn menu_item(
        &mut self,
        id: I,
        icon: &str,
        label: &str,
        trailing: &Trailing,
        emphasis: bool,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let role = if emphasis {
            Role::MenuEmphasis
        } else {
            Role::Body
        };
        let style = self.style(role);
        let row = MENU_ROW_MIN_HEIGHT.0.max(style.line_height());
        let middle = |h: f32| y + (row - h) / 2.0;
        self.push(Part::MenuRow, Rect::new(x, y, w, row), Some(id));
        let side = MENU_ICON.0;
        let column_x = x + MENU_ITEM_PAD.0;
        self.push(
            Part::MenuIcon {
                name: icon.to_owned(),
                emphasis,
            },
            Rect::new(
                column_x + (MENU_ICON_COLUMN.0 - side) / 2.0,
                middle(side),
                side,
                side,
            ),
            Some(id),
        );
        let end_x = x + w - MENU_ITEM_PAD.0;
        let (end_w, end_h) = match trailing {
            Trailing::None => (0.0, 0.0),
            Trailing::Key(key) => {
                let small = self.style(Role::Small);
                (self.width_of(small, key), small.line_height())
            }
            Trailing::Dot => (MENU_DOT.0, MENU_DOT.0),
            Trailing::Switch(_) => (
                self.em(TOGGLE.w).max(TOGGLE_MIN.w.0),
                self.em(TOGGLE.h).max(TOGGLE_MIN.h.0),
            ),
        };
        let end = Rect::new(end_x - end_w, middle(end_h), end_w, end_h);
        match trailing {
            Trailing::None => {}
            Trailing::Key(key) => {
                self.push(
                    Part::Text {
                        role: Role::Small,
                        text: key.clone(),
                        wrap: Wrap::OneLine,
                        align: Align::Start,
                    },
                    end,
                    Some(id),
                );
            }
            Trailing::Dot => {
                self.push(Part::Dot(Status::Working), end, Some(id));
            }
            Trailing::Switch(on) => {
                self.push(Part::Toggle { on: *on }, end, Some(id));
            }
        }
        let label_x = column_x + MENU_ICON_COLUMN.0 + MENU_ITEM_GAP.0;
        let room = if end_w > 0.0 {
            end.x - MENU_ITEM_GAP.0 - label_x
        } else {
            end_x - label_x
        };
        self.push(
            Part::Text {
                role,
                text: label.to_owned(),
                wrap: Wrap::OneLine,
                align: Align::Start,
            },
            Rect::new(
                label_x,
                middle(style.line_height()),
                room.max(0.0),
                style.line_height(),
            ),
            Some(id),
        );
        row
    }

    /// The example with a key (passport §6 «Клавиша»): the word typed, the key in its frame, an
    /// arrow, the word fixed — the words in the monospace face, each in its own direction; the row
    /// mirrors with the window, and the arrow points the way the language reads.
    fn example(&mut self, typed: &str, key: &str, fixed: &str, x: f32, y: f32, w: f32) -> f32 {
        let code = self.style(Role::Code);
        let body = self.style(Role::Body);
        let key_w = self.width_of(code, key) + self.em(KEY_PAD.h) * 2.0;
        let key_h = code.line_height() + self.em(KEY_PAD.v) * 2.0 + KEY_BOTTOM.0;
        let row = key_h.max(code.line_height()).max(body.line_height());
        let middle = |h: f32| y + (row - h) / 2.0;
        let gap = self.em(CONTROL_GAP);
        let arrow = if self.rtl { "←" } else { "→" };
        let mut at = x;
        let word = |this: &mut Self, role: Role, text: &str, at: &mut f32| {
            let style = this.style(role);
            let width = this.width_of(style, text).min((x + w - *at).max(0.0));
            this.push(
                Part::Text {
                    role,
                    text: text.to_owned(),
                    wrap: Wrap::OneLine,
                    align: Align::Start,
                },
                Rect::new(*at, middle(style.line_height()), width, style.line_height()),
                None,
            );
            *at += width + gap;
        };
        word(self, Role::Code, typed, &mut at);
        self.push(
            Part::Key {
                label: key.to_owned(),
            },
            Rect::new(at, middle(key_h), key_w, key_h),
            None,
        );
        at += key_w + gap;
        word(self, Role::Body, arrow, &mut at);
        word(self, Role::Code, fixed, &mut at);
        row
    }

    /// Whether `node` stands out of the flow: over the picture of the panel, or the sections of the
    /// settings in their panel.
    fn out_of_flow(&self, node: &Node<I>) -> bool {
        match node {
            Node::OverPicture { .. } => self.art.is_some() && self.picture.is_some(),
            Node::Sections { .. } => self.sections_at.is_some(),
            _ => false,
        }
    }

    /// `element` in the named place `anchor` of the picture `art` drawn in `circle` (T-120-25): the
    /// place is taken from the coordinates of the picture into the window's by the fitting the
    /// canvas draws the picture with — its field fitted to the square around the circle inside the
    /// ring. Neither the picture nor what lies on it is mirrored: in a window that reads right to
    /// left each rectangle is reflected here in the circle's own axis, so the mirror of the whole
    /// window puts it back in its place on the picture.
    fn over_picture(&mut self, art: Art, circle: Rect, anchor: &str, element: &OnPicture) {
        let Some([left, top, width, height]) = crate::ui::art::anchor(art.name(), anchor) else {
            return;
        };
        let radius = circle.w / 2.0 - ART_RING.0;
        let side = radius * 2.0;
        let place = Rect::new(
            circle.x + circle.w / 2.0 - radius + left * side,
            circle.y + circle.h / 2.0 - radius + top * side,
            width * side,
            height * side,
        );
        let style = |size: f32| Style {
            size: size.max(TYPE_FLOOR.0),
            ..self.style(Role::Code).with_weight(ON_PICTURE_WEIGHT)
        };
        let mut placed: Vec<(Part, Rect)> = Vec::new();

        match element {
            OnPicture::Word { word, fixed } => {
                let style = style(place.h * ON_PICTURE_WORD.0);
                let inset = place.h * ON_PICTURE_WORD_INSET.0;
                let line = style.line_height();
                placed.push((
                    Part::PictureWord {
                        word: word.clone(),
                        size: style.size,
                        fixed: *fixed,
                    },
                    Rect::new(
                        place.x + inset,
                        place.y + (place.h - line) / 2.0,
                        (place.w - inset * 2.0).max(0.0),
                        line,
                    ),
                ));
            }
            OnPicture::Keys { labels, pressed } => {
                let style = style(place.h * ON_PICTURE_KEY_LABEL.0);
                let (pad, least) = (
                    place.h * ON_PICTURE_KEY_PAD.0,
                    place.h * ON_PICTURE_KEY_MIN.0,
                );
                let (key_h, gap) = (place.h * ON_PICTURE_KEY.0, place.h * ON_PICTURE_KEY_GAP.0);
                let widths: Vec<f32> = labels
                    .iter()
                    .map(|label| (self.width_of(style, label) + pad * 2.0).max(least))
                    .collect();
                let total =
                    widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
                let mut at = place.x + (place.w - total) / 2.0;
                for (label, width) in labels.iter().zip(widths) {
                    placed.push((
                        Part::PictureKey {
                            label: label.clone(),
                            size: style.size,
                            pressed: *pressed,
                        },
                        Rect::new(at, place.y + (place.h - key_h) / 2.0, width, key_h),
                    ));
                    at += width + gap;
                }
            }
        }

        for (part, mut rect) in placed {
            if self.rtl {
                rect.x = circle.x * 2.0 + circle.w - rect.x - rect.w;
            }
            self.push(part, rect, None);
        }
    }

    /// The card of a letter (passport §6): a nested surface; the label, the title and one line of
    /// the text cut with an ellipsis at the start; the buttons at the end, in the middle of its
    /// height.
    #[allow(clippy::too_many_arguments)]
    fn letter_card(
        &mut self,
        label: &str,
        title: &str,
        text: &str,
        buttons: &[Node<I>],
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let (pad_v, pad_h) = (self.em(NESTED_PAD.v), self.em(NESTED_PAD.h));
        let card = self.push(
            Part::Surface(Surface::Nested),
            Rect::new(x, y, w, 0.0),
            None,
        );
        let gap = self.em(BUTTON_GAP);
        let sizes: Vec<(f32, f32)> = buttons.iter().map(|node| self.button_size(node)).collect();
        let row_w = sizes.iter().map(|(bw, _)| bw).sum::<f32>()
            + gap * sizes.len().saturating_sub(1) as f32;
        let row_h = sizes.iter().map(|(_, bh)| *bh).fold(0.0, f32::max);
        let inner_w = w - pad_h * 2.0;
        let column_w = if sizes.is_empty() {
            inner_w
        } else {
            (inner_w - row_w - gap).max(0.0)
        };
        let column_x = x + pad_h;
        let mut h = self.text(
            Role::Label,
            label,
            column_x,
            y + pad_v,
            column_w,
            Wrap::OneLine,
        );
        h += self.text(
            Role::Strong,
            title,
            column_x,
            y + pad_v + h,
            column_w,
            Wrap::Words,
        );
        h += self.text(
            Role::Body,
            text,
            column_x,
            y + pad_v + h,
            column_w,
            Wrap::Ellipsis,
        );
        let inner_h = h.max(row_h);
        let mut at = x + w - pad_h - row_w;
        for (node, (bw, bh)) in buttons.iter().zip(sizes) {
            self.button(
                node,
                Rect::new(at, y + pad_v + (inner_h - bh) / 2.0, bw, bh),
            );
            at += bw + gap;
        }
        let total = inner_h + pad_v * 2.0;
        self.out[card].rect.h = total;
        total
    }

    fn column(
        &mut self,
        children: &[Node<I>],
        x: f32,
        y: f32,
        w: f32,
        slot: Option<f32>,
        gap: f32,
    ) -> f32 {
        let fills = children
            .iter()
            .filter(|child| matches!(child, Node::Fill(_)))
            .count();
        let fill = match slot {
            Some(total) if fills > 0 => {
                let fixed: f32 = children
                    .iter()
                    .filter(|child| !matches!(child, Node::Fill(_)))
                    .map(|child| self.height(child, w, None))
                    .sum();
                let flowing = children
                    .iter()
                    .filter(|child| !self.out_of_flow(child))
                    .count();
                let gaps = gap * flowing.saturating_sub(1) as f32;
                Some(((total - fixed - gaps) / fills as f32).max(0.0))
            }
            _ => None,
        };

        // What stands out of the flow (over the picture; the sections in the panel of the settings)
        // takes no room in it, and no gap either.
        let mut top = y;
        let mut first = true;
        for child in children {
            if self.out_of_flow(child) {
                self.node(child, x, top, w, None);
                continue;
            }
            if !first {
                top += gap;
            }
            first = false;
            let slot = if matches!(child, Node::Fill(_)) {
                fill
            } else {
                None
            };
            top += self.node(child, x, top, w, slot);
        }
        top - y
    }

    /// Side by side at their own widths, wrapping like the words of a line.
    fn row(&mut self, children: &[Node<I>], x: f32, y: f32, w: f32) -> f32 {
        let (gap, line_gap) = (self.em(BUTTON_GAP), self.em(BLOCK_ROW_GAP));

        // A field in a row takes what the others leave (the mock-up's `.inrow`: the field, then its
        // button beside it), the row aligned at its bottom — while the field keeps its least width;
        // narrower, the row wraps like words.
        let fields = children
            .iter()
            .filter(|child| matches!(child, Node::Field { .. }))
            .count();
        if fields > 0 {
            let fixed: f32 = children
                .iter()
                .filter(|child| !matches!(child, Node::Field { .. }))
                .map(|child| self.natural_width(child).min(w))
                .sum();
            let gaps = gap * children.len().saturating_sub(1) as f32;
            let rest = (w - fixed - gaps) / fields as f32;
            if rest >= self.em(ROW_FIELD_MIN) {
                let widths: Vec<f32> = children
                    .iter()
                    .map(|child| match child {
                        Node::Field { .. } => rest,
                        _ => self.natural_width(child).min(w),
                    })
                    .collect();
                let heights: Vec<f32> = children
                    .iter()
                    .zip(&widths)
                    .map(|(child, width)| self.height(child, *width, None))
                    .collect();
                let line = heights.iter().copied().fold(0.0, f32::max);
                let mut left = x;
                for ((child, width), h) in children.iter().zip(&widths).zip(&heights) {
                    self.node(child, left, y + line - h, *width, None);
                    left += width + gap;
                }
                return line;
            }
        }

        let (mut left, mut top, mut line) = (x, y, 0.0_f32);

        for child in children {
            let width = self.natural_width(child).min(w);
            if left > x && left + width > x + w {
                top += line + line_gap;
                left = x;
                line = 0.0;
            }
            line = line.max(self.node(child, left, top, width, None));
            left += width + gap;
        }
        top + line - y
    }

    /// The width a node wants on one line.
    fn natural_width(&self, node: &Node<I>) -> f32 {
        match node {
            Node::Button { .. } => self.button_size(node).0,
            Node::Link { words, .. } => self.width_of(self.style(Role::Link), words),
            Node::Line(role, text) | Node::Text(role, text) => {
                self.width_of(self.style(*role), text)
            }
            _ => f32::MAX,
        }
    }

    /// Equal columns while each is at least 20 B; narrower — one under another.
    fn columns(&mut self, nodes: &[&Node<I>], x: f32, y: f32, w: f32, slot: Option<f32>) -> f32 {
        let gap = self.em(BLOCK_GAP);
        let count = nodes.len() as f32;
        let each = (w - gap * (count - 1.0)) / count;

        // Step 2: a half narrower than its widest unbreakable content stands in a column too.
        let beside = columns_fit(nodes.len(), w, self.b)
            && !(self.gives(Concession::Pair)
                && nodes.iter().any(|node| self.overflows_at(node, each)));
        if !beside {
            let mut top = y;
            for (index, node) in nodes.iter().enumerate() {
                if index > 0 {
                    top += gap;
                }
                top += self.node(node, x, top, w, None);
            }
            return top - y;
        }

        let in_pair = std::mem::replace(&mut self.in_pair, true);
        let tallest = nodes
            .iter()
            .map(|node| self.height(node, each, None))
            .fold(0.0, f32::max);
        let h = slot.map_or(tallest, |slot| slot.max(tallest));
        for (index, node) in nodes.iter().enumerate() {
            self.node(node, x + (each + gap) * index as f32, y, each, Some(h));
        }
        self.in_pair = in_pair;
        h
    }

    fn surface(
        &mut self,
        kind: Surface,
        child: &Node<I>,
        x: f32,
        y: f32,
        w: f32,
        slot: Option<f32>,
    ) -> f32 {
        let pad = match kind {
            Surface::Block => BLOCK_PAD,
            Surface::Nested => NESTED_PAD,
            Surface::Note => NOTE_PAD,
        };
        let (v, h) = (self.em(pad.v), self.em(pad.h));
        let index = self.push(Part::Surface(kind), Rect::new(x, y, w, 0.0), None);
        let inner = self.node(child, x + h, y + v, w - h * 2.0, None);
        let height = (inner + v * 2.0).max(slot.unwrap_or(0.0));
        self.out[index].rect.h = height;
        height
    }

    fn header(
        &mut self,
        eyebrow: Option<&str>,
        title: &str,
        subtitle: Option<&str>,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        // Step 4 of §3а takes the circle away.
        let circle = self.one_column
            && self.art.is_some()
            && self.kind != WindowKind::Settings
            && !self.gives(Concession::Circle);
        let side = if circle { self.em(NARROW_ART) } else { 0.0 };
        let beside = if circle {
            side + self.em(BLOCK_GAP)
        } else {
            0.0
        };
        let (tx, tw) = (x + beside, w - beside);

        let first = self.out.len();
        let mut top = y;
        if let Some(eyebrow) = eyebrow {
            top += self.text(Role::Eyebrow, eyebrow, tx, top, tw, Wrap::OneLine);
            top += self.em(HEADER_TITLE_GAP);
        }
        top += self.text(Role::Title, title, tx, top, tw, Wrap::Words);
        if let Some(subtitle) = subtitle {
            top += self.em(HEADER_SUBTITLE_GAP);
            top += self.text(Role::Small, subtitle, tx, top, tw, Wrap::Words);
        }
        let text = top - y;

        if !circle {
            return text;
        }
        // The circle and the lines beside it share one middle.
        let h = text.max(side);
        for item in &mut self.out[first..] {
            item.rect.y += (h - text) / 2.0;
        }
        if let Some(art) = self.art {
            self.push(
                Part::Art(art),
                Rect::new(x, y + (h - side) / 2.0, side, side),
                None,
            );
        }
        h
    }

    fn text(&mut self, role: Role, text: &str, x: f32, y: f32, w: f32, wrap: Wrap) -> f32 {
        let style = self.style(role);
        let h = match wrap {
            Wrap::OneLine | Wrap::Ellipsis => style.line_height(),
            Wrap::Words => {
                let size = self.measure.measure(text, style, wrap, w);
                self.over = self.over.max(size.width - w);
                size.height
            }
        };
        let part = Part::Text {
            role,
            text: text.to_owned(),
            wrap,
            align: Align::Start,
        };
        self.push(part, Rect::new(x, y, w, h), None);
        h
    }

    /// The label a button shows: with the sign ↗ when it opens the browser.
    fn label(node: &Node<I>) -> String {
        match node {
            Node::Button {
                label,
                browser: true,
                ..
            } => format!("{label} ↗"),
            Node::Button { label, .. } => label.clone(),
            _ => String::new(),
        }
    }

    fn button_style(&self, node: &Node<I>) -> Style {
        let style = self.style(Role::Button);
        match node {
            Node::Button {
                kind: ButtonKind::Primary,
                ..
            } => style.with_weight(BUTTON_PRIMARY_WEIGHT),
            _ => style,
        }
    }

    /// A button in a place `w` wide: its own size while it fits; wider — the width of the place,
    /// its label on a second line once the order of giving way has come to step 5, cut and counted
    /// as running over before.
    fn fitted_button(&mut self, node: &Node<I>, w: f32) -> (f32, f32) {
        let (bw, bh) = self.button_size(node);
        if bw <= w {
            return (bw, bh);
        }
        if !self.gives(Concession::Wrap) {
            self.over = self.over.max(bw - w);
            return (w, bh);
        }
        let (v, h) = (self.em(BUTTON_PAD.v), self.em(BUTTON_PAD.h));
        let inner = (w - h * 2.0).max(0.0);
        let label = self.measure.measure(
            &Self::label(node),
            self.button_style(node),
            Wrap::Words,
            inner,
        );
        self.over = self.over.max(label.width - inner);
        (w, (label.height + v * 2.0).max(bh))
    }

    /// The width of `nodes` side by side.
    fn row_width(&self, nodes: &[&Node<I>]) -> f32 {
        nodes
            .iter()
            .map(|node| self.button_size(node).0)
            .sum::<f32>()
            + self.em(BUTTON_GAP) * nodes.len().saturating_sub(1) as f32
    }

    /// `nodes` side by side, pressed to the right of `w`; what runs over is counted.
    fn button_row(&mut self, nodes: &[&Node<I>], x: f32, y: f32, w: f32) -> f32 {
        let gap = self.em(BUTTON_GAP);
        let total = self.row_width(nodes);
        self.over = self.over.max(total - w);
        let mut left = x + w - total;
        let mut h = 0.0_f32;
        for node in nodes {
            let (bw, bh) = self.button_size(node);
            self.button(node, Rect::new(left, y, bw, bh));
            left += bw + gap;
            h = h.max(bh);
        }
        h
    }

    fn button_size(&self, node: &Node<I>) -> (f32, f32) {
        let style = self.button_style(node);
        let (v, h) = (self.em(BUTTON_PAD.v), self.em(BUTTON_PAD.h));
        let width = self.width_of(style, &Self::label(node)) + h * 2.0;
        let height = (style.line_height() + v * 2.0).max(self.em(BUTTON_MIN_HEIGHT));
        (width, height)
    }

    fn button(&mut self, node: &Node<I>, at: Rect) {
        if let Node::Button { id, kind, .. } = node {
            let part = Part::Button {
                kind: *kind,
                label: Self::label(node),
            };
            self.push(part, at, Some(*id));
        }
    }

    /// The row of buttons of a window. In the wizard the first of them («Отмена») stands at the
    /// left edge and the others to the right as in any window (passport §6), while the row fits on
    /// one line; else as in any window (§3а).
    fn window_buttons(&mut self, window: &Window<I>, x: f32, y: f32, w: f32) -> f32 {
        if window.kind == WindowKind::Wizard
            && let Some((first, rest)) = window.buttons.split_first()
            && !rest.is_empty()
        {
            let gap = self.em(BUTTON_GAP);
            let rest: Vec<&Node<I>> = rest.iter().collect();
            let (first_w, first_h) = self.button_size(first);
            if first_w + gap + self.row_width(&rest) <= w {
                self.button(first, Rect::new(x, y, first_w, first_h));
                let h = self.buttons(
                    &window.buttons[1..],
                    x + first_w + gap,
                    y,
                    w - first_w - gap,
                );
                return h.max(first_h);
            }
        }
        self.buttons(&window.buttons, x, y, w)
    }

    /// A row of buttons pressed to the right, the main one the rightmost. When it does not fit —
    /// §3а, step 1 — two rows, the main one in the lower at its edge; then one under another across
    /// the width, the main one on top, a label wider than the width on two lines (step 5).
    fn buttons(&mut self, buttons: &[Node<I>], x: f32, y: f32, w: f32) -> f32 {
        if self.kind == WindowKind::Corner {
            return self.corner_buttons(buttons, x, y, w);
        }
        let is_primary = |node: &&Node<I>| {
            matches!(
                node,
                Node::Button {
                    kind: ButtonKind::Primary,
                    ..
                }
            )
        };
        let gap = self.em(BUTTON_GAP);
        let mut order: Vec<&Node<I>> = buttons.iter().filter(|node| !is_primary(node)).collect();
        order.extend(buttons.iter().filter(is_primary));

        if self.row_width(&order) <= w || !self.gives(Concession::Buttons) {
            return self.button_row(&order, x, y, w);
        }
        if let Some((main, rest)) = order.split_last()
            && !rest.is_empty()
            && self.row_width(rest) <= w
            && self.row_width(&[*main]) <= w
        {
            let upper = self.button_row(rest, x, y, w);
            let lower = self.button_row(&[*main], x, y + upper + gap, w);
            return upper + gap + lower;
        }

        let mut top = y;
        for (index, node) in order.iter().rev().enumerate() {
            if index > 0 {
                top += gap;
            }
            let (_, bh) = self.fitted_button(node, w);
            self.button(node, Rect::new(x, top, w, bh));
            top += bh;
        }
        top - y
    }

    /// An action card: the title and the text, the buttons under them — or beside them, §3а step 3,
    /// in a pair standing side by side when this makes the card lower.
    fn action(
        &mut self,
        title: &str,
        text: &str,
        buttons: &[Node<I>],
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let gap = self.em(BLOCK_ROW_GAP);
        let under = |this: &mut Self| {
            let mut top = y + this.text(Role::Strong, title, x, y, w, Wrap::Words);
            top += gap + this.text(Role::Body, text, x, top + gap, w, Wrap::Words);
            if !buttons.is_empty() {
                top += gap + this.buttons(buttons, x, top + gap, w);
            }
            top - y
        };
        let beside_lowers = self.gives(Concession::Card)
            && self.in_pair
            && !buttons.is_empty()
            && self.trial(|this| this.action_beside(title, text, buttons, x, y, w))
                < self.trial(under);

        if !beside_lowers {
            let mut top = y + self.text(Role::Strong, title, x, y, w, Wrap::Words);
            top += gap + self.text(Role::Body, text, x, top + gap, w, Wrap::Words);
            if !buttons.is_empty() {
                top += gap + self.buttons(buttons, x, top + gap, w);
            }
            return top - y;
        }
        self.action_beside(title, text, buttons, x, y, w)
    }

    /// The card with its buttons beside the title and the text.
    fn action_beside(
        &mut self,
        title: &str,
        text: &str,
        buttons: &[Node<I>],
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let gap = self.em(BLOCK_ROW_GAP);
        let widest = buttons
            .iter()
            .map(|node| self.button_size(node).0)
            .fold(0.0, f32::max);
        let beside = if buttons.is_empty() {
            0.0
        } else {
            widest + self.em(BUTTON_GAP)
        };
        let tw = w - beside;
        let mut top = y + self.text(Role::Strong, title, x, y, tw, Wrap::Words);
        top += gap + self.text(Role::Body, text, x, top + gap, tw, Wrap::Words);

        let mut right = y;
        for (index, node) in buttons.iter().enumerate() {
            if index > 0 {
                right += self.em(BUTTON_GAP);
            }
            let (_, bh) = self.button_size(node);
            self.button(node, Rect::new(x + w - widest, right, widest, bh));
            right += bh;
        }
        (top - y).max(right - y)
    }

    /// A line with a control: a switch at its end, or a check box before the label. The control
    /// stands in the middle of the label's first line; the whole line is pressed.
    #[allow(clippy::too_many_arguments)]
    fn control(
        &mut self,
        id: I,
        (label, badge): (&str, Option<&str>),
        hint: Option<&str>,
        part: Part,
        (cw, ch): (f32, f32),
        before: bool,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let gap = self.em(CONTROL_GAP);
        // The check box of the window in the corner has its own size (§2).
        let role = if self.kind == WindowKind::Corner {
            Role::CornerCheck
        } else {
            Role::Body
        };
        let first = self.style(role).line_height();
        let line = first.max(ch);
        let (tx, cx) = if before {
            (x + cw + gap, x)
        } else {
            (x, x + w - cw)
        };
        let tw = w - cw - gap;

        let hit = self.push(Part::Hit, Rect::new(x, y, w, 0.0), Some(id));
        let lift = (line - first) / 2.0;
        let mut text = match badge {
            None => self.text(role, label, tx, y + lift, tw, Wrap::Words),
            // The label «new» after the words, on their line while it fits there; under them when
            // it does not.
            Some(badge) => {
                let badge_style = self.style(Role::Label);
                let gap = self.em(BADGE_GAP);
                let label_w = self.width_of(self.style(role), label);
                let badge_w = self.width_of(badge_style, badge).min(tw);
                let part = Part::Badge {
                    text: badge.to_owned(),
                };
                if label_w + gap + badge_w <= tw {
                    let h = self.text(role, label, tx, y + lift, label_w, Wrap::OneLine);
                    self.push(
                        part,
                        Rect::new(tx + label_w + gap, y + lift, badge_w, h),
                        None,
                    );
                    h
                } else {
                    let h = self.text(role, label, tx, y + lift, tw, Wrap::Words);
                    let line = badge_style.line_height();
                    self.push(part, Rect::new(tx, y + lift + h, badge_w, line), None);
                    h + line
                }
            }
        };
        if let Some(hint) = hint {
            text += self.text(Role::Small, hint, tx, y + lift + text, tw, Wrap::Words);
        }
        self.push(part, Rect::new(cx, y + (line - ch) / 2.0, cw, ch), Some(id));

        let h = (lift + text).max(line);
        self.out[hit].rect.h = h;
        h
    }

    #[allow(clippy::too_many_arguments)]
    fn segments(
        &mut self,
        id: I,
        label: &str,
        options: &[String],
        selected: usize,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let style = self.style(Role::Body);
        let (v, h) = (self.em(SEGMENT_PAD.v), self.em(SEGMENT_PAD.h));
        let count = options.len().max(1) as f32;
        let cell = options
            .iter()
            .map(|option| self.width_of(style, option))
            .fold(0.0, f32::max)
            + h * 2.0;
        let (track_w, track_h) = (cell * count, style.line_height() + v * 2.0);
        let gap = self.em(CONTROL_GAP);
        let label_w = self.width_of(style, label);

        if label_w + gap + track_w <= w {
            let line = style.line_height().max(track_h);
            self.text(
                Role::Body,
                label,
                x,
                y + (line - style.line_height()) / 2.0,
                w - track_w - gap,
                Wrap::Words,
            );
            self.track(
                id,
                options,
                selected,
                Rect::new(
                    x + w - track_w,
                    y + (line - track_h) / 2.0,
                    track_w,
                    track_h,
                ),
            );
            return line;
        }
        if track_w <= w {
            let above = self.text(Role::Body, label, x, y, w, Wrap::Words) + self.em(BLOCK_ROW_GAP);
            self.track(id, options, selected, Rect::new(x, y + above, w, track_h));
            return above + track_h;
        }

        self.choice(id, label, options, selected, x, y, w)
    }

    /// A choice card: the card is pressed as a whole; the radio button before the title and its line.
    #[allow(clippy::too_many_arguments)]
    fn choice_card(
        &mut self,
        id: I,
        title: &str,
        text: &str,
        selected: bool,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let (v, h) = (self.em(CHOICE_PAD.v), self.em(CHOICE_PAD.h));
        let card = self.push(Part::Card { selected }, Rect::new(x, y, w, 0.0), Some(id));
        let side = CHECK.at(self.b);
        let gap = self.em(CONTROL_GAP);
        let first = self.style(Role::Strong).line_height();
        let (tx, tw) = (x + h + side + gap, w - h * 2.0 - side - gap);
        let mut inner = self.text(Role::Strong, title, tx, y + v, tw, Wrap::Words);
        inner += self.text(Role::Small, text, tx, y + v + inner, tw, Wrap::Words);
        self.push(
            Part::Radio { on: selected },
            Rect::new(x + h, y + v + (first - side) / 2.0, side, side),
            None,
        );
        let height = inner + v * 2.0;
        self.out[card].rect.h = height;
        height
    }

    /// A numbered list: a column of numbers as wide as the widest, the items beside them.
    fn numbered(&mut self, items: &[String], x: f32, y: f32, w: f32) -> f32 {
        let style = self.style(Role::Body);
        let numbers: Vec<String> = (1..=items.len()).map(|n| format!("{n}.")).collect();
        let column = numbers
            .iter()
            .map(|n| self.width_of(style, n))
            .fold(0.0, f32::max)
            + self.em(CONTROL_GAP);
        let mut top = y;
        for (index, (number, item)) in numbers.into_iter().zip(items).enumerate() {
            if index > 0 {
                top += self.em(BLOCK_ROW_GAP);
            }
            self.push(
                Part::Number { text: number },
                Rect::new(x, top, column, style.line_height()),
                None,
            );
            top += self.text(Role::Body, item, x + column, top, w - column, Wrap::Words);
        }
        top - y
    }

    fn track(&mut self, id: I, options: &[String], selected: usize, at: Rect) {
        self.push(Part::Track, at, Some(id));
        let cell = at.w / options.len().max(1) as f32;
        for (index, option) in options.iter().enumerate() {
            let part = Part::Segment {
                index,
                selected: index == selected,
                label: option.clone(),
            };
            self.push(
                part,
                Rect::new(at.x + cell * index as f32, at.y, cell, at.h),
                Some(id),
            );
        }
    }

    /// A label and a choice field as wide as its widest option and its corner mark: the field
    /// does not change its width with the value (the live stand: «Eng…»).
    #[allow(clippy::too_many_arguments)]
    fn choice(
        &mut self,
        id: I,
        label: &str,
        options: &[String],
        selected: usize,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let style = self.style(Role::Body);
        let value = options.get(selected).map_or("", String::as_str);
        let (v, h) = (self.em(FIELD_PAD.v), self.em(FIELD_PAD.h));
        let widest = options
            .iter()
            .map(|option| self.width_of(style, option))
            .fold(0.0, f32::max);
        let (field_w, field_h) = (
            (widest + h * 2.0 + CHEVRON.at(self.b)).min(w),
            style.line_height() + v * 2.0,
        );
        let gap = self.em(CONTROL_GAP);
        let part = Part::Field {
            value: value.to_owned(),
        };

        if self.width_of(style, label) + gap + field_w <= w {
            let line = style.line_height().max(field_h);
            self.text(
                Role::Body,
                label,
                x,
                y + (line - style.line_height()) / 2.0,
                w - field_w - gap,
                Wrap::Words,
            );
            self.push(
                part,
                Rect::new(
                    x + w - field_w,
                    y + (line - field_h) / 2.0,
                    field_w,
                    field_h,
                ),
                Some(id),
            );
            return line;
        }

        let above = self.text(Role::Body, label, x, y, w, Wrap::Words) + self.em(BLOCK_ROW_GAP);
        self.push(part, Rect::new(x, y + above, w, field_h), Some(id));
        above + field_h
    }

    /// A text field: its label above it, the wrapper one line high or `FIELD_TEXT_LINES` lines.
    #[allow(clippy::too_many_arguments)]
    fn field(
        &mut self,
        id: I,
        label: &str,
        value: &str,
        hint: &str,
        kind: FieldKind,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let above = if label.is_empty() {
            0.0
        } else {
            self.text(Role::FieldLabel, label, x, y, w, Wrap::Words) + self.em(FIELD_LABEL_GAP)
        };
        let (pad, lines) = match kind {
            FieldKind::Line | FieldKind::Key { .. } => (FIELD_PAD, 1.0),
            FieldKind::Text => (MULTILINE_PAD, FIELD_TEXT_LINES.0),
        };
        let capture = match kind {
            FieldKind::Key { capturing } => Some(capturing),
            FieldKind::Line | FieldKind::Text => None,
        };
        let h = self.style(Role::Body).line_height() * lines + self.em(pad.v) * 2.0;
        self.push(
            Part::Input {
                multiline: kind == FieldKind::Text,
                capture,
                value: value.to_owned(),
                hint: hint.to_owned(),
            },
            Rect::new(x, y + above, w, h),
            Some(id),
        );
        above + h
    }

    /// A list: as many rows as its slot holds, or `LIST_VISIBLE_ROWS`, from `first` (kept inside
    /// the list); a thin scroll bar at the end when not every row is shown.
    #[allow(clippy::too_many_arguments)]
    fn list(
        &mut self,
        id: I,
        rows: &[ListItem],
        selected: usize,
        first: usize,
        x: f32,
        y: f32,
        w: f32,
        slot: Option<f32>,
    ) -> f32 {
        let style = self.style(Role::Body);
        let (pad, row_v, row_h) = (
            self.em(LIST_PAD),
            self.em(LIST_ROW_PAD.v),
            self.em(LIST_ROW_PAD.h),
        );
        let row = style.line_height() + row_v * 2.0;
        let total = rows.len();
        let room = match slot {
            Some(slot) => ((slot - pad * 2.0) / row).floor().max(1.0) as usize,
            None => LIST_VISIBLE_ROWS.0 as usize,
        };
        let visible = room.min(total).max(1);
        let first = first.min(total.saturating_sub(visible));
        // Given room (a fill: the mock-up's `.grow`), the frame takes all of it, the rows from its top.
        let h = slot.map_or(pad * 2.0 + row * visible as f32, |slot| {
            slot.max(pad * 2.0 + row * visible as f32)
        });
        let bar = if total > visible {
            LIST_BAR.0 + pad
        } else {
            0.0
        };
        let checked = rows.get(selected).and_then(|row| row.check);
        // The rows of a list with a value keep room for its tick at their end.
        let side = CHECK.at(self.b);
        let tick = if rows.iter().any(|row| row.mark) {
            side + self.em(CONTROL_GAP)
        } else {
            0.0
        };
        self.push(
            Part::List(ListView {
                selected,
                first,
                visible,
                total,
                checked,
            }),
            Rect::new(x, y, w, h),
            Some(id),
        );

        for (index, item) in rows.iter().enumerate().skip(first).take(visible) {
            let top = y + pad + row * (index - first) as f32;
            let end = x + w - pad - bar;
            self.push(
                Part::ListRow {
                    selected: index == selected,
                    index,
                    check: item.check,
                },
                Rect::new(x + pad, top, end - x - pad, row),
                Some(id),
            );
            if item.mark {
                self.push(
                    Part::Tick,
                    Rect::new(end - row_h - side, top + (row - side) / 2.0, side, side),
                    None,
                );
            }
            let mut at = x + pad + row_h;
            if let Some(on) = item.check {
                self.push(
                    Part::Check { on },
                    Rect::new(at, top + (row - side) / 2.0, side, side),
                    None,
                );
                at += side + self.em(CONTROL_GAP);
            }
            self.text(
                Role::Body,
                &item.label,
                at,
                top + row_v,
                end - row_h - tick - at,
                Wrap::OneLine,
            );
        }

        if bar > 0.0 {
            self.push(
                Part::Scroll {
                    first,
                    visible,
                    total,
                },
                Rect::new(x + w - pad - LIST_BAR.0, y + pad, LIST_BAR.0, h - pad * 2.0),
                None,
            );
        }
        h
    }

    /// The sections of the settings: a column, or in a narrow window a row of tabs that wraps.
    #[allow(clippy::too_many_arguments)]
    fn sections(&mut self, id: I, items: &[String], current: usize, x: f32, y: f32, w: f32) -> f32 {
        let style = self.style(Role::Body);
        let (v, h) = (self.em(SECTION_PAD.v), self.em(SECTION_PAD.h));
        let gap = self.em(SECTION_GAP);
        let item_h = style.line_height() + v * 2.0;
        let narrow = self.one_column;
        let (mut left, mut top) = (x, y);

        for (index, label) in items.iter().enumerate() {
            let width = if narrow {
                (self.width_of(style, label) + h * 2.0).min(w)
            } else {
                w
            };
            if narrow {
                if left > x && left + width > x + w {
                    left = x;
                    top += item_h + gap;
                }
            } else if index > 0 {
                top += item_h + gap;
            }
            let part = Part::Section {
                index,
                current: index == current,
                label: label.clone(),
            };
            self.push(part, Rect::new(left, top, width, item_h), Some(id));
            if narrow {
                left += width + gap;
            }
        }
        if items.is_empty() {
            0.0
        } else {
            top + item_h - y
        }
    }
}

/// One node placed alone at the left top corner of `width` points, for the element bench and the
/// tests of the elements: its parts and its height.
pub fn place_node<I: Copy>(
    node: &Node<I>,
    width: f32,
    b: f32,
    class: Class,
    measure: &dyn Measure,
) -> (Vec<Placed<I>>, f32) {
    let mut place = Placer {
        b,
        h: WINDOW_MIN.h.0,
        one_column: class == Class::Narrow,
        kind: WindowKind::Ordinary,
        art: None,
        picture: None,
        sections_at: None,
        given: 0,
        in_pair: false,
        measure,
        out: Vec::new(),
        rtl: false,
        over: 0.0,
    };
    let height = place.node(node, 0.0, 0.0, width, None);
    (place.out, height)
}
