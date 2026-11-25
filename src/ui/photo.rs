//! **The picture of an author's card on the canvas** — stage Э121, task T-121-4 (решение 208.244
//! п. 7): what part of the picture a box shows, and the pictures the windows of this thread hold.
//!
//! The bytes come here only from [`crate::cards::kept`] — read back from the disk and checked again
//! (the size, the sum the feed signed, the format and the size in pixels out of the header). The
//! canvas decodes them with the decoder of Windows of exactly that format (WIC, no crate), cut to
//! the part the box shows and brought down to the pixels of the box at once
//! (`ui::canvas::Painter::photo`): a picture made for 200 % is made smaller, never larger than the
//! author made it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::cards::Format;

/// What a box shows of a picture: the part of the picture, in its pixels, and where that part
/// stands in the box, in the units of the box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    /// `x`, `y`, `w`, `h` of the part of the picture.
    pub source: (u32, u32, u32, u32),
    /// `x`, `y`, `w`, `h` of the place in the box; the whole box, unless the middle had to be kept.
    pub place: (f32, f32, f32, f32),
}

/// **Cover, centred** — the box filled, and what is over its proportions cut evenly at both ends.
///
/// `keep` — the share of the width that is the middle of the picture and is never cut: 0 for the
/// panel (Б), [`crate::cards::FULL_MIDDLE_PIXELS`] of [`crate::cards::FULL_PIXELS`] for the whole
/// window (В1, «на малом экране края срезаются», the middle never). A box narrower than the middle's
/// proportions shows the whole middle at its whole height, fitted to the width of the box and centred
/// in it; the colour of the window shows above and below. `None` for an empty picture or box.
pub fn fit(src_w: u32, src_h: u32, box_w: f32, box_h: f32, keep: f32) -> Option<Fit> {
    if src_w == 0 || src_h == 0 || box_w.is_nan() || box_h.is_nan() || box_w < 1.0 || box_h < 1.0 {
        return None;
    }

    let (sw, sh) = (src_w as f32, src_h as f32);
    let aspect = box_w / box_h;
    let whole = (0.0, 0.0, box_w, box_h);

    if aspect >= sw / sh {
        // The box is wider than the picture: its whole width, a band of its height.
        let h = (sw / aspect).round().clamp(1.0, sh);
        let y = ((sh - h) / 2.0).floor();
        return Some(Fit {
            source: (0, y as u32, src_w, h as u32),
            place: whole,
        });
    }

    // The box is narrower: the whole height, a band of the width — and never narrower than the
    // middle.
    let least = (sw * keep.clamp(0.0, 1.0)).round();
    let w = (sh * aspect).round().clamp(1.0, sw);

    if w >= least {
        let x = ((sw - w) / 2.0).floor();
        return Some(Fit {
            source: (x as u32, 0, w as u32, src_h),
            place: whole,
        });
    }

    let x = ((sw - least) / 2.0).floor();
    let h = box_w * sh / least;

    Some(Fit {
        source: (x as u32, 0, least as u32, src_h),
        place: (0.0, (box_h - h) / 2.0, box_w, h),
    })
}

/// A picture the windows of this thread may draw: the checked bytes, their format, their size.
#[derive(Clone, Debug)]
pub struct Held {
    pub bytes: Rc<[u8]>,
    pub format: Format,
    pub w: u32,
    pub h: u32,
}

/// How many pictures a thread holds — the card on the screen and the one before it; a picture
/// is at most 1 MB.
const HELD: usize = 2;

thread_local! {
    /// The pictures of the cards, by their key ([`key_of`]).
    static PHOTOS: RefCell<Vec<(u64, Held)>> = const { RefCell::new(Vec::new()) };
}

/// The key of a picture: the first eight bytes of its sum. The same picture is the same key, in
/// every window and every build of the description of a window.
pub fn key_of(sha256: &[u8; 32]) -> u64 {
    u64::from_le_bytes([
        sha256[0], sha256[1], sha256[2], sha256[3], sha256[4], sha256[5], sha256[6], sha256[7],
    ])
}

/// **Holds a checked picture** for the windows of this thread and answers its key. The newest
/// [`HELD`] are held; an older one is let go.
pub fn hold(key: u64, held: Held) -> u64 {
    PHOTOS.with_borrow_mut(|photos| {
        photos.retain(|(known, _)| *known != key);
        photos.push((key, held));
        if photos.len() > HELD {
            photos.drain(..photos.len() - HELD);
        }
    });
    key
}

/// The picture held under `key`, if it is.
pub fn held(key: u64) -> Option<Held> {
    PHOTOS.with_borrow(|photos| {
        photos
            .iter()
            .find(|(known, _)| *known == key)
            .map(|(_, held)| held.clone())
    })
}

/// **The picture of a card for its window** — stage Э121, task T-121-5: where the view of the card
/// puts it (the start or the end of the line for Б, the whole window for В1), and the picture read
/// back from `folder`, checked again ([`crate::cards::kept`]) and held for the windows of this
/// thread. `None` — the plain template (А): a card of that view, one with no picture, or a picture
/// that is not on the disk or not whole.
pub fn of_card(
    card: &crate::cards::FeedCard,
    folder: &std::path::Path,
) -> Option<crate::ui::scene::Photo> {
    use crate::cards::{CardSide, CardView};
    use crate::ui::scene::{Photo, PhotoPlace};

    let place = match card.view {
        CardView::Panel(CardSide::Start) => PhotoPlace::Start,
        CardView::Panel(CardSide::End) => PhotoPlace::End,
        CardView::Full => PhotoPlace::Whole,
        CardView::Text => return None,
    };
    let image = card.image.as_ref()?;
    let bytes = crate::cards::kept(card, folder)?;
    let (format, w, h) = crate::cards::sniff(&bytes)?;
    let key = hold(
        key_of(&image.sha256),
        Held {
            bytes: Rc::from(bytes),
            format,
            w,
            h,
        },
    );

    Some(Photo { key, place })
}
