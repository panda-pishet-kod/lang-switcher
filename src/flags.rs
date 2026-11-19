//! The flag of the layout on the tray icon — task T-119-2, решения 208.88, 208.89 (рекомендации
//! 1–5, 7).
//!
//! With the setting `[general] tray_flag` on, the icon of the program becomes the flag of the
//! country of the layout in force:
//!
//! * the country is the one Windows names for the layout's language (`en-US` → США, `en-GB` →
//!   Великобритания) — [`face_of`];
//! * two layouts of one country show one flag, and the tooltip tells them apart;
//! * a country this table has no flag for gets a **plate** with the letters of the language;
//! * every flag is drawn here, by code, at the size the shell asks for (100–200 %), with a thin
//!   frame for a light and for a dark taskbar — [`render`];
//! * the pause greys the flag, an unread letter puts the dot of FR-90 on it.
//!
//! # Why the flags are figures and not pictures
//!
//! A picture per country and per scale would be a hundred resources times four sizes, and a scale
//! between the frames would be stretched — the blur of решение 124б.2. A flag here is a handful of
//! figures in a field of 16 × 12 units (the 16 × 12 of the accepted mock-up), and [`render`] lays
//! them on the pixels of any size with a 4 × 4 sub-sample per pixel: at 100 % and 200 % the edges
//! fall on whole pixels and are sharp, between them they are smoothed instead of stretched.
//!
//! Emblems are simplified to what 16 pixels can carry. A flag that is only an emblem away from
//! another (Slovakia, Slovenia and Russia share the three colours) keeps a sign of its emblem.
//!
//! # Pure, and why
//!
//! Everything but [`face_of`] (which asks the locale service) and [`icon`] (which hands the pixels
//! to the system) is a function of its arguments: `tests\flags.rs` reads the pixels back.

use crate::layouts::LayoutId;

/// A colour, straight (not premultiplied) RGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// One figure of a flag, in units of the field of 16 × 12. A later figure lies over an earlier one.
#[derive(Clone, Copy, Debug)]
pub enum Figure {
    /// A rectangle `[x0, x1) × [y0, y1)`.
    Rect(f32, f32, f32, f32, Rgb),
    /// A disc: centre and radius.
    Disc(f32, f32, f32, Rgb),
    /// The part of a disc below (`true`) or above (`false`) the line `y = cut`.
    Half(f32, f32, f32, f32, bool, Rgb),
    /// A five-pointed star, one point up: centre and outer radius.
    Star(f32, f32, f32, Rgb),
    /// A polygon, even-odd rule, points in order.
    Poly(&'static [(f32, f32)], Rgb),
    /// Another flag's figures, shrunk by `scale` and moved to `(x, y)` — the Union Jack in the
    /// canton of Australia and New Zealand.
    Inset(&'static [Figure], f32, f32, f32),
}

/// A flag: the country it belongs to, its outline and its figures.
#[derive(Debug)]
pub struct Flag {
    /// ISO 3166 code, upper case — what `LOCALE_SISO3166CTRYNAME` answers.
    pub country: &'static str,
    /// The outline: `None` is the whole field of 16 × 12; a polygon is a flag of another shape
    /// (Nepal).
    pub outline: Option<&'static [(f32, f32)]>,
    /// The figures, bottom first.
    pub figures: &'static [Figure],
}

/// What the icon shows for a layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Face {
    /// The flag of the country, by its index in [`FLAGS`].
    Flag(usize),
    /// No flag: a plate with up to two letters of the language, upper case, A–Z.
    Plate(String),
}

/// How the icon is to look besides the flag itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    /// The side of the icon in pixels — `tray::small_icon_size`: 16 at 100 %, 20, 24, 32 at 200 %.
    pub size: u32,
    /// The program is paused: the flag is grey and paler (решение 208.89 п. 5).
    pub paused: bool,
    /// A letter is unread: the dot of FR-90.
    pub unread: bool,
    /// The taskbar is dark: the frame is light rather than dark.
    pub dark_bar: bool,
}

// -----------------------------------------------------------------------------------------
// The colours and the table
// -----------------------------------------------------------------------------------------

const W: Rgb = Rgb(255, 255, 255);
const K: Rgb = Rgb(0, 0, 0);

use Figure::{Disc, Half, Inset, Poly, Rect, Star};

/// Three horizontal stripes of one height.
const fn h3(a: Rgb, b: Rgb, c: Rgb) -> [Figure; 3] {
    [
        Rect(0.0, 0.0, 16.0, 4.0, a),
        Rect(0.0, 4.0, 16.0, 8.0, b),
        Rect(0.0, 8.0, 16.0, 12.0, c),
    ]
}

/// Three vertical stripes of one width.
const fn v3(a: Rgb, b: Rgb, c: Rgb) -> [Figure; 3] {
    [
        Rect(0.0, 0.0, 5.333, 12.0, a),
        Rect(5.333, 0.0, 10.667, 12.0, b),
        Rect(10.667, 0.0, 16.0, 12.0, c),
    ]
}

/// A Nordic cross: the field, the cross (`x0..x1` × `y0..y1`), and an inner cross if any.
const fn nordic(field: Rgb, cross: Rgb, x: (f32, f32), y: (f32, f32)) -> [Figure; 3] {
    [
        Rect(0.0, 0.0, 16.0, 12.0, field),
        Rect(x.0, 0.0, x.1, 12.0, cross),
        Rect(0.0, y.0, 16.0, y.1, cross),
    ]
}

const GB_BLUE: Rgb = Rgb(1, 33, 105);
const GB_RED: Rgb = Rgb(200, 16, 46);

/// The Union Jack, simplified: the white saltire, the thin red one, the white and the red cross.
const UNION_JACK: [Figure; 9] = [
    Rect(0.0, 0.0, 16.0, 12.0, GB_BLUE),
    Poly(
        &[(-0.72, 0.96), (15.28, 12.96), (16.72, 11.04), (0.72, -0.96)],
        W,
    ),
    Poly(
        &[(0.72, 12.96), (16.72, 0.96), (15.28, -0.96), (-0.72, 11.04)],
        W,
    ),
    Poly(
        &[(-0.24, 0.32), (15.76, 12.32), (16.24, 11.68), (0.24, -0.32)],
        GB_RED,
    ),
    Poly(
        &[(0.24, 12.32), (16.24, 0.32), (15.76, -0.32), (-0.24, 11.68)],
        GB_RED,
    ),
    Rect(6.0, 0.0, 10.0, 12.0, W),
    Rect(0.0, 4.0, 16.0, 8.0, W),
    Rect(6.8, 0.0, 9.2, 12.0, GB_RED),
    Rect(0.0, 4.8, 16.0, 7.2, GB_RED),
];

/// The United States as the accepted mock-up draws it: six white stripes on red, the canton,
/// eight stars as points.
const US: [Figure; 16] = [
    Rect(0.0, 0.0, 16.0, 12.0, Rgb(178, 34, 52)),
    Rect(0.0, 1.0, 16.0, 2.0, W),
    Rect(0.0, 3.0, 16.0, 4.0, W),
    Rect(0.0, 5.0, 16.0, 6.0, W),
    Rect(0.0, 7.0, 16.0, 8.0, W),
    Rect(0.0, 9.0, 16.0, 10.0, W),
    Rect(0.0, 11.0, 16.0, 12.0, W),
    Rect(0.0, 0.0, 7.0, 7.0, Rgb(60, 59, 110)),
    Rect(1.0, 1.0, 2.0, 2.0, W),
    Rect(3.0, 1.0, 4.0, 2.0, W),
    Rect(5.0, 1.0, 6.0, 2.0, W),
    Rect(2.0, 3.0, 3.0, 4.0, W),
    Rect(4.0, 3.0, 5.0, 4.0, W),
    Rect(1.0, 5.0, 2.0, 6.0, W),
    Rect(3.0, 5.0, 4.0, 6.0, W),
    Rect(5.0, 5.0, 6.0, 6.0, W),
];

/// Red, white and blue in three — Russia; Slovakia and Slovenia carry a sign of their emblems.
const RU3: [Figure; 3] = h3(W, Rgb(0, 57, 166), Rgb(213, 43, 30));

