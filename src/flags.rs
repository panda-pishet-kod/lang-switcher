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

/// The callback of [`ListWatch`]: the list closed → [`WM_APP_FLAG_LATER`] and
/// [`WM_APP_LIST_CLOSED`]. Nothing here can panic or block; the class name is read into a stack
/// buffer.
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
        // Task T-119-3: a list the program opened itself is finished by the UI thread; a list the
        // person opened at the clock is not the program's — the UI thread drops this then.
        let _ = crate::app::post_to_ui_thread(WM_APP_LIST_CLOSED);
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
///
/// ⚠ **Task T-119-3: a name with the program's own in it is never the indicator.** The tooltip of
/// the flag — the name of its button on the taskbar of Windows 11, where every button of the tray
/// has the automation id `SystemTrayIcon` — begins with the language of its layout since T-119-2,
/// and the flag stands left of РУС/ENG: unanswered, the console would read the flag's own layout
/// back and a click on the flag would press the flag.
pub fn layout_named_in(
    name: &str,
    session: &[LayoutId],
    language_of: impl Fn(LayoutId) -> Option<String>,
) -> Option<LayoutId> {
    if name.contains(crate::APP_NAME) {
        return None;
    }

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
    let client = crate::guard::automation_client()?;

    if let Some(button) = indicator_button() {
        // SAFETY: a live window handle by value; the call crosses into explorer and answers.
        let element = unsafe { client.ElementFromHandle(button) }.ok()?;
        // SAFETY: a live element of this STA.
        return unsafe { element.CurrentName() }
            .ok()
            .map(|name| name.to_string());
    }

    let element = indicator_element(&client)?;

    // SAFETY: a live element of this STA.
    unsafe { element.CurrentName() }
        .ok()
        .map(|name| name.to_string())
}

/// Windows 10: the Win32 button of the input indicator — `InputIndicatorButton` inside
/// `TrayInputIndicatorWClass` of the taskbar, found by class whatever the language of Windows.
/// `None` on Windows 11, which has no such window.
fn indicator_button() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW};
    use windows::core::w;

    // SAFETY: constant class names and window handles by value; a null answer is checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;

    // SAFETY: as above.
    unsafe {
        FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None)
            .and_then(|notify| {
                FindWindowExW(Some(notify), None, w!("TrayInputIndicatorWClass"), None)
            })
            .and_then(|indicator| {
                FindWindowExW(Some(indicator), None, w!("InputIndicatorButton"), None)
            })
    }
    .ok()
}

/// Windows 11: the button of the taskbar with the automation id `SystemTrayIcon` whose name names
/// a language of the session — and not the flag's own ([`layout_named_in`]). On the watcher
/// thread's STA.
fn indicator_element(
    client: &windows::Win32::UI::Accessibility::IUIAutomation,
) -> Option<windows::Win32::UI::Accessibility::IUIAutomationElement> {
    use windows::Win32::UI::Accessibility::TreeScope_Descendants;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::w;

    // SAFETY: a constant class name; a null answer is checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: a live window handle by value; the true condition needs no VARIANT (the feature list
    // is closed).
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
            return Some(element);
        }
    }

    None
}

// -----------------------------------------------------------------------------------------
// A click on the flag — task T-119-3, решения 208.88, 208.123, 208.164
// -----------------------------------------------------------------------------------------

/// `WM_APP + 28`, posted to the UI window by the callback of [`ListWatch`] beside
/// [`WM_APP_FLAG_LATER`] when the list of layouts closed. For a list the program opened itself the
/// UI thread gives the focus back to the window the person typed in and, where the choice did not
/// reach it, the layout ([`layout_to_give`]). SEC-05: carries nothing.
pub const WM_APP_LIST_CLOSED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 28;

/// `WM_APP + 29`, posted by the UI thread to the **watcher** window: press the input indicator of
/// Windows 11 by UI Automation (its STA, section 6.1). SEC-05: carries nothing; a forged one opens
/// the list of layouts, which a click on РУС/ENG does anyway.
pub const WM_APP_LIST_OPEN: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 29;

/// `WM_APP + 30`, posted by the watcher back to the UI window when it found no input indicator to
/// press — РУС/ENG is hidden at the clock, and there is no list of Windows to open. SEC-05: nothing.
pub const WM_APP_LIST_MISSING: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 30;

/// The one-shot timers of the UI window for a list opened by a click on the flag: whether it
/// opened at all (1500 ms after the press), the reading of the choice after it closed (60 ms), the
/// look at the person's window once the focus went back (120 ms). Identifiers of `tray`'s window,
/// after [`REREAD_TIMERS`].
pub const LIST_TIMERS: [usize; 3] = [27, 28, 29];

