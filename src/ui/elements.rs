//! The elements of the interface shell: the look of every part of a laid-out window, **one
//! definition each** (passport §6), and the states of §5 by **one mechanism** for all of them.
//!
//! An element has no parameter «colour», «type size» or «padding»: what it takes is its kind (the
//! [`Part`] the layout placed), its [`State`], its text and the [`Look`] of the window — the theme,
//! the direction, the type size B. Every number and colour comes from `tokens` (`tests\ui_guard.rs`,
//! rules 1 and 4).
//!
//! # The states, once for all (§5)
//!
//! * **under the mouse, pressed** — a fill one, then two steps along `shape::FILL_STEPS`
//!   (`surface` → `surface2` → `line`); an element with no fill of its own starts before the first
//!   step. The main button alone is mixed with `bg` instead: `HOVER_MIX`, `PRESSED_MIX`;
//! * **with the keyboard ring** — the ring of §4 around the element, `FOCUS_GAP` outside it;
//! * **unavailable** — text `ink3`, no fill, frames `line`, and no area that answers a press
//!   ([`hit_area`]).
//!
//! A switch, a check box, a radio button and a choice card answer a press on **their whole line or
//! card** (lesson of stage E81): the layout places that line as a [`Part::Hit`] or the card as a
//! [`Part::Card`], and it is that part that takes the states of the mouse.
//!
//! Knows nothing of particular windows.

use crate::ui::canvas::{Painter, Rect};
use crate::ui::layout::{Part, Status};
use crate::ui::scene::{ButtonKind, LINK_BEGIN, Surface};
use crate::ui::text::{Align, Direction, Measure, Style, Text, Wrap};
use crate::ui::tokens::shape::{CHECK_TICK, CHEVRON, FILL_STEPS, RADIO_DOT};
use crate::ui::tokens::{
    ART_RING, BUTTON_PRIMARY_WEIGHT, CHECK_BORDER, CURRENT_STRIPE, Color, FIELD_PAD, FOCUS_GAP,
    FOCUS_RING, HOVER_MIX, KEY_BOTTOM, LINE, LINK_UNDERLINE, MULTILINE_PAD, PANEL_DOT,
    PANEL_DOT_STEP, PRESSED_MIX, RADIUS_CHECK, RADIUS_MEDIUM, RADIUS_SMALL, Rgb, Role, SECTION_PAD,
    SEGMENT_HOVER_INSET, SELECTED_BORDER, STATUS_DOT, TOGGLE_GAP, TOGGLE_KNOB, Theme,
};
use crate::ui::tokens::{BRAND_NAME_COLOR, BRAND_NAME_WEIGHT, CORNER_CROSS, Face};
use crate::ui::tokens::{BUTTON_PAD, CHOICE_ICON_GLYPH};
use crate::ui::tokens::{
    ON_PICTURE_KEY_FRAME, ON_PICTURE_KEY_RADIUS, ON_PICTURE_WEIGHT, picture_colours,
};

/// What every element of one window shares: the theme, the direction, the type size and the faces.
pub struct Look<'t> {
    pub theme: Theme,
    /// The language reads right to left: the window is mirrored.
    pub rtl: bool,
    /// The type size B.
    pub b: f32,
    /// The window's height, for the brand line.
    pub h: f32,
    pub text: &'t Text,
}

impl Look<'_> {
    fn colour(&self, colour: Color) -> Rgb {
        colour.get(self.theme)
    }
}

/// The state of an element, by the mouse, the keyboard and availability (§5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub hover: bool,
    pub pressed: bool,
    pub focus: bool,
    pub disabled: bool,
}

impl State {
    pub const NORMAL: Self = Self {
        hover: false,
        pressed: false,
        focus: false,
        disabled: false,
    };
    pub const HOVER: Self = Self {
        hover: true,
        ..Self::NORMAL
    };
    pub const PRESSED: Self = Self {
        hover: true,
        pressed: true,
        ..Self::NORMAL
    };
    pub const FOCUS: Self = Self {
        focus: true,
        ..Self::NORMAL
    };
    pub const DISABLED: Self = Self {
        disabled: true,
        ..Self::NORMAL
    };
    /// Every state of §5 an element can be drawn in.
    pub const ALL: &[Self] = &[
        Self::NORMAL,
        Self::HOVER,
        Self::PRESSED,
        Self::FOCUS,
        Self::DISABLED,
    ];

