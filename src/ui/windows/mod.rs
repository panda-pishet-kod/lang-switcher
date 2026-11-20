//! Window descriptions: which elements, in what order, which strings. No numbers, colours or
//! coordinates.

pub mod about;
pub mod author;
pub mod corner;
pub mod letter;
pub mod letters_list;
pub mod tray_menu;

use crate::settings::Language;
use crate::ui::scene::Window;

/// Every window of the shell in every state it shows — each step and each section — in
/// `language`, named for the reports. The instrument of fitting (`tests\ui_fit.rs`) goes over this
/// list: a new window adds its line here, and from then on it is fitted with the others.
pub fn catalogue(language: Language) -> Vec<(String, Window<u16>)> {
    let mut all = Vec::new();
    all.extend(
        about::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&|id| id as u16))),
    );
    all.extend(
        author::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&author::Id::number))),
    );
    all.extend(
        letter::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&letter::Id::number))),
    );
    all.extend(
        letters_list::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&letters_list::Id::number))),
    );
    // The windows in the corner (T-120-14): the hint of the flag and «Обновлено» in the words of
    // `corner.rs`; the knock of a letter in the words of `letters.rs`.
    let mut corners = vec![
        ("corner hint", crate::corner::hint_content_in(language)),
        (
            "corner updated",
            crate::corner::release_content_in(language, "1.6.1"),
        ),
    ];
    for (name, letter) in [
        ("corner letter thanks", crate::letters::Letter::Thanks),
        (
            "corner letter news",
            crate::letters::Letter::News(crate::letters::sample_entry().id),
        ),
    ] {
        if let Some(content) = crate::letters::notice_content_in(language, letter) {
            corners.push((name, content));
        }
    }
    // The menu of the icon (T-120-15) in its five states.
    all.extend(
        tray_menu::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&|id| id as u16))),
    );
    for (name, content) in corners {
        let window = corner::describe(&content, false, None);
        all.push((name.to_owned(), window.map(&|id| id as u16)));
    }
    all
}
