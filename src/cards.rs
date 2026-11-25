//! **The author's cards** — stage Э121 (решения 208.195, 208.244): a letter from the author with a
//! picture of his own, shown once in a term «from — to».
//!
//! # What a card is
//!
//! A record `[[card]]` of the signed feed — a top-level array of its own, which every released
//! version skips (`RawFeed` has no `deny_unknown_fields`; `tests\feed_cards.rs` reads the file with
//! the frozen parse of 1.8.0). Nothing of a card is put on an `[[item]]`: one field there refuses
//! the whole feed in every released version.
//!
//! A card names its **view** — Б, the picture in place of the panel with the panda, at the start
//! or the end of the line ([`CardView::Panel`]), or В1, the picture over the whole window with the
//! words and the buttons in a strip of the window's colour under it ([`CardView::Full`]) — and the
//! one picture both themes show. A picture that did not come, or did not pass its checks, leaves
//! the card in the plain template of a news item (А, [`CardView::Text`]): the greeting is not lost.
//!
//! # The picture is bytes the author signed, or nothing
//!
//! The feed names the address of the picture — only the author's site, `/langswitcher/cards/` —
//! its SHA-256 and its size. The program downloads no more than that size, no sooner than
//! [`DOWNLOAD_AHEAD_DAYS`] days before the term; the sum is checked **before** anything parses
//! the file, so the decoder of Windows (WIC) is handed only bytes the signature stands behind; the
//! size in pixels is read out of the header ([`sniff`]) before the picture is decoded; after the
//! term the file is deleted.

use crate::letters::Date;

/// How many cards the program keeps from one feed — the ones with the greatest numbers.
pub const CARDS_KEPT: usize = 3;

/// The largest picture of a card, in bytes — 1 MB (208.244 п. 7).
pub const IMAGE_CAP: u64 = 1024 * 1024;

/// How many days before its term a card's picture may be downloaded.
pub const DOWNLOAD_AHEAD_DAYS: i64 = 7;

/// The picture of the panel (Б), in pixels: twice the panel of the largest window, 1280 × 720 at
/// 200 % (a panel of 38 % is 972 × 1440), with room.
pub const PANEL_PIXELS: (u32, u32) = (1000, 1440);

/// The picture of the whole window (В1), in pixels: 2 : 1, twice the largest window. Its middle,
/// [`FULL_MIDDLE_PIXELS`], is what every window shows; the sides are cut on a small screen.
pub const FULL_PIXELS: (u32, u32) = (2560, 1280);

/// The middle of the picture of the whole window, which is never cut.
pub const FULL_MIDDLE_PIXELS: (u32, u32) = (1780, 1280);

/// Which side of the line the picture of the panel stands at — the author's word. In a language
/// read right to left the sides change places; the picture itself is never mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardSide {
    Start,
    End,
}

/// The view of a card — the author's word in the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardView {
    /// А — the template of a news item, no picture: a view the author did not name or this build
    /// does not know, and the view of every card whose picture is not there.
    Text,
    /// Б — the picture in place of the panel with the panda, at a side of the line.
    Panel(CardSide),
    /// В1 — the picture over the whole window, the words and the buttons in a strip under it.
    Full,
}

impl CardView {
    /// The view of a record: `view` and `side` as the author wrote them.
    pub fn of(view: &str, side: &str) -> Self {
        match view {
            "panel" => CardView::Panel(if side == "end" {
                CardSide::End
            } else {
                CardSide::Start
            }),
            "full" => CardView::Full,
            _ => CardView::Text,
        }
    }

    /// The size in pixels the picture of this view is made at; `None` for the plain template.
    pub const fn pixels(self) -> Option<(u32, u32)> {
        match self {
            CardView::Text => None,
            CardView::Panel(_) => Some(PANEL_PIXELS),
            CardView::Full => Some(FULL_PIXELS),
        }
    }
}

/// The picture a card names: where it lies, its SHA-256 and its size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardImage {
    pub url: String,
    pub sha256: [u8; 32],
    /// In bytes, from one to [`IMAGE_CAP`].
    pub size: u64,
}