    /// How many steps the mouse moves a fill.
    fn steps(self) -> usize {
        if self.disabled {
            0
        } else if self.pressed {
            2
        } else {
            usize::from(self.hover)
        }
    }
}

/// `base` moved `state`'s steps along the fill steps; `None` is no fill. Unavailable — no fill.
fn stepped(base: Option<Color>, state: State) -> Option<Color> {
    stepped_by(base, state, state.steps())
}

/// [`stepped`] by `steps` steps of the fill rather than the ones of `state`.
fn stepped_by(base: Option<Color>, state: State, steps: usize) -> Option<Color> {
    if state.disabled {
        return None;
    }
    let start = match base {
        Some(colour) => match FILL_STEPS.iter().position(|step| *step == colour) {
            Some(at) => at,
            None => return Some(colour),
        },
        None if steps == 0 => return None,
        None => 0,
    };
    let at = (start + steps).min(FILL_STEPS.len() - 1);
    Some(FILL_STEPS[at])
}

/// The area that answers a press for `part` at `rect`: the whole line or card for the controls
/// that have one, none for what is not pressed or is unavailable.
pub fn hit_area(part: &Part, rect: Rect, state: State) -> Option<Rect> {
    if state.disabled {
        return None;
    }
    match part {
        Part::Hit
        | Part::Card { .. }
        | Part::Button { .. }
        | Part::Link { .. }
        | Part::InlineLink
        | Part::Segment { .. }
        | Part::Field { .. }
        | Part::Input { .. }
        | Part::Cross
        | Part::Section { .. }
        | Part::ListRow { .. }
        | Part::MenuRow => Some(rect),
        _ => None,
    }
}

/// ⭐ **The thumb of a scroll bar** at `bar` — what the bar draws and what the mouse takes hold of
/// (решение 208.235: «за палочку не тянется»): as long as the share of the rows shown, no shorter
/// than twice the bar is wide, at the share of the way the first row shown has gone. None when every
/// row is shown. One body for the drawing and the dragging, so the thumb taken is the thumb seen.
pub fn scroll_thumb(bar: Rect, first: usize, visible: usize, total: usize) -> Option<Rect> {
    if total <= visible {
        return None;
    }
    let length = (bar.h * visible as f32 / total as f32).max(bar.w * 2.0);
    let travel = bar.h - length;
    let at = travel * first as f32 / (total - visible) as f32;
    Some(Rect::new(bar.x, bar.y + at, bar.w, length))
}

/// The first row shown when the thumb of `bar` has its top at `top` — the reverse of
/// [`scroll_thumb`], kept inside the list: the thumb dragged above the bar is the first row, below
/// it the last page.
pub fn scroll_first_at(bar: Rect, visible: usize, total: usize, top: f32) -> usize {
    let Some(thumb) = scroll_thumb(bar, 0, visible, total) else {
        return 0;
    };
    let travel = bar.h - thumb.h;
    if travel <= 0.0 {
        return 0;
    }
    let share = ((top - bar.y) / travel).clamp(0.0, 1.0);
    (share * (total - visible) as f32).round() as usize
}

/// Whether the keyboard ring is drawn around this part when it has the focus.
///
/// A group that is one stop of Tab — the sections of the settings, the segments of a switch, the
/// rows of a list — rings its current element alone, as the radio buttons of Windows do, and the
/// arrows move it (AUDIT-180b п. 2, решение 208.227): not every section, the whole track or the
/// whole list.
fn takes_ring(part: &Part) -> bool {
    matches!(
        part,
        Part::Hit
            | Part::Card { .. }
            | Part::Button { .. }
            | Part::Link { .. }
            | Part::InlineLink
            | Part::Field { .. }
            | Part::Section { current: true, .. }
            | Part::Segment { selected: true, .. }
            | Part::ListRow { selected: true, .. }
            | Part::Cross
    )
}

