//! The factor k of a window's type size.
//!
//! The factor is one per language and class of window, from a table a probe measures (T-120-11):
//! no window picks its own while it runs. Until the table exists — and after, as the road of last
//! resort only — a window whose content does not fit at its factor is laid out at a smaller one,
//! down to the floor of 12 points, and the journal says so ([`emergency`]).
//!
//! Knows nothing of Direct2D.

use crate::diag;
use crate::ui::layout::{Class, Layout, WindowSize, lay_out};
use crate::ui::scene::Window;
use crate::ui::text::Measure;
use crate::ui::tokens::{TYPE_FLOOR, window_b};

/// The journal's name for a window laid out at a smaller type size than its factor
/// (`src\diag.rs`, the vocabulary).
pub const TYPE_SIZE_LOWERED: &str = "ui layout: type size lowered";

/// Halvings of the search between the floor and the factor: the factor is then known to a 256th of
/// the span, finer than a tenth of a point of type.
const SEARCH_STEPS: u32 = 8;

/// `window` laid out at the factor `k`; when its content does not fit, at the largest factor that
/// fits down to the one that gives B = 12 — or at that one, fitting or not. A lowered factor is
/// written to the journal.
pub fn emergency<I: Copy>(
    window: &Window<I>,
    size: WindowSize,
    k: f32,
    rtl: bool,
    measure: &dyn Measure,
) -> Layout<I> {
    let at = |k: f32| lay_out(window, size, k, rtl, measure);
    let first = at(k);
    if first.shortfall.fits() {
        return first;
    }

    let floor = (TYPE_FLOOR.0 / window_b(size.w, size.h, 1.0)).min(k);
    let mut best = at(floor);
    if best.shortfall.fits() {
        let (mut fits, mut fails) = (floor, k);
        for _ in 0..SEARCH_STEPS {
            let middle = (fits + fails) / 2.0;
            let tried = at(middle);
            if tried.shortfall.fits() {
                fits = middle;
                best = tried;
            } else {
                fails = middle;
            }
        }
    }

    diag::record(
        diag::Operation::from_name(TYPE_SIZE_LOWERED),
        diag::OsCode::NONE,
    );
    best
}

/// The factor k of one language for the three classes of window (passport §2: narrow 620 × 500,
/// ordinary 900 × 540, large 1280 × 690 — each the tightest window of its layout).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct K {
    pub narrow: f32,
    pub ordinary: f32,
    pub large: f32,
}

impl K {
    /// The factor of `class`.
    pub fn of(self, class: Class) -> f32 {
        match class {
            Class::Narrow => self.narrow,
            Class::Ordinary => self.ordinary,
            Class::Large => self.large,
        }
    }
}

/// The factor of the language tagged `tag` in a window of `class`, from [`TABLE`]: no window picks
/// its own while it runs (T-120-11). A language the table lacks takes 1, and the road of last resort
/// ([`emergency`]) remains.
pub fn k_for(tag: &str, class: Class) -> f32 {
    TABLE
        .iter()
        .find(|(known, _)| *known == tag)
        .map_or(1.0, |(_, k)| k.of(class))
}

// ⟦ The table of k — printed by `tests\ui_fit.rs` (LANGSW_UI_FIT_WRITE): the smallest factor every
// window of the shell needs in the language and class, with both main faces, so that its content
// still fits at a type 3 % larger. Not edited by hand; the guard of the table checks it. ⟧
pub const TABLE: &[(&str, K)] = &[
    (
        "ru",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.94,
        },
    ),
    (
        "en",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "uk",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "de",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.94,
        },
    ),
    (
        "fr",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "es",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "pt",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "it",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "pl",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 1.0,
        },
    ),
    (
        "cs",
        K {
            narrow: 0.888,
            ordinary: 0.888,
            large: 1.0,
        },
    ),
    (
        "tr",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "el",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "he",
        K {
            narrow: 0.888,
            ordinary: 0.888,
            large: 1.0,
        },
    ),
    (
        "ar",
        K {
            narrow: 0.895,
            ordinary: 0.895,
            large: 1.0,
        },
    ),
    (
        "hi",
        K {
            narrow: 0.912,
            ordinary: 0.912,
            large: 0.931,
        },
    ),
    (
        "bn",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "ur",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "pa",
        K {
            narrow: 0.912,
            ordinary: 0.912,
            large: 0.931,
        },
    ),
    (
        "mr",
        K {
            narrow: 0.847,
            ordinary: 0.847,
            large: 0.964,
        },
    ),
    (
        "te",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.931,
        },
    ),
    (
        "ta",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.867,
        },
    ),
    (
        "fa",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "gu",
        K {
            narrow: 0.862,
            ordinary: 0.862,
            large: 1.0,
        },
    ),
    (
        "th",
        K {
            narrow: 0.904,
            ordinary: 0.904,
            large: 1.0,
        },
    ),
    (
        "kn",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "or",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "ml",
        K {
            narrow: 0.841,
            ordinary: 0.841,
            large: 0.938,
        },
    ),
    (
        "uz",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.907,
        },
    ),
    (
        "uz-Cyrl",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.907,
        },
    ),
    (
        "ne",
        K {
            narrow: 0.882,
            ordinary: 0.882,
            large: 1.0,
        },
    ),
    (
        "as",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "az",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "km",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "kk",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.931,
        },
    ),
    (
        "sr",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "sr-Latn",
        K {
            narrow: 0.866,
            ordinary: 0.866,
            large: 1.0,
        },
    ),
    (
        "ug",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "bg",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "hy",
        K {
            narrow: 0.875,
            ordinary: 0.875,
            large: 0.909,
        },
    ),
    (
        "lo",
        K {
            narrow: 0.879,
            ordinary: 0.879,
            large: 1.0,
        },
    ),
    (
        "be",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "tt",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "ka",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "mk",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "vi",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "nl",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "ro",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "hu",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.931,
        },
    ),
    (
        "sv",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "sq",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "da",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "fi",
        K {
            narrow: 0.847,
            ordinary: 0.847,
            large: 0.964,
        },
    ),
    (
        "hr",
        K {
            narrow: 0.912,
            ordinary: 0.912,
            large: 0.931,
        },
    ),
    (
        "sk",
        K {
            narrow: 0.847,
            ordinary: 0.847,
            large: 0.964,
        },
    ),
    (
        "nb",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "lt",
        K {
            narrow: 0.847,
            ordinary: 0.847,
            large: 0.964,
        },
    ),
    (
        "sl",
        K {
            narrow: 0.876,
            ordinary: 0.876,
            large: 1.0,
        },
    ),
    (
        "bs",
        K {
            narrow: 0.847,
            ordinary: 0.847,
            large: 0.964,
        },
    ),
    (
        "lv",
        K {
            narrow: 0.888,
            ordinary: 0.888,
            large: 1.0,
        },
    ),
    (
        "et",
        K {
            narrow: 0.888,
            ordinary: 0.888,
            large: 1.0,
        },
    ),
    (
        "cy",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.94,
        },
    ),
    (
        "mt",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "is",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
    (
        "lb",
        K {
            narrow: 0.877,
            ordinary: 0.877,
            large: 0.94,
        },
    ),
    (
        "mi",
        K {
            narrow: 0.878,
            ordinary: 0.878,
            large: 0.94,
        },
    ),
    (
        "nn",
        K {
            narrow: 0.856,
            ordinary: 0.856,
            large: 0.974,
        },
    ),
];
// ⟦ End of the table. ⟧