/// One card of the feed, with the language chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedCard {
    /// The author's number of the card — what «shown» is remembered by.
    pub id: u64,
    /// The first day of the term.
    pub from: Date,
    /// The last day of the term.
    pub to: Date,
    pub view: CardView,
    /// `None` — the record names no picture, or not the whole of one.
    pub image: Option<CardImage>,
    /// The heading and the words, in the language of the interface, else English, else Russian.
    pub title: String,
    pub text: String,
    /// The address of «Читать на сайте»; may be empty.
    pub link: String,
}

impl FeedCard {
    /// Whether `today` is within the term.
    pub fn in_term(&self, today: Date) -> bool {
        self.from <= today && today <= self.to
    }

    /// Whether the picture may be downloaded today: from [`DOWNLOAD_AHEAD_DAYS`] days before the
    /// term to its last day.
    pub fn picture_is_due(&self, today: Date) -> bool {
        self.image.is_some()
            && self.view != CardView::Text
            && self.from.days_since(today) <= DOWNLOAD_AHEAD_DAYS
            && today <= self.to
    }
}

/// Reads 64 hexadecimal digits, in either case.
pub fn sum_of(digits: &str) -> Option<[u8; 32]> {
    let digits = digits.as_bytes();

    if digits.len() != 64 {
        return None;
    }

    let mut sum = [0u8; 32];

    for (byte, pair) in sum.iter_mut().zip(digits.chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16)?;
        let low = char::from(pair[1]).to_digit(16)?;

        *byte = u8::try_from(high * 16 + low).ok()?;
    }

    Some(sum)
}

/// The format of a picture, by its signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
}

/// **The format and the size in pixels of a picture, out of its header** — without decoding it.
///
/// PNG: the signature and the first chunk, `IHDR`. JPEG: the markers from the start of the image
/// to the first frame header (`SOF0`…`SOF15`, but not `DHT`, `JPG` and `DAC`), skipping every
/// segment by its length. `None` for anything else — a format the card does not take, a header cut
/// short, a size of zero.
pub fn sniff(bytes: &[u8]) -> Option<(Format, u32, u32)> {
    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    if bytes.starts_with(&PNG) {
        let ihdr = bytes.get(8..24)?;
        if &ihdr[4..8] != b"IHDR" || ihdr[..4] != [0, 0, 0, 13] {
            return None;
        }
        let w = u32::from_be_bytes(ihdr[8..12].try_into().ok()?);
        let h = u32::from_be_bytes(ihdr[12..16].try_into().ok()?);
        return (w > 0 && h > 0).then_some((Format::Png, w, h));
    }

    if bytes.starts_with(&[0xFF, 0xD8]) {
        let mut at = 2usize;
        loop {
            // Fill bytes before a marker are allowed.
            while bytes.get(at) == Some(&0xFF) && bytes.get(at + 1) == Some(&0xFF) {
                at += 1;
            }
            if *bytes.get(at)? != 0xFF {
                return None;
            }
            let marker = *bytes.get(at + 1)?;
            // A marker with no segment: the restarts, a stuffed zero, TEM.
            if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                at += 2;
                continue;
            }
            // The end of the image or the start of the scan before a frame: no size.
            if marker == 0xD9 || marker == 0xDA {
                return None;
            }
            let length = usize::from(u16::from_be_bytes([
                *bytes.get(at + 2)?,
                *bytes.get(at + 3)?,
            ]));
            if length < 2 {
                return None;
            }
            let frame = (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
            if frame {
                let header = bytes.get(at + 4..at + 9)?;
                let h = u32::from(u16::from_be_bytes([header[1], header[2]]));
                let w = u32::from(u16::from_be_bytes([header[3], header[4]]));
                return (w > 0 && h > 0).then_some((Format::Jpeg, w, h));
            }
            at = at.checked_add(2 + length)?;
        }
    }

    None
}

/// **Whether a picture of `w` × `h` pixels is one the card may show in `view`** — no larger than the
/// size the view is made at ([`CardView::pixels`]), so a picture never costs more memory than the
/// author's own; any smaller one is taken and stretched. Nothing for the plain template.
pub fn size_fits(view: CardView, w: u32, h: u32) -> bool {
    view.pixels()
        .is_some_and(|(most_w, most_h)| w > 0 && h > 0 && w <= most_w && h <= most_h)
}