/// The flags, in the order of their countries' codes. **Adding a country is a row here** and
/// nothing else; `tests\flags.rs` checks that every code is two capital letters and appears once.
pub static FLAGS: &[Flag] = &[
    Flag {
        country: "AE",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(0, 115, 47)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, K),
            Rect(0.0, 0.0, 4.0, 12.0, Rgb(255, 0, 0)),
        ],
    },
    Flag {
        country: "AF",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 5.333, 12.0, K),
            Rect(5.333, 0.0, 10.667, 12.0, Rgb(190, 0, 0)),
            Rect(10.667, 0.0, 16.0, 12.0, Rgb(0, 122, 54)),
            Disc(8.0, 6.0, 1.8, W),
            Disc(8.0, 6.0, 1.1, Rgb(190, 0, 0)),
        ],
    },
    Flag {
        country: "AL",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(228, 30, 32)),
            Poly(
                &[
                    (8.0, 2.4),
                    (10.8, 3.4),
                    (12.0, 5.8),
                    (9.6, 6.4),
                    (9.0, 9.6),
                    (8.0, 10.2),
                    (7.0, 9.6),
                    (6.4, 6.4),
                    (4.0, 5.8),
                    (5.2, 3.4),
                ],
                K,
            ),
        ],
    },
    Flag {
        country: "AM",
        outline: None,
        figures: &h3(Rgb(217, 0, 18), Rgb(0, 51, 160), Rgb(242, 168, 0)),
    },
    Flag {
        country: "AR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(116, 172, 223)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(116, 172, 223)),
            Disc(8.0, 6.0, 1.2, Rgb(246, 180, 14)),
        ],
    },
    Flag {
        country: "AT",
        outline: None,
        figures: &h3(Rgb(237, 41, 57), W, Rgb(237, 41, 57)),
    },
    Flag {
        country: "AU",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 0, 139)),
            Inset(&UNION_JACK, 0.0, 0.0, 0.5),
            Star(4.0, 9.0, 1.5, W),
            Star(12.0, 2.4, 0.8, W),
            Star(14.2, 5.2, 0.8, W),
            Star(10.6, 5.8, 0.8, W),
            Star(12.0, 9.8, 0.8, W),
        ],
    },
    Flag {
        country: "AZ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(0, 181, 226)),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(239, 51, 64)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(80, 158, 47)),
            Disc(7.4, 6.0, 1.7, W),
            Disc(7.9, 6.0, 1.4, Rgb(239, 51, 64)),
            Star(9.6, 6.0, 0.8, W),
        ],
    },
    Flag {
        country: "BA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 35, 149)),
            Poly(&[(4.4, 0.0), (11.6, 0.0), (11.6, 12.0)], Rgb(254, 203, 0)),
            Disc(3.8, 0.6, 0.45, W),
            Disc(5.2, 2.0, 0.45, W),
            Disc(6.6, 4.0, 0.45, W),
            Disc(8.0, 6.0, 0.45, W),
            Disc(9.4, 8.0, 0.45, W),
            Disc(10.8, 10.0, 0.45, W),
        ],
    },
    Flag {
        country: "BD",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 106, 78)),
            Disc(7.1, 6.0, 3.3, Rgb(244, 42, 65)),
        ],
    },
    Flag {
        country: "BE",
        outline: None,
        figures: &v3(K, Rgb(250, 224, 66), Rgb(237, 41, 57)),
    },
    Flag {
        country: "BG",
        outline: None,
        figures: &h3(W, Rgb(0, 150, 110), Rgb(214, 38, 18)),
    },
    Flag {
        country: "BR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 156, 59)),
            Poly(
                &[(8.0, 1.3), (14.6, 6.0), (8.0, 10.7), (1.4, 6.0)],
                Rgb(255, 223, 0),
            ),
            Disc(8.0, 6.0, 2.8, Rgb(0, 39, 118)),
        ],
    },
    Flag {
        country: "BY",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 8.0, Rgb(200, 49, 62)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(74, 166, 87)),
            Rect(0.0, 0.0, 2.4, 12.0, W),
            Poly(
                &[(1.2, 0.6), (2.0, 2.0), (1.2, 3.4), (0.4, 2.0)],
                Rgb(200, 49, 62),
            ),
            Poly(
                &[(1.2, 4.6), (2.0, 6.0), (1.2, 7.4), (0.4, 6.0)],
                Rgb(200, 49, 62),
            ),
            Poly(
                &[(1.2, 8.6), (2.0, 10.0), (1.2, 11.4), (0.4, 10.0)],
                Rgb(200, 49, 62),
            ),
        ],
    },
    Flag {
        country: "CA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(0.0, 0.0, 4.0, 12.0, Rgb(255, 0, 0)),
            Rect(12.0, 0.0, 16.0, 12.0, Rgb(255, 0, 0)),
            Poly(
                &[
                    (8.0, 2.0),
                    (8.8, 3.6),
                    (10.2, 3.2),
                    (9.6, 5.6),
                    (10.8, 5.2),
                    (10.4, 6.6),
                    (8.4, 8.0),
                    (8.4, 9.8),
                    (7.6, 9.8),
                    (7.6, 8.0),
                    (5.6, 6.6),
                    (5.2, 5.2),
                    (6.4, 5.6),
                    (5.8, 3.2),
                    (7.2, 3.6),
                ],
                Rgb(255, 0, 0),
            ),
        ],
    },
    Flag {
        country: "CH",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(218, 41, 28)),
            Rect(6.9, 2.4, 9.1, 9.6, W),
            Rect(4.4, 4.9, 11.6, 7.1, W),
        ],
    },
    Flag {
        country: "CL",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, W),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(213, 43, 30)),
            Rect(0.0, 0.0, 5.333, 6.0, Rgb(0, 57, 166)),
            Star(2.667, 3.0, 1.4, W),
        ],
    },
    Flag {
        country: "CN",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(238, 28, 37)),
            Star(2.7, 3.0, 1.8, Rgb(255, 255, 0)),
            Star(5.3, 1.2, 0.6, Rgb(255, 255, 0)),
            Star(6.4, 2.4, 0.6, Rgb(255, 255, 0)),
            Star(6.4, 4.0, 0.6, Rgb(255, 255, 0)),
            Star(5.3, 5.2, 0.6, Rgb(255, 255, 0)),
        ],
    },
    Flag {
        country: "CO",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(252, 209, 22)),
            Rect(0.0, 6.0, 16.0, 9.0, Rgb(0, 56, 147)),
            Rect(0.0, 9.0, 16.0, 12.0, Rgb(206, 17, 38)),
        ],
    },
    Flag {
        country: "CY",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Poly(
                &[
                    (3.6, 5.0),
                    (6.0, 3.8),
                    (9.6, 3.4),
                    (12.4, 2.6),
                    (11.0, 4.2),
                    (8.6, 5.6),
                    (5.6, 6.0),
                ],
                Rgb(213, 120, 0),
            ),
            Rect(6.0, 7.4, 10.0, 8.2, Rgb(68, 97, 33)),
        ],
    },
    Flag {
        country: "CZ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, W),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(215, 20, 26)),
            Poly(&[(0.0, 0.0), (8.0, 6.0), (0.0, 12.0)], Rgb(17, 69, 126)),
        ],
    },
    Flag {
        country: "DE",
        outline: None,
        figures: &h3(K, Rgb(221, 0, 0), Rgb(255, 206, 0)),
    },
    Flag {
        country: "DK",
        outline: None,
        figures: &nordic(Rgb(200, 16, 46), W, (5.2, 6.9), (5.1, 6.9)),
    },
    Flag {
        country: "EE",
        outline: None,
        figures: &h3(Rgb(0, 114, 206), K, W),
    },
    Flag {
        country: "EG",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(206, 17, 38)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, K),
            Rect(7.2, 4.8, 8.8, 7.2, Rgb(192, 147, 0)),
        ],
    },
    Flag {
        country: "ES",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(170, 21, 27)),
            Rect(0.0, 3.0, 16.0, 9.0, Rgb(241, 191, 0)),
            Rect(3.6, 4.6, 5.6, 7.4, Rgb(170, 21, 27)),
        ],
    },
    Flag {
        country: "ET",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(7, 137, 48)),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(252, 221, 9)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(218, 18, 26)),
            Disc(8.0, 6.0, 2.6, Rgb(15, 71, 175)),
            Star(8.0, 6.1, 1.8, Rgb(252, 221, 9)),
        ],
    },
    Flag {
        country: "FI",
        outline: None,
        figures: &nordic(W, Rgb(0, 47, 108), (4.4, 7.1), (4.4, 7.6)),
    },
    Flag {
        country: "FO",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(4.4, 0.0, 7.3, 12.0, Rgb(0, 101, 189)),
            Rect(0.0, 4.5, 16.0, 7.5, Rgb(0, 101, 189)),
            Rect(5.1, 0.0, 6.6, 12.0, Rgb(237, 41, 57)),
            Rect(0.0, 5.25, 16.0, 6.75, Rgb(237, 41, 57)),
        ],
    },
    Flag {
        country: "FR",
        outline: None,
        figures: &v3(Rgb(0, 85, 164), W, Rgb(239, 65, 53)),
    },
    Flag {
        country: "GB",
        outline: None,
        figures: &UNION_JACK,
    },
    Flag {
        country: "GE",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(6.9, 0.0, 9.1, 12.0, Rgb(255, 0, 0)),
            Rect(0.0, 4.9, 16.0, 7.1, Rgb(255, 0, 0)),
            Rect(2.8, 1.8, 4.2, 3.2, Rgb(255, 0, 0)),
            Rect(11.8, 1.8, 13.2, 3.2, Rgb(255, 0, 0)),
            Rect(2.8, 8.8, 4.2, 10.2, Rgb(255, 0, 0)),
            Rect(11.8, 8.8, 13.2, 10.2, Rgb(255, 0, 0)),
        ],
    },
    Flag {
        country: "GR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(13, 94, 175)),
            Rect(0.0, 1.333, 16.0, 2.667, W),
            Rect(0.0, 4.0, 16.0, 5.333, W),
            Rect(0.0, 6.667, 16.0, 8.0, W),
            Rect(0.0, 9.333, 16.0, 10.667, W),
            Rect(0.0, 0.0, 6.667, 6.667, Rgb(13, 94, 175)),
            Rect(2.667, 0.0, 4.0, 6.667, W),
            Rect(0.0, 2.667, 6.667, 4.0, W),
        ],
    },
    Flag {
        country: "HK",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(222, 41, 16)),
            Star(8.0, 6.2, 3.4, W),
        ],
    },
    Flag {
        country: "HR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(255, 0, 0)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(23, 23, 150)),
            Rect(6.4, 3.4, 9.6, 8.6, W),
            Rect(6.4, 3.4, 7.47, 4.47, Rgb(255, 0, 0)),
            Rect(8.53, 3.4, 9.6, 4.47, Rgb(255, 0, 0)),
            Rect(7.47, 4.47, 8.53, 5.53, Rgb(255, 0, 0)),
            Rect(6.4, 5.53, 7.47, 6.6, Rgb(255, 0, 0)),
            Rect(8.53, 5.53, 9.6, 6.6, Rgb(255, 0, 0)),
            Rect(7.47, 6.6, 8.53, 7.67, Rgb(255, 0, 0)),
        ],
    },
    Flag {
        country: "HU",
        outline: None,
        figures: &h3(Rgb(205, 42, 62), W, Rgb(67, 111, 77)),
    },
    Flag {
        country: "ID",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(255, 0, 0)),
            Rect(0.0, 6.0, 16.0, 12.0, W),
        ],
    },
    Flag {
        country: "IE",
        outline: None,
        figures: &v3(Rgb(22, 155, 98), W, Rgb(255, 136, 62)),
    },
    Flag {
        country: "IL",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(0.0, 1.4, 16.0, 3.0, Rgb(0, 56, 184)),
            Rect(0.0, 9.0, 16.0, 10.6, Rgb(0, 56, 184)),
            Poly(&[(8.0, 3.5), (10.4, 7.6), (5.6, 7.6)], Rgb(0, 56, 184)),
            Poly(&[(8.0, 8.5), (5.6, 4.4), (10.4, 4.4)], Rgb(0, 56, 184)),
            Poly(&[(8.0, 4.7), (9.4, 7.0), (6.6, 7.0)], W),
            Poly(&[(8.0, 7.3), (6.6, 5.0), (9.4, 5.0)], W),
        ],
    },
    Flag {
        country: "IN",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(255, 153, 51)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(19, 136, 8)),
            Disc(8.0, 6.0, 1.7, Rgb(0, 0, 128)),
            Disc(8.0, 6.0, 1.2, W),
            Disc(8.0, 6.0, 0.45, Rgb(0, 0, 128)),
        ],
    },
    Flag {
        country: "IQ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(206, 17, 38)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, K),
            Rect(5.0, 5.4, 11.0, 6.6, Rgb(0, 122, 61)),
        ],
    },
    Flag {
        country: "IR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(35, 159, 64)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(218, 0, 0)),
            Disc(8.0, 6.0, 1.2, Rgb(218, 0, 0)),
        ],
    },
    Flag {
        country: "IS",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(2, 82, 156)),
            Rect(4.5, 0.0, 7.4, 12.0, W),
            Rect(0.0, 4.7, 16.0, 7.3, W),
            Rect(5.1, 0.0, 6.7, 12.0, Rgb(220, 30, 53)),
            Rect(0.0, 5.3, 16.0, 6.7, Rgb(220, 30, 53)),
        ],
    },
    Flag {
        country: "IT",
        outline: None,
        figures: &v3(Rgb(0, 146, 70), W, Rgb(206, 43, 55)),
    },
    Flag {
        country: "JP",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Disc(8.0, 6.0, 3.6, Rgb(188, 0, 45)),
        ],
    },
    Flag {
        country: "KE",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 3.6, K),
            Rect(0.0, 3.6, 16.0, 8.4, W),
            Rect(0.0, 4.2, 16.0, 7.8, Rgb(187, 0, 0)),
            Rect(0.0, 8.4, 16.0, 12.0, Rgb(0, 102, 0)),
            Poly(
                &[(8.0, 2.4), (9.6, 6.0), (8.0, 9.6), (6.4, 6.0)],
                Rgb(187, 0, 0),
            ),
            Rect(7.6, 3.8, 8.4, 8.2, K),
        ],
    },
    Flag {
        country: "KG",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(232, 17, 45)),
            Disc(8.0, 6.0, 3.2, Rgb(255, 239, 0)),
            Disc(8.0, 6.0, 1.8, Rgb(232, 17, 45)),
            Disc(8.0, 6.0, 1.0, Rgb(255, 239, 0)),
        ],
    },
    Flag {
        country: "KH",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(3, 46, 161)),
            Rect(0.0, 3.0, 16.0, 9.0, Rgb(224, 0, 37)),
            Poly(
                &[
                    (5.0, 8.2),
                    (11.0, 8.2),
                    (10.4, 6.0),
                    (9.4, 6.0),
                    (8.6, 3.8),
                    (8.0, 3.2),
                    (7.4, 3.8),
                    (6.6, 6.0),
                    (5.6, 6.0),
                ],
                W,
            ),
        ],
    },
    Flag {
        country: "KR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Disc(8.0, 6.0, 2.8, Rgb(205, 46, 58)),
            Half(8.0, 6.0, 2.8, 6.0, true, Rgb(0, 71, 160)),
            Rect(2.6, 2.0, 4.6, 3.4, K),
            Rect(11.4, 2.0, 13.4, 3.4, K),
            Rect(2.6, 8.6, 4.6, 10.0, K),
            Rect(11.4, 8.6, 13.4, 10.0, K),
        ],
    },
    Flag {
        country: "KZ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 175, 202)),
            Disc(8.0, 5.4, 2.4, Rgb(254, 197, 12)),
            Rect(0.8, 0.0, 1.5, 12.0, Rgb(254, 197, 12)),
        ],
    },
    Flag {
        country: "LA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(206, 17, 38)),
            Rect(0.0, 3.0, 16.0, 9.0, Rgb(0, 40, 104)),
            Disc(8.0, 6.0, 2.4, W),
        ],
    },
    Flag {
        country: "LK",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(255, 190, 41)),
            Rect(0.8, 0.8, 3.2, 11.2, Rgb(0, 83, 78)),
            Rect(3.2, 0.8, 5.6, 11.2, Rgb(235, 116, 0)),
            Rect(6.4, 0.8, 15.2, 11.2, Rgb(141, 21, 58)),
            Rect(9.0, 4.0, 12.6, 8.0, Rgb(255, 190, 41)),
        ],
    },
    Flag {
        country: "LT",
        outline: None,
        figures: &h3(Rgb(253, 185, 19), Rgb(0, 106, 68), Rgb(193, 39, 45)),
    },
    Flag {
        country: "LU",
        outline: None,
        figures: &h3(Rgb(237, 41, 57), W, Rgb(0, 161, 222)),
    },
    Flag {
        country: "LV",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(158, 48, 57)),
            Rect(0.0, 4.8, 16.0, 7.2, W),
        ],
    },
    Flag {
        country: "MD",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 5.333, 12.0, Rgb(0, 70, 174)),
            Rect(5.333, 0.0, 10.667, 12.0, Rgb(255, 210, 0)),
            Rect(10.667, 0.0, 16.0, 12.0, Rgb(204, 9, 47)),
            Rect(7.0, 4.2, 9.0, 7.8, Rgb(176, 126, 64)),
        ],
    },
    Flag {
        country: "MK",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(210, 0, 0)),
            Poly(&[(8.0, 6.0), (0.0, 0.0), (1.8, 0.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (16.0, 0.0), (14.2, 0.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (0.0, 12.0), (1.8, 12.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (16.0, 12.0), (14.2, 12.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (7.2, 0.0), (8.8, 0.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (7.2, 12.0), (8.8, 12.0)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (0.0, 5.2), (0.0, 6.8)], Rgb(255, 230, 0)),
            Poly(&[(8.0, 6.0), (16.0, 5.2), (16.0, 6.8)], Rgb(255, 230, 0)),
            Disc(8.0, 6.0, 1.9, Rgb(255, 230, 0)),
            Disc(8.0, 6.0, 1.5, Rgb(210, 0, 0)),
            Disc(8.0, 6.0, 1.2, Rgb(255, 230, 0)),
        ],
    },
    Flag {
        country: "MM",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(254, 203, 0)),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(52, 178, 51)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(234, 40, 57)),
            Star(8.0, 6.5, 3.6, W),
        ],
    },
    Flag {
        country: "MN",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 5.333, 12.0, Rgb(196, 39, 47)),
            Rect(5.333, 0.0, 10.667, 12.0, Rgb(1, 81, 151)),
            Rect(10.667, 0.0, 16.0, 12.0, Rgb(196, 39, 47)),
            Rect(2.0, 3.0, 3.4, 9.0, Rgb(249, 207, 2)),
            Disc(2.7, 2.2, 0.7, Rgb(249, 207, 2)),
        ],
    },
    Flag {
        country: "MT",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 8.0, 12.0, W),
            Rect(8.0, 0.0, 16.0, 12.0, Rgb(207, 20, 43)),
            Rect(1.4, 1.4, 3.6, 3.6, Rgb(160, 160, 160)),
        ],
    },
    Flag {
        country: "MV",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(210, 16, 52)),
            Rect(4.0, 3.0, 12.0, 9.0, Rgb(0, 126, 58)),
            Disc(8.6, 6.0, 1.8, W),
            Disc(9.3, 6.0, 1.5, Rgb(0, 126, 58)),
        ],
    },
    Flag {
        country: "MX",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 5.333, 12.0, Rgb(0, 104, 71)),
            Rect(5.333, 0.0, 10.667, 12.0, W),
            Rect(10.667, 0.0, 16.0, 12.0, Rgb(206, 17, 38)),
            Disc(8.0, 6.0, 1.3, Rgb(140, 106, 61)),
        ],
    },
    Flag {
        country: "MY",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, W),
            Rect(0.0, 0.0, 16.0, 0.857, Rgb(204, 0, 1)),
            Rect(0.0, 1.714, 16.0, 2.571, Rgb(204, 0, 1)),
            Rect(0.0, 3.429, 16.0, 4.286, Rgb(204, 0, 1)),
            Rect(0.0, 5.143, 16.0, 6.0, Rgb(204, 0, 1)),
            Rect(0.0, 6.857, 16.0, 7.714, Rgb(204, 0, 1)),
            Rect(0.0, 8.571, 16.0, 9.429, Rgb(204, 0, 1)),
            Rect(0.0, 10.286, 16.0, 11.143, Rgb(204, 0, 1)),
            Rect(0.0, 0.0, 8.0, 6.857, Rgb(1, 0, 102)),
            Disc(3.2, 3.4, 2.2, Rgb(255, 204, 0)),
            Disc(3.9, 3.4, 1.9, Rgb(1, 0, 102)),
            Star(6.0, 3.4, 1.2, Rgb(255, 204, 0)),
        ],
    },
    Flag {
        country: "NG",
        outline: None,
        figures: &v3(Rgb(0, 135, 81), W, Rgb(0, 135, 81)),
    },
    Flag {
        country: "NL",
        outline: None,
        figures: &h3(Rgb(174, 28, 40), W, Rgb(33, 70, 139)),
    },
    Flag {
        country: "NO",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(186, 12, 47)),
            Rect(4.36, 0.0, 7.27, 12.0, W),
            Rect(0.0, 4.5, 16.0, 7.5, W),
            Rect(5.09, 0.0, 6.55, 12.0, Rgb(0, 32, 91)),
            Rect(0.0, 5.25, 16.0, 6.75, Rgb(0, 32, 91)),
        ],
    },
    // Nepal is the one flag that is not a rectangle: two pennants, crimson with a blue border.
    Flag {
        country: "NP",
        outline: Some(&[
            (1.5, 0.0),
            (11.5, 6.2),
            (6.0, 6.2),
            (11.5, 12.0),
            (1.5, 12.0),
        ]),
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 56, 147)),
            Poly(
                &[(2.3, 1.5), (9.0, 5.5), (4.2, 5.5), (9.6, 11.2), (2.3, 11.2)],
                Rgb(220, 20, 60),
            ),
            Disc(4.4, 4.0, 1.0, W),
            Disc(4.6, 8.8, 1.3, W),
        ],
    },
    Flag {
        country: "NZ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 36, 125)),
            Inset(&UNION_JACK, 0.0, 0.0, 0.5),
            Star(12.0, 2.4, 1.0, W),
            Star(12.0, 2.4, 0.7, Rgb(204, 20, 43)),
            Star(14.0, 5.2, 1.0, W),
            Star(14.0, 5.2, 0.7, Rgb(204, 20, 43)),
            Star(10.6, 5.8, 1.0, W),
            Star(10.6, 5.8, 0.7, Rgb(204, 20, 43)),
            Star(12.2, 9.6, 1.0, W),
            Star(12.2, 9.6, 0.7, Rgb(204, 20, 43)),
        ],
    },
    Flag {
        country: "PE",
        outline: None,
        figures: &v3(Rgb(217, 16, 35), W, Rgb(217, 16, 35)),
    },
    Flag {
        country: "PH",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(0, 56, 168)),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(206, 17, 38)),
            Poly(&[(0.0, 0.0), (10.4, 6.0), (0.0, 12.0)], W),
            Disc(3.4, 6.0, 1.4, Rgb(252, 209, 22)),
        ],
    },
    Flag {
        country: "PK",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 4.0, 12.0, W),
            Rect(4.0, 0.0, 16.0, 12.0, Rgb(1, 65, 28)),
            Disc(10.0, 6.2, 3.0, W),
            Disc(10.9, 5.5, 2.6, Rgb(1, 65, 28)),
            Star(11.8, 4.6, 1.1, W),
        ],
    },
    Flag {
        country: "PL",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, W),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(220, 20, 60)),
        ],
    },
    Flag {
        country: "PT",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 6.4, 12.0, Rgb(0, 102, 0)),
            Rect(6.4, 0.0, 16.0, 12.0, Rgb(255, 0, 0)),
            Disc(6.4, 6.0, 2.2, Rgb(255, 255, 0)),
            Disc(6.4, 6.0, 1.3, Rgb(255, 0, 0)),
            Rect(5.7, 5.0, 7.1, 7.4, W),
        ],
    },
    Flag {
        country: "RO",
        outline: None,
        figures: &v3(Rgb(0, 43, 127), Rgb(252, 209, 22), Rgb(206, 17, 38)),
    },
    Flag {
        country: "RS",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(198, 54, 60)),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(12, 64, 118)),
            Rect(0.0, 8.0, 16.0, 12.0, W),
            Poly(
                &[(3.6, 2.6), (7.0, 2.6), (7.0, 7.4), (5.3, 9.0), (3.6, 7.4)],
                Rgb(198, 54, 60),
            ),
            Rect(4.6, 4.0, 6.0, 7.0, W),
        ],
    },
    Flag {
        country: "RU",
        outline: None,
        figures: &RU3,
    },
    Flag {
        country: "SA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 108, 53)),
            Rect(3.6, 3.2, 12.4, 4.0, W),
            Rect(4.4, 4.6, 11.6, 5.4, W),
            Rect(3.6, 7.6, 12.4, 8.2, W),
        ],
    },
    Flag {
        country: "SE",
        outline: None,
        figures: &nordic(Rgb(0, 106, 167), Rgb(254, 204, 2), (5.0, 7.0), (4.8, 7.2)),
    },
    Flag {
        country: "SG",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(239, 51, 64)),
            Rect(0.0, 6.0, 16.0, 12.0, W),
            Disc(3.2, 3.0, 2.1, W),
            Disc(3.9, 3.0, 1.9, Rgb(239, 51, 64)),
            Star(5.6, 2.0, 0.5, W),
            Star(6.6, 3.0, 0.5, W),
            Star(5.6, 4.0, 0.5, W),
        ],
    },
    Flag {
        country: "SI",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, W),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(0, 93, 164)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(237, 28, 36)),
            Poly(
                &[(3.0, 2.2), (6.6, 2.2), (6.6, 5.6), (4.8, 7.0), (3.0, 5.6)],
                Rgb(237, 28, 36),
            ),
            Poly(
                &[(3.4, 2.6), (6.2, 2.6), (6.2, 5.4), (4.8, 6.5), (3.4, 5.4)],
                Rgb(0, 93, 164),
            ),
            Poly(&[(3.6, 5.2), (4.8, 3.4), (6.0, 5.2)], W),
        ],
    },
    Flag {
        country: "SK",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, W),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(11, 78, 162)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(238, 28, 37)),
            Poly(
                &[(3.0, 2.4), (7.4, 2.4), (7.4, 7.4), (5.2, 9.4), (3.0, 7.4)],
                W,
            ),
            Poly(
                &[(3.4, 2.8), (7.0, 2.8), (7.0, 7.2), (5.2, 8.8), (3.4, 7.2)],
                Rgb(238, 28, 37),
            ),
            Rect(4.8, 3.2, 5.6, 7.6, W),
            Rect(4.0, 4.2, 6.4, 4.8, W),
            Rect(3.8, 5.6, 6.6, 6.2, W),
            Poly(
                &[(3.4, 7.2), (5.2, 6.8), (7.0, 7.2), (5.2, 8.8)],
                Rgb(11, 78, 162),
            ),
        ],
    },
    Flag {
        country: "SN",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 5.333, 12.0, Rgb(0, 133, 63)),
            Rect(5.333, 0.0, 10.667, 12.0, Rgb(253, 239, 66)),
            Rect(10.667, 0.0, 16.0, 12.0, Rgb(227, 27, 35)),
            Star(8.0, 6.2, 1.8, Rgb(0, 133, 63)),
        ],
    },
    Flag {
        country: "SY",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(206, 17, 38)),
            Rect(0.0, 4.0, 16.0, 8.0, W),
            Rect(0.0, 8.0, 16.0, 12.0, K),
            Star(5.3, 6.1, 1.3, Rgb(0, 122, 61)),
            Star(10.7, 6.1, 1.3, Rgb(0, 122, 61)),
        ],
    },
    Flag {
        country: "TH",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(165, 25, 49)),
            Rect(0.0, 2.0, 16.0, 10.0, W),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(45, 42, 74)),
        ],
    },
    Flag {
        country: "TJ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 3.43, Rgb(204, 0, 0)),
            Rect(0.0, 3.43, 16.0, 8.57, W),
            Rect(0.0, 8.57, 16.0, 12.0, Rgb(0, 102, 0)),
            Poly(
                &[
                    (6.6, 7.0),
                    (6.6, 5.4),
                    (7.3, 6.0),
                    (8.0, 4.8),
                    (8.7, 6.0),
                    (9.4, 5.4),
                    (9.4, 7.0),
                ],
                Rgb(248, 195, 0),
            ),
        ],
    },
    Flag {
        country: "TM",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(0, 132, 61)),
            Rect(2.0, 0.0, 4.8, 12.0, Rgb(210, 33, 51)),
            Disc(6.6, 2.6, 1.3, W),
            Disc(7.1, 2.4, 1.1, Rgb(0, 132, 61)),
        ],
    },
    Flag {
        country: "TR",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(227, 10, 23)),
            Disc(5.4, 6.0, 3.0, W),
            Disc(6.2, 6.0, 2.4, Rgb(227, 10, 23)),
            Star(9.0, 6.0, 1.4, W),
        ],
    },
    Flag {
        country: "TW",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(254, 0, 0)),
            Rect(0.0, 0.0, 8.0, 6.0, Rgb(0, 0, 149)),
            Star(4.0, 3.1, 2.2, W),
            Disc(4.0, 3.0, 1.2, W),
        ],
    },
    Flag {
        country: "UA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(0, 87, 183)),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(255, 215, 0)),
        ],
    },
    Flag {
        country: "US",
        outline: None,
        figures: &US,
    },
    Flag {
        country: "UZ",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 3.8, Rgb(0, 153, 181)),
            Rect(0.0, 3.8, 16.0, 8.2, Rgb(206, 17, 38)),
            Rect(0.0, 4.2, 16.0, 7.8, W),
            Rect(0.0, 8.2, 16.0, 12.0, Rgb(30, 181, 58)),
            Disc(2.6, 1.9, 1.3, W),
            Disc(3.1, 1.9, 1.1, Rgb(0, 153, 181)),
        ],
    },
    Flag {
        country: "VE",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 4.0, Rgb(255, 204, 0)),
            Rect(0.0, 4.0, 16.0, 8.0, Rgb(0, 36, 125)),
            Rect(0.0, 8.0, 16.0, 12.0, Rgb(207, 20, 43)),
            Disc(5.0, 6.6, 0.45, W),
            Disc(6.0, 5.6, 0.45, W),
            Disc(7.3, 5.1, 0.45, W),
            Disc(8.7, 5.1, 0.45, W),
            Disc(10.0, 5.6, 0.45, W),
            Disc(11.0, 6.6, 0.45, W),
        ],
    },
    Flag {
        country: "VN",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 12.0, Rgb(218, 37, 29)),
            Star(8.0, 6.3, 3.4, Rgb(255, 255, 0)),
        ],
    },
    Flag {
        country: "ZA",
        outline: None,
        figures: &[
            Rect(0.0, 0.0, 16.0, 6.0, Rgb(224, 60, 49)),
            Rect(0.0, 6.0, 16.0, 12.0, Rgb(0, 20, 137)),
            Poly(
                &[
                    (0.0, -0.6),
                    (7.4, 4.2),
                    (16.0, 4.2),
                    (16.0, 7.8),
                    (7.4, 7.8),
                    (0.0, 12.6),
                ],
                W,
            ),
            Poly(
                &[
                    (0.0, 0.8),
                    (6.8, 5.0),
                    (16.0, 5.0),
                    (16.0, 7.0),
                    (6.8, 7.0),
                    (0.0, 11.2),
                ],
                Rgb(0, 119, 73),
            ),
            Poly(&[(0.0, 2.2), (5.2, 6.0), (0.0, 9.8)], Rgb(255, 184, 28)),
            Poly(&[(0.0, 3.0), (4.2, 6.0), (0.0, 9.0)], K),
        ],
    },
];