/// Draws `part` at `rect` in `state`.
pub fn draw(painter: &mut Painter, look: &Look, part: &Part, rect: Rect, state: State) {
    let c = |colour| look.colour(colour);
    let line = LINE.0;

    match part {
        Part::Panel => {
            painter.fill_rect(rect, 0.0, c(Color::Surface2));
            painter.dots(rect, PANEL_DOT.0, PANEL_DOT_STEP.0, c(Color::Line2));
            let edge = if look.rtl {
                rect.x
            } else {
                rect.right() - line
            };
            painter.fill_rect(Rect::new(edge, rect.y, line, rect.h), 0.0, c(Color::Line));
        }
        Part::Art(art) => {
            // The ring of `bg` around the picture (§3), the picture inside it.
            let (x, y, radius) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, rect.w / 2.0);
            painter.fill_circle(x, y, radius, c(Color::Bg));
            painter.picture(art.name(), x, y, radius - ART_RING.0);
        }
        Part::Surface(kind) => {
            let (fill, frame) = match kind {
                Surface::Block => (Color::Surface, Color::Line),
                Surface::Nested => (Color::Surface2, Color::Line),
                Surface::Note => (Color::Note, Color::NoteLine),
            };
            painter.fill_rect(rect, RADIUS_MEDIUM.0, c(fill));
            painter.stroke_rect(rect, RADIUS_MEDIUM.0, line, c(frame));
        }
        Part::Text {
            role,
            text,
            wrap,
            align,
        } => {
            let colour = if state.disabled {
                Color::Ink3
            } else {
                role.style().color.unwrap_or(Color::Text)
            };
            // A paragraph with a link (208.217): the words of the link in the colour of a link,
            // underlined by the layout of the text; the press is the parts' over them.
            if text.contains(LINK_BEGIN) {
                if let Ok(laid) = look.text.lay(
                    text,
                    Style::of(*role, look.b, look.h),
                    *wrap,
                    *align,
                    rect.w,
                ) {
                    painter.tint_links(&laid, c(Color::Accent));
                    painter.text(&laid, rect.x, rect.y, c(colour));
                }
            } else {
                write(
                    painter,
                    look,
                    *role,
                    None,
                    text,
                    rect,
                    *wrap,
                    *align,
                    false,
                    c(colour),
                );
            }
        }
        // The paragraph draws the words; the part answers the press and takes the ring.
        Part::InlineLink => {}
        Part::Button { kind, label } => button(painter, look, *kind, label, rect, state),
        Part::ButtonRule | Part::Divider => painter.fill_rect(rect, 0.0, c(Color::Line)),
        Part::Toggle { on } => {
            let track = match (state.disabled, on) {
                (true, _) => Color::Line,
                (false, true) => Color::Accent,
                (false, false) => Color::Edge,
            };
            painter.fill_rect(rect, rect.h / 2.0, c(track));
            let gap = TOGGLE_GAP.at(look.b);
            let knob = TOGGLE_KNOB.at(look.b).min(rect.h - gap * 2.0);
            let right = *on != look.rtl;
            let x = if right {
                rect.right() - gap - knob / 2.0
            } else {
                rect.x + gap + knob / 2.0
            };
            painter.fill_circle(x, rect.y + rect.h / 2.0, knob / 2.0, c(Color::Knob));
        }
        Part::Check { on } => {
            let (fill, frame) = match (state.disabled, on) {
                (true, _) => (None, Color::Line),
                (false, true) => (Some(Color::Accent), Color::Accent),
                (false, false) => (Some(Color::Surface), Color::Edge),
            };
            if let Some(fill) = fill {
                painter.fill_rect(rect, RADIUS_CHECK.0, c(fill));
            }
            painter.stroke_rect(rect, RADIUS_CHECK.0, CHECK_BORDER.0, c(frame));
            if *on {
                let tick = if state.disabled {
                    Color::Ink3
                } else {
                    Color::Bg
                };
                let point = |at: usize| {
                    (
                        rect.x + rect.w * CHECK_TICK[at].0,
                        rect.y + rect.h * CHECK_TICK[at].1,
                    )
                };
                let (a, b, d) = (point(0), point(1), point(2));
                painter.line(a.0, a.1, b.0, b.1, CHECK_BORDER.0, c(tick));
                painter.line(b.0, b.1, d.0, d.1, CHECK_BORDER.0, c(tick));
            }
        }
        Part::AppIcon => painter.app_icon(rect),
        Part::MenuIcon { name, emphasis } => {
            let colour = if state.disabled {
                Color::Ink3
            } else if *emphasis {
                Color::Accent
            } else {
                Color::Ink2
            };
            painter.menu_icon(name, rect, c(colour));
        }
        // The icon of a choice card (208.132): a plate of `surface2`, the icon `accent` in its
        // middle; grey with the card when it is unavailable.
        Part::CardIcon { name } => {
            painter.fill_rect(rect, RADIUS_MEDIUM.0, c(Color::Surface2));
            let side = CHOICE_ICON_GLYPH.at(look.b);
            let glyph = Rect::new(
                rect.x + (rect.w - side) / 2.0,
                rect.y + (rect.h - side) / 2.0,
                side,
                side,
            );
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Accent
            };
            painter.menu_icon(name, glyph, c(colour));
        }
        Part::Taskbar(picture) => crate::ui::picture::draw(painter, look, picture, rect),
        Part::Cross => {
            if let Some(fill) = stepped(None, state) {
                painter.fill_rect(rect, RADIUS_SMALL.0, c(fill));
            }
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Ink2
            };
            let half = CORNER_CROSS.0 / 2.0;
            let (cx, cy) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
            painter.line(
                cx - half,
                cy - half,
                cx + half,
                cy + half,
                CHECK_BORDER.0,
                c(colour),
            );
            painter.line(
                cx - half,
                cy + half,
                cx + half,
                cy - half,
                CHECK_BORDER.0,
                c(colour),
            );
        }
        Part::Tick => {
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Accent
            };
            let point = |at: usize| {
                (
                    rect.x + rect.w * CHECK_TICK[at].0,
                    rect.y + rect.h * CHECK_TICK[at].1,
                )
            };
            let (a, b, d) = (point(0), point(1), point(2));
            painter.line(a.0, a.1, b.0, b.1, CHECK_BORDER.0, c(colour));
            painter.line(b.0, b.1, d.0, d.1, CHECK_BORDER.0, c(colour));
        }
        Part::Radio { on } => {
            let (x, y, radius) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, rect.w / 2.0);
            let frame = match (state.disabled, on) {
                (true, _) => Color::Line,
                (false, true) => Color::Accent,
                (false, false) => Color::Edge,
            };
            if !state.disabled {
                painter.fill_circle(x, y, radius, c(Color::Surface));
            }
            painter.stroke_circle(x, y, radius, CHECK_BORDER.0, c(frame));
            if *on {
                let dot = if state.disabled {
                    Color::Ink3
                } else {
                    Color::Accent
                };
                painter.fill_circle(x, y, radius * RADIO_DOT.0, c(dot));
            }
        }
        Part::Track => {
            painter.fill_rect(rect, RADIUS_SMALL.0, c(Color::Surface2));
            painter.stroke_rect(rect, RADIUS_SMALL.0, line, c(frame_of(state)));
        }
        Part::Segment {
            selected, label, ..
        } => {
            if *selected && !state.disabled {
                selected_frame(painter, look, rect, RADIUS_SMALL.0, state);
            } else if let Some(fill) =
                stepped(Some(Color::Surface2), state).filter(|_| state.steps() > 0)
            {
                // Inside the track, its frame whole (AUDIT-180b п. 3, решение 208.227).
                let inset = SEGMENT_HOVER_INSET.0;
                painter.fill_rect(rect.inset(inset), RADIUS_SMALL.0 - inset, c(fill));
            }
            let weight = selected.then_some(BUTTON_PRIMARY_WEIGHT);
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Ink
            };
            write(
                painter,
                look,
                Role::Body,
                weight,
                label,
                rect,
                Wrap::OneLine,
                Align::Centre,
                true,
                c(colour),
            );
        }
        Part::Field { value } => {
            field(painter, look, rect, state);
            let pad = FIELD_PAD.h.at(look.b);
            let mark = CHEVRON.at(look.b);
            // The value ends a padding short of the corner mark, cut with an ellipsis: a long
            // name ran into the mark (Б6 of AUDIT-180, «English (United States)» of «Источник»).
            let room = (rect.w - pad * 2.0 - pad - mark).max(0.0);
            let text = Rect::new(rect.x + pad, rect.y, room, rect.h);
            let text = if look.rtl {
                Rect::new(rect.x + pad * 2.0 + mark, rect.y, room, rect.h)
            } else {
                text
            };
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Ink
            };
            write(
                painter,
                look,
                Role::Body,
                None,
                value,
                text,
                Wrap::Ellipsis,
                Align::Start,
                true,
                c(colour),
            );
            chevron(painter, look, rect, state);
        }
        Part::Input {
            multiline,
            code,
            rtl,
            value,
            hint,
            ..
        } => {
            field(painter, look, rect, state);
            let pad = if *multiline { MULTILINE_PAD } else { FIELD_PAD };
            let (pad_h, pad_v) = (pad.h.at(look.b), pad.v.at(look.b));
            let (text, colour) = if value.is_empty() {
                (hint, Color::Ink3)
            } else if state.disabled {
                (value, Color::Ink3)
            } else {
                (value, Color::Ink)
            };
            let inner = Rect::new(
                rect.x + pad_h,
                rect.y + pad_v,
                rect.w - pad_h * 2.0,
                rect.h - pad_v * 2.0,
            );
            let wrap = if *multiline {
                Wrap::Words
            } else {
                Wrap::OneLine
            };
            // A field of code reads in the direction of its own text (`FieldKind::Code { rtl }`).
            let direction = match rtl {
                Some(true) => Direction::RightToLeft,
                Some(false) => Direction::LeftToRight,
                None => look.text.direction(),
            };
            painter.clip_rect(inner);
            write_in(
                painter,
                look,
                direction,
                if *code { Role::Code } else { Role::Body },
                None,
                text,
                inner,
                wrap,
                Align::Start,
                !*multiline,
                c(colour),
            );
            painter.unclip();
        }
        Part::Section { current, label, .. } => {
            if *current && !state.disabled {
                painter.fill_rect(rect, RADIUS_MEDIUM.0, c(Color::Surface));
                painter.stroke_rect(rect, RADIUS_MEDIUM.0, line, c(Color::Line));
                let stripe = CURRENT_STRIPE.0;
                let x = if look.rtl {
                    rect.right() - stripe
                } else {
                    rect.x
                };
                painter.clip_rect(rect);
                painter.fill_rect(Rect::new(x, rect.y, stripe, rect.h), 0.0, c(Color::Accent));
                painter.unclip();
            } else if state.steps() > 0 {
                // On the panel of `surface2` a step of the fill would not show (AUDIT-180b п. 4,
                // решение 208.227; the mock-up has the same fault): under the mouse the ground of the
                // current section, pressed `line`.
                let fill = if state.pressed {
                    Color::Line
                } else {
                    Color::Surface
                };
                painter.fill_rect(rect, RADIUS_MEDIUM.0, c(fill));
            }
            let pad = SECTION_PAD.h.at(look.b);
            let text = Rect::new(rect.x + pad, rect.y, rect.w - pad * 2.0, rect.h);
            let (role, colour) = match (*current, state.disabled) {
                (_, true) => (Role::Body, Color::Ink3),
                (true, false) => (Role::Strong, Color::Ink),
                (false, false) => (Role::Body, Color::Text),
            };
            write(
                painter,
                look,
                role,
                None,
                label,
                text,
                Wrap::OneLine,
                Align::Start,
                true,
                c(colour),
            );
        }
        Part::Link { text, role } => {
            if let Some(fill) = stepped(None, state) {
                painter.fill_rect(rect, RADIUS_SMALL.0, c(fill));
            }
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Accent
            };
            let laid = write(
                painter,
                look,
                *role,
                None,
                text,
                rect,
                Wrap::OneLine,
                Align::Start,
                false,
                c(colour),
            );
            if let (Some(laid), false) = (laid, state.disabled) {
                let width = laid.size().width;
                let x = if look.rtl {
                    rect.right() - width
                } else {
                    rect.x
                };
                let y = rect.y + laid.baseline() + LINK_UNDERLINE.0;
                painter.fill_rect(Rect::new(x, y, width, line), 0.0, c(colour));
            }
        }
        Part::Hit | Part::MenuRow => {
            let radius = if matches!(part, Part::MenuRow) {
                RADIUS_MEDIUM.0
            } else {
                RADIUS_SMALL.0
            };
            let focused_row = matches!(part, Part::MenuRow) && state.focus;
            let state = State {
                hover: state.hover || focused_row,
                ..state
            };
            if let Some(fill) = stepped(None, state) {
                painter.fill_rect(rect, radius, c(fill));
            }
        }
        Part::Card { selected } => {
            if *selected && !state.disabled {
                selected_frame(painter, look, rect, RADIUS_MEDIUM.0, state);
            } else {
                let fill = stepped(Some(Color::Surface), state);
                if let Some(fill) = fill {
                    painter.fill_rect(rect, RADIUS_MEDIUM.0, c(fill));
                }
                painter.stroke_rect(rect, RADIUS_MEDIUM.0, line, c(frame_of(state)));
            }
        }
        Part::List(_) => {
            painter.fill_rect(rect, RADIUS_MEDIUM.0, c(Color::Surface));
            painter.stroke_rect(rect, RADIUS_MEDIUM.0, line, c(frame_of(state)));
        }
        Part::ListRow {
            selected, check, ..
        } => {
            // In a list of check boxes the cursor chooses nothing — the boxes do —, so its frame
            // is shown only to the keyboard (AUDIT-180b п. 1, решение 208.227); in a list of a
            // choice it is the value.
            if *selected && !state.disabled && (check.is_none() || state.focus) {
                selected_frame(painter, look, rect, RADIUS_SMALL.0, state);
            } else if let Some(fill) = stepped(None, state) {
                painter.fill_rect(rect, RADIUS_SMALL.0, c(fill));
            }
        }
        Part::Scroll {
            first,
            visible,
            total,
        } => {
            painter.fill_rect(rect, rect.w / 2.0, c(Color::Surface2));
            if let Some(thumb) = scroll_thumb(rect, *first, *visible, *total) {
                painter.fill_rect(thumb, rect.w / 2.0, c(Color::Edge));
            }
        }
        Part::Progress { done } => {
            let colour = if *done { Color::Accent } else { Color::Line2 };
            painter.fill_rect(rect, rect.h / 2.0, c(colour));
        }
        Part::Key { label } => {
            painter.fill_rect(rect, RADIUS_CHECK.0, c(frame_of(state)));
            let inner = Rect::new(
                rect.x + line,
                rect.y + line,
                rect.w - line * 2.0,
                rect.h - line - KEY_BOTTOM.0,
            );
            painter.fill_rect(inner, RADIUS_CHECK.0 - line, c(Color::Surface));
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Ink
            };
            write(
                painter,
                look,
                Role::Code,
                None,
                label,
                inner,
                Wrap::OneLine,
                Align::Centre,
                true,
                c(colour),
            );
        }
        Part::PictureWord { word, size, fixed } => {
            // From the left of its place in any language (the picture is not mirrored): the start
            // of a run that reads right to left is its right edge.
            let colours = picture_colours();
            let style = Style {
                size: *size,
                ..Style::of(Role::Code, look.b, look.h).with_weight(ON_PICTURE_WEIGHT)
            };
            let align = if look.rtl { Align::End } else { Align::Start };
            if let Ok(laid) = look.text.lay(word, style, Wrap::OneLine, align, rect.w) {
                let colour = if *fixed { colours.fixed } else { colours.word };
                painter.text(&laid, rect.x, rect.y, colour);
            }
        }
        Part::PictureKey {
            label,
            size,
            pressed,
        } => {
            let colours = picture_colours();
            let (radius, frame) = (
                rect.h * ON_PICTURE_KEY_RADIUS.0,
                rect.h * ON_PICTURE_KEY_FRAME.0,
            );
            let fill = if *pressed { colours.lit } else { colours.key };
            painter.fill_rect(rect, radius, fill);
            painter.stroke_rect(rect, radius, frame, colours.ink);
            let style = Style {
                size: *size,
                ..Style::of(Role::Code, look.b, look.h).with_weight(ON_PICTURE_WEIGHT)
            };
            if let Ok(laid) = look
                .text
                .lay(label, style, Wrap::OneLine, Align::Centre, rect.w)
            {
                let y = rect.y + (rect.h - laid.size().height) / 2.0;
                painter.text(&laid, rect.x, y, colours.ink);
            }
        }
        Part::Dot(status) => {
            let colour = match status {
                Status::Working => Color::Accent,
                Status::Idle => Color::Ink3,
                Status::Restart => Color::Warn,
            };
            let radius = STATUS_DOT.at(look.b) / 2.0;
            let x = if look.rtl {
                rect.right() - radius
            } else {
                rect.x + radius
            };
            painter.fill_circle(x, rect.y + rect.h / 2.0, radius, c(colour));
        }
        Part::Number { text } => {
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Accent
            };
            write(
                painter,
                look,
                Role::Body,
                None,
                text,
                rect,
                Wrap::OneLine,
                Align::Start,
                false,
                c(colour),
            );
        }
        Part::BrandName { text } => {
            let style = Style {
                face: Face::Main,
                weight: BRAND_NAME_WEIGHT,
                ..Style::of(Role::Brand, look.b, look.h)
            };
            if let Ok(laid) = look
                .text
                .lay(text, style, Wrap::OneLine, Align::Start, rect.w)
            {
                painter.text(&laid, rect.x, rect.y, c(BRAND_NAME_COLOR));
            }
        }
        Part::Badge { text } => {
            let colour = if state.disabled {
                Color::Ink3
            } else {
                Color::Accent
            };
            write(
                painter,
                look,
                Role::Label,
                None,
                text,
                rect,
                Wrap::OneLine,
                Align::Start,
                true,
                c(colour),
            );
        }
    }

    if state.focus && !state.disabled && takes_ring(part) {
        ring(painter, look, part, rect);
    }
}

