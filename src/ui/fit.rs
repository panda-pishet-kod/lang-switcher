//! The factor k of a window's type size.
//!
//! The factor is one per language and class of window, from a table a probe measures (T-120-11):
//! no window picks its own while it runs. Until the table exists — and after, as the road of last
//! resort only — a window whose content does not fit at its factor is laid out at a smaller one,
//! down to the floor of 12 points, and the journal says so ([`emergency`]).
//!
//! Knows nothing of Direct2D.

use crate::diag;
use crate::ui::layout::{Layout, WindowSize, lay_out};
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