/// The flag of a country, if this table has one.
pub fn flag_of(country: &str) -> Option<usize> {
    FLAGS
        .iter()
        .position(|flag| flag.country.eq_ignore_ascii_case(country))
}

/// What the icon shows for the country and the language Windows names for a layout — the pure
/// half of [`face_of`].
///
/// The flag of the country if there is one; else a plate with the first two letters of the
/// language (ISO 639, upper case, A–Z only); a layout of which nothing is known gets a plate with
/// a question mark — the icon is never empty.
pub fn face_from(country: Option<&str>, language: Option<&str>) -> Face {
    if let Some(index) = country.and_then(flag_of) {
        return Face::Flag(index);
    }

    let letters: String = language
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .take(2)
        .map(|c| c.to_ascii_uppercase())
        .collect();

    Face::Plate(if letters.is_empty() {
        "?".to_owned()
    } else {
        letters
    })
}

/// What the icon shows for a layout — the country and the language asked of the locale service
/// for the layout's language identifier (`LCIDToLocaleName`, then `LOCALE_SISO3166CTRYNAME` and
/// `LOCALE_SISO639LANGNAME`).
pub fn face_of(layout: LayoutId) -> Face {
    face_of_language(layout.language_id())
}

/// [`face_of`] by the language identifier alone — what `tests\flags.rs` walks the registry of
/// Windows layouts with.
pub fn face_of_language(language_id: u16) -> Face {
    let mut locale = [0u16; 85];

    // SAFETY: `locale` is a live buffer of this frame, its length is the slice's, and the call
    // writes at most that many units. Zero is the documented failure and is checked below.
    let written = unsafe {
        windows::Win32::Globalization::LCIDToLocaleName(
            u32::from(language_id),
            Some(&mut locale),
            0,
        )
    };

    if written <= 0 {
        return face_from(None, None);
    }

    let info = |kind: u32| -> Option<String> {
        let mut out = [0u16; 16];

        // SAFETY: `locale` holds the NUL-terminated name the call above wrote; `out` is a live
        // buffer of this frame whose length bounds the write. Zero is the documented failure.
        let written = unsafe {
            windows::Win32::Globalization::GetLocaleInfoEx(
                windows::core::PCWSTR(locale.as_ptr()),
                kind,
                Some(&mut out),
            )
        };

        let units = usize::try_from(written).ok()?.checked_sub(1)?;
        Some(String::from_utf16_lossy(&out[..units.min(out.len())]))
    };

    let country = info(windows::Win32::Globalization::LOCALE_SISO3166CTRYNAME);
    let language = info(windows::Win32::Globalization::LOCALE_SISO639LANGNAME);

    face_from(country.as_deref(), language.as_deref())
}