/// The delays of [`LIST_TIMERS`], in milliseconds.
pub const LIST_MS: [u32; 3] = [1500, 60, 120];

/// Whether a window of this class is a part of the shell rather than a window a person types in —
/// the taskbars, the overflow of the tray, the list of layouts and its overlay, the popups of XAML
/// and of the Start menu, the desktop. A click on the flag puts the taskbar in front, and the focus
/// goes back to the last window that is **not** one of these. Pure.
pub fn is_shell_window_class(class: &str) -> bool {
    matches!(
        class,
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
            | "TopLevelWindowForOverflowXamlIsland"
            | "Xaml_WindowedPopupClass"
            | "XamlExplorerHostIslandWindow"
            | "Windows.UI.Core.CoreWindow"
            | "Windows.UI.Input.InputSite.WindowClass"
            | "#32768"
            | "Progman"
            | "WorkerW"
    ) || class == LIST_WINDOW_CLASS
        || class == LIST_OVERLAY_CLASS
}

/// What the person's window gets once a list the program opened has closed — решение 208.164: the
/// layout chosen in the list, and only where it did not arrive by itself. `before` — the layout the
/// flag showed at the click; `chosen` — what the shell's thread reads after the close; `now` and
/// `blind` — the reading of the person's window once the focus went back. Edge does not get a layout
/// chosen in a list the program opened (T-119-1: 0 of 20 on each Windows), Блокнот and cmd do (20
/// of 20) — there nothing is switched a second time and nothing sounds. Esc, a click aside and the
/// same row choose nothing; a blind reading (a console) and an unread one decide nothing. Pure.
pub fn layout_to_give(
    before: LayoutId,
    chosen: LayoutId,
    now: LayoutId,
    blind: bool,
) -> Option<LayoutId> {
    let known = chosen.raw() != 0 && now.raw() != 0 && !blind;

    (known && chosen != before && now != chosen).then_some(chosen)
}

/// The same for the list of the program's own (part Б): nothing chosen there reaches the window by
/// itself, so the chosen layout is given unless the window is already in it — and a console, whose
/// reading is blind, is given it too (FR-50 posts to a console without verifying). Pure.
pub fn own_layout_to_give(chosen: LayoutId, now: LayoutId, blind: bool) -> Option<LayoutId> {
    (chosen.raw() != 0 && (blind || now != chosen)).then_some(chosen)
}

/// Windows 10: presses the input indicator the way the probe measured 80 of 80 (решение 208.123) —
/// `WM_LBUTTONDOWN` and `WM_LBUTTONUP` posted to the middle of `InputIndicatorButton`. UI Automation
/// `Invoke` races the focus there (9 of 31), MSAA needs `VARIANT`, outside the closed feature list.
/// `false` when there is no such button or it is hidden — Windows 11, or РУС/ENG turned off.
pub fn press_indicator_button() -> bool {
    use windows::Win32::Foundation::{LPARAM, RECT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, IsWindowVisible, PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP,
    };

    let Some(button) = indicator_button() else {
        return false;
    };

    // SAFETY: a window handle by value; the answer is a plain BOOL.
    if !unsafe { IsWindowVisible(button) }.as_bool() {
        return false;
    }

    let mut client = RECT::default();

    // SAFETY: a window handle by value and a local rectangle.
    if unsafe { GetClientRect(button, &mut client) }.is_err() {
        return false;
    }

    let x = u16::try_from(client.right.max(0) / 2).unwrap_or(0);
    let y = u16::try_from(client.bottom.max(0) / 2).unwrap_or(0);
    let point = LPARAM(isize::try_from((u32::from(y) << 16) | u32::from(x)).unwrap_or(0));

    // SAFETY: posts of the system's own mouse messages to a window by value; they queue and return.
    let down = unsafe { PostMessageW(Some(button), WM_LBUTTONDOWN, WPARAM(0x0001), point) };
    // SAFETY: as above.
    let up = unsafe { PostMessageW(Some(button), WM_LBUTTONUP, WPARAM(0), point) };

    down.is_ok() && up.is_ok()
}