/// The frame of what is pressed or filled in, or `line` when unavailable.
fn frame_of(state: State) -> Color {
    if state.disabled {
        Color::Line
    } else {
        Color::Edge
    }
}

/// «Selected» of §5: `surface` and a frame of `ink`.
fn selected_frame(painter: &mut Painter, look: &Look, rect: Rect, radius: f32, state: State) {
    let fill = stepped(Some(Color::Surface), state).unwrap_or(Color::Surface);
    painter.fill_rect(rect, radius, look.colour(fill));
    painter.stroke_rect(rect, radius, SELECTED_BORDER.0, look.colour(Color::Ink));
}

/// The wrapper of a field: `surface` and a frame of `edge`, of `ink` with the keyboard.
/// The fill of a field's wrapper in `state`: the background the `EDIT` inside it takes too.
pub fn input_fill(state: State) -> Color {
    stepped(Some(Color::Surface), state).unwrap_or(Color::Bg)
}

fn field(painter: &mut Painter, look: &Look, rect: Rect, state: State) {
    if let Some(fill) = stepped(Some(Color::Surface), state) {
        painter.fill_rect(rect, RADIUS_SMALL.0, look.colour(fill));
    }
    let frame = if state.focus && !state.disabled {
        Color::Ink
    } else {
        frame_of(state)
    };
    painter.stroke_rect(rect, RADIUS_SMALL.0, LINE.0, look.colour(frame));
}

