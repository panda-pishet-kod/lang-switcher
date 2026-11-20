//! The picture of the taskbar in «Обновлено» (T-120-14; the mock-up's animation of T-119-6),
//! drawn by code on the canvas with the person's own layouts: a strip of the taskbar with the flag
//! of one layout and its code; the plate of the keys «Alt + Shift»; the cursor on its way to the
//! flag; the list of the two layouts over the strip, its first row hovered. The geometry is the
//! picture of E119, in points. It follows Windows, not the language of the program (passport §7):
//! where Windows reads right to left, the icons — and the whole picture — stand mirrored.

use crate::ui::canvas::{Painter, Rect};
use crate::ui::elements::Look;
use crate::ui::scene::Taskbar;
use crate::ui::text::{Align, Style, Wrap};
use crate::ui::tokens::{Color, LINE, RADIUS_MEDIUM, Role, Theme};

/// The strip of the taskbar: its height and its inset from the frame of the picture.
const BAR: f32 = 36.0;
const INSET: f32 = 1.0;
/// The flag: its side, its place from the end of the strip, the gap to its code.
const FLAG: f32 = 20.0;
const FLAG_FROM_END: f32 = 64.0;
const CODE_GAP: f32 = 8.0;
const CODE_END: f32 = 4.0;
/// The plate of the keys.
const PLATE_X: f32 = 16.0;
const PLATE_Y: f32 = 16.0;
const PLATE_W: f32 = 120.0;
const PLATE_H: f32 = 30.0;
const PLATE_RADIUS: f32 = 6.0;
const PLATE_KEYS: &str = "Alt + Shift";
/// The list of the two layouts over the strip.
const ROW: f32 = 30.0;
const LIST_GAP: f32 = 8.0;
const LIST_BEFORE: f32 = 120.0;
const LIST_AFTER: f32 = 40.0;
const LIST_INSET: f32 = 4.0;
const ROW_PAD: f32 = 6.0;
/// The cursor: where it starts and its shape.
const CURSOR_FROM_Y: f32 = 40.0;
const CURSOR_HEIGHT: f32 = 14.0;
const CURSOR_NOTCH_X: f32 = 4.0;
const CURSOR_NOTCH_Y: f32 = 10.0;
const CURSOR_WIDTH: f32 = 10.0;

/// Draws `picture` into `rect`.
pub fn draw(painter: &mut Painter, look: &Look, picture: &Taskbar, rect: Rect) {
    let c = |colour: Color| colour.get(look.theme);
    // Laid out as on a taskbar read left to right, then turned where Windows reads right to left.
    let side = |r: Rect| {
        if picture.icons_left {
            Rect::new(rect.x + rect.right() - r.right(), r.y, r.w, r.h)
        } else {
            r
        }
    };
    let side_x = |x: f32| {
        if picture.icons_left {
            rect.x + rect.right() - x
        } else {
            x
        }
    };

    painter.fill_rect(rect, RADIUS_MEDIUM.0, c(Color::Surface2));
    painter.stroke_rect(rect, RADIUS_MEDIUM.0, LINE.0, c(Color::Line));

    let bar = Rect::new(
        rect.x + INSET,
        rect.bottom() - BAR,
        rect.w - INSET * 2.0,
        BAR - INSET,
    );
    painter.fill_rect(side(bar), 0.0, c(Color::Surface));
    painter.fill_rect(
        side(Rect::new(bar.x, bar.y, bar.w, LINE.0)),
        0.0,
        c(Color::Line),
    );

    let flag = Rect::new(
        bar.right() - FLAG_FROM_END,
        bar.y + (bar.h - FLAG) / 2.0,
        FLAG,
        FLAG,
    );
    let dark = look.theme == Theme::Dark;
    let shown = picture.shown.min(1);
    painter.flag(picture.layouts[shown], side(flag), dark);
    let code = Rect::new(
        flag.right() + CODE_GAP,
        bar.y,
        bar.right() - CODE_END - flag.right() - CODE_GAP,
        bar.h,
    );
    write(
        painter,
        look,
        picture,
        &picture.codes[shown],
        side(code),
        false,
    );

    if picture.plate {
        let plate = Rect::new(rect.x + PLATE_X, rect.y + PLATE_Y, PLATE_W, PLATE_H);
        painter.fill_rect(side(plate), PLATE_RADIUS, c(Color::Surface));
        painter.stroke_rect(side(plate), PLATE_RADIUS, LINE.0, c(Color::Edge));
        write(painter, look, picture, PLATE_KEYS, side(plate), true);
    }

    if picture.list {
        let list = Rect::new(
            flag.x - LIST_BEFORE,
            bar.y - LIST_GAP - ROW * 2.0 - LIST_GAP,
            LIST_BEFORE + FLAG + LIST_AFTER,
            ROW * 2.0 + LIST_GAP,
        );
        painter.fill_rect(side(list), RADIUS_MEDIUM.0, c(Color::Surface));
        painter.stroke_rect(side(list), RADIUS_MEDIUM.0, LINE.0, c(Color::Line));
        for (index, (layout, code)) in picture.layouts.iter().zip(&picture.codes).enumerate() {
            let place = Rect::new(
                list.x + LIST_INSET,
                list.y + LIST_INSET + ROW * index as f32,
                list.w - LIST_INSET * 2.0,
                ROW,
            );
            if picture.hover && index == 0 {
                painter.fill_rect(side(place), 0.0, c(Color::Surface2));
            }
            let icon = Rect::new(place.x + ROW_PAD, place.y + (ROW - FLAG) / 2.0, FLAG, FLAG);
            painter.flag(*layout, side(icon), dark);
            let words = Rect::new(
                icon.right() + CODE_GAP,
                place.y,
                place.right() - icon.right() - CODE_GAP,
                ROW,
            );
            write(painter, look, picture, code, side(words), false);
        }
    }

    if let Some(share) = picture.cursor {
        let from = (rect.x + rect.w / 3.0, rect.y + CURSOR_FROM_Y);
        let to = (flag.x + flag.w / 2.0, flag.y + flag.h / 2.0);
        let (x, y) = (
            from.0 + (to.0 - from.0) * share,
            from.1 + (to.1 - from.1) * share,
        );
        let points = [
            (side_x(x), y),
            (side_x(x), y + CURSOR_HEIGHT),
            (side_x(x + CURSOR_NOTCH_X), y + CURSOR_NOTCH_Y),
            (side_x(x + CURSOR_WIDTH), y + CURSOR_NOTCH_Y),
        ];
        painter.fill_polygon(&points, c(Color::Ink));
    }
}

/// One line of the picture in the type of the check box of the window, `ink`, in the middle of
/// `place`: centred, or from the edge the picture reads from.
fn write(
    painter: &mut Painter,
    look: &Look,
    picture: &Taskbar,
    words: &str,
    place: Rect,
    centred: bool,
) {
    let style = Style::of(Role::CornerCheck, look.b, look.h);
    // The edge the picture reads from is Windows'; `Align` counts from the language's own edge.
    let align = if centred {
        Align::Centre
    } else if picture.icons_left == look.rtl {
        Align::Start
    } else {
        Align::End
    };
    if let Ok(laid) = look.text.lay(words, style, Wrap::OneLine, align, place.w) {
        let y = place.y + (place.h - laid.size().height) / 2.0;
        painter.text(&laid, place.x, y, Color::Ink.get(look.theme));
    }
}