/// The watcher's answer to [`WM_APP_LIST_OPEN`] — Windows 11: the input indicator pressed by UI
/// Automation `Invoke` (PROBE Э119-П, every time), or [`WM_APP_LIST_MISSING`] back to the UI thread
/// when there is no indicator to press. Nothing when the flag went off meanwhile.
pub(crate) fn answer_list_open() {
    use windows::Win32::UI::Accessibility::{IUIAutomationInvokePattern, UIA_InvokePatternId};

    if !is_on() {
        return;
    }

    let pressed = crate::guard::automation_client()
        .and_then(|client| indicator_element(&client))
        .and_then(|element| {
            // SAFETY: a live element of this STA; the pattern is asked by its own identifier.
            unsafe {
                element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
            }
            .ok()
        })
        // SAFETY: a live pattern of this STA; the call crosses into explorer and returns.
        .is_some_and(|invoke| unsafe { invoke.Invoke() }.is_ok());

    if !pressed {
        let _ = crate::app::post_to_ui_thread(WM_APP_LIST_MISSING);
    }
}

/// Whether the list of layouts of Windows is on the screen — a visible, uncloaked window of
/// [`LIST_WINDOW_CLASS`] (решение 208.123), or on Windows 11 its overlay.
pub fn list_is_open() -> bool {
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};
    use windows::core::w;

    let shown = |window: windows::Win32::Foundation::HWND| {
        let mut cloaked = 0u32;

        // SAFETY: a window handle by value and a local of the size passed.
        let read = unsafe {
            DwmGetWindowAttribute(
                window,
                DWMWA_CLOAKED,
                (&raw mut cloaked).cast(),
                u32::try_from(size_of::<u32>()).unwrap_or(4),
            )
        };

        // SAFETY: a window handle by value.
        unsafe { IsWindowVisible(window) }.as_bool() && (read.is_err() || cloaked == 0)
    };

    // SAFETY: constant class names; a null answer is checked.
    let list = unsafe { FindWindowW(w!("Shell_InputSwitchTopLevelWindow"), None) };
    // SAFETY: as above.
    let overlay = unsafe { FindWindowW(w!("Shell_InputSwitchDismissOverlay"), None) };

    list.is_ok_and(shown) || overlay.is_ok_and(shown)
}

// -----------------------------------------------------------------------------------------
// The list of the program's own — task T-119-3, part Б (решение 208.88): РУС/ENG hidden at the
// clock, Windows has no list to open, and the flag shows a copy of it in the style of Windows 11
// (the mock-up's `.w11.sw`, design-notes of E119): the title «Раскладка», a row per layout of the
// session — the code, the name — the row of the layout in force marked by the accent bar, the row
// under the mouse or the keyboard by a ground, a separator and the link «Дополнительные параметры
// клавиатуры». Not a new style: the colours are those of the list of Windows, light or dark as
// Windows is.
// -----------------------------------------------------------------------------------------

/// One row of the list of the program's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnRow {
    /// The layout the row switches to.
    pub layout: LayoutId,
    /// The code in the left column — the three letters of the language, `ENG`, `RUS`.
    pub code: String,
    /// The name — the language in its own words, as the lists of the settings window give it
    /// (вопрос 92, the discriminator of решение 82.1 when two rows share it).
    pub name: String,
    /// The second line — the name Windows gives the keyboard layout (`US`, `Russian`), as in the
    /// list of Windows; empty when Windows names it nowhere (решение контролёра по вопросу 82,
    /// 2026-11-20).
    pub layout_name: String,
}

/// The rows of the list: every layout of the session in its order, with its code, its language
/// name and the name of its keyboard layout. Pure.
pub fn own_rows(
    session: &[LayoutId],
    names: &[String],
    code_of: impl Fn(LayoutId) -> String,
    layout_name_of: impl Fn(LayoutId) -> String,
) -> Vec<OwnRow> {
    session
        .iter()
        .zip(names)
        .map(|(layout, name)| OwnRow {
            layout: *layout,
            code: code_of(*layout),
            name: name.clone(),
            layout_name: layout_name_of(*layout),
        })
        .collect()
}

/// The code of a layout's language — `LOCALE_SISO639LANGNAME2` in capitals (`ENG`, `RUS`), what the
/// indicator of an English Windows writes; empty when Windows does not name the language.
pub fn language_code(layout: LayoutId) -> String {
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
        return String::new();
    }

    let mut out = [0u16; 16];

    // SAFETY: the NUL-terminated name written above and a live buffer whose length bounds the write.
    let written = unsafe {
        windows::Win32::Globalization::GetLocaleInfoEx(
            windows::core::PCWSTR(locale.as_ptr()),
            windows::Win32::Globalization::LOCALE_SISO639LANGNAME2,
            Some(&mut out),
        )
    };

    let units = usize::try_from(written).unwrap_or(0).saturating_sub(1);
    String::from_utf16_lossy(&out[..units.min(out.len())]).to_uppercase()
}