/// The corner mark of a choice field, at its end — at the left in a mirrored window (§7).
fn chevron(painter: &mut Painter, look: &Look, rect: Rect, state: State) {
    let (width, pad) = (CHEVRON.at(look.b), FIELD_PAD.h.at(look.b));
    let left = if look.rtl {
        rect.x + pad
    } else {
        rect.right() - pad - width
    };
    let (top, middle) = (rect.y + (rect.h - width / 2.0) / 2.0, left + width / 2.0);
    let colour = look.colour(if state.disabled {
        Color::Ink3
    } else {
        Color::Ink2
    });
    painter.line(left, top, middle, top + width / 2.0, LINE.0, colour);
    painter.line(middle, top + width / 2.0, left + width, top, LINE.0, colour);
}

/// A button of its kind (§6): the main one dark and mixed with `bg` by the mouse, the usual one on
/// `surface` with a frame of `edge` — in the dark theme a step further under the mouse, where
/// `surface2` is hardly told from `surface` (AUDIT-180b п. 3). There is no third kind: the quiet one
/// without a frame did not read as a button (AUDIT-180b пп. 8–9, решение 208.227).
fn button(
    painter: &mut Painter,
    look: &Look,
    kind: ButtonKind,
    label: &str,
    rect: Rect,
    state: State,
) {
    let radius = RADIUS_MEDIUM.0;
    let c = |colour: Color| look.colour(colour);

    let (fill, frame, text, weight) = match (kind, state.disabled) {
        (_, true) => (None, Some(Color::Line), c(Color::Ink3), None),
        (ButtonKind::Primary, false) => {
            let share = if state.pressed {
                PRESSED_MIX.0
            } else if state.hover {
                HOVER_MIX.0
            } else {
                0.0
            };
            let fill = c(Color::Button).mix(c(Color::Bg), share);
            painter.fill_rect(rect, radius, fill);
            (None, None, c(Color::ButtonFg), Some(BUTTON_PRIMARY_WEIGHT))
        }
        (ButtonKind::Normal, false) => {
            let further = usize::from(look.theme == Theme::Dark && state.steps() > 0);
            (
                stepped_by(Some(Color::Surface), state, state.steps() + further),
                Some(Color::Edge),
                c(Color::Ink),
                None,
            )
        }
    };

    if let Some(fill) = fill {
        painter.fill_rect(rect, radius, c(fill));
    }
    if let Some(frame) = frame {
        painter.stroke_rect(rect, radius, LINE.0, c(frame));
    }
    // A label wider than its button goes on to a second line, inside the paddings — only where the
    // layout made the button taller for it (§3а, step 5).
    let (pad, pad_v) = (BUTTON_PAD.h.at(look.b), BUTTON_PAD.v.at(look.b));
    let mut style = Style::of(Role::Button, look.b, look.h);
    if let Some(weight) = weight {
        style = style.with_weight(weight);
    }
    let wraps = button_label_wraps(
        look.text
            .measure(label, style, Wrap::OneLine, f32::MAX)
            .width,
        rect.w - pad * 2.0,
        rect.h - pad_v * 2.0,
        style.line_height(),
    );
    let (place, wrap) = if wraps {
        (
            Rect::new(rect.x + pad, rect.y, (rect.w - pad * 2.0).max(0.0), rect.h),
            Wrap::Words,
        )
    } else {
        (rect, Wrap::OneLine)
    };
    write(
        painter,
        look,
        Role::Button,
        weight,
        label,
        place,
        wrap,
        Align::Centre,
        true,
        text,
    );
}

