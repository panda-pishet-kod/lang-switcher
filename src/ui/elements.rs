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
use crate::ui::scene::{ButtonKind, Surface};
use crate::ui::text::{Align, Style, Text, Wrap};
use crate::ui::tokens::shape::{CHECK_TICK, CHEVRON, FILL_STEPS, RADIO_DOT};
use crate::ui::tokens::{
    ART_RING, BUTTON_PRIMARY_WEIGHT, CHECK_BORDER, CURRENT_STRIPE, Color, FIELD_PAD, FOCUS_GAP,
    FOCUS_RING, HOVER_MIX, KEY_BOTTOM, LINE, LINK_UNDERLINE, PANEL_DOT, PANEL_DOT_STEP,
    PRESSED_MIX, RADIUS_CHECK, RADIUS_MEDIUM, RADIUS_SMALL, Rgb, Role, SECTION_PAD,
    SELECTED_BORDER, STATUS_DOT, TOGGLE_GAP, TOGGLE_KNOB, Theme,
};
use crate::ui::tokens::{BRAND_NAME_COLOR, BRAND_NAME_WEIGHT, Face};

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
    if state.disabled {
        return None;
    }
    let start = match base {
        Some(colour) => match FILL_STEPS.iter().position(|step| *step == colour) {
            Some(at) => at,
            None => return Some(colour),
        },
        None if state.steps() == 0 => return None,
        None => 0,
    };
    let at = (start + state.steps()).min(FILL_STEPS.len() - 1);
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
        | Part::Segment { .. }
        | Part::Field { .. }
        | Part::Input
        | Part::Section { .. }
        | Part::ListRow { .. }
        | Part::MenuRow => Some(rect),
        _ => None,
    }
}

/// Whether the keyboard ring is drawn around this part when it has the focus.
fn takes_ring(part: &Part) -> bool {
    matches!(
        part,
        Part::Hit
            | Part::Card { .. }
            | Part::Button { .. }
            | Part::Link { .. }
            | Part::Track
            | Part::Field { .. }
            | Part::Section { .. }
            | Part::ListRow { .. }
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
                painter.fill_rect(rect, RADIUS_SMALL.0, c(fill));
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
            let text = Rect::new(rect.x + pad, rect.y, rect.w - pad * 2.0 - mark, rect.h);
            let text = if look.rtl {
                Rect::new(rect.x + pad + mark, rect.y, text.w, rect.h)
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
                Wrap::OneLine,
                Align::Start,
                true,
                c(colour),
            );
            chevron(painter, look, rect, state);
        }
        Part::Input => field(painter, look, rect, state),
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
            } else if let Some(fill) = stepped(None, state) {
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
        Part::Link { text } => {
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
                Role::Link,
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
        Part::List => {
            painter.fill_rect(rect, RADIUS_MEDIUM.0, c(Color::Surface));
            painter.stroke_rect(rect, RADIUS_MEDIUM.0, line, c(frame_of(state)));
        }
        Part::ListRow { selected } => {
            if *selected && !state.disabled {
                selected_frame(painter, look, rect, RADIUS_SMALL.0, state);
            } else if let Some(fill) = stepped(None, state) {
                painter.fill_rect(rect, RADIUS_SMALL.0, c(fill));
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
/// `surface` with a frame of `edge`, the quiet one with neither.
fn button(
    painter: &mut Painter,
    look: &Look,
    kind: ButtonKind,
    label: &str,
    rect: Rect,
    state: State,
) {
    let radius = match kind {
        ButtonKind::Quiet => RADIUS_SMALL.0,
        _ => RADIUS_MEDIUM.0,
    };
    let c = |colour: Color| look.colour(colour);

    let (fill, frame, text, weight) = match (kind, state.disabled) {
        (_, true) => (
            None,
            (kind != ButtonKind::Quiet).then_some(Color::Line),
            c(Color::Ink3),
            None,
        ),
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
        (ButtonKind::Normal, false) => (
            stepped(Some(Color::Surface), state),
            Some(Color::Edge),
            c(Color::Ink),
            None,
        ),
        (ButtonKind::Quiet, false) => (stepped(None, state), None, c(Color::Ink), None),
    };

    if let Some(fill) = fill {
        painter.fill_rect(rect, radius, c(fill));
    }
    if let Some(frame) = frame {
        painter.stroke_rect(rect, radius, LINE.0, c(frame));
    }
    write(
        painter,
        look,
        Role::Button,
        weight,
        label,
        rect,
        Wrap::OneLine,
        Align::Centre,
        true,
        text,
    );
}

/// The keyboard ring of §4 around `rect`, `FOCUS_GAP` outside it, its corners following the part's.
fn ring(painter: &mut Painter, look: &Look, part: &Part, rect: Rect) {
    let radius = match part {
        Part::Card { .. }
        | Part::Section { .. }
        | Part::Button {
            kind: ButtonKind::Normal | ButtonKind::Primary,
            ..
        } => RADIUS_MEDIUM.0,
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
    let mut style = Style::of(role, look.b, look.h);
    if let Some(weight) = weight {
        style = style.with_weight(weight);
    }
    let laid = look.text.lay(text, style, wrap, align, rect.w).ok()?;
    let y = if middle {
        rect.y + (rect.h - laid.size().height) / 2.0
    } else {
        rect.y
    };
    painter.text(&laid, rect.x, y, colour);
    Some(laid)
}
