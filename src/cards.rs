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

// =========================================================================================
// The picture on the disk — task T-121-3 (решение 208.244 п. 8)
// =========================================================================================

/// The folder of the pictures inside the program's own folder of `%APPDATA%`.
pub const FOLDER_NAME: &str = "cards";

/// What the file of a picture is called: the digits of its sum and `.card`. The name says which
/// bytes it must hold, and a file of the same name with other bytes is no picture ([`kept`]).
pub const FILE_EXTENSION: &str = "card";

/// The folder of the pictures in an arbitrary application data directory —
/// `%APPDATA%\Lang_Switcher\cards` for the product ([`folder`]).
pub fn folder_in(app_data: &std::path::Path) -> std::path::PathBuf {
    app_data.join(crate::CONFIG_DIR_NAME).join(FOLDER_NAME)
}

/// The standard folder of the pictures — [`folder_in`] over `%APPDATA%`, `None` without it.
pub fn folder() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| folder_in(std::path::Path::new(&app_data)))
}

/// The file of `image` in `folder`.
pub fn path_in(folder: &std::path::Path, image: &CardImage) -> std::path::PathBuf {
    let digits: String = image
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    folder.join(format!("{digits}.{FILE_EXTENSION}"))
}

/// **The address the pictures are taken from** — [`crate::links::CARDS_URL`]. Under the `testing`
/// feature only, the stand's: the folder `cards/` beside the feed the doors of the stand name
/// ([`crate::update::Places::of_this_machine`]), so that a guest without a network has its cards.
pub fn folder_url() -> String {
    #[cfg(feature = "testing")]
    if let Some(stand) = stand_folder_url() {
        return stand;
    }

    crate::links::CARDS_URL.to_owned()
}

/// The folder `cards/` beside the feed of the doors of the stand; `None` when the doors name no feed.
#[cfg(feature = "testing")]
fn stand_folder_url() -> Option<String> {
    let places = crate::update::Places::of_this_machine()?;

    if places.feed_url == crate::links::FEED_URLS[0] {
        return None;
    }

    let (base, _) = places.feed_url.rsplit_once('/')?;

    Some(format!("{base}/{FOLDER_NAME}/"))
}

/// Why a picture was not kept — the journal is told one fact and no value (SEC-07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The address is not a file in the folder of the cards.
    Address,
    /// The host did not answer, or answered with more than the record named.
    Network,
    /// Not the size the record named.
    Size,
    /// Not the sum the record named.
    Sum,
    /// Not a PNG or a JPEG.
    Format,
    /// Larger in pixels than the view is made at.
    Pixels,
    /// The file could not be written.
    Disk,
}

/// **Whether these bytes are the picture `image` names, for `view`** — in the order of решение
/// 208.244 п. 8: the size; then the sum, **before** a byte of the picture is parsed — the header
/// is read only out of bytes the signature of the feed stands behind; then the format and the size
/// in pixels out of the header ([`sniff`], [`size_fits`]). The decoder of Windows is reached only
/// after all of it, by the window.
pub fn check(
    bytes: &[u8],
    image: &CardImage,
    view: CardView,
) -> Result<(Format, u32, u32), Refused> {
    if bytes.len() as u64 != image.size || image.size > IMAGE_CAP {
        return Err(Refused::Size);
    }

    let mut hasher = crate::feed::Sha256::new().ok_or(Refused::Sum)?;
    if !hasher.update(bytes) || hasher.finish() != Some(image.sha256) {
        return Err(Refused::Sum);
    }

    let (format, w, h) = sniff(bytes).ok_or(Refused::Format)?;

    if !size_fits(view, w, h) {
        return Err(Refused::Pixels);
    }

    Ok((format, w, h))
}