/// The geometry of the list at a DPI, in pixels — the mock-up's numbers at 96 DPI, scaled. Pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnGeometry {
    /// Width of the window (300 at 96 DPI).
    pub width: i32,
    /// The inner padding of the window (6).
    pub padding: i32,
    /// The height of the title «Раскладка» with its own padding (25).
    pub title: i32,
    /// The height of a row — two lines, the language and the keyboard layout (52).
    pub row: i32,
    /// The width of the column of the code (44).
    pub code: i32,
    /// The separator with its margins (9).
    pub separator: i32,
    /// The height of the link (32).
    pub link: i32,
    /// The height of the whole window.
    pub height: i32,
}

/// [`OwnGeometry`] for `rows` rows at `dpi`. Pure.
pub fn own_geometry(rows: usize, dpi: i32) -> OwnGeometry {
    let dpi = if dpi > 0 { dpi } else { 96 };
    let at = |px: i32| (px * dpi + 48) / 96;
    let rows = i32::try_from(rows).unwrap_or(64).clamp(1, 64);

    let padding = at(6);
    let title = at(25);
    let row = at(52);
    let separator = at(9);
    let link = at(32);

    OwnGeometry {
        width: at(300),
        padding,
        title,
        row,
        code: at(44),
        separator,
        link,
        height: padding * 2 + title + row * rows + separator + link,
    }
}

/// Where the list stands — the window's left-top — beside the icon it was opened from, inside the
/// work area: above the icon when the taskbar is below, below it when the taskbar is at the top,
/// beside it when the taskbar is on a side; its right edge on the icon's right. `gap` — the distance
/// from the icon. Pure.
pub fn own_place(
    icon: windows::Win32::Foundation::RECT,
    work: windows::Win32::Foundation::RECT,
    size: (i32, i32),
    gap: i32,
) -> (i32, i32) {
    let (width, height) = size;
    let (x, y) = if icon.top >= work.bottom - gap {
        // The taskbar is below the work area.
        (icon.right - width, icon.top - gap - height)
    } else if icon.bottom <= work.top + gap {
        // At the top.
        (icon.right - width, icon.bottom + gap)
    } else if icon.left >= work.right - gap {
        // On the right.
        (icon.left - gap - width, icon.bottom - height)
    } else if icon.right <= work.left + gap {
        // On the left.
        (icon.right + gap, icon.bottom - height)
    } else {
        // Inside the work area — the open list of ^ above the taskbar.
        (icon.right - width, icon.top - gap - height)
    };

    (
        x.clamp(work.left, (work.right - width).max(work.left)),
        y.clamp(work.top, (work.bottom - height).max(work.top)),
    )
}

/// The row the keyboard marks after a key — `VK_UP`, `VK_DOWN`, `VK_HOME`, `VK_END` — in a list of
/// `rows` rows, as in the list of Windows: no wrapping. Other keys leave it. Pure.
pub fn own_step(at: usize, rows: usize, vk: u16) -> usize {
    let last = rows.saturating_sub(1);

    match vk {
        0x26 => at.saturating_sub(1).min(last),
        0x28 => (at + 1).min(last),
        0x24 => 0,
        0x23 => last,
        _ => at.min(last),
    }
}

/// How a list of the program's own ended — what `tray::own_list_ended` is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnListEnd {
    /// A row was chosen — by the mouse, or by `Enter` / space on the marked row.
    Chosen(LayoutId),
    /// `Esc`: the list closed with nothing chosen; the focus goes back to the person's window.
    Escaped,
    /// The list lost the activation — a click elsewhere, the link: nothing chosen, and the focus is
    /// where the person put it.
    Away,
}

/// The class of the window of the list.
const OWN_CLASS: windows::core::PCWSTR = windows::core::w!("LangSwitcher.LayoutList");

/// The URI the link opens — the keyboard page of the settings of Windows.
const OWN_LINK: &str = "ms-settings:keyboard";

/// The colours of the list of Windows 11 — the mock-up's, light and dark: the ground, the frame and
/// separator, the title, the text, the ground of a marked row, the accent (bar, link).
#[derive(Clone, Copy)]
struct OwnColours {
    ground: (u8, u8, u8),
    frame: (u8, u8, u8),
    muted: (u8, u8, u8),
    text: (u8, u8, u8),
    hover: (u8, u8, u8),
    accent: (u8, u8, u8),
}

