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
use crate::ui::text::{Align, Measure, Style, Wrap};
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
use crate::ui::tokens::{BRAND_GAP, BRAND_NAME_WEIGHT, Face};

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
    Link {
        text: String,
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
            Part::ListRow { .. } => "list row",
            Part::Progress { .. } => "wizard progress",
            Part::Key { .. } => "key",
            Part::Dot(_) => "status dot",
            Part::Badge { .. } => "badge",
            Part::MenuRow => "menu row",
            Part::BrandName { .. } => "brand name",
            Part::Number { .. } => "list number",
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
    let b = tokens::window_b(size.w, size.h, k);
    let mut place = Placer {
        b,
        h: size.h,
        one_column: size.w < TWO_COLUMNS_FROM.0,
        kind: window.kind,
        art: window.art,
        tight: false,
        measure,
        out: Vec::new(),
        over: 0.0,
    };

    let mut shortfall = place.window(window, size);
    if shortfall.h > 0.0 {
        // The second step of the rule of the pair: the buttons of the action cards move beside
        // their text.
        place.tight = true;
        place.out.clear();
        place.over = 0.0;
        shortfall = place.window(window, size);
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
    }
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

/// What places the nodes of one window.
struct Placer<'m, I> {
    b: f32,
    /// The window's height: the brand line is sized by it.
    h: f32,
    /// The window stands in one column: the circle beside the header, the sections as tabs.
    one_column: bool,
    kind: WindowKind,
    art: Option<Art>,
    tight: bool,
    measure: &'m dyn Measure,
    out: Vec<Placed<I>>,
    /// The widest overflow met: a run of text wider than its room.
    over: f32,
}