/// **The picture of a card, if it is on the disk and whole** — read back and checked again
/// ([`check`]) every time: a file somebody changed, cut or replaced is no picture, and the card
/// shows the plain template (А). `None` for a card with no picture, or a view with none.
pub fn kept(card: &FeedCard, folder: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let image = card.image.as_ref()?;
    let file = std::fs::File::open(path_in(folder, image)).ok()?;
    let mut bytes = Vec::new();
    file.take(IMAGE_CAP + 1).read_to_end(&mut bytes).ok()?;

    check(&bytes, image, card.view).ok().map(|_| bytes)
}

/// **One `GET` of a picture**, through the one body of the network of this program
/// ([`crate::feed::get`]): the bytes, at most `cap`; `None` for every refusal, and for more.
pub fn get_picture(url: &str, cap: u64) -> Option<Vec<u8>> {
    let target = crate::feed::target_of(url)?;
    let mut bytes = Vec::new();

    crate::feed::get(&target, cap, &mut |chunk| {
        bytes.extend_from_slice(chunk);
        true
    })?;

    Some(bytes)
}

/// **The pictures of a good read of the feed**, on the thread of the feed — task T-121-3.
///
/// Each card whose picture is due today ([`FeedCard::picture_is_due`]: from
/// [`DOWNLOAD_AHEAD_DAYS`] days before the term to its end) and is not already whole on the disk
/// ([`kept`]) is asked for **once**: the address only in `folder_url`, no more than the size the
/// record names, then [`check`], then the file, written whole or not at all. What did not come is
/// asked again at the next read, the next day. Then [`sweep`].
///
/// **Nothing at all without a card**: no request, no folder, no file — the sentry of the stage. The
/// request itself is `get`, so that a test can count it; the product passes [`get_picture`].
pub fn fetch_pictures(
    cards: &[FeedCard],
    today: Date,
    folder: &std::path::Path,
    folder_url: &str,
    get: &mut dyn FnMut(&str, u64) -> Option<Vec<u8>>,
) -> Vec<(u64, Result<(), Refused>)> {
    let mut outcomes = Vec::new();

    for card in cards.iter().filter(|card| card.picture_is_due(today)) {
        let Some(image) = card.image.as_ref() else {
            continue;
        };

        if kept(card, folder).is_some() {
            continue;
        }

        let outcome = fetch_one(card, image, folder, folder_url, get);

        crate::diag::record(
            crate::diag::Operation::from_name(if outcome.is_ok() {
                "card picture kept"
            } else {
                "card picture refused"
            }),
            crate::diag::OsCode::NONE,
        );
        outcomes.push((card.id, outcome));
    }

    sweep(cards, today, folder);

    outcomes
}

/// One picture: the address, the request, the checks, the file.
fn fetch_one(
    card: &FeedCard,
    image: &CardImage,
    folder: &std::path::Path,
    folder_url: &str,
    get: &mut dyn FnMut(&str, u64) -> Option<Vec<u8>>,
) -> Result<(), Refused> {
    if !crate::update::address_is_ours(&image.url, folder_url) {
        return Err(Refused::Address);
    }

    let bytes = get(&image.url, image.size).ok_or(Refused::Network)?;

    check(&bytes, image, card.view)?;

    crate::settings::write_bytes_to(&path_in(folder, image), &bytes)
        .map(|_| ())
        .map_err(|_| Refused::Disk)
}

/// **Deletes what no card needs** — every file of the folder but the pictures of the cards whose
/// term is not over (решение 208.244 п. 8: «после срока файл стереть»), and the folder itself when
/// nothing is left in it. A folder that is not there is left not there.
pub fn sweep(cards: &[FeedCard], today: Date, folder: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };

    let needed: Vec<std::path::PathBuf> = cards
        .iter()
        .filter(|card| today <= card.to)
        .filter_map(|card| card.image.as_ref())
        .map(|image| path_in(folder, image))
        .collect();

    for entry in entries.flatten() {
        let path = entry.path();

        if !needed.contains(&path) {
            crate::sandbox::guard_path(&path);
            let _ = std::fs::remove_file(&path);
        }
    }

    // Removed only when empty — `remove_dir` refuses a folder with anything in it.
    let _ = std::fs::remove_dir(folder);
}
