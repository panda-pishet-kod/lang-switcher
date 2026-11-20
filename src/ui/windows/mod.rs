//! Window descriptions: which elements, in what order, which strings. No numbers, colours or
//! coordinates.

pub mod about;
pub mod corner;
pub mod settings;
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
    // The windows in the corner (T-120-14): the hint of the flag and «Обновлено» in the words of
    // `corner.rs`; the notice of a letter is built in `letters.rs`.
    let corners = [
        ("corner hint", crate::corner::hint_content_in(language)),
        (
            "corner updated",
            crate::corner::release_content_in(language, "1.6.1"),
        ),
    ];
    // The window of the settings (T-120-20): five sections and «Несколько раскладок».
    all.extend(
        settings::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&|id| id as u16))),
    );
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