const OWN_LIGHT: OwnColours = OwnColours {
    ground: (0xF9, 0xF9, 0xF9),
    frame: (0xE0, 0xE0, 0xE0),
    muted: (0x5F, 0x5F, 0x5F),
    text: (0x1A, 0x1A, 0x1A),
    hover: (0xED, 0xED, 0xED),
    accent: (0x00, 0x5F, 0xB8),
};

const OWN_DARK: OwnColours = OwnColours {
    ground: (0x2C, 0x2C, 0x2C),
    frame: (0x41, 0x41, 0x41),
    muted: (0xB8, 0xB8, 0xB8),
    text: (0xFF, 0xFF, 0xFF),
    hover: (0x3F, 0x3F, 0x3F),
    accent: (0x60, 0xCD, 0xFF),
};

fn colour((r, g, b): (u8, u8, u8)) -> windows::Win32::Foundation::COLORREF {
    windows::Win32::Foundation::COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16))
}

/// The state of the list on the screen — one at a time, on the UI thread.
struct OwnState {
    hwnd: windows::Win32::Foundation::HWND,
    rows: Vec<OwnRow>,
    current: LayoutId,
    marked: usize,
    link_hover: bool,
    tracking: bool,
    title: String,
    link: String,
    ended: bool,
}

thread_local! {
    static OWN: std::cell::RefCell<Option<OwnState>> = const { std::cell::RefCell::new(None) };
}

/// Whether the list of the program's own is on the screen.
pub fn own_list_is_open() -> bool {
    OWN.with(|own| own.borrow().is_some())
}

/// Shows the list of the program's own beside `icon` (the rectangle of the flag's icon), with
/// `current` marked, and gives it the foreground — the click on the flag gave this process the
/// right. `false` when the window could not be made (NFR-13: the click then does nothing).
pub fn show_own_list(icon: windows::Win32::Foundation::RECT, current: LayoutId) -> bool {
    use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CS_DROPSHADOW, CreateWindowExW, HWND_TOPMOST, IDC_ARROW, LoadCursorW, RegisterClassExW,
        SW_SHOW, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow, WNDCLASSEXW,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    if own_list_is_open() {
        return true;
    }

    let session = crate::layouts::enumerate().unwrap_or_default();

    if session.is_empty() {
        return false;
    }

    let names = crate::settings::layout_labels(&session, &session);
    let rows = own_rows(&session, &names, language_code, |layout| {
        crate::layouts::display_name(layout).unwrap_or_default()
    });
    let marked = rows
        .iter()
        .position(|row| row.layout == current)
        .unwrap_or(0);

    // SAFETY: `None` names the running executable; the handle is borrowed, not freed.
    let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
        return false;
    };

    // SAFETY: a stock cursor of the system; a refusal leaves the class without one.
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default();
    let class = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(own_proc),
        hInstance: module.into(),
        hCursor: cursor,
        lpszClassName: OWN_CLASS,
        ..Default::default()
    };

    // SAFETY: a live structure of this frame; a second registration of the class answers 0 and is
    // no error — the class is already there.
    let _ = unsafe { RegisterClassExW(&class) };

    // SAFETY: the class just registered, a module of this process, no parent: a top-level popup.
    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            OWN_CLASS,
            windows::core::PCWSTR::null(),
            WS_POPUP,
            icon.left,
            icon.top,
            1,
            1,
            None,
            None,
            Some(module.into()),
            None,
        )
    }) else {
        return false;
    };

    // Windows 11 rounds the corners as it rounds its own list's (`DWMWA_WINDOW_CORNER_PREFERENCE`,
    // `DWMWCP_ROUND`); Windows 10 has no such attribute and keeps them square.
    let round = 2u32;

    // SAFETY: the live window just made and a local of the size passed.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            (&raw const round).cast(),
            u32::try_from(size_of::<u32>()).unwrap_or(4),
        )
    };

    let dpi = window_dpi(hwnd);
    let geometry = own_geometry(rows.len(), dpi);

    // SAFETY: a rectangle by reference; the nearest monitor is always answered.
    let monitor = unsafe { MonitorFromRect(&icon, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: a monitor handle by value and a live local whose `cbSize` describes it.
    let _ = unsafe { GetMonitorInfoW(monitor, &mut info) };

    let (x, y) = own_place(
        icon,
        info.rcWork,
        (geometry.width, geometry.height),
        (8 * dpi + 48) / 96,
    );

    OWN.with(|own| {
        *own.borrow_mut() = Some(OwnState {
            hwnd,
            rows,
            current,
            marked,
            link_hover: false,
            tracking: false,
            title: crate::settings::text(crate::settings::IDS_OWN_LIST_TITLE),
            link: crate::settings::text(crate::settings::IDS_OWN_LIST_MORE),
            ended: false,
        });
    });

    // SAFETY: the live window of this thread; the position and size computed above.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            geometry.width,
            geometry.height,
            SWP_SHOWWINDOW,
        )
    };

    // SAFETY: as above.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
    // SAFETY: as above; the click on the flag gave this process the right to the foreground.
    let _ = unsafe { SetForegroundWindow(hwnd) };

    true
}