// -----------------------------------------------------------------------------------------
// Drawing
// -----------------------------------------------------------------------------------------

/// The frame round the flag: a dark line on a light taskbar, a light one on a dark taskbar, at
/// 35 % — the `rgba(0, 0, 0, .35)` of the mock-up, turned round for the dark bar.
const FRAME_ON_LIGHT: Rgb = Rgb(0, 0, 0);
const FRAME_ON_DARK: Rgb = Rgb(255, 255, 255);
const FRAME_ALPHA: f32 = 0.35;

/// The plate of a country without a flag: dark with white letters on a light taskbar, light with
/// dark letters on a dark one.
const PLATE_ON_LIGHT: (Rgb, Rgb) = (Rgb(43, 47, 54), Rgb(255, 255, 255));
const PLATE_ON_DARK: (Rgb, Rgb) = (Rgb(233, 236, 240), Rgb(27, 31, 36));

/// The dot of FR-90 for an unread letter, as on the program's own icon: a light disc in a dark
/// ring.
const DOT_FILL: Rgb = Rgb(245, 246, 247);
const DOT_RING: Rgb = Rgb(43, 47, 54);

/// The pause: grey, and this much of the opacity left (решение 208.89 п. 5, the mock-up's `.6`).
const PAUSED_OPACITY: f32 = 0.6;