/// Whether the label of a button goes on to a second line (§3а, step 5): when the layout made the
/// inside of the button taller than one line for it **and** the label is wider than the inside by
/// more than the rounding of a measure. A label that fits its button to the last hundredth stays on
/// its line whole — the stand of 2026-11-20, Windows 11: the main button «Close» of «О программе»,
/// exactly as wide as its label, came out «Clos / e». Pure: the width of the label on one line, the
/// width and height of the inside of the button, the height of one line.
pub fn button_label_wraps(label: f32, inner_w: f32, inner_h: f32, line: f32) -> bool {
    label > inner_w + 0.5 && inner_h > line + line * 0.5
}

/// The keyboard ring of §4 around `rect`, `FOCUS_GAP` outside it, its corners following the part's.
fn ring(painter: &mut Painter, look: &Look, part: &Part, rect: Rect) {
    let radius = match part {
        Part::Card { .. } | Part::Section { .. } | Part::Button { .. } => RADIUS_MEDIUM.0,
        _ => RADIUS_SMALL.0,
    };
    let out = FOCUS_GAP.0 + FOCUS_RING.0;
    painter.stroke_rect(
        rect.inset(-out),
        radius + out,
        FOCUS_RING.0,
        look.colour(Color::Accent),
    );
}

/// Lays `text` out in `role` (and `weight`) within `rect` and draws it in `colour`; `middle` —
/// centred in the height of `rect`, else from its top. Answers the layout, or `None` when
/// DirectWrite refused it (NFR-13: the element is drawn without its text).
#[allow(clippy::too_many_arguments)]
fn write(
    painter: &mut Painter,
    look: &Look,
    role: Role,
    weight: Option<u16>,
    text: &str,
    rect: Rect,
    wrap: Wrap,
    align: Align,
    middle: bool,
    colour: Rgb,
) -> Option<crate::ui::text::Laid> {
    let direction = look.text.direction();
    write_in(
        painter, look, direction, role, weight, text, rect, wrap, align, middle, colour,
    )
}

/// [`write`] in a reading `direction` of its own, not the window's (a field of code).
#[allow(clippy::too_many_arguments)]
fn write_in(
    painter: &mut Painter,
    look: &Look,
    direction: Direction,
    role: Role,
    weight: Option<u16>,
    text: &str,
    rect: Rect,
    wrap: Wrap,
    align: Align,
    middle: bool,
    colour: Rgb,
) -> Option<crate::ui::text::Laid> {
    let mut style = Style::of(role, look.b, look.h);
    if let Some(weight) = weight {
        style = style.with_weight(weight);
    }
    let laid = look
        .text
        .lay_in(direction, text, style, wrap, align, rect.w)
        .ok()?;
    let y = if middle {
        rect.y + (rect.h - laid.size().height) / 2.0
    } else {
        rect.y
    };
    painter.text(&laid, rect.x, y, colour);
    Some(laid)
}