/// The DPI of the monitor a window is on, by its DC (`PerMonitorV2`, section 8.1); 96 when no DC.
fn window_dpi(hwnd: windows::Win32::Foundation::HWND) -> i32 {
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

    // SAFETY: a live window of this thread; the DC is released below on the same thread.
    let dc = unsafe { GetDC(Some(hwnd)) };

    if dc.is_invalid() {
        return 96;
    }

    let dpi = crate::theme::dc_dpi(dc);

    // SAFETY: the DC taken a line above from this window.
    unsafe { ReleaseDC(Some(hwnd), dc) };

    dpi
}

/// Ends the list: the window goes, the state goes, and the tray is told how — outside the borrow.
fn own_end(end: OwnListEnd) {
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

    let hwnd = OWN.with(|own| {
        let mut own = own.borrow_mut();
        let state = own.as_mut()?;

        if state.ended {
            return None;
        }

        state.ended = true;
        Some(state.hwnd)
    });

    let Some(hwnd) = hwnd else {
        return;
    };

    // ⚠ The tray first — it gives the focus back to the person's window **while the list is still
    // the foreground window of this process**. Destroyed first, the list handed the foreground to
    // the taskbar and the program's `SetForegroundWindow` was refused (stand T-119-3, Windows 11,
    // 2026-11-20: the focus and the layout stayed behind 4 times of 4 by the keys). The `WM_ACTIVATE`
    // that follows finds `ended` set and adds nothing.
    crate::tray::own_list_ended(end);

    // SAFETY: the live window of this thread, destroyed once (`ended`).
    let _ = unsafe { DestroyWindow(hwnd) };

    OWN.with(|own| *own.borrow_mut() = None);
}

/// Which row (or the link, `Some(usize::MAX)`) a point of the client area falls on. Pure.
pub fn own_hit(rows: usize, geometry: &OwnGeometry, x: i32, y: i32) -> Option<usize> {
    if x < geometry.padding || x >= geometry.width - geometry.padding {
        return None;
    }

    let top = geometry.padding + geometry.title;
    let rows = i32::try_from(rows).unwrap_or(0);

    if y >= top && y < top + geometry.row * rows {
        return usize::try_from((y - top) / geometry.row).ok();
    }

    let link = top + geometry.row * rows + geometry.separator;

    (y >= link && y < link + geometry.link).then_some(usize::MAX)
}

/// The row count of the list on the screen, for its geometry (1 when there is none).
fn own_rows_now() -> usize {
    OWN.with(|own| own.borrow().as_ref().map_or(1, |state| state.rows.len()))
}