/// Sub-samples per pixel side: 4 × 4.
const SUB: u32 = 4;

/// The pixels of the icon: `size × size`, rows top-down, each `[b, g, r, a]` with **straight**
/// alpha — what an icon's 32-bit colour bitmap holds.
pub fn render(face: &Face, look: Look) -> Vec<[u8; 4]> {
    let size = look.size.max(1);
    let scale = size as f32 / 16.0;
    // The field of 16 × 12 sits in the middle of the square: two units of air above and below.
    let top = 2.0;
    let mut pixels = vec![[0u8; 4]; (size * size) as usize];

    for py in 0..size {
        for px in 0..size {
            let mut sum = [0f32; 3];
            let mut covered = 0u32;

            for sy in 0..SUB {
                for sx in 0..SUB {
                    let u = (px as f32 + (sx as f32 + 0.5) / SUB as f32) / scale;
                    let v = (py as f32 + (sy as f32 + 0.5) / SUB as f32) / scale - top;

                    if let Some(color) = sample(face, look, u, v) {
                        sum[0] += f32::from(color.0);
                        sum[1] += f32::from(color.1);
                        sum[2] += f32::from(color.2);
                        covered += 1;
                    }
                }
            }

            if covered == 0 {
                continue;
            }

            let n = covered as f32;
            let mut rgb = [sum[0] / n, sum[1] / n, sum[2] / n];
            let mut alpha = n / (SUB * SUB) as f32;

            if look.paused {
                let grey = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
                rgb = [grey, grey, grey];
                alpha *= PAUSED_OPACITY;
            }

            pixels[(py * size + px) as usize] = [
                rgb[2].round() as u8,
                rgb[1].round() as u8,
                rgb[0].round() as u8,
                (alpha * 255.0).round() as u8,
            ];
        }
    }

    if look.unread {
        draw_dot(&mut pixels, size, scale);
    }

    pixels
}