impl<I: Copy> Placer<'_, I> {
    fn em(&self, value: Em) -> f32 {
        value.at(self.b)
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
        // One column under the width of two (208.168: 880), whatever the class of type sizes.
        let narrow = size.w < TWO_COLUMNS_FROM.0;
        let (mut area_x, mut area_w) = (0.0, size.w);

        if let (Some(art), false) = (window.art, narrow) {
            let panel = size.w * PANEL_SHARE.0;
            self.push(Part::Panel, Rect::new(0.0, 0.0, panel, size.h), None);
            let circle = match window.kind {
                WindowKind::Settings => SETTINGS_ART,
                _ => PANEL_ART,
            };
            let d = circle.at(size.w, size.h);
            let at = Rect::new((panel - d) / 2.0, (size.h - d) / 2.0, d, d);
            self.push(Part::Art(art), at, None);

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
                self.push(
                    Part::BrandName {
                        text: brand.name.clone(),
                    },
                    Rect::new(at.x, at.y, name_w, line),
                    None,
                );
                self.push(
                    Part::Text {
                        role: Role::Brand,
                        text: brand.address.clone(),
                        wrap: Wrap::OneLine,
                        align: Align::Start,
                    },
                    Rect::new(
                        at.x + name_w + gap,
                        at.y,
                        (at.w - name_w - gap).max(0.0),
                        line,
                    ),
                    None,
                );
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
            self.buttons(&window.buttons, x, buttons, column);
        }

        Shortfall {
            w: self.over,
            h: natural - room,
        }
    }

    /// Places `node` at `x`, `y` in `w`; `slot` is the height it must take, where it has one.
    /// Answers the height it took.
    fn node(&mut self, node: &Node<I>, x: f32, y: f32, w: f32, slot: Option<f32>) -> f32 {
        match node {
            Node::Column(children) => self.column(children, x, y, w, slot, self.em(BLOCK_GAP)),
            Node::Rows(children) => self.column(children, x, y, w, None, self.em(BLOCK_ROW_GAP)),
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
                let (bw, bh) = self.button_size(node);
                self.button(node, Rect::new(x, y, bw.min(w), bh));
                bh
            }
            Node::Buttons(buttons) => self.buttons(buttons, x, y, w),
            Node::Action {
                title,
                text,
                buttons,
            } => self.action(title, text, buttons, x, y, w),
            Node::Toggle {
                id,
                label,
                hint,
                on,
            } => {
                let track = (
                    self.em(TOGGLE.w).max(TOGGLE_MIN.w.0),
                    self.em(TOGGLE.h).max(TOGGLE_MIN.h.0),
                );
                self.control(
                    *id,
                    label,
                    hint.as_deref(),
                    Part::Toggle { on: *on },
                    track,
                    false,
                    x,
                    y,
                    w,
                )
            }
            Node::Check { id, label, on } => {
                let side = CHECK.at(self.b);
                self.control(
                    *id,
                    label,
                    None,
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
                    label,
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
                text,
                selected,
            } => self.choice_card(*id, title, text, *selected, x, y, w),
            Node::Segments {
                id,
                label,
                options,
                selected,
            } => self.segments(*id, label, options, *selected, x, y, w),
            Node::Choice { id, label, value } => self.choice(*id, label, value, x, y, w),
            Node::Sections { id, items, current } => self.sections(*id, items, *current, x, y, w),
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
            Node::Link { id, text } => {
                let style = self.style(Role::Link);
                let width = self.width_of(style, text).min(w);
                let h = style.line_height();
                self.push(
                    Part::Link { text: text.clone() },
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
                let gaps = gap * children.len().saturating_sub(1) as f32;
                Some(((total - fixed - gaps) / fills as f32).max(0.0))
            }
            _ => None,
        };

        let mut top = y;
        for (index, child) in children.iter().enumerate() {
            if index > 0 {
                top += gap;
            }
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
            Node::Link { text, .. } => self.width_of(self.style(Role::Link), text),
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

        if !columns_fit(nodes.len(), w, self.b) {
            let mut top = y;
            for (index, node) in nodes.iter().enumerate() {
                if index > 0 {
                    top += gap;
                }
                top += self.node(node, x, top, w, None);
            }
            return top - y;
        }

        let tallest = nodes
            .iter()
            .map(|node| self.height(node, each, None))
            .fold(0.0, f32::max);
        let h = slot.map_or(tallest, |slot| slot.max(tallest));
        for (index, node) in nodes.iter().enumerate() {
            self.node(node, x + (each + gap) * index as f32, y, each, Some(h));
        }
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
        let circle = self.one_column && self.art.is_some() && self.kind != WindowKind::Settings;
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
            Wrap::OneLine => style.line_height(),
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

    fn button_size(&self, node: &Node<I>) -> (f32, f32) {
        let mut style = self.style(Role::Button);
        if let Node::Button {
            kind: ButtonKind::Primary,
            ..
        } = node
        {
            style = style.with_weight(BUTTON_PRIMARY_WEIGHT);
        }
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

    /// A row of buttons pressed to the right, the main one the rightmost; when they do not fit —
    /// one under another across the width, the main one on top.
    fn buttons(&mut self, buttons: &[Node<I>], x: f32, y: f32, w: f32) -> f32 {
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

        let sizes: Vec<(f32, f32)> = order.iter().map(|node| self.button_size(node)).collect();
        let total = sizes.iter().map(|size| size.0).sum::<f32>()
            + gap * sizes.len().saturating_sub(1) as f32;

        if total <= w {
            let mut left = x + w - total;
            let h = sizes.iter().map(|size| size.1).fold(0.0, f32::max);
            for (node, (bw, bh)) in order.iter().zip(&sizes) {
                self.button(node, Rect::new(left, y, *bw, *bh));
                left += bw + gap;
            }
            return h;
        }

        let mut top = y;
        for (index, (node, (_, bh))) in order.iter().zip(&sizes).rev().enumerate() {
            if index > 0 {
                top += gap;
            }
            self.button(node, Rect::new(x, top, w, *bh));
            top += bh;
        }
        top - y
    }

    /// An action card: the title and the text, the buttons under them — or beside them when the
    /// window is tight.
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

        if !self.tight {
            let mut top = y + self.text(Role::Strong, title, x, y, w, Wrap::Words);
            top += gap + self.text(Role::Body, text, x, top + gap, w, Wrap::Words);
            if !buttons.is_empty() {
                top += gap + self.buttons(buttons, x, top + gap, w);
            }
            return top - y;
        }

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
        label: &str,
        hint: Option<&str>,
        part: Part,
        (cw, ch): (f32, f32),
        before: bool,
        x: f32,
        y: f32,
        w: f32,
    ) -> f32 {
        let gap = self.em(CONTROL_GAP);
        let first = self.style(Role::Body).line_height();
        let line = first.max(ch);
        let (tx, cx) = if before {
            (x + cw + gap, x)
        } else {
            (x, x + w - cw)
        };
        let tw = w - cw - gap;

        let hit = self.push(Part::Hit, Rect::new(x, y, w, 0.0), Some(id));
        let lift = (line - first) / 2.0;
        let mut text = self.text(Role::Body, label, tx, y + lift, tw, Wrap::Words);
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

        let value = options.get(selected).cloned().unwrap_or_default();
        self.choice(id, label, &value, x, y, w)
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

    fn choice(&mut self, id: I, label: &str, value: &str, x: f32, y: f32, w: f32) -> f32 {
        let style = self.style(Role::Body);
        let (v, h) = (self.em(FIELD_PAD.v), self.em(FIELD_PAD.h));
        let (field_w, field_h) = (
            self.width_of(style, value) + h * 2.0,
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
            FieldKind::Line => (FIELD_PAD, 1.0),
            FieldKind::Text => (MULTILINE_PAD, FIELD_TEXT_LINES.0),
        };
        let h = self.style(Role::Body).line_height() * lines + self.em(pad.v) * 2.0;
        self.push(
            Part::Input {
                multiline: kind == FieldKind::Text,
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
        let h = pad * 2.0 + row * visible as f32;
        let bar = if total > visible {
            LIST_BAR.0 + pad
        } else {
            0.0
        };
        let checked = rows.get(selected).and_then(|row| row.check);
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
            let mut at = x + pad + row_h;
            if let Some(on) = item.check {
                let side = CHECK.at(self.b);
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
                end - row_h - at,
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
        tight: false,
        measure,
        out: Vec::new(),
        over: 0.0,
    };
    let height = place.node(node, 0.0, 0.0, width, None);
    (place.out, height)
}