/// The window procedure of the list.
unsafe extern "system" fn own_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::Graphics::Gdi::InvalidateRect;
    use windows::Win32::UI::Controls::WM_MOUSELEAVE;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, WA_INACTIVE, WM_ACTIVATE, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
    };

    let low = |value: isize| i32::from((value & 0xFFFF) as u16 as i16);
    let high = |value: isize| i32::from(((value >> 16) & 0xFFFF) as u16 as i16);
    let redraw = || {
        // SAFETY: the live window this procedure runs for.
        let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
    };

    match message {
        WM_PAINT => {
            own_paint(hwnd);
            return LRESULT(0);
        }
        WM_ACTIVATE if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE => {
            own_end(OwnListEnd::Away);
            return LRESULT(0);
        }
        WM_KEYDOWN => {
            let vk = u16::try_from(wparam.0 & 0xFFFF).unwrap_or(0);

            match vk {
                // VK_ESCAPE
                0x1B => own_end(OwnListEnd::Escaped),
                // VK_RETURN, VK_SPACE
                0x0D | 0x20 => {
                    let chosen = OWN.with(|own| {
                        own.borrow()
                            .as_ref()
                            .and_then(|state| state.rows.get(state.marked).map(|row| row.layout))
                    });

                    if let Some(layout) = chosen {
                        own_end(OwnListEnd::Chosen(layout));
                    }
                }
                _ => {
                    OWN.with(|own| {
                        if let Some(state) = own.borrow_mut().as_mut() {
                            state.marked = own_step(state.marked, state.rows.len(), vk);
                        }
                    });
                    redraw();
                }
            }

            return LRESULT(0);
        }
        WM_MOUSEMOVE => {
            let geometry = own_geometry(own_rows_now(), window_dpi(hwnd));
            let first = OWN.with(|own| {
                let mut own = own.borrow_mut();
                let state = own.as_mut()?;
                let hit = own_hit(state.rows.len(), &geometry, low(lparam.0), high(lparam.0));

                if let Some(at) = hit.filter(|at| *at != usize::MAX) {
                    state.marked = at;
                }

                state.link_hover = hit == Some(usize::MAX);

                let first = !state.tracking;
                state.tracking = true;
                Some(first)
            });

            if first == Some(true) {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: u32::try_from(size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };

                // SAFETY: a live local whose `cbSize` describes it, for the live window.
                let _ = unsafe { TrackMouseEvent(&mut tracking) };
            }

            redraw();
            return LRESULT(0);
        }
        WM_MOUSELEAVE => {
            OWN.with(|own| {
                if let Some(state) = own.borrow_mut().as_mut() {
                    state.link_hover = false;
                    state.tracking = false;
                }
            });
            redraw();
            return LRESULT(0);
        }
        WM_LBUTTONUP => {
            let geometry = own_geometry(own_rows_now(), window_dpi(hwnd));
            let hit = OWN.with(|own| {
                let own = own.borrow();
                let state = own.as_ref()?;
                let at = own_hit(state.rows.len(), &geometry, low(lparam.0), high(lparam.0))?;

                Some(if at == usize::MAX {
                    None
                } else {
                    state.rows.get(at).map(|row| row.layout)
                })
            });

            match hit {
                Some(Some(layout)) => own_end(OwnListEnd::Chosen(layout)),
                Some(None) => {
                    own_end(OwnListEnd::Away);
                    open_keyboard_settings();
                }
                None => {}
            }

            return LRESULT(0);
        }
        _ => {}
    }

    // SAFETY: the arguments this procedure was called with, handed on unchanged.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// The link: the keyboard page of the settings of Windows — through the one door this program
/// opens anything by (`settings::open_in_the_shell`). A page that does not open is seen by the
/// person, and nothing of this program depends on it.
fn open_keyboard_settings() {
    let _ = crate::settings::open_in_the_shell(OWN_LINK);
}