/// The colour at one point `(u, v)` of the field (units), or `None` outside the flag.
fn sample(face: &Face, look: Look, u: f32, v: f32) -> Option<Rgb> {
    match face {
        Face::Flag(index) => {
            let flag = FLAGS.get(*index)?;

            let inside = match flag.outline {
                None => (0.0..16.0).contains(&u) && (0.0..12.0).contains(&v),
                Some(points) => in_polygon(points, u, v),
            };

            if !inside {
                return None;
            }

            let color = figures_at(flag.figures, u, v).unwrap_or(W);

            // The thin frame: one unit along the edge of a rectangular flag.
            if flag.outline.is_none() && !(1.0..15.0).contains(&u)
                || flag.outline.is_none() && !(1.0..11.0).contains(&v)
            {
                let frame = if look.dark_bar {
                    FRAME_ON_DARK
                } else {
                    FRAME_ON_LIGHT
                };
                return Some(mix(color, frame, FRAME_ALPHA));
            }

            Some(color)
        }
        Face::Plate(letters) => {
            if !in_rounded(u, v, 16.0, 12.0, 2.0) {
                return None;
            }

            let (plate, ink) = if look.dark_bar {
                PLATE_ON_DARK
            } else {
                PLATE_ON_LIGHT
            };

            Some(if in_letters(letters, u, v) {
                ink
            } else {
                plate
            })
        }
    }
}

/// The colour of the topmost figure under `(u, v)`.
fn figures_at(figures: &[Figure], u: f32, v: f32) -> Option<Rgb> {
    figures.iter().rev().find_map(|figure| match *figure {
        Rect(x0, y0, x1, y1, color) => {
            ((x0..x1).contains(&u) && (y0..y1).contains(&v)).then_some(color)
        }
        Disc(cx, cy, r, color) => ((u - cx).powi(2) + (v - cy).powi(2) <= r * r).then_some(color),
        Half(cx, cy, r, cut, below, color) => {
            let disc = (u - cx).powi(2) + (v - cy).powi(2) <= r * r;
            let side = if below { v >= cut } else { v < cut };
            (disc && side).then_some(color)
        }
        Star(cx, cy, r, color) => in_star(cx, cy, r, u, v).then_some(color),
        Poly(points, color) => in_polygon(points, u, v).then_some(color),
        Inset(inner, x, y, scale) => {
            let iu = (u - x) / scale;
            let iv = (v - y) / scale;
            if (0.0..16.0).contains(&iu) && (0.0..12.0).contains(&iv) {
                figures_at(inner, iu, iv)
            } else {
                None
            }
        }
    })
}

/// Even-odd point in polygon.
fn in_polygon(points: &[(f32, f32)], u: f32, v: f32) -> bool {
    let mut inside = false;
    let mut j = points.len().wrapping_sub(1);

    for i in 0..points.len() {
        let (xi, yi) = points[i];
        let (xj, yj) = points[j];

        if (yi > v) != (yj > v) && u < (xj - xi) * (v - yi) / (yj - yi) + xi {
            inside = !inside;
        }

        j = i;
    }

    inside
}

/// A five-pointed star with one point up; the inner radius is 0.382 of the outer.
fn in_star(cx: f32, cy: f32, r: f32, u: f32, v: f32) -> bool {
    let mut points = [(0f32, 0f32); 10];

    for (i, point) in points.iter_mut().enumerate() {
        let radius = if i % 2 == 0 { r } else { r * 0.382 };
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        *point = (cx + radius * angle.cos(), cy + radius * angle.sin());
    }

    in_polygon(&points, u, v)
}

/// A rectangle `w × h` from the origin with corners rounded by `r`.
fn in_rounded(u: f32, v: f32, w: f32, h: f32, r: f32) -> bool {
    if !(0.0..w).contains(&u) || !(0.0..h).contains(&v) {
        return false;
    }

    let cx = u.clamp(r, w - r);
    let cy = v.clamp(r, h - r);

    (u - cx).powi(2) + (v - cy).powi(2) <= r * r
}

/// `a` with `b` laid over it at `alpha`.
fn mix(a: Rgb, b: Rgb, alpha: f32) -> Rgb {
    let m = |x: u8, y: u8| (f32::from(x) * (1.0 - alpha) + f32::from(y) * alpha).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// The dot of FR-90 in the corner: a disc of 7 units in the bottom right of the square, a light
/// fill in a dark ring of 1.5 units — the dot of the mock-up, kept inside the icon.
fn draw_dot(pixels: &mut [[u8; 4]], size: u32, scale: f32) {
    let (cx, cy, r, ring) = (16.0 - 3.5, 16.0 - 3.5, 3.5, 1.5);

    for py in 0..size {
        for px in 0..size {
            let mut fill = 0u32;
            let mut edge = 0u32;

            for sy in 0..SUB {
                for sx in 0..SUB {
                    let u = (px as f32 + (sx as f32 + 0.5) / SUB as f32) / scale;
                    let v = (py as f32 + (sy as f32 + 0.5) / SUB as f32) / scale;
                    let d = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();

                    if d <= r - ring {
                        fill += 1;
                    } else if d <= r {
                        edge += 1;
                    }
                }
            }

            if fill + edge == 0 {
                continue;
            }

            let total = (SUB * SUB) as f32;
            let cover = (fill + edge) as f32 / total;
            let dot = mix(DOT_RING, DOT_FILL, fill as f32 / (fill + edge) as f32);
            let pixel = &mut pixels[(py * size + px) as usize];

            // The dot over whatever is beneath, by "over" with straight alpha.
            let below_a = f32::from(pixel[3]) / 255.0;
            let out_a = cover + below_a * (1.0 - cover);
            let blend = |top: u8, under: u8| {
                ((f32::from(top) * cover + f32::from(under) * below_a * (1.0 - cover)) / out_a)
                    .round() as u8
            };

            *pixel = [
                blend(dot.2, pixel[0]),
                blend(dot.1, pixel[1]),
                blend(dot.0, pixel[2]),
                (out_a * 255.0).round() as u8,
            ];
        }
    }
}

/// The capital letters A–Z of the plate in five by seven, each row the five bits left to right.
const GLYPHS: [[u8; 7]; 26] = [
    [
        0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // A
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
    ], // B
    [
        0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
    ], // C
    [
        0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
    ], // D
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
    ], // E
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // F
    [
        0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
    ], // G
    [
        0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // H
    [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ], // I
    [
        0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
    ], // J
    [
        0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
    ], // K
    [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
    ], // L
    [
        0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
    ], // M
    [
        0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
    ], // N
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // O
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // P
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
    ], // Q
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
    ], // R
    [
        0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
    ], // S
    [
        0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // T
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // U
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
    ], // V
    [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
    ], // W
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
    ], // X
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // Y
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
    ], // Z
];

/// The question mark of a plate for a layout nothing is known of.
const QUESTION: [u8; 7] = [
    0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
];

/// Whether `(u, v)` falls on a stroke of the letters of a plate: glyphs of 5 × 7 units with one
/// unit between them, centred in the field of 16 × 12.
fn in_letters(letters: &str, u: f32, v: f32) -> bool {
    let glyphs: Vec<&[u8; 7]> = letters
        .chars()
        .map(|c| match c {
            'A'..='Z' => &GLYPHS[(c as u8 - b'A') as usize],
            _ => &QUESTION,
        })
        .collect();

    let count = glyphs.len() as f32;
    let width = count * 5.0 + (count - 1.0).max(0.0);
    // Whole units, so that the strokes fall on whole pixels at 100 % and 200 %.
    let left = ((16.0 - width) / 2.0).floor();
    let top = 2.0;

    let x = u - left;
    let y = v - top;

    if x < 0.0 || !(0.0..7.0).contains(&y) {
        return false;
    }

    let cell = (x / 6.0).floor();
    let column = x - cell * 6.0;

    if cell >= count || column >= 5.0 {
        return false;
    }

    let row = glyphs[cell as usize][y as usize];
    row & (0b10000 >> column as u32) != 0
}

// -----------------------------------------------------------------------------------------
// The icon
// -----------------------------------------------------------------------------------------

/// The pixels of [`render`] as an icon the shell can show — a 32-bit colour bitmap with straight
/// alpha and an empty mask, which is what an icon with an alpha channel is. `None` if the system
/// refused (NFR-13: the caller then shows the program's own icon).
///
/// The handle is the caller's to destroy (`DestroyIcon`).
pub fn icon(face: &Face, look: Look) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
        DeleteObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};

    let pixels = render(face, look);
    let side = i32::try_from(look.size.max(1)).ok()?;

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: side,
            // Negative: the rows go top-down, as `render` writes them.
            biHeight: -side,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();

    // SAFETY: `info` is a complete header of this frame for a 32-bit top-down bitmap; `bits`
    // receives the pointer to the section the call allocates, valid while the bitmap lives. No
    // device context and no file mapping are involved.
    let color =
        unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }.ok()?;

    if bits.is_null() {
        // SAFETY: `color` is the bitmap just created and not handed to anyone.
        let _ = unsafe { DeleteObject(color.into()) };
        return None;
    }

    // SAFETY: the section holds `side × side` pixels of four bytes, exactly `pixels.len() × 4`
    // bytes, and nothing else writes it while this copy runs.
    unsafe {
        std::ptr::copy_nonoverlapping(
            pixels.as_ptr().cast::<u8>(),
            bits.cast::<u8>(),
            pixels.len() * 4,
        );
    }

    // The mask: one bit per pixel, rows padded to a word, all zero — the alpha channel decides.
    let stride = usize::try_from((side + 15) / 16 * 2).ok()?;
    let zeros = vec![0u8; stride * usize::try_from(side).ok()?];

    // SAFETY: `zeros` holds the whole monochrome bitmap the call describes and lives for the call.
    let mask = unsafe { CreateBitmap(side, side, 1, 1, Some(zeros.as_ptr().cast())) };

    let made = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: color,
    };

    // SAFETY: both bitmaps are live and of one size; the call copies them, so they are deleted
    // right after, whatever it answers.
    let icon = unsafe { CreateIconIndirect(&made) }.ok();

    // SAFETY: the two bitmaps are ours and no longer needed by anybody.
    unsafe {
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
    }

    icon
}

// -----------------------------------------------------------------------------------------
// Knowing the layout without a timer — task T-119-4 (NFR-10, решения 208.113, 208.139, 208.142)
// -----------------------------------------------------------------------------------------

/// Whether the flag of FR-90б is on — published by the tray (the UI thread) whenever the setting
/// is read or changed, and read by the paths that tell the flag the layout may have moved: the
/// `WinEvent` callback of the watcher, the Raw Input of the input thread and the keyboard hook.
/// **Off, each of them costs one load of this and nothing else**: no post, no subscription, no
/// thread — «флаг выключен — нового кода нет в работе».
static FLAG_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Publishes whether the flag is on — the tray, on the UI thread.
pub fn set_on(on: bool) {
    FLAG_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the flag is on — any thread, one relaxed load.
pub fn is_on() -> bool {
    FLAG_ON.load(std::sync::atomic::Ordering::Relaxed)
}

/// `WM_APP + 25`, posted to the UI window: re-read the layout for the flag now. The first number
/// free above `app::WM_APP_OWN_LAYOUT` (`+ 24`). SEC-05: it carries nothing, and a forged one buys
/// its sender one reading of the layout of the window in front.
pub const WM_APP_FLAG_NOW: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 25;

/// `WM_APP + 26`, posted to the UI window: re-read now and again after each of [`REREAD_MS`] —
/// two one-shot timers, not a poll (NFR-10). For the moments the layout is about to change rather
/// than has changed: a mouse button released, the keys of Windows released, the focus moving, the
/// list of layouts closing. SEC-05 as [`WM_APP_FLAG_NOW`].
pub const WM_APP_FLAG_LATER: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 26;

/// The two re-reads after [`WM_APP_FLAG_LATER`], in milliseconds — PROBE Э119-П3: 60 ms holds the
/// worst case of the stand (36 ms after the release, 24 after the list closed) with room, 300 is
/// the margin for a slow machine.
pub const REREAD_MS: [u32; 2] = [60, 300];

/// The timer identifiers of the two re-reads on the UI window (the letters' clock is 2).
pub const REREAD_TIMERS: [usize; 2] = [25, 26];

/// Whether a key released now is the end of a switch of layout by the keys of Windows —
/// `Alt+Shift`, `Ctrl+Shift`, `Win+Space` (решение 208.113: the hook sees them, then one reading).
/// Pure: the caller hands in the virtual key released and which modifiers are still held.
///
/// Typing alone does not answer yes: a capital letter's `Shift` is released with neither `Alt` nor
/// `Ctrl` held, a space between words without `Win`. Either order of releasing a pair is caught —
/// the key let go first finds its partner still down. `Win` released is always a yes: it is rare,
/// and `Win+Space` may be let go of `Win` first.
pub fn ends_a_switch_by_keys(vk: u16, alt: bool, ctrl: bool, shift: bool, win: bool) -> bool {
    match vk {
        // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
        0x10 | 0xA0 | 0xA1 => alt || ctrl,
        // VK_MENU, VK_LMENU, VK_RMENU; VK_CONTROL, VK_LCONTROL, VK_RCONTROL
        0x12 | 0xA4 | 0xA5 | 0x11 | 0xA2 | 0xA3 => shift,
        // VK_SPACE
        0x20 => win,
        // VK_LWIN, VK_RWIN
        0x5B | 0x5C => true,
        _ => false,
    }
}

/// The class of the window Windows shows its list of layouts in — `Shell_InputSwitchTopLevelWindow`
/// of the process `explorer` (PROBE Э119-П2, П3). Undocumented; a Windows that renames it loses the
/// main path on Windows 10 and keeps the release of the mouse (решение 208.142).
pub const LIST_WINDOW_CLASS: &str = "Shell_InputSwitchTopLevelWindow";

/// The class of the overlay the shell of Windows 11 puts behind the list of layouts, to close it by
/// a click elsewhere — `Shell_InputSwitchDismissOverlay` of the process `explorer`. Windows 11 (26300)
/// closes the list without hiding [`LIST_WINDOW_CLASS`]: at every close — `Enter`, a click on a
/// row, `Esc` — it is this overlay that is hidden and destroyed (stand T-119-4, 4 of 4). Undocumented
/// as the other; a Windows that renames it loses the choice by the arrows and `Enter` on 11 and keeps
/// the release of the mouse.
pub const LIST_OVERLAY_CLASS: &str = "Shell_InputSwitchDismissOverlay";

/// Whether a `WinEvent` says the list of layouts closed — `EVENT_OBJECT_HIDE` or
/// `EVENT_OBJECT_DESTROY` of the window itself (`OBJID_WINDOW`, `CHILDID_SELF`) of
/// [`LIST_WINDOW_CLASS`] (Windows 10) or of its overlay [`LIST_OVERLAY_CLASS`] (Windows 11, which
/// never hides the list's own window). Not `EVENT_OBJECT_CLOAKED`: the window is born cloaked too
/// (PROBE Э119-П3), and the hide follows the choice by 8–10 ms on 10 and by ≈ 100–130 ms on 11, which
/// the 60 and 300 ms of [`REREAD_MS`] cover. Pure.
pub fn closes_the_list(event: u32, object_id: i32, child_id: i32, class: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE};

    (event == EVENT_OBJECT_HIDE || event == EVENT_OBJECT_DESTROY)
        && object_id == 0
        && child_id == 0
        && (class == LIST_WINDOW_CLASS || class == LIST_OVERLAY_CLASS)
}