/// `WM_PAINT` of the list: the ground and the frame, the title, the rows (the marked one on its
/// ground, the layout in force with the accent bar), the separator, the link.
fn own_paint(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateFontIndirectW, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
        DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint, FW_NORMAL,
        FW_SEMIBOLD, FillRect, FrameRect, HFONT, HGDIOBJ, PAINTSTRUCT, SelectObject, SetBkMode,
        SetTextColor, TRANSPARENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut paint = PAINTSTRUCT::default();

    // SAFETY: the live window this `WM_PAINT` is for; ended by `EndPaint` below.
    let dc = unsafe { BeginPaint(hwnd, &mut paint) };

    let snapshot = OWN.with(|own| {
        own.borrow().as_ref().map(|state| {
            (
                state.rows.clone(),
                state.current,
                state.marked,
                state.link_hover,
                state.title.clone(),
                state.link.clone(),
            )
        })
    });

    if let Some((rows, current, marked, link_hover, title, link)) = snapshot {
        let dpi = crate::theme::dc_dpi(dc);
        let geometry = own_geometry(rows.len(), dpi);
        let colours = if crate::theme::system_is_light() {
            OWN_LIGHT
        } else {
            OWN_DARK
        };
        let at = |px: i32| (px * dpi + 48) / 96;
        let mut client = RECT::default();

        // SAFETY: the live window and a local rectangle.
        let _ = unsafe { GetClientRect(hwnd, &mut client) };

        // SAFETY: colours by value; the brushes are deleted at the end of this paint.
        let (ground, frame, hover, accent) = unsafe {
            (
                CreateSolidBrush(colour(colours.ground)),
                CreateSolidBrush(colour(colours.frame)),
                CreateSolidBrush(colour(colours.hover)),
                CreateSolidBrush(colour(colours.accent)),
            )
        };

        // SAFETY: the live paint DC and brushes of this frame.
        unsafe {
            FillRect(dc, &client, ground);
            FrameRect(dc, &client, frame);
        }

        let font = |px: i32, weight: u32| -> HFONT {
            let mut logical = crate::tray::menu_item_logfont().unwrap_or_default();
            logical.lfHeight = -at(px);
            logical.lfWidth = 0;
            logical.lfWeight = i32::try_from(weight).unwrap_or(400);

            // SAFETY: a live local; the font is deleted at the end of this paint.
            unsafe { CreateFontIndirectW(&raw const logical) }
        };

        let small = font(11, FW_NORMAL.0);
        let normal = font(13, FW_NORMAL.0);
        let bold = font(13, FW_SEMIBOLD.0);

        // SAFETY: the live paint DC; the mode is the DC's own for this paint.
        unsafe { SetBkMode(dc, TRANSPARENT) };

        let text = |what: &str, mut area: RECT, ink: (u8, u8, u8), face: HFONT, centred: bool| {
            let mut wide: Vec<u16> = what.encode_utf16().collect();
            let align = if centred { DT_CENTER } else { DT_LEFT };

            // SAFETY: the live paint DC, a live font of this frame, a local buffer and rectangle;
            // the previous font is selected back.
            unsafe {
                SetTextColor(dc, colour(ink));
                let previous = SelectObject(dc, HGDIOBJ::from(face));
                DrawTextW(
                    dc,
                    &mut wide,
                    &mut area,
                    align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
                );
                SelectObject(dc, previous);
            }
        };

        let left = geometry.padding;
        let right = geometry.width - geometry.padding;
        let mut top = geometry.padding;

        text(
            &title,
            RECT {
                left: left + at(8),
                top: top + at(4),
                right: right - at(8),
                bottom: top + geometry.title - at(6),
            },
            colours.muted,
            small,
            false,
        );
        top += geometry.title;

        for (index, row) in rows.iter().enumerate() {
            let area = RECT {
                left,
                top,
                right,
                bottom: top + geometry.row,
            };

            if index == marked {
                crate::theme::paint_rounded(dc, &area, at(4), colour(colours.hover), hover, dpi);
            }

            if row.layout == current {
                let bar = RECT {
                    left,
                    top: top + at(14),
                    right: left + at(3),
                    bottom: top + geometry.row - at(14),
                };

                // SAFETY: the live paint DC and a brush of this frame.
                unsafe { FillRect(dc, &bar, accent) };
            }

            text(
                &row.code,
                RECT {
                    left,
                    top,
                    right: left + geometry.code,
                    bottom: area.bottom,
                },
                colours.text,
                bold,
                true,
            );
            // Two lines as in the list of Windows: the language, and under it, muted, the name of
            // the keyboard layout (one line, centred, when Windows names no layout).
            let middle = top + geometry.row / 2;
            let (first, second) = if row.layout_name.is_empty() {
                ((top, area.bottom), None)
            } else {
                (
                    (top + at(6), middle + at(1)),
                    Some((middle + at(1), area.bottom - at(6))),
                )
            };

            text(
                &row.name,
                RECT {
                    left: left + geometry.code + at(8),
                    top: first.0,
                    right: right - at(8),
                    bottom: first.1,
                },
                colours.text,
                normal,
                false,
            );

            if let Some((upper, lower)) = second {
                text(
                    &row.layout_name,
                    RECT {
                        left: left + geometry.code + at(8),
                        top: upper,
                        right: right - at(8),
                        bottom: lower,
                    },
                    colours.muted,
                    small,
                    false,
                );
            }

            top += geometry.row;
        }

        let line = RECT {
            left: left + at(8),
            top: top + at(4),
            right: right - at(8),
            bottom: top + at(5),
        };

        // SAFETY: the live paint DC and a brush of this frame.
        unsafe { FillRect(dc, &line, frame) };
        top += geometry.separator;

        if link_hover {
            let area = RECT {
                left,
                top,
                right,
                bottom: top + geometry.link,
            };
            crate::theme::paint_rounded(dc, &area, at(4), colour(colours.hover), hover, dpi);
        }

        text(
            &link,
            RECT {
                left: left + at(8),
                top,
                right: right - at(8),
                bottom: top + geometry.link,
            },
            colours.accent,
            normal,
            false,
        );

        // SAFETY: objects of this frame, no longer selected into any DC.
        unsafe {
            let _ = DeleteObject(ground.into());
            let _ = DeleteObject(frame.into());
            let _ = DeleteObject(hover.into());
            let _ = DeleteObject(accent.into());
            let _ = DeleteObject(small.into());
            let _ = DeleteObject(normal.into());
            let _ = DeleteObject(bold.into());
        }
    }

    // SAFETY: the paint begun above.
    let _ = unsafe { EndPaint(hwnd, &paint) };
}