/// The subscription to the list of layouts closing — Windows 10's main path of решение 208.142, and
/// on Windows 11 the one path that sees a choice by the arrows and `Enter` (through the overlay,
/// [`LIST_OVERLAY_CLASS`]) — set **only while the flag is on**, out of context, for the process `explorer` alone, events
/// `EVENT_OBJECT_DESTROY`…`EVENT_OBJECT_HIDE`. Removed when this value is dropped. Installed by the
/// UI thread, so the callback runs there and posts [`WM_APP_FLAG_LATER`] to its own window.
///
/// Explorer that restarts is another process: the tray drops this and takes a new one on
/// `TaskbarCreated` (FR-81).
pub struct ListWatch {
    hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
}

impl ListWatch {
    /// Subscribes, or `None` when there is no taskbar or the system refused (NFR-13: the flag then
    /// follows the other paths — the release of the mouse, the keys, the window in front).
    pub fn start() -> Option<Self> {
        use windows::Win32::UI::Accessibility::SetWinEventHook;
        use windows::Win32::UI::WindowsAndMessaging::{
            EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, FindWindowW, GetWindowThreadProcessId,
            WINEVENT_OUTOFCONTEXT,
        };

        // SAFETY: a constant class name, no window name; a null answer is checked.
        let taskbar = unsafe { FindWindowW(windows::core::w!("Shell_TrayWnd"), None) }.ok()?;
        let mut explorer = 0u32;

        // SAFETY: a live window handle by value and a local for the process id.
        unsafe { GetWindowThreadProcessId(taskbar, Some(&mut explorer)) };

        if explorer == 0 {
            return None;
        }

        // SAFETY: out of context — no DLL anywhere; the callback is a `'static` function of this
        // image; the range is two event codes of one process. Null is the documented failure.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_HIDE,
                None,
                Some(list_event_proc),
                explorer,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };

        (!hook.0.is_null()).then_some(Self { hook })
    }
}

impl Drop for ListWatch {
    fn drop(&mut self) {
        // SAFETY: the handle of a successful `SetWinEventHook` of this thread, unhooked once.
        let _ = unsafe { windows::Win32::UI::Accessibility::UnhookWinEvent(self.hook) };
    }
}

/// The callback of [`ListWatch`]: the list closed → [`WM_APP_FLAG_LATER`]. Nothing here can panic,
/// allocate or block; the class name is read into a stack buffer.
unsafe extern "system" fn list_event_proc(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    event: u32,
    window: windows::Win32::Foundation::HWND,
    object_id: i32,
    child_id: i32,
    _thread: u32,
    _time: u32,
) {
    if object_id != 0 || child_id != 0 || window.0.is_null() {
        return;
    }

    let mut name = [0u16; 64];

    // SAFETY: a window handle by value (the window is alive: the class is readable even in its
    // `EVENT_OBJECT_DESTROY`, PROBE Э119-П3) and a stack buffer whose length is passed.
    let len = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassNameW(window, &mut name) };
    let len = usize::try_from(len).unwrap_or(0).min(name.len());
    let class = String::from_utf16_lossy(&name[..len]);

    if closes_the_list(event, object_id, child_id, &class) {
        let _ = crate::app::post_to_ui_thread(WM_APP_FLAG_LATER);
    }
}

// -----------------------------------------------------------------------------------------
// The console — task T-119-4, решение 208.142: the name of the input indicator
// -----------------------------------------------------------------------------------------

/// `WM_APP + 27`, posted by the UI thread to the **watcher** window when the window in front is a
/// classic console (`switch::read` is blind there, FR-52) at the last re-read after an event: the
/// watcher reads the name of the input indicator by UI Automation (its STA, section 6.1) and posts
/// [`WM_APP_FLAG_NOW`] back. SEC-05: carries nothing; a forged one buys one reading of a name.
pub const WM_APP_FLAG_CONSOLE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 27;

/// The layout the input indicator named at the watcher's last reading, raw, or 0 — taken once by
/// the UI thread ([`take_console_layout`]).
static CONSOLE_LAYOUT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The layout the watcher found for a console, once: a second blind reading does not see it again.
pub fn take_console_layout() -> Option<LayoutId> {
    let raw = CONSOLE_LAYOUT.swap(0, std::sync::atomic::Ordering::Relaxed);
    (raw != 0).then(|| LayoutId::from_raw(raw))
}

/// The first layout of the session whose language the indicator's name names — the name says the
/// language, not the layout (PROBE Э119-П2, Б), so two layouts of one language are not told apart
/// and the first stands for both. `language_of` gives the language of a layout in the words of
/// Windows' interface — the words the indicator is written in. Pure.
pub fn layout_named_in(
    name: &str,
    session: &[LayoutId],
    language_of: impl Fn(LayoutId) -> Option<String>,
) -> Option<LayoutId> {
    session.iter().copied().find(|layout| {
        language_of(*layout)
            .is_some_and(|language| !language.is_empty() && name.contains(&language))
    })
}

/// The language of a layout in the words of Windows' interface — `LOCALE_SLOCALIZEDLANGUAGENAME`.
fn localized_language(layout: LayoutId) -> Option<String> {
    let mut locale = [0u16; 85];

    // SAFETY: a live buffer of this frame and its length; zero is the documented failure.
    let written = unsafe {
        windows::Win32::Globalization::LCIDToLocaleName(
            u32::from(layout.language_id()),
            Some(&mut locale),
            0,
        )
    };

    if written <= 0 {
        return None;
    }

    let mut out = [0u16; 128];

    // SAFETY: the NUL-terminated name written above and a live buffer whose length bounds the write.
    let written = unsafe {
        windows::Win32::Globalization::GetLocaleInfoEx(
            windows::core::PCWSTR(locale.as_ptr()),
            windows::Win32::Globalization::LOCALE_SLOCALIZEDLANGUAGENAME,
            Some(&mut out),
        )
    };

    let units = usize::try_from(written).ok()?.checked_sub(1)?;
    Some(String::from_utf16_lossy(&out[..units.min(out.len())]))
}

/// The watcher's answer to [`WM_APP_FLAG_CONSOLE`]: read the indicator's name, find its layout,
/// publish it and ask the UI thread to read again. Nothing when the flag went off meanwhile.
pub(crate) fn answer_console() {
    if !is_on() {
        return;
    }

    let Some(name) = indicator_name() else {
        return;
    };

    let session = crate::layouts::enumerate().unwrap_or_default();

    if let Some(layout) = layout_named_in(&name, &session, localized_language) {
        CONSOLE_LAYOUT.store(layout.raw(), std::sync::atomic::Ordering::Relaxed);
        let _ = crate::app::post_to_ui_thread(WM_APP_FLAG_NOW);
    }
}

/// The name of the input indicator (РУС/ENG) by UI Automation, on the watcher thread.
///
/// Windows 10: the Win32 window `InputIndicatorButton` inside `TrayInputIndicatorWClass` — found by
/// class, whatever the language of Windows. Windows 11: the button of the taskbar with the
/// automation id `SystemTrayIcon` whose name names a language of the session (every icon of the tray
/// has that id; the name is the only sign — decision 208.113 п. 3, ⚠ Russian Windows 11 not measured).
fn indicator_name() -> Option<String> {
    use windows::Win32::UI::Accessibility::TreeScope_Descendants;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW};
    use windows::core::w;

    let client = crate::guard::automation_client()?;

    // SAFETY: constant class names and window handles by value; a null answer is checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    let button = unsafe {
        FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None)
            .and_then(|notify| {
                FindWindowExW(Some(notify), None, w!("TrayInputIndicatorWClass"), None)
            })
            .and_then(|indicator| {
                FindWindowExW(Some(indicator), None, w!("InputIndicatorButton"), None)
            })
    };

    if let Ok(button) = button {
        // SAFETY: a live window handle by value; the call crosses into explorer and answers.
        let element = unsafe { client.ElementFromHandle(button) }.ok()?;
        // SAFETY: a live element of this STA.
        return unsafe { element.CurrentName() }
            .ok()
            .map(|name| name.to_string());
    }

    // Windows 11.
    // SAFETY: as above; the true condition needs no VARIANT (the feature list is closed).
    let root = unsafe { client.ElementFromHandle(taskbar) }.ok()?;
    let condition = unsafe { client.CreateTrueCondition() }.ok()?;
    let all = unsafe { root.FindAll(TreeScope_Descendants, &condition) }.ok()?;
    let count = unsafe { all.Length() }.ok()?;
    let session = crate::layouts::enumerate().unwrap_or_default();

    for index in 0..count {
        // SAFETY: an index inside the length the array answered.
        let Ok(element) = (unsafe { all.GetElement(index) }) else {
            continue;
        };
        let id = unsafe { element.CurrentAutomationId() }
            .map(|id| id.to_string())
            .unwrap_or_default();

        if id != "SystemTrayIcon" {
            continue;
        }

        let name = unsafe { element.CurrentName() }
            .map(|name| name.to_string())
            .unwrap_or_default();

        if layout_named_in(&name, &session, localized_language).is_some() {
            return Some(name);
        }
    }

    None
}
